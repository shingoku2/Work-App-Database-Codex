use crate::{
    auth::{hash_password, new_token, token_hash, user_from_row, verify_password},
    config::ServerConfig,
};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use fleet_shared::{
    normalize_and_validate_miner, normalize_username, validate_part, validate_password, ApiError,
    ChangePasswordRequest, CountByStatus, CreateMiner, CreatePart, CreateUserRequest,
    DashboardSummary, LoginRequest, LoginResponse, Miner, MinerImportResult, PairingInfo, Part,
    ResetPasswordRequest, ServerInfo, UpdateMiner, UpdateUserRequest, User, UserRole, API_VERSION,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration as StdDuration, Instant},
};
use tokio::sync::Mutex;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    session_days: i64,
    login_limiter: Arc<Mutex<LoginLimiter>>,
    dummy_password_hash: String,
    pairing: PairingInfo,
}

const LOGIN_WINDOW: StdDuration = StdDuration::from_secs(60);
const LOGIN_ACCOUNT_LIMIT: usize = 5;
const LOGIN_SOURCE_LIMIT: usize = 30;
const LOGIN_LIMITER_CAPACITY: usize = 10_000;

#[derive(Default)]
struct LoginLimiter {
    account_attempts: HashMap<(IpAddr, String), Vec<Instant>>,
    source_attempts: HashMap<IpAddr, Vec<Instant>>,
}

impl LoginLimiter {
    fn allow(&mut self, source: IpAddr, username: &str, now: Instant) -> bool {
        self.prune(now);
        let account_key = (source, username.to_string());
        if self.account_attempts.get(&account_key).map_or(0, Vec::len) >= LOGIN_ACCOUNT_LIMIT
            || self.source_attempts.get(&source).map_or(0, Vec::len) >= LOGIN_SOURCE_LIMIT
        {
            return false;
        }
        self.ensure_capacity();
        self.account_attempts
            .entry(account_key)
            .or_default()
            .push(now);
        self.source_attempts.entry(source).or_default().push(now);
        true
    }

    fn clear(&mut self, source: IpAddr, username: &str) {
        self.account_attempts
            .remove(&(source, username.to_string()));
        self.source_attempts.remove(&source);
    }

    fn prune(&mut self, now: Instant) {
        self.account_attempts.retain(|_, attempts| {
            attempts.retain(|attempt| now.duration_since(*attempt) < LOGIN_WINDOW);
            !attempts.is_empty()
        });
        self.source_attempts.retain(|_, attempts| {
            attempts.retain(|attempt| now.duration_since(*attempt) < LOGIN_WINDOW);
            !attempts.is_empty()
        });
    }

    fn ensure_capacity(&mut self) {
        while self.account_attempts.len() + self.source_attempts.len() + 2 > LOGIN_LIMITER_CAPACITY
        {
            if let Some(key) = self.account_attempts.keys().next().cloned() {
                self.account_attempts.remove(&key);
            } else if let Some(key) = self.source_attempts.keys().next().copied() {
                self.source_attempts.remove(&key);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn login_limiter_is_source_aware_and_expires_entries() {
        let now = Instant::now();
        let first = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let second = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let mut limiter = LoginLimiter::default();

        for _ in 0..LOGIN_ACCOUNT_LIMIT {
            assert!(limiter.allow(first, "admin", now));
        }
        assert!(!limiter.allow(first, "admin", now));
        assert!(limiter.allow(second, "admin", now));
        assert!(limiter.allow(first, "admin", now + LOGIN_WINDOW));
    }

    #[test]
    fn login_limiter_storage_is_bounded() {
        let now = Instant::now();
        let mut limiter = LoginLimiter::default();
        for index in 0..6_000u32 {
            let source = IpAddr::V4(Ipv4Addr::from(index + 1));
            assert!(limiter.allow(source, "unknown", now));
        }
        assert!(
            limiter.account_attempts.len() + limiter.source_attempts.len()
                <= LOGIN_LIMITER_CAPACITY
        );
    }
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "version_conflict",
            message: message.into(),
        }
    }

    fn database(error: sqlx::Error) -> Self {
        tracing::error!(error = %error, "database operation failed");
        let (status, code, message) = if let Some(db_error) = error.as_database_error() {
            if db_error.is_unique_violation() {
                (
                    StatusCode::CONFLICT,
                    "duplicate",
                    "a record with that identity already exists".to_string(),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "database operation failed".to_string(),
                )
            }
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "database operation failed".to_string(),
            )
        };
        Self {
            status,
            code,
            message,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiError {
                code: self.code.into(),
                message: self.message,
                details: None,
            }),
        )
            .into_response()
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(serde::Deserialize)]
struct VersionQuery {
    version: i64,
}

pub async fn serve(config: ServerConfig, pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let certificate_pem = std::fs::read_to_string(&config.tls.certificate)?;
    let mut cert_reader = std::io::BufReader::new(certificate_pem.as_bytes());
    let certificate_der = rustls_pemfile::certs(&mut cert_reader)
        .next()
        .ok_or("TLS certificate file contains no certificate")??;
    let fingerprint_sha256 = Sha256::digest(certificate_der.as_ref())
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":");
    let state = AppState {
        pool,
        session_days: config.session_days,
        login_limiter: Arc::new(Mutex::new(LoginLimiter::default())),
        dummy_password_hash: hash_password("dummy-password-never-used")?,
        pairing: PairingInfo {
            server: server_info(),
            certificate_pem,
            fingerprint_sha256,
        },
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/pairing", get(pairing))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/password", put(change_password))
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/users/{id}", put(update_user))
        .route("/api/v1/users/{id}/password", put(reset_user_password))
        .route("/api/v1/miners", get(list_miners).post(create_miner))
        .route("/api/v1/miners/import", post(import_miners))
        .route(
            "/api/v1/miners/{id}",
            put(update_miner).delete(delete_miner),
        )
        .route("/api/v1/parts", get(list_parts).post(create_part))
        .route("/api/v1/parts/{sku}", put(update_part).delete(delete_part))
        .route("/api/v1/dashboard", get(dashboard))
        .layer(RequestBodyLimitLayer::new(30 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(
        &config.tls.certificate,
        &config.tls.private_key,
    )
    .await?;
    tracing::info!(listen = %config.listen, "starting HTTPS server");
    axum_server::bind_rustls(config.listen, tls)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}

async fn health() -> Json<ServerInfo> {
    Json(server_info())
}

fn server_info() -> ServerInfo {
    ServerInfo {
        product: "Antminer Fleet Server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        api_version: API_VERSION.into(),
    }
}

async fn pairing(State(state): State<AppState>) -> Json<PairingInfo> {
    Json(state.pairing)
}

async fn authenticated_user(state: &AppState, headers: &HeaderMap) -> AppResult<(User, String)> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::unauthorized("authentication required"))?;
    let hash = token_hash(token);
    let row = sqlx::query(
        r#"
        SELECT u.id, u.username, u.display_name, u.role, u.enabled, u.version
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > NOW() AND u.enabled = TRUE
        "#,
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::unauthorized("session expired or invalid"))?;
    Ok((user_from_row(&row), hash))
}

async fn require_admin(state: &AppState, headers: &HeaderMap) -> AppResult<User> {
    let (user, _) = authenticated_user(state, headers).await?;
    if user.role != UserRole::Admin {
        return Err(AppError::forbidden("administrator access required"));
    }
    Ok(user)
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(input): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let username = normalize_username(&input.username);
    if username.is_empty() {
        return Err(AppError::bad_request("username is required"));
    }
    {
        let mut limiter = state.login_limiter.lock().await;
        if !limiter.allow(remote.ip(), &username, Instant::now()) {
            return Err(AppError {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "rate_limited",
                message: "too many login attempts; try again in one minute".into(),
            });
        }
    }

    let row = sqlx::query(
        "SELECT id, username, display_name, password_hash, role, enabled, version FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?;
    let password_hash = row
        .as_ref()
        .map(|row| row.get::<String, _>("password_hash"))
        .unwrap_or_else(|| state.dummy_password_hash.clone());
    let password_valid = verify_password(&input.password, &password_hash);
    let Some(row) = row else {
        return Err(AppError::unauthorized("invalid username or password"));
    };
    if !row.get::<bool, _>("enabled") || !password_valid {
        return Err(AppError::unauthorized("invalid username or password"));
    }

    state
        .login_limiter
        .lock()
        .await
        .clear(remote.ip(), &username);
    let user = user_from_row(&row);
    let token = new_token();
    let expires_at = Utc::now() + Duration::days(state.session_days);
    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(token_hash(&token))
        .bind(expires_at)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(LoginResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
        user,
    }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<StatusCode> {
    let (_, hash) = authenticated_user(&state, &headers).await?;
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE token_hash = $1")
        .bind(hash)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<User>> {
    authenticated_user(&state, &headers)
        .await
        .map(|(user, _)| Json(user))
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ChangePasswordRequest>,
) -> AppResult<StatusCode> {
    validate_password(&input.new_password).map_err(AppError::bad_request)?;
    let (user, _) = authenticated_user(&state, &headers).await?;
    let current_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(AppError::database)?;
    if !verify_password(&input.current_password, &current_hash) {
        return Err(AppError::unauthorized("current password is incorrect"));
    }
    let password_hash = hash_password(&input.new_password).map_err(AppError::bad_request)?;
    let mut tx = state.pool.begin().await.map_err(AppError::database)?;
    sqlx::query("UPDATE users SET password_hash = $1, version = version + 1, updated_at = NOW() WHERE id = $2")
        .bind(password_hash)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::database)?;
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::database)?;
    tx.commit().await.map_err(AppError::database)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<User>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, username, display_name, role, enabled, version FROM users ORDER BY username",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(user_from_row).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<User>)> {
    require_admin(&state, &headers).await?;
    let username = normalize_username(&input.username);
    if username.is_empty() || input.display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "username and display name are required",
        ));
    }
    let password_hash = hash_password(&input.password).map_err(AppError::bad_request)?;
    let row = sqlx::query(
        "INSERT INTO users (username, display_name, password_hash, role) VALUES ($1,$2,$3,$4) RETURNING id, username, display_name, role, enabled, version",
    )
    .bind(username)
    .bind(input.display_name.trim())
    .bind(password_hash)
    .bind(input.role.as_str())
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok((StatusCode::CREATED, Json(user_from_row(&row))))
}

async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await.map_err(AppError::database)?;
    sqlx::query("SELECT pg_advisory_xact_lock(807311001)")
        .execute(&mut *tx)
        .await
        .map_err(AppError::database)?;
    let existing_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM users WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::database)?;
    let existing_role = existing_role.ok_or_else(|| AppError::not_found("user not found"))?;
    if existing_role == "admin" && (input.role != UserRole::Admin || !input.enabled) {
        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND enabled = TRUE",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::database)?;
        if admin_count <= 1 {
            return Err(AppError::bad_request(
                "the final enabled administrator cannot be disabled or demoted",
            ));
        }
    }
    let row = sqlx::query(
        "UPDATE users SET display_name=$1, role=$2, enabled=$3, version=version+1, updated_at=NOW() WHERE id=$4 AND version=$5 RETURNING id, username, display_name, role, enabled, version",
    )
    .bind(input.display_name.trim())
    .bind(input.role.as_str())
    .bind(input.enabled)
    .bind(id)
    .bind(input.version)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("user changed; reload and try again"))?;
    if !input.enabled {
        sqlx::query("UPDATE sessions SET revoked_at=NOW() WHERE user_id=$1 AND revoked_at IS NULL")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::database)?;
    }
    tx.commit().await.map_err(AppError::database)?;
    Ok(Json(user_from_row(&row)))
}

async fn reset_user_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ResetPasswordRequest>,
) -> AppResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let password_hash = hash_password(&input.password).map_err(AppError::bad_request)?;
    let mut tx = state.pool.begin().await.map_err(AppError::database)?;
    let result = sqlx::query(
        "UPDATE users SET password_hash=$1, version=version+1, updated_at=NOW() WHERE id=$2",
    )
    .bind(password_hash)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("user not found"));
    }
    sqlx::query("UPDATE sessions SET revoked_at=NOW() WHERE user_id=$1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::database)?;
    tx.commit().await.map_err(AppError::database)?;
    Ok(StatusCode::NO_CONTENT)
}

fn miner_from_row(row: &sqlx::postgres::PgRow) -> Miner {
    Miner {
        id: row.get("id"),
        serial: row.get("serial"),
        model: row.get("model"),
        firmware: row.get("firmware"),
        client_name: row.get("client_name"),
        miner_type: row.get("miner_type"),
        ip_address: row.get("ip_address"),
        mac_address: row.get("mac_address"),
        pickaxe: row.get("pickaxe"),
        miner_state: row.get("miner_state"),
        miner_row: row.get("miner_row"),
        miner_index: row.get("miner_index"),
        miner_rack: row.get("miner_rack"),
        miner_rack_group: row.get("miner_rack_group"),
        location: row.get("location"),
        status: row.get("status"),
        acquired_date: row.get("acquired_date"),
        notes: row.get("notes"),
        version: row.get("version"),
    }
}

const MINER_COLUMNS: &str = "id, serial, model, firmware, client_name, miner_type, ip_address, mac_address, pickaxe, miner_state, miner_row, miner_index, miner_rack, miner_rack_group, location, status, acquired_date, notes, version";

async fn list_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Miner>>> {
    authenticated_user(&state, &headers).await?;
    let rows = sqlx::query(&format!(
        "SELECT {MINER_COLUMNS} FROM miners ORDER BY serial"
    ))
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(miner_from_row).collect()))
}

async fn create_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut input): Json<CreateMiner>,
) -> AppResult<(StatusCode, Json<Miner>)> {
    authenticated_user(&state, &headers).await?;
    normalize_and_validate_miner(&mut input).map_err(AppError::bad_request)?;
    let row = sqlx::query(&format!(
        "INSERT INTO miners (serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17) RETURNING {MINER_COLUMNS}"
    ))
    .bind(input.serial).bind(input.model).bind(input.firmware).bind(input.client_name)
    .bind(input.miner_type).bind(input.ip_address).bind(input.mac_address).bind(input.pickaxe)
    .bind(input.miner_state).bind(input.miner_row).bind(input.miner_index).bind(input.miner_rack)
    .bind(input.miner_rack_group).bind(input.location).bind(input.status).bind(input.acquired_date)
    .bind(input.notes).fetch_one(&state.pool).await.map_err(AppError::database)?;
    Ok((StatusCode::CREATED, Json(miner_from_row(&row))))
}

async fn update_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateMiner>,
) -> AppResult<Json<Miner>> {
    authenticated_user(&state, &headers).await?;
    if input.id != id {
        return Err(AppError::bad_request("path and body miner IDs differ"));
    }
    let mut validated = CreateMiner {
        serial: input.serial,
        model: input.model,
        firmware: input.firmware,
        client_name: input.client_name,
        miner_type: input.miner_type,
        ip_address: input.ip_address,
        mac_address: input.mac_address,
        pickaxe: input.pickaxe,
        miner_state: input.miner_state,
        miner_row: input.miner_row,
        miner_index: input.miner_index,
        miner_rack: input.miner_rack,
        miner_rack_group: input.miner_rack_group,
        location: input.location,
        status: input.status,
        acquired_date: input.acquired_date,
        notes: input.notes,
    };
    normalize_and_validate_miner(&mut validated).map_err(AppError::bad_request)?;
    let row = sqlx::query(&format!(
        "UPDATE miners SET serial=$1,model=$2,firmware=$3,client_name=$4,miner_type=$5,ip_address=$6,mac_address=$7,pickaxe=$8,miner_state=$9,miner_row=$10,miner_index=$11,miner_rack=$12,miner_rack_group=$13,location=$14,status=$15,acquired_date=$16,notes=$17,version=version+1,updated_at=NOW() WHERE id=$18 AND version=$19 RETURNING {MINER_COLUMNS}"
    ))
    .bind(validated.serial).bind(validated.model).bind(validated.firmware).bind(validated.client_name)
    .bind(validated.miner_type).bind(validated.ip_address).bind(validated.mac_address).bind(validated.pickaxe)
    .bind(validated.miner_state).bind(validated.miner_row).bind(validated.miner_index).bind(validated.miner_rack)
    .bind(validated.miner_rack_group).bind(validated.location).bind(validated.status).bind(validated.acquired_date)
    .bind(validated.notes).bind(id).bind(input.version)
    .fetch_optional(&state.pool).await.map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("miner changed or was deleted; reload and try again"))?;
    Ok(Json(miner_from_row(&row)))
}

async fn delete_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    authenticated_user(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM miners WHERE id=$1 AND version=$2")
        .bind(id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "miner changed or was deleted; reload and try again",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn import_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(inputs): Json<Vec<CreateMiner>>,
) -> AppResult<Json<MinerImportResult>> {
    require_admin(&state, &headers).await?;
    let mut seen = HashSet::new();
    let mut miners = Vec::with_capacity(inputs.len());
    let mut skipped = 0;
    for mut miner in inputs {
        miner.serial = miner.serial.trim().to_string();
        if miner.serial.is_empty() || !seen.insert(miner.serial.clone()) {
            skipped += 1;
            continue;
        }
        normalize_and_validate_miner(&mut miner).map_err(AppError::bad_request)?;
        miners.push(miner);
    }
    let mut tx = state.pool.begin().await.map_err(AppError::database)?;
    let mut imported = 0;
    let mut conflicts = Vec::new();
    for miner in miners {
        let serial = miner.serial.clone();
        let inserted: Option<i64> = sqlx::query_scalar(
            "INSERT INTO miners (serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17) ON CONFLICT (serial) DO NOTHING RETURNING id",
        )
        .bind(miner.serial).bind(miner.model).bind(miner.firmware).bind(miner.client_name)
        .bind(miner.miner_type).bind(miner.ip_address).bind(miner.mac_address).bind(miner.pickaxe)
        .bind(miner.miner_state).bind(miner.miner_row).bind(miner.miner_index).bind(miner.miner_rack)
        .bind(miner.miner_rack_group).bind(miner.location).bind(miner.status).bind(miner.acquired_date)
        .bind(miner.notes).fetch_optional(&mut *tx).await.map_err(AppError::database)?;
        if inserted.is_some() {
            imported += 1;
        } else {
            skipped += 1;
            conflicts.push(serial);
        }
    }
    tx.commit().await.map_err(AppError::database)?;
    Ok(Json(MinerImportResult {
        imported,
        updated: 0,
        skipped,
        conflicts,
    }))
}

fn part_from_row(row: &sqlx::postgres::PgRow) -> Part {
    Part {
        sku: row.get("sku"),
        name: row.get("name"),
        category: row.get("category"),
        qty_on_hand: row.get("qty_on_hand"),
        reorder_threshold: row.get("reorder_threshold"),
        supplier: row.get("supplier"),
        unit_cost_cents: row.get("unit_cost_cents"),
        notes: row.get("notes"),
        version: row.get("version"),
    }
}

const PART_COLUMNS: &str =
    "sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes,version";

async fn list_parts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Part>>> {
    authenticated_user(&state, &headers).await?;
    let rows = sqlx::query(&format!("SELECT {PART_COLUMNS} FROM parts ORDER BY name"))
        .fetch_all(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(part_from_row).collect()))
}

async fn create_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut input): Json<CreatePart>,
) -> AppResult<(StatusCode, Json<Part>)> {
    authenticated_user(&state, &headers).await?;
    input.sku = input.sku.trim().to_string();
    validate_part(&input).map_err(AppError::bad_request)?;
    let row = sqlx::query(&format!(
        "INSERT INTO parts (sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING {PART_COLUMNS}"
    ))
    .bind(input.sku).bind(input.name.trim()).bind(input.category).bind(input.qty_on_hand)
    .bind(input.reorder_threshold).bind(input.supplier).bind(input.unit_cost_cents).bind(input.notes)
    .fetch_one(&state.pool).await.map_err(AppError::database)?;
    Ok((StatusCode::CREATED, Json(part_from_row(&row))))
}

async fn update_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(sku): Path<String>,
    Json(input): Json<Part>,
) -> AppResult<Json<Part>> {
    authenticated_user(&state, &headers).await?;
    if input.sku != sku {
        return Err(AppError::bad_request("path and body SKUs differ"));
    }
    validate_part(&CreatePart {
        sku: input.sku.clone(),
        name: input.name.clone(),
        category: input.category.clone(),
        qty_on_hand: input.qty_on_hand,
        reorder_threshold: input.reorder_threshold,
        supplier: input.supplier.clone(),
        unit_cost_cents: input.unit_cost_cents,
        notes: input.notes.clone(),
    })
    .map_err(AppError::bad_request)?;
    let row = sqlx::query(&format!(
        "UPDATE parts SET name=$1,category=$2,qty_on_hand=$3,reorder_threshold=$4,supplier=$5,unit_cost_cents=$6,notes=$7,version=version+1,updated_at=NOW() WHERE sku=$8 AND version=$9 RETURNING {PART_COLUMNS}"
    ))
    .bind(input.name.trim()).bind(input.category).bind(input.qty_on_hand).bind(input.reorder_threshold)
    .bind(input.supplier).bind(input.unit_cost_cents).bind(input.notes).bind(&sku).bind(input.version)
    .fetch_optional(&state.pool).await.map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("part changed or was deleted; reload and try again"))?;
    Ok(Json(part_from_row(&row)))
}

async fn delete_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(sku): Path<String>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    authenticated_user(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM parts WHERE sku=$1 AND version=$2")
        .bind(sku)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "part changed or was deleted; reload and try again",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<DashboardSummary>> {
    authenticated_user(&state, &headers).await?;
    let counts = sqlx::query(
        "SELECT (SELECT COUNT(*) FROM miners) unit_count, (SELECT COUNT(*) FROM parts) part_count, (SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold) low_stock_count",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    let statuses =
        sqlx::query("SELECT status, COUNT(*) count FROM miners GROUP BY status ORDER BY status")
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?;
    let parts = sqlx::query(&format!(
        "SELECT {PART_COLUMNS} FROM parts WHERE qty_on_hand <= reorder_threshold ORDER BY qty_on_hand, name LIMIT 10"
    ))
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(DashboardSummary {
        unit_count: counts.get("unit_count"),
        part_count: counts.get("part_count"),
        low_stock_count: counts.get("low_stock_count"),
        units_by_status: statuses
            .iter()
            .map(|row| CountByStatus {
                status: row.get("status"),
                count: row.get("count"),
            })
            .collect(),
        low_stock_parts: parts.iter().map(part_from_row).collect(),
    }))
}

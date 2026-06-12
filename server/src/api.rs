use crate::{
    auth::{hash_password, new_token, token_hash, user_from_row, verify_password},
    config::ServerConfig,
};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header::AUTHORIZATION, header::USER_AGENT, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use fleet_shared::{
    normalize_and_validate_miner, normalize_username, validate_part, validate_password, ApiError,
    AuditLogEntry, AuditLogQuery, ChangePasswordRequest, CountByStatus, CreateMiner, CreatePart,
    CreateSite, CreateUserRequest, CreateWebhook, DashboardSummary, LoginRequest, LoginResponse,
    Miner, MinerImportResult, PairingInfo, Part, ResetPasswordRequest, ServerInfo, Site,
    SiteQuery, UpdateMiner, UpdateSite, UpdateUserRequest, UpdateWebhook, User, UserRole,
    Webhook, WebhookDelivery, API_VERSION,
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
    webhook_client: reqwest::Client,
    session_days: i64,
    login_limiter: Arc<Mutex<LoginLimiter>>,
    dummy_password_hash: String,
    pairing: PairingInfo,
}

impl AppState {
    async fn audit_log(
        &self,
        user_id: Option<i64>,
        username: Option<String>,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<&str>,
        target_serial: Option<&str>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        headers: &HeaderMap,
        remote: SocketAddr,
    ) {
        let ip_address = remote.ip().to_string();
        let user_agent = headers
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let insert_result = sqlx::query(
            r#"
            INSERT INTO audit_log (user_id, username, action, target_type, target_id, target_serial, old_values, new_values, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(user_id)
        .bind(&username)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(target_serial)
        .bind(&old_values)
        .bind(&new_values)
        .bind(&ip_address)
        .bind(user_agent)
        .execute(&self.pool)
        .await;

        if insert_result.is_ok() {
            self.dispatch_webhooks(
                action,
                serde_json::json!({
                    "action": action,
                    "user_id": user_id,
                    "username": username,
                    "target_type": target_type,
                    "target_id": target_id,
                    "target_serial": target_serial,
                    "old_values": old_values,
                    "new_values": new_values,
                    "ip_address": ip_address,
                }),
            )
            .await;
        }
    }

    async fn dispatch_webhooks(&self, action: &str, payload: serde_json::Value) {
        let Some(event) = webhook_event_for_action(action) else {
            return;
        };
        let rows = match sqlx::query(
            "SELECT id, name, url, secret, events FROM webhooks WHERE enabled = TRUE AND $1 = ANY(events)",
        )
        .bind(event)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(error = %error, "failed to load webhooks");
                return;
            }
        };

        let body = match serde_json::to_string(&payload) {
            Ok(body) => body,
            Err(error) => {
                tracing::warn!(error = %error, "failed to serialize webhook payload");
                return;
            }
        };

        for row in rows {
            let webhook_id: i64 = row.get("id");
            let url: String = row.get("url");
            let secret: Option<String> = row.get("secret");
            let mut request = self
                .webhook_client
                .post(&url)
                .header("content-type", "application/json")
                .header("x-fleet-event", event)
                .body(body.clone());
            if let Some(secret) = secret.as_deref().filter(|value| !value.is_empty()) {
                let signature = format!("sha256={:x}", Sha256::digest(format!("{secret}{body}").as_bytes()));
                request = request.header("x-fleet-signature", signature);
            }

            let started = Utc::now();
            let result = request.send().await;
            let (success, status, response_body, error) = match result {
                Ok(response) => {
                    let status = response.status().as_u16() as i32;
                    let success = response.status().is_success();
                    let text = response.text().await.unwrap_or_default();
                    (success, Some(status), Some(text.chars().take(2000).collect::<String>()), None)
                }
                Err(error) => (false, None, None, Some(error.to_string())),
            };

            let _ = sqlx::query(
                "INSERT INTO webhook_deliveries (webhook_id, event, payload, response_status, response_body, success, error, delivered_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(webhook_id)
            .bind(event)
            .bind(&payload)
            .bind(status)
            .bind(response_body)
            .bind(success)
            .bind(error)
            .bind(started)
            .execute(&self.pool)
            .await;
        }
    }
}

fn webhook_event_for_action(action: &str) -> Option<&'static str> {
    match action {
        "create_miner" | "import_miners" => Some("miner.created"),
        "update_miner" => Some("miner.updated"),
        "delete_miner" => Some("miner.deleted"),
        "create_part" => Some("part.created"),
        "update_part" => Some("part.updated"),
        "delete_part" => Some("part.deleted"),
        "create_user" => Some("user.created"),
        "update_user" | "reset_user_password" | "change_password" => Some("user.updated"),
        _ => None,
    }
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
    site_id: Option<i64>,
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
        webhook_client: reqwest::Client::builder()
            .timeout(StdDuration::from_secs(10))
            .build()?,
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
        .route("/api/v1/audit-log", get(list_audit_log))
        .route("/api/v1/webhooks", get(list_webhooks).post(create_webhook))
        .route(
            "/api/v1/webhooks/{id}",
            put(update_webhook).delete(delete_webhook),
        )
        .route("/api/v1/webhooks/{id}/deliveries", get(list_webhook_deliveries))
        .route("/api/v1/sites", get(list_sites).post(create_site))
        .route("/api/v1/sites/{id}", put(update_site).delete(delete_site))
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
        SELECT u.id, u.site_id, s.name site_name, u.username, u.display_name, u.role, u.enabled, u.version
        FROM sessions se
        JOIN users u ON u.id = se.user_id
        LEFT JOIN sites s ON s.id = u.site_id
        WHERE se.token_hash = $1 AND se.revoked_at IS NULL AND se.expires_at > NOW() AND u.enabled = TRUE
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
    headers: HeaderMap,
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
        "SELECT u.id, u.site_id, s.name site_name, u.username, u.display_name, u.password_hash, u.role, u.enabled, u.version FROM users u LEFT JOIN sites s ON s.id = u.site_id WHERE u.username = $1",
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
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "login",
            Some("user"),
            Some(&user.id.to_string()),
            None,
            None,
            None,
            &headers,
            remote,
        )
        .await;
    Ok(Json(LoginResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
        user,
    }))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
) -> AppResult<StatusCode> {
    let (user, hash) = authenticated_user(&state, &headers).await?;
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE token_hash = $1")
        .bind(hash)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "logout",
            Some("user"),
            Some(&user.id.to_string()),
            None,
            None,
            None,
            &headers,
            remote,
        )
        .await;
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
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
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
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "change_password",
            Some("user"),
            Some(&user.id.to_string()),
            None,
            None,
            None,
            &headers,
            remote,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<User>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.site_id, s.name site_name, u.username, u.display_name, u.role, u.enabled, u.version FROM users u LEFT JOIN sites s ON s.id = u.site_id ORDER BY u.username",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(user_from_row).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(input): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<User>)> {
    let acting_user = require_admin(&state, &headers).await?;
    let username = normalize_username(&input.username);
    if username.is_empty() || input.display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "username and display name are required",
        ));
    }
    let password_hash = hash_password(&input.password).map_err(AppError::bad_request)?;
    let row = sqlx::query(
        "INSERT INTO users (site_id, username, display_name, password_hash, role) VALUES ($1,$2,$3,$4,$5) RETURNING id, site_id, NULL::TEXT site_name, username, display_name, role, enabled, version",
    )
    .bind(input.site_id)
    .bind(username)
    .bind(input.display_name.trim())
    .bind(password_hash)
    .bind(input.role.as_str())
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    let new_user = user_from_row(&row);
    state
        .audit_log(
            Some(acting_user.id),
            Some(acting_user.username.clone()),
            "create_user",
            Some("user"),
            Some(&new_user.id.to_string()),
            None,
            None,
            Some(serde_json::json!({
                "username": new_user.username,
                "display_name": new_user.display_name,
                "role": new_user.role,
                "enabled": new_user.enabled
            })),
            &headers,
            remote,
        )
        .await;
    Ok((StatusCode::CREATED, Json(new_user)))
}

async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    require_admin(&state, &headers).await?;
    let (acting_user, _) = authenticated_user(&state, &headers).await?;
    let mut tx = state.pool.begin().await.map_err(AppError::database)?;
    sqlx::query("SELECT pg_advisory_xact_lock(807311001)")
        .execute(&mut *tx)
        .await
        .map_err(AppError::database)?;
    let existing: Option<(Option<i64>, String, String, String, bool)> = sqlx::query_as(
        "SELECT site_id, username, display_name, role, enabled FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::database)?;
    let (existing_site_id, existing_username, existing_display_name, existing_role, existing_enabled) =
        existing.ok_or_else(|| AppError::not_found("user not found"))?;
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
        "UPDATE users SET site_id=$1, display_name=$2, role=$3, enabled=$4, version=version+1, updated_at=NOW() WHERE id=$5 AND version=$6 RETURNING id, site_id, NULL::TEXT site_name, username, display_name, role, enabled, version",
    )
    .bind(input.site_id)
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
    let updated_user = user_from_row(&row);
    state
        .audit_log(
            Some(acting_user.id),
            Some(acting_user.username.clone()),
            "update_user",
            Some("user"),
            Some(&updated_user.id.to_string()),
            None,
            Some(serde_json::json!({
                "site_id": existing_site_id,
                "username": existing_username,
                "display_name": existing_display_name,
                "role": existing_role,
                "enabled": existing_enabled
            })),
            Some(serde_json::json!({
                "site_id": updated_user.site_id,
                "username": updated_user.username,
                "display_name": updated_user.display_name,
                "role": updated_user.role,
                "enabled": updated_user.enabled
            })),
            &headers,
            remote,
        )
        .await;
    Ok(Json(updated_user))
}

async fn reset_user_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Json(input): Json<ResetPasswordRequest>,
) -> AppResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let (acting_user, _) = authenticated_user(&state, &headers).await?;
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
    state
        .audit_log(
            Some(acting_user.id),
            Some(acting_user.username.clone()),
            "reset_user_password",
            Some("user"),
            Some(&id.to_string()),
            None,
            None,
            None,
            &headers,
            remote,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

fn miner_from_row(row: &sqlx::postgres::PgRow) -> Miner {
    Miner {
        id: row.get("id"),
        site_id: row.get("site_id"),
        site_name: row.try_get("site_name").unwrap_or(None),
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

const MINER_COLUMNS: &str = "m.id, m.site_id, s.name site_name, m.serial, m.model, m.firmware, m.client_name, m.miner_type, m.ip_address, m.mac_address, m.pickaxe, m.miner_state, m.miner_row, m.miner_index, m.miner_rack, m.miner_rack_group, m.location, m.status, m.acquired_date, m.notes, m.version";

async fn list_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<Vec<Miner>>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = query.site_id.or(user.site_id);
    let rows = sqlx::query(&format!(
        "SELECT {MINER_COLUMNS} FROM miners m JOIN sites s ON s.id = m.site_id WHERE ($1::BIGINT IS NULL OR m.site_id = $1) ORDER BY m.serial"
    ))
    .bind(site_id)
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(miner_from_row).collect()))
}

async fn fetch_miner_by_id(state: &AppState, id: i64) -> AppResult<Miner> {
    let row = sqlx::query(&format!(
        "SELECT {MINER_COLUMNS} FROM miners m JOIN sites s ON s.id = m.site_id WHERE m.id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("miner not found"))?;
    Ok(miner_from_row(&row))
}

async fn default_site_id(state: &AppState) -> AppResult<i64> {
    sqlx::query_scalar("SELECT id FROM sites WHERE enabled = TRUE ORDER BY id LIMIT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(AppError::database)
}

async fn create_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(mut input): Json<CreateMiner>,
) -> AppResult<(StatusCode, Json<Miner>)> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    normalize_and_validate_miner(&mut input).map_err(AppError::bad_request)?;
    let site_id = input.site_id.or(user.site_id).unwrap_or(default_site_id(&state).await?);
    let miner_id: i64 = sqlx::query_scalar(
        "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) RETURNING id"
    )
    .bind(site_id).bind(input.serial).bind(input.model).bind(input.firmware).bind(input.client_name)
    .bind(input.miner_type).bind(input.ip_address).bind(input.mac_address).bind(input.pickaxe)
    .bind(input.miner_state).bind(input.miner_row).bind(input.miner_index).bind(input.miner_rack)
    .bind(input.miner_rack_group).bind(input.location).bind(input.status).bind(input.acquired_date)
    .bind(input.notes).fetch_one(&state.pool).await.map_err(AppError::database)?;
    let new_miner = fetch_miner_by_id(&state, miner_id).await?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "create_miner",
            Some("miner"),
            Some(&new_miner.id.to_string()),
            Some(&new_miner.serial),
            None,
            Some(serde_json::json!({
                "serial": new_miner.serial,
                "model": new_miner.model,
                "status": new_miner.status,
                "location": new_miner.location
            })),
            &headers,
            remote,
        )
        .await;
    Ok((StatusCode::CREATED, Json(new_miner)))
}

async fn update_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateMiner>,
) -> AppResult<Json<Miner>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    if input.id != id {
        return Err(AppError::bad_request("path and body miner IDs differ"));
    }
    let existing = sqlx::query(&format!("SELECT {MINER_COLUMNS} FROM miners m JOIN sites s ON s.id = m.site_id WHERE m.id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?;
    let existing_miner = existing
        .as_ref()
        .map(miner_from_row)
        .ok_or_else(|| AppError::not_found("miner not found"))?;
    let mut validated = CreateMiner {
        site_id: input.site_id.or(Some(existing_miner.site_id)),
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
    let site_id = validated.site_id.unwrap_or(existing_miner.site_id);
    let miner_id: i64 = sqlx::query_scalar(
        "UPDATE miners SET site_id=$1,serial=$2,model=$3,firmware=$4,client_name=$5,miner_type=$6,ip_address=$7,mac_address=$8,pickaxe=$9,miner_state=$10,miner_row=$11,miner_index=$12,miner_rack=$13,miner_rack_group=$14,location=$15,status=$16,acquired_date=$17,notes=$18,version=version+1,updated_at=NOW() WHERE id=$19 AND version=$20 RETURNING id"
    )
    .bind(site_id).bind(validated.serial).bind(validated.model).bind(validated.firmware).bind(validated.client_name)
    .bind(validated.miner_type).bind(validated.ip_address).bind(validated.mac_address).bind(validated.pickaxe)
    .bind(validated.miner_state).bind(validated.miner_row).bind(validated.miner_index).bind(validated.miner_rack)
    .bind(validated.miner_rack_group).bind(validated.location).bind(validated.status).bind(validated.acquired_date)
    .bind(validated.notes).bind(id).bind(input.version)
    .fetch_optional(&state.pool).await.map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("miner changed or was deleted; reload and try again"))?;
    let updated_miner = fetch_miner_by_id(&state, miner_id).await?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "update_miner",
            Some("miner"),
            Some(&updated_miner.id.to_string()),
            Some(&updated_miner.serial),
            Some(serde_json::json!({
                "serial": existing_miner.serial,
                "model": existing_miner.model,
                "status": existing_miner.status,
                "location": existing_miner.location
            })),
            Some(serde_json::json!({
                "serial": updated_miner.serial,
                "model": updated_miner.model,
                "status": updated_miner.status,
                "location": updated_miner.location
            })),
            &headers,
            remote,
        )
        .await;
    Ok(Json(updated_miner))
}

async fn delete_miner(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let existing = sqlx::query(&format!("SELECT {MINER_COLUMNS} FROM miners m JOIN sites s ON s.id = m.site_id WHERE m.id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?;
    let existing_miner = existing
        .as_ref()
        .map(miner_from_row)
        .ok_or_else(|| AppError::not_found("miner not found"))?;
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
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "delete_miner",
            Some("miner"),
            Some(&id.to_string()),
            Some(&existing_miner.serial),
            Some(serde_json::json!({
                "serial": existing_miner.serial,
                "model": existing_miner.model,
                "status": existing_miner.status,
                "location": existing_miner.location
            })),
            None,
            &headers,
            remote,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn import_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(inputs): Json<Vec<CreateMiner>>,
) -> AppResult<Json<MinerImportResult>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
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
    let mut imported_serials = Vec::new();
    let import_site_id = user.site_id.unwrap_or(default_site_id(&state).await?);
    for miner in miners {
        let serial = miner.serial.clone();
        let inserted: Option<i64> = sqlx::query_scalar(
            "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) ON CONFLICT (site_id, serial) DO NOTHING RETURNING id",
        )
        .bind(miner.site_id.unwrap_or(import_site_id)).bind(miner.serial).bind(miner.model).bind(miner.firmware).bind(miner.client_name)
        .bind(miner.miner_type).bind(miner.ip_address).bind(miner.mac_address).bind(miner.pickaxe)
        .bind(miner.miner_state).bind(miner.miner_row).bind(miner.miner_index).bind(miner.miner_rack)
        .bind(miner.miner_rack_group).bind(miner.location).bind(miner.status).bind(miner.acquired_date)
        .bind(miner.notes).fetch_optional(&mut *tx).await.map_err(AppError::database)?;
        if inserted.is_some() {
            imported += 1;
            imported_serials.push(serial);
        } else {
            skipped += 1;
            conflicts.push(serial);
        }
    }
    tx.commit().await.map_err(AppError::database)?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "import_miners",
            Some("miner"),
            None,
            None,
            None,
            Some(serde_json::json!({
                "imported": imported,
                "skipped": skipped,
                "conflicts": conflicts,
                "imported_serials": imported_serials
            })),
            &headers,
            remote,
        )
        .await;
    Ok(Json(MinerImportResult {
        imported,
        updated: 0,
        skipped,
        conflicts,
    }))
}

fn part_from_row(row: &sqlx::postgres::PgRow) -> Part {
    Part {
        site_id: row.get("site_id"),
        site_name: row.try_get("site_name").unwrap_or(None),
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
    "p.site_id, s.name site_name, p.sku, p.name, p.category, p.qty_on_hand, p.reorder_threshold, p.supplier, p.unit_cost_cents, p.notes, p.version";

async fn list_parts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<Vec<Part>>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = query.site_id.or(user.site_id);
    let rows = sqlx::query(&format!("SELECT {PART_COLUMNS} FROM parts p JOIN sites s ON s.id = p.site_id WHERE ($1::BIGINT IS NULL OR p.site_id = $1) ORDER BY p.name"))
        .bind(site_id)
        .fetch_all(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(part_from_row).collect()))
}

async fn fetch_part_by_sku(state: &AppState, sku: &str, site_id: i64) -> AppResult<Part> {
    let row = sqlx::query(&format!("SELECT {PART_COLUMNS} FROM parts p JOIN sites s ON s.id = p.site_id WHERE p.sku = $1 AND p.site_id = $2"))
        .bind(sku)
        .bind(site_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("part not found"))?;
    Ok(part_from_row(&row))
}

async fn create_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(mut input): Json<CreatePart>,
) -> AppResult<(StatusCode, Json<Part>)> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    input.sku = input.sku.trim().to_string();
    validate_part(&input).map_err(AppError::bad_request)?;
    let site_id = input.site_id.or(user.site_id).unwrap_or(default_site_id(&state).await?);
    sqlx::query(
        "INSERT INTO parts (site_id,sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"
    )
    .bind(site_id).bind(&input.sku).bind(input.name.trim()).bind(input.category).bind(input.qty_on_hand)
    .bind(input.reorder_threshold).bind(input.supplier).bind(input.unit_cost_cents).bind(input.notes)
    .execute(&state.pool).await.map_err(AppError::database)?;
    let new_part = fetch_part_by_sku(&state, &input.sku, site_id).await?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "create_part",
            Some("part"),
            Some(&new_part.sku),
            None,
            None,
            Some(serde_json::json!({
                "sku": new_part.sku,
                "name": new_part.name,
                "category": new_part.category,
                "qty_on_hand": new_part.qty_on_hand
            })),
            &headers,
            remote,
        )
        .await;
    Ok((StatusCode::CREATED, Json(new_part)))
}

async fn update_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(sku): Path<String>,
    Json(input): Json<Part>,
) -> AppResult<Json<Part>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    if input.sku != sku {
        return Err(AppError::bad_request("path and body SKUs differ"));
    }
    let existing = sqlx::query(&format!("SELECT {PART_COLUMNS} FROM parts p JOIN sites s ON s.id = p.site_id WHERE p.sku = $1 AND p.site_id = $2"))
        .bind(&sku)
        .bind(input.site_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?;
    let existing_part = existing
        .as_ref()
        .map(part_from_row)
        .ok_or_else(|| AppError::not_found("part not found"))?;
    validate_part(&CreatePart {
        site_id: Some(input.site_id),
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
    sqlx::query(
        "UPDATE parts SET name=$1,category=$2,qty_on_hand=$3,reorder_threshold=$4,supplier=$5,unit_cost_cents=$6,notes=$7,version=version+1,updated_at=NOW() WHERE sku=$8 AND site_id=$9 AND version=$10"
    )
    .bind(input.name.trim()).bind(input.category).bind(input.qty_on_hand).bind(input.reorder_threshold)
    .bind(input.supplier).bind(input.unit_cost_cents).bind(input.notes).bind(&sku).bind(input.site_id).bind(input.version)
    .execute(&state.pool).await.map_err(AppError::database)?;
    let updated_part = fetch_part_by_sku(&state, &sku, input.site_id).await?;
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "update_part",
            Some("part"),
            Some(&updated_part.sku),
            None,
            Some(serde_json::json!({
                "sku": existing_part.sku,
                "name": existing_part.name,
                "category": existing_part.category,
                "qty_on_hand": existing_part.qty_on_hand,
                "reorder_threshold": existing_part.reorder_threshold,
                "unit_cost_cents": existing_part.unit_cost_cents
            })),
            Some(serde_json::json!({
                "sku": updated_part.sku,
                "name": updated_part.name,
                "category": updated_part.category,
                "qty_on_hand": updated_part.qty_on_hand,
                "reorder_threshold": updated_part.reorder_threshold,
                "unit_cost_cents": updated_part.unit_cost_cents
            })),
            &headers,
            remote,
        )
        .await;
    Ok(Json(updated_part))
}

async fn delete_part(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(sku): Path<String>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = query.site_id.or(user.site_id).unwrap_or(default_site_id(&state).await?);
    let existing = sqlx::query(&format!("SELECT {PART_COLUMNS} FROM parts p JOIN sites s ON s.id = p.site_id WHERE p.sku = $1 AND p.site_id = $2"))
        .bind(&sku)
        .bind(site_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?;
    let existing_part = existing
        .as_ref()
        .map(part_from_row)
        .ok_or_else(|| AppError::not_found("part not found"))?;
    let result = sqlx::query("DELETE FROM parts WHERE sku=$1 AND site_id=$2 AND version=$3")
        .bind(sku)
        .bind(site_id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "part changed or was deleted; reload and try again",
        ));
    }
    state
        .audit_log(
            Some(user.id),
            Some(user.username.clone()),
            "delete_part",
            Some("part"),
            Some(&existing_part.sku),
            None,
            Some(serde_json::json!({
                "sku": existing_part.sku,
                "name": existing_part.name,
                "category": existing_part.category
            })),
            None,
            &headers,
            remote,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<DashboardSummary>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = query.site_id.or(user.site_id);
    let counts = sqlx::query(
        "SELECT (SELECT COUNT(*) FROM miners WHERE ($1::BIGINT IS NULL OR site_id = $1)) unit_count, (SELECT COUNT(*) FROM parts WHERE ($1::BIGINT IS NULL OR site_id = $1)) part_count, (SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold AND ($1::BIGINT IS NULL OR site_id = $1)) low_stock_count",
    )
    .bind(site_id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    let statuses =
        sqlx::query("SELECT status, COUNT(*) count FROM miners WHERE ($1::BIGINT IS NULL OR site_id = $1) GROUP BY status ORDER BY status")
            .bind(site_id)
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?;
    let parts = sqlx::query(&format!(
        "SELECT {PART_COLUMNS} FROM parts p JOIN sites s ON s.id = p.site_id WHERE p.qty_on_hand <= p.reorder_threshold AND ($1::BIGINT IS NULL OR p.site_id = $1) ORDER BY p.qty_on_hand, p.name LIMIT 10"
    ))
    .bind(site_id)
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

async fn list_audit_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> AppResult<Json<Vec<AuditLogEntry>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, username, action, target_type, target_id, target_serial,
               old_values, new_values, ip_address, user_agent, created_at
        FROM audit_log
        WHERE ($1::BIGINT IS NULL OR user_id = $1)
          AND ($2::TEXT IS NULL OR action = $2)
          AND ($3::TEXT IS NULL OR target_type = $3)
          AND ($4::TEXT IS NULL OR target_id = $4)
          AND ($5::TEXT IS NULL OR created_at >= $5::TIMESTAMPTZ)
          AND ($6::TEXT IS NULL OR created_at <= $6::TIMESTAMPTZ)
        ORDER BY created_at DESC
        LIMIT $7 OFFSET $8
        "#,
    )
    .bind(query.user_id)
    .bind(query.action.filter(|value| !value.is_empty()))
    .bind(query.target_type.filter(|value| !value.is_empty()))
    .bind(query.target_id.filter(|value| !value.is_empty()))
    .bind(query.from.filter(|value| !value.is_empty()))
    .bind(query.to.filter(|value| !value.is_empty()))
    .bind(query.limit.unwrap_or(100).clamp(1, 1000))
    .bind(query.offset.unwrap_or(0).max(0))
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;

    Ok(Json(
        rows.iter()
            .map(|row| AuditLogEntry {
                id: row.get("id"),
                user_id: row.get("user_id"),
                username: row.get("username"),
                action: row.get("action"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                target_serial: row.get("target_serial"),
                old_values: row.get("old_values"),
                new_values: row.get("new_values"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            })
            .collect(),
    ))
}

fn webhook_from_row(row: &sqlx::postgres::PgRow) -> Webhook {
    Webhook {
        id: row.get("id"),
        name: row.get("name"),
        url: row.get("url"),
        secret: row.get::<Option<String>, _>("secret").map(|_| "********".to_string()),
        events: row.get("events"),
        enabled: row.get("enabled"),
        version: row.get("version"),
    }
}

async fn list_webhooks(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<Webhook>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query("SELECT id, name, url, secret, events, enabled, version FROM webhooks ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(webhook_from_row).collect()))
}

async fn create_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(input): Json<CreateWebhook>,
) -> AppResult<(StatusCode, Json<Webhook>)> {
    let user = require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() || input.url.trim().is_empty() {
        return Err(AppError::bad_request("webhook name and URL are required"));
    }
    let row = sqlx::query("INSERT INTO webhooks (name, url, secret, events, enabled) VALUES ($1,$2,$3,$4,$5) RETURNING id, name, url, secret, events, enabled, version")
        .bind(input.name.trim())
        .bind(input.url.trim())
        .bind(input.secret.filter(|value| !value.is_empty()))
        .bind(input.events)
        .bind(input.enabled)
        .fetch_one(&state.pool)
        .await
        .map_err(AppError::database)?;
    let webhook = webhook_from_row(&row);
    state.audit_log(Some(user.id), Some(user.username.clone()), "create_webhook", Some("webhook"), Some(&webhook.id.to_string()), None, None, Some(serde_json::json!({"name": webhook.name, "url": webhook.url, "events": webhook.events, "enabled": webhook.enabled})), &headers, remote).await;
    Ok((StatusCode::CREATED, Json(webhook)))
}

async fn update_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateWebhook>,
) -> AppResult<Json<Webhook>> {
    let user = require_admin(&state, &headers).await?;
    if input.id != id {
        return Err(AppError::bad_request("path and body webhook IDs differ"));
    }
    let existing = sqlx::query("SELECT name, url, events, enabled FROM webhooks WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("webhook not found"))?;
    let row = sqlx::query(
        "UPDATE webhooks SET name=$1, url=$2, secret=COALESCE($3, secret), events=$4, enabled=$5, version=version+1, updated_at=NOW() WHERE id=$6 AND version=$7 RETURNING id, name, url, secret, events, enabled, version",
    )
    .bind(input.name.trim())
    .bind(input.url.trim())
    .bind(input.secret.filter(|value| !value.is_empty() && value != "********"))
    .bind(input.events)
    .bind(input.enabled)
    .bind(id)
    .bind(input.version)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("webhook changed; reload and try again"))?;
    let webhook = webhook_from_row(&row);
    state.audit_log(Some(user.id), Some(user.username.clone()), "update_webhook", Some("webhook"), Some(&id.to_string()), None, Some(serde_json::json!({"name": existing.get::<String, _>("name"), "url": existing.get::<String, _>("url"), "events": existing.get::<Vec<String>, _>("events"), "enabled": existing.get::<bool, _>("enabled")})), Some(serde_json::json!({"name": webhook.name, "url": webhook.url, "events": webhook.events, "enabled": webhook.enabled})), &headers, remote).await;
    Ok(Json(webhook))
}

async fn delete_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    let user = require_admin(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM webhooks WHERE id=$1 AND version=$2")
        .bind(id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict("webhook changed or was deleted; reload and try again"));
    }
    state.audit_log(Some(user.id), Some(user.username.clone()), "delete_webhook", Some("webhook"), Some(&id.to_string()), None, None, None, &headers, remote).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_webhook_deliveries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<WebhookDelivery>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query("SELECT id, webhook_id, event, payload, response_status, response_body, success, error, attempts, created_at, delivered_at FROM webhook_deliveries WHERE webhook_id=$1 ORDER BY created_at DESC LIMIT 100")
        .bind(id)
        .fetch_all(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(|row| WebhookDelivery {
        id: row.get("id"),
        webhook_id: row.get("webhook_id"),
        event: row.get("event"),
        payload: row.get("payload"),
        response_status: row.get("response_status"),
        response_body: row.get("response_body"),
        success: row.get("success"),
        error: row.get("error"),
        attempts: row.get("attempts"),
        created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        delivered_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("delivered_at").map(|dt| dt.to_rfc3339()),
    }).collect()))
}

fn site_from_row(row: &sqlx::postgres::PgRow) -> Site {
    Site {
        id: row.get("id"),
        name: row.get("name"),
        code: row.get("code"),
        description: row.get("description"),
        enabled: row.get("enabled"),
        version: row.get("version"),
    }
}

async fn list_sites(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<Site>>> {
    authenticated_user(&state, &headers).await?;
    let rows = sqlx::query("SELECT id, name, code, description, enabled, version FROM sites ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(site_from_row).collect()))
}

async fn create_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Json(input): Json<CreateSite>,
) -> AppResult<(StatusCode, Json<Site>)> {
    let user = require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() || input.code.trim().is_empty() {
        return Err(AppError::bad_request("site name and code are required"));
    }
    let row = sqlx::query("INSERT INTO sites (name, code, description, enabled) VALUES ($1,$2,$3,$4) RETURNING id, name, code, description, enabled, version")
        .bind(input.name.trim())
        .bind(input.code.trim())
        .bind(input.description)
        .bind(input.enabled)
        .fetch_one(&state.pool)
        .await
        .map_err(AppError::database)?;
    let site = site_from_row(&row);
    state.audit_log(Some(user.id), Some(user.username.clone()), "create_site", Some("site"), Some(&site.id.to_string()), Some(&site.code), None, Some(serde_json::json!({"name": site.name, "code": site.code, "enabled": site.enabled})), &headers, remote).await;
    Ok((StatusCode::CREATED, Json(site)))
}

async fn update_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateSite>,
) -> AppResult<Json<Site>> {
    let user = require_admin(&state, &headers).await?;
    if input.id != id {
        return Err(AppError::bad_request("path and body site IDs differ"));
    }
    let row = sqlx::query("UPDATE sites SET name=$1, code=$2, description=$3, enabled=$4, version=version+1, updated_at=NOW() WHERE id=$5 AND version=$6 RETURNING id, name, code, description, enabled, version")
        .bind(input.name.trim())
        .bind(input.code.trim())
        .bind(input.description)
        .bind(input.enabled)
        .bind(id)
        .bind(input.version)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::conflict("site changed; reload and try again"))?;
    let site = site_from_row(&row);
    state.audit_log(Some(user.id), Some(user.username.clone()), "update_site", Some("site"), Some(&site.id.to_string()), Some(&site.code), None, Some(serde_json::json!({"name": site.name, "code": site.code, "enabled": site.enabled})), &headers, remote).await;
    Ok(Json(site))
}

async fn delete_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    let user = require_admin(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM sites WHERE id=$1 AND version=$2")
        .bind(id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict("site changed, is in use, or was deleted; reload and try again"));
    }
    state.audit_log(Some(user.id), Some(user.username.clone()), "delete_site", Some("site"), Some(&id.to_string()), None, None, None, &headers, remote).await;
    Ok(StatusCode::NO_CONTENT)
}


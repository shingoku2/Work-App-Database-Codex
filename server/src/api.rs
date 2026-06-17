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
    normalize_and_validate_miner, normalize_username, public_key_fingerprint_sha256,
    validate_part, validate_password, ApiError, ApproveTunnelKeyRequest, AuditLogEntry,
    AuditLogQuery, ChangePasswordRequest, CountByStatus, CreateMiner, CreatePart, CreateSite,
    CreateUserRequest, CreateWebhook, DashboardSummary, LoginRequest, LoginResponse, Miner,
    MinerImportResult, PairingInfo, Part, ResetPasswordRequest, ServerInfo, Site, SiteQuery,
    SubmitTunnelKeyRequest, TunnelClientConfig, TunnelKeyRequest, TunnelKeyRequestStatus,
    UpdateMiner, UpdateSite, UpdateUserRequest, UpdateWebhook, User, UserRole, Webhook,
    WebhookDelivery, API_VERSION,
};
use serde::Deserialize;
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
use uuid::Uuid;

const SECRET_MASK: &str = "********";

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    session_days: i64,
    login_limiter: Arc<Mutex<LoginLimiter>>,
    status_limiter: Arc<Mutex<StatusLimiter>>,
    dummy_password_hash: String,
    pairing: PairingInfo,
    webhook_client: reqwest::Client,
    tunnel_client: TunnelClientConfig,
}

const LOGIN_WINDOW: StdDuration = StdDuration::from_secs(60);
const LOGIN_ACCOUNT_LIMIT: usize = 5;
const LOGIN_SOURCE_LIMIT: usize = 30;
const LOGIN_LIMITER_CAPACITY: usize = 10_000;
const STATUS_WINDOW: StdDuration = StdDuration::from_secs(60);
const STATUS_SOURCE_LIMIT: usize = 120;

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

#[derive(Default)]
struct StatusLimiter {
    source_attempts: HashMap<IpAddr, Vec<Instant>>,
}

impl StatusLimiter {
    fn allow(&mut self, source: IpAddr, now: Instant) -> bool {
        self.prune(now);
        if self.source_attempts.get(&source).map_or(0, Vec::len) >= STATUS_SOURCE_LIMIT {
            return false;
        }
        self.source_attempts.entry(source).or_default().push(now);
        true
    }

    fn prune(&mut self, now: Instant) {
        self.source_attempts.retain(|_, attempts| {
            attempts.retain(|attempt| now.duration_since(*attempt) < STATUS_WINDOW);
            !attempts.is_empty()
        });
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

#[derive(serde::Deserialize)]
struct VersionSiteQuery {
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

    let webhook_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let state = AppState {
        pool,
        session_days: config.session_days,
        login_limiter: Arc::new(Mutex::new(LoginLimiter::default())),
        status_limiter: Arc::new(Mutex::new(StatusLimiter::default())),
        dummy_password_hash: hash_password("dummy-password-never-used")?,
        pairing: PairingInfo {
            server: server_info(),
            certificate_pem,
            fingerprint_sha256,
        },
        webhook_client,
        tunnel_client: TunnelClientConfig {
            ssh_destination: config.tunnel_client.ssh_destination.clone(),
            ssh_port: config.tunnel_client.ssh_port,
            local_port: config.tunnel_client.local_port,
            remote_host: config.tunnel_client.remote_host.clone(),
            remote_port: config.tunnel_client.remote_port,
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
        .route(
            "/api/v1/webhooks/{id}/deliveries",
            get(list_webhook_deliveries),
        )
        .route("/api/v1/sites", get(list_sites).post(create_site))
        .route(
            "/api/v1/sites/{id}",
            put(update_site).delete(delete_site),
        )
        .route(
            "/api/v1/tunnel-key-requests",
            post(submit_tunnel_key_request).get(list_tunnel_key_requests),
        )
        .route(
            "/api/v1/tunnel-key-requests/{id}/approve",
            post(approve_tunnel_key_request),
        )
        .route(
            "/api/v1/tunnel-key-requests/{id}/reject",
            post(reject_tunnel_key_request),
        )
        .route(
            "/api/v1/tunnel-key-requests/{id}/revoke",
            post(revoke_tunnel_key_request),
        )
        .route(
            "/api/v1/tunnel-key-requests/{id}/status",
            get(get_tunnel_key_request_status),
        )
        .route(
            "/api/v1/tunnel-key-requests/{id}",
            axum::routing::delete(delete_tunnel_key_request),
        )
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
        SELECT u.id, u.site_id, s.name AS site_name, u.username, u.display_name, u.role, u.enabled, u.version
        FROM sessions ses
        JOIN users u ON u.id = ses.user_id
        LEFT JOIN sites s ON s.id = u.site_id
        WHERE ses.token_hash = $1 AND ses.revoked_at IS NULL AND ses.expires_at > NOW() AND u.enabled = TRUE
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

/// Resolve the effective site_id for a request.
/// Priority: explicit query > user's assigned site > default enabled site.
async fn resolve_site_id(
    _pool: &PgPool,
    explicit: Option<i64>,
    user_site_id: Option<i64>,
) -> AppResult<Option<i64>> {
    if let Some(id) = explicit {
        return Ok(Some(id));
    }
    if let Some(id) = user_site_id {
        return Ok(Some(id));
    }
    // Admin with no site assigned and no filter → None (all sites)
    Ok(None)
}

/// Get the default enabled site id (used when site_id must be set but was not provided).
async fn default_site_id(pool: &PgPool) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>("SELECT id FROM sites WHERE enabled = TRUE ORDER BY id LIMIT 1")
        .fetch_optional(pool)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::bad_request("no enabled site found; create a site first"))
}

/// Insert an audit log row.  Failures are swallowed — they must not break the caller.
async fn audit_log(
    state: &AppState,
    user_id: Option<i64>,
    username: Option<&str>,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    target_serial: Option<&str>,
    old_values: Option<&serde_json::Value>,
    new_values: Option<&serde_json::Value>,
    ip_address: Option<&str>,
) {
    let result = sqlx::query(
        r#"INSERT INTO audit_log
           (user_id, username, action, target_type, target_id, target_serial, old_values, new_values, ip_address)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(target_serial)
    .bind(old_values)
    .bind(new_values)
    .bind(ip_address)
    .execute(&state.pool)
    .await;

    if let Err(err) = result {
        tracing::warn!(error = %err, "audit log insert failed (non-fatal)");
        return;
    }

    // Dispatch webhooks for audit-driven events (fire-and-forget)
    let event = action_to_webhook_event(action);
    if let Some(event) = event {
        let payload = serde_json::json!({
            "event": event,
            "action": action,
            "target_type": target_type,
            "target_id": target_id,
            "target_serial": target_serial,
        });
        dispatch_webhooks(state, event, payload).await;
    }
}

fn action_to_webhook_event(action: &str) -> Option<&'static str> {
    match action {
        "miner.created" => Some("miner.created"),
        "miner.updated" => Some("miner.updated"),
        "miner.deleted" => Some("miner.deleted"),
        "part.created" => Some("part.created"),
        "part.updated" => Some("part.updated"),
        "part.deleted" => Some("part.deleted"),
        "user.created" => Some("user.created"),
        "user.updated" => Some("user.updated"),
        _ => None,
    }
}

/// Fire webhook deliveries for all enabled webhooks subscribed to `event`.
/// Best-effort: failures are logged but never propagate.
async fn dispatch_webhooks(state: &AppState, event: &str, payload: serde_json::Value) {
    let rows = match sqlx::query(
        "SELECT id, url, secret FROM webhooks WHERE enabled = TRUE AND $1 = ANY(events)",
    )
    .bind(event)
    .fetch_all(&state.pool)
    .await
    {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(error = %err, "webhook query failed (non-fatal)");
            return;
        }
    };

    for row in rows {
        let webhook_id: i64 = row.get("id");
        let url: String = row.get("url");
        let secret: Option<String> = row.get("secret");

        let mut req = state
            .webhook_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Fleet-Event", event);

        if let Some(ref s) = secret {
            req = req.header("X-Fleet-Signature", s.as_str());
        }

        let (success, response_status, response_body, error_msg) =
            match req.json(&payload).send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16() as i32;
                    let ok = resp.status().is_success();
                    let body = resp.text().await.unwrap_or_default();
                    (ok, Some(status), Some(body), None::<String>)
                }
                Err(err) => (false, None, None, Some(err.to_string())),
            };

        let _ = sqlx::query(
            r#"INSERT INTO webhook_deliveries
               (webhook_id, event, payload, response_status, response_body, success, error, delivered_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7, CASE WHEN $6 THEN NOW() ELSE NULL END)"#,
        )
        .bind(webhook_id)
        .bind(event)
        .bind(&payload)
        .bind(response_status)
        .bind(response_body)
        .bind(success)
        .bind(error_msg)
        .execute(&state.pool)
        .await;
    }
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

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

    // Build a full user row for the response (need site join)
    let user_row = sqlx::query(
        r#"SELECT u.id, u.site_id, s.name AS site_name, u.username, u.display_name, u.role, u.enabled, u.version
           FROM users u LEFT JOIN sites s ON s.id = u.site_id WHERE u.id = $1"#,
    )
    .bind(row.get::<i64, _>("id"))
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    let user = user_from_row(&user_row);

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

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<User>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        r#"SELECT u.id, u.site_id, s.name AS site_name, u.username, u.display_name, u.role, u.enabled, u.version
           FROM users u LEFT JOIN sites s ON s.id = u.site_id ORDER BY u.username"#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(user_from_row).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<User>)> {
    let admin = require_admin(&state, &headers).await?;
    let username = normalize_username(&input.username);
    if username.is_empty() || input.display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "username and display name are required",
        ));
    }
    let password_hash = hash_password(&input.password).map_err(AppError::bad_request)?;
    let row = sqlx::query(
        r#"INSERT INTO users (username, display_name, password_hash, role, site_id)
           VALUES ($1,$2,$3,$4,$5)
           RETURNING id, site_id, NULL::TEXT AS site_name, username, display_name, role, enabled, version"#,
    )
    .bind(&username)
    .bind(input.display_name.trim())
    .bind(password_hash)
    .bind(input.role.as_str())
    .bind(input.site_id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    let user = user_from_row(&row);
    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "user.created",
        Some("user"),
        Some(&user.id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"username": user.username, "role": user.role})),
        Some(&remote.ip().to_string()),
    )
    .await;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_user(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    let admin = require_admin(&state, &headers).await?;
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
        r#"UPDATE users SET display_name=$1, role=$2, enabled=$3, site_id=$4, version=version+1, updated_at=NOW()
           WHERE id=$5 AND version=$6
           RETURNING id, site_id, NULL::TEXT AS site_name, username, display_name, role, enabled, version"#,
    )
    .bind(input.display_name.trim())
    .bind(input.role.as_str())
    .bind(input.enabled)
    .bind(input.site_id)
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
    let user = user_from_row(&row);
    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "user.updated",
        Some("user"),
        Some(&user.id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"role": user.role, "enabled": user.enabled})),
        Some(&remote.ip().to_string()),
    )
    .await;
    Ok(Json(user))
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

// ---------------------------------------------------------------------------
// Miners
// ---------------------------------------------------------------------------

fn miner_from_row(row: &sqlx::postgres::PgRow) -> Miner {
    Miner {
        id: row.get("id"),
        site_id: row.get("site_id"),
        site_name: row.get("site_name"),
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

const MINER_SELECT: &str = r#"
    SELECT m.id, m.site_id, s.name AS site_name,
           m.serial, m.model, m.firmware, m.client_name, m.miner_type,
           m.ip_address, m.mac_address, m.pickaxe, m.miner_state,
           m.miner_row, m.miner_index, m.miner_rack, m.miner_rack_group,
           m.location, m.status, m.acquired_date, m.notes, m.version
    FROM miners m LEFT JOIN sites s ON s.id = m.site_id
"#;

async fn list_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<Vec<Miner>>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = resolve_site_id(&state.pool, query.site_id, user.site_id).await?;
    let rows = if let Some(sid) = site_id {
        sqlx::query(&format!("{MINER_SELECT} WHERE m.site_id = $1 ORDER BY m.serial"))
            .bind(sid)
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?
    } else {
        sqlx::query(&format!("{MINER_SELECT} ORDER BY m.serial"))
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?
    };
    Ok(Json(rows.iter().map(miner_from_row).collect()))
}

async fn create_miner(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(mut input): Json<CreateMiner>,
) -> AppResult<(StatusCode, Json<Miner>)> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    normalize_and_validate_miner(&mut input).map_err(AppError::bad_request)?;
    let site_id = match input.site_id {
        Some(id) => id,
        None => match user.site_id {
            Some(id) => id,
            None => default_site_id(&state.pool).await?,
        },
    };
    let row = sqlx::query(&format!(
        r#"INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,
           pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
           RETURNING id, site_id, NULL::TEXT AS site_name,
           serial,model,firmware,client_name,miner_type,ip_address,mac_address,
           pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,
           location,status,acquired_date,notes,version"#
    ))
    .bind(site_id)
    .bind(&input.serial).bind(&input.model).bind(&input.firmware).bind(&input.client_name)
    .bind(&input.miner_type).bind(&input.ip_address).bind(&input.mac_address).bind(&input.pickaxe)
    .bind(&input.miner_state).bind(&input.miner_row).bind(&input.miner_index).bind(&input.miner_rack)
    .bind(&input.miner_rack_group).bind(&input.location).bind(&input.status).bind(&input.acquired_date)
    .bind(&input.notes).fetch_one(&state.pool).await.map_err(AppError::database)?;
    let miner = miner_from_row(&row);
    audit_log(
        &state, Some(user.id), Some(&user.username), "miner.created",
        Some("miner"), Some(&miner.id.to_string()), Some(&miner.serial),
        None, Some(&serde_json::json!({"serial": miner.serial, "model": miner.model})),
        Some(&remote.ip().to_string()),
    ).await;
    Ok((StatusCode::CREATED, Json(miner)))
}

async fn update_miner(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateMiner>,
) -> AppResult<Json<Miner>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    if input.id != id {
        return Err(AppError::bad_request("path and body miner IDs differ"));
    }
    let mut validated = CreateMiner {
        site_id: input.site_id,
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
    // If site_id not in update, keep existing
    let row = sqlx::query(&format!(
        r#"UPDATE miners SET
           site_id=COALESCE($1, site_id),
           serial=$2,model=$3,firmware=$4,client_name=$5,miner_type=$6,ip_address=$7,mac_address=$8,
           pickaxe=$9,miner_state=$10,miner_row=$11,miner_index=$12,miner_rack=$13,miner_rack_group=$14,
           location=$15,status=$16,acquired_date=$17,notes=$18,version=version+1,updated_at=NOW()
           WHERE id=$19 AND version=$20
           RETURNING id, site_id, NULL::TEXT AS site_name,
           serial,model,firmware,client_name,miner_type,ip_address,mac_address,
           pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,
           location,status,acquired_date,notes,version"#
    ))
    .bind(validated.site_id)
    .bind(&validated.serial).bind(&validated.model).bind(&validated.firmware).bind(&validated.client_name)
    .bind(&validated.miner_type).bind(&validated.ip_address).bind(&validated.mac_address).bind(&validated.pickaxe)
    .bind(&validated.miner_state).bind(&validated.miner_row).bind(&validated.miner_index).bind(&validated.miner_rack)
    .bind(&validated.miner_rack_group).bind(&validated.location).bind(&validated.status).bind(&validated.acquired_date)
    .bind(&validated.notes).bind(id).bind(input.version)
    .fetch_optional(&state.pool).await.map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("miner changed or was deleted; reload and try again"))?;
    let miner = miner_from_row(&row);
    audit_log(
        &state, Some(user.id), Some(&user.username), "miner.updated",
        Some("miner"), Some(&miner.id.to_string()), Some(&miner.serial),
        None, Some(&serde_json::json!({"serial": miner.serial, "status": miner.status})),
        Some(&remote.ip().to_string()),
    ).await;
    Ok(Json(miner))
}

async fn delete_miner(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let serial: Option<String> = sqlx::query_scalar("SELECT serial FROM miners WHERE id=$1")
        .bind(id).fetch_optional(&state.pool).await.map_err(AppError::database)?;
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
    audit_log(
        &state, Some(user.id), Some(&user.username), "miner.deleted",
        Some("miner"), Some(&id.to_string()), serial.as_deref(),
        None, None, Some(&remote.ip().to_string()),
    ).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn import_miners(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(inputs): Json<Vec<CreateMiner>>,
) -> AppResult<Json<MinerImportResult>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    if user.role != UserRole::Admin {
        return Err(AppError::forbidden("administrator access required"));
    }
    let site_id = match user.site_id {
        Some(id) => id,
        None => default_site_id(&state.pool).await?,
    };
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
        let effective_site = miner.site_id.unwrap_or(site_id);
        let inserted: Option<i64> = sqlx::query_scalar(
            "INSERT INTO miners (site_id,serial,model,firmware,client_name,miner_type,ip_address,mac_address,pickaxe,miner_state,miner_row,miner_index,miner_rack,miner_rack_group,location,status,acquired_date,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) ON CONFLICT (site_id, serial) DO NOTHING RETURNING id",
        )
        .bind(effective_site)
        .bind(&miner.serial).bind(&miner.model).bind(&miner.firmware).bind(&miner.client_name)
        .bind(&miner.miner_type).bind(&miner.ip_address).bind(&miner.mac_address).bind(&miner.pickaxe)
        .bind(&miner.miner_state).bind(&miner.miner_row).bind(&miner.miner_index).bind(&miner.miner_rack)
        .bind(&miner.miner_rack_group).bind(&miner.location).bind(&miner.status).bind(&miner.acquired_date)
        .bind(&miner.notes).fetch_optional(&mut *tx).await.map_err(AppError::database)?;
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

// ---------------------------------------------------------------------------
// Parts
// ---------------------------------------------------------------------------

fn part_from_row(row: &sqlx::postgres::PgRow) -> Part {
    Part {
        site_id: row.get("site_id"),
        site_name: row.get("site_name"),
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

const PART_SELECT: &str = r#"
    SELECT p.site_id, s.name AS site_name,
           p.sku, p.name, p.category, p.qty_on_hand, p.reorder_threshold,
           p.supplier, p.unit_cost_cents, p.notes, p.version
    FROM parts p LEFT JOIN sites s ON s.id = p.site_id
"#;

async fn list_parts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<Vec<Part>>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = resolve_site_id(&state.pool, query.site_id, user.site_id).await?;
    let rows = if let Some(sid) = site_id {
        sqlx::query(&format!("{PART_SELECT} WHERE p.site_id = $1 ORDER BY p.name"))
            .bind(sid)
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?
    } else {
        sqlx::query(&format!("{PART_SELECT} ORDER BY p.name"))
            .fetch_all(&state.pool)
            .await
            .map_err(AppError::database)?
    };
    Ok(Json(rows.iter().map(part_from_row).collect()))
}

async fn create_part(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(mut input): Json<CreatePart>,
) -> AppResult<(StatusCode, Json<Part>)> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    input.sku = input.sku.trim().to_string();
    validate_part(&input).map_err(AppError::bad_request)?;
    let site_id = match input.site_id {
        Some(id) => id,
        None => match user.site_id {
            Some(id) => id,
            None => default_site_id(&state.pool).await?,
        },
    };
    let row = sqlx::query(&format!(
        "INSERT INTO parts (site_id,sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING site_id, NULL::TEXT AS site_name, sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes,version"
    ))
    .bind(site_id)
    .bind(&input.sku).bind(input.name.trim()).bind(&input.category).bind(input.qty_on_hand)
    .bind(input.reorder_threshold).bind(&input.supplier).bind(input.unit_cost_cents).bind(&input.notes)
    .fetch_one(&state.pool).await.map_err(AppError::database)?;
    let part = part_from_row(&row);
    audit_log(
        &state, Some(user.id), Some(&user.username), "part.created",
        Some("part"), Some(&part.sku), None,
        None, Some(&serde_json::json!({"sku": part.sku, "name": part.name})),
        Some(&remote.ip().to_string()),
    ).await;
    Ok((StatusCode::CREATED, Json(part)))
}

async fn update_part(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(sku): Path<String>,
    Json(input): Json<Part>,
) -> AppResult<Json<Part>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    if input.sku != sku {
        return Err(AppError::bad_request("path and body SKUs differ"));
    }
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
    let row = sqlx::query(
        "UPDATE parts SET name=$1,category=$2,qty_on_hand=$3,reorder_threshold=$4,supplier=$5,unit_cost_cents=$6,notes=$7,version=version+1,updated_at=NOW() WHERE sku=$8 AND site_id=$9 AND version=$10 RETURNING site_id, NULL::TEXT AS site_name, sku,name,category,qty_on_hand,reorder_threshold,supplier,unit_cost_cents,notes,version"
    )
    .bind(input.name.trim()).bind(&input.category).bind(input.qty_on_hand).bind(input.reorder_threshold)
    .bind(&input.supplier).bind(input.unit_cost_cents).bind(&input.notes).bind(&sku)
    .bind(input.site_id).bind(input.version)
    .fetch_optional(&state.pool).await.map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("part changed or was deleted; reload and try again"))?;
    let part = part_from_row(&row);
    audit_log(
        &state, Some(user.id), Some(&user.username), "part.updated",
        Some("part"), Some(&part.sku), None,
        None, Some(&serde_json::json!({"sku": part.sku, "qty_on_hand": part.qty_on_hand})),
        Some(&remote.ip().to_string()),
    ).await;
    Ok(Json(part))
}

async fn delete_part(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(sku): Path<String>,
    Query(query): Query<VersionSiteQuery>,
) -> AppResult<StatusCode> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = match query.site_id {
        Some(id) => id,
        None => match user.site_id {
            Some(id) => id,
            None => default_site_id(&state.pool).await?,
        },
    };
    let result = sqlx::query("DELETE FROM parts WHERE sku=$1 AND site_id=$2 AND version=$3")
        .bind(&sku)
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
    audit_log(
        &state, Some(user.id), Some(&user.username), "part.deleted",
        Some("part"), Some(&sku), None,
        None, None, Some(&remote.ip().to_string()),
    ).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SiteQuery>,
) -> AppResult<Json<DashboardSummary>> {
    let (user, _) = authenticated_user(&state, &headers).await?;
    let site_id = resolve_site_id(&state.pool, query.site_id, user.site_id).await?;
    let (where_clause, bind_sid): (&str, Option<i64>) = if let Some(sid) = site_id {
        (" WHERE site_id = $1", Some(sid))
    } else {
        ("", None)
    };
    let counts = if let Some(sid) = bind_sid {
        sqlx::query(
            &format!("SELECT (SELECT COUNT(*) FROM miners{where_clause}) unit_count, (SELECT COUNT(*) FROM parts{where_clause}) part_count, (SELECT COUNT(*) FROM parts{where_clause} AND qty_on_hand <= reorder_threshold) low_stock_count")
        ).bind(sid).bind(sid).bind(sid).fetch_one(&state.pool).await.map_err(AppError::database)?
    } else {
        sqlx::query(
            "SELECT (SELECT COUNT(*) FROM miners) unit_count, (SELECT COUNT(*) FROM parts) part_count, (SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold) low_stock_count"
        ).fetch_one(&state.pool).await.map_err(AppError::database)?
    };
    let statuses = if let Some(sid) = bind_sid {
        sqlx::query("SELECT status, COUNT(*) count FROM miners WHERE site_id=$1 GROUP BY status ORDER BY status")
            .bind(sid).fetch_all(&state.pool).await.map_err(AppError::database)?
    } else {
        sqlx::query("SELECT status, COUNT(*) count FROM miners GROUP BY status ORDER BY status")
            .fetch_all(&state.pool).await.map_err(AppError::database)?
    };
    let low_parts = if let Some(sid) = bind_sid {
        sqlx::query(&format!("{PART_SELECT} WHERE p.site_id=$1 AND p.qty_on_hand <= p.reorder_threshold ORDER BY p.qty_on_hand, p.name LIMIT 10"))
            .bind(sid).fetch_all(&state.pool).await.map_err(AppError::database)?
    } else {
        sqlx::query(&format!("{PART_SELECT} WHERE p.qty_on_hand <= p.reorder_threshold ORDER BY p.qty_on_hand, p.name LIMIT 10"))
            .fetch_all(&state.pool).await.map_err(AppError::database)?
    };
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
        low_stock_parts: low_parts.iter().map(part_from_row).collect(),
    }))
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

async fn list_audit_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> AppResult<Json<Vec<AuditLogEntry>>> {
    require_admin(&state, &headers).await?;
    // Build dynamic query
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_index: i32 = 1;

    if query.user_id.is_some() {
        conditions.push(format!("user_id = ${bind_index}"));
        bind_index += 1;
    }
    if query.action.is_some() {
        conditions.push(format!("action = ${bind_index}"));
        bind_index += 1;
    }
    if query.target_type.is_some() {
        conditions.push(format!("target_type = ${bind_index}"));
        bind_index += 1;
    }
    if query.target_id.is_some() {
        conditions.push(format!("target_id = ${bind_index}"));
        bind_index += 1;
    }
    if query.from.is_some() {
        conditions.push(format!("created_at >= ${bind_index}"));
        bind_index += 1;
    }
    if query.to.is_some() {
        conditions.push(format!("created_at <= ${bind_index}"));
        bind_index += 1;
    }

    let _ = bind_index;

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let limit = query.limit.unwrap_or(200).min(1000);
    let offset = query.offset.unwrap_or(0);
    let sql = format!(
        "SELECT id, user_id, username, action, target_type, target_id, target_serial, old_values, new_values, ip_address, user_agent, created_at FROM audit_log {where_clause} ORDER BY created_at DESC LIMIT {limit} OFFSET {offset}"
    );

    let mut q = sqlx::query(&sql);
    if let Some(v) = query.user_id { q = q.bind(v); }
    if let Some(v) = query.action { q = q.bind(v); }
    if let Some(v) = query.target_type { q = q.bind(v); }
    if let Some(v) = query.target_id { q = q.bind(v); }
    if let Some(v) = query.from { q = q.bind(v); }
    if let Some(v) = query.to { q = q.bind(v); }

    let rows = q.fetch_all(&state.pool).await.map_err(AppError::database)?;
    let entries = rows
        .iter()
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
            created_at: row
                .get::<chrono::DateTime<Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect();
    Ok(Json(entries))
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

fn mask_webhook_secret(secret: Option<String>) -> Option<String> {
    secret.map(|_| SECRET_MASK.to_string())
}

fn webhook_from_row(row: &sqlx::postgres::PgRow) -> Webhook {
    Webhook {
        id: row.get("id"),
        name: row.get("name"),
        url: row.get("url"),
        secret: mask_webhook_secret(row.get("secret")),
        events: row.get("events"),
        enabled: row.get("enabled"),
        version: row.get("version"),
    }
}

fn delivery_from_row(row: &sqlx::postgres::PgRow) -> WebhookDelivery {
    WebhookDelivery {
        id: row.get("id"),
        webhook_id: row.get("webhook_id"),
        event: row.get("event"),
        payload: row.get("payload"),
        response_status: row.get("response_status"),
        response_body: row.get("response_body"),
        success: row.get("success"),
        error: row.get("error"),
        attempts: row.get("attempts"),
        created_at: row
            .get::<chrono::DateTime<Utc>, _>("created_at")
            .to_rfc3339(),
        delivered_at: row
            .get::<Option<chrono::DateTime<Utc>>, _>("delivered_at")
            .map(|t| t.to_rfc3339()),
    }
}

async fn list_webhooks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Webhook>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, name, url, secret, events, enabled, version FROM webhooks ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(webhook_from_row).collect()))
}

async fn create_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateWebhook>,
) -> AppResult<(StatusCode, Json<Webhook>)> {
    require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("webhook name is required"));
    }
    if input.url.trim().is_empty() {
        return Err(AppError::bad_request("webhook URL is required"));
    }
    let secret = input.secret.filter(|s| !s.is_empty() && s != SECRET_MASK);
    let row = sqlx::query(
        "INSERT INTO webhooks (name, url, secret, events, enabled) VALUES ($1,$2,$3,$4,$5) RETURNING id, name, url, secret, events, enabled, version",
    )
    .bind(input.name.trim())
    .bind(input.url.trim())
    .bind(secret)
    .bind(&input.events)
    .bind(input.enabled)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok((StatusCode::CREATED, Json(webhook_from_row(&row))))
}

async fn update_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateWebhook>,
) -> AppResult<Json<Webhook>> {
    require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("webhook name is required"));
    }
    if input.url.trim().is_empty() {
        return Err(AppError::bad_request("webhook URL is required"));
    }
    // null / "" / "********" → preserve; any other non-empty → replace
    let secret_update = input
        .secret
        .filter(|s| !s.is_empty() && s != SECRET_MASK);

    let row = if let Some(new_secret) = secret_update {
        sqlx::query(
            "UPDATE webhooks SET name=$1, url=$2, secret=$3, events=$4, enabled=$5, version=version+1, updated_at=NOW() WHERE id=$6 AND version=$7 RETURNING id, name, url, secret, events, enabled, version",
        )
        .bind(input.name.trim())
        .bind(input.url.trim())
        .bind(new_secret)
        .bind(&input.events)
        .bind(input.enabled)
        .bind(id)
        .bind(input.version)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?
    } else {
        sqlx::query(
            "UPDATE webhooks SET name=$1, url=$2, events=$3, enabled=$4, version=version+1, updated_at=NOW() WHERE id=$5 AND version=$6 RETURNING id, name, url, secret, events, enabled, version",
        )
        .bind(input.name.trim())
        .bind(input.url.trim())
        .bind(&input.events)
        .bind(input.enabled)
        .bind(id)
        .bind(input.version)
        .fetch_optional(&state.pool)
        .await
        .map_err(AppError::database)?
    };

    let row = row.ok_or_else(|| AppError::conflict("webhook changed; reload and try again"))?;
    Ok(Json(webhook_from_row(&row)))
}

async fn delete_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM webhooks WHERE id=$1 AND version=$2")
        .bind(id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "webhook changed or was deleted; reload and try again",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_webhook_deliveries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<WebhookDelivery>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, webhook_id, event, payload, response_status, response_body, success, error, attempts, created_at, delivered_at FROM webhook_deliveries WHERE webhook_id=$1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(delivery_from_row).collect()))
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

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

async fn list_sites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Site>>> {
    authenticated_user(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, name, code, description, enabled, version FROM sites ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(site_from_row).collect()))
}

async fn create_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateSite>,
) -> AppResult<(StatusCode, Json<Site>)> {
    require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() || input.code.trim().is_empty() {
        return Err(AppError::bad_request("site name and code are required"));
    }
    let row = sqlx::query(
        "INSERT INTO sites (name, code, description, enabled) VALUES ($1,$2,$3,$4) RETURNING id, name, code, description, enabled, version",
    )
    .bind(input.name.trim())
    .bind(input.code.trim().to_uppercase())
    .bind(input.description.as_deref().map(str::trim))
    .bind(input.enabled)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok((StatusCode::CREATED, Json(site_from_row(&row))))
}

async fn update_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpdateSite>,
) -> AppResult<Json<Site>> {
    require_admin(&state, &headers).await?;
    if input.name.trim().is_empty() || input.code.trim().is_empty() {
        return Err(AppError::bad_request("site name and code are required"));
    }
    let row = sqlx::query(
        "UPDATE sites SET name=$1, code=$2, description=$3, enabled=$4, version=version+1, updated_at=NOW() WHERE id=$5 AND version=$6 RETURNING id, name, code, description, enabled, version",
    )
    .bind(input.name.trim())
    .bind(input.code.trim().to_uppercase())
    .bind(input.description.as_deref().map(str::trim))
    .bind(input.enabled)
    .bind(id)
    .bind(input.version)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::conflict("site changed; reload and try again"))?;
    Ok(Json(site_from_row(&row)))
}

async fn delete_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<StatusCode> {
    require_admin(&state, &headers).await?;
    // Reject if any users/miners/parts still reference this site
    let miner_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM miners WHERE site_id=$1")
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .map_err(AppError::database)?;
    let part_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM parts WHERE site_id=$1")
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .map_err(AppError::database)?;
    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE site_id=$1")
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .map_err(AppError::database)?;
    if miner_count + part_count + user_count > 0 {
        return Err(AppError::bad_request(
            "site is still referenced by users, miners, or parts; reassign them first",
        ));
    }
    let result = sqlx::query("DELETE FROM sites WHERE id=$1 AND version=$2")
        .bind(id)
        .bind(query.version)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "site changed or was deleted; reload and try again",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

async fn submit_tunnel_key_request(
    State(state): State<AppState>,
    Json(input): Json<SubmitTunnelKeyRequest>,
) -> AppResult<(StatusCode, Json<TunnelKeyRequest>)> {
    let label = input.label.trim().to_string();
    let public_key = input.public_key.trim().to_string();

    if label.is_empty() {
        return Err(AppError::bad_request("label is required"));
    }
    if !label.chars().all(|c| c.is_alphanumeric() || "._@+-".contains(c)) {
        return Err(AppError::bad_request(
            "label may contain only letters, numbers, dot, underscore, at, plus, and dash",
        ));
    }
    if public_key.is_empty() {
        return Err(AppError::bad_request("public_key is required"));
    }
    let mut parts = public_key.splitn(3, char::is_whitespace);
    let key_type = parts.next().unwrap_or("").to_string();
    let key_body = parts.next().unwrap_or("").to_string();
    if key_type.is_empty() || key_body.is_empty() {
        return Err(AppError::bad_request("public_key must be in OpenSSH format"));
    }
    let allowed_types = [
        "ssh-ed25519",
        "ecdsa-sha2-nistp256",
        "ecdsa-sha2-nistp384",
        "ecdsa-sha2-nistp521",
        "rsa-sha2-256",
        "rsa-sha2-512",
        "ssh-rsa",
    ];
    if !allowed_types.contains(&key_type.as_str()) {
        return Err(AppError::bad_request("unsupported public key type"));
    }

    let status_token = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO tunnel_key_requests (label, public_key, status_token)
         VALUES ($1, $2, $3)
         RETURNING id, label, public_key, status, note, status_token, created_at",
    )
    .bind(&label)
    .bind(&public_key)
    .bind(&status_token)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    Ok((StatusCode::CREATED, Json(tunnel_key_request_from_row(&row))))
}

async fn list_tunnel_key_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<TunnelKeyRequest>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, label, public_key, status, note, status_token, created_at
         FROM tunnel_key_requests
         ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(tunnel_key_request_from_row).collect()))
}

async fn approve_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ApproveTunnelKeyRequest>,
) -> AppResult<Json<TunnelKeyRequest>> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "SELECT id, label, public_key, status, note, status_token, created_at
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    if row.get::<String, _>("status") != "pending" {
        return Err(AppError::bad_request("only pending requests can be approved"));
    }

    let label: String = row.get("label");
    let public_key: String = row.get("public_key");

    let script_path = tunnel_script_path("authorize-client-tunnel-key.sh")?;

    let output = tokio::process::Command::new(&script_path)
        .args(["--label", &label, "--public-key", &public_key])
        .output()
        .await
        .map_err(|e| AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Could not run key authorization script: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if stderr.is_empty() { stdout } else { stderr };
        return Err(AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Key authorization script failed: {msg}"),
        });
    }

    let updated = sqlx::query(
        "UPDATE tunnel_key_requests
         SET status = 'approved', note = $1, updated_at = NOW()
         WHERE id = $2
         RETURNING id, label, public_key, status, note, status_token, created_at",
    )
    .bind(input.note.as_deref())
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "tunnel_key.approved",
        Some("tunnel_key_request"),
        Some(&id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"label": label})),
        Some(&remote.ip().to_string()),
    )
    .await;

    Ok(Json(tunnel_key_request_from_row(&updated)))
}

async fn reject_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ApproveTunnelKeyRequest>,
) -> AppResult<Json<TunnelKeyRequest>> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "SELECT id, label, public_key, status, note, status_token, created_at
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    let status: String = row.get("status");
    if status != "pending" {
        return Err(AppError::bad_request("only pending requests can be rejected"));
    }

    let label: String = row.get("label");
    let updated = sqlx::query(
        "UPDATE tunnel_key_requests
         SET status = 'rejected', note = $1, updated_at = NOW()
         WHERE id = $2
         RETURNING id, label, public_key, status, note, status_token, created_at",
    )
    .bind(input.note.as_deref())
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "tunnel_key.rejected",
        Some("tunnel_key_request"),
        Some(&id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"label": label})),
        Some(&remote.ip().to_string()),
    )
    .await;

    Ok(Json(tunnel_key_request_from_row(&updated)))
}

async fn revoke_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ApproveTunnelKeyRequest>,
) -> AppResult<Json<TunnelKeyRequest>> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "SELECT id, label, public_key, status, note, status_token, created_at
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    let status: String = row.get("status");
    if status != "approved" {
        return Err(AppError::bad_request("only approved requests can be revoked"));
    }

    let label: String = row.get("label");
    let script_path = tunnel_script_path("revoke-client-tunnel-key.sh")?;
    let output = tokio::process::Command::new(&script_path)
        .args(["--label", &label])
        .output()
        .await
        .map_err(|e| AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Could not run key revocation script: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if stderr.is_empty() { stdout } else { stderr };
        return Err(AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Key revocation script failed: {msg}"),
        });
    }

    let updated = sqlx::query(
        "UPDATE tunnel_key_requests
         SET status = 'revoked', note = $1, updated_at = NOW()
         WHERE id = $2
         RETURNING id, label, public_key, status, note, status_token, created_at",
    )
    .bind(input.note.as_deref())
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "tunnel_key.revoked",
        Some("tunnel_key_request"),
        Some(&id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"label": label})),
        Some(&remote.ip().to_string()),
    )
    .await;

    Ok(Json(tunnel_key_request_from_row(&updated)))
}

#[derive(Debug, Deserialize)]
struct TunnelKeyStatusQuery {
    token: String,
}

async fn get_tunnel_key_request_status(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Path(id): Path<i64>,
    Query(query): Query<TunnelKeyStatusQuery>,
) -> AppResult<Json<TunnelKeyRequestStatus>> {
    {
        let mut limiter = state.status_limiter.lock().await;
        if !limiter.allow(remote.ip(), Instant::now()) {
            return Err(AppError {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "rate_limited",
                message: "too many status checks; try again shortly".into(),
            });
        }
    }

    let row = sqlx::query(
        "SELECT id, status, note, status_token
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    let stored_token: String = row.get("status_token");
    if stored_token != query.token.trim() {
        return Err(AppError::not_found("tunnel key request not found"));
    }

    let status: String = row.get("status");
    let client_config = if status == "approved" {
        Some(state.tunnel_client.clone())
    } else {
        None
    };

    Ok(Json(TunnelKeyRequestStatus {
        id: row.get("id"),
        status,
        note: row.get("note"),
        client_config,
    }))
}

async fn delete_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "SELECT id, label, status
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    let status: String = row.get("status");
    if status == "pending" {
        let label: String = row.get("label");
        sqlx::query(
            "UPDATE tunnel_key_requests
             SET status = 'rejected', updated_at = NOW()
             WHERE id = $1",
        )
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(AppError::database)?;
        audit_log(
            &state,
            Some(admin.id),
            Some(&admin.username),
            "tunnel_key.rejected",
            Some("tunnel_key_request"),
            Some(&id.to_string()),
            None,
            None,
            Some(&serde_json::json!({"label": label})),
            Some(&remote.ip().to_string()),
        )
        .await;
        return Ok(StatusCode::NO_CONTENT);
    }

    Err(AppError {
        status: StatusCode::GONE,
        code: "use_revoke",
        message: "approved tunnel keys must be revoked, not deleted".into(),
    })
}

fn tunnel_script_path(script_name: &str) -> Result<String, AppError> {
    let installed = format!("/usr/lib/antminer-fleet-server/{script_name}");
    let dev = {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push(format!("../../server/scripts/{script_name}"));
        p
    };
    if std::path::Path::new(&installed).exists() {
        Ok(installed)
    } else if dev.exists() {
        Ok(dev.display().to_string())
    } else {
        Err(AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_not_found",
            message: format!("{script_name} not found; is the server package installed?"),
        })
    }
}

fn tunnel_key_request_from_row(row: &sqlx::postgres::PgRow) -> TunnelKeyRequest {
    let public_key: String = row.get("public_key");
    TunnelKeyRequest {
        id: row.get("id"),
        label: row.get("label"),
        public_key: public_key.clone(),
        status: row.get("status"),
        note: row.get("note"),
        status_token: row.get("status_token"),
        fingerprint_sha256: public_key_fingerprint_sha256(&public_key),
        created_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
    }
}

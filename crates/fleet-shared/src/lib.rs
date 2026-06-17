use serde::{Deserialize, Serialize};

pub const API_VERSION: &str = "v1";
pub const MINER_MODELS: &[&str] = &["S21", "S21+", "S21 Pro", "S21 XP"];
pub const MINER_STATUSES: &[&str] = &["In Service", "Under Repair", "RMA", "Retired", "Spare"];
pub const PART_CATEGORIES: &[&str] = &["Hashboard", "Control Board", "PSU", "Fan", "Cable", "Misc"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub site_id: Option<i64>,
    pub site_name: Option<String>,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub site_id: Option<i64>,
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub id: i64,
    pub site_id: Option<i64>,
    pub display_name: String,
    pub role: UserRole,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub product: String,
    pub version: String,
    pub api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInfo {
    pub server: ServerInfo,
    pub certificate_pem: String,
    pub fingerprint_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Miner {
    pub id: i64,
    pub site_id: i64,
    pub site_name: Option<String>,
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMiner {
    pub site_id: Option<i64>,
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMiner {
    pub id: i64,
    pub site_id: Option<i64>,
    pub serial: String,
    pub model: String,
    pub firmware: Option<String>,
    pub client_name: Option<String>,
    pub miner_type: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub pickaxe: Option<String>,
    pub miner_state: Option<String>,
    pub miner_row: Option<String>,
    pub miner_index: Option<String>,
    pub miner_rack: Option<String>,
    pub miner_rack_group: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub acquired_date: Option<String>,
    pub notes: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerImportResult {
    pub imported: i64,
    pub updated: i64,
    pub skipped: i64,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub site_id: i64,
    pub site_name: Option<String>,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub qty_on_hand: i64,
    pub reorder_threshold: i64,
    pub supplier: Option<String>,
    pub unit_cost_cents: i64,
    pub notes: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePart {
    pub site_id: Option<i64>,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub qty_on_hand: i64,
    pub reorder_threshold: i64,
    pub supplier: Option<String>,
    pub unit_cost_cents: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountByStatus {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub unit_count: i64,
    pub part_count: i64,
    pub low_stock_count: i64,
    pub units_by_status: Vec<CountByStatus>,
    pub low_stock_parts: Vec<Part>,
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub target_serial: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogQuery {
    pub user_id: Option<i64>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: i64,
    pub name: String,
    pub url: String,
    /// Always returned as "********" when a secret is set; null when no secret.
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhook {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebhook {
    pub id: i64,
    pub name: String,
    pub url: String,
    /// null / "" / "********" → preserve existing secret.  Other non-empty value → replace.
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: i64,
    pub webhook_id: i64,
    pub event: String,
    pub payload: serde_json::Value,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub attempts: i32,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSite {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSite {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteQuery {
    pub site_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

pub fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

pub fn normalize_and_validate_miner(input: &mut CreateMiner) -> Result<(), String> {
    input.serial = input.serial.trim().to_string();
    if input.serial.is_empty() {
        return Err("serial must not be empty".into());
    }
    if !MINER_MODELS.contains(&input.model.as_str()) {
        return Err(format!("model must be one of {MINER_MODELS:?}"));
    }
    if !MINER_STATUSES.contains(&input.status.as_str()) {
        return Err(format!("status must be one of {MINER_STATUSES:?}"));
    }
    Ok(())
}

pub fn validate_part(input: &CreatePart) -> Result<(), String> {
    if input.sku.trim().is_empty() {
        return Err("sku must not be empty".into());
    }
    if input.name.trim().is_empty() {
        return Err("name must not be empty".into());
    }
    if !PART_CATEGORIES.contains(&input.category.as_str()) {
        return Err(format!("category must be one of {PART_CATEGORIES:?}"));
    }
    if input.qty_on_hand < 0 || input.reorder_threshold < 0 || input.unit_cost_cents < 0 {
        return Err("inventory quantities and cost must not be negative".into());
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 12 {
        return Err("password must be at least 12 characters".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames_are_canonical() {
        assert_eq!(normalize_username("  Admin.User "), "admin.user");
    }

    #[test]
    fn passwords_require_twelve_characters() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough-1").is_ok());
    }

    #[test]
    fn public_key_fingerprint_parses_ed25519() {
        let key =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWiVhcv4K4fFL test";
        let fp = public_key_fingerprint_sha256(key);
        assert!(fp.is_some());
        assert!(fp.unwrap().starts_with("SHA256:"));
    }
}

// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTunnelKeyRequest {
    pub label: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelKeyRequest {
    pub id: i64,
    pub label: String,
    pub public_key: String,
    pub status: String, // "pending" | "approved" | "rejected" | "revoked"
    pub note: Option<String>,
    pub status_token: String,
    pub fingerprint_sha256: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveTunnelKeyRequest {
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelClientConfig {
    pub ssh_destination: String,
    pub ssh_port: u16,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelKeyRequestStatus {
    pub id: i64,
    pub status: String,
    pub note: Option<String>,
    pub client_config: Option<TunnelClientConfig>,
}

/// OpenSSH SHA256 fingerprint (`SHA256:...`) or `None` when the key cannot be parsed.
pub fn public_key_fingerprint_sha256(public_key: &str) -> Option<String> {
    use ssh_key::{HashAlg, PublicKey};
    public_key
        .trim()
        .parse::<PublicKey>()
        .ok()
        .map(|key| key.fingerprint(HashAlg::Sha256).to_string())
}

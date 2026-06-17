use crate::client::{ClientState, ConnectionState};
use fleet_shared::{
    AuditLogEntry, AuditLogQuery, ChangePasswordRequest, CreateMiner, CreatePart, CreateSite,
    CreateUserRequest, CreateWebhook, DashboardSummary, LoginResponse, Miner, MinerImportResult,
    PairingInfo, Part, ResetPasswordRequest, Site, UpdateMiner, UpdateSite, UpdateUserRequest,
    UpdateWebhook, User, Webhook, WebhookDelivery,
    ApproveTunnelKeyRequest, SubmitTunnelKeyRequest, TunnelKeyRequest,
    TunnelKeyRequestStatus,
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri::State;

const DEFAULT_TUNNEL_PORT: u16 = 8443;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfigInput {
    pub ssh_destination: String,
    pub ssh_port: Option<u16>,
    pub identity_file: Option<String>,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelStatus {
    pub supported: bool,
    pub configured: bool,
    pub running: bool,
    pub local_port_open: bool,
    pub local_url: String,
    pub remote_target: String,
    pub process_id: Option<u32>,
    pub config_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelKeyInfo {
    pub identity_file: String,
    pub public_key_file: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TunnelConfigFile {
    ssh_destination: String,
    ssh_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity_file: Option<String>,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
}

#[tauri::command]
pub async fn get_connection_state(
    state: State<'_, ClientState>,
) -> Result<ConnectionState, String> {
    state.connection_state().await
}

#[tauri::command]
pub fn get_tunnel_status(app: AppHandle) -> Result<TunnelStatus, String> {
    tunnel_status(&app)
}

#[tauri::command]
pub fn save_tunnel_config(
    app: AppHandle,
    input: TunnelConfigInput,
) -> Result<TunnelStatus, String> {
    let config = normalize_tunnel_config(input)?;
    let path = tunnel_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    std::fs::write(&path, format!("{json}\n")).map_err(|error| error.to_string())?;
    start_tunnel_with_config(&app, &path)?;
    tunnel_status(&app)
}

#[tauri::command]
pub fn start_tunnel_connection(app: AppHandle) -> Result<TunnelStatus, String> {
    let config_path = tunnel_config_path()?;
    if !config_path.exists() {
        return Ok(unconfigured_tunnel_status(Some(
            "Tunnel config has not been created yet.".into(),
        )));
    }
    start_tunnel_with_config(&app, &config_path)?;
    tunnel_status(&app)
}

#[tauri::command]
pub fn generate_tunnel_key() -> Result<TunnelKeyInfo, String> {
    let identity = default_identity_path()?;
    if let Some(parent) = identity.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    if !identity.exists() {
        let status = std::process::Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "antminer-fleet-tunnel",
                "-f",
            ])
            .arg(&identity)
            .status()
            .map_err(|error| format!("Could not run ssh-keygen: {error}"))?;
        if !status.success() {
            return Err(format!(
                "ssh-keygen failed with exit code {}",
                status.code().unwrap_or(-1)
            ));
        }
    }

    let public_key_path = identity.with_extension("pub");
    if !public_key_path.exists() {
        let output = std::process::Command::new("ssh-keygen")
            .args(["-y", "-f"])
            .arg(&identity)
            .output()
            .map_err(|error| format!("Could not export public key: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        std::fs::write(&public_key_path, &output.stdout).map_err(|error| error.to_string())?;
    }

    let public_key = std::fs::read_to_string(&public_key_path)
        .map_err(|error| format!("Could not read public key: {error}"))?
        .trim()
        .to_string();
    Ok(TunnelKeyInfo {
        identity_file: identity.display().to_string(),
        public_key_file: public_key_path.display().to_string(),
        public_key,
    })
}

#[tauri::command]
pub async fn probe_server(url: String) -> Result<PairingInfo, String> {
    ClientState::probe(&url).await
}

#[tauri::command]
pub async fn pair_server(
    state: State<'_, ClientState>,
    url: String,
    certificate_pem: String,
    fingerprint_sha256: String,
) -> Result<(), String> {
    state.pair(url, certificate_pem, fingerprint_sha256).await
}

#[tauri::command]
pub async fn unpair_server(state: State<'_, ClientState>) -> Result<(), String> {
    state.unpair().await
}

#[tauri::command]
pub async fn login(
    state: State<'_, ClientState>,
    username: String,
    password: String,
) -> Result<LoginResponse, String> {
    state.login(username, password).await
}

#[tauri::command]
pub async fn logout(state: State<'_, ClientState>) -> Result<(), String> {
    state.logout().await
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, ClientState>,
    input: ChangePasswordRequest,
) -> Result<(), String> {
    state.put_empty("/api/v1/auth/password", &input).await?;
    state.clear_token()
}

#[tauri::command]
pub async fn list_users(state: State<'_, ClientState>) -> Result<Vec<User>, String> {
    state.get("/api/v1/users").await
}

#[tauri::command]
pub async fn create_user(
    state: State<'_, ClientState>,
    input: CreateUserRequest,
) -> Result<User, String> {
    state.post("/api/v1/users", &input).await
}

#[tauri::command]
pub async fn update_user(
    state: State<'_, ClientState>,
    input: UpdateUserRequest,
    id: i64,
) -> Result<User, String> {
    state.put(&format!("/api/v1/users/{id}"), &input).await
}

#[tauri::command]
pub async fn reset_user_password(
    state: State<'_, ClientState>,
    id: i64,
    input: ResetPasswordRequest,
) -> Result<(), String> {
    state
        .put_empty(&format!("/api/v1/users/{id}/password"), &input)
        .await
}

#[tauri::command]
pub async fn list_miners(
    state: State<'_, ClientState>,
    site_id: Option<i64>,
) -> Result<Vec<Miner>, String> {
    let path = match site_id {
        Some(id) => format!("/api/v1/miners?site_id={id}"),
        None => "/api/v1/miners".to_string(),
    };
    state.get(&path).await
}

#[tauri::command]
pub async fn create_miner(
    state: State<'_, ClientState>,
    input: CreateMiner,
) -> Result<i64, String> {
    state
        .post::<_, Miner>("/api/v1/miners", &input)
        .await
        .map(|miner| miner.id)
}

#[tauri::command]
pub async fn update_miner(state: State<'_, ClientState>, input: UpdateMiner) -> Result<(), String> {
    state
        .put::<_, Miner>(&format!("/api/v1/miners/{}", input.id), &input)
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn import_miners(
    state: State<'_, ClientState>,
    miners: Vec<CreateMiner>,
) -> Result<MinerImportResult, String> {
    state.post("/api/v1/miners/import", &miners).await
}

#[tauri::command]
pub async fn delete_miner(
    state: State<'_, ClientState>,
    id: i64,
    version: i64,
) -> Result<(), String> {
    state
        .delete(&format!("/api/v1/miners/{id}?version={version}"))
        .await
}

#[tauri::command]
pub async fn list_parts(
    state: State<'_, ClientState>,
    site_id: Option<i64>,
) -> Result<Vec<Part>, String> {
    let path = match site_id {
        Some(id) => format!("/api/v1/parts?site_id={id}"),
        None => "/api/v1/parts".to_string(),
    };
    state.get(&path).await
}

#[tauri::command]
pub async fn create_part(state: State<'_, ClientState>, input: CreatePart) -> Result<(), String> {
    state
        .post::<_, Part>("/api/v1/parts", &input)
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn update_part(state: State<'_, ClientState>, input: Part) -> Result<(), String> {
    state
        .put::<_, Part>(&part_path(&input.sku), &input)
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn delete_part(
    state: State<'_, ClientState>,
    sku: String,
    version: i64,
    site_id: Option<i64>,
) -> Result<(), String> {
    let mut path = format!("{}?version={version}", part_path(&sku));
    if let Some(id) = site_id {
        path.push_str(&format!("&site_id={id}"));
    }
    state.delete(&path).await
}

fn part_path(sku: &str) -> String {
    format!("/api/v1/parts/{}", urlencoding::encode(sku))
}

#[tauri::command]
pub async fn get_dashboard_summary(
    state: State<'_, ClientState>,
    site_id: Option<i64>,
) -> Result<DashboardSummary, String> {
    let path = match site_id {
        Some(id) => format!("/api/v1/dashboard?site_id={id}"),
        None => "/api/v1/dashboard".to_string(),
    };
    state.get(&path).await
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_audit_log(
    state: State<'_, ClientState>,
    query: AuditLogQuery,
) -> Result<Vec<AuditLogEntry>, String> {
    let mut params: Vec<String> = Vec::new();
    if let Some(v) = query.user_id {
        params.push(format!("user_id={v}"));
    }
    if let Some(ref v) = query.action {
        params.push(format!("action={}", urlencoding::encode(v)));
    }
    if let Some(ref v) = query.target_type {
        params.push(format!("target_type={}", urlencoding::encode(v)));
    }
    if let Some(ref v) = query.target_id {
        params.push(format!("target_id={}", urlencoding::encode(v)));
    }
    if let Some(ref v) = query.from {
        params.push(format!("from={}", urlencoding::encode(v)));
    }
    if let Some(ref v) = query.to {
        params.push(format!("to={}", urlencoding::encode(v)));
    }
    if let Some(v) = query.limit {
        params.push(format!("limit={v}"));
    }
    if let Some(v) = query.offset {
        params.push(format!("offset={v}"));
    }
    let path = if params.is_empty() {
        "/api/v1/audit-log".to_string()
    } else {
        format!("/api/v1/audit-log?{}", params.join("&"))
    };
    state.get(&path).await
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_webhooks(state: State<'_, ClientState>) -> Result<Vec<Webhook>, String> {
    state.get("/api/v1/webhooks").await
}

#[tauri::command]
pub async fn create_webhook(
    state: State<'_, ClientState>,
    input: CreateWebhook,
) -> Result<Webhook, String> {
    state.post("/api/v1/webhooks", &input).await
}

#[tauri::command]
pub async fn update_webhook(
    state: State<'_, ClientState>,
    input: UpdateWebhook,
) -> Result<Webhook, String> {
    state
        .put(&format!("/api/v1/webhooks/{}", input.id), &input)
        .await
}

#[tauri::command]
pub async fn delete_webhook(
    state: State<'_, ClientState>,
    id: i64,
    version: i64,
) -> Result<(), String> {
    state
        .delete(&format!("/api/v1/webhooks/{id}?version={version}"))
        .await
}

#[tauri::command]
pub async fn list_webhook_deliveries(
    state: State<'_, ClientState>,
    id: i64,
) -> Result<Vec<WebhookDelivery>, String> {
    state
        .get(&format!("/api/v1/webhooks/{id}/deliveries"))
        .await
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_sites(state: State<'_, ClientState>) -> Result<Vec<Site>, String> {
    state.get("/api/v1/sites").await
}

#[tauri::command]
pub async fn create_site(state: State<'_, ClientState>, input: CreateSite) -> Result<Site, String> {
    state.post("/api/v1/sites", &input).await
}

#[tauri::command]
pub async fn update_site(state: State<'_, ClientState>, input: UpdateSite) -> Result<Site, String> {
    state
        .put(&format!("/api/v1/sites/{}", input.id), &input)
        .await
}

#[tauri::command]
pub async fn delete_site(
    state: State<'_, ClientState>,
    id: i64,
    version: i64,
) -> Result<(), String> {
    state
        .delete(&format!("/api/v1/sites/{id}?version={version}"))
        .await
}

#[cfg(target_os = "windows")]
fn tunnel_config_path() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| "Could not locate LOCALAPPDATA for tunnel configuration".to_string())?;
    Ok(base
        .join("AntminerFleetManager")
        .join("fleet-tunnel.local.json"))
}

#[cfg(not(target_os = "windows"))]
fn tunnel_config_path() -> Result<PathBuf, String> {
    Err("SSH tunnel setup is only supported by the Windows desktop build".into())
}

#[cfg(target_os = "windows")]
fn tunnel_script_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_script = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("fleet-tunnel.ps1"))
        .filter(|path| path.exists());
    if let Some(path) = resource_script {
        return Ok(path);
    }

    let dev_script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("scripts")
        .join("fleet-tunnel.ps1");
    if dev_script.exists() {
        return Ok(dev_script);
    }

    Err("Bundled tunnel script was not found.".into())
}

#[cfg(not(target_os = "windows"))]
fn tunnel_script_path(_app: &AppHandle) -> Result<PathBuf, String> {
    Err("SSH tunnel setup is only supported by the Windows desktop build".into())
}

fn normalize_tunnel_config(input: TunnelConfigInput) -> Result<TunnelConfigFile, String> {
    let ssh_destination = input.ssh_destination.trim().to_string();
    if ssh_destination.is_empty() || ssh_destination.contains("CHANGE_ME") {
        return Err(
            "Enter the SSH destination assigned to this user, for example username@10.83.1.120."
                .into(),
        );
    }
    if ssh_destination.split_whitespace().count() != 1 {
        return Err(
            "SSH destination must be one host alias or USER@HOST value, with no spaces.".into(),
        );
    }

    let local_port = input.local_port.unwrap_or(DEFAULT_TUNNEL_PORT);
    let remote_port = input.remote_port.unwrap_or(DEFAULT_TUNNEL_PORT);
    let ssh_port = input.ssh_port.unwrap_or(22);
    if local_port == 0 || remote_port == 0 || ssh_port == 0 {
        return Err("Ports must be between 1 and 65535.".into());
    }

    let remote_host = input
        .remote_host
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("127.0.0.1")
        .to_string();
    let identity_file = input
        .identity_file
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(TunnelConfigFile {
        ssh_destination,
        ssh_port,
        identity_file,
        local_port,
        remote_host,
        remote_port,
    })
}

fn unconfigured_tunnel_status(error: Option<String>) -> TunnelStatus {
    TunnelStatus {
        supported: cfg!(target_os = "windows"),
        configured: false,
        running: false,
        local_port_open: false,
        local_url: format!("https://localhost:{DEFAULT_TUNNEL_PORT}"),
        remote_target: format!("127.0.0.1:{DEFAULT_TUNNEL_PORT}"),
        process_id: None,
        config_path: tunnel_config_path()
            .ok()
            .map(|path| path.display().to_string()),
        error,
    }
}

#[cfg(target_os = "windows")]
fn run_tunnel_script(
    app: &AppHandle,
    action: &str,
    config_path: &std::path::Path,
) -> Result<String, String> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script_path = tunnel_script_path(app)?;
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NonInteractive",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script_path)
        .args(["-Action", action, "-Config"])
        .arg(config_path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("Could not run SSH tunnel helper: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let message = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };
        Err(message.trim().to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn run_tunnel_script(
    _app: &AppHandle,
    _action: &str,
    _config_path: &std::path::Path,
) -> Result<String, String> {
    Err("SSH tunnel setup is only supported by the Windows desktop build".into())
}

fn start_tunnel_with_config(app: &AppHandle, config_path: &std::path::Path) -> Result<(), String> {
    run_tunnel_script(app, "Start", config_path).map(|_| ())
}

fn tunnel_status(app: &AppHandle) -> Result<TunnelStatus, String> {
    let config_path = match tunnel_config_path() {
        Ok(path) => path,
        Err(error) => return Ok(unconfigured_tunnel_status(Some(error))),
    };
    if !config_path.exists() {
        return Ok(unconfigured_tunnel_status(None));
    }

    let output = match run_tunnel_script(app, "Status", &config_path) {
        Ok(output) => output,
        Err(error) => {
            return Ok(TunnelStatus {
                supported: cfg!(target_os = "windows"),
                configured: true,
                running: false,
                local_port_open: false,
                local_url: format!("https://localhost:{DEFAULT_TUNNEL_PORT}"),
                remote_target: format!("127.0.0.1:{DEFAULT_TUNNEL_PORT}"),
                process_id: None,
                config_path: Some(config_path.display().to_string()),
                error: Some(error),
            })
        }
    };

    Ok(parse_tunnel_status(&output, config_path))
}

fn parse_tunnel_status(output: &str, config_path: PathBuf) -> TunnelStatus {
    let field = |name: &str| -> Option<String> {
        output.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.trim().eq_ignore_ascii_case(name) {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
    };
    let running = field("Running")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let local_port_open = field("LocalPortOpen")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let process_id = field("ProcessId").and_then(|value| value.parse::<u32>().ok());
    let local_url =
        field("LocalUrl").unwrap_or_else(|| format!("https://localhost:{DEFAULT_TUNNEL_PORT}"));
    let remote_target =
        field("RemoteTarget").unwrap_or_else(|| format!("127.0.0.1:{DEFAULT_TUNNEL_PORT}"));

    TunnelStatus {
        supported: cfg!(target_os = "windows"),
        configured: true,
        running,
        local_port_open,
        local_url,
        remote_target,
        process_id,
        config_path: Some(config_path.display().to_string()),
        error: None,
    }
}

fn default_identity_path() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or_else(|| "Could not determine user profile directory.".to_string())?
        .join(".ssh")
        .join("antminer_fleet_tunnel"))
}

#[cfg(test)]
mod tests {
    use super::{normalize_tunnel_config, parse_tunnel_status, part_path, TunnelConfigInput};

    #[test]
    fn part_paths_encode_reserved_characters() {
        assert_eq!(
            part_path("PSU / S21?#1"),
            "/api/v1/parts/PSU%20%2F%20S21%3F%231"
        );
    }

    #[test]
    fn tunnel_config_defaults_to_user_supplied_destination() {
        let config = normalize_tunnel_config(TunnelConfigInput {
            ssh_destination: "alice@10.83.1.120".into(),
            ssh_port: None,
            identity_file: Some("".into()),
            local_port: None,
            remote_host: None,
            remote_port: None,
        })
        .unwrap();

        assert_eq!(config.ssh_destination, "alice@10.83.1.120");
        assert_eq!(config.ssh_port, 22);
        assert_eq!(config.local_port, 8443);
        assert_eq!(config.remote_host, "127.0.0.1");
        assert_eq!(config.remote_port, 8443);
        assert!(config.identity_file.is_none());
    }

    #[test]
    fn tunnel_status_parser_reads_powershell_output() {
        let status = parse_tunnel_status(
            "Running       : True\nProcessId     : 1234\nLocalUrl      : https://localhost:8443\nLocalPortOpen : True\nRemoteTarget  : 127.0.0.1:8443\n",
            std::path::PathBuf::from("C:/config.json"),
        );

        assert!(status.configured);
        assert!(status.running);
        assert!(status.local_port_open);
        assert_eq!(status.process_id, Some(1234));
        assert_eq!(status.local_url, "https://localhost:8443");
        assert_eq!(status.remote_target, "127.0.0.1:8443");
    }
}

// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn submit_tunnel_key_request(
    server_url: String,
    input: SubmitTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    ClientState::post_no_auth_to_url(&server_url, "/api/v1/tunnel-key-requests", &input).await
}

#[tauri::command]
pub async fn list_tunnel_key_requests(
    state: State<'_, ClientState>,
) -> Result<Vec<TunnelKeyRequest>, String> {
    state.get("/api/v1/tunnel-key-requests").await
}

#[tauri::command]
pub async fn approve_tunnel_key_request(
    state: State<'_, ClientState>,
    id: i64,
    input: ApproveTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    state
        .post(&format!("/api/v1/tunnel-key-requests/{id}/approve"), &input)
        .await
}

#[tauri::command]
pub async fn reject_tunnel_key_request(
    state: State<'_, ClientState>,
    id: i64,
    input: ApproveTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    state
        .post(
            &format!("/api/v1/tunnel-key-requests/{id}/reject"),
            &input,
        )
        .await
}

#[tauri::command]
pub async fn revoke_tunnel_key_request(
    state: State<'_, ClientState>,
    id: i64,
    input: ApproveTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    state
        .post(
            &format!("/api/v1/tunnel-key-requests/{id}/revoke"),
            &input,
        )
        .await
}

#[tauri::command]
pub async fn get_tunnel_key_request_status(
    server_url: String,
    id: i64,
    token: String,
) -> Result<TunnelKeyRequestStatus, String> {
    ClientState::get_no_auth_to_url(
        &server_url,
        &format!("/api/v1/tunnel-key-requests/{id}/status?token={token}"),
    )
    .await
}

#[tauri::command]
pub fn save_tunnel_key_onboarding(
    app: AppHandle,
    state: crate::tunnel_onboarding::TunnelKeyOnboardingState,
) -> Result<(), String> {
    crate::tunnel_onboarding::save_tunnel_key_onboarding(&app, &state)
}

#[tauri::command]
pub fn load_tunnel_key_onboarding(
    app: AppHandle,
) -> Result<Option<crate::tunnel_onboarding::TunnelKeyOnboardingState>, String> {
    crate::tunnel_onboarding::load_tunnel_key_onboarding(&app)
}

#[tauri::command]
pub fn clear_tunnel_key_onboarding(app: AppHandle) -> Result<(), String> {
    crate::tunnel_onboarding::clear_tunnel_key_onboarding(&app)
}

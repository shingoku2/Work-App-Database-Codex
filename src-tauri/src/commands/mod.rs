use crate::client::{ClientState, ConnectionState};
use fleet_shared::{
    AuditLogEntry, AuditLogQuery, ChangePasswordRequest, CreateMiner, CreatePart, CreateSite,
    CreateUserRequest, CreateWebhook, DashboardSummary, LoginResponse, Miner, MinerImportResult,
    PairingInfo, Part, ResetPasswordRequest, Site, UpdateMiner, UpdateSite, UpdateUserRequest,
    UpdateWebhook, User, Webhook, WebhookDelivery,
};
use tauri::State;

#[tauri::command]
pub async fn get_connection_state(
    state: State<'_, ClientState>,
) -> Result<ConnectionState, String> {
    state.connection_state().await
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
pub async fn current_user(state: State<'_, ClientState>) -> Result<User, String> {
    state.get("/api/v1/auth/me").await
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
    state.get(&format!("/api/v1/webhooks/{id}/deliveries")).await
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_sites(state: State<'_, ClientState>) -> Result<Vec<Site>, String> {
    state.get("/api/v1/sites").await
}

#[tauri::command]
pub async fn create_site(
    state: State<'_, ClientState>,
    input: CreateSite,
) -> Result<Site, String> {
    state.post("/api/v1/sites", &input).await
}

#[tauri::command]
pub async fn update_site(
    state: State<'_, ClientState>,
    input: UpdateSite,
) -> Result<Site, String> {
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

#[cfg(test)]
mod tests {
    use super::part_path;

    #[test]
    fn part_paths_encode_reserved_characters() {
        assert_eq!(
            part_path("PSU / S21?#1"),
            "/api/v1/parts/PSU%20%2F%20S21%3F%231"
        );
    }
}

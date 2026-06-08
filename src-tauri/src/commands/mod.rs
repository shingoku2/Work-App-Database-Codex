use crate::client::{ClientState, ConnectionState};
use fleet_shared::{
    ChangePasswordRequest, CreateMiner, CreatePart, CreateUserRequest, DashboardSummary,
    LoginResponse, Miner, MinerImportResult, PairingInfo, Part, ResetPasswordRequest, UpdateMiner,
    UpdateUserRequest, User,
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
pub async fn list_miners(state: State<'_, ClientState>) -> Result<Vec<Miner>, String> {
    state.get("/api/v1/miners").await
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
pub async fn list_parts(state: State<'_, ClientState>) -> Result<Vec<Part>, String> {
    state.get("/api/v1/parts").await
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
) -> Result<(), String> {
    state
        .delete(&format!("{}?version={version}", part_path(&sku)))
        .await
}

fn part_path(sku: &str) -> String {
    format!("/api/v1/parts/{}", urlencoding::encode(sku))
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

#[tauri::command]
pub async fn get_dashboard_summary(
    state: State<'_, ClientState>,
) -> Result<DashboardSummary, String> {
    state.get("/api/v1/dashboard").await
}

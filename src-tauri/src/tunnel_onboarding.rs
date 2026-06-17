use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const ONBOARDING_FILE: &str = "tunnel_key_onboarding.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelKeyOnboardingState {
    pub request_id: Option<i64>,
    pub status_token: Option<String>,
    pub label: String,
    pub public_key: String,
    pub server_url: String,
    pub identity_file: String,
}

fn onboarding_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&app_data).map_err(|error| error.to_string())?;
    Ok(app_data.join(ONBOARDING_FILE))
}

pub fn save_tunnel_key_onboarding(
    app: &AppHandle,
    state: &TunnelKeyOnboardingState,
) -> Result<(), String> {
    let path = onboarding_path(app)?;
    let json = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

pub fn load_tunnel_key_onboarding(
    app: &AppHandle,
) -> Result<Option<TunnelKeyOnboardingState>, String> {
    let path = onboarding_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let json = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&json)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub fn clear_tunnel_key_onboarding(app: &AppHandle) -> Result<(), String> {
    let path = onboarding_path(app)?;
    if path.is_file() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

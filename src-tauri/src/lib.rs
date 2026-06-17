mod client;
mod commands;
mod tunnel_onboarding;

use tauri::Manager;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            start_tunnel(app);
            let state = client::ClientState::load(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_connection_state,
            commands::get_tunnel_status,
            commands::generate_tunnel_key,
            commands::save_tunnel_config,
            commands::start_tunnel_connection,
            commands::probe_server,
            commands::pair_server,
            commands::unpair_server,
            commands::login,
            commands::logout,
            commands::change_password,
            commands::list_users,
            commands::create_user,
            commands::update_user,
            commands::reset_user_password,
            commands::list_miners,
            commands::create_miner,
            commands::update_miner,
            commands::import_miners,
            commands::delete_miner,
            commands::list_parts,
            commands::create_part,
            commands::update_part,
            commands::delete_part,
            commands::get_dashboard_summary,
            commands::list_audit_log,
            commands::list_webhooks,
            commands::create_webhook,
            commands::update_webhook,
            commands::delete_webhook,
            commands::list_webhook_deliveries,
            commands::list_sites,
            commands::create_site,
            commands::update_site,
            commands::delete_site,
            commands::submit_tunnel_key_request,
            commands::list_tunnel_key_requests,
            commands::approve_tunnel_key_request,
            commands::reject_tunnel_key_request,
            commands::revoke_tunnel_key_request,
            commands::get_tunnel_key_request_status,
            commands::save_tunnel_key_onboarding,
            commands::load_tunnel_key_onboarding,
            commands::clear_tunnel_key_onboarding,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(target_os = "windows")]
fn start_tunnel(app: &tauri::App) {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let script_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("fleet-tunnel.ps1"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("scripts")
                .join("fleet-tunnel.ps1")
        });
    if !script_path.exists() {
        return;
    }

    let Some(config_path) = tunnel_config_path() else {
        return;
    };
    if !config_path.exists() {
        return;
    }

    let mut command = std::process::Command::new("powershell.exe");
    command
        .args([
            "-NonInteractive",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .args(["-Action", "Start", "-Config"])
        .arg(&config_path)
        .creation_flags(CREATE_NO_WINDOW);
    let _ = command.status();
}

#[cfg(not(target_os = "windows"))]
fn start_tunnel(_app: &tauri::App) {}

#[cfg(target_os = "windows")]
fn tunnel_config_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|directory| {
        directory
            .join("AntminerFleetManager")
            .join("fleet-tunnel.local.json")
    })
}

mod client;
mod commands;

use tauri::Manager;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn start_tunnel(app: &tauri::App) {
    // Locate the bundled script; fall back to the dev-time source path.
    let script_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("fleet-tunnel.ps1"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("scripts")
                .join("fleet-tunnel.ps1")
        });

    if !script_path.exists() {
        return;
    }

    // Config lives next to the script in dev; in production the user places it
    // in the well-known LOCALAPPDATA location used by the installer helpers.
    let config_path = {
        let sibling = script_path
            .parent()
            .unwrap()
            .join("fleet-tunnel.local.json");
        if sibling.exists() {
            sibling
        } else {
            let mut p = dirs::data_local_dir().unwrap_or_default();
            p.push("AntminerFleetManager");
            p.push("fleet-tunnel.local.json");
            p
        }
    };

    if !config_path.exists() {
        return; // Not configured on this machine — skip silently.
    }

    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args([
        "-NonInteractive",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script_path.to_str().unwrap_or_default(),
        "-Action",
        "Start",
        "-Config",
        config_path.to_str().unwrap_or_default(),
    ]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let _ = cmd.spawn();
}

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
            commands::probe_server,
            commands::pair_server,
            commands::unpair_server,
            commands::login,
            commands::logout,
            commands::current_user,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

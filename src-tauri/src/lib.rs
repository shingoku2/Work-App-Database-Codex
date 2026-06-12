mod client;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
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
            commands::list_webhooks,
            commands::create_webhook,
            commands::update_webhook,
            commands::delete_webhook,
            commands::list_webhook_deliveries,
            commands::list_sites,
            commands::create_site,
            commands::update_site,
            commands::delete_site,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

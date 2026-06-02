mod commands;
mod db;
mod models;

use tauri_plugin_sql::{Migration, MigrationKind};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "initial_schema",
            sql: include_str!("../migrations/0001_initial_schema.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "remove_ticketing",
            sql: include_str!("../migrations/0003_remove_ticketing.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "miner_import_fields",
            sql: include_str!("../migrations/0004_miner_import_fields.sql"),
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:fleet.db", migrations)
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = db::init_pool(&handle).await?;
                handle.manage(pool);
                Ok::<(), String>(())
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::miners::list_miners,
            commands::miners::create_miner,
            commands::miners::update_miner,
            commands::miners::import_miners,
            commands::miners::delete_miner,
            commands::parts::list_parts,
            commands::parts::create_part,
            commands::parts::update_part,
            commands::parts::delete_part,
            commands::dashboard::get_dashboard_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

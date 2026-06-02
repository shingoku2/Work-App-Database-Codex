use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tauri::{AppHandle, Manager};

pub type DbPool = SqlitePool;

pub async fn init_pool(app: &AppHandle) -> Result<DbPool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;

    let db_path = app_data_dir.join("fleet.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|error| format!("Failed to open SQLite database: {error}"))?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .map_err(|error| format!("Failed to enable SQLite foreign keys: {error}"))?;

    run_migrations(
        &pool,
        &[
            (1, include_str!("../migrations/0001_initial_schema.sql")),
            (3, include_str!("../migrations/0003_remove_ticketing.sql")),
            (4, include_str!("../migrations/0004_miner_import_fields.sql")),
        ],
    )
    .await?;

    Ok(pool)
}

async fn run_migrations(pool: &DbPool, migrations: &[(i64, &str)]) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
    )
    .execute(pool)
    .await
    .map_err(|error| format!("Failed to initialize migration table: {error}"))?;

    for (version, sql) in migrations {
        let already_applied: Option<i64> =
            sqlx::query_scalar("SELECT version FROM schema_migrations WHERE version = ?1")
                .bind(version)
                .fetch_optional(pool)
                .await
                .map_err(|error| format!("Failed to check migration version {version}: {error}"))?;

        if already_applied.is_none() {
            run_migration(pool, sql).await?;
            sqlx::query("INSERT INTO schema_migrations (version) VALUES (?1)")
                .bind(version)
                .execute(pool)
                .await
                .map_err(|error| format!("Failed to record migration version {version}: {error}"))?;
        }
    }

    Ok(())
}

async fn run_migration(pool: &DbPool, sql: &str) -> Result<(), String> {
    for statement in sql.split(';').map(str::trim).filter(|statement| !statement.is_empty()) {
        if let Err(error) = sqlx::query(statement).execute(pool).await {
            let message = error.to_string();
            if !message.contains("duplicate column name") {
                return Err(format!("Failed to apply database migration: {message}"));
            }
        }
    }

    Ok(())
}

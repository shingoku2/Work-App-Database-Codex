mod api;
mod auth;
mod config;
mod import;

use clap::{Parser, Subcommand};
use config::ServerConfig;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "antminer-fleet-server", version)]
struct Cli {
    #[arg(long, default_value = "/etc/antminer-fleet/server.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve,
    Migrate,
    ValidateConfig,
    CreateAdmin {
        username: String,
        display_name: String,
        #[arg(long)]
        password_stdin: bool,
    },
    ResetPassword {
        username: String,
        #[arg(long)]
        password_stdin: bool,
    },
    GenerateTls {
        #[arg(long)]
        host: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    ImportSqlite {
        path: PathBuf,
        #[arg(long)]
        apply: bool,
        #[arg(long, value_enum, default_value_t = import::ConflictPolicy::Abort)]
        conflict: import::ConflictPolicy,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "antminer_fleet_server=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = ServerConfig::load(&cli.config)?;

    match cli.command {
        Command::ValidateConfig => {
            config.validate_deployable().await?;
            println!("configuration is valid");
        }
        Command::GenerateTls { host, force } => config.generate_tls(&host, force)?,
        command => {
            config.validate_base()?;
            if matches!(command, Command::Serve) {
                config.validate_deployable().await?;
            }
            let pool = PgPoolOptions::new()
                .max_connections(config.database.max_connections)
                .connect(&config.database.url)
                .await?;
            sqlx::migrate!("./migrations").run(&pool).await?;

            match command {
                Command::Serve => {
                    api::serve(config, pool).await?;
                }
                Command::Migrate => {
                    println!("database migrations applied");
                }
                Command::CreateAdmin {
                    username,
                    display_name,
                    password_stdin,
                } => {
                    let password = auth::resolve_password(password_stdin)?;
                    auth::create_admin(&pool, &username, &display_name, &password).await?;
                    println!("administrator created");
                }
                Command::ResetPassword {
                    username,
                    password_stdin,
                } => {
                    let password = auth::resolve_password(password_stdin)?;
                    auth::reset_password(&pool, &username, &password).await?;
                    println!("password reset and existing sessions revoked");
                }
                Command::ImportSqlite {
                    path,
                    apply,
                    conflict,
                } => import::run(&pool, &path, apply, conflict).await?,
                Command::ValidateConfig | Command::GenerateTls { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}

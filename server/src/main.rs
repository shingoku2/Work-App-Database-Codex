mod api;
mod auth;
mod config;
mod import;

use clap::{Parser, Subcommand, ValueEnum};
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
    Backup {
        #[arg(long, default_value = "backup.sql")]
        output: PathBuf,
        #[arg(long, default_value = "plain", value_enum)]
        format: BackupFormat,
        #[arg(long)]
        compress: bool,
    },
    Restore {
        input: PathBuf,
        #[arg(long)]
        clean: bool,
        #[arg(long)]
        no_owner: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum BackupFormat {
    Plain,
    Custom,
    Directory,
    Tar,
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
                Command::Backup {
                    output,
                    format,
                    compress,
                } => {
                    run_backup(&config.database.url, &output, format, compress).await?;
                }
                Command::Restore {
                    input,
                    clean,
                    no_owner,
                } => {
                    run_restore(&config.database.url, &input, clean, no_owner).await?;
                }
                Command::ValidateConfig | Command::GenerateTls { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}

async fn run_backup(
    database_url: &str,
    output: &PathBuf,
    format: BackupFormat,
    compress: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let format_flag = match format {
        BackupFormat::Plain => "-Fp",
        BackupFormat::Custom => "-Fc",
        BackupFormat::Directory => "-Fd",
        BackupFormat::Tar => "-Ft",
    };

    let mut args = vec![
        "pg_dump",
        database_url,
        format_flag,
        "-f",
        output.to_str().ok_or("invalid output path")?,
    ];

    if compress && format != BackupFormat::Plain {
        args.push("-Z");
        args.push("9");
    } else if compress && format == BackupFormat::Plain {
        eprintln!("Warning: compression not supported for plain format, ignoring --compress");
    }

    let status = tokio::process::Command::new(args[0])
        .args(&args[1..])
        .status()
        .await?;

    if !status.success() {
        return Err("pg_dump failed".into());
    }

    println!("backup written to {}", output.display());
    Ok(())
}

async fn run_restore(
    database_url: &str,
    input: &PathBuf,
    clean: bool,
    no_owner: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("input file does not exist: {}", input.display()).into());
    }

    let mut args = vec!["pg_restore", "-d", database_url];

    if clean {
        args.push("--clean");
    }
    if no_owner {
        args.push("--no-owner");
    }

    args.push(input.to_str().ok_or("invalid input path")?);

    let status = tokio::process::Command::new(args[0])
        .args(&args[1..])
        .status()
        .await?;

    if !status.success() {
        return Err("pg_restore failed".into());
    }

    println!("restore completed from {}", input.display());
    Ok(())
}

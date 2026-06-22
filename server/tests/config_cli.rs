use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn test_directory(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "fleet-server-config-{label}-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("test directory should be created");
    directory
}

fn write_config(
    directory: &Path,
    database_url: &str,
    max_connections: u32,
    session_days: i64,
    same_tls_path: bool,
) -> PathBuf {
    write_config_with_tunnel(
        directory,
        database_url,
        max_connections,
        session_days,
        same_tls_path,
        "antminer-fleet-client-tunnel@127.0.0.1",
    )
}

fn write_config_with_tunnel(
    directory: &Path,
    database_url: &str,
    max_connections: u32,
    session_days: i64,
    same_tls_path: bool,
    tunnel_destination: &str,
) -> PathBuf {
    let certificate = directory.join("server.crt");
    let private_key = if same_tls_path {
        certificate.clone()
    } else {
        directory.join("server.key")
    };
    let config = directory.join("server.toml");
    let tunnel_client_block = format!(
        "[tunnel_client]\nssh_destination = \"{tunnel_destination}\"\n"
    );
    fs::write(
        &config,
        format!(
            "listen = \"127.0.0.1:8443\"\n\
             session_days = {session_days}\n\
             [database]\n\
             url = \"{database_url}\"\n\
             max_connections = {max_connections}\n\
             [tls]\n\
             certificate = '{}'\n\
             private_key = '{}'\n\
             {tunnel_client_block}",
            certificate.display(),
            private_key.display(),
        ),
    )
    .expect("test configuration should be written");
    config
}

fn validate(config: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_antminer-fleet-server"))
        .args(["--config", config.to_str().unwrap(), "validate-config"])
        .output()
        .expect("server CLI should run")
}

fn assert_rejected(output: Output, expected: &str) {
    assert!(
        !output.status.success(),
        "invalid configuration was accepted"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected stderr to contain {expected:?}, got {stderr:?}"
    );
}

#[test]
fn validate_config_rejects_non_postgres_and_placeholder_database_urls() {
    let directory = test_directory("database");

    let config = write_config(&directory, "sqlite://fleet.db", 10, 30, false);
    assert_rejected(
        validate(&config),
        "must use the postgres or postgresql scheme",
    );

    let config = write_config(
        &directory,
        "postgres://fleet:replace-with-secret@127.0.0.1/fleet",
        10,
        30,
        false,
    );
    assert_rejected(validate(&config), "placeholder credential");

    fs::remove_dir_all(directory).expect("test directory should be removed");
}

#[test]
fn validate_config_rejects_unsafe_pool_and_session_ranges() {
    let directory = test_directory("ranges");

    let config = write_config(
        &directory,
        "postgres://fleet:secret@127.0.0.1/fleet",
        0,
        30,
        false,
    );
    assert_rejected(
        validate(&config),
        "max_connections must be between 1 and 100",
    );

    let config = write_config(
        &directory,
        "postgres://fleet:secret@127.0.0.1/fleet",
        10,
        366,
        false,
    );
    assert_rejected(validate(&config), "session_days must be between 1 and 365");

    fs::remove_dir_all(directory).expect("test directory should be removed");
}

#[test]
fn validate_config_rejects_reusing_one_file_for_certificate_and_key() {
    let directory = test_directory("tls-paths");
    let config = write_config(
        &directory,
        "postgres://fleet:secret@127.0.0.1/fleet",
        10,
        30,
        true,
    );

    assert_rejected(validate(&config), "must use different files");

    fs::remove_dir_all(directory).expect("test directory should be removed");
}

#[test]
fn validate_config_rejects_change_me_tunnel_destination() {
    let directory = test_directory("tunnel-change-me");
    let config = write_config_with_tunnel(
        &directory,
        "postgres://fleet:secret@127.0.0.1/fleet",
        10,
        30,
        false,
        "antminer-fleet-client-tunnel@CHANGE_ME",
    );

    assert_rejected(validate(&config), "placeholder host");

    fs::remove_dir_all(directory).expect("test directory should be removed");
}

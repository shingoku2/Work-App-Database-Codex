use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn script_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("scripts/{name}"))
}

fn temp_authorized_keys(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "fleet-tunnel-keys-{label}-{}-{suffix}",
        std::process::id()
    ));
    fs::write(&path, "").expect("authorized_keys stub should be created");
    path
}

#[test]
fn authorize_and_revoke_client_tunnel_keys() {
    if cfg!(windows) {
        return;
    }

    let authorized_keys = temp_authorized_keys("lifecycle");
    let public_key_material =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWiVhcv4K4fFL";
    let public_key = format!("{public_key_material} test");

    let authorize = Command::new("sh")
        .arg(script_path("authorize-client-tunnel-key.sh"))
        .env(
            "ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS",
            &authorized_keys,
        )
        .args(["--label", "alice-laptop", "--public-key", &public_key])
        .output()
        .expect("authorize script should run");
    assert!(
        authorize.status.success(),
        "authorize failed: {}",
        String::from_utf8_lossy(&authorize.stderr)
    );

    let content = fs::read_to_string(&authorized_keys).expect("authorized_keys should be readable");
    assert!(content.contains("antminer-fleet-client:alice-laptop"));
    assert!(content.contains(public_key_material));

    let revoke = Command::new("sh")
        .arg(script_path("revoke-client-tunnel-key.sh"))
        .env(
            "ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS",
            &authorized_keys,
        )
        .args(["--label", "alice-laptop"])
        .output()
        .expect("revoke script should run");
    assert!(
        revoke.status.success(),
        "revoke failed: {}",
        String::from_utf8_lossy(&revoke.stderr)
    );

    let after_revoke =
        fs::read_to_string(&authorized_keys).expect("authorized_keys should be readable");
    assert!(!after_revoke.contains("antminer-fleet-client:alice-laptop"));

    let revoke_again = Command::new("sh")
        .arg(script_path("revoke-client-tunnel-key.sh"))
        .env(
            "ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS",
            &authorized_keys,
        )
        .args(["--label", "alice-laptop"])
        .output()
        .expect("second revoke script should run");
    assert_eq!(
        revoke_again.status.code(),
        Some(2),
        "missing marker should exit 2"
    );

    let _ = fs::remove_file(authorized_keys);
}

use rcgen::{generate_simple_self_signed, CertifiedKey};
use serde::Deserialize;
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub database: DatabaseConfig,
    pub tls: TlsConfig,
    #[serde(default = "default_tunnel_client")]
    pub tunnel_client: TunnelClientConfig,
    #[serde(default = "default_session_days")]
    pub session_days: i64,
}

#[derive(Clone, Deserialize)]
pub struct TunnelClientConfig {
    pub ssh_destination: String,
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    #[serde(default = "default_local_port")]
    pub local_port: u16,
    #[serde(default = "default_remote_host")]
    pub remote_host: String,
    #[serde(default = "default_remote_port")]
    pub remote_port: u16,
}

#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_connections")]
    pub max_connections: u32,
}

#[derive(Clone, Deserialize)]
pub struct TlsConfig {
    pub certificate: PathBuf,
    pub private_key: PathBuf,
}

fn default_connections() -> u32 {
    10
}

fn default_session_days() -> i64 {
    30
}

fn default_ssh_port() -> u16 {
    22
}

fn default_local_port() -> u16 {
    8443
}

fn default_remote_host() -> String {
    "127.0.0.1".into()
}

fn default_remote_port() -> u16 {
    8443
}

fn default_tunnel_client() -> TunnelClientConfig {
    TunnelClientConfig {
        ssh_destination: "antminer-fleet-client-tunnel@CHANGE_ME".into(),
        ssh_port: default_ssh_port(),
        local_port: default_local_port(),
        remote_host: default_remote_host(),
        remote_port: default_remote_port(),
    }
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn validate_base(&self) -> Result<(), String> {
        let database_url = Url::parse(self.database.url.trim())
            .map_err(|error| format!("invalid database.url: {error}"))?;
        if !matches!(database_url.scheme(), "postgres" | "postgresql") {
            return Err("database.url must use the postgres or postgresql scheme".into());
        }
        let normalized_url = self.database.url.to_ascii_lowercase();
        if ["change_me", "changeme", "replace_me", "replace-with"]
            .iter()
            .any(|placeholder| normalized_url.contains(placeholder))
        {
            return Err("database.url still contains a placeholder credential".into());
        }
        if database_url.host_str().is_none() || database_url.path().trim_matches('/').is_empty() {
            return Err("database.url must include a host and database name".into());
        }
        if !(1..=100).contains(&self.database.max_connections) {
            return Err("database.max_connections must be between 1 and 100".into());
        }
        if !(1..=365).contains(&self.session_days) {
            return Err("session_days must be between 1 and 365".into());
        }
        self.validate_tunnel_client()?;
        self.validate_tls_paths()?;
        Ok(())
    }

    fn validate_tunnel_client(&self) -> Result<(), String> {
        let destination = self.tunnel_client.ssh_destination.trim();
        if destination.is_empty() {
            return Err("tunnel_client.ssh_destination must not be empty".into());
        }
        let normalized = destination.to_ascii_lowercase();
        if ["change_me", "changeme", "replace_me", "replace-with"]
            .iter()
            .any(|placeholder| normalized.contains(placeholder))
        {
            return Err(
                "tunnel_client.ssh_destination still contains a placeholder host".into(),
            );
        }
        if !destination.contains('@') {
            return Err(
                "tunnel_client.ssh_destination must be user@host, e.g. antminer-fleet-client-tunnel@ssh.example".into(),
            );
        }
        if !(1..=65535).contains(&self.tunnel_client.ssh_port) {
            return Err("tunnel_client.ssh_port must be between 1 and 65535".into());
        }
        if !(1..=65535).contains(&self.tunnel_client.local_port) {
            return Err("tunnel_client.local_port must be between 1 and 65535".into());
        }
        if self.tunnel_client.remote_host.trim().is_empty() {
            return Err("tunnel_client.remote_host must not be empty".into());
        }
        if !(1..=65535).contains(&self.tunnel_client.remote_port) {
            return Err("tunnel_client.remote_port must be between 1 and 65535".into());
        }
        Ok(())
    }

    pub async fn validate_deployable(&self) -> Result<(), String> {
        self.validate_base()?;
        if !self.tls.certificate.is_file() {
            return Err(format!(
                "TLS certificate does not exist: {}",
                self.tls.certificate.display()
            ));
        }
        if !self.tls.private_key.is_file() {
            return Err(format!(
                "TLS private key does not exist: {}",
                self.tls.private_key.display()
            ));
        }
        axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &self.tls.certificate,
            &self.tls.private_key,
        )
        .await
        .map_err(|error| {
            format!("TLS certificate/private key are invalid or do not match: {error}")
        })?;
        Ok(())
    }

    pub fn generate_tls(
        &self,
        hosts: &[String],
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_tls_paths()?;
        if hosts.is_empty() {
            return Err("at least one --host DNS name or IP address is required".into());
        }
        if !force && (self.tls.certificate.exists() || self.tls.private_key.exists()) {
            return Err("TLS output already exists; pass --force to replace it".into());
        }
        if let Some(parent) = self.tls.certificate.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = self.tls.private_key.parent() {
            fs::create_dir_all(parent)?;
        }
        let CertifiedKey { cert, signing_key } = generate_simple_self_signed(hosts.to_vec())?;
        fs::write(&self.tls.certificate, cert.pem())?;
        fs::write(&self.tls.private_key, signing_key.serialize_pem())?;
        println!("certificate written to {}", self.tls.certificate.display());
        println!("private key written to {}", self.tls.private_key.display());
        Ok(())
    }

    fn validate_tls_paths(&self) -> Result<(), String> {
        if self.tls.certificate.as_os_str().is_empty()
            || self.tls.private_key.as_os_str().is_empty()
        {
            return Err("TLS certificate and private-key paths must not be empty".into());
        }
        if self.tls.certificate == self.tls.private_key {
            return Err("TLS certificate and private key must use different files".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ServerConfig {
        ServerConfig {
            listen: "127.0.0.1:8443".parse().unwrap(),
            database: DatabaseConfig {
                url: "postgres://fleet:secret@127.0.0.1/fleet".into(),
                max_connections: 10,
            },
            tls: TlsConfig {
                certificate: "server.crt".into(),
                private_key: "server.key".into(),
            },
            tunnel_client: TunnelClientConfig {
                ssh_destination: "antminer-fleet-client-tunnel@127.0.0.1".into(),
                ssh_port: 22,
                local_port: 8443,
                remote_host: "127.0.0.1".into(),
                remote_port: 8443,
            },
            session_days: 30,
        }
    }

    #[test]
    fn base_validation_rejects_invalid_database_and_ranges() {
        let mut value = config();
        value.database.url = "sqlite://fleet.db".into();
        assert!(value.validate_base().is_err());

        let mut value = config();
        value.database.max_connections = 0;
        assert!(value.validate_base().is_err());

        let mut value = config();
        value.database.url = "postgres://fleet:replace-with-secret@127.0.0.1/fleet".into();
        assert!(value.validate_base().is_err());

        let mut value = config();
        value.session_days = 366;
        assert!(value.validate_base().is_err());

        let mut value = config();
        value.tunnel_client.ssh_destination = "antminer-fleet-client-tunnel@CHANGE_ME".into();
        assert!(value.validate_base().is_err());
    }
}

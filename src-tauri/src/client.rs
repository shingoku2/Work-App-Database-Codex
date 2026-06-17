use fleet_shared::{ApiError, LoginRequest, LoginResponse, PairingInfo, ServerInfo, User};
use keyring::Entry;
use reqwest::{Method, StatusCode};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;
use url::Url;

const CREDENTIAL_SERVICE: &str = "antminer-fleet-manager";
const CREDENTIAL_ACCOUNT: &str = "active-session";
const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConnectionConfig {
    schema_version: u32,
    url: String,
    certificate_pem: String,
    fingerprint_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Unpaired,
    Unauthenticated,
    Authenticated,
    Unavailable,
    RepairRequired,
}

#[derive(Debug, Serialize)]
pub struct ConnectionState {
    pub paired: bool,
    pub status: ConnectionStatus,
    pub url: Option<String>,
    pub fingerprint_sha256: Option<String>,
    pub user: Option<User>,
    pub error: Option<String>,
}

pub struct ClientState {
    config_path: PathBuf,
    config: RwLock<Option<ConnectionConfig>>,
    load_error: RwLock<Option<String>>,
}

impl ClientState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        fs::create_dir_all(&app_data).map_err(|error| error.to_string())?;
        let config_path = app_data.join("server.json");
        let (config, load_error) = load_config(&config_path);
        Ok(Self {
            config_path,
            config: RwLock::new(config),
            load_error: RwLock::new(load_error),
        })
    }

    pub async fn probe(url: &str) -> Result<PairingInfo, String> {
        let base_url = normalize_url(url)?;
        let response = one_shot_request::<()>(Method::GET, &base_url, "/pairing", None)
            .await
            .map_err(|error| format!("Could not reach server: {error}"))?;
        parse_response(response).await
    }

    /// POST without a bearer token to an explicit URL, before the client is
    /// paired. Used for unauthenticated pre-pair endpoints (currently
    /// `POST /api/v1/tunnel-key-requests`). The TLS certificate is NOT pinned
    /// here — the user is expected to compare the server fingerprint when
    /// they pair, which is when the cert gets pinned.
    pub async fn get_no_auth_to_url<T: DeserializeOwned>(
        url: &str,
        path: &str,
    ) -> Result<T, String> {
        let base_url = normalize_url(url)?;
        let response = one_shot_request::<()>(Method::GET, &base_url, path, None)
            .await
            .map_err(network_error)?;
        parse_response(response).await
    }

    pub async fn post_no_auth_to_url<B: Serialize + ?Sized, T: DeserializeOwned>(
        url: &str,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let base_url = normalize_url(url)?;
        let response = one_shot_request(Method::POST, &base_url, path, Some(body))
            .await
            .map_err(network_error)?;
        parse_response(response).await
    }

    pub async fn pair(
        &self,
        url: String,
        certificate_pem: String,
        fingerprint_sha256: String,
    ) -> Result<(), String> {
        let url = normalize_url(&url)?;
        let actual_fingerprint = certificate_fingerprint(&certificate_pem)?;
        if actual_fingerprint != fingerprint_sha256 {
            return Err("certificate fingerprint does not match the pairing response".into());
        }
        let config = ConnectionConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            url,
            certificate_pem,
            fingerprint_sha256,
        };
        let client = build_client(&config)?;
        let server: ServerInfo = parse_response(
            client
                .get(join_url(&config.url, "/health")?)
                .send()
                .await
                .map_err(|error| format!("Pinned certificate verification failed: {error}"))?,
        )
        .await?;
        if server.api_version != fleet_shared::API_VERSION {
            return Err(format!(
                "server API version {} is not supported",
                server.api_version
            ));
        }
        save_config(&self.config_path, &config)?;
        *self.config.write().await = Some(config);
        *self.load_error.write().await = None;
        self.clear_token()
    }

    pub async fn unpair(&self) -> Result<(), String> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path).map_err(|error| error.to_string())?;
        }
        *self.config.write().await = None;
        *self.load_error.write().await = None;
        self.clear_token()
    }

    pub async fn connection_state(&self) -> Result<ConnectionState, String> {
        let config = self.config.read().await.clone();
        let Some(config) = config else {
            let load_error = self.load_error.read().await.clone();
            return Ok(ConnectionState {
                paired: false,
                status: if load_error.is_some() {
                    ConnectionStatus::RepairRequired
                } else {
                    ConnectionStatus::Unpaired
                },
                url: None,
                fingerprint_sha256: None,
                user: None,
                error: load_error,
            });
        };
        let Some(token) = self.read_token()? else {
            return Ok(ConnectionState {
                paired: true,
                status: ConnectionStatus::Unauthenticated,
                url: Some(config.url),
                fingerprint_sha256: Some(config.fingerprint_sha256),
                user: None,
                error: None,
            });
        };
        let client = build_client(&config)?;
        let response = match client
            .get(join_url(&config.url, "/api/v1/auth/me")?)
            .bearer_auth(token)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(ConnectionState {
                    paired: true,
                    status: ConnectionStatus::Unavailable,
                    url: Some(config.url),
                    fingerprint_sha256: Some(config.fingerprint_sha256),
                    user: None,
                    error: Some(network_error(error)),
                })
            }
        };
        if response.status() == StatusCode::UNAUTHORIZED {
            self.clear_token()?;
            return Ok(ConnectionState {
                paired: true,
                status: ConnectionStatus::Unauthenticated,
                url: Some(config.url),
                fingerprint_sha256: Some(config.fingerprint_sha256),
                user: None,
                error: None,
            });
        }
        let user = match parse_response::<User>(response).await {
            Ok(user) => Some(user),
            Err(error) => {
                return Ok(ConnectionState {
                    paired: true,
                    status: ConnectionStatus::Unavailable,
                    url: Some(config.url),
                    fingerprint_sha256: Some(config.fingerprint_sha256),
                    user: None,
                    error: Some(error),
                })
            }
        };
        Ok(ConnectionState {
            paired: true,
            status: ConnectionStatus::Authenticated,
            url: Some(config.url),
            fingerprint_sha256: Some(config.fingerprint_sha256),
            user,
            error: None,
        })
    }

    pub async fn login(&self, username: String, password: String) -> Result<LoginResponse, String> {
        let config = self.require_config().await?;
        let client = build_client(&config)?;
        let response: LoginResponse = parse_response(
            client
                .post(join_url(&config.url, "/api/v1/auth/login")?)
                .json(&LoginRequest { username, password })
                .send()
                .await
                .map_err(network_error)?,
        )
        .await?;
        credential_entry()?
            .set_password(&response.token)
            .map_err(|error| format!("Could not store session credential: {error}"))?;
        Ok(response)
    }

    pub async fn logout(&self) -> Result<(), String> {
        let result = self.post_empty("/api/v1/auth/logout", &()).await;
        self.clear_token()?;
        result
    }

    pub fn clear_token(&self) -> Result<(), String> {
        match credential_entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("Could not clear session credential: {error}")),
        }
    }

    fn read_token(&self) -> Result<Option<String>, String> {
        match credential_entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("Could not read session credential: {error}")),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        self.request(Method::GET, path, Option::<&()>::None).await
    }

    pub async fn post<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        self.request(Method::POST, path, Some(body)).await
    }

    pub async fn put<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        self.request(Method::PUT, path, Some(body)).await
    }

    pub async fn post_empty<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), String> {
        self.request_empty(Method::POST, path, Some(body)).await
    }

    pub async fn put_empty<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), String> {
        self.request_empty(Method::PUT, path, Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<(), String> {
        self.request_empty(Method::DELETE, path, Option::<&()>::None)
            .await
    }

    async fn request<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, String> {
        let response = self.send(method, path, body).await?;
        parse_response(response).await
    }

    async fn request_empty<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<(), String> {
        let response = self.send(method, path, body).await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(response_error(response).await)
        }
    }

    async fn send<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<reqwest::Response, String> {
        let config = self.require_config().await?;
        let token = self
            .read_token()?
            .ok_or_else(|| "AUTH_REQUIRED: authentication required".to_string())?;
        let client = build_client(&config)?;
        let mut request = client
            .request(method, join_url(&config.url, path)?)
            .bearer_auth(token);
        if let Some(body) = body {
            request = request.json(body);
        }
        request.send().await.map_err(network_error)
    }

    async fn require_config(&self) -> Result<ConnectionConfig, String> {
        self.config
            .read()
            .await
            .clone()
            .ok_or_else(|| "server is not configured".into())
    }
}

fn normalize_url(input: &str) -> Result<String, String> {
    let mut url =
        Url::parse(input.trim()).map_err(|error| format!("Invalid server URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("server URL must use https".into());
    }
    if url.host_str().is_none() {
        return Err("server URL must include a host".into());
    }
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn join_url(base: &str, path: &str) -> Result<Url, String> {
    Url::parse(&format!("{}{}", base.trim_end_matches('/'), path))
        .map_err(|error| error.to_string())
}

fn certificate_fingerprint(pem: &str) -> Result<String, String> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let der = rustls_pemfile::certs(&mut reader)
        .next()
        .ok_or_else(|| "certificate PEM contains no certificate".to_string())?
        .map_err(|error| error.to_string())?;
    Ok(Sha256::digest(der.as_ref())
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":"))
}

fn build_client(config: &ConnectionConfig) -> Result<reqwest::Client, String> {
    let expected_certificate = certificate_der(&config.certificate_pem)?;
    let provider = Arc::new(ring::default_provider());
    let tls = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedCertificateVerifier {
            expected_certificate,
            provider,
        }))
        .with_no_client_auth();
    reqwest::Client::builder()
        .https_only(true)
        .tls_built_in_root_certs(false)
        .use_preconfigured_tls(tls)
        .build()
        .map_err(|error| error.to_string())
}

fn save_config(path: &Path, config: &ConnectionConfig) -> Result<(), String> {
    let text = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Could not atomically save server configuration: {error}")
    })
}

fn load_config(path: &Path) -> (Option<ConnectionConfig>, Option<String>) {
    if !path.exists() {
        return (None, None);
    }
    let loaded = fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            serde_json::from_str::<ConnectionConfig>(&text).map_err(|error| error.to_string())
        })
        .and_then(validate_connection_config);
    match loaded {
        Ok(config) => (Some(config), None),
        Err(error) => {
            let quarantine = path.with_extension("invalid.json");
            let quarantine_result = fs::rename(path, &quarantine);
            let message = match quarantine_result {
                Ok(()) => format!(
                    "Saved server configuration was invalid and moved to {}: {error}",
                    quarantine.display()
                ),
                Err(rename_error) => format!(
                    "Saved server configuration is invalid: {error}. It could not be quarantined: {rename_error}"
                ),
            };
            (None, Some(message))
        }
    }
}

fn validate_connection_config(config: ConnectionConfig) -> Result<ConnectionConfig, String> {
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(format!(
            "unsupported server profile schema version {}",
            config.schema_version
        ));
    }
    if normalize_url(&config.url)? != config.url {
        return Err("server profile contains a non-canonical URL".into());
    }
    let actual = certificate_fingerprint(&config.certificate_pem)?;
    if actual != config.fingerprint_sha256 {
        return Err("server profile certificate fingerprint does not match".into());
    }
    Ok(config)
}

fn certificate_der(pem: &str) -> Result<Vec<u8>, String> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let result = rustls_pemfile::certs(&mut reader)
        .next()
        .ok_or_else(|| "certificate PEM contains no certificate".to_string())?
        .map(|certificate| certificate.as_ref().to_vec())
        .map_err(|error| error.to_string());
    result
}

#[derive(Debug)]
struct PinnedCertificateVerifier {
    expected_certificate: Vec<u8>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedCertificateVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if end_entity.as_ref() == self.expected_certificate {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "server certificate does not match the paired certificate".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn credential_entry() -> Result<Entry, String> {
    Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT).map_err(|error| error.to_string())
}

fn network_error(error: reqwest::Error) -> String {
    format!("Server connection failed: {error}")
}

/// Build a one-shot reqwest client that does not pin the server certificate,
/// and perform a single request to `base_url` + `path`. Used for endpoints
/// that must be reachable before the client has ever paired (e.g. `/pairing`,
/// `POST /api/v1/tunnel-key-requests`). Certificate trust is the user's
/// responsibility at this stage — pairing and fingerprint comparison are what
/// turn the URL into a pinned identity.
async fn one_shot_request<B: Serialize + ?Sized>(
    method: Method,
    base_url: &str,
    path: &str,
    body: Option<&B>,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .https_only(true)
        .build()
        .expect("static one-shot HTTPS client must build");
    let mut request = client.request(method, join_url(base_url, path).expect("url is canonical"));
    if let Some(body) = body {
        request = request.json(body);
    }
    request.send().await
}

async fn parse_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    if response.status().is_success() {
        response.json().await.map_err(|error| error.to_string())
    } else {
        Err(response_error(response).await)
    }
}

async fn response_error(response: reqwest::Response) -> String {
    let status = response.status();
    let parsed = response.json::<ApiError>().await.ok();
    match parsed {
        Some(error) if status == StatusCode::CONFLICT => {
            format!("VERSION_CONFLICT: {}", error.message)
        }
        Some(error) if status == StatusCode::UNAUTHORIZED => {
            format!("AUTH_REQUIRED: {}", error.message)
        }
        Some(error) => error.message,
        None => format!("server returned HTTP {status}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::CertificateDer;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn pinned_verifier_rejects_every_other_certificate() {
        let provider = Arc::new(ring::default_provider());
        let verifier = PinnedCertificateVerifier {
            expected_certificate: vec![1, 2, 3],
            provider,
        };
        let name = ServerName::try_from("fleet.example").unwrap();
        assert!(verifier
            .verify_server_cert(
                &CertificateDer::from(vec![1, 2, 3]),
                &[],
                &name,
                &[],
                UnixTime::since_unix_epoch(std::time::Duration::from_secs(1)),
            )
            .is_ok());
        assert!(verifier
            .verify_server_cert(
                &CertificateDer::from(vec![4, 5, 6]),
                &[],
                &name,
                &[],
                UnixTime::since_unix_epoch(std::time::Duration::from_secs(1)),
            )
            .is_err());
    }

    #[test]
    fn malformed_saved_config_is_quarantined() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("fleet-client-config-{suffix}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("server.json");
        fs::write(&path, "{not-json").unwrap();

        let (config, error) = load_config(&path);

        assert!(config.is_none());
        assert!(error.is_some());
        assert!(!path.exists());
        assert!(directory.join("server.invalid.json").exists());
        fs::remove_dir_all(directory).unwrap();
    }
}

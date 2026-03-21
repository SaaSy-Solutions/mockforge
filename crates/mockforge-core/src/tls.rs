//! Shared TLS utilities for MockForge protocol crates.
//!
//! This module provides a common [`TlsConfig`] struct and builder functions
//! for creating `rustls` [`ServerConfig`](tokio_rustls::rustls::ServerConfig)
//! and [`ClientConfig`](tokio_rustls::rustls::ClientConfig) instances.
//!
//! Protocol crates (MQTT, AMQP, SMTP, TCP, etc.) that need TLS support can
//! use these helpers instead of duplicating certificate-loading logic.
//!
//! # Examples
//!
//! ```rust,no_run
//! use mockforge_core::tls::TlsConfig;
//!
//! let config = TlsConfig::new("certs/server.pem", "certs/server-key.pem");
//! let server_tls = mockforge_core::tls::build_server_tls_config(&config).unwrap();
//! ```

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Errors that can occur during TLS configuration.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    /// The certificate file was not found at the specified path.
    #[error("TLS certificate file not found: {0}")]
    CertNotFound(String),

    /// The private key file was not found at the specified path.
    #[error("TLS private key file not found: {0}")]
    KeyNotFound(String),

    /// Failed to read the certificate file.
    #[error("Failed to read certificate: {0}")]
    CertReadError(String),

    /// Failed to read the private key file.
    #[error("Failed to read private key: {0}")]
    KeyReadError(String),

    /// The certificate file contained no valid certificates.
    #[error("No certificates found in certificate file")]
    NoCertificates,

    /// The key file contained no valid private key.
    #[error("No private key found in key file")]
    NoPrivateKey,

    /// A general TLS configuration error.
    #[error("TLS configuration error: {0}")]
    ConfigError(String),
}

/// TLS configuration holding paths to certificate, key, and optional CA files.
///
/// This is a protocol-agnostic configuration struct. Protocol crates can
/// convert their own config types into `TlsConfig` before calling the
/// shared builder functions.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to the PEM-encoded certificate chain file.
    pub cert_path: PathBuf,
    /// Path to the PEM-encoded private key file.
    pub key_path: PathBuf,
    /// Optional path to a PEM-encoded CA certificate file for client/server verification.
    pub ca_path: Option<PathBuf>,
}

impl TlsConfig {
    /// Create a new `TlsConfig` with cert and key paths.
    pub fn new(cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_path: None,
        }
    }

    /// Set the CA certificate path (for client auth verification or custom root CAs).
    pub fn with_ca(mut self, ca_path: impl Into<PathBuf>) -> Self {
        self.ca_path = Some(ca_path.into());
        self
    }
}

/// Load PEM-encoded certificates from a file.
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = File::open(path)
        .map_err(|e| TlsError::CertReadError(format!("{}: {}", path.display(), e)))?;
    let mut reader = BufReader::new(file);

    let certs_result: Vec<CertificateDer<'static>> =
        certs(&mut reader).filter_map(|c| c.ok()).collect();

    if certs_result.is_empty() {
        return Err(TlsError::NoCertificates);
    }

    Ok(certs_result)
}

/// Load a PEM-encoded private key from a file.
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let file = File::open(path)
        .map_err(|e| TlsError::KeyReadError(format!("{}: {}", path.display(), e)))?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)
        .map_err(|e| TlsError::KeyReadError(e.to_string()))?
        .ok_or(TlsError::NoPrivateKey)
}

/// Build a rustls [`ServerConfig`](tokio_rustls::rustls::ServerConfig) from the given [`TlsConfig`].
///
/// If `config.ca_path` is set, client certificate verification is enabled
/// using the CA certificates from that file. Otherwise, no client authentication
/// is required.
///
/// # Errors
///
/// Returns [`TlsError`] if certificate/key files cannot be read or the
/// configuration is invalid.
pub fn build_server_tls_config(
    config: &TlsConfig,
) -> Result<Arc<tokio_rustls::rustls::ServerConfig>, TlsError> {
    // Verify files exist
    if !config.cert_path.exists() {
        return Err(TlsError::CertNotFound(config.cert_path.display().to_string()));
    }
    if !config.key_path.exists() {
        return Err(TlsError::KeyNotFound(config.key_path.display().to_string()));
    }

    let certs_vec = load_certs(&config.cert_path)?;
    let key = load_private_key(&config.key_path)?;

    let provider = rustls::crypto::ring::default_provider();
    // Install as process-level default (ignored if already installed by another thread).
    // This is needed because WebPkiClientVerifier::builder().build() looks up the
    // process-level CryptoProvider internally.
    let _ = provider.clone().install_default();

    let server_config = if let Some(ca_path) = &config.ca_path {
        // Client-auth mode: require client certificates signed by the CA.
        if !ca_path.exists() {
            return Err(TlsError::CertNotFound(format!("CA certificate: {}", ca_path.display())));
        }

        let ca_certs = load_certs(ca_path)?;
        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| TlsError::ConfigError(format!("Failed to add CA cert: {}", e)))?;
        }

        let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
            .build()
            .map_err(|e| {
                TlsError::ConfigError(format!("Failed to create client verifier: {}", e))
            })?;

        rustls::ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs_vec, key)
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
    } else {
        // No client auth.
        rustls::ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
            .with_no_client_auth()
            .with_single_cert(certs_vec, key)
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
    };

    Ok(Arc::new(server_config))
}

/// Build a rustls [`ClientConfig`](tokio_rustls::rustls::ClientConfig) from the given [`TlsConfig`].
///
/// If `config.ca_path` is set, the CA certificates are used as trusted roots
/// instead of the system default roots. The client certificate and key from
/// `cert_path` / `key_path` are presented for mutual TLS if the server requests
/// client authentication.
///
/// # Errors
///
/// Returns [`TlsError`] if certificate/key files cannot be read or the
/// configuration is invalid.
pub fn build_client_tls_config(
    config: &TlsConfig,
) -> Result<Arc<tokio_rustls::rustls::ClientConfig>, TlsError> {
    // Verify files exist
    if !config.cert_path.exists() {
        return Err(TlsError::CertNotFound(config.cert_path.display().to_string()));
    }
    if !config.key_path.exists() {
        return Err(TlsError::KeyNotFound(config.key_path.display().to_string()));
    }

    let certs_vec = load_certs(&config.cert_path)?;
    let key = load_private_key(&config.key_path)?;

    let provider = rustls::crypto::ring::default_provider();

    // Build root cert store
    let mut root_store = rustls::RootCertStore::empty();

    if let Some(ca_path) = &config.ca_path {
        if !ca_path.exists() {
            return Err(TlsError::CertNotFound(format!("CA certificate: {}", ca_path.display())));
        }
        let ca_certs = load_certs(ca_path)?;
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| TlsError::ConfigError(format!("Failed to add CA cert: {}", e)))?;
        }
    } else {
        // Use webpki roots as default trusted CAs
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }

    let client_config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| TlsError::ConfigError(e.to_string()))?
        .with_root_certificates(root_store)
        .with_client_auth_cert(certs_vec, key)
        .map_err(|e| TlsError::ConfigError(e.to_string()))?;

    Ok(Arc::new(client_config))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Self-signed test certificate and key generated for unit testing only.
    // These are NOT real credentials.

    fn write_test_cert_and_key(dir: &tempfile::TempDir) -> (PathBuf, PathBuf) {
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");

        // Generate a self-signed cert+key with rcgen
        let subject_alt_names = vec!["localhost".to_string()];
        let cert_params =
            rcgen::CertificateParams::new(subject_alt_names).expect("Failed to create cert params");
        let key_pair = rcgen::KeyPair::generate().expect("Failed to generate key pair");
        let cert = cert_params.self_signed(&key_pair).expect("Failed to self-sign cert");

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        std::fs::write(&cert_path, cert_pem).unwrap();
        std::fs::write(&key_path, key_pem).unwrap();

        (cert_path, key_path)
    }

    #[test]
    fn test_tls_config_new() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem");
        assert_eq!(config.cert_path, PathBuf::from("/tmp/cert.pem"));
        assert_eq!(config.key_path, PathBuf::from("/tmp/key.pem"));
        assert!(config.ca_path.is_none());
    }

    #[test]
    fn test_tls_config_with_ca() {
        let config = TlsConfig::new("/tmp/cert.pem", "/tmp/key.pem").with_ca("/tmp/ca.pem");
        assert_eq!(config.ca_path, Some(PathBuf::from("/tmp/ca.pem")));
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::CertNotFound("/path/to/cert.pem".to_string());
        assert!(err.to_string().contains("/path/to/cert.pem"));

        let err = TlsError::NoCertificates;
        assert!(err.to_string().contains("No certificates"));

        let err = TlsError::NoPrivateKey;
        assert!(err.to_string().contains("No private key"));

        let err = TlsError::ConfigError("bad config".to_string());
        assert!(err.to_string().contains("bad config"));
    }

    #[test]
    fn test_build_server_tls_config_cert_not_found() {
        let config = TlsConfig::new("/nonexistent/cert.pem", "/nonexistent/key.pem");
        let result = build_server_tls_config(&config);
        assert!(matches!(result, Err(TlsError::CertNotFound(_))));
    }

    #[test]
    fn test_build_server_tls_config_key_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("cert.pem");
        std::fs::write(&cert_path, "placeholder").unwrap();

        let config = TlsConfig::new(&cert_path, "/nonexistent/key.pem");
        let result = build_server_tls_config(&config);
        assert!(matches!(result, Err(TlsError::KeyNotFound(_))));
    }

    #[test]
    fn test_build_server_tls_config_empty_cert() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, "").unwrap();
        std::fs::write(&key_path, "").unwrap();

        let config = TlsConfig::new(&cert_path, &key_path);
        let result = build_server_tls_config(&config);
        assert!(matches!(result, Err(TlsError::NoCertificates)));
    }

    #[test]
    fn test_build_server_tls_config_valid() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        let config = TlsConfig::new(&cert_path, &key_path);
        let result = build_server_tls_config(&config);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    #[test]
    fn test_build_server_tls_config_with_client_auth() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        // Use the same cert as CA for testing
        let ca_path = dir.path().join("ca.pem");
        std::fs::copy(&cert_path, &ca_path).unwrap();

        let config = TlsConfig::new(&cert_path, &key_path).with_ca(&ca_path);
        let result = build_server_tls_config(&config);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    #[test]
    fn test_build_server_tls_config_ca_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        let config = TlsConfig::new(&cert_path, &key_path).with_ca("/nonexistent/ca.pem");
        let result = build_server_tls_config(&config);
        assert!(matches!(result, Err(TlsError::CertNotFound(_))));
    }

    #[test]
    fn test_build_client_tls_config_cert_not_found() {
        let config = TlsConfig::new("/nonexistent/cert.pem", "/nonexistent/key.pem");
        let result = build_client_tls_config(&config);
        assert!(matches!(result, Err(TlsError::CertNotFound(_))));
    }

    #[test]
    fn test_build_client_tls_config_valid_with_ca() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        let ca_path = dir.path().join("ca.pem");
        std::fs::copy(&cert_path, &ca_path).unwrap();

        let config = TlsConfig::new(&cert_path, &key_path).with_ca(&ca_path);
        let result = build_client_tls_config(&config);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    #[test]
    fn test_build_client_tls_config_valid_default_roots() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        let config = TlsConfig::new(&cert_path, &key_path);
        let result = build_client_tls_config(&config);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    #[test]
    fn test_build_client_tls_config_ca_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let (cert_path, key_path) = write_test_cert_and_key(&dir);

        let config = TlsConfig::new(&cert_path, &key_path).with_ca("/nonexistent/ca.pem");
        let result = build_client_tls_config(&config);
        assert!(matches!(result, Err(TlsError::CertNotFound(_))));
    }
}

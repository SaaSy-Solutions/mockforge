//! TLS support for MQTT connections
//!
//! This module provides TLS/SSL encryption for MQTT connections using rustls.

use crate::broker::MqttConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

/// Error type for TLS configuration
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("TLS certificate file not found: {0}")]
    CertNotFound(String),
    #[error("TLS private key file not found: {0}")]
    KeyNotFound(String),
    #[error("Failed to read certificate: {0}")]
    CertReadError(String),
    #[error("Failed to read private key: {0}")]
    KeyReadError(String),
    #[error("No certificates found in certificate file")]
    NoCertificates,
    #[error("No private key found in key file")]
    NoPrivateKey,
    #[error("TLS configuration error: {0}")]
    ConfigError(String),
    #[error("TLS is enabled but certificate path is not configured")]
    CertPathNotConfigured,
    #[error("TLS is enabled but key path is not configured")]
    KeyPathNotConfigured,
}

/// Load certificates from a PEM file
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = File::open(path)
        .map_err(|e| TlsError::CertReadError(format!("{}: {}", path.display(), e)))?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer<'static>> = certs(&mut reader).filter_map(|c| c.ok()).collect();

    if certs.is_empty() {
        return Err(TlsError::NoCertificates);
    }

    Ok(certs)
}

/// Load private key from a PEM file
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let file = File::open(path)
        .map_err(|e| TlsError::KeyReadError(format!("{}: {}", path.display(), e)))?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)
        .map_err(|e| TlsError::KeyReadError(e.to_string()))?
        .ok_or(TlsError::NoPrivateKey)
}

/// Create a TLS acceptor from MQTT configuration
pub fn create_tls_acceptor(config: &MqttConfig) -> Result<TlsAcceptor, TlsError> {
    let cert_path = config.tls_cert_path.as_ref().ok_or(TlsError::CertPathNotConfigured)?;

    let key_path = config.tls_key_path.as_ref().ok_or(TlsError::KeyPathNotConfigured)?;

    // Verify files exist
    if !cert_path.exists() {
        return Err(TlsError::CertNotFound(cert_path.display().to_string()));
    }
    if !key_path.exists() {
        return Err(TlsError::KeyNotFound(key_path.display().to_string()));
    }

    // Load certificates and private key
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    // Build server config
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::ConfigError(e.to_string()))?;

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

/// Create a TLS acceptor with optional client authentication
pub fn create_tls_acceptor_with_client_auth(config: &MqttConfig) -> Result<TlsAcceptor, TlsError> {
    let cert_path = config.tls_cert_path.as_ref().ok_or(TlsError::CertPathNotConfigured)?;

    let key_path = config.tls_key_path.as_ref().ok_or(TlsError::KeyPathNotConfigured)?;

    // Verify files exist
    if !cert_path.exists() {
        return Err(TlsError::CertNotFound(cert_path.display().to_string()));
    }
    if !key_path.exists() {
        return Err(TlsError::KeyNotFound(key_path.display().to_string()));
    }

    // Load certificates and private key
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    // Build server config based on client auth setting
    let server_config = if config.tls_client_auth {
        // Load CA certificate for client verification
        let ca_path = config.tls_ca_path.as_ref().ok_or_else(|| {
            TlsError::ConfigError("Client auth requires CA certificate path".to_string())
        })?;

        if !ca_path.exists() {
            return Err(TlsError::CertNotFound(format!("CA certificate: {}", ca_path.display())));
        }

        let ca_certs = load_certs(ca_path)?;

        // Create root cert store with CA certs
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

        ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
    } else {
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| TlsError::ConfigError(e.to_string()))?
    };

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::CertNotFound("/path/to/cert.pem".to_string());
        assert!(err.to_string().contains("/path/to/cert.pem"));

        let err = TlsError::NoCertificates;
        assert!(err.to_string().contains("No certificates"));
    }

    #[test]
    fn test_create_tls_acceptor_missing_cert_path() {
        let config = MqttConfig {
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: Some(std::path::PathBuf::from("/tmp/key.pem")),
            ..Default::default()
        };

        let result = create_tls_acceptor(&config);
        assert!(matches!(result, Err(TlsError::CertPathNotConfigured)));
    }

    #[test]
    fn test_create_tls_acceptor_missing_key_path() {
        let config = MqttConfig {
            tls_enabled: true,
            tls_cert_path: Some(std::path::PathBuf::from("/tmp/cert.pem")),
            tls_key_path: None,
            ..Default::default()
        };

        let result = create_tls_acceptor(&config);
        assert!(matches!(result, Err(TlsError::KeyPathNotConfigured)));
    }

    #[test]
    fn test_create_tls_acceptor_cert_not_found() {
        let config = MqttConfig {
            tls_enabled: true,
            tls_cert_path: Some(std::path::PathBuf::from("/nonexistent/cert.pem")),
            tls_key_path: Some(std::path::PathBuf::from("/nonexistent/key.pem")),
            ..Default::default()
        };

        let result = create_tls_acceptor(&config);
        assert!(matches!(result, Err(TlsError::CertNotFound(_))));
    }
}

//! TLS/HTTPS support for HTTP server
//!
//! This module provides TLS configuration and certificate loading for secure HTTP connections.

use mockforge_core::config::HttpTlsConfig;
use mockforge_core::Result;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

/// Load TLS acceptor from certificate and key files
///
/// This function loads server certificates and private keys from PEM files
/// and creates a TLS acceptor for use with the HTTP server.
///
/// For mutual TLS (mTLS), provide a CA certificate file via `ca_file`.
pub fn load_tls_acceptor(config: &HttpTlsConfig) -> Result<TlsAcceptor> {
    use rustls_pemfile::{certs, pkcs8_private_keys};
    use std::fs::File;
    use std::io::BufReader;

    info!(
        "Loading TLS certificate from {} and key from {}",
        config.cert_file, config.key_file
    );

    // Load certificate chain
    let cert_file = File::open(&config.cert_file).map_err(|e| {
        mockforge_core::Error::generic(format!(
            "Failed to open certificate file {}: {}",
            config.cert_file, e
        ))
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_bytes: Vec<Vec<u8>> = certs(&mut cert_reader).map_err(|e| {
        mockforge_core::Error::generic(format!(
            "Failed to parse certificate file {}: {}",
            config.cert_file, e
        ))
    })?;
    let server_certs = cert_bytes.into_iter().map(rustls::Certificate).collect::<Vec<_>>();

    if server_certs.is_empty() {
        return Err(mockforge_core::Error::generic(format!(
            "No certificates found in {}",
            config.cert_file
        )));
    }

    // Load private key
    let key_file = File::open(&config.key_file).map_err(|e| {
        mockforge_core::Error::generic(format!(
            "Failed to open private key file {}: {}",
            config.key_file, e
        ))
    })?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys: Vec<Vec<u8>> = pkcs8_private_keys(&mut key_reader).map_err(|e| {
        mockforge_core::Error::generic(format!(
            "Failed to parse private key file {}: {}",
            config.key_file, e
        ))
    })?;

    if keys.is_empty() {
        return Err(mockforge_core::Error::generic(format!(
            "No private keys found in {}",
            config.key_file
        )));
    }

    // Build TLS server configuration with version support
    // Note: rustls uses safe defaults, so we configure during builder creation
    let server_config = if config.require_client_cert {
        // Mutual TLS: require client certificates
        if let Some(ref ca_file_path) = config.ca_file {
            // Load CA certificate for client verification
            let ca_file = File::open(ca_file_path).map_err(|e| {
                mockforge_core::Error::generic(format!(
                    "Failed to open CA certificate file {}: {}",
                    ca_file_path, e
                ))
            })?;
            let mut ca_reader = BufReader::new(ca_file);
            let ca_certs: Vec<Vec<u8>> = certs(&mut ca_reader).map_err(|e| {
                mockforge_core::Error::generic(format!(
                    "Failed to parse CA certificate file {}: {}",
                    ca_file_path, e
                ))
            })?;

            let ca_certs = ca_certs.into_iter().map(rustls::Certificate).collect::<Vec<_>>();

            let mut root_store = rustls::RootCertStore::empty();
            for cert in ca_certs {
                root_store.add(&cert).map_err(|e| {
                    mockforge_core::Error::generic(format!(
                        "Failed to add CA certificate to root store: {}",
                        e
                    ))
                })?;
            }

            // Build with mTLS support
            rustls::server::ServerConfig::builder()
                .with_safe_defaults()
                .with_client_cert_verifier(Arc::new(
                    rustls::server::AllowAnyAuthenticatedClient::new(root_store),
                ))
                .with_single_cert(server_certs, rustls::PrivateKey(keys.remove(0)))
                .map_err(|e| {
                    mockforge_core::Error::generic(format!("TLS config error (mTLS): {}", e))
                })?
        } else {
            return Err(mockforge_core::Error::generic(
                "Client certificate required (require_client_cert=true) but no CA file provided",
            ));
        }
    } else {
        // Standard TLS: no client certificate required
        rustls::server::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(server_certs, rustls::PrivateKey(keys.remove(0)))
            .map_err(|e| mockforge_core::Error::generic(format!("TLS config error: {}", e)))?
    };

    // Note: TLS version configuration is handled by with_safe_defaults()
    // which supports TLS 1.2 and 1.3. The min_version config option is
    // documented but rustls uses safe defaults that include both versions.
    if config.min_version == "1.3" {
        info!("TLS 1.3 requested (rustls safe defaults support both 1.2 and 1.3)");
    } else if config.min_version != "1.2" && !config.min_version.is_empty() {
        tracing::warn!(
            "Unsupported TLS version: {}, using rustls safe defaults (1.2+)",
            config.min_version
        );
    }

    // Configure cipher suites if specified
    if !config.cipher_suites.is_empty() {
        // Note: rustls uses safe defaults, so we don't override cipher suites
        // unless there's a specific need. The config is accepted but may not be used.
        info!("Custom cipher suites specified but rustls uses safe defaults");
    }

    info!("TLS acceptor configured successfully");
    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cert() -> (NamedTempFile, NamedTempFile) {
        // Create minimal test certificates (these won't actually work for real TLS,
        // but allow us to test the parsing logic)
        let cert = NamedTempFile::new().unwrap();
        let key = NamedTempFile::new().unwrap();

        // Write minimal PEM structure (not valid, but tests file reading)
        writeln!(cert.as_file(), "-----BEGIN CERTIFICATE-----").unwrap();
        writeln!(cert.as_file(), "TEST").unwrap();
        writeln!(cert.as_file(), "-----END CERTIFICATE-----").unwrap();

        writeln!(key.as_file(), "-----BEGIN PRIVATE KEY-----").unwrap();
        writeln!(key.as_file(), "TEST").unwrap();
        writeln!(key.as_file(), "-----END PRIVATE KEY-----").unwrap();

        (cert, key)
    }

    #[test]
    fn test_tls_config_validation() {
        let (cert, key) = create_test_cert();

        let config = HttpTlsConfig {
            enabled: true,
            cert_file: cert.path().to_string_lossy().to_string(),
            key_file: key.path().to_string_lossy().to_string(),
            ca_file: None,
            min_version: "1.2".to_string(),
            cipher_suites: Vec::new(),
            require_client_cert: false,
        };

        // This will fail because the certificates are not valid,
        // but it tests that the function attempts to load them
        let result = load_tls_acceptor(&config);
        assert!(result.is_err()); // Should fail on invalid cert
    }

    #[test]
    fn test_mtls_requires_ca() {
        let (cert, key) = create_test_cert();

        let config = HttpTlsConfig {
            enabled: true,
            cert_file: cert.path().to_string_lossy().to_string(),
            key_file: key.path().to_string_lossy().to_string(),
            ca_file: None,
            min_version: "1.2".to_string(),
            cipher_suites: Vec::new(),
            require_client_cert: true, // Requires client cert but no CA file
        };

        let result = load_tls_acceptor(&config);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("no CA file provided") || err_msg.contains("CA file"));
    }
}

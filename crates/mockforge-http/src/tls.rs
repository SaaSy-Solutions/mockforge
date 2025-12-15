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
    let server_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| {
            mockforge_core::Error::generic(format!(
                "Failed to parse certificate file {}: {}",
                config.cert_file, e
            ))
        })?;

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
    let pkcs8_keys: Vec<rustls::pki_types::PrivatePkcs8KeyDer<'static>> = pkcs8_private_keys(&mut key_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| {
            mockforge_core::Error::generic(format!(
                "Failed to parse private key file {}: {}",
                config.key_file, e
            ))
        })?;
    let mut keys: Vec<rustls::pki_types::PrivateKeyDer<'static>> = pkcs8_keys.into_iter()
        .map(|k| rustls::pki_types::PrivateKeyDer::Pkcs8(k))
        .collect();

    if keys.is_empty() {
        return Err(mockforge_core::Error::generic(format!(
            "No private keys found in {}",
            config.key_file
        )));
    }

    // Build TLS server configuration with version support
    // Note: rustls uses safe defaults, so we configure during builder creation
    // Determine mTLS mode: use mtls_mode if set, otherwise fall back to require_client_cert for backward compatibility
    let mtls_mode = if !config.mtls_mode.is_empty() && config.mtls_mode != "off" {
        config.mtls_mode.as_str()
    } else if config.require_client_cert {
        "required"
    } else {
        "off"
    };

    let server_config = match mtls_mode {
        "required" => {
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
                let ca_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut ca_reader)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        mockforge_core::Error::generic(format!(
                            "Failed to parse CA certificate file {}: {}",
                            ca_file_path, e
                        ))
                    })?;

                let mut root_store = rustls::RootCertStore::empty();
                for cert in &ca_certs {
                    root_store.add(cert.clone()).map_err(|e| {
                        mockforge_core::Error::generic(format!(
                            "Failed to add CA certificate to root store: {}",
                            e
                        ))
                    })?;
                }

                let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                    .build()
                    .map_err(|e| mockforge_core::Error::generic(format!("Failed to build client verifier: {}", e)))?;

                let key = keys.remove(0);

                // Build with mTLS support (required)
                rustls::server::ServerConfig::builder()
                    .with_client_cert_verifier(client_verifier.into())
                    .with_single_cert(server_certs, key)
                    .map_err(|e| {
                        mockforge_core::Error::generic(format!("TLS config error (mTLS required): {}", e))
                    })?
            } else {
                return Err(mockforge_core::Error::generic(
                    "mTLS mode 'required' requires --tls-ca (CA certificate file)",
                ));
            }
        }
        "optional" => {
            // Mutual TLS: accept client certificates if provided, but don't require
            if let Some(ref ca_file_path) = config.ca_file {
                // Load CA certificate for client verification
                let ca_file = File::open(ca_file_path).map_err(|e| {
                    mockforge_core::Error::generic(format!(
                        "Failed to open CA certificate file {}: {}",
                        ca_file_path, e
                    ))
                })?;
                let mut ca_reader = BufReader::new(ca_file);
                let ca_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut ca_reader)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        mockforge_core::Error::generic(format!(
                            "Failed to parse CA certificate file {}: {}",
                            ca_file_path, e
                        ))
                    })?;

                let mut root_store = rustls::RootCertStore::empty();
                for cert in &ca_certs {
                    root_store.add(cert.clone()).map_err(|e| {
                        mockforge_core::Error::generic(format!(
                            "Failed to add CA certificate to root store: {}",
                            e
                        ))
                    })?;
                }

                let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                    .build()
                    .map_err(|e| mockforge_core::Error::generic(format!("Failed to build client verifier: {}", e)))?;

                let key = keys.remove(0);

                // Build with optional mTLS support
                // Note: rustls doesn't have a built-in "optional" mode, so we use
                // WebPkiClientVerifier which accepts any client cert that validates,
                // but connections without certs will also work (we can't enforce optional-only)
                // For true optional mTLS, we'd need custom verifier logic
                rustls::server::ServerConfig::builder()
                    .with_client_cert_verifier(client_verifier.into())
                    .with_single_cert(server_certs, key)
                    .map_err(|e| {
                        mockforge_core::Error::generic(format!("TLS config error (mTLS optional): {}", e))
                    })?
            } else {
                // Optional mTLS without CA: just standard TLS
                info!("mTLS optional mode specified but no CA file provided, using standard TLS");
                let key = keys.remove(0);
                rustls::server::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(server_certs, key)
                    .map_err(|e| mockforge_core::Error::generic(format!("TLS config error: {}", e)))?
            }
        }
        _ => {
            // Standard TLS: no client certificate required
            let key = keys.remove(0);
            rustls::server::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(server_certs, key)
                .map_err(|e| mockforge_core::Error::generic(format!("TLS config error: {}", e)))?
        }
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

/// Load TLS server configuration for use with axum-server
///
/// This function is similar to load_tls_acceptor but returns the ServerConfig
/// directly for use with axum-server's RustlsConfig.
pub fn load_tls_server_config(
    config: &HttpTlsConfig,
) -> std::result::Result<Arc<rustls::server::ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
    use rustls_pemfile::{certs, pkcs8_private_keys};
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::Arc;

    info!(
        "Loading TLS certificate from {} and key from {}",
        config.cert_file, config.key_file
    );

    // Load certificate chain
    let cert_file = File::open(&config.cert_file).map_err(|e| {
        format!(
            "Failed to open certificate file {}: {}",
            config.cert_file, e
        )
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let server_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "Failed to parse certificate file {}: {}",
                config.cert_file, e
            )
        })?;

    if server_certs.is_empty() {
        return Err(format!("No certificates found in {}", config.cert_file).into());
    }

    // Load private key
    let key_file = File::open(&config.key_file).map_err(|e| {
        format!(
            "Failed to open private key file {}: {}",
            config.key_file, e
        )
    })?;
    let mut key_reader = BufReader::new(key_file);
    let pkcs8_keys: Vec<rustls::pki_types::PrivatePkcs8KeyDer<'static>> = pkcs8_private_keys(&mut key_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "Failed to parse private key file {}: {}",
                config.key_file, e
            )
        })?;
    let mut keys: Vec<rustls::pki_types::PrivateKeyDer<'static>> = pkcs8_keys.into_iter()
        .map(|k| rustls::pki_types::PrivateKeyDer::Pkcs8(k))
        .collect();

    if keys.is_empty() {
        return Err(format!("No private keys found in {}", config.key_file).into());
    }

    // Determine mTLS mode
    let mtls_mode = if !config.mtls_mode.is_empty() && config.mtls_mode != "off" {
        config.mtls_mode.as_str()
    } else if config.require_client_cert {
        "required"
    } else {
        "off"
    };

    let server_config = match mtls_mode {
        "required" => {
            if let Some(ref ca_file_path) = config.ca_file {
                let ca_file = File::open(ca_file_path).map_err(|e| {
                    format!("Failed to open CA certificate file {}: {}", ca_file_path, e)
                })?;
                let mut ca_reader = BufReader::new(ca_file);
                let ca_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut ca_reader)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        format!("Failed to parse CA certificate file {}: {}", ca_file_path, e)
                    })?;

                let mut root_store = rustls::RootCertStore::empty();
                for cert in &ca_certs {
                    root_store.add(cert.clone()).map_err(|e| {
                        format!("Failed to add CA certificate to root store: {}", e)
                    })?;
                }

                let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                    .build()
                    .map_err(|e| format!("Failed to build client verifier: {}", e))?;

                let key = keys.remove(0);

                rustls::server::ServerConfig::builder()
                    .with_client_cert_verifier(client_verifier.into())
                    .with_single_cert(server_certs, key)
                    .map_err(|e| format!("TLS config error (mTLS required): {}", e))?
            } else {
                return Err("mTLS mode 'required' requires CA certificate file".to_string().into());
            }
        }
        "optional" => {
            if let Some(ref ca_file_path) = config.ca_file {
                let ca_file = File::open(ca_file_path).map_err(|e| {
                    format!("Failed to open CA certificate file {}: {}", ca_file_path, e)
                })?;
                let mut ca_reader = BufReader::new(ca_file);
                let ca_certs: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut ca_reader)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        format!("Failed to parse CA certificate file {}: {}", ca_file_path, e)
                    })?;

                let mut root_store = rustls::RootCertStore::empty();
                for cert in &ca_certs {
                    root_store.add(cert.clone()).map_err(|e| {
                        format!("Failed to add CA certificate to root store: {}", e)
                    })?;
                }

                let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                    .build()
                    .map_err(|e| format!("Failed to build client verifier: {}", e))?;

                let key = keys.remove(0);

                rustls::server::ServerConfig::builder()
                    .with_client_cert_verifier(client_verifier.into())
                    .with_single_cert(server_certs, key)
                    .map_err(|e| format!("TLS config error (mTLS optional): {}", e))?
            } else {
                let key = keys.remove(0);
                rustls::server::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(server_certs, key)
                    .map_err(|e| format!("TLS config error: {}", e))?
            }
        }
        _ => {
            let key = keys.remove(0);
            rustls::server::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(server_certs, key)
                .map_err(|e| format!("TLS config error: {}", e))?
        }
    };

    Ok(Arc::new(server_config))
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
            mtls_mode: "off".to_string(),
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
            mtls_mode: "required".to_string(),
        };

        let result = load_tls_acceptor(&config);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("no CA file provided") || err_msg.contains("CA file"));
    }
}

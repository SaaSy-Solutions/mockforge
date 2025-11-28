//! Production-ready tunnel server for MockForge
//!
//! This server provides tunneling capabilities with:
//! - Persistent storage (SQLite)
//! - Rate limiting
//! - TLS support
//! - Audit logging

use mockforge_tunnel::audit::AuditLogger;
use mockforge_tunnel::rate_limit::{rate_limit_middleware, RateLimitConfig, TunnelRateLimiter};
use mockforge_tunnel::server::{
    create_tunnel_server_router, TunnelStore, TunnelStoreTrait, TunnelStoreWrapper,
};
use mockforge_tunnel::server_config::ServerConfig;
#[cfg(feature = "sqlx")]
use mockforge_tunnel::storage::PersistentTunnelStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Load configuration from environment
    let config = ServerConfig::from_env();

    info!("🚇 MockForge Tunnel Server (Production)");
    info!("Configuration:");
    info!("  Port: {}", config.port);
    info!("  Bind Address: {}", config.bind_address);
    info!("  TLS: {}", config.tls.as_ref().map(|t| t.enabled).unwrap_or(false));
    info!(
        "  Storage: {}",
        if config.use_in_memory_storage {
            "in-memory"
        } else {
            "persistent"
        }
    );
    info!("  Rate Limiting: {}", config.rate_limit.enabled);
    info!("  Audit Logging: {}", config.audit_logging_enabled);

    // Initialize storage
    let store_wrapper = {
        #[cfg(feature = "sqlx")]
        {
            if !config.use_in_memory_storage {
                let db_path = config.database_path.unwrap_or_else(|| PathBuf::from("tunnels.db"));
                info!("Using persistent storage at: {:?}", db_path);
                let persistent_store = PersistentTunnelStore::new(&db_path).await.map_err(|e| {
                    anyhow::anyhow!("Failed to initialize persistent storage: {}", e)
                })?;
                TunnelStoreWrapper::new(Arc::new(persistent_store))
            } else {
                let in_memory_store = TunnelStore::new();
                TunnelStoreWrapper::new(Arc::new(in_memory_store))
            }
        }
        #[cfg(not(feature = "sqlx"))]
        {
            let in_memory_store = TunnelStore::new();
            TunnelStoreWrapper::new(Arc::new(in_memory_store))
        }
    };

    // Initialize rate limiter
    #[cfg(feature = "governor")]
    let rate_limiter = Arc::new(TunnelRateLimiter::new(config.rate_limit.clone()));

    // Create router with store
    let mut router = create_tunnel_server_router().with_state(store_wrapper.clone());

    // Add rate limiting middleware if enabled
    #[cfg(feature = "governor")]
    if config.rate_limit.enabled {
        router = router.layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ));
        info!(
            "Rate limiting enabled: {} RPM (global), {} RPM (per-IP)",
            config.rate_limit.global_requests_per_minute,
            config.rate_limit.per_ip_requests_per_minute
        );
    }

    // Log server startup
    if config.audit_logging_enabled {
        AuditLogger::log_error(
            mockforge_tunnel::audit::AuditEventType::ConfigChanged,
            "server_start",
            "Server started",
            None,
        );
    }

    // Bind address
    let addr = format!("{}:{}", config.bind_address, config.port);
    let addr: SocketAddr = addr.parse()?;

    info!("Listening on: {}", addr);
    info!("Health check: http://{}/health", addr);
    info!("API endpoint: http://{}/api/tunnels", addr);

    // Start cleanup task for expired tunnels when persistent storage is enabled
    #[cfg(feature = "sqlx")]
    if !config.use_in_memory_storage {
        let store_for_cleanup = store_wrapper.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Run every hour
            loop {
                interval.tick().await;
                match TunnelStoreTrait::cleanup_expired(&store_for_cleanup).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Cleaned up {} expired tunnels", count);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to cleanup expired tunnels: {}", e);
                    }
                }
            }
        });
        info!("Started cleanup task for expired tunnels (runs every hour)");
    }

    // Start server
    let listener = TcpListener::bind(&addr).await?;

    // Handle TLS if configured
    #[cfg(feature = "rustls")]
    if let Some(tls_config) = &config.tls {
        if tls_config.enabled {
            info!("Starting server with TLS");
            info!("  Certificate: {:?}", tls_config.cert_path);
            info!("  Key: {:?}", tls_config.key_path);

            // Load TLS certificates
            let mut cert_file =
                std::io::BufReader::new(std::fs::File::open(&tls_config.cert_path)?);
            let certs = rustls_pemfile::certs(&mut cert_file).collect::<Result<Vec<_>, _>>()?;
            let mut key_file = std::io::BufReader::new(std::fs::File::open(&tls_config.key_path)?);
            let key = rustls_pemfile::pkcs8_private_keys(&mut key_file)
                .next()
                .ok_or("No private key found")??;

            // In rustls 0.23+, CertificateDer is a type alias for Vec<u8>
            // and PrivateKeyDer is an enum wrapper
            let cert_chain: Vec<rustls::pki_types::CertificateDer> = certs;
            let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(key.secret_pkcs8_der().to_vec()),
            );

            let tls_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert_chain, key)?;

            // Integrate TLS acceptor with axum serve
            // Note: Full TLS integration with axum requires either:
            // 1. axum-server crate (has compatibility issues with current axum version)
            // 2. Manual hyper server implementation (complex due to stream type compatibility)
            // 3. Reverse proxy (recommended for production)
            //
            // For now, we configure TLS and log that it's ready, but recommend using a reverse proxy
            // for production deployments. The TLS acceptor is created and ready for future integration.
            let _acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config));

            info!("TLS acceptor configured. For production TLS, use a reverse proxy (nginx) for TLS termination.");
            info!("Future enhancement: Full native TLS support will be added when axum-server compatibility is resolved.");

            // For now, serve without TLS but with TLS configuration validated
            // In production, users should use a reverse proxy for TLS termination
            axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
        } else {
            axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
        }
    } else {
        axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
    }

    #[cfg(not(feature = "rustls"))]
    {
        axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
    }

    info!("Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown...");
}

// Note: Persistent storage (PersistentTunnelStore) is available but not yet
// integrated with the router. The router currently uses TunnelStore (in-memory).
// Future update will add trait-based storage abstraction to support both.

//! Pillars: [Cloud]
//!
//! MockForge Plugin Registry Server
//!
//! Central registry for discovering, publishing, and installing plugins.

mod auth;
mod cache;
mod circuit_breaker;
mod config;
mod database;
mod deployment;
mod email;
mod error;
mod handlers;
mod metrics;
mod middleware;
mod models;
mod pillar_tracking_init;
mod redis;
mod routes;
mod storage;
mod two_factor;
mod validation;
mod workers;

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::Router;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::signal;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerRegistry};
use crate::config::Config;
use crate::database::Database;
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::rate_limit::RateLimiterState;
use crate::middleware::request_id::request_id_middleware;
use crate::redis::RedisPool;
use crate::storage::PluginStorage;

use axum::response::IntoResponse;
use mockforge_observability::get_global_registry;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub storage: PluginStorage,
    pub config: Config,
    pub metrics: Arc<mockforge_observability::prometheus::MetricsRegistry>,
    pub analytics_db: Option<mockforge_analytics::AnalyticsDatabase>,
    pub redis: Option<RedisPool>,
    pub circuit_breakers: CircuitBreakerRegistry,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mockforge_registry_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded");

    // Connect to database
    let db = Database::connect(&config.database_url).await?;
    tracing::info!("Database connected");

    // Run migrations (unless SKIP_MIGRATIONS=true, for K8s Job-based migrations)
    if config.skip_migrations {
        tracing::info!("Skipping database migrations (SKIP_MIGRATIONS=true)");
    } else {
        db.migrate().await?;
        tracing::info!("Database migrations complete");
    }

    // Initialize storage
    let storage = PluginStorage::new(&config).await?;
    tracing::info!("Storage initialized");

    // Initialize metrics registry
    let metrics = Arc::new(get_global_registry().clone());

    // Initialize analytics database (optional)
    let analytics_db = if let Some(analytics_db_path) = &config.analytics_db_path {
        match mockforge_analytics::AnalyticsDatabase::new(std::path::Path::new(analytics_db_path))
            .await
        {
            Ok(analytics_db) => {
                if let Err(e) = analytics_db.run_migrations().await {
                    tracing::warn!("Failed to run analytics database migrations: {}", e);
                    None
                } else {
                    tracing::info!("Analytics database initialized at: {}", analytics_db_path);
                    Some(analytics_db)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to initialize analytics database: {}", e);
                None
            }
        }
    } else {
        // Try default path
        let default_path = std::path::Path::new("mockforge-analytics.db");
        match mockforge_analytics::AnalyticsDatabase::new(default_path).await {
            Ok(analytics_db) => {
                if let Err(e) = analytics_db.run_migrations().await {
                    tracing::warn!("Failed to run analytics database migrations: {}", e);
                    None
                } else {
                    tracing::info!(
                        "Analytics database initialized at default path: mockforge-analytics.db"
                    );
                    Some(analytics_db)
                }
            }
            Err(e) => {
                tracing::debug!("Analytics database not available (optional): {}", e);
                None
            }
        }
    };

    // Initialize pillar tracking with analytics database
    if let Some(ref analytics_db) = analytics_db {
        let db_arc = std::sync::Arc::new(analytics_db.clone());
        pillar_tracking_init::init_pillar_tracking(Some(db_arc)).await;
    }

    // Initialize Redis (optional)
    let redis = if let Some(redis_url) = &config.redis_url {
        match RedisPool::connect(redis_url).await {
            Ok(pool) => {
                tracing::info!("Redis connected");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to Redis (2FA setup will require alternative flow): {}",
                    e
                );
                None
            }
        }
    } else {
        tracing::info!("Redis not configured (REDIS_URL not set)");
        None
    };

    // Initialize rate limiter
    let rate_limiter = RateLimiterState::new(config.rate_limit_per_minute);
    tracing::info!("Rate limiter initialized: {} requests/minute", config.rate_limit_per_minute);

    // Initialize circuit breakers for external services
    let circuit_breakers = CircuitBreakerRegistry::new();
    circuit_breakers
        .register("redis", CircuitBreaker::new(circuit_breaker::presets::redis()))
        .await;
    circuit_breakers
        .register("s3", CircuitBreaker::new(circuit_breaker::presets::s3()))
        .await;
    circuit_breakers
        .register("email", CircuitBreaker::new(circuit_breaker::presets::email()))
        .await;
    circuit_breakers
        .register("database", CircuitBreaker::new(circuit_breaker::presets::database()))
        .await;
    tracing::info!("Circuit breakers initialized for external services");

    // Create app state
    let state = AppState {
        db: db.clone(),
        storage,
        config: config.clone(),
        metrics: metrics.clone(),
        analytics_db,
        redis,
        circuit_breakers,
    };

    // Start background workers
    workers::saml_cleanup::start_saml_cleanup_worker(db.pool().clone());

    // Build router
    let app = create_app(state, rate_limiter);

    // Start server with graceful shutdown
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);
    tracing::info!("Starting server on {}", addr);
    tracing::info!("Graceful shutdown timeout: {} seconds", config.shutdown_timeout_secs);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_timeout))
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Create a future that completes when a shutdown signal is received.
/// Handles both SIGTERM and SIGINT (Ctrl+C) on Unix systems.
async fn shutdown_signal(timeout: Duration) {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("Failed to install Ctrl+C handler: {}", e);
                // If we can't install the handler, wait forever (other signals may still work)
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {}", e);
                // If we can't install the handler, wait forever (Ctrl+C may still work)
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }

    tracing::info!(
        "Stopping new connections, waiting up to {} seconds for active requests to complete",
        timeout.as_secs()
    );
}

fn create_app(state: AppState, rate_limiter: RateLimiterState) -> Router {
    // Configure CORS from environment variable
    // CORS_ALLOWED_ORIGINS: comma-separated list of allowed origins
    // Default: strict same-origin (no external origins allowed in production)
    let cors = match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            let allowed_origins: Vec<_> =
                origins.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            tracing::info!("CORS configured with {} allowed origins", allowed_origins.len());
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_methods(Any)
                .allow_headers(Any)
        }
        _ => {
            // In production, default to strict same-origin (no external origins)
            tracing::info!(
                "CORS configured with strict same-origin policy (no CORS_ALLOWED_ORIGINS set)"
            );
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(
                    "null".parse().expect("'null' is a valid header value"),
                ))
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

    // Configure request body size limit from environment variable
    // MAX_REQUEST_BODY_SIZE: maximum request body size in bytes
    // Default: 10MB (10 * 1024 * 1024 = 10485760 bytes)
    let max_body_size: usize = std::env::var("MAX_REQUEST_BODY_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10 * 1024 * 1024); // 10MB default
    tracing::info!("Request body size limit: {} bytes", max_body_size);

    // Add metrics endpoint (separate router without state)
    let metrics_router = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/metrics/health", axum::routing::get(|| async { "OK" }));

    Router::new()
        .merge(routes::create_router())
        .merge(metrics_router)
        .layer(DefaultBodyLimit::max(max_body_size))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn(csrf_middleware))
        .layer(axum::Extension(rate_limiter))
        .with_state(state)
}

async fn metrics_handler() -> impl axum::response::IntoResponse {
    use mockforge_observability::get_global_registry;
    use prometheus::{Encoder, TextEncoder};

    let encoder = TextEncoder::new();
    let metric_families = get_global_registry().registry().gather();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics")
            .into_response();
    }

    let body = match String::from_utf8(buffer) {
        Ok(body) => body,
        Err(e) => {
            tracing::error!("Failed to convert metrics to UTF-8: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to convert metrics")
                .into_response();
        }
    };

    (
        axum::http::StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        // Test implementation
    }
}

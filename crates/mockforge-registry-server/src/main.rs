//! Pillars: [Cloud]
//!
//! MockForge Plugin Registry Server
//!
//! Central registry for discovering, publishing, and installing plugins.

mod auth;
mod cache;
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
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::database::Database;
use crate::storage::PluginStorage;

use mockforge_observability::get_global_registry;
use std::sync::Arc;
use axum::response::IntoResponse;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub storage: PluginStorage,
    pub config: Config,
    pub metrics: Arc<mockforge_observability::prometheus::MetricsRegistry>,
    pub analytics_db: Option<mockforge_analytics::AnalyticsDatabase>,
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

    // Run migrations
    db.migrate().await?;
    tracing::info!("Database migrations complete");

    // Initialize storage
    let storage = PluginStorage::new(&config).await?;
    tracing::info!("Storage initialized");

    // Initialize metrics registry
    let metrics = Arc::new(get_global_registry().clone());

    // Initialize analytics database (optional)
    let analytics_db = if let Some(analytics_db_path) = &config.analytics_db_path {
        match mockforge_analytics::AnalyticsDatabase::new(
            std::path::Path::new(analytics_db_path)
        ).await {
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
                    tracing::info!("Analytics database initialized at default path: mockforge-analytics.db");
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

    // Create app state
    let state = AppState {
        db: db.clone(),
        storage,
        config: config.clone(),
        metrics: metrics.clone(),
        analytics_db,
    };

    // Start background workers
    workers::saml_cleanup::start_saml_cleanup_worker(db.pool().clone());

    // Build router
    let app = create_app(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_app(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // Add metrics endpoint (separate router without state)
    let metrics_router = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/metrics/health", axum::routing::get(|| async { "OK" }));

    Router::new()
        .merge(routes::create_router())
        .merge(metrics_router)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
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
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response();
    }

    let body = match String::from_utf8(buffer) {
        Ok(body) => body,
        Err(e) => {
            tracing::error!("Failed to convert metrics to UTF-8: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to convert metrics").into_response();
        }
    };

    (
        axum::http::StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    ).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        // Test implementation
    }
}

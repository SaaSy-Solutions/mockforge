//! MockForge Plugin Registry Server
//!
//! Central registry for discovering, publishing, and installing plugins.

mod auth;
mod config;
mod database;
mod error;
mod handlers;
mod middleware;
mod models;
mod routes;
mod storage;

use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::database::Database;
use crate::storage::PluginStorage;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub storage: PluginStorage,
    pub config: Config,
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

    // Create app state
    let state = AppState {
        db,
        storage,
        config: config.clone(),
    };

    // Build router
    let app = create_app(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    axum::Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

fn create_app(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    Router::new()
        .merge(routes::create_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        // Test implementation
    }
}

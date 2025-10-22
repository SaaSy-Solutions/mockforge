//! Collaboration server implementation

use crate::api::{create_router as create_api_router, ApiState};
use crate::auth::AuthService;
use crate::config::CollabConfig;
use crate::error::Result;
use crate::events::EventBus;
use crate::history::History;
use crate::sync::SyncEngine;
use crate::websocket::{ws_handler, WsState};
use crate::workspace::WorkspaceService;
use axum::routing::get;
use axum::Router;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

/// Collaboration server
pub struct CollabServer {
    /// Configuration
    config: CollabConfig,
    /// Database pool
    db: Pool<Sqlite>,
    /// Authentication service
    auth: Arc<AuthService>,
    /// Workspace service
    workspace: Arc<WorkspaceService>,
    /// Event bus
    event_bus: Arc<EventBus>,
    /// Sync engine
    sync: Arc<SyncEngine>,
    /// History tracker
    history: Arc<History>,
}

impl CollabServer {
    /// Create a new collaboration server
    pub async fn new(config: CollabConfig) -> Result<Self> {
        // Initialize database
        let db = sqlx::SqlitePool::connect(&config.database_url).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&db).await?;

        // Create services
        let auth = Arc::new(AuthService::new(config.jwt_secret.clone()));
        let workspace = Arc::new(WorkspaceService::new(db.clone()));
        let event_bus = Arc::new(EventBus::new(config.event_bus_capacity));
        let sync = Arc::new(SyncEngine::new(event_bus.clone()));
        let mut history = History::new(db.clone());
        history.set_auto_commit(config.auto_commit);
        let history = Arc::new(history);

        Ok(Self {
            config,
            db,
            auth,
            workspace,
            event_bus,
            sync,
            history,
        })
    }

    /// Start the collaboration server
    pub async fn run(self, addr: &str) -> Result<()> {
        tracing::info!("Starting MockForge Collaboration Server on {}", addr);

        // Create API router
        let api_state = ApiState {
            auth: self.auth.clone(),
            workspace: self.workspace.clone(),
        };
        let api_router = create_api_router(api_state);

        // Create WebSocket state
        let ws_state = WsState {
            auth: self.auth.clone(),
            sync: self.sync.clone(),
            event_bus: self.event_bus.clone(),
        };

        // Combine routers
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(ws_state)
            .merge(api_router);

        // Parse address
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::CollabError::Internal(format!("Failed to bind: {}", e)))?;

        tracing::info!("Server listening on {}", addr);

        // Run server
        axum::serve(listener, app)
            .await
            .map_err(|e| crate::error::CollabError::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Get authentication service
    pub fn auth(&self) -> Arc<AuthService> {
        self.auth.clone()
    }

    /// Get workspace service
    pub fn workspace(&self) -> Arc<WorkspaceService> {
        self.workspace.clone()
    }

    /// Get sync engine
    pub fn sync(&self) -> Arc<SyncEngine> {
        self.sync.clone()
    }

    /// Get history tracker
    pub fn history(&self) -> Arc<History> {
        self.history.clone()
    }
}

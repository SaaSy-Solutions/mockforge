//! Collaboration server implementation

use crate::api::{create_router as create_api_router, ApiState};
use crate::auth::AuthService;
use crate::backup::BackupService;
use crate::config::CollabConfig;
use crate::core_bridge::CoreBridge;
use crate::error::Result;
use crate::events::EventBus;
use crate::history::{History, VersionControl};
use crate::merge::MergeService;
use crate::sync::SyncEngine;
use crate::user::UserService;
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
    /// User service
    user: Arc<UserService>,
    /// Workspace service
    workspace: Arc<WorkspaceService>,
    /// Event bus
    event_bus: Arc<EventBus>,
    /// Sync engine
    sync: Arc<SyncEngine>,
    /// History tracker
    history: Arc<History>,
    /// Merge service
    merge: Arc<MergeService>,
    /// Backup service
    backup: Arc<BackupService>,
}

impl CollabServer {
    /// Create a new collaboration server
    pub async fn new(config: CollabConfig) -> Result<Self> {
        // Initialize database
        let db = sqlx::SqlitePool::connect(&config.database_url).await?;

        // Run migrations automatically
        Self::run_migrations(&db).await?;

        // Create CoreBridge for workspace integration
        let workspace_dir = config.workspace_dir.as_deref().unwrap_or("./workspaces");
        let core_bridge = Arc::new(CoreBridge::new(workspace_dir));

        // Create services
        let auth = Arc::new(AuthService::new(config.jwt_secret.clone()));
        let user = Arc::new(UserService::new(db.clone(), auth.clone()));
        let workspace =
            Arc::new(WorkspaceService::with_core_bridge(db.clone(), core_bridge.clone()));
        let event_bus = Arc::new(EventBus::new(config.event_bus_capacity));
        let sync = Arc::new(SyncEngine::with_integration(
            event_bus.clone(),
            db.clone(),
            core_bridge.clone(),
            workspace.clone(),
        ));
        let mut history = History::new(db.clone());
        history.set_auto_commit(config.auto_commit);
        let history = Arc::new(history);

        // Create merge service
        let merge = Arc::new(MergeService::new(db.clone()));

        // Create backup service
        let backup = Arc::new(BackupService::new(
            db.clone(),
            config.backup_dir.clone(),
            core_bridge,
            workspace.clone(),
        ));

        Ok(Self {
            config,
            db,
            auth,
            user,
            workspace,
            event_bus,
            sync,
            history,
            merge,
            backup,
        })
    }

    /// Run database migrations
    ///
    /// This method can be called independently to ensure migrations are up to date.
    /// It's automatically called during server initialization.
    pub async fn run_migrations(db: &sqlx::SqlitePool) -> Result<()> {
        tracing::info!("Running database migrations");
        sqlx::migrate!("./migrations").run(db).await.map_err(|e| {
            tracing::error!("Migration failed: {}", e);
            crate::error::CollabError::DatabaseError(format!("Migration failed: {e}"))
        })?;
        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Start the collaboration server
    pub async fn run(self, addr: &str) -> Result<()> {
        tracing::info!("Starting MockForge Collaboration Server on {}", addr);

        // Create API router
        let version_control = Arc::new(VersionControl::new(self.db.clone()));

        // Get merge and backup services from config or create them
        // For now, we'll need to store them in the server struct
        // Let me check what we have available...

        // Actually, we need to restructure this - let me add merge and backup to the server struct
        let api_state = ApiState {
            auth: self.auth.clone(),
            user: self.user.clone(),
            workspace: self.workspace.clone(),
            history: version_control,
            merge: self.merge.clone(),
            backup: self.backup.clone(),
            sync: self.sync.clone(),
        };
        let api_router = create_api_router(api_state);

        // Create WebSocket state
        let ws_state = WsState {
            auth: self.auth.clone(),
            sync: self.sync.clone(),
            event_bus: self.event_bus.clone(),
            workspace: self.workspace.clone(),
        };

        // Combine routers
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(ws_state)
            .merge(api_router);

        // Parse address
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::CollabError::Internal(format!("Failed to bind: {e}")))?;

        tracing::info!("Server listening on {}", addr);

        // Run server
        axum::serve(listener, app)
            .await
            .map_err(|e| crate::error::CollabError::Internal(format!("Server error: {e}")))?;

        Ok(())
    }

    /// Get authentication service
    #[must_use]
    pub fn auth(&self) -> Arc<AuthService> {
        self.auth.clone()
    }

    /// Get workspace service
    #[must_use]
    pub fn workspace(&self) -> Arc<WorkspaceService> {
        self.workspace.clone()
    }

    /// Get sync engine
    #[must_use]
    pub fn sync(&self) -> Arc<SyncEngine> {
        self.sync.clone()
    }

    /// Get history tracker
    #[must_use]
    pub fn history(&self) -> Arc<History> {
        self.history.clone()
    }
}

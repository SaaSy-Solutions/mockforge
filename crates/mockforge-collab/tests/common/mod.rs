//! Common test utilities and helpers

use axum::Router;
use mockforge_collab::{
    api::{create_router, ApiState},
    auth::AuthService,
    backup::BackupService,
    config::CollabConfig,
    core_bridge::CoreBridge,
    events::EventBus,
    history::VersionControl,
    merge::MergeService,
    models::{User, UserRole},
    sync::SyncEngine,
    user::UserService,
    workspace::WorkspaceService,
};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

/// Test context holding database and services
pub struct TestContext {
    pub db: Pool<Sqlite>,
    pub auth: Arc<AuthService>,
    pub user: Arc<UserService>,
    pub workspace: Arc<WorkspaceService>,
    pub history: Arc<VersionControl>,
    pub router: Router,
    pub _temp_dir: TempDir,
}

impl TestContext {
    /// Create a new test context with in-memory database
    pub async fn new() -> Self {
        // Create temporary directory for test database
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let database_url = format!("sqlite:{}", db_path.display());

        // Create database pool
        let db = SqlitePool::connect(&database_url)
            .await
            .expect("Failed to create database pool");

        // Run migrations
        sqlx::migrate!("./migrations").run(&db).await.expect("Failed to run migrations");

        // Create temporary directories for workspace and backup storage
        let workspace_dir = temp_dir.path().join("workspaces");
        let backup_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace dir");
        std::fs::create_dir_all(&backup_dir).expect("Failed to create backup dir");

        // Create CoreBridge
        let core_bridge = Arc::new(CoreBridge::new(&workspace_dir));

        // Create services
        let auth = Arc::new(AuthService::new("test-secret-key".to_string()));
        let user = Arc::new(UserService::new(db.clone(), auth.clone()));
        let workspace =
            Arc::new(WorkspaceService::with_core_bridge(db.clone(), core_bridge.clone()));
        let history = Arc::new(VersionControl::new(db.clone()));

        // Create event bus and sync engine
        let event_bus = Arc::new(EventBus::new(1000));
        let sync = Arc::new(SyncEngine::with_integration(
            event_bus.clone(),
            db.clone(),
            core_bridge.clone(),
            workspace.clone(),
        ));

        // Create merge service
        let merge = Arc::new(MergeService::new(db.clone()));

        // Create backup service
        let backup = Arc::new(BackupService::new(
            db.clone(),
            Some(backup_dir.to_string_lossy().to_string()),
            core_bridge.clone(),
            workspace.clone(),
        ));

        // Create API router
        let api_state = ApiState {
            auth: auth.clone(),
            user: user.clone(),
            workspace: workspace.clone(),
            history: history.clone(),
            merge: merge.clone(),
            backup: backup.clone(),
            sync: sync.clone(),
        };
        let router = create_router(api_state);

        Self {
            db,
            auth,
            user,
            workspace,
            history,
            router,
            _temp_dir: temp_dir,
        }
    }

    /// Create a test user and return (user, token)
    pub async fn create_test_user(&self, username: &str, email: &str) -> (User, String) {
        let user = self
            .user
            .create_user(username.to_string(), email.to_string(), "password123".to_string())
            .await
            .expect("Failed to create test user");

        let token = self.auth.generate_token(&user).expect("Failed to generate token");

        (user, token.access_token)
    }

    /// Create a test workspace owned by the given user
    pub async fn create_test_workspace(&self, owner_id: Uuid, name: &str) -> Uuid {
        let workspace = self
            .workspace
            .create_workspace(name.to_string(), Some("Test workspace".to_string()), owner_id)
            .await
            .expect("Failed to create workspace");

        workspace.id
    }

    /// Add a user to a workspace with a specific role
    pub async fn add_workspace_member(
        &self,
        workspace_id: Uuid,
        owner_id: Uuid,
        member_id: Uuid,
        role: UserRole,
    ) {
        self.workspace
            .add_member(workspace_id, owner_id, member_id, role)
            .await
            .expect("Failed to add member");
    }
}

/// Helper to create authorization header
pub fn auth_header(token: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {}", token))
}

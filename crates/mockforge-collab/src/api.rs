//! REST API endpoints for collaboration

use crate::auth::{AuthService, Credentials};
use crate::backup::{BackupService, StorageBackend};
use crate::error::{CollabError, Result};
use crate::history::VersionControl;
use crate::merge::MergeService;
use crate::middleware::{auth_middleware, AuthUser};
use crate::models::UserRole;
use crate::sync::SyncEngine;
use crate::user::UserService;
use crate::workspace::WorkspaceService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub auth: Arc<AuthService>,
    pub user: Arc<UserService>,
    pub workspace: Arc<WorkspaceService>,
    pub history: Arc<VersionControl>,
    pub merge: Arc<MergeService>,
    pub backup: Arc<BackupService>,
    pub sync: Arc<SyncEngine>,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        // Workspaces
        .route("/workspaces", post(create_workspace))
        .route("/workspaces", get(list_workspaces))
        .route("/workspaces/:id", get(get_workspace))
        .route("/workspaces/:id", put(update_workspace))
        .route("/workspaces/:id", delete(delete_workspace))
        // Members
        .route("/workspaces/:id/members", post(add_member))
        .route("/workspaces/:id/members/:user_id", delete(remove_member))
        .route("/workspaces/:id/members/:user_id/role", put(change_role))
        .route("/workspaces/:id/members", get(list_members))
        // Version Control - Commits
        .route("/workspaces/:id/commits", post(create_commit))
        .route("/workspaces/:id/commits", get(list_commits))
        .route("/workspaces/:id/commits/:commit_id", get(get_commit))
        .route("/workspaces/:id/restore/:commit_id", post(restore_to_commit))
        // Version Control - Snapshots
        .route("/workspaces/:id/snapshots", post(create_snapshot))
        .route("/workspaces/:id/snapshots", get(list_snapshots))
        .route("/workspaces/:id/snapshots/:name", get(get_snapshot))
        // Fork and Merge
        .route("/workspaces/:id/fork", post(fork_workspace))
        .route("/workspaces/:id/forks", get(list_forks))
        .route("/workspaces/:id/merge", post(merge_workspaces))
        .route("/workspaces/:id/merges", get(list_merges))
        // Backup and Restore
        .route("/workspaces/:id/backup", post(create_backup))
        .route("/workspaces/:id/backups", get(list_backups))
        .route("/workspaces/:id/backups/:backup_id", delete(delete_backup))
        .route("/workspaces/:id/restore", post(restore_workspace))
        // State Management
        .route("/workspaces/:id/state", get(get_workspace_state))
        .route("/workspaces/:id/state", post(update_workspace_state))
        .route("/workspaces/:id/state/history", get(get_state_history))
        .route_layer(middleware::from_fn_with_state(
            state.auth.clone(),
            auth_middleware,
        ));

    // Combine routes
    Router::new().merge(public_routes).merge(protected_routes).with_state(state)
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommitRequest {
    pub message: String,
    pub changes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    pub name: String,
    pub description: Option<String>,
    pub commit_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

// ===== Error Handling =====

impl IntoResponse for CollabError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CollabError::AuthenticationFailed(msg) => (StatusCode::UNAUTHORIZED, msg),
            CollabError::AuthorizationFailed(msg) => (StatusCode::FORBIDDEN, msg),
            CollabError::WorkspaceNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            CollabError::UserNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            CollabError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            CollabError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            CollabError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// ===== Handler Functions =====

/// Register a new user
async fn register(
    State(state): State<ApiState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    // Create user
    let user = state
        .user
        .create_user(payload.username, payload.email, payload.password)
        .await?;

    // Generate token
    let token = state.auth.generate_token(&user)?;

    Ok(Json(AuthResponse {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// Login user
async fn login(
    State(state): State<ApiState>,
    Json(payload): Json<Credentials>,
) -> Result<Json<AuthResponse>> {
    // Authenticate user
    let user = state.user.authenticate(&payload.username, &payload.password).await?;

    // Generate token
    let token = state.auth.generate_token(&user)?;

    Ok(Json(AuthResponse {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// Create a new workspace
async fn create_workspace(
    State(state): State<ApiState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Create workspace
    let workspace = state
        .workspace
        .create_workspace(payload.name, payload.description, auth_user.user_id)
        .await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// List user's workspaces
async fn list_workspaces(
    State(state): State<ApiState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // List workspaces
    let workspaces = state.workspace.list_user_workspaces(auth_user.user_id).await?;

    Ok(Json(serde_json::to_value(workspaces)?))
}

/// Get workspace by ID
async fn get_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(id, auth_user.user_id).await?;

    // Get workspace
    let workspace = state.workspace.get_workspace(id).await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// Update workspace
async fn update_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Update workspace (permission check inside)
    let workspace = state
        .workspace
        .update_workspace(id, auth_user.user_id, payload.name, payload.description, None)
        .await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// Delete workspace
async fn delete_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<StatusCode> {
    // Delete workspace (permission check inside)
    state.workspace.delete_workspace(id, auth_user.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Add member to workspace
async fn add_member(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>> {
    // Add member (permission check inside)
    let member = state
        .workspace
        .add_member(workspace_id, auth_user.user_id, payload.user_id, payload.role)
        .await?;

    Ok(Json(serde_json::to_value(member)?))
}

/// Remove member from workspace
async fn remove_member(
    State(state): State<ApiState>,
    Path((workspace_id, member_user_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<StatusCode> {
    // Remove member (permission check inside)
    state
        .workspace
        .remove_member(workspace_id, auth_user.user_id, member_user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Change member role
async fn change_role(
    State(state): State<ApiState>,
    Path((workspace_id, member_user_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<ChangeRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    // Change role (permission check inside)
    let member = state
        .workspace
        .change_role(workspace_id, auth_user.user_id, member_user_id, payload.role)
        .await?;

    Ok(Json(serde_json::to_value(member)?))
}

/// List workspace members
async fn list_members(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List all members
    let members = state.workspace.list_members(workspace_id).await?;

    Ok(Json(serde_json::to_value(members)?))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Readiness check endpoint with database health check
async fn readiness_check(State(state): State<ApiState>) -> impl IntoResponse {
    // Check database connection by running a simple query
    // We need to access the database pool - let's add it to ApiState or use workspace service
    // For now, we'll use a workspace service method to check DB health
    let db_healthy = state.workspace.check_database_health().await;

    if db_healthy {
        Json(serde_json::json!({
            "status": "ready",
            "database": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "database": "unhealthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
            .into_response()
    }
}

// ===== Validation Helpers =====

/// Validate commit message
fn validate_commit_message(message: &str) -> Result<()> {
    if message.is_empty() {
        return Err(CollabError::InvalidInput("Commit message cannot be empty".to_string()));
    }
    if message.len() > 500 {
        return Err(CollabError::InvalidInput(
            "Commit message cannot exceed 500 characters".to_string(),
        ));
    }
    Ok(())
}

/// Validate snapshot name
fn validate_snapshot_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(CollabError::InvalidInput("Snapshot name cannot be empty".to_string()));
    }
    if name.len() > 100 {
        return Err(CollabError::InvalidInput(
            "Snapshot name cannot exceed 100 characters".to_string(),
        ));
    }
    // Allow alphanumeric, hyphens, underscores, and dots
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(CollabError::InvalidInput(
            "Snapshot name can only contain alphanumeric characters, hyphens, underscores, and dots".to_string(),
        ));
    }
    Ok(())
}

// ===== Version Control Handlers =====

/// Create a commit in the workspace.
///
/// Creates a new commit capturing the current state of the workspace along with
/// a description of changes. This is similar to `git commit`.
///
/// # Requirements
/// - User must be a workspace member with Editor or Admin role
/// - Commit message must be 1-500 characters
///
/// # Request Body
/// - `message`: Commit message describing the changes (required, 1-500 chars)
/// - `changes`: JSON object describing what changed
///
/// # Response
/// Returns the created Commit object with:
/// - `id`: Unique commit ID
/// - `workspace_id`: ID of the workspace
/// - `author_id`: ID of the user who created the commit
/// - `message`: Commit message
/// - `parent_id`: ID of the parent commit (null for first commit)
/// - `version`: Version number (auto-incremented)
/// - `snapshot`: Full workspace state at this commit
/// - `changes`: Description of what changed
/// - `created_at`: Timestamp
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: User is not Editor or Admin
/// - `404 Not Found`: Workspace not found or user not a member
/// - `400 Bad Request`: Invalid commit message
async fn create_commit(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateCommitRequest>,
) -> Result<Json<serde_json::Value>> {
    // Validate input
    validate_commit_message(&payload.message)?;

    // Verify user has permission (Editor or Admin)
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can create commits".to_string(),
        ));
    }

    // Get current workspace state
    let workspace = state.workspace.get_workspace(workspace_id).await?;

    // Get parent commit (latest)
    let parent = state.history.get_latest_commit(workspace_id).await?;
    let parent_id = parent.as_ref().map(|c| c.id);
    let version = parent.as_ref().map(|c| c.version + 1).unwrap_or(1);

    // Create snapshot of current state
    let snapshot = serde_json::to_value(&workspace)?;

    // Create commit
    let commit = state
        .history
        .create_commit(
            workspace_id,
            auth_user.user_id,
            payload.message,
            parent_id,
            version,
            snapshot,
            payload.changes,
        )
        .await?;

    Ok(Json(serde_json::to_value(commit)?))
}

/// List commits for a workspace.
///
/// Returns the commit history for a workspace in reverse chronological order
/// (most recent first). Supports pagination via query parameters.
///
/// # Requirements
/// - User must be a workspace member (any role)
///
/// # Query Parameters
/// - `limit`: Number of commits to return (default: 50, max: 100)
/// - `offset`: Number of commits to skip (default: 0)
///
/// # Response
/// Returns a JSON object with:
/// - `commits`: Array of Commit objects
/// - `pagination`: Object with `limit` and `offset` values
///
/// # Example
/// ```
/// GET /workspaces/{id}/commits?limit=20&offset=0
/// ```
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `404 Not Found`: Workspace not found or user not a member
async fn list_commits(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Validate pagination params
    let limit = pagination.limit.clamp(1, 100);

    // Get commit history
    let commits = state.history.get_history(workspace_id, Some(limit)).await?;

    // Return with pagination metadata
    Ok(Json(serde_json::json!({
        "commits": commits,
        "pagination": {
            "limit": limit,
            "offset": pagination.offset,
        }
    })))
}

/// Get a specific commit by ID.
///
/// Retrieves detailed information about a specific commit, including the full
/// workspace state snapshot at that point in time.
///
/// # Requirements
/// - User must be a workspace member (any role)
/// - Commit must belong to the specified workspace
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `404 Not Found`: Commit or workspace not found
/// - `400 Bad Request`: Commit doesn't belong to this workspace
async fn get_commit(
    State(state): State<ApiState>,
    Path((workspace_id, commit_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Get commit
    let commit = state.history.get_commit(commit_id).await?;

    // Verify commit belongs to this workspace
    if commit.workspace_id != workspace_id {
        return Err(CollabError::InvalidInput(
            "Commit does not belong to this workspace".to_string(),
        ));
    }

    Ok(Json(serde_json::to_value(commit)?))
}

/// Restore workspace to a specific commit.
///
/// Reverts the workspace to the state captured in the specified commit.
/// This is a destructive operation that should be used carefully.
///
/// # Requirements
/// - User must be a workspace member with Editor or Admin role
/// - Commit must exist and belong to the workspace
///
/// # Response
/// Returns an object with:
/// - `workspace_id`: ID of the restored workspace
/// - `commit_id`: ID of the commit that was restored
/// - `restored_state`: The workspace state from the commit
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: User is not Editor or Admin
/// - `404 Not Found`: Commit or workspace not found
async fn restore_to_commit(
    State(state): State<ApiState>,
    Path((workspace_id, commit_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has permission (Editor or Admin)
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can restore workspaces".to_string(),
        ));
    }

    // Restore to commit
    let restored_state = state.history.restore_to_commit(workspace_id, commit_id).await?;

    Ok(Json(serde_json::json!({
        "workspace_id": workspace_id,
        "commit_id": commit_id,
        "restored_state": restored_state
    })))
}

/// Create a named snapshot.
///
/// Creates a named reference to a specific commit, similar to a git tag.
/// Snapshots are useful for marking important states like releases.
///
/// # Requirements
/// - User must be a workspace member with Editor or Admin role
/// - Snapshot name must be 1-100 characters, alphanumeric with -, _, or .
/// - Commit must exist
///
/// # Request Body
/// - `name`: Name for the snapshot (required, 1-100 chars, alphanumeric)
/// - `description`: Optional description
/// - `commit_id`: ID of the commit to snapshot
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `403 Forbidden`: User is not Editor or Admin
/// - `404 Not Found`: Workspace or commit not found
/// - `400 Bad Request`: Invalid snapshot name
async fn create_snapshot(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateSnapshotRequest>,
) -> Result<Json<serde_json::Value>> {
    // Validate input
    validate_snapshot_name(&payload.name)?;

    // Verify user has permission (Editor or Admin)
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can create snapshots".to_string(),
        ));
    }

    // Create snapshot
    let snapshot = state
        .history
        .create_snapshot(
            workspace_id,
            payload.name,
            payload.description,
            payload.commit_id,
            auth_user.user_id,
        )
        .await?;

    Ok(Json(serde_json::to_value(snapshot)?))
}

/// List snapshots for a workspace.
///
/// Returns all named snapshots for the workspace in reverse chronological order.
///
/// # Requirements
/// - User must be a workspace member (any role)
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `404 Not Found`: Workspace not found or user not a member
async fn list_snapshots(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List snapshots
    let snapshots = state.history.list_snapshots(workspace_id).await?;

    Ok(Json(serde_json::to_value(snapshots)?))
}

/// Get a specific snapshot by name.
///
/// Retrieves details about a named snapshot, including which commit it references.
///
/// # Requirements
/// - User must be a workspace member (any role)
///
/// # Errors
/// - `401 Unauthorized`: Not authenticated
/// - `404 Not Found`: Snapshot, workspace not found, or user not a member
async fn get_snapshot(
    State(state): State<ApiState>,
    Path((workspace_id, name)): Path<(Uuid, String)>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Get snapshot
    let snapshot = state.history.get_snapshot(workspace_id, &name).await?;

    Ok(Json(serde_json::to_value(snapshot)?))
}

// ===== Fork and Merge Handlers =====

#[derive(Debug, Deserialize)]
pub struct ForkWorkspaceRequest {
    pub name: Option<String>,
    pub fork_point_commit_id: Option<Uuid>,
}

/// Fork a workspace
async fn fork_workspace(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<ForkWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has access to source workspace
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Fork workspace
    let forked = state
        .workspace
        .fork_workspace(workspace_id, payload.name, auth_user.user_id, payload.fork_point_commit_id)
        .await?;

    Ok(Json(serde_json::to_value(forked)?))
}

/// List all forks of a workspace
async fn list_forks(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List forks
    let forks = state.workspace.list_forks(workspace_id).await?;

    Ok(Json(serde_json::to_value(forks)?))
}

#[derive(Debug, Deserialize)]
pub struct MergeWorkspacesRequest {
    pub source_workspace_id: Uuid,
}

/// Merge changes from another workspace
async fn merge_workspaces(
    State(state): State<ApiState>,
    Path(target_workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<MergeWorkspacesRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has permission to merge into target
    let member = state.workspace.get_member(target_workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can merge workspaces".to_string(),
        ));
    }

    // Perform merge
    let (merged_state, conflicts) = state
        .merge
        .merge_workspaces(payload.source_workspace_id, target_workspace_id, auth_user.user_id)
        .await?;

    Ok(Json(serde_json::json!({
        "merged_state": merged_state,
        "conflicts": conflicts,
        "has_conflicts": !conflicts.is_empty()
    })))
}

/// List merge operations for a workspace
async fn list_merges(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List merges
    let merges = state.merge.list_merges(workspace_id).await?;

    Ok(Json(serde_json::to_value(merges)?))
}

// ===== Backup and Restore Handlers =====

#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub storage_backend: Option<String>,
    pub format: Option<String>,
    pub commit_id: Option<Uuid>,
}

/// Create a backup of a workspace
async fn create_backup(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateBackupRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has permission
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can create backups".to_string(),
        ));
    }

    // Determine storage backend
    let storage_backend = match payload.storage_backend.as_deref() {
        Some("s3") => StorageBackend::S3,
        Some("azure") => StorageBackend::Azure,
        Some("gcs") => StorageBackend::Gcs,
        Some("custom") => StorageBackend::Custom,
        _ => StorageBackend::Local,
    };

    // Create backup
    let backup = state
        .backup
        .backup_workspace(
            workspace_id,
            auth_user.user_id,
            storage_backend,
            payload.format,
            payload.commit_id,
        )
        .await?;

    Ok(Json(serde_json::to_value(backup)?))
}

/// List backups for a workspace
async fn list_backups(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List backups
    let backups = state.backup.list_backups(workspace_id, Some(pagination.limit)).await?;

    Ok(Json(serde_json::to_value(backups)?))
}

/// Delete a backup
async fn delete_backup(
    State(state): State<ApiState>,
    Path((workspace_id, backup_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<StatusCode> {
    // Verify user has permission
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin) {
        return Err(CollabError::AuthorizationFailed("Only Admins can delete backups".to_string()));
    }

    // Delete backup
    state.backup.delete_backup(backup_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RestoreWorkspaceRequest {
    pub backup_id: Uuid,
    pub target_workspace_id: Option<Uuid>,
}

/// Restore a workspace from a backup
async fn restore_workspace(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<RestoreWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has permission
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins can restore workspaces".to_string(),
        ));
    }

    // Restore workspace
    let restored_id = state
        .backup
        .restore_workspace(payload.backup_id, payload.target_workspace_id, auth_user.user_id)
        .await?;

    Ok(Json(serde_json::json!({
        "workspace_id": restored_id,
        "restored_from_backup": payload.backup_id
    })))
}

// ===== State Management Handlers =====

/// Get current workspace state
async fn get_workspace_state(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Get version if specified
    let version = params.get("version").and_then(|v| v.parse::<i64>().ok());

    // Get state from sync engine - try full workspace state first
    let sync_state = if let Some(version) = version {
        state.sync.load_state_snapshot(workspace_id, Some(version)).await?
    } else {
        // Try to get full workspace state using CoreBridge
        if let Ok(Some(full_state)) = state.sync.get_full_workspace_state(workspace_id).await {
            // Get workspace for version info
            let workspace = state.workspace.get_workspace(workspace_id).await?;
            return Ok(Json(serde_json::json!({
                "workspace_id": workspace_id,
                "version": workspace.version,
                "state": full_state,
                "last_updated": workspace.updated_at
            })));
        }

        // Fallback to in-memory state
        state.sync.get_state(workspace_id)
    };

    if let Some(state_val) = sync_state {
        Ok(Json(serde_json::json!({
            "workspace_id": workspace_id,
            "version": state_val.version,
            "state": state_val.state,
            "last_updated": state_val.last_updated
        })))
    } else {
        // Return workspace metadata if no state available
        let workspace = state.workspace.get_workspace(workspace_id).await?;
        Ok(Json(serde_json::json!({
            "workspace_id": workspace_id,
            "version": workspace.version,
            "state": workspace.config,
            "last_updated": workspace.updated_at
        })))
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceStateRequest {
    pub state: serde_json::Value,
}

/// Update workspace state
async fn update_workspace_state(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<UpdateWorkspaceStateRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has permission
    let member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;
    if !matches!(member.role, UserRole::Admin | UserRole::Editor) {
        return Err(CollabError::AuthorizationFailed(
            "Only Admins and Editors can update workspace state".to_string(),
        ));
    }

    // Update state in sync engine
    state.sync.update_state(workspace_id, payload.state.clone())?;

    // Record state change
    let workspace = state.workspace.get_workspace(workspace_id).await?;
    state
        .sync
        .record_state_change(
            workspace_id,
            "full_sync",
            payload.state.clone(),
            workspace.version + 1,
            auth_user.user_id,
        )
        .await?;

    Ok(Json(serde_json::json!({
        "workspace_id": workspace_id,
        "version": workspace.version + 1,
        "state": payload.state
    })))
}

/// Get state change history
async fn get_state_history(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // Get since_version if specified
    let since_version =
        params.get("since_version").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);

    // Get state changes
    let changes = state.sync.get_state_changes_since(workspace_id, since_version).await?;

    Ok(Json(serde_json::json!({
        "workspace_id": workspace_id,
        "since_version": since_version,
        "changes": changes
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        // Just ensure router can be created
        use crate::events::EventBus;
        let event_bus = Arc::new(EventBus::new(100));
        let state = ApiState {
            auth: Arc::new(AuthService::new("test".to_string())),
            user: Arc::new(UserService::new(
                todo!(),
                Arc::new(AuthService::new("test".to_string())),
            )),
            workspace: Arc::new(WorkspaceService::new(todo!())),
            history: Arc::new(VersionControl::new(todo!())),
            merge: Arc::new(MergeService::new(todo!())),
            backup: Arc::new(BackupService::new(todo!(), None, todo!(), todo!())),
            sync: Arc::new(SyncEngine::new(event_bus)),
        };
        let _router = create_router(state);
    }
}

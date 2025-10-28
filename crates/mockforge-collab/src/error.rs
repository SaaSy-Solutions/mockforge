//! Error types for collaboration features

/// Result type for collaboration operations
pub type Result<T> = std::result::Result<T, CollabError>;

/// Errors that can occur during collaboration operations
#[derive(Debug, thiserror::Error)]
pub enum CollabError {
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authorization failed (insufficient permissions)
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    /// Workspace not found
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Conflict detected
    #[error("Conflict detected: {0}")]
    ConflictDetected(String),

    /// Sync error
    #[error("Sync error: {0}")]
    SyncError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Operation timeout
    #[error("Operation timeout: {0}")]
    Timeout(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for CollabError {
    fn from(err: sqlx::Error) -> Self {
        CollabError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for CollabError {
    fn from(err: serde_json::Error) -> Self {
        CollabError::SerializationError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for CollabError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        CollabError::Timeout(err.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for CollabError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        CollabError::DatabaseError(err.to_string())
    }
}

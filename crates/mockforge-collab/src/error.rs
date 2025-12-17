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
        Self::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for CollabError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for CollabError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(err.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for CollabError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_failed() {
        let err = CollabError::AuthenticationFailed("invalid token".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("invalid token"));
    }

    #[test]
    fn test_authorization_failed() {
        let err = CollabError::AuthorizationFailed("missing permission".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Authorization failed"));
        assert!(msg.contains("missing permission"));
    }

    #[test]
    fn test_workspace_not_found() {
        let err = CollabError::WorkspaceNotFound("ws-123".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Workspace not found"));
        assert!(msg.contains("ws-123"));
    }

    #[test]
    fn test_user_not_found() {
        let err = CollabError::UserNotFound("user-456".to_string());
        let msg = err.to_string();
        assert!(msg.contains("User not found"));
        assert!(msg.contains("user-456"));
    }

    #[test]
    fn test_conflict_detected() {
        let err = CollabError::ConflictDetected("concurrent edit".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Conflict detected"));
        assert!(msg.contains("concurrent edit"));
    }

    #[test]
    fn test_sync_error() {
        let err = CollabError::SyncError("sync failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Sync error"));
        assert!(msg.contains("sync failed"));
    }

    #[test]
    fn test_database_error() {
        let err = CollabError::DatabaseError("connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Database error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_websocket_error() {
        let err = CollabError::WebSocketError("connection closed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("WebSocket error"));
        assert!(msg.contains("connection closed"));
    }

    #[test]
    fn test_serialization_error() {
        let err = CollabError::SerializationError("invalid json".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Serialization error"));
        assert!(msg.contains("invalid json"));
    }

    #[test]
    fn test_invalid_input() {
        let err = CollabError::InvalidInput("empty name".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid input"));
        assert!(msg.contains("empty name"));
    }

    #[test]
    fn test_already_exists() {
        let err = CollabError::AlreadyExists("workspace-name".to_string());
        let msg = err.to_string();
        assert!(msg.contains("already exists"));
        assert!(msg.contains("workspace-name"));
    }

    #[test]
    fn test_timeout() {
        let err = CollabError::Timeout("operation timed out".to_string());
        let msg = err.to_string();
        assert!(msg.contains("timeout"));
        assert!(msg.contains("operation timed out"));
    }

    #[test]
    fn test_connection_error() {
        let err = CollabError::ConnectionError("host unreachable".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Connection error"));
        assert!(msg.contains("host unreachable"));
    }

    #[test]
    fn test_version_mismatch() {
        let err = CollabError::VersionMismatch {
            expected: 10,
            actual: 8,
        };
        let msg = err.to_string();
        assert!(msg.contains("Version mismatch"));
        assert!(msg.contains("10"));
        assert!(msg.contains("8"));
    }

    #[test]
    fn test_internal_error() {
        let err = CollabError::Internal("unexpected failure".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Internal error"));
        assert!(msg.contains("unexpected failure"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err: serde_json::Error = serde_json::from_str::<String>("invalid").unwrap_err();
        let err: CollabError = json_err.into();
        assert!(matches!(err, CollabError::SerializationError(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = CollabError::AuthenticationFailed("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("AuthenticationFailed"));
    }
}

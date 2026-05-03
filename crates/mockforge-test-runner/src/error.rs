//! Error types for the test runner worker.

use thiserror::Error;

/// Result alias for runner operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type. Wraps the leaf failures we expect at runtime
/// (Redis, HTTP callback, JSON, executor) so the dispatcher can decide
/// whether a failure is retriable per-kind.
#[derive(Debug, Error)]
pub enum Error {
    /// Failure talking to Redis (connection, command, parse).
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Failure talking to the registry's internal callback endpoints.
    #[error("registry callback error: {0}")]
    Callback(#[from] reqwest::Error),

    /// Failure parsing a queue payload or callback body.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Executor returned an error. Per-kind impls should be the place
    /// most user-visible failures originate.
    #[error("executor error: {0}")]
    Executor(String),

    /// Configuration was missing or malformed at startup.
    #[error("configuration error: {0}")]
    Config(String),

    /// No executor registered for the requested kind. The dispatcher
    /// treats this as a permanent failure (job → 'errored') rather
    /// than retrying.
    #[error("no executor registered for kind '{0}'")]
    UnknownKind(String),

    /// Catch-all for unexpected failures. Avoid using outside leaf code
    /// — the typed variants above carry better diagnostic info.
    #[error("unexpected error: {0}")]
    Other(#[from] anyhow::Error),
}

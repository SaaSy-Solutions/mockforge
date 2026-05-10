//! Error types for `mockforge-chaos-proxy`.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ChaosProxyError>;

/// Setup-failure errors. Per-request network failures are NOT
/// errors — they're [`crate::ChaosOutcome`] data points. `Err` here
/// means the executor should abort the campaign rather than continue
/// probing.
#[derive(Debug, Error)]
pub enum ChaosProxyError {
    /// `reqwest::Client::builder()` failed. Should be unreachable on
    /// supported platforms.
    #[error("HTTP client build failed: {0}")]
    ClientBuild(#[source] reqwest::Error),

    /// SSRF guard rejected the target URL. Carries the guard's
    /// reason so the executor can surface it in the abort log.
    #[error("target URL rejected by SSRF guard: {0}")]
    SsrfRejected(String),

    /// Caller passed an HTTP method string `reqwest` couldn't parse.
    #[error("invalid HTTP method")]
    BadMethod,
}

//! Protocol contracts with core-hosted authentication, logging and latency adapters.

pub use mockforge_protocol::protocol_abstraction::*;

pub mod auth;
pub mod middleware;

pub use auth::{AuthMiddleware, AuthResult, Claims};
pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};

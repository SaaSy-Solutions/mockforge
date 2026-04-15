//! `MockAiBehavior` trait — type-erased handle for consumers that hold a MockAI
//! reference without depending on the full implementation in mockforge-core.
//!
//! Concrete implementation lives in `mockforge-core::intelligent_behavior::MockAI`.

use crate::error::Result;
use crate::intelligent_behavior::{Request, Response};
use async_trait::async_trait;

/// Trait implemented by MockAI (in `mockforge-core`) to allow type-erased handles.
///
/// Consumers that need to hold a reference to a MockAI instance without depending
/// on the full implementation (e.g., `mockforge-chaos`, `mockforge-openapi`'s
/// route builder) can hold `Arc<RwLock<dyn MockAiBehavior + Send + Sync>>` and
/// dispatch requests through [`process_request`] without pulling in the full
/// MockAI engine.
///
/// [`process_request`]: MockAiBehavior::process_request
#[async_trait]
pub trait MockAiBehavior: Send + Sync {
    /// Downcast helper — implementations may override to expose their underlying type.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Process an incoming HTTP request through the behavioral-mock pipeline
    /// and return the generated response.
    ///
    /// Implementations typically extract session context from headers, build a
    /// response (schema-based, example-based, or AI-generated), and record the
    /// exchange for downstream behavioural analysis.
    async fn process_request(&self, request: &Request) -> Result<Response>;
}

//! `MockAiBehavior` trait — type-erased handle for consumers that hold a MockAI
//! reference without depending on the full implementation in mockforge-core.
//!
//! Concrete implementation lives in `mockforge-core::intelligent_behavior::MockAI`.

/// Trait implemented by MockAI (in `mockforge-core`) to allow type-erased handles.
///
/// Consumers that need to hold a reference to a MockAI instance without depending
/// on the full implementation (e.g., `mockforge-chaos`) can hold
/// `Arc<RwLock<dyn MockAiBehavior + Send + Sync>>`.
pub trait MockAiBehavior: Send + Sync {
    /// Downcast helper — implementations may override to expose their underlying type.
    fn as_any(&self) -> &dyn std::any::Any;
}

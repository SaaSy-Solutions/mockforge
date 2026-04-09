//! Error types for the registry server.
//!
//! The actual definitions now live in `mockforge_registry_core::error` so
//! that both the SaaS binary and the OSS admin UI share the same `ApiError`
//! / `StoreError` types and the same `IntoResponse` implementation.
//!
//! This module re-exports those types under the old `crate::error::*` path
//! so existing handler and middleware imports continue to work.

pub use mockforge_registry_core::error::{ApiError, ApiResult, StoreError, StoreResult};

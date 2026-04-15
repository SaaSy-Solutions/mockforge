//! AI-assisted response generation for dynamic mock endpoints.
//!
//! The data types and prompt expansion helper previously lived here; they
//! have been promoted to [`mockforge_foundation::ai_response`] so that
//! `mockforge-openapi` (and any other leaf crate) can depend on them without
//! pulling in the full `mockforge-core` graph. This module re-exports them
//! for backwards compatibility.

pub use mockforge_foundation::ai_response::*;

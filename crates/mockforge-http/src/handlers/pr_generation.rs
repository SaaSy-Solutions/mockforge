//! PR generation handlers — re-export shim.
//!
//! Issue #555 phase 2 moved this module to
//! [`mockforge_intelligence::handlers::pr_generation`]. The handler's only
//! foreign import was `mockforge_intelligence::pr_generation::*` (the
//! underlying `pr_generation` module had already moved to intelligence in
//! Issue #562 phase 1), so the handler followed once intelligence grew an
//! `axum` dep.
//!
//! This shim keeps existing
//! `mockforge_http::handlers::pr_generation::{PRGenerationState,
//! GeneratePRRequest, pr_generation_router, ...}` callers (notably the
//! router wired up in `mockforge_http::lib`) resolving unchanged. Future
//! phases of #555 may drop this shim; until then, prefer importing from
//! `mockforge_intelligence::handlers::pr_generation` directly in new code.

pub use mockforge_intelligence::handlers::pr_generation::*;

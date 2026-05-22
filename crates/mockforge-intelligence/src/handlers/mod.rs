//! HTTP handlers for AI-coupled features.
//!
//! Issue #555 carves the AI-touching handler files out of
//! `mockforge-http/src/handlers/` and re-homes them next to the
//! intelligence logic they wrap. The HTTP crate keeps thin
//! re-export shims at the legacy paths for one minor version so
//! router-construction code in `mockforge_http::lib` keeps resolving
//! unchanged.
//!
//! Currently migrated:
//! - [`pr_generation`]: PR generation HTTP surface (#555 phase 2). The
//!   underlying `pr_generation` module moved in #562 phase 1; the
//!   handler followed once intelligence grew an axum dep.

pub mod pr_generation;
pub mod semantic_drift;

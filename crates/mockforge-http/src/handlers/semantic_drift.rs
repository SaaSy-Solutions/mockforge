//! Semantic drift incident handlers — re-export shim.
//!
//! Issue #555 phase 3 moved this module to
//! [`mockforge_intelligence::handlers::semantic_drift`]. All three of its
//! foreign deps had already migrated to intelligence in earlier #562
//! phases — `ai_contract_diff` in phase 4, `incidents::semantic_manager`
//! in phase 9, and the sqlx `Database` wrapper via the #611 prereq — so
//! the handler followed once the path-rewrites were mechanical.
//!
//! This shim keeps existing
//! `mockforge_http::handlers::semantic_drift::{SemanticDriftState,
//! ...}` callers (notably the router code in `mockforge_http::lib`)
//! resolving unchanged. Future #555 phases may drop the shim; prefer
//! `mockforge_intelligence::handlers::semantic_drift::*` in new code.

pub use mockforge_intelligence::handlers::semantic_drift::*;

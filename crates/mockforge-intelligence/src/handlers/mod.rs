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
//! - [`semantic_drift`]: Semantic-drift incident HTTP surface (#555
//!   phase 3). All three of its foreign deps moved here in earlier #562
//!   phases (`ai_contract_diff` in phase 4, `incidents::semantic_manager`
//!   in phase 9) and the sqlx wrapper landed via the #611 prereq.
//! - [`threat_modeling`]: Threat-modeling HTTP surface (#555 phase 4).
//!   Same pattern as `semantic_drift` — `contract_drift::threat_modeling`
//!   moved here in #562 phase 3, `incidents::integrations` moved in
//!   phase 9, and the threat types live in
//!   `mockforge_foundation::threat_modeling_types`.

pub mod ai_studio;
pub mod forecasting;
pub mod pr_generation;
pub mod semantic_drift;
pub mod threat_modeling;

//! AI-powered spec generation — re-export shim.
//!
//! Issue #656 split the original ~994L `management/ai_gen.rs` into four
//! topic files (this one, `chaos_admin`, `rule_explanations`,
//! `traffic_to_openapi`) and then moved the genuinely-AI subset to
//! [`mockforge_intelligence::handlers::spec_generation`]. The moved
//! handler dropped the `State<ManagementState>` extractor (it was
//! never read), and the `data-faker` feature flag now propagates from
//! `mockforge-http` to `mockforge-intelligence` to keep the dual
//! data-faker / stub-503 contract on a single toggle.
//!
//! This shim keeps `mockforge_http::management::ai_gen::*` callers
//! (notably the route wiring in `management/mod.rs`) resolving
//! unchanged. Future drains may drop this shim; until then, prefer
//! importing from `mockforge_intelligence::handlers::spec_generation`
//! directly in new code.

pub use mockforge_intelligence::handlers::spec_generation::*;

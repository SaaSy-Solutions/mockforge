//! Shared core for MockForge registry backends.
//!
//! This crate houses the domain models, storage trait, error types,
//! authentication middleware, and OSS-safe HTTP handlers that power both:
//!
//!   * `mockforge-registry-server` — the multi-tenant SaaS binary (Postgres
//!     backend plus SaaS-only integrations like Stripe, SSO/SAML, S3, Redis).
//!   * `mockforge-ui`'s embedded OSS admin server (SQLite backend only).
//!
//! This is the Phase 2a shell. The actual module moves happen in follow-up
//! commits to keep each step reviewable. Until those land, the crate is
//! intentionally empty beyond this module doc — the workspace still compiles
//! because nothing depends on it yet.

#![deny(unsafe_code)]
#![allow(clippy::module_inception)]

// TODO: move `mockforge-registry-server::{error, models, store, middleware,
// validation, auth, two_factor}` into this crate, plus OSS-safe handlers
// (auth/tokens/users/orgs/audit/gdpr/health/legal/status/faq).

/// Placeholder marker type so the crate has something to export while the
/// real modules are being moved in. Will be removed once `models` lands.
#[doc(hidden)]
pub struct CorePlaceholder;

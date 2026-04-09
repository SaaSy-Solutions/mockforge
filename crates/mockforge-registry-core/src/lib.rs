//! Shared core for MockForge registry backends.
//!
//! This crate houses the domain models, storage trait, error types,
//! authentication middleware, and OSS-safe HTTP handlers that power both:
//!
//!   * `mockforge-registry-server` — the multi-tenant SaaS binary (Postgres
//!     backend plus SaaS-only integrations like Stripe, SSO/SAML, S3, Redis).
//!   * `mockforge-ui`'s embedded OSS admin server (SQLite backend only).
//!
//! The cloud-core extraction is being landed incrementally. Modules move
//! over in small reviewable commits; this file tracks what's here so far.

#![deny(unsafe_code)]
#![allow(clippy::module_inception)]

pub mod error;
pub mod models;
pub mod permissions;
pub mod store;
pub mod validation;

pub use error::{ApiError, ApiResult, StoreError, StoreResult};

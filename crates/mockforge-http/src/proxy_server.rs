//! Browser/Mobile Proxy Server — re-export shim.
//!
//! Issue #555 phase 1 moved this module to
//! [`mockforge_proxy::server`]. The file's only foreign imports were
//! already in `mockforge_proxy` (`body_transform`, `config::ProxyConfig`),
//! so the move was mechanical.
//!
//! This shim keeps existing
//! `mockforge_http::proxy_server::{ProxyServer, ...}` callers (notably
//! the workspace's `tests/proxy_verification_tests.rs`) resolving
//! unchanged. Future phases of #555 may drop this shim entirely; until
//! then, prefer importing from `mockforge_proxy::server` directly in
//! new code.

pub use mockforge_proxy::server::*;

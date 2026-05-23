//! Reality-slider middleware — re-export shim.
//!
//! Issue #555 phase 8 moved this module to
//! [`mockforge_proxy::reality`]. The middleware's only foreign dep
//! (`mockforge_core::consistency::UnifiedState`) already lived in the
//! proxy crate's dep graph, so the move was a straight transplant.
//!
//! This shim keeps existing
//! `mockforge_http::reality_proxy::{RealityProxyConfig,
//! reality_proxy_middleware}` callers (notably the layer wired up in
//! `mockforge_http::lib`) resolving unchanged. Future phases of #555
//! may drop this shim; until then, prefer importing from
//! `mockforge_proxy::reality` directly in new code.

pub use mockforge_proxy::reality::*;

//! Management endpoints for the server-side conformance violation feed.
//!
//! Issue #79 round 12 — Srikanth asked for a TUI section showing
//! conformance failures captured on the **server** side (incoming
//! requests that violate the loaded spec), so he can cross-check his
//! proxy logs against MockForge's view of the same traffic.
//!
//! Storage lives in `mockforge_foundation::conformance_violations`
//! (bounded, in-memory ring buffer populated by the OpenAPI router).
//! This module just exposes:
//!
//!   - `GET    /__mockforge/api/conformance/violations` → recent violations
//!   - `DELETE /__mockforge/api/conformance/violations` → reset buffer
//!
//! The TUI's `Conformance` screen polls the GET endpoint every tick.

use axum::extract::Query;
use axum::Json;
use mockforge_foundation::conformance_violations;
use serde::{Deserialize, Serialize};

/// Query params for the GET endpoint.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ListQuery {
    /// Cap the response at this many newest violations. Defaults to the
    /// buffer's full size (currently 256) when omitted.
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListResponse {
    /// Snapshot, newest-first.
    pub violations: Vec<conformance_violations::ServerConformanceViolation>,
    /// Total violations buffered before truncation. Useful for the TUI's
    /// "N violations (showing X)" header when limit < total.
    pub total: usize,
}

pub(crate) async fn get_conformance_violations(
    Query(query): Query<ListQuery>,
) -> Json<ListResponse> {
    let mut violations = conformance_violations::snapshot();
    let total = violations.len();
    if let Some(limit) = query.limit {
        violations.truncate(limit);
    }
    Json(ListResponse { violations, total })
}

pub(crate) async fn clear_conformance_violations() -> Json<serde_json::Value> {
    let before = conformance_violations::len();
    conformance_violations::clear();
    Json(serde_json::json!({ "cleared": before }))
}

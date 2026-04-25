//! Minimal OTLP HTTP receiver scaffold (#233).
//!
//! Hosted-mock deployments configured with `MOCKFORGE_OTLP_ENDPOINT` ship
//! spans to this endpoint. **This is intentionally a scaffold today** — it
//! validates the deployment-scoped JWT, parses the OTLP/HTTP envelope at
//! the top level, counts the spans, and returns success. Persistent
//! storage, query API, and a UI surface are tracked as follow-ups; see the
//! "Remaining" section on issue #233.
//!
//! The scaffold lets us:
//!   1. Confirm spans actually arrive from a deployed hosted mock (operators
//!      can watch the request_count metric or read the warn-log).
//!   2. Defer the storage decision (ClickHouse / Tempo / per-tenant index)
//!      without blocking the rest of Phase 6.
//!
//! Wire format: OTLP/HTTP with JSON encoding (the Protobuf path can be
//! added later — opentelemetry-otlp's HTTP exporter supports both).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    AppState,
};

/// OTLP/HTTP-JSON top-level envelope. We only inspect the resource_spans
/// array length — full parsing waits on a storage decision.
#[derive(Debug, Deserialize)]
pub struct OtlpExportTraceServiceRequest {
    #[serde(default, alias = "resource_spans", alias = "resourceSpans")]
    pub resource_spans: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct OtlpExportTraceServiceResponse {
    pub spans_received: usize,
    pub stored: bool,
    pub message: &'static str,
}

/// Receive an OTLP trace export from a deployed hosted mock. Authenticates
/// with the same deployment-scoped JWT used by the log-ingest endpoint.
pub async fn ingest_traces(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<OtlpExportTraceServiceRequest>,
) -> ApiResult<Json<OtlpExportTraceServiceResponse>> {
    let auth = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::InvalidRequest("Missing deployment ingest token".to_string()))?;

    let token_deployment_id = mockforge_registry_core::auth::verify_deployment_ingest_token(
        auth,
        &state.config.jwt_secret,
    )
    .map_err(|e| {
        tracing::warn!(error = %e, "OTLP ingest token rejected");
        ApiError::InvalidRequest("Invalid deployment ingest token".to_string())
    })?;

    if token_deployment_id != deployment_id {
        return Err(ApiError::InvalidRequest(
            "Token deployment id does not match URL path".to_string(),
        ));
    }

    // Count the spans for observability and let the operator know storage
    // isn't connected yet. Real implementation: persist resource_spans to
    // ClickHouse / Tempo / Postgres-with-JSONB and surface in the admin UI.
    let span_groups = payload.resource_spans.len();
    let total_spans: usize = payload
        .resource_spans
        .iter()
        .map(|rs| {
            rs.get("scope_spans")
                .or_else(|| rs.get("scopeSpans"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|ss| {
                            ss.get("spans")
                                .and_then(|v| v.as_array())
                                .map(|spans| spans.len())
                                .unwrap_or(0)
                        })
                        .sum::<usize>()
                })
                .unwrap_or(0)
        })
        .sum();

    tracing::info!(
        deployment_id = %deployment_id,
        resource_span_groups = span_groups,
        total_spans = total_spans,
        "OTLP traces received (storage stub — see #233)"
    );

    Ok(Json(OtlpExportTraceServiceResponse {
        spans_received: total_spans,
        stored: false,
        message: "OTLP receiver scaffold: spans counted but not persisted; \
                  storage backend is a follow-up on #233",
    }))
}

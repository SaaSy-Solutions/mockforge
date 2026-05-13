//! OTLP HTTP receiver + read API for hosted-mock deployments.
//!
//! Hosted-mock deployments configured with `MOCKFORGE_OTLP_ENDPOINT` ship
//! spans to this endpoint. PR #236 had the receiver as a counter-only
//! scaffold; this module persists them into Postgres and adds list/detail
//! reads so the admin UI has something to render.
//!
//! ## Wire format
//!
//! OTLP/HTTP with JSON encoding. The protobuf path is a follow-up — most
//! exporters can be configured to use JSON, and operators who need
//! protobuf can stand up an `otel-collector` sidecar that converts.
//!
//! ## Storage
//!
//! Postgres-JSONB. See `migrations/20250101000052_runtime_traces.sql`
//! for the rationale and schema. One row per span; attributes/events/
//! links kept as JSONB so we stay forward-compatible without migrations.
//!
//! ## Auth
//!
//! Same deployment-scoped JWT as the log ingest endpoint. The handler
//! verifies the token's subject matches the URL path's deployment id
//! and rejects mismatches.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::HostedMock,
    AppState,
};

/// OTLP/HTTP-JSON top-level envelope. The `data` accessors below pull out
/// the bits we care about; everything else stays in the JSON tree until
/// the storage layer flattens it into rows.
#[derive(Debug, Deserialize)]
pub struct OtlpExportTraceServiceRequest {
    #[serde(default, alias = "resource_spans", alias = "resourceSpans")]
    pub resource_spans: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct OtlpExportTraceServiceResponse {
    pub spans_received: usize,
    pub spans_stored: usize,
    pub message: &'static str,
}

/// Receive an OTLP trace export from a deployed hosted mock and persist
/// each span into `runtime_traces`. Rejects when the deployment-scoped
/// token doesn't match the URL path.
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

    // Flatten the OTLP envelope into individual span rows. We carry the
    // resource attributes through onto each span so a query like "spans
    // where service.name = 'foo'" can be answered without a join.
    let rows = flatten_resource_spans(&payload.resource_spans);
    let total_spans = rows.len();

    if total_spans == 0 {
        return Ok(Json(OtlpExportTraceServiceResponse {
            spans_received: 0,
            spans_stored: 0,
            message: "no spans found in payload",
        }));
    }

    // Cap accepted batch size as a defense against runaway exporters.
    // Match the in-container shipper's guardrail conceptually — one batch
    // shouldn't be able to flood the table.
    const MAX_BATCH: usize = 1000;
    let rows = if rows.len() > MAX_BATCH {
        tracing::warn!(
            deployment_id = %deployment_id,
            received = total_spans,
            kept = MAX_BATCH,
            "OTLP batch exceeded MAX_BATCH; truncating"
        );
        &rows[..MAX_BATCH]
    } else {
        &rows[..]
    };

    let pool = state.db.pool();
    let mut tx = pool.begin().await.map_err(ApiError::Database)?;
    let mut stored = 0usize;
    for row in rows {
        let result = sqlx::query(
            r#"
            INSERT INTO runtime_traces (
                deployment_id, trace_id, span_id, parent_span_id,
                service_name, name, kind,
                start_unix_nano, end_unix_nano, occurred_at,
                status_code, status_message,
                attributes, events, links, resource_attributes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
        )
        .bind(deployment_id)
        .bind(&row.trace_id)
        .bind(&row.span_id)
        .bind(row.parent_span_id.as_ref())
        .bind(row.service_name.as_ref())
        .bind(&row.name)
        .bind(row.kind)
        .bind(row.start_unix_nano)
        .bind(row.end_unix_nano)
        .bind(row.occurred_at)
        .bind(row.status_code)
        .bind(row.status_message.as_ref())
        .bind(&row.attributes)
        .bind(&row.events)
        .bind(&row.links)
        .bind(&row.resource_attributes)
        .execute(&mut *tx)
        .await;
        match result {
            Ok(_) => stored += 1,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to insert OTLP span; skipping");
            }
        }
    }
    tx.commit().await.map_err(ApiError::Database)?;

    tracing::info!(
        deployment_id = %deployment_id,
        received = total_spans,
        stored,
        "OTLP traces ingested"
    );

    Ok(Json(OtlpExportTraceServiceResponse {
        spans_received: total_spans,
        spans_stored: stored,
        message: "ok",
    }))
}

/// Internal representation of a flattened OTLP span ready for INSERT.
struct SpanRow {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    service_name: Option<String>,
    name: String,
    kind: Option<i16>,
    start_unix_nano: i64,
    end_unix_nano: i64,
    occurred_at: DateTime<Utc>,
    status_code: Option<i16>,
    status_message: Option<String>,
    attributes: serde_json::Value,
    events: serde_json::Value,
    links: serde_json::Value,
    resource_attributes: serde_json::Value,
}

/// Walk the OTLP envelope and flatten it into per-span rows. We're
/// permissive about field names because OTLP/JSON has both snake_case
/// and camelCase variants in the wild.
fn flatten_resource_spans(resource_spans: &[serde_json::Value]) -> Vec<SpanRow> {
    let mut rows = Vec::new();
    for rs in resource_spans {
        let resource = rs.get("resource").cloned().unwrap_or(serde_json::Value::Null);
        let resource_attributes = attributes_to_json(&resource);
        let service_name = lookup_attr_string(&resource, "service.name");

        let scope_spans = rs
            .get("scope_spans")
            .or_else(|| rs.get("scopeSpans"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for ss in &scope_spans {
            let spans = ss.get("spans").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            for span in spans {
                if let Some(row) =
                    parse_span(&span, service_name.clone(), resource_attributes.clone())
                {
                    rows.push(row);
                }
            }
        }
    }
    rows
}

fn parse_span(
    span: &serde_json::Value,
    service_name: Option<String>,
    resource_attributes: serde_json::Value,
) -> Option<SpanRow> {
    let trace_id = span
        .get("trace_id")
        .or_else(|| span.get("traceId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;
    let span_id = span
        .get("span_id")
        .or_else(|| span.get("spanId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;
    let parent_span_id = span
        .get("parent_span_id")
        .or_else(|| span.get("parentSpanId"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let name = span.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let kind = span.get("kind").and_then(|v| v.as_i64()).map(|n| n as i16);

    let start_unix_nano = unix_nano(span, &["start_time_unix_nano", "startTimeUnixNano"])?;
    let end_unix_nano =
        unix_nano(span, &["end_time_unix_nano", "endTimeUnixNano"]).unwrap_or(start_unix_nano);

    // Convert nanos → DateTime<Utc> for the denormalized occurred_at.
    let occurred_at = DateTime::<Utc>::from_timestamp_nanos(start_unix_nano);

    let (status_code, status_message) = parse_status(span);

    let attributes = attributes_to_json(span);
    let events = span.get("events").cloned().unwrap_or_else(|| serde_json::json!([]));
    let links = span.get("links").cloned().unwrap_or_else(|| serde_json::json!([]));

    Some(SpanRow {
        trace_id,
        span_id,
        parent_span_id,
        service_name,
        name,
        kind,
        start_unix_nano,
        end_unix_nano,
        occurred_at,
        status_code,
        status_message,
        attributes,
        events,
        links,
        resource_attributes,
    })
}

/// OTLP encodes timestamps as Unix nanos, but in JSON they're serialized
/// as either string ("1740000000000000000") or number depending on the
/// exporter. Handle both.
fn unix_nano(value: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(n) = v.as_i64() {
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Span status: { code: 1, message: "..." } in OTLP. Both fields are
/// optional; default to UNSET when missing.
fn parse_status(span: &serde_json::Value) -> (Option<i16>, Option<String>) {
    let status = match span.get("status") {
        Some(s) => s,
        None => return (None, None),
    };
    let code = status.get("code").and_then(|v| v.as_i64()).map(|n| n as i16);
    let message = status
        .get("message")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    (code, message)
}

/// Flatten an OTLP attributes array into a flat JSON object.
/// OTLP wire shape: `attributes: [{ "key": "k", "value": { "stringValue": "v" } }]`.
/// We unwrap each value to its primitive so queries can use `attributes->>'http.method'`.
fn attributes_to_json(value: &serde_json::Value) -> serde_json::Value {
    let array = value.get("attributes").and_then(|v| v.as_array());
    let Some(array) = array else {
        return serde_json::json!({});
    };
    let mut map = serde_json::Map::new();
    for entry in array {
        let key = entry.get("key").and_then(|v| v.as_str()).unwrap_or("");
        if key.is_empty() {
            continue;
        }
        let raw_value = entry.get("value").cloned().unwrap_or(serde_json::Value::Null);
        let unwrapped = unwrap_otel_value(&raw_value);
        map.insert(key.to_string(), unwrapped);
    }
    serde_json::Value::Object(map)
}

/// OTLP wraps every attribute value in a tagged-union JSON object:
///   { "stringValue": "..." } | { "intValue": 42 } | { "doubleValue": 1.0 }
///   | { "boolValue": true } | { "arrayValue": { "values": [...] } }
/// We unwrap to a plain JSON value so the column is ergonomic to query.
fn unwrap_otel_value(value: &serde_json::Value) -> serde_json::Value {
    if let Some(s) = value.get("stringValue").or_else(|| value.get("string_value")) {
        return s.clone();
    }
    if let Some(n) = value.get("intValue").or_else(|| value.get("int_value")) {
        // OTLP encodes int64 as string in JSON. Re-parse and store as number.
        if let Some(s) = n.as_str() {
            if let Ok(parsed) = s.parse::<i64>() {
                return serde_json::Value::Number(parsed.into());
            }
        }
        return n.clone();
    }
    if let Some(n) = value.get("doubleValue").or_else(|| value.get("double_value")) {
        return n.clone();
    }
    if let Some(b) = value.get("boolValue").or_else(|| value.get("bool_value")) {
        return b.clone();
    }
    if let Some(arr) = value.get("arrayValue").or_else(|| value.get("array_value")) {
        if let Some(values) = arr.get("values").and_then(|v| v.as_array()) {
            let unwrapped: Vec<serde_json::Value> = values.iter().map(unwrap_otel_value).collect();
            return serde_json::Value::Array(unwrapped);
        }
    }
    value.clone()
}

/// Pull a single string attribute out of a `{ attributes: [...] }` value.
/// Used to extract `service.name` from the resource block onto each row.
fn lookup_attr_string(value: &serde_json::Value, key: &str) -> Option<String> {
    let array = value.get("attributes")?.as_array()?;
    for entry in array {
        if entry.get("key").and_then(|v| v.as_str()) == Some(key) {
            let raw = entry.get("value")?;
            if let Some(s) = raw.get("stringValue").or_else(|| raw.get("string_value")) {
                return s.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Trace summary for the list view — one row per distinct trace_id with
/// aggregate metrics that let the admin UI render a useful list without
/// fetching every span.
#[derive(Debug, Serialize)]
pub struct TraceSummary {
    pub trace_id: String,
    pub span_count: i64,
    pub start: DateTime<Utc>,
    pub duration_ms: f64,
    /// Name of the root span (parent_span_id IS NULL). Falls back to
    /// "(unknown)" when the receiver hasn't observed the root yet.
    pub root_name: String,
    pub service_name: Option<String>,
    /// True if any span in the trace had status_code = 2 (ERROR).
    pub has_error: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListTracesQuery {
    pub limit: Option<i64>,
    pub since: Option<String>,
}

/// Recent traces for a deployment, newest first. Powers the admin UI's
/// (future) Traces tab. Today the same data can be inspected via this
/// endpoint directly.
pub async fn list_traces(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<ListTracesQuery>,
) -> ApiResult<Json<Vec<TraceSummary>>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let since = params
        .since
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));

    // The aggregate query: per trace_id, count spans, find the min start
    // time, the duration (max end - min start) and whether any span has
    // an ERROR status. Root-span name is pulled with a correlated
    // subquery — there's exactly one parent_span_id IS NULL span per
    // well-formed trace.
    type Row = (String, i64, DateTime<Utc>, f64, Option<String>, Option<String>, bool);

    let rows: Vec<Row> = if let Some(since) = since {
        sqlx::query_as(
            r#"
            SELECT
                t.trace_id,
                COUNT(*)::bigint AS span_count,
                MIN(t.occurred_at) AS start,
                (MAX(t.end_unix_nano) - MIN(t.start_unix_nano))::float8 / 1.0e6 AS duration_ms,
                (
                    SELECT name FROM runtime_traces
                    WHERE deployment_id = t.deployment_id
                      AND trace_id = t.trace_id
                      AND parent_span_id IS NULL
                    LIMIT 1
                ) AS root_name,
                (
                    SELECT service_name FROM runtime_traces
                    WHERE deployment_id = t.deployment_id
                      AND trace_id = t.trace_id
                      AND parent_span_id IS NULL
                    LIMIT 1
                ) AS service_name,
                BOOL_OR(t.status_code = 2) AS has_error
            FROM runtime_traces t
            WHERE t.deployment_id = $1 AND t.occurred_at > $2
            GROUP BY t.deployment_id, t.trace_id
            ORDER BY MIN(t.occurred_at) DESC
            LIMIT $3
            "#,
        )
        .bind(deployment_id)
        .bind(since)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?
    } else {
        sqlx::query_as(
            r#"
            SELECT
                t.trace_id,
                COUNT(*)::bigint AS span_count,
                MIN(t.occurred_at) AS start,
                (MAX(t.end_unix_nano) - MIN(t.start_unix_nano))::float8 / 1.0e6 AS duration_ms,
                (
                    SELECT name FROM runtime_traces
                    WHERE deployment_id = t.deployment_id
                      AND trace_id = t.trace_id
                      AND parent_span_id IS NULL
                    LIMIT 1
                ) AS root_name,
                (
                    SELECT service_name FROM runtime_traces
                    WHERE deployment_id = t.deployment_id
                      AND trace_id = t.trace_id
                      AND parent_span_id IS NULL
                    LIMIT 1
                ) AS service_name,
                BOOL_OR(t.status_code = 2) AS has_error
            FROM runtime_traces t
            WHERE t.deployment_id = $1
            GROUP BY t.deployment_id, t.trace_id
            ORDER BY MIN(t.occurred_at) DESC
            LIMIT $2
            "#,
        )
        .bind(deployment_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?
    };

    let summaries = rows
        .into_iter()
        .map(
            |(trace_id, span_count, start, duration_ms, root_name, service_name, has_error)| {
                TraceSummary {
                    trace_id,
                    span_count,
                    start,
                    duration_ms,
                    root_name: root_name.unwrap_or_else(|| "(unknown)".to_string()),
                    service_name,
                    has_error,
                }
            },
        )
        .collect();

    Ok(Json(summaries))
}

/// Single span as returned by the trace-detail endpoint. Mirrors the
/// stored row shape; attributes/events/links stay as JSONB.
#[derive(Debug, Serialize)]
pub struct SpanResponse {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: Option<String>,
    pub name: String,
    pub kind: Option<i16>,
    pub start_unix_nano: i64,
    pub end_unix_nano: i64,
    pub status_code: Option<i16>,
    pub status_message: Option<String>,
    pub attributes: serde_json::Value,
    pub events: serde_json::Value,
    pub links: serde_json::Value,
}

/// All spans of a single trace, sorted by start time. The admin UI can
/// build a waterfall directly from this — every span has its parent_id,
/// so client-side tree assembly is trivial.
pub async fn get_trace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((deployment_id, trace_id)): Path<(Uuid, String)>,
) -> ApiResult<Json<Vec<SpanResponse>>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    type Row = (
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<i16>,
        i64,
        i64,
        Option<i16>,
        Option<String>,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
    );

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            trace_id, span_id, parent_span_id, service_name, name, kind,
            start_unix_nano, end_unix_nano, status_code, status_message,
            attributes, events, links
        FROM runtime_traces
        WHERE deployment_id = $1 AND trace_id = $2
        ORDER BY start_unix_nano ASC
        "#,
    )
    .bind(deployment_id)
    .bind(&trace_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;

    let spans = rows
        .into_iter()
        .map(|row| SpanResponse {
            trace_id: row.0,
            span_id: row.1,
            parent_span_id: row.2,
            service_name: row.3,
            name: row.4,
            kind: row.5,
            start_unix_nano: row.6,
            end_unix_nano: row.7,
            status_code: row.8,
            status_message: row.9,
            attributes: row.10,
            events: row.11,
            links: row.12,
        })
        .collect();

    Ok(Json(spans))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_string_attribute() {
        let raw = serde_json::json!({
            "attributes": [
                { "key": "http.method", "value": { "stringValue": "GET" } },
                { "key": "http.status_code", "value": { "intValue": "200" } },
                { "key": "http.url", "value": { "string_value": "/users" } }
            ]
        });
        let flat = attributes_to_json(&raw);
        assert_eq!(flat["http.method"], serde_json::json!("GET"));
        assert_eq!(flat["http.status_code"], serde_json::json!(200));
        assert_eq!(flat["http.url"], serde_json::json!("/users"));
    }

    #[test]
    fn unix_nano_accepts_string_or_number() {
        let raw_string = serde_json::json!({ "start_time_unix_nano": "1740000000000000000" });
        assert_eq!(unix_nano(&raw_string, &["start_time_unix_nano"]), Some(1740000000000000000));
        let raw_number = serde_json::json!({ "startTimeUnixNano": 1740000000000000000_i64 });
        assert_eq!(unix_nano(&raw_number, &["startTimeUnixNano"]), Some(1740000000000000000));
    }

    #[test]
    fn parses_array_attribute() {
        let raw = serde_json::json!({
            "attributes": [{
                "key": "http.headers",
                "value": {
                    "arrayValue": {
                        "values": [
                            { "stringValue": "Accept: application/json" },
                            { "stringValue": "User-Agent: curl/8.0" }
                        ]
                    }
                }
            }]
        });
        let flat = attributes_to_json(&raw);
        let arr = flat["http.headers"].as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], serde_json::json!("Accept: application/json"));
    }

    #[test]
    fn flattens_resource_spans_to_per_span_rows() {
        let payload = serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [{ "key": "service.name", "value": { "stringValue": "checkout" } }]
                },
                "scopeSpans": [{
                    "spans": [{
                        "traceId": "abc123",
                        "spanId": "def456",
                        "name": "GET /cart",
                        "kind": 2,
                        "startTimeUnixNano": "1740000000000000000",
                        "endTimeUnixNano": "1740000000005000000",
                        "status": { "code": 1 },
                        "attributes": [
                            { "key": "http.method", "value": { "stringValue": "GET" } }
                        ]
                    }]
                }]
            }]
        });
        let resource_spans = payload["resourceSpans"].as_array().unwrap().clone();
        let rows = flatten_resource_spans(&resource_spans);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].trace_id, "abc123");
        assert_eq!(rows[0].span_id, "def456");
        assert_eq!(rows[0].name, "GET /cart");
        assert_eq!(rows[0].service_name.as_deref(), Some("checkout"));
        assert_eq!(rows[0].status_code, Some(1));
        assert_eq!(rows[0].attributes["http.method"], serde_json::json!("GET"));
        assert_eq!(rows[0].end_unix_nano - rows[0].start_unix_nano, 5_000_000);
    }
}

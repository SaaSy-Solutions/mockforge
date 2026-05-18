//! OTLP gRPC trace receiver (#548).
//!
//! Implements the `opentelemetry.proto.collector.trace.v1.TraceService`
//! gRPC service on a separate listener (canonical OTLP gRPC port 4317).
//! Customers can point `opentelemetry-otlp` with the `GrpcTonic` exporter
//! directly at the registry without standing up an `otel-collector`
//! sidecar — that workaround was the deferred path documented in #233.
//!
//! ## Auth
//!
//! Same deployment-scoped JWT contract as the HTTP/JSON receiver in
//! `crate::handlers::otlp`. The token's subject (`deployment:<uuid>`)
//! identifies the deployment that owns the spans, so the deployment id
//! comes from the token rather than a URL path. Bad/missing creds map
//! to `Status::unauthenticated` (gRPC status code 16) with no rows
//! persisted.
//!
//! ## Persistence
//!
//! Spans are flattened into `SpanRow`s and handed to
//! `crate::handlers::otlp::persist_span_rows`, the same helper the
//! HTTP/JSON path uses. Storage shape and batch cap stay identical
//! across both wire formats.

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTracePartialSuccess, ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use opentelemetry_proto::tonic::common::v1::{any_value::Value as AnyVal, AnyValue, KeyValue};
use opentelemetry_proto::tonic::resource::v1::Resource;
use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, Span, Status as SpanStatus};
use tokio::task::JoinHandle;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

use crate::handlers::otlp::{persist_span_rows, SpanRow, MAX_BATCH};
use crate::AppState;

/// gRPC `TraceService` implementation. Holds a clone of `AppState` so
/// it can reach the database pool and the JWT secret.
pub struct OtlpTraceService {
    state: AppState,
}

impl OtlpTraceService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TraceService for OtlpTraceService {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        // Authenticate from gRPC metadata BEFORE touching the payload —
        // a bad token must result in no rows persisted, matching the
        // HTTP path's contract.
        let deployment_id =
            extract_deployment_id(request.metadata(), &self.state.config.jwt_secret).map_err(
                |reason| {
                    tracing::warn!(reason = %reason, "OTLP gRPC ingest token rejected");
                    Status::unauthenticated(reason)
                },
            )?;

        let req = request.into_inner();
        let rows = flatten_resource_spans_proto(&req.resource_spans);
        let received = rows.len();

        let pool = self.state.db.pool();
        let (_, stored) = persist_span_rows(pool, deployment_id, rows).await.map_err(|e| {
            tracing::error!(error = ?e, "OTLP gRPC persistence failed");
            Status::internal("failed to persist spans")
        })?;

        // Honor the partial_success contract: report any spans we dropped
        // (either because they failed to INSERT, or because the batch
        // exceeded MAX_BATCH and got truncated upstream of stored).
        let rejected = received.saturating_sub(stored) as i64;
        let response = ExportTraceServiceResponse {
            partial_success: if rejected > 0 {
                Some(ExportTracePartialSuccess {
                    rejected_spans: rejected,
                    error_message: if received > MAX_BATCH {
                        format!("batch truncated to {} spans", MAX_BATCH)
                    } else {
                        "some spans failed to persist".to_string()
                    },
                })
            } else {
                None
            },
        };

        Ok(Response::new(response))
    }
}

/// Pull the deployment-scoped JWT out of gRPC metadata and verify it.
/// Returns the deployment UUID from the token's subject claim.
///
/// Errors carry the reason as a string so the caller can attach it to
/// the `Status::unauthenticated` it returns to the client.
fn extract_deployment_id(
    metadata: &tonic::metadata::MetadataMap,
    jwt_secret: &str,
) -> Result<Uuid, String> {
    let auth = metadata
        .get("authorization")
        .ok_or_else(|| "Missing authorization metadata".to_string())?
        .to_str()
        .map_err(|_| "authorization metadata is not ASCII".to_string())?;

    let token = auth
        .strip_prefix("Bearer ")
        .ok_or_else(|| "authorization metadata is not a Bearer token".to_string())?;

    mockforge_registry_core::auth::verify_deployment_ingest_token(token, jwt_secret)
        .map_err(|e| format!("Invalid deployment ingest token: {e}"))
}

/// Bind the OTLP gRPC service on `addr` and spawn it. The returned
/// `JoinHandle` is fire-and-forget for the registry binary; if the
/// listener errors, we log and let the task end — main HTTP continues
/// serving so a misconfigured OTLP port doesn't take the whole process
/// down.
pub fn spawn_otlp_grpc_server(state: AppState, addr: SocketAddr) -> JoinHandle<()> {
    tokio::spawn(async move {
        let service = TraceServiceServer::new(OtlpTraceService::new(state));
        tracing::info!("OTLP gRPC trace receiver listening on {}", addr);
        let mut builder = Server::builder();
        if let Err(e) = builder.add_service(service).serve(addr).await {
            tracing::error!(error = %e, "OTLP gRPC server exited with error");
        }
    })
}

/// Flatten OTLP/proto `ResourceSpans` into per-span rows ready for
/// `persist_span_rows`. Mirrors the JSON-flattener in
/// `crate::handlers::otlp::flatten_resource_spans` so both wire formats
/// produce byte-equivalent rows for the same logical span.
fn flatten_resource_spans_proto(resource_spans: &[ResourceSpans]) -> Vec<SpanRow> {
    let mut rows = Vec::new();
    for rs in resource_spans {
        let resource_attributes = resource_attributes_to_json(rs.resource.as_ref());
        let service_name =
            rs.resource.as_ref().and_then(|r| lookup_resource_string(r, "service.name"));

        for ss in &rs.scope_spans {
            for span in &ss.spans {
                if let Some(row) =
                    parse_span_proto(span, service_name.clone(), resource_attributes.clone())
                {
                    rows.push(row);
                }
            }
        }
    }
    rows
}

fn parse_span_proto(
    span: &Span,
    service_name: Option<String>,
    resource_attributes: serde_json::Value,
) -> Option<SpanRow> {
    // OTLP/gRPC requires non-empty trace_id / span_id; skip malformed
    // spans rather than letting them blow up the INSERT.
    if span.trace_id.is_empty() || span.span_id.is_empty() {
        return None;
    }

    let trace_id = hex_encode(&span.trace_id);
    let span_id = hex_encode(&span.span_id);
    let parent_span_id = if span.parent_span_id.is_empty() {
        None
    } else {
        Some(hex_encode(&span.parent_span_id))
    };

    let start_unix_nano = u64_to_i64(span.start_time_unix_nano);
    let end_unix_nano = if span.end_time_unix_nano == 0 {
        start_unix_nano
    } else {
        u64_to_i64(span.end_time_unix_nano)
    };
    let occurred_at = DateTime::<Utc>::from_timestamp_nanos(start_unix_nano);

    // SpanKind in proto is an enum with i32 representation. Cast to i16
    // for the existing column type; values are 0..=5 so the cast is safe.
    let kind = if span.kind == 0 {
        None
    } else {
        Some(span.kind as i16)
    };

    let (status_code, status_message) = parse_status_proto(span.status.as_ref());

    Some(SpanRow {
        trace_id,
        span_id,
        parent_span_id,
        service_name,
        name: span.name.clone(),
        kind,
        start_unix_nano,
        end_unix_nano,
        occurred_at,
        status_code,
        status_message,
        attributes: key_values_to_json(&span.attributes),
        events: span_events_to_json(span),
        links: span_links_to_json(span),
        resource_attributes,
    })
}

fn parse_status_proto(status: Option<&SpanStatus>) -> (Option<i16>, Option<String>) {
    match status {
        Some(s) => {
            let code = if s.code == 0 {
                None
            } else {
                Some(s.code as i16)
            };
            let message = if s.message.is_empty() {
                None
            } else {
                Some(s.message.clone())
            };
            (code, message)
        }
        None => (None, None),
    }
}

fn resource_attributes_to_json(resource: Option<&Resource>) -> serde_json::Value {
    match resource {
        Some(r) => key_values_to_json(&r.attributes),
        None => serde_json::json!({}),
    }
}

fn lookup_resource_string(resource: &Resource, key: &str) -> Option<String> {
    for kv in &resource.attributes {
        if kv.key == key {
            if let Some(AnyValue {
                value: Some(AnyVal::StringValue(s)),
            }) = kv.value.as_ref()
            {
                return Some(s.clone());
            }
        }
    }
    None
}

fn key_values_to_json(attrs: &[KeyValue]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for kv in attrs {
        if kv.key.is_empty() {
            continue;
        }
        map.insert(kv.key.clone(), any_value_to_json(kv.value.as_ref()));
    }
    serde_json::Value::Object(map)
}

/// Convert OTLP/proto `AnyValue` to a plain JSON value matching the
/// unwrapping shape produced by `crate::handlers::otlp::unwrap_otel_value`.
fn any_value_to_json(value: Option<&AnyValue>) -> serde_json::Value {
    let Some(av) = value else {
        return serde_json::Value::Null;
    };
    match &av.value {
        Some(AnyVal::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(AnyVal::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(AnyVal::IntValue(i)) => serde_json::Value::Number((*i).into()),
        Some(AnyVal::DoubleValue(f)) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(AnyVal::ArrayValue(arr)) => {
            let items: Vec<_> = arr.values.iter().map(|v| any_value_to_json(Some(v))).collect();
            serde_json::Value::Array(items)
        }
        Some(AnyVal::KvlistValue(kv)) => key_values_to_json(&kv.values),
        Some(AnyVal::BytesValue(bytes)) => serde_json::Value::String(hex_encode(bytes)),
        Some(AnyVal::StringValueStrindex(_)) | None => serde_json::Value::Null,
    }
}

fn span_events_to_json(span: &Span) -> serde_json::Value {
    let events: Vec<_> = span
        .events
        .iter()
        .map(|e| {
            serde_json::json!({
                "timeUnixNano": e.time_unix_nano.to_string(),
                "name": e.name,
                "attributes": key_values_to_json(&e.attributes),
            })
        })
        .collect();
    serde_json::Value::Array(events)
}

fn span_links_to_json(span: &Span) -> serde_json::Value {
    let links: Vec<_> = span
        .links
        .iter()
        .map(|l| {
            serde_json::json!({
                "traceId": hex_encode(&l.trace_id),
                "spanId": hex_encode(&l.span_id),
                "traceState": l.trace_state,
                "attributes": key_values_to_json(&l.attributes),
            })
        })
        .collect();
    serde_json::Value::Array(links)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// OTLP encodes Unix-nano timestamps as `u64`; our schema column is
/// `BIGINT`. Saturate at `i64::MAX` rather than wrap — a saturated value
/// (year 2262) is still a sortable, queryable timestamp; a wrapped one
/// is a negative epoch that breaks every downstream query.
fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_proto::tonic::common::v1::{ArrayValue, KeyValueList};
    use opentelemetry_proto::tonic::trace::v1::{ScopeSpans, Span as ProtoSpan};

    fn string_kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: Some(AnyValue {
                value: Some(AnyVal::StringValue(value.to_string())),
            }),
            ..Default::default()
        }
    }

    fn int_kv(key: &str, value: i64) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: Some(AnyValue {
                value: Some(AnyVal::IntValue(value)),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn flattens_proto_resource_spans_to_rows() {
        let payload = vec![ResourceSpans {
            resource: Some(Resource {
                attributes: vec![string_kv("service.name", "checkout")],
                ..Default::default()
            }),
            scope_spans: vec![ScopeSpans {
                spans: vec![ProtoSpan {
                    trace_id: vec![0xab; 16],
                    span_id: vec![0xcd; 8],
                    name: "GET /cart".to_string(),
                    kind: 2, // SERVER
                    start_time_unix_nano: 1_740_000_000_000_000_000,
                    end_time_unix_nano: 1_740_000_000_005_000_000,
                    status: Some(SpanStatus {
                        code: 1,
                        message: String::new(),
                    }),
                    attributes: vec![
                        string_kv("http.method", "GET"),
                        int_kv("http.status_code", 200),
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }];

        let rows = flatten_resource_spans_proto(&payload);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.trace_id, "abababababababababababababababab");
        assert_eq!(row.span_id, "cdcdcdcdcdcdcdcd");
        assert_eq!(row.name, "GET /cart");
        assert_eq!(row.kind, Some(2));
        assert_eq!(row.service_name.as_deref(), Some("checkout"));
        assert_eq!(row.status_code, Some(1));
        assert_eq!(row.attributes["http.method"], serde_json::json!("GET"));
        assert_eq!(row.attributes["http.status_code"], serde_json::json!(200));
        assert_eq!(row.end_unix_nano - row.start_unix_nano, 5_000_000);
        assert_eq!(row.resource_attributes["service.name"], serde_json::json!("checkout"));
    }

    #[test]
    fn skips_span_with_empty_ids() {
        let span = ProtoSpan {
            trace_id: vec![],
            span_id: vec![0xcd; 8],
            name: "x".to_string(),
            start_time_unix_nano: 1,
            end_time_unix_nano: 2,
            ..Default::default()
        };
        assert!(parse_span_proto(&span, None, serde_json::json!({})).is_none());
    }

    #[test]
    fn unwraps_array_and_kvlist_attributes() {
        let kv = KeyValue {
            key: "tags".to_string(),
            value: Some(AnyValue {
                value: Some(AnyVal::ArrayValue(ArrayValue {
                    values: vec![
                        AnyValue {
                            value: Some(AnyVal::StringValue("a".to_string())),
                        },
                        AnyValue {
                            value: Some(AnyVal::StringValue("b".to_string())),
                        },
                    ],
                })),
            }),
            ..Default::default()
        };
        let kvlist = KeyValue {
            key: "meta".to_string(),
            value: Some(AnyValue {
                value: Some(AnyVal::KvlistValue(KeyValueList {
                    values: vec![string_kv("k", "v")],
                })),
            }),
            ..Default::default()
        };
        let flat = key_values_to_json(&[kv, kvlist]);
        assert_eq!(flat["tags"], serde_json::json!(["a", "b"]));
        assert_eq!(flat["meta"], serde_json::json!({"k": "v"}));
    }

    #[test]
    fn missing_authorization_metadata_rejected() {
        let metadata = tonic::metadata::MetadataMap::new();
        let err = extract_deployment_id(&metadata, "secret").unwrap_err();
        assert!(err.contains("Missing"));
    }

    #[test]
    fn non_bearer_authorization_rejected() {
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert("authorization", "Basic foo".parse().unwrap());
        let err = extract_deployment_id(&metadata, "secret").unwrap_err();
        assert!(err.contains("Bearer"));
    }

    #[test]
    fn invalid_bearer_token_rejected() {
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert("authorization", "Bearer not-a-real-token".parse().unwrap());
        let err = extract_deployment_id(&metadata, "secret").unwrap_err();
        assert!(err.contains("Invalid"));
    }

    #[test]
    fn valid_deployment_token_round_trips() {
        let secret = "test-secret";
        let deployment_id = Uuid::new_v4();
        let token = mockforge_registry_core::auth::create_deployment_ingest_token(
            deployment_id,
            secret,
            7, // ttl_days
        )
        .expect("issue token");
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert("authorization", format!("Bearer {token}").parse().unwrap());
        let recovered = extract_deployment_id(&metadata, secret).expect("verify");
        assert_eq!(recovered, deployment_id);
    }

    #[test]
    fn end_time_zero_falls_back_to_start() {
        let span = ProtoSpan {
            trace_id: vec![1; 16],
            span_id: vec![2; 8],
            name: "x".to_string(),
            start_time_unix_nano: 1_000,
            end_time_unix_nano: 0,
            ..Default::default()
        };
        let row = parse_span_proto(&span, None, serde_json::json!({})).unwrap();
        assert_eq!(row.start_unix_nano, row.end_unix_nano);
    }
}

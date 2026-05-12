//! Dynamic gRPC router for handling arbitrary services discovered from proto files
//!
//! Routes incoming gRPC requests to the appropriate dynamically-discovered service
//! using the service registry and descriptor pool for response generation.

use super::ServiceRegistry;
use http::header::HeaderValue;
use mockforge_core::config::{GrpcOverride, GrpcOverrideResponse};
use prost_reflect::prost::Message as _;
use prost_reflect::{DynamicMessage, MessageDescriptor, Value};
use tonic::{Code, Status};
use tracing::{debug, warn};

/// Maximum recursion depth for generating nested mock messages
const MAX_MOCK_DEPTH: usize = 5;

/// Parse a gRPC path into (service_name, method_name)
///
/// gRPC paths follow the format `/<package.Service>/<Method>`
pub fn parse_grpc_path(path: &str) -> Option<(&str, &str)> {
    let path = path.strip_prefix('/')?;
    let (service, method) = path.split_once('/')?;
    if service.is_empty() || method.is_empty() {
        return None;
    }
    Some((service, method))
}

/// Generate a mock `DynamicMessage` from a message descriptor
///
/// Recursively populates fields with type-appropriate mock data.
pub fn generate_message_from_descriptor(
    descriptor: &MessageDescriptor,
    depth: usize,
) -> DynamicMessage {
    let mut msg = DynamicMessage::new(descriptor.clone());

    if depth >= MAX_MOCK_DEPTH {
        return msg;
    }

    for field in descriptor.fields() {
        if let Some(v) = mock_value_for_field(&field, depth) {
            if field.is_list() {
                msg.set_field(&field, Value::List(vec![v]));
            } else {
                msg.set_field(&field, v);
            }
        }
    }

    msg
}

/// Generate a mock value for a single field descriptor
fn mock_value_for_field(field: &prost_reflect::FieldDescriptor, depth: usize) -> Option<Value> {
    use prost_reflect::Kind;

    let value = match field.kind() {
        Kind::Double => Value::F64(99.99),
        Kind::Float => Value::F32(42.5),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Value::I32(42),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Value::I64(42),
        Kind::Uint32 | Kind::Fixed32 => Value::U32(42),
        Kind::Uint64 | Kind::Fixed64 => Value::U64(42),
        Kind::Bool => Value::Bool(true),
        Kind::String => {
            let name = field.name().to_lowercase();
            let s = match name.as_str() {
                "id" => "mock-id-001".to_string(),
                "name" | "title" => format!("Mock {}", field.name()),
                "email" => "mock@example.com".to_string(),
                "message" => "Hello from MockForge dynamic service".to_string(),
                _ => format!("mock_{}", field.name()),
            };
            Value::String(s)
        }
        Kind::Bytes => Value::Bytes(b"mock_bytes".to_vec().into()),
        Kind::Message(nested_desc) => {
            let nested = generate_message_from_descriptor(&nested_desc, depth + 1);
            Value::Message(nested)
        }
        Kind::Enum(enum_desc) => {
            let val = enum_desc
                .values()
                .find(|v| v.number() != 0)
                .unwrap_or_else(|| enum_desc.default_value());
            Value::EnumNumber(val.number())
        }
    };

    Some(value)
}

/// Decode a gRPC message body by stripping the 5-byte frame header.
///
/// gRPC frame format: \[compressed(1 byte)\]\[length(4 bytes big-endian)\]\[data\]
pub fn decode_grpc_body(body: &[u8]) -> Result<&[u8], Status> {
    if body.len() < 5 {
        return Err(Status::new(
            Code::InvalidArgument,
            "gRPC body too short: missing frame header",
        ));
    }

    let _compressed = body[0];
    let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;

    if body.len() < 5 + length {
        return Err(Status::new(
            Code::InvalidArgument,
            format!("gRPC body truncated: expected {} bytes, got {}", length, body.len() - 5),
        ));
    }

    Ok(&body[5..5 + length])
}

/// Encode a protobuf message with the 5-byte gRPC frame header.
pub fn encode_grpc_body(message: &DynamicMessage) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(message.encoded_len());
    // DynamicMessage::encode writes protobuf wire format into buf
    message
        .encode(&mut encoded)
        .expect("encoding DynamicMessage to Vec should not fail");
    let len = encoded.len() as u32;
    let mut buf = Vec::with_capacity(5 + encoded.len());
    buf.push(0); // not compressed
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&encoded);
    buf
}

/// Create an HTTP response representing a gRPC error
pub fn create_grpc_error_response(status: Status) -> axum::response::Response {
    let code = status.code() as i32;
    let message = status.message().to_string();

    let mut response = axum::response::Response::new(axum::body::Body::empty());
    *response.status_mut() = http::StatusCode::OK; // gRPC always returns HTTP 200
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/grpc"));
    response.headers_mut().insert(
        "grpc-status",
        HeaderValue::from_str(&code.to_string()).unwrap_or(HeaderValue::from_static("2")),
    );
    if !message.is_empty() {
        if let Ok(val) = HeaderValue::from_str(&message) {
            response.headers_mut().insert("grpc-message", val);
        }
    }
    response
}

/// Handle a dynamic gRPC request using the service registry and descriptor pool.
///
/// This is the main entry point called from the axum fallback handler.
pub async fn handle_dynamic_grpc_request(
    registry: &ServiceRegistry,
    service_name: &str,
    method_name: &str,
    _body: axum::body::Bytes,
) -> Result<axum::response::Response, Status> {
    debug!("Dynamic gRPC handler: {}/{}", service_name, method_name);

    // Look up the service in the registry
    let service = registry.get(service_name).ok_or_else(|| {
        warn!("Service not found: {}", service_name);
        Status::unimplemented(format!("Service '{}' not found", service_name))
    })?;

    // Find the method
    let method =
        service
            .service()
            .methods
            .iter()
            .find(|m| m.name == method_name)
            .ok_or_else(|| {
                warn!("Method not found: {}/{}", service_name, method_name);
                Status::unimplemented(format!(
                    "Method '{}/{}' not found",
                    service_name, method_name
                ))
            })?;

    // Determine streaming type and handle
    match (method.client_streaming, method.server_streaming) {
        (false, false) => handle_unary(registry, service_name, method, &_body).await,
        (false, true) => handle_server_streaming(registry, service_name, method).await,
        (true, false) => {
            // Client streaming: aggregate incoming frames, respond with single message
            handle_client_streaming(registry, service_name, method, &_body).await
        }
        (true, true) => {
            // Bidirectional streaming: respond with multiple frames
            handle_bidi_streaming(registry, service_name, method, &_body).await
        }
    }
}

/// Find the first override rule that matches a `service/method` request.
///
/// Match semantics:
/// - The override's `service` must exactly equal `service_name` (callers should
///   pass the same form the proto uses — fully-qualified or not — consistently).
/// - The override's `method` must exactly equal `method_name`.
/// - If the override has a non-empty `match` map, every key must correspond to
///   a top-level field of the decoded request whose stringified value equals
///   the match value. If `request_body` is None or the request can't be
///   decoded against `input_desc`, match conditions are skipped (a catch-all
///   override — empty match — still applies; one with `match` does not).
pub(super) fn find_matching_override<'a>(
    overrides: &'a [GrpcOverride],
    service_name: &str,
    method_name: &str,
    input_desc: Option<&MessageDescriptor>,
    request_body: Option<&[u8]>,
) -> Option<&'a GrpcOverride> {
    for rule in overrides {
        if rule.service != service_name || rule.method != method_name {
            continue;
        }
        if rule.r#match.is_empty() {
            return Some(rule);
        }
        // Need a decoded request to check match conditions.
        let Some(desc) = input_desc else { continue };
        let Some(body) = request_body else { continue };
        let Ok(payload) = decode_grpc_body(body) else {
            continue;
        };
        let Ok(decoded) = DynamicMessage::decode(desc.clone(), payload) else {
            continue;
        };

        let all_match = rule.r#match.iter().all(|(field_name, expected)| {
            let Some(field) = desc.get_field_by_name(field_name) else {
                return false;
            };
            let value = decoded.get_field(&field);
            stringify_value(value.as_ref()) == *expected
        });
        if all_match {
            return Some(rule);
        }
    }
    None
}

/// Stringify a `prost_reflect::Value` for equality matching against the
/// `match` map (which is string-typed in YAML).
fn stringify_value(value: &Value) -> String {
    match value {
        Value::Bool(v) => v.to_string(),
        Value::I32(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::U32(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::F32(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::String(s) => s.clone(),
        Value::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        Value::EnumNumber(n) => n.to_string(),
        // For complex types (Message, List, Map) we don't define equality.
        // Match map rules are intended for primitive field comparisons.
        _ => String::new(),
    }
}

/// Parse a gRPC status code name (case-insensitive) into a `tonic::Code`.
/// Returns `Code::Unknown` for anything we don't recognize.
fn parse_status_code(name: &str) -> Code {
    match name.to_ascii_uppercase().as_str() {
        "OK" => Code::Ok,
        "CANCELLED" => Code::Cancelled,
        "UNKNOWN" => Code::Unknown,
        "INVALID_ARGUMENT" => Code::InvalidArgument,
        "DEADLINE_EXCEEDED" => Code::DeadlineExceeded,
        "NOT_FOUND" => Code::NotFound,
        "ALREADY_EXISTS" => Code::AlreadyExists,
        "PERMISSION_DENIED" => Code::PermissionDenied,
        "RESOURCE_EXHAUSTED" => Code::ResourceExhausted,
        "FAILED_PRECONDITION" => Code::FailedPrecondition,
        "ABORTED" => Code::Aborted,
        "OUT_OF_RANGE" => Code::OutOfRange,
        "UNIMPLEMENTED" => Code::Unimplemented,
        "INTERNAL" => Code::Internal,
        "UNAVAILABLE" => Code::Unavailable,
        "DATA_LOSS" => Code::DataLoss,
        "UNAUTHENTICATED" => Code::Unauthenticated,
        _ => Code::Unknown,
    }
}

/// Build a response from an override rule.
///
/// - If `response.status` is set and non-OK, returns a gRPC error response.
/// - Otherwise, if `response.body` is set, builds a `DynamicMessage` from the
///   JSON object against the output descriptor and returns it.
/// - If neither is usable (no descriptor available, body is invalid JSON),
///   returns Ok(None) so the caller falls back to default generation.
fn apply_override(
    rule: &GrpcOverrideResponse,
    output_desc: Option<&MessageDescriptor>,
) -> Result<Option<axum::response::Response>, Status> {
    // Status code first — non-OK short-circuits regardless of body.
    if let Some(name) = rule.status.as_deref() {
        let code = parse_status_code(name);
        if code != Code::Ok {
            let msg = rule.message.clone().unwrap_or_default();
            return Ok(Some(create_grpc_error_response(Status::new(code, msg))));
        }
    }

    let Some(body) = rule.body.as_ref() else {
        // No status (or OK) and no body — nothing to apply. Caller falls back.
        return Ok(None);
    };
    let Some(desc) = output_desc else {
        warn!("Override has body but output descriptor unavailable; falling back to mock");
        return Ok(None);
    };

    // Use the existing http_bridge converter — it already handles nested
    // messages, repeated fields, enums, well-known types like timestamps,
    // etc. — so we don't re-derive that logic here.
    let converter =
        super::http_bridge::converters::ProtobufJsonConverter::new(desc.parent_pool().clone());
    match converter.json_to_protobuf(desc, body) {
        Ok(msg) => Ok(Some(build_grpc_response(encode_grpc_body(&msg))?)),
        Err(e) => {
            warn!("Override body failed to convert into response message: {}", e);
            Ok(None)
        }
    }
}

/// Handle a unary gRPC call
async fn handle_unary(
    registry: &ServiceRegistry,
    service_name: &str,
    method: &super::proto_parser::ProtoMethod,
    request_body: &[u8],
) -> Result<axum::response::Response, Status> {
    let pool = registry.descriptor_pool();
    let input_desc = pool.get_message_by_name(&method.input_type);
    let output_desc = pool.get_message_by_name(&method.output_type);

    // Try configured overrides before falling back to default mock generation.
    if let Some(rule) = find_matching_override(
        registry.overrides(),
        service_name,
        &method.name,
        input_desc.as_ref(),
        Some(request_body),
    ) {
        debug!(
            "Applying override for {}.{} (rule match keys: {:?})",
            service_name,
            method.name,
            rule.r#match.keys().collect::<Vec<_>>()
        );
        if let Some(resp) = apply_override(&rule.response, output_desc.as_ref())? {
            return Ok(resp);
        }
        // Override didn't actually produce a response (e.g. body wouldn't
        // deserialize) — log already happened, fall through.
    }

    // Default path: descriptor-based response generation.
    let response_bytes = if let Some(output_desc) = output_desc {
        debug!("Generating response from descriptor for {}.{}", service_name, method.name);
        let mock_msg = generate_message_from_descriptor(&output_desc, 0);
        encode_grpc_body(&mock_msg)
    } else {
        // Fall back to the DynamicGrpcService's JSON-based mock
        debug!(
            "Falling back to JSON mock for {}.{} (type '{}' not in descriptor pool)",
            service_name, method.name, method.output_type
        );
        json_fallback_frame(registry, service_name, &method.name)
    };

    build_grpc_response(response_bytes)
}

/// Handle a server-streaming gRPC call by returning multiple frames
async fn handle_server_streaming(
    registry: &ServiceRegistry,
    service_name: &str,
    method: &super::proto_parser::ProtoMethod,
) -> Result<axum::response::Response, Status> {
    let pool = registry.descriptor_pool();
    let stream_count = 3; // Send 3 mock messages

    let mut all_frames = Vec::new();

    if let Some(output_desc) = pool.get_message_by_name(&method.output_type) {
        for _ in 0..stream_count {
            let mock_msg = generate_message_from_descriptor(&output_desc, 0);
            all_frames.extend_from_slice(&encode_grpc_body(&mock_msg));
        }
    } else {
        for _ in 0..stream_count {
            all_frames.extend_from_slice(&json_fallback_frame(
                registry,
                service_name,
                &method.name,
            ));
        }
    }

    build_grpc_response(all_frames)
}

/// Handle a client-streaming gRPC call.
///
/// Aggregates all incoming gRPC frames from the request body, then returns
/// a single response message.
async fn handle_client_streaming(
    registry: &ServiceRegistry,
    service_name: &str,
    method: &super::proto_parser::ProtoMethod,
    body: &[u8],
) -> Result<axum::response::Response, Status> {
    let pool = registry.descriptor_pool();

    // Count frames in the incoming body (for logging)
    let frame_count = count_grpc_frames(body);
    debug!(
        "Client streaming {}/{}: received {} frames",
        service_name, method.name, frame_count
    );

    // Generate a single aggregated response
    let response_bytes = if let Some(output_desc) = pool.get_message_by_name(&method.output_type) {
        let mock_msg = generate_message_from_descriptor(&output_desc, 0);
        encode_grpc_body(&mock_msg)
    } else {
        json_fallback_frame(registry, service_name, &method.name)
    };

    build_grpc_response(response_bytes)
}

/// Handle a bidirectional-streaming gRPC call.
///
/// Reads all incoming frames and returns a stream of response frames
/// (one response per incoming frame, or a minimum of 3).
async fn handle_bidi_streaming(
    registry: &ServiceRegistry,
    service_name: &str,
    method: &super::proto_parser::ProtoMethod,
    body: &[u8],
) -> Result<axum::response::Response, Status> {
    let pool = registry.descriptor_pool();

    // Count incoming frames to determine how many responses to send
    let incoming_count = count_grpc_frames(body);
    let response_count = incoming_count.max(3); // At least 3 responses

    debug!(
        "Bidirectional streaming {}/{}: {} incoming frames, sending {} responses",
        service_name, method.name, incoming_count, response_count
    );

    let mut all_frames = Vec::new();

    if let Some(output_desc) = pool.get_message_by_name(&method.output_type) {
        for _ in 0..response_count {
            let mock_msg = generate_message_from_descriptor(&output_desc, 0);
            all_frames.extend_from_slice(&encode_grpc_body(&mock_msg));
        }
    } else {
        for _ in 0..response_count {
            all_frames.extend_from_slice(&json_fallback_frame(
                registry,
                service_name,
                &method.name,
            ));
        }
    }

    build_grpc_response(all_frames)
}

/// Count the number of complete gRPC frames in a body buffer.
///
/// Each frame has a 5-byte header: \[compressed(1)\]\[length(4)\].
/// Only frames with a complete payload are counted.
fn count_grpc_frames(body: &[u8]) -> usize {
    let mut count = 0;
    let mut offset = 0;
    while offset + 5 <= body.len() {
        let length = u32::from_be_bytes([
            body[offset + 1],
            body[offset + 2],
            body[offset + 3],
            body[offset + 4],
        ]) as usize;
        if offset + 5 + length > body.len() {
            break; // Incomplete frame
        }
        offset += 5 + length;
        count += 1;
    }
    count
}

/// Create a gRPC frame from the JSON-based mock response as fallback
fn json_fallback_frame(
    registry: &ServiceRegistry,
    service_name: &str,
    method_name: &str,
) -> Vec<u8> {
    let json_bytes = if let Some(svc) = registry.get(service_name) {
        svc.get_mock_response(method_name)
            .map(|r| r.response_json.as_bytes().to_vec())
            .unwrap_or_else(|| b"{}".to_vec())
    } else {
        b"{}".to_vec()
    };

    let len = json_bytes.len() as u32;
    let mut buf = Vec::with_capacity(5 + json_bytes.len());
    buf.push(0); // not compressed
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&json_bytes);
    buf
}

/// Build a proper gRPC HTTP/2 response from encoded body bytes
fn build_grpc_response(body: Vec<u8>) -> Result<axum::response::Response, Status> {
    let mut response = axum::response::Response::new(axum::body::Body::from(body));
    *response.status_mut() = http::StatusCode::OK;
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/grpc"));
    response.headers_mut().insert("grpc-status", HeaderValue::from_static("0"));
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn override_rule(
        service: &str,
        method: &str,
        match_fields: &[(&str, &str)],
        status: Option<&str>,
    ) -> GrpcOverride {
        GrpcOverride {
            service: service.to_string(),
            method: method.to_string(),
            r#match: match_fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            response: GrpcOverrideResponse {
                status: status.map(|s| s.to_string()),
                message: None,
                body: None,
            },
        }
    }

    #[test]
    fn test_find_override_catch_all_matches_when_service_method_match() {
        let rules = vec![override_rule(
            "OrderService",
            "GetOrder",
            &[],
            Some("NOT_FOUND"),
        )];
        let m = find_matching_override(&rules, "OrderService", "GetOrder", None, None);
        assert!(m.is_some(), "catch-all override should match");
    }

    #[test]
    fn test_find_override_returns_none_when_service_differs() {
        let rules = vec![override_rule("OrderService", "GetOrder", &[], None)];
        assert!(find_matching_override(&rules, "PaymentService", "GetOrder", None, None).is_none());
        assert!(find_matching_override(&rules, "OrderService", "ListOrders", None, None).is_none());
    }

    #[test]
    fn test_find_override_skips_match_rule_when_request_missing() {
        // Rule has a match block, but caller passed no request body — rule
        // can't be evaluated, so it should NOT fire.
        let rules = vec![override_rule(
            "OrderService",
            "GetOrder",
            &[("order_id", "x")],
            Some("OK"),
        )];
        assert!(find_matching_override(&rules, "OrderService", "GetOrder", None, None).is_none());
    }

    #[test]
    fn test_find_override_first_match_wins() {
        // Multiple catch-all rules for the same method: the first one in declaration
        // order should win, even if a later one would also match.
        let rules = vec![
            override_rule("OrderService", "GetOrder", &[], Some("NOT_FOUND")),
            override_rule("OrderService", "GetOrder", &[], Some("PERMISSION_DENIED")),
        ];
        let m = find_matching_override(&rules, "OrderService", "GetOrder", None, None).unwrap();
        assert_eq!(m.response.status.as_deref(), Some("NOT_FOUND"));
    }

    #[test]
    fn test_parse_status_code_recognizes_standard_names() {
        assert_eq!(parse_status_code("NOT_FOUND"), Code::NotFound);
        assert_eq!(parse_status_code("not_found"), Code::NotFound);
        assert_eq!(parse_status_code("PERMISSION_DENIED"), Code::PermissionDenied);
        assert_eq!(parse_status_code("OK"), Code::Ok);
        // Unknown name maps to Code::Unknown (safe fallback).
        assert_eq!(parse_status_code("totally-made-up"), Code::Unknown);
    }

    #[test]
    fn test_parse_grpc_path() {
        assert_eq!(
            parse_grpc_path("/mypackage.MyService/MyMethod"),
            Some(("mypackage.MyService", "MyMethod"))
        );
        assert_eq!(parse_grpc_path("/Service/Method"), Some(("Service", "Method")));
        assert_eq!(parse_grpc_path("/"), None);
        assert_eq!(parse_grpc_path(""), None);
        assert_eq!(parse_grpc_path("/Service/"), None);
        assert_eq!(parse_grpc_path("//Method"), None);
    }

    #[test]
    fn test_decode_grpc_body() {
        // Valid frame: 0 (not compressed) + 4 bytes length + payload
        let payload = b"hello";
        let mut frame = vec![0u8]; // not compressed
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);

        let result = decode_grpc_body(&frame).unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_decode_grpc_body_too_short() {
        assert!(decode_grpc_body(b"").is_err());
        assert!(decode_grpc_body(b"\x00\x00").is_err());
    }

    #[test]
    fn test_encode_grpc_body_frame_structure() {
        // Use the greeter proto descriptor set compiled by the build script
        let pool = prost_reflect::DescriptorPool::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin")).as_ref(),
        )
        .expect("built-in descriptor set should be valid");

        // Find any message in the pool
        let desc = pool.all_messages().next().expect("pool should have messages");
        let msg = generate_message_from_descriptor(&desc, 0);
        let encoded = encode_grpc_body(&msg);

        // Verify gRPC frame structure: [0:compressed][1-4:length][5+:data]
        assert_eq!(encoded[0], 0, "compression flag should be 0");
        let len = u32::from_be_bytes([encoded[1], encoded[2], encoded[3], encoded[4]]) as usize;
        assert_eq!(encoded.len(), 5 + len, "frame length should match payload");

        // Decode should produce the same payload
        let decoded = decode_grpc_body(&encoded).unwrap();
        assert_eq!(decoded.len(), len);
    }

    #[test]
    fn test_count_grpc_frames() {
        // Empty body = 0 frames
        assert_eq!(count_grpc_frames(b""), 0);

        // Single frame: [0][len=5 as u32][hello]
        let mut single = vec![0u8];
        single.extend_from_slice(&5u32.to_be_bytes());
        single.extend_from_slice(b"hello");
        assert_eq!(count_grpc_frames(&single), 1);

        // Two frames back-to-back
        let mut double = single.clone();
        double.push(0);
        double.extend_from_slice(&3u32.to_be_bytes());
        double.extend_from_slice(b"bye");
        assert_eq!(count_grpc_frames(&double), 2);

        // Incomplete frame (header but truncated body)
        let mut partial = vec![0u8];
        partial.extend_from_slice(&100u32.to_be_bytes());
        partial.extend_from_slice(b"short");
        assert_eq!(count_grpc_frames(&partial), 0);
    }

    #[test]
    fn test_create_grpc_error_response() {
        let status = Status::not_found("Service not found");
        let response = create_grpc_error_response(status);

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/grpc");
        assert_eq!(response.headers().get("grpc-status").unwrap(), "5"); // NOT_FOUND
        assert_eq!(response.headers().get("grpc-message").unwrap(), "Service not found");
    }
}

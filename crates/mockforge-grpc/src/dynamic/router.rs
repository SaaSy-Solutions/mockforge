//! Dynamic gRPC router for handling arbitrary services discovered from proto files
//!
//! Routes incoming gRPC requests to the appropriate dynamically-discovered service
//! using the service registry and descriptor pool for response generation.

use super::ServiceRegistry;
use http::header::HeaderValue;
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
                msg.set_field(&field, prost_reflect::Value::List(vec![v]));
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
        (false, false) => handle_unary(registry, service_name, method).await,
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

/// Handle a unary gRPC call
async fn handle_unary(
    registry: &ServiceRegistry,
    service_name: &str,
    method: &super::proto_parser::ProtoMethod,
) -> Result<axum::response::Response, Status> {
    let pool = registry.descriptor_pool();

    // Try descriptor-based response generation
    let response_bytes = if let Some(output_desc) = pool.get_message_by_name(&method.output_type) {
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

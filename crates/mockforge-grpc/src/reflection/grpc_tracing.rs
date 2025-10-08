//! Distributed tracing for gRPC reflection proxy

use mockforge_tracing::{create_request_span, record_error, record_success, Protocol};
use opentelemetry::{global::BoxedSpan, trace::Span, KeyValue};
use std::collections::HashMap;
use tracing::debug;

/// Create a span for a gRPC method invocation
pub fn create_grpc_span(service_name: &str, method_name: &str) -> BoxedSpan {
    let method_full = format!("{}::{}", service_name, method_name);

    create_request_span(
        Protocol::Grpc,
        &method_full,
        vec![
            KeyValue::new("rpc.system", "grpc"),
            KeyValue::new("rpc.service", service_name.to_string()),
            KeyValue::new("rpc.method", method_name.to_string()),
        ],
    )
}

/// Record successful gRPC method completion
pub fn record_grpc_success(
    span: &mut BoxedSpan,
    duration_ms: u64,
    request_size: Option<usize>,
    response_size: Option<usize>,
) {
    let mut attributes = vec![
        KeyValue::new("rpc.grpc.status_code", 0i64), // OK
        KeyValue::new("rpc.duration_ms", duration_ms as i64),
    ];

    if let Some(size) = request_size {
        attributes.push(KeyValue::new("rpc.request.size", size as i64));
    }

    if let Some(size) = response_size {
        attributes.push(KeyValue::new("rpc.response.size", size as i64));
    }

    record_success(span, attributes);

    debug!(
        duration_ms = duration_ms,
        "gRPC method completed successfully"
    );
}

/// Record gRPC method error
pub fn record_grpc_error(
    span: &mut BoxedSpan,
    error_code: i32,
    error_message: &str,
    duration_ms: u64,
) {
    span.set_attribute(KeyValue::new("rpc.grpc.status_code", error_code as i64));
    span.set_attribute(KeyValue::new("rpc.duration_ms", duration_ms as i64));

    record_error(span, error_message);

    debug!(
        error_code = error_code,
        error_message = error_message,
        duration_ms = duration_ms,
        "gRPC method failed"
    );
}

/// Extract trace context from gRPC metadata
pub fn extract_grpc_trace_context(
    metadata: &HashMap<String, String>,
) -> opentelemetry::Context {
    mockforge_tracing::extract_trace_context(metadata)
}

/// Inject trace context into gRPC metadata
pub fn inject_grpc_trace_context(
    ctx: &opentelemetry::Context,
    metadata: &mut HashMap<String, String>,
) {
    mockforge_tracing::inject_trace_context(ctx, metadata);
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::global;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    #[test]
    fn test_create_grpc_span() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut span = create_grpc_span("UserService", "GetUser");
        record_grpc_success(&mut span, 42, Some(100), Some(200));

        // Span should be created successfully
    }

    #[test]
    fn test_trace_context_propagation() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut metadata = HashMap::new();
        metadata.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let ctx = extract_grpc_trace_context(&metadata);

        let mut output_metadata = HashMap::new();
        inject_grpc_trace_context(&ctx, &mut output_metadata);

        // Context should be preserved
        assert!(output_metadata.contains_key("traceparent"));
    }
}

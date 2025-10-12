//! Distributed tracing for WebSocket connections

use mockforge_tracing::{create_request_span, record_error, record_success, Protocol};
use opentelemetry::{global::BoxedSpan, trace::Span, KeyValue};
use std::collections::HashMap;
use tracing::debug;

/// Create a span for a WebSocket connection
pub fn create_ws_connection_span(path: &str) -> BoxedSpan {
    create_request_span(
        Protocol::WebSocket,
        &format!("WS Connect {}", path),
        vec![
            KeyValue::new("ws.path", path.to_string()),
            KeyValue::new("network.protocol.name", "websocket"),
        ],
    )
}

/// Create a span for a WebSocket message
pub fn create_ws_message_span(direction: &str, message_type: &str, size: usize) -> BoxedSpan {
    create_request_span(
        Protocol::WebSocket,
        &format!("WS Message {}", direction),
        vec![
            KeyValue::new("ws.direction", direction.to_string()),
            KeyValue::new("ws.message.type", message_type.to_string()),
            KeyValue::new("ws.message.size", size as i64),
        ],
    )
}

/// Record successful WebSocket connection
pub fn record_ws_connection_success(
    span: &mut BoxedSpan,
    duration_ms: u64,
    messages_sent: usize,
    messages_received: usize,
) {
    let attributes = vec![
        KeyValue::new("ws.duration_ms", duration_ms as i64),
        KeyValue::new("ws.messages.sent", messages_sent as i64),
        KeyValue::new("ws.messages.received", messages_received as i64),
    ];

    record_success(span, attributes);

    debug!(
        duration_ms = duration_ms,
        messages_sent = messages_sent,
        messages_received = messages_received,
        "WebSocket connection completed"
    );
}

/// Record WebSocket connection error
pub fn record_ws_error(span: &mut BoxedSpan, error_message: &str, duration_ms: u64) {
    span.set_attribute(KeyValue::new("ws.duration_ms", duration_ms as i64));
    record_error(span, error_message);

    debug!(
        error_message = error_message,
        duration_ms = duration_ms,
        "WebSocket connection failed"
    );
}

/// Record WebSocket message success
pub fn record_ws_message_success(span: &mut BoxedSpan, processing_time_us: u64) {
    let attributes = vec![KeyValue::new(
        "ws.processing_time_us",
        processing_time_us as i64,
    )];

    record_success(span, attributes);
}

/// Extract trace context from WebSocket headers
pub fn extract_ws_trace_context(headers: &HashMap<String, String>) -> opentelemetry::Context {
    mockforge_tracing::extract_trace_context(headers)
}

/// Inject trace context into WebSocket response headers
pub fn inject_ws_trace_context(
    ctx: &opentelemetry::Context,
    headers: &mut HashMap<String, String>,
) {
    mockforge_tracing::inject_trace_context(ctx, headers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::global;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    #[test]
    fn test_create_ws_connection_span() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut span = create_ws_connection_span("/ws/chat");
        record_ws_connection_success(&mut span, 5000, 10, 12);
    }

    #[test]
    fn test_create_ws_message_span() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut span = create_ws_message_span("inbound", "text", 256);
        record_ws_message_success(&mut span, 150);
    }

    #[test]
    fn test_ws_trace_context_propagation() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let ctx = extract_ws_trace_context(&headers);

        let mut output_headers = HashMap::new();
        inject_ws_trace_context(&ctx, &mut output_headers);

        assert!(output_headers.contains_key("traceparent"));
    }
}

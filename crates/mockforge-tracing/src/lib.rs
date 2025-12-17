//! OpenTelemetry tracing integration for MockForge
//!
//! This crate provides distributed tracing capabilities across all MockForge protocols
//! (HTTP, gRPC, WebSocket, GraphQL) using OpenTelemetry and Jaeger.

pub mod context;
pub mod exporter;
pub mod tracer;

pub use context::{
    extract_from_axum_headers, extract_trace_context, inject_into_axum_headers,
    inject_trace_context, TraceContext,
};
pub use exporter::{
    ExporterError, ExporterType, JaegerExporter, OtlpCompression, OtlpExporter, OtlpProtocol,
};
pub use tracer::{init_tracer, shutdown_tracer, TracingConfig};

use opentelemetry::global::BoxedSpan;
use opentelemetry::trace::{Span, SpanKind, Status, Tracer};
use opentelemetry::{global, KeyValue};
use std::time::SystemTime;

/// Protocol types for tracing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    Grpc,
    WebSocket,
    GraphQL,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Grpc => "grpc",
            Protocol::WebSocket => "websocket",
            Protocol::GraphQL => "graphql",
        }
    }
}

/// Create a span for an incoming request
pub fn create_request_span(
    protocol: Protocol,
    operation_name: &str,
    attributes: Vec<KeyValue>,
) -> BoxedSpan {
    let tracer = global::tracer("mockforge");

    let mut span = tracer
        .span_builder(operation_name.to_string())
        .with_kind(SpanKind::Server)
        .with_start_time(SystemTime::now())
        .with_attributes(attributes)
        .start(&tracer);

    // Add protocol attribute
    span.set_attribute(KeyValue::new("mockforge.protocol", protocol.as_str()));

    span
}

/// Record span success with optional attributes
pub fn record_success(span: &mut BoxedSpan, attributes: Vec<KeyValue>) {
    for attr in attributes {
        span.set_attribute(attr);
    }
    span.set_status(Status::Ok);
}

/// Record span error
pub fn record_error(span: &mut BoxedSpan, error_message: &str) {
    span.set_status(Status::error(error_message.to_string()));
    span.set_attribute(KeyValue::new("error", true));
    span.set_attribute(KeyValue::new("error.message", error_message.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_as_str() {
        assert_eq!(Protocol::Http.as_str(), "http");
        assert_eq!(Protocol::Grpc.as_str(), "grpc");
        assert_eq!(Protocol::WebSocket.as_str(), "websocket");
        assert_eq!(Protocol::GraphQL.as_str(), "graphql");
    }

    #[test]
    fn test_protocol_debug() {
        assert_eq!(format!("{:?}", Protocol::Http), "Http");
        assert_eq!(format!("{:?}", Protocol::Grpc), "Grpc");
        assert_eq!(format!("{:?}", Protocol::WebSocket), "WebSocket");
        assert_eq!(format!("{:?}", Protocol::GraphQL), "GraphQL");
    }

    #[test]
    fn test_protocol_clone() {
        let proto = Protocol::Http;
        let cloned = proto.clone();
        assert_eq!(proto, cloned);
    }

    #[test]
    fn test_protocol_copy() {
        let proto = Protocol::Grpc;
        let copied = proto; // Copy, not move
        assert_eq!(proto, copied);
        assert_eq!(Protocol::Grpc, copied); // proto still accessible
    }

    #[test]
    fn test_protocol_eq() {
        assert_eq!(Protocol::Http, Protocol::Http);
        assert_ne!(Protocol::Http, Protocol::Grpc);
        assert_ne!(Protocol::WebSocket, Protocol::GraphQL);
    }

    #[test]
    fn test_create_request_span() {
        // Initialize a no-op tracer for testing
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let attributes = vec![
            KeyValue::new("http.method", "GET"),
            KeyValue::new("http.url", "/api/users"),
        ];

        let span = create_request_span(Protocol::Http, "test-operation", attributes);

        // Verify span was created (it's a BoxedSpan)
        assert!(!span.span_context().trace_id().to_string().is_empty());
    }

    #[test]
    fn test_create_request_span_all_protocols() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let protocols = [
            Protocol::Http,
            Protocol::Grpc,
            Protocol::WebSocket,
            Protocol::GraphQL,
        ];

        for protocol in protocols {
            let span = create_request_span(protocol, "test-op", vec![]);
            assert!(!span.span_context().trace_id().to_string().is_empty());
        }
    }

    #[test]
    fn test_record_success() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let mut span = create_request_span(Protocol::Http, "success-test", vec![]);

        let attributes = vec![
            KeyValue::new("http.status_code", 200),
            KeyValue::new("response.size", 1024),
        ];

        record_success(&mut span, attributes);
        // If we get here without panic, the function worked
    }

    #[test]
    fn test_record_success_empty_attributes() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let mut span = create_request_span(Protocol::Grpc, "success-empty", vec![]);
        record_success(&mut span, vec![]);
        // Should complete without error
    }

    #[test]
    fn test_record_error() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let mut span = create_request_span(Protocol::Http, "error-test", vec![]);

        record_error(&mut span, "Connection refused");
        // If we get here without panic, the function worked
    }

    #[test]
    fn test_record_error_with_details() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let mut span = create_request_span(Protocol::WebSocket, "error-details", vec![]);

        record_error(&mut span, "WebSocket handshake failed: 401 Unauthorized");
        // Should complete without error
    }

    #[test]
    fn test_record_error_empty_message() {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let provider = SdkTracerProvider::builder().build();
        global::set_tracer_provider(provider);

        let mut span = create_request_span(Protocol::GraphQL, "error-empty", vec![]);
        record_error(&mut span, "");
        // Should handle empty error messages gracefully
    }
}

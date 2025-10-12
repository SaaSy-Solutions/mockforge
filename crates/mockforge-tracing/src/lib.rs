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
}

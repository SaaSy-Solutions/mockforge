//! Trace context propagation utilities
//!
//! Implements W3C Trace Context standard for propagating trace information
//! across service boundaries.

use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::trace::{TraceContextExt, TraceId, SpanId};
use opentelemetry::{global, Context};
use std::collections::HashMap;

/// Trace context information
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub trace_flags: u8,
}

impl TraceContext {
    /// Create from OpenTelemetry context
    pub fn from_context(ctx: &Context) -> Option<Self> {
        let span = ctx.span();
        let span_context = span.span_context();
        if span_context.is_valid() {
            Some(Self {
                trace_id: format!("{:032x}", span_context.trace_id()),
                span_id: format!("{:016x}", span_context.span_id()),
                trace_flags: span_context.trace_flags().to_u8(),
            })
        } else {
            None
        }
    }

    /// Get trace ID as TraceId type
    pub fn trace_id(&self) -> Option<TraceId> {
        TraceId::from_hex(&self.trace_id).ok()
    }

    /// Get span ID as SpanId type
    pub fn span_id(&self) -> Option<SpanId> {
        SpanId::from_hex(&self.span_id).ok()
    }
}

/// Extract trace context from HTTP headers
pub fn extract_trace_context(headers: &HashMap<String, String>) -> Context {
    let extractor = HeaderExtractor(headers);
    global::get_text_map_propagator(|prop| prop.extract(&extractor))
}

/// Inject trace context into HTTP headers
pub fn inject_trace_context(ctx: &Context, headers: &mut HashMap<String, String>) {
    let mut injector = HeaderInjector(headers);
    global::get_text_map_propagator(|prop| prop.inject_context(ctx, &mut injector));
}

/// HTTP header extractor for trace context
struct HeaderExtractor<'a>(&'a HashMap<String, String>);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|v| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// HTTP header injector for trace context
struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

/// Extract trace context from Axum HTTP headers
pub fn extract_from_axum_headers(
    headers: &axum::http::HeaderMap,
) -> Context {
    let mut header_map = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(key.to_string(), value_str.to_string());
        }
    }
    extract_trace_context(&header_map)
}

/// Inject trace context into Axum HTTP headers
pub fn inject_into_axum_headers(
    ctx: &Context,
    headers: &mut axum::http::HeaderMap,
) {
    let mut header_map = HashMap::new();
    inject_trace_context(ctx, &mut header_map);

    for (key, value) in header_map {
        if let (Ok(header_name), Ok(header_value)) = (
            axum::http::HeaderName::try_from(&key),
            axum::http::HeaderValue::try_from(&value),
        ) {
            headers.insert(header_name, header_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_inject_round_trip() {
        // Set up the global propagator
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let ctx = extract_trace_context(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        assert!(trace_ctx.is_some());
        let trace_ctx = trace_ctx.unwrap();
        assert_eq!(trace_ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_empty_headers() {
        let headers = HashMap::new();
        let ctx = extract_trace_context(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        // Should create a new trace context
        assert!(trace_ctx.is_none());
    }
}

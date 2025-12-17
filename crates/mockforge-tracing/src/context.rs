//! Trace context propagation utilities
//!
//! Implements W3C Trace Context standard for propagating trace information
//! across service boundaries.

use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::trace::{SpanId, TraceContextExt, TraceId};
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
pub fn extract_from_axum_headers(headers: &http::HeaderMap) -> Context {
    let mut header_map = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(key.to_string(), value_str.to_string());
        }
    }
    extract_trace_context(&header_map)
}

/// Inject trace context into Axum HTTP headers
pub fn inject_into_axum_headers(ctx: &Context, headers: &mut http::HeaderMap) {
    let mut header_map = HashMap::new();
    inject_trace_context(ctx, &mut header_map);

    for (key, value) in header_map {
        if let (Ok(header_name), Ok(header_value)) =
            (http::HeaderName::try_from(&key), http::HeaderValue::try_from(&value))
        {
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

    #[test]
    fn test_trace_context_debug() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        let debug_str = format!("{:?}", trace_ctx);
        assert!(debug_str.contains("TraceContext"));
        assert!(debug_str.contains("0af7651916cd43dd8448eb211c80319c"));
        assert!(debug_str.contains("b7ad6b7169203331"));
    }

    #[test]
    fn test_trace_context_clone() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        let cloned = trace_ctx.clone();
        assert_eq!(cloned.trace_id, trace_ctx.trace_id);
        assert_eq!(cloned.span_id, trace_ctx.span_id);
        assert_eq!(cloned.trace_flags, trace_ctx.trace_flags);
    }

    #[test]
    fn test_trace_context_trace_id_valid() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        let trace_id = trace_ctx.trace_id();
        assert!(trace_id.is_some());
    }

    #[test]
    fn test_trace_context_trace_id_invalid() {
        let trace_ctx = TraceContext {
            trace_id: "invalid".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        let trace_id = trace_ctx.trace_id();
        assert!(trace_id.is_none());
    }

    #[test]
    fn test_trace_context_span_id_valid() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        let span_id = trace_ctx.span_id();
        assert!(span_id.is_some());
    }

    #[test]
    fn test_trace_context_span_id_invalid() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "invalid".to_string(),
            trace_flags: 1,
        };
        let span_id = trace_ctx.span_id();
        assert!(span_id.is_none());
    }

    #[test]
    fn test_inject_trace_context() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        // First extract a context from headers
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let ctx = extract_trace_context(&headers);

        // Now inject into new headers
        let mut new_headers = HashMap::new();
        inject_trace_context(&ctx, &mut new_headers);

        // Verify traceparent was injected
        assert!(new_headers.contains_key("traceparent"));
        let traceparent = new_headers.get("traceparent").unwrap();
        assert!(traceparent.starts_with("00-0af7651916cd43dd8448eb211c80319c"));
    }

    #[test]
    fn test_inject_trace_context_empty_context() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let ctx = Context::new();
        let mut headers = HashMap::new();
        inject_trace_context(&ctx, &mut headers);

        // Empty context shouldn't inject anything meaningful
        // The header might be empty or not present
    }

    #[test]
    fn test_extract_from_axum_headers() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = http::HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".parse().unwrap(),
        );

        let ctx = extract_from_axum_headers(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        assert!(trace_ctx.is_some());
        let trace_ctx = trace_ctx.unwrap();
        assert_eq!(trace_ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_extract_from_axum_headers_empty() {
        let headers = http::HeaderMap::new();
        let ctx = extract_from_axum_headers(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        assert!(trace_ctx.is_none());
    }

    #[test]
    fn test_extract_from_axum_headers_with_tracestate() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = http::HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".parse().unwrap(),
        );
        headers.insert("tracestate", "congo=t61rcWkgMzE".parse().unwrap());

        let ctx = extract_from_axum_headers(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        assert!(trace_ctx.is_some());
    }

    #[test]
    fn test_inject_into_axum_headers() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        // First extract a context
        let mut input_headers = HashMap::new();
        input_headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );
        let ctx = extract_trace_context(&input_headers);

        // Inject into axum headers
        let mut axum_headers = http::HeaderMap::new();
        inject_into_axum_headers(&ctx, &mut axum_headers);

        // Verify header was injected
        assert!(axum_headers.contains_key("traceparent"));
    }

    #[test]
    fn test_inject_into_axum_headers_empty_context() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let ctx = Context::new();
        let mut headers = http::HeaderMap::new();
        inject_into_axum_headers(&ctx, &mut headers);

        // Should not panic, headers might be empty
    }

    #[test]
    fn test_header_extractor() {
        let mut headers = HashMap::new();
        headers.insert("key1".to_string(), "value1".to_string());
        headers.insert("key2".to_string(), "value2".to_string());

        let extractor = HeaderExtractor(&headers);

        assert_eq!(extractor.get("key1"), Some("value1"));
        assert_eq!(extractor.get("key2"), Some("value2"));
        assert_eq!(extractor.get("nonexistent"), None);

        let keys = extractor.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1"));
        assert!(keys.contains(&"key2"));
    }

    #[test]
    fn test_header_injector() {
        let mut headers = HashMap::new();

        {
            let mut injector = HeaderInjector(&mut headers);
            injector.set("key1", "value1".to_string());
            injector.set("key2", "value2".to_string());
        }

        assert_eq!(headers.get("key1"), Some(&"value1".to_string()));
        assert_eq!(headers.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_header_injector_overwrite() {
        let mut headers = HashMap::new();
        headers.insert("key1".to_string(), "old_value".to_string());

        {
            let mut injector = HeaderInjector(&mut headers);
            injector.set("key1", "new_value".to_string());
        }

        assert_eq!(headers.get("key1"), Some(&"new_value".to_string()));
    }

    #[test]
    fn test_trace_context_trace_flags() {
        let trace_ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 0,
        };
        assert_eq!(trace_ctx.trace_flags, 0);

        let trace_ctx_sampled = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 1,
        };
        assert_eq!(trace_ctx_sampled.trace_flags, 1);
    }

    #[test]
    fn test_extract_multiple_headers() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );
        headers.insert("x-custom-header".to_string(), "custom-value".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let ctx = extract_trace_context(&headers);
        let trace_ctx = TraceContext::from_context(&ctx);

        // Should still extract trace context correctly despite other headers
        assert!(trace_ctx.is_some());
        assert_eq!(trace_ctx.unwrap().trace_id, "0af7651916cd43dd8448eb211c80319c");
    }
}

//! Distributed tracing for GraphQL queries and mutations

use mockforge_tracing::{create_request_span, record_error, record_success, Protocol};
use opentelemetry::{global::BoxedSpan, trace::Span, KeyValue};
use std::collections::HashMap;
use tracing::debug;

/// Create a span for a GraphQL query/mutation
pub fn create_graphql_span(
    operation_type: &str,
    operation_name: Option<&str>,
    query: &str,
) -> BoxedSpan {
    let span_name = if let Some(name) = operation_name {
        format!("GraphQL {} {}", operation_type, name)
    } else {
        format!("GraphQL {}", operation_type)
    };

    let mut attributes = vec![
        KeyValue::new("graphql.operation.type", operation_type.to_string()),
        KeyValue::new("graphql.document", query.to_string()),
    ];

    if let Some(name) = operation_name {
        attributes.push(KeyValue::new("graphql.operation.name", name.to_string()));
    }

    create_request_span(Protocol::GraphQL, &span_name, attributes)
}

/// Record successful GraphQL query execution
pub fn record_graphql_success(
    span: &mut BoxedSpan,
    duration_ms: u64,
    field_count: usize,
    resolver_calls: usize,
) {
    let attributes = vec![
        KeyValue::new("graphql.duration_ms", duration_ms as i64),
        KeyValue::new("graphql.fields_resolved", field_count as i64),
        KeyValue::new("graphql.resolver_calls", resolver_calls as i64),
    ];

    record_success(span, attributes);

    debug!(
        duration_ms = duration_ms,
        field_count = field_count,
        resolver_calls = resolver_calls,
        "GraphQL query completed successfully"
    );
}

/// Record GraphQL query error
pub fn record_graphql_error(
    span: &mut BoxedSpan,
    error_message: &str,
    error_path: Option<Vec<String>>,
    duration_ms: u64,
) {
    span.set_attribute(KeyValue::new("graphql.duration_ms", duration_ms as i64));
    span.set_attribute(KeyValue::new("graphql.error.message", error_message.to_string()));

    if let Some(path) = error_path {
        span.set_attribute(KeyValue::new(
            "graphql.error.path",
            format!("{:?}", path),
        ));
    }

    record_error(span, error_message);

    debug!(
        error_message = error_message,
        duration_ms = duration_ms,
        "GraphQL query failed"
    );
}

/// Create a span for a specific field resolver
pub fn create_resolver_span(
    parent_type: &str,
    field_name: &str,
) -> BoxedSpan {
    create_request_span(
        Protocol::GraphQL,
        &format!("Resolve {}.{}", parent_type, field_name),
        vec![
            KeyValue::new("graphql.resolver.parent_type", parent_type.to_string()),
            KeyValue::new("graphql.resolver.field_name", field_name.to_string()),
        ],
    )
}

/// Record resolver success
pub fn record_resolver_success(
    span: &mut BoxedSpan,
    duration_us: u64,
) {
    let attributes = vec![
        KeyValue::new("graphql.resolver.duration_us", duration_us as i64),
    ];

    record_success(span, attributes);
}

/// Record resolver error
pub fn record_resolver_error(
    span: &mut BoxedSpan,
    error_message: &str,
) {
    record_error(span, error_message);
}

/// Extract trace context from GraphQL request headers
pub fn extract_graphql_trace_context(
    headers: &HashMap<String, String>,
) -> opentelemetry::Context {
    mockforge_tracing::extract_trace_context(headers)
}

/// Inject trace context into GraphQL response headers
pub fn inject_graphql_trace_context(
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
    fn test_create_graphql_span() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let query = "query GetUser($id: ID!) { user(id: $id) { name } }";
        let mut span = create_graphql_span("query", Some("GetUser"), query);
        record_graphql_success(&mut span, 150, 3, 5);
    }

    #[test]
    fn test_create_resolver_span() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut span = create_resolver_span("User", "name");
        record_resolver_success(&mut span, 250);
    }

    #[test]
    fn test_graphql_trace_context_propagation() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let ctx = extract_graphql_trace_context(&headers);

        let mut output_headers = HashMap::new();
        inject_graphql_trace_context(&ctx, &mut output_headers);

        assert!(output_headers.contains_key("traceparent"));
    }
}

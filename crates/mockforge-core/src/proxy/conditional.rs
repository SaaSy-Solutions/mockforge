//! Conditional proxy evaluation using expressions (JSONPath, JavaScript-like, etc.)

use crate::conditions::{evaluate_condition, ConditionContext};
use crate::proxy::config::ProxyRule;
use crate::{Error, Result};
use axum::http::{HeaderMap, Method, Uri};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Evaluate whether a proxy rule's condition matches the request
pub fn evaluate_proxy_condition(
    rule: &ProxyRule,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Option<&[u8]>,
) -> Result<bool> {
    // If no condition is specified, the rule matches (path pattern already matched)
    let Some(ref condition) = rule.condition else {
        return Ok(true);
    };

    // Build condition context from request
    let mut context = ConditionContext::new()
        .with_method(method.as_str().to_string())
        .with_path(uri.path().to_string());

    // Extract query parameters
    let query_params: HashMap<String, String> = uri
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();
    context = context.with_query_params(query_params);

    // Extract headers
    let headers_map: HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|v_str| (k.as_str().to_lowercase(), v_str.to_string()))
        })
        .collect();
    context = context.with_headers(headers_map);

    // Parse request body if present
    if let Some(body_bytes) = body {
        if let Ok(body_str) = std::str::from_utf8(body_bytes) {
            // Try to parse as JSON
            if let Ok(json_value) = serde_json::from_str::<Value>(body_str) {
                context = context.with_request_body(json_value);
            }
        }
    }

    // Evaluate the condition
    match evaluate_condition(condition, &context) {
        Ok(result) => {
            debug!(
                "Proxy condition '{}' evaluated to {} for {} {}",
                condition,
                result,
                method,
                uri.path()
            );
            Ok(result)
        }
        Err(e) => {
            // Log error but don't fail - treat as false (don't proxy)
            tracing::warn!(
                "Failed to evaluate proxy condition '{}': {}. Treating as false.",
                condition,
                e
            );
            Ok(false)
        }
    }
}

/// Find matching proxy rule with condition evaluation
pub fn find_matching_rule<'a>(
    rules: &'a [ProxyRule],
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Option<&[u8]>,
    path_matches: impl Fn(&str, &str) -> bool,
) -> Option<&'a ProxyRule> {
    for rule in rules {
        if !rule.enabled {
            continue;
        }

        // Check if path matches
        if !path_matches(&rule.path_pattern, uri.path()) {
            continue;
        }

        // Evaluate condition if present
        match evaluate_proxy_condition(rule, method, uri, headers, body) {
            Ok(true) => return Some(rule),
            Ok(false) => continue, // Condition didn't match, try next rule
            Err(e) => {
                tracing::warn!("Error evaluating condition for rule {}: {}", rule.path_pattern, e);
                continue;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::config::ProxyRule;
    use axum::http::HeaderValue;
    use serde_json::json;

    fn create_test_rule(path: &str, condition: Option<&str>) -> ProxyRule {
        ProxyRule {
            path_pattern: path.to_string(),
            target_url: "http://example.com".to_string(),
            enabled: true,
            pattern: path.to_string(),
            upstream_url: "http://example.com".to_string(),
            migration_mode: crate::proxy::config::MigrationMode::Auto,
            migration_group: None,
            condition: condition.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_no_condition() {
        let rule = create_test_rule("/api/users", None);
        let method = Method::GET;
        let uri = Uri::from_static("/api/users");
        let headers = HeaderMap::new();

        let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None).unwrap();
        assert!(result); // No condition means always true
    }

    #[test]
    fn test_header_condition() {
        let rule = create_test_rule("/api/users", Some("header[authorization] != ''"));
        let method = Method::GET;
        let uri = Uri::from_static("/api/users");
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer token123"));

        let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None).unwrap();
        assert!(result);
    }

    #[test]
    fn test_jsonpath_condition() {
        let rule = create_test_rule("/api/users", Some("$.user.role"));
        let method = Method::POST;
        let uri = Uri::from_static("/api/users");
        let headers = HeaderMap::new();
        let body = json!({
            "user": {
                "role": "admin"
            }
        });
        let body_bytes = serde_json::to_string(&body).unwrap().into_bytes();

        let result =
            evaluate_proxy_condition(&rule, &method, &uri, &headers, Some(&body_bytes)).unwrap();
        assert!(result);
    }

    #[test]
    fn test_query_param_condition() {
        let rule = create_test_rule("/api/users", Some("query[env] == 'production'"));
        let method = Method::GET;
        let uri = Uri::from_static("/api/users?env=production");
        let headers = HeaderMap::new();

        let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None).unwrap();
        assert!(result);
    }

    #[test]
    fn test_complex_condition() {
        let rule = create_test_rule(
            "/api/users",
            Some("AND(header[authorization] != '', query[env] == 'production')"),
        );
        let method = Method::GET;
        let uri = Uri::from_static("/api/users?env=production");
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer token"));

        let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None).unwrap();
        assert!(result);
    }
}

//! Per-deployment HTTP response routers, with no public backend origin.

use crate::models::HostedMock;
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Method, Request, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::Instant,
};
use tower::ServiceExt;
use uuid::Uuid;

const MAX_CACHED_DEPLOYMENTS: usize = 128;
const MAX_CONFIG_BYTES: usize = 1024 * 1024;
const MAX_ROUTES: usize = 256;

struct Cached {
    revision: DateTime<Utc>,
    router: Router,
    used: Instant,
}

static ROUTERS: LazyLock<Mutex<HashMap<(Uuid, Uuid), Cached>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Bound buffering and configured delays across all locally hosted requests.
static IN_FLIGHT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(64);

pub(crate) fn is_local(deployment: &HostedMock) -> bool {
    deployment.metadata_json.get("hosting_backend").and_then(|v| v.as_str()) == Some("multitenant")
}

pub(crate) fn validate_spec(
    spec: &mockforge_openapi::OpenApiSpec,
    config: &serde_json::Value,
) -> Result<(), String> {
    let document = serde_json::to_value(&spec.spec).map_err(|_| "Invalid OpenAPI document")?;
    validate_document_resources(&document)?;
    let paths = spec.all_paths_and_operations();
    let count: usize = paths.values().map(|ops| ops.len()).sum();
    let configured: mockforge_core::config::ServerConfig =
        serde_json::from_value(config.clone()).map_err(|e| e.to_string())?;
    if count + configured.routes.len() > MAX_ROUTES {
        return Err("Hosted mock configuration exceeds 256 operations".into());
    }
    // Reuse the CLI engine's path/method validation before Axum registration.
    // This catches conflicting parameter templates and configured overrides.
    let mut routes = configured.routes;
    for (path, operations) in paths {
        for method in operations.keys() {
            routes.push(mockforge_core::config::RouteConfig {
                path: path.clone(),
                method: method.to_uppercase(),
                request: None,
                response: mockforge_core::config::RouteResponseConfig {
                    status: 200,
                    headers: HashMap::new(),
                    body: None,
                },
                fault_injection: None,
                latency: None,
            });
        }
    }
    let _router = mockforge_http::configured_routes::build_configured_routes_router(routes, false)?;
    Ok(())
}

/// Bound recursive schema expansion before the synchronous response generator runs.
/// Each array may synthesize up to 100 items, including nested arrays and refs.
fn validate_document_resources(document: &serde_json::Value) -> Result<(), String> {
    fn visit(
        value: &serde_json::Value,
        document: &serde_json::Value,
        references: &mut Vec<String>,
        depth: usize,
        copies: usize,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let node_bytes = 64 + value.as_str().map_or(0, str::len);
        let cost = copies.saturating_mul(node_bytes);
        if depth > 32 || cost > *remaining {
            return Err("Hosted specification exceeds the response generation budget".into());
        }
        *remaining -= cost;
        match value {
            serde_json::Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(|value| value.as_str()) {
                    let pointer = reference.strip_prefix('#').ok_or_else(|| {
                        "Hosted specifications require local references".to_string()
                    })?;
                    if references.iter().any(|active| active == reference) {
                        return Err(
                            "Recursive schemas require an isolated deployment provider".into()
                        );
                    }
                    let target = document
                        .pointer(pointer)
                        .ok_or_else(|| "Unresolved specification reference".to_string())?;
                    references.push(reference.to_string());
                    visit(target, document, references, depth + 1, copies, remaining)?;
                    references.pop();
                }
                for (key, child) in object {
                    if key == "$ref" {
                        continue;
                    }
                    // Pagination may also expand an example's items array.
                    let multiplier = if key == "items" { 100 } else { 1 };
                    visit(
                        child,
                        document,
                        references,
                        depth + 1,
                        copies.saturating_mul(multiplier),
                        remaining,
                    )?;
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, document, references, depth + 1, copies, remaining)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(document, document, &mut Vec::new(), 0, 1, &mut (1024 * 1024))
}

/// Compile persisted configuration before marking a local deployment active.
pub(crate) fn prepare(deployment: &HostedMock) -> Result<Router, String> {
    let mut cache =
        ROUTERS.lock().map_err(|_| "Hosted response cache is unavailable".to_string())?;
    let key = (deployment.org_id, deployment.id);
    if let Some(cached) = cache.get_mut(&key) {
        if cached.revision == deployment.updated_at {
            cached.used = Instant::now();
            return Ok(cached.router.clone());
        }
    }
    let spec = if let Some(source) = deployment.openapi_spec_url.as_deref() {
        if deployment
            .metadata_json
            .get("runtime_openapi_source_sha256")
            .and_then(|value| value.as_str())
            != Some(super::spec_source::fingerprint(source).as_str())
        {
            return Err(
                "OpenAPI specification must be refreshed by deploying this configuration".into()
            );
        }
        let document = deployment
            .metadata_json
            .get("runtime_openapi")
            .ok_or_else(|| "OpenAPI specification snapshot is missing".to_string())?;
        let spec = mockforge_openapi::OpenApiSpec::from_json(document.clone())
            .map_err(|_| "Invalid OpenAPI specification".to_string())?;
        validate_spec(&spec, &deployment.config_json)?;
        Some(spec)
    } else {
        None
    };
    let encoded = serde_json::to_vec(&deployment.config_json).map_err(|e| e.to_string())?;
    if encoded.len() > MAX_CONFIG_BYTES {
        return Err("Hosted mock configuration exceeds 1 MiB".into());
    }
    let config: mockforge_core::config::ServerConfig =
        serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
    if config.routes.len() > MAX_ROUTES {
        return Err("Hosted mock configuration exceeds 256 routes".into());
    }
    if config.routes.iter().any(|route| {
        route
            .response
            .headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("set-cookie"))
    }) {
        return Err("Cookie-setting responses require an isolated deployment provider.".into());
    }
    let routes_empty = config.routes.is_empty() && spec.is_none();
    let has_health = config.routes.iter().any(|r| r.path == "/health/live")
        || spec
            .as_ref()
            .is_some_and(|spec| spec.all_paths_and_operations().contains_key("/health/live"));
    // Do not inherit process environment, global management APIs or provider
    // keys. This shared engine serves only this deployment's configured routes.
    let mut router =
        mockforge_http::configured_routes::build_configured_routes_router(config.routes, false)?;
    if let Some(spec) = spec {
        router = router.merge(
            mockforge_openapi::openapi_routes::OpenApiRouteRegistry::new_with_options(
                spec,
                mockforge_openapi::openapi_routes::ValidationOptions::default(),
            )
            .build_hosted_router(),
        );
    }
    if !has_health {
        router = router
            .route("/health/live", get(|| async { Json(serde_json::json!({"status":"healthy"})) }));
    }
    if routes_empty {
        router = router.route(
            "/",
            get(|| async { Json(serde_json::json!({"message":"MockForge hosted mock is ready"})) }),
        );
    }
    if cache.len() >= MAX_CACHED_DEPLOYMENTS && !cache.contains_key(&key) {
        if let Some(oldest) = cache.iter().min_by_key(|(_, value)| value.used).map(|(key, _)| *key)
        {
            cache.remove(&oldest);
        }
    }
    cache.insert(
        key,
        Cached {
            revision: deployment.updated_at,
            router: router.clone(),
            used: Instant::now(),
        },
    );
    Ok(router)
}

pub(crate) async fn serve(
    deployment: &HostedMock,
    method: Method,
    path: &str,
    query: Option<&str>,
    headers: HeaderMap,
    body: Body,
    max_body_bytes: usize,
) -> Result<Response, StatusCode> {
    let _permit = IN_FLIGHT.try_acquire().map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    let router = prepare(deployment).map_err(|error| {
        tracing::warn!(deployment_id = %deployment.id, %error, "Hosted response configuration is invalid");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    let dispatch = async {
        let body = to_bytes(body, max_body_bytes)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let uri = match query {
            Some(query) => format!("{path}?{query}"),
            None => path.to_string(),
        };
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::from(body))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        *request.headers_mut() = headers;
        let mut response =
            router.oneshot(request).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        // The path gateway shares the registry origin. Customer-controlled
        // content must not execute with its browser credentials or override
        // this document sandbox, even when it supplies an HTML content type.
        response.headers_mut().insert(
            "content-security-policy",
            axum::http::HeaderValue::from_static(
                "sandbox; default-src 'none'; frame-ancestors 'none'",
            ),
        );
        response
            .headers_mut()
            .insert("x-content-type-options", axum::http::HeaderValue::from_static("nosniff"));
        response
            .headers_mut()
            .insert("referrer-policy", axum::http::HeaderValue::from_static("no-referrer"));
        response.headers_mut().remove("set-cookie");
        // JSON bodies have a known actual length. Customer-supplied framing
        // must not truncate responses or inflate metered egress.
        for name in [
            "content-length",
            "transfer-encoding",
            "connection",
            "upgrade",
            "keep-alive",
            "trailer",
        ] {
            response.headers_mut().remove(name);
        }
        Ok(response)
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), dispatch)
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
}

#[cfg(test)]
mod resource_tests {
    use super::validate_document_resources;
    use serde_json::json;

    #[test]
    fn generation_budget_rejects_cycles_and_nested_array_amplification() {
        let recursive = json!({"components":{"schemas":{"Node":{"type":"object","properties":{"next":{"$ref":"#/components/schemas/Node"}}}}}});
        assert!(validate_document_resources(&recursive).unwrap_err().contains("Recursive"));
        let amplified = json!({"type":"array","items":{"type":"array","items":{"type":"array","items":{"type":"string"}}}});
        assert!(validate_document_resources(&amplified).unwrap_err().contains("budget"));
        let normal = json!({"components":{"schemas":{"Item":{"type":"object","properties":{"name":{"type":"string"}}}}},"schema":{"type":"array","items":{"$ref":"#/components/schemas/Item"}}});
        assert!(validate_document_resources(&normal).is_ok());
        assert!(
            validate_document_resources(&json!({"$ref":"https://fixture.invalid/spec"})).is_err()
        );
    }
}

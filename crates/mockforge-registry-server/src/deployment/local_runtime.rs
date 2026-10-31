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
    super::spec_budget::validate_document_resources(&document)?;
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

/// Compile persisted configuration before marking a local deployment active.
pub(crate) fn prepare(deployment: &HostedMock) -> Result<Router, String> {
    let mut cache =
        ROUTERS.lock().map_err(|_| "Hosted response cache is unavailable".to_string())?;
    let key = (deployment.org_id, deployment.id);
    if let Some(cached) = cache.get_mut(&key).filter(|c| c.revision == deployment.updated_at) {
        cached.used = Instant::now();
        return Ok(cached.router.clone());
    }
    let spec = snapshot_spec(deployment)?;
    let config = bounded_config(&deployment.config_json)?;
    let router = build_router(config, spec)?;
    cache_router(&mut cache, key, deployment.updated_at, router.clone());
    Ok(router)
}

/// The deployed OpenAPI snapshot, only if it matches the configured source.
fn snapshot_spec(
    deployment: &HostedMock,
) -> Result<Option<mockforge_openapi::OpenApiSpec>, String> {
    let Some(source) = deployment.openapi_spec_url.as_deref() else {
        return Ok(None);
    };
    let fingerprint = deployment
        .metadata_json
        .get("runtime_openapi_source_sha256")
        .and_then(|v| v.as_str());
    if fingerprint != Some(super::spec_source::fingerprint(source).as_str()) {
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
    Ok(Some(spec))
}

/// Parse the persisted configuration within the shared runtime's limits.
fn bounded_config(
    config_json: &serde_json::Value,
) -> Result<mockforge_core::config::ServerConfig, String> {
    let encoded = serde_json::to_vec(config_json).map_err(|e| e.to_string())?;
    if encoded.len() > MAX_CONFIG_BYTES {
        return Err("Hosted mock configuration exceeds 1 MiB".into());
    }
    let config: mockforge_core::config::ServerConfig =
        serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
    if config.routes.len() > MAX_ROUTES {
        return Err("Hosted mock configuration exceeds 256 routes".into());
    }
    let sets_cookie = config
        .routes
        .iter()
        .flat_map(|route| route.response.headers.keys())
        .any(|name| name.eq_ignore_ascii_case("set-cookie"));
    if sets_cookie {
        return Err("Cookie-setting responses require an isolated deployment provider.".into());
    }
    Ok(config)
}

/// Configured routes plus spec routes, with default health and root routes.
/// Does not inherit process environment, global management APIs or provider
/// keys: this shared engine serves only this deployment's configured routes.
fn build_router(
    config: mockforge_core::config::ServerConfig,
    spec: Option<mockforge_openapi::OpenApiSpec>,
) -> Result<Router, String> {
    let routes_empty = config.routes.is_empty() && spec.is_none();
    let has_health = config.routes.iter().any(|r| r.path == "/health/live")
        || spec
            .as_ref()
            .is_some_and(|spec| spec.all_paths_and_operations().contains_key("/health/live"));
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
    Ok(router)
}

/// Insert a compiled router, evicting the least recently used deployment at capacity.
fn cache_router(
    cache: &mut HashMap<(Uuid, Uuid), Cached>,
    key: (Uuid, Uuid),
    revision: DateTime<Utc>,
    router: Router,
) {
    if cache.len() >= MAX_CACHED_DEPLOYMENTS && !cache.contains_key(&key) {
        if let Some(oldest) = cache.iter().min_by_key(|(_, value)| value.used).map(|(key, _)| *key)
        {
            cache.remove(&oldest);
        }
    }
    cache.insert(
        key,
        Cached {
            revision,
            router,
            used: Instant::now(),
        },
    );
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
        harden_response_headers(response.headers_mut());
        Ok(response)
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), dispatch)
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
}

/// The path gateway shares the registry origin. Customer-controlled content
/// must not execute with its browser credentials or override this document
/// sandbox, even when it supplies an HTML content type. JSON bodies have a
/// known actual length, so customer-supplied framing must not truncate
/// responses or inflate metered egress.
fn harden_response_headers(headers: &mut HeaderMap) {
    use axum::http::HeaderValue;
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("sandbox; default-src 'none'; frame-ancestors 'none'"),
    );
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    for name in [
        "set-cookie",
        "content-length",
        "transfer-encoding",
        "connection",
        "upgrade",
        "keep-alive",
        "trailer",
    ] {
        headers.remove(name);
    }
}

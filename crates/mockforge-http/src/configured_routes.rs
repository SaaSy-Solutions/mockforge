//! Configured HTTP responses without management APIs or process-wide services.
use super::apply_route_chaos;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::MethodFilter,
    Json, Router,
};
use mockforge_core::config::RouteConfig;
use mockforge_core::priority_handler::RouteChaosInjectorTrait;
use mockforge_template_expansion::RequestContext;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
#[cfg(feature = "route-chaos")]
use tracing::warn;
use tracing::{debug, info};

/// Shared fault/latency injector; a trait object avoids a dependency cycle
/// (the trait lives in mockforge-core, the implementation in route-chaos).
type ChaosInjector = Option<Arc<dyn RouteChaosInjectorTrait>>;

/// Build the same configured response routes used by the CLI, without adding
/// management, file-serving, authentication, or other process-wide APIs.
///
/// Validates paths, methods and response status before registering any route.
/// Template expansion is explicitly selected by the caller.
pub fn build_configured_routes_router(
    routes: Vec<RouteConfig>,
    template_expand: bool,
) -> Result<Router, String> {
    let filters = validate_routes(&routes)?;
    let injector = route_chaos_injector(&routes);
    if !routes.is_empty() {
        info!("Registering {} custom route(s) from config", routes.len());
    }
    let mut app = Router::new();
    for (route, filter) in routes.into_iter().zip(filters) {
        app = register_route(app, route, filter, template_expand, injector.clone());
    }
    Ok(app)
}

/// Reject anything route registration could panic on, before registering any
/// route. Returns each route's method filter in input order.
fn validate_routes(routes: &[RouteConfig]) -> Result<Vec<MethodFilter>, String> {
    let mut paths = matchit::Router::<()>::new();
    let mut registered_paths = HashSet::new();
    let mut registered_methods = HashSet::new();
    routes
        .iter()
        .map(|route| {
            if !is_static_route_path(&route.path) {
                return Err(format!("Invalid route path: {}", route.path));
            }
            if registered_paths.insert(route.path.clone()) {
                paths.insert(&route.path, ()).map_err(|e| e.to_string())?;
            }
            let method =
                route.method.to_uppercase().parse::<http::Method>().map_err(|e| e.to_string())?;
            let filter = MethodFilter::try_from(method.clone()).map_err(|e| e.to_string())?;
            if !registered_methods.insert((route.path.clone(), method)) {
                return Err(format!("Duplicate route: {} {}", route.method, route.path));
            }
            validate_response(route)?;
            Ok(filter)
        })
        .collect()
}

/// Absolute path without query, fragment, or legacy `:param` / `*wildcard` segments.
fn is_static_route_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains('?')
        && !path.contains('#')
        && !path
            .split('/')
            .any(|segment| segment.starts_with(':') || segment.starts_with('*'))
}

fn validate_response(route: &RouteConfig) -> Result<(), String> {
    StatusCode::from_u16(route.response.status).map_err(|e| e.to_string())?;
    for (name, value) in &route.response.headers {
        http::HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?;
        http::HeaderValue::from_str(value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(feature = "route-chaos")]
fn route_chaos_injector(routes: &[RouteConfig]) -> ChaosInjector {
    if routes.is_empty() {
        return None;
    }
    match mockforge_route_chaos::RouteChaosInjector::new(routes.to_vec()) {
        Ok(injector) => {
            info!("Initialized advanced routing features for {} route(s)", routes.len());
            Some(Arc::new(injector))
        }
        Err(e) => {
            warn!("Failed to initialize advanced routing features: {}. Using basic routing.", e);
            None
        }
    }
}

#[cfg(not(feature = "route-chaos"))]
fn route_chaos_injector(_routes: &[RouteConfig]) -> ChaosInjector {
    None
}

/// Register one configured response. Method routing preserves multiple
/// configured methods on one path.
fn register_route(
    app: Router,
    route: RouteConfig,
    filter: MethodFilter,
    template_expand: bool,
    injector: ChaosInjector,
) -> Router {
    let RouteConfig {
        path,
        method,
        response,
        ..
    } = route;
    let status = response.status;
    let body = response.body;
    let headers = response.headers;
    debug!("Registered route: {} {}", method, path);
    app.route(
        &path,
        #[allow(clippy::non_send_fields_in_send_ty)]
        axum::routing::on(filter, move |req: Request<Body>| {
            let body = body.clone();
            let headers = headers.clone();
            let injector = injector.clone();
            async move {
                configured_response(req, body, headers, status, template_expand, injector).await
            }
        }),
    )
}

/// Apply route chaos, then return the configured body (optionally template
/// expanded), status and headers.
async fn configured_response(
    req: Request<Body>,
    body: Option<Value>,
    headers: HashMap<String, String>,
    status: u16,
    template_expand: bool,
    injector: ChaosInjector,
) -> Response {
    if let Some(fault_response) =
        apply_route_chaos(injector.as_deref(), req.method(), req.uri()).await
    {
        return fault_response;
    }
    let mut body_value = body.unwrap_or(serde_json::json!({}));
    if template_expand {
        body_value = expand_body(body_value, request_context(&req)).await;
    }
    let mut response = Json(body_value).into_response();
    *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
    for (key, value) in headers {
        if let (Ok(name), Ok(value)) = (
            http::HeaderName::from_bytes(key.as_bytes()),
            http::HeaderValue::from_str(&value),
        ) {
            response.headers_mut().insert(name, value);
        }
    }
    response
}

/// Request data available to templates: `{{request.query.name}}`,
/// `{{request.headers.name}}`, method and path. The body is not read.
fn request_context(req: &Request<Body>) -> RequestContext {
    let query_params: HashMap<String, Value> = req
        .uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .map(|(k, v)| (k, Value::String(v)))
                .collect()
        })
        .unwrap_or_default();
    let headers: HashMap<String, Value> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_str().unwrap_or_default().to_string())))
        .collect();
    RequestContext {
        method: req.method().to_string(),
        path: req.uri().path().to_string(),
        query_params,
        headers,
        body: None,
        path_params: HashMap::new(),
        multipart_fields: HashMap::new(),
        multipart_files: HashMap::new(),
    }
}

/// Expand templates off the async runtime. mockforge-template-expansion is
/// isolated from mockforge-core so nothing non-Send crosses this boundary;
/// a join failure keeps the unexpanded body.
async fn expand_body(body: Value, context: RequestContext) -> Value {
    let original = body.clone();
    tokio::task::spawn_blocking(move || {
        mockforge_template_expansion::expand_templates_in_json(body, &context)
    })
    .await
    .unwrap_or(original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn configured_routes_share_a_path_without_mounting_management_apis() {
        let routes = serde_json::from_value(serde_json::json!([
            {"path":"/users/{id}","method":"GET","response":{"status":200,"headers":{"x-mock":"alpha"},"body":{"name":"{{uuid}}"}}},
            {"path":"/users/{id}","method":"POST","response":{"status":201,"body":{"created":true}}}
        ])).unwrap();
        let router = build_configured_routes_router(routes, false).unwrap();
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/users/42").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["x-mock"], "alpha");
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["name"], "{{uuid}}");
        for (method, path, status) in [
            ("POST", "/users/42", 201),
            ("HEAD", "/users/42", 200),
            ("DELETE", "/users/42", 405),
            ("GET", "/__mockforge/api/config", 404),
        ] {
            let response = router
                .clone()
                .oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{method} {path}");
        }
    }

    #[test]
    fn invalid_configuration_cannot_panic_route_registration() {
        for path in [
            "users",
            "/:id",
            "/{broken",
            "/users?query",
            "/users#fragment",
            "/{*rest}/tail",
        ] {
            let routes = serde_json::from_value(
                serde_json::json!([{"path":path,"method":"GET","response":{"status":200}}]),
            )
            .unwrap();
            assert!(build_configured_routes_router(routes, false).is_err(), "{path}");
        }
        let routes = serde_json::from_value(serde_json::json!([
            {"path":"/users/{id}","method":"GET","response":{"status":200}},
            {"path":"/users/{name}","method":"POST","response":{"status":201}}
        ]))
        .unwrap();
        assert!(build_configured_routes_router(routes, false).is_err());
    }
}

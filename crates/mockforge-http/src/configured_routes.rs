//! Configured HTTP responses without management APIs or process-wide services.
use super::apply_route_chaos;
use axum::{body::Body, extract::Request, Json, Router};
use std::collections::HashSet;
#[cfg(feature = "route-chaos")]
use tracing::warn;
use tracing::{debug, info};

/// Build the same configured response routes used by the CLI, without adding
/// management, file-serving, authentication, or other process-wide APIs.
///
/// Validates paths, methods and response status before registering any route.
/// Template expansion is explicitly selected by the caller.
pub fn build_configured_routes_router(
    routes: Vec<mockforge_core::config::RouteConfig>,
    template_expand: bool,
) -> Result<Router, String> {
    let mut paths = matchit::Router::<()>::new();
    let mut registered_paths = HashSet::new();
    let mut registered_methods = HashSet::new();
    for route in &routes {
        if !route.path.starts_with('/')
            || route.path.contains('?')
            || route.path.contains('#')
            || route
                .path
                .split('/')
                .any(|segment| segment.starts_with(':') || segment.starts_with('*'))
        {
            return Err(format!("Invalid route path: {}", route.path));
        }
        if registered_paths.insert(route.path.clone()) {
            paths.insert(&route.path, ()).map_err(|e| e.to_string())?;
        }
        let method =
            route.method.to_uppercase().parse::<http::Method>().map_err(|e| e.to_string())?;
        axum::routing::MethodFilter::try_from(method.clone()).map_err(|e| e.to_string())?;
        if !registered_methods.insert((route.path.clone(), method)) {
            return Err(format!("Duplicate route: {} {}", route.method, route.path));
        }
        http::StatusCode::from_u16(route.response.status).map_err(|e| e.to_string())?;
        for (name, value) in &route.response.headers {
            http::HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?;
            http::HeaderValue::from_str(value).map_err(|e| e.to_string())?;
        }
    }
    let mut app = Router::new();
    let route_configs = Some(routes);
    // Register custom routes from config with advanced routing features
    // Create RouteChaosInjector for advanced fault injection and latency
    // Store as trait object to avoid circular dependency (RouteChaosInjectorTrait is in mockforge-core)
    #[cfg(feature = "route-chaos")]
    let route_chaos_injector: Option<
        std::sync::Arc<dyn mockforge_core::priority_handler::RouteChaosInjectorTrait>,
    > = if let Some(ref route_configs) = route_configs {
        if !route_configs.is_empty() {
            // Convert to the type expected by RouteChaosInjector
            // Note: Both use the same mockforge-core, but we need to ensure type compatibility
            let route_configs_converted: Vec<mockforge_core::config::RouteConfig> =
                route_configs.to_vec();
            match mockforge_route_chaos::RouteChaosInjector::new(route_configs_converted) {
                Ok(injector) => {
                    info!(
                        "Initialized advanced routing features for {} route(s)",
                        route_configs.len()
                    );
                    // RouteChaosInjector implements RouteChaosInjectorTrait, so we can cast it
                    // The trait is implemented in mockforge-route-chaos/src/lib.rs
                    Some(std::sync::Arc::new(injector)
                        as std::sync::Arc<
                            dyn mockforge_core::priority_handler::RouteChaosInjectorTrait,
                        >)
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize advanced routing features: {}. Using basic routing.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(not(feature = "route-chaos"))]
    let route_chaos_injector: Option<
        std::sync::Arc<dyn mockforge_core::priority_handler::RouteChaosInjectorTrait>,
    > = None;

    if let Some(route_configs) = route_configs {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;

        if !route_configs.is_empty() {
            info!("Registering {} custom route(s) from config", route_configs.len());
        }

        let injector = route_chaos_injector.clone();
        for route_config in route_configs {
            let status = route_config.response.status;
            let body = route_config.response.body.clone();
            let headers = route_config.response.headers.clone();
            let path = route_config.path.clone();
            let method = route_config.method.clone();

            // Create handler that returns the configured response with template expansion
            // Supports both basic templates ({{uuid}}, {{now}}) and request-aware templates
            // ({{request.query.name}}, {{request.path.id}}, {{request.headers.name}})
            // Method routing preserves multiple configured methods on one path.
            let expected_method = method.to_uppercase();
            // Clone Arc for the closure - Arc is Send-safe
            // Note: RouteChaosInjector is marked as Send+Sync via unsafe impl, so we have to
            // clone the Arc rather than move the inner injector into the route handler.
            let injector_clone = injector.clone();
            app = app.route(
                &path,
                #[allow(clippy::non_send_fields_in_send_ty)]
                axum::routing::on(
                    axum::routing::MethodFilter::try_from(
                        expected_method.parse::<http::Method>().map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?,
                    move |req: Request<Body>| {
                        let body = body.clone();
                        let headers = headers.clone();
                        let expand = template_expand;
                        let status_code = status;
                        // Clone Arc again for the async block
                        let injector_for_chaos = injector_clone.clone();

                        async move {
                            // Apply advanced routing features (fault injection and latency) if available
                            // Use helper function to avoid capturing RouteChaosInjector in closure
                            // Pass the Arc as a reference to the helper function
                            if let Some(fault_response) = apply_route_chaos(
                                injector_for_chaos.as_deref(),
                                req.method(),
                                req.uri(),
                            )
                            .await
                            {
                                return fault_response;
                            }

                            // Create JSON response from body, or empty object if None
                            let mut body_value = body.unwrap_or(serde_json::json!({}));

                            // Apply template expansion if enabled
                            // Use mockforge-template-expansion crate which is completely isolated
                            // from mockforge-core to avoid Send issues (no rng() in dependency chain)
                            if expand {
                                use mockforge_template_expansion::RequestContext;
                                use serde_json::Value;
                                use std::collections::HashMap;

                                // Extract request data for template expansion
                                let method = req.method().to_string();
                                let path = req.uri().path().to_string();

                                // Extract query parameters
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

                                // Extract headers
                                let headers: HashMap<String, Value> = req
                                    .headers()
                                    .iter()
                                    .map(|(k, v)| {
                                        (
                                            k.to_string(),
                                            Value::String(
                                                v.to_str().unwrap_or_default().to_string(),
                                            ),
                                        )
                                    })
                                    .collect();

                                // Create RequestContext for expand_prompt_template
                                // Using RequestContext from mockforge-template-expansion (not mockforge-core)
                                // to avoid bringing rng() into scope
                                let context = RequestContext {
                                    method,
                                    path,
                                    query_params,
                                    headers,
                                    body: None, // Body extraction would require reading the request stream
                                    path_params: HashMap::new(),
                                    multipart_fields: HashMap::new(),
                                    multipart_files: HashMap::new(),
                                };

                                // Perform template expansion in spawn_blocking to ensure Send safety
                                // The template expansion crate is completely isolated from mockforge-core
                                // and doesn't have rng() in its dependency chain
                                let body_value_clone = body_value.clone();
                                let context_clone = context.clone();
                                body_value = match tokio::task::spawn_blocking(move || {
                                    mockforge_template_expansion::expand_templates_in_json(
                                        body_value_clone,
                                        &context_clone,
                                    )
                                })
                                .await
                                {
                                    Ok(result) => result,
                                    Err(_) => body_value, // Fallback to original on error
                                };
                            }

                            let mut response = Json(body_value).into_response();

                            // Set status code
                            *response.status_mut() =
                                StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);

                            // Add custom headers
                            for (key, value) in headers {
                                if let Ok(header_name) =
                                    http::HeaderName::from_bytes(key.as_bytes())
                                {
                                    if let Ok(header_value) = http::HeaderValue::from_str(&value) {
                                        response.headers_mut().insert(header_name, header_value);
                                    }
                                }
                            }

                            response
                        }
                    },
                ),
            );

            debug!("Registered route: {} {}", method, path);
        }
    }

    Ok(app)
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
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&body).unwrap()["name"], "{{uuid}}");
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

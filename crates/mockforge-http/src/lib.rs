pub mod latency_profiles;
pub mod op_middleware;
pub mod overrides;
pub mod replay_listing;
pub mod schema_diff;

use axum::Router;
use mockforge_core::{OpenApiRouteRegistry, OpenApiSpec};
use mockforge_core::openapi_routes::ValidationOptions;
use axum::{routing::get, Json};
use serde::{Deserialize, Serialize};
use mockforge_core::{load_config, save_config};
use tracing::*;

/// Build the base HTTP router, optionally from an OpenAPI spec.
pub async fn build_router(spec_path: Option<String>, mut options: Option<ValidationOptions>) -> Router {
    // Set up the basic router
    let mut app = Router::new();

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec) = spec_path {
        match OpenApiSpec::from_file(&spec).await {
            Ok(openapi) => {
                info!("Loaded OpenAPI spec from {}", spec);
                // Add admin skip prefixes based on config via env (mount path) and internal admin API prefix
                if let Some(ref mut opts) = options {
                    if let Ok(pref) = std::env::var("MOCKFORGE_ADMIN_MOUNT_PREFIX") { if !pref.is_empty() { opts.admin_skip_prefixes.push(pref); } }
                    opts.admin_skip_prefixes.push("/__mockforge".to_string());
                }
                let registry = if let Some(opts) = options.clone() {
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    OpenApiRouteRegistry::new_with_env(openapi)
                };
                
                // Clone registry for routes listing before moving it to build_router
                let routes_registry = registry.clone();
                app = registry.build_router();

                // Expose routes listing for Admin UI
                if let Some(_opts) = options {
                    let routes_json = routes_registry
                        .routes()
                        .iter()
                        .map(|r| serde_json::json!({"method": r.method, "path": r.path}))
                        .collect::<Vec<_>>();
                    let handler = move || {
                        let data = routes_json.clone();
                        async move { Json(serde_json::json!({"routes": data})) }
                    };
                    app = app.route("/__mockforge/routes", get(handler));
                }
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {}: {}. Starting without OpenAPI integration.", spec, e);
                // Fall back to basic router
            }
        }
    }

    // Add basic health check endpoint if not already provided by OpenAPI spec
    app.route(
        "/health",
        axum::routing::get(|| async {
            use mockforge_core::server_utils::health::HealthStatus;
            axum::Json(serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")).unwrap())
        }),
    )
    // Admin: runtime validation toggle
    .route("/__mockforge/validation", get(get_validation).post(set_validation))
}

/// Serve a provided router on the given port.
pub async fn serve_router(
    port: u16,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("HTTP listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

/// Backwards-compatible start that builds + serves the base router.
pub async fn start(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router(spec_path, options).await;
    serve_router(port, app).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationSettings {
    mode: Option<String>,
    aggregate_errors: Option<bool>,
    validate_responses: Option<bool>,
    overrides: Option<serde_json::Map<String, serde_json::Value>>,
}

async fn get_validation() -> Json<ValidationSettings> {
    let mode = std::env::var("MOCKFORGE_REQUEST_VALIDATION").ok();
    let aggregate_errors = std::env::var("MOCKFORGE_AGGREGATE_ERRORS").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true"));
    let validate_responses = std::env::var("MOCKFORGE_RESPONSE_VALIDATION").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true"));
    let overrides = std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned());
    Json(ValidationSettings { mode, aggregate_errors, validate_responses, overrides })
}

async fn set_validation(Json(payload): Json<ValidationSettings>) -> Json<serde_json::Value> {
    if let Some(mode) = payload.mode { std::env::set_var("MOCKFORGE_REQUEST_VALIDATION", mode); }
    if let Some(agg) = payload.aggregate_errors { std::env::set_var("MOCKFORGE_AGGREGATE_ERRORS", if agg {"true"} else {"false"}); }
    if let Some(resp) = payload.validate_responses { std::env::set_var("MOCKFORGE_RESPONSE_VALIDATION", if resp {"true"} else {"false"}); }
    if let Some(map) = payload.overrides {
        let json = serde_json::Value::Object(map);
        if let Ok(s) = serde_json::to_string(&json) { std::env::set_var("MOCKFORGE_VALIDATION_OVERRIDES_JSON", s); }
    }
    // Optionally persist to config file if MOCKFORGE_CONFIG_PATH is set
    if let Ok(cfg_path) = std::env::var("MOCKFORGE_CONFIG_PATH") {
        if let Ok(mut cfg) = load_config(&cfg_path).await {
            if let Ok(mode) = std::env::var("MOCKFORGE_REQUEST_VALIDATION") { cfg.http.request_validation = mode; }
            if let Ok(agg) = std::env::var("MOCKFORGE_AGGREGATE_ERRORS") { cfg.http.aggregate_validation_errors = agg == "1" || agg.eq_ignore_ascii_case("true"); }
            if let Ok(rv) = std::env::var("MOCKFORGE_RESPONSE_VALIDATION") { cfg.http.validate_responses = rv == "1" || rv.eq_ignore_ascii_case("true"); }
            if let Ok(over_json) = std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&over_json) {
                    if let Some(obj) = val.as_object() {
                        cfg.http.validation_overrides.clear();
                        for (k, v) in obj {
                            if let Some(s) = v.as_str() { cfg.http.validation_overrides.insert(k.clone(), s.to_string()); }
                        }
                    }
                }
            }
            let _ = save_config(&cfg_path, &cfg).await;
        }
    }
    Json(serde_json::json!({"status":"ok"}))
}

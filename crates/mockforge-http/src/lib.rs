pub mod latency_profiles;
pub mod op_middleware;
pub mod overrides;
pub mod replay_listing;
pub mod schema_diff;

use axum::http::StatusCode;
use axum::Router;
use axum::{routing::get, Json};
use mockforge_core::openapi_routes::{
    get_last_validation_error, get_validation_errors, ValidationOptions,
};
use mockforge_core::{load_config, save_config, latency::LatencyInjector, LatencyProfile};
use mockforge_core::{OpenApiRouteRegistry, OpenApiSpec};
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use serde::{Deserialize, Serialize};
use tracing::*;

/// Build the base HTTP router, optionally from an OpenAPI spec.
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
) -> Router {
    build_router_with_latency(spec_path, options, None).await
}

/// Build the base HTTP router with latency injection support
pub async fn build_router_with_latency(
    spec_path: Option<String>,
    mut options: Option<ValidationOptions>,
    latency_injector: Option<LatencyInjector>,
) -> Router {
    build_router_with_injectors(spec_path, options, latency_injector, None).await
}

/// Build the base HTTP router with both latency and failure injection support
pub async fn build_router_with_injectors(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    latency_injector: Option<LatencyInjector>,
    failure_injector: Option<mockforge_core::FailureInjector>,
) -> Router {
    // If richer faker is available, register provider once (idempotent)
    #[cfg(feature = "data-faker")]
    {
        register_core_faker_provider();
    }
    // Set up the basic router
    let mut app = Router::new();

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec) = spec_path {
        match OpenApiSpec::from_file(&spec).await {
            Ok(openapi) => {
                info!("Loaded OpenAPI spec from {}", spec);
                // Add admin skip prefixes based on config via env (mount path) and internal admin API prefix
                if let Some(ref mut opts) = options {
                    if let Ok(pref) = std::env::var("MOCKFORGE_ADMIN_MOUNT_PREFIX") {
                        if !pref.is_empty() {
                            opts.admin_skip_prefixes.push(pref);
                        }
                    }
                    opts.admin_skip_prefixes.push("/__mockforge".to_string());
                }
                let registry = if let Some(mut opts) = options.clone() {
                    // Thread env overrides for new options if present
                    if let Ok(s) = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND") {
                        if s == "1" || s.eq_ignore_ascii_case("true") {
                            opts.response_template_expand = true;
                        }
                    }
                    if let Ok(s) = std::env::var("MOCKFORGE_VALIDATION_STATUS") {
                        if let Ok(c) = s.parse::<u16>() {
                            opts.validation_status = Some(c);
                        }
                    }
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    OpenApiRouteRegistry::new_with_env(openapi)
                };

                // Clone registry for routes listing before moving it to build_router
                let routes_registry = registry.clone();

                // Build router with latency and failure injection if provided
                if let Some(injector) = latency_injector {
                    app = registry.build_router_with_injectors(injector, failure_injector);
                } else {
                    app = registry.build_router();
                }

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
    // Admin: fetch last validation error
    .route("/__mockforge/validation/last_error", get(get_last_error))
    .route("/__mockforge/validation/history", get(get_error_history))
    // Admin: download config and overrides YAML
    .route("/__mockforge/config.yaml", get(download_config_yaml))
    .route("/__mockforge/validation/patch.yaml", get(download_overrides_yaml))
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
    start_with_latency(port, spec_path, options, None).await
}

/// Start HTTP server with latency injection support
pub async fn start_with_latency(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let latency_injector = latency_profile.map(|profile| {
        LatencyInjector::new(profile, Default::default())
    });

    let app = build_router_with_latency(spec_path, options, latency_injector).await;
    serve_router(port, app).await
}

/// Start HTTP server with both latency and failure injection support
pub async fn start_with_injectors(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    latency_profile: Option<LatencyProfile>,
    failure_injector: Option<mockforge_core::FailureInjector>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let latency_injector = latency_profile.map(|profile| {
        LatencyInjector::new(profile, Default::default())
    });

    let app = build_router_with_injectors(spec_path, options, latency_injector, failure_injector).await;
    serve_router(port, app).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationSettings {
    mode: Option<String>,
    aggregate_errors: Option<bool>,
    validate_responses: Option<bool>,
    overrides: Option<serde_json::Map<String, serde_json::Value>>,
    config_path: Option<String>,
}

async fn get_validation() -> Json<ValidationSettings> {
    let mode = std::env::var("MOCKFORGE_REQUEST_VALIDATION").ok();
    let aggregate_errors = std::env::var("MOCKFORGE_AGGREGATE_ERRORS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let validate_responses = std::env::var("MOCKFORGE_RESPONSE_VALIDATION")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let overrides = std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned());
    let config_path = std::env::var("MOCKFORGE_CONFIG_PATH").ok();
    Json(ValidationSettings {
        mode,
        aggregate_errors,
        validate_responses,
        overrides,
        config_path,
    })
}

async fn set_validation(Json(payload): Json<ValidationSettings>) -> Json<serde_json::Value> {
    if let Some(mode) = payload.mode {
        std::env::set_var("MOCKFORGE_REQUEST_VALIDATION", mode);
    }
    if let Some(agg) = payload.aggregate_errors {
        std::env::set_var("MOCKFORGE_AGGREGATE_ERRORS", if agg { "true" } else { "false" });
    }
    if let Some(resp) = payload.validate_responses {
        std::env::set_var("MOCKFORGE_RESPONSE_VALIDATION", if resp { "true" } else { "false" });
    }
    if let Some(map) = payload.overrides {
        let json = serde_json::Value::Object(map);
        if let Ok(s) = serde_json::to_string(&json) {
            std::env::set_var("MOCKFORGE_VALIDATION_OVERRIDES_JSON", s);
        }
    }
    // Optionally persist to config file if MOCKFORGE_CONFIG_PATH is set
    if let Ok(cfg_path) = std::env::var("MOCKFORGE_CONFIG_PATH") {
        if let Ok(mut cfg) = load_config(&cfg_path).await {
            if let Ok(mode) = std::env::var("MOCKFORGE_REQUEST_VALIDATION") {
                cfg.http.request_validation = mode;
            }
            if let Ok(agg) = std::env::var("MOCKFORGE_AGGREGATE_ERRORS") {
                cfg.http.aggregate_validation_errors =
                    agg == "1" || agg.eq_ignore_ascii_case("true");
            }
            if let Ok(rv) = std::env::var("MOCKFORGE_RESPONSE_VALIDATION") {
                cfg.http.validate_responses = rv == "1" || rv.eq_ignore_ascii_case("true");
            }
            if let Ok(over_json) = std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&over_json) {
                    if let Some(obj) = val.as_object() {
                        cfg.http.validation_overrides.clear();
                        for (k, v) in obj {
                            if let Some(s) = v.as_str() {
                                cfg.http.validation_overrides.insert(k.clone(), s.to_string());
                            }
                        }
                    }
                }
            }
            let _ = save_config(&cfg_path, &cfg).await;
        }
    }
    Json(serde_json::json!({"status":"ok"}))
}

async fn get_last_error() -> Json<serde_json::Value> {
    if let Some(err) = get_last_validation_error() {
        Json(err)
    } else {
        Json(serde_json::json!({"error":"none"}))
    }
}

async fn get_error_history() -> Json<serde_json::Value> {
    let items = get_validation_errors();
    Json(serde_json::json!({"errors": items}))
}

async fn download_config_yaml() -> axum::response::Response {
    if let Ok(path) = std::env::var("MOCKFORGE_CONFIG_PATH") {
        if let Ok(cfg) = load_config(&path).await {
            if let Ok(yaml) = serde_yaml::to_string(&cfg) {
                return axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .header(axum::http::header::CONTENT_TYPE, "application/x-yaml")
                    .header(
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=mockforge.config.yaml",
                    )
                    .body(axum::body::Body::from(yaml))
                    .unwrap();
            }
        }
    }
    axum::response::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(axum::body::Body::from("Config not available"))
        .unwrap()
}

async fn download_overrides_yaml() -> axum::response::Response {
    // Compose YAML snippet with validation settings + overrides
    let mode = std::env::var("MOCKFORGE_REQUEST_VALIDATION").unwrap_or_else(|_| "enforce".into());
    let agg = std::env::var("MOCKFORGE_AGGREGATE_ERRORS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    let resp = std::env::var("MOCKFORGE_RESPONSE_VALIDATION")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let overrides = std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let mut y = String::new();
    use std::fmt::Write as _;
    let _ = writeln!(&mut y, "http:");
    let _ = writeln!(&mut y, "  request_validation: \"{}\"", mode);
    let _ =
        writeln!(&mut y, "  aggregate_validation_errors: {}", if agg { "true" } else { "false" });
    let _ = writeln!(&mut y, "  validate_responses: {}", if resp { "true" } else { "false" });
    let _ = writeln!(&mut y, "  validation_overrides:");
    if let Some(map) = overrides.as_object() {
        for (k, v) in map {
            let mode = v.as_str().unwrap_or("enforce");
            let _ = writeln!(&mut y, "    \"{}\": \"{}\"", k, mode);
        }
    }
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/x-yaml")
        .header(
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=validation.overrides.yaml",
        )
        .body(axum::body::Body::from(y))
        .unwrap()
}

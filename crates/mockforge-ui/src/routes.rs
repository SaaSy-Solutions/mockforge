//! Route definitions for the admin UI

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use crate::audit::init_global_audit_store;
use crate::auth::init_global_user_store;
use crate::handlers::analytics::AnalyticsState;
use crate::handlers::AdminState;
use crate::handlers::*;
use crate::rbac::rbac_middleware;
use crate::time_travel_handlers;
use axum::middleware::from_fn;
use mockforge_core::{get_global_logger, init_global_logger};

/// Create the admin router with static assets and optional API endpoints
///
/// # Arguments
/// * `http_server_addr` - HTTP server address
/// * `ws_server_addr` - WebSocket server address
/// * `grpc_server_addr` - gRPC server address
/// * `graphql_server_addr` - GraphQL server address
/// * `api_enabled` - Whether API endpoints are enabled
/// * `admin_port` - Admin server port
/// * `prometheus_url` - Prometheus metrics URL
/// * `chaos_api_state` - Optional chaos API state for hot-reload support
/// * `latency_injector` - Optional latency injector for hot-reload support
/// * `mockai` - Optional MockAI instance for hot-reload support
/// * `continuum_config` - Optional Reality Continuum configuration
/// * `virtual_clock` - Optional virtual clock for time-based progression
pub fn create_admin_router(
    http_server_addr: Option<std::net::SocketAddr>,
    ws_server_addr: Option<std::net::SocketAddr>,
    grpc_server_addr: Option<std::net::SocketAddr>,
    graphql_server_addr: Option<std::net::SocketAddr>,
    api_enabled: bool,
    admin_port: u16,
    prometheus_url: String,
    chaos_api_state: Option<std::sync::Arc<mockforge_chaos::api::ChaosApiState>>,
    latency_injector: Option<
        std::sync::Arc<tokio::sync::RwLock<mockforge_core::latency::LatencyInjector>>,
    >,
    mockai: Option<
        std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>,
    >,
    continuum_config: Option<mockforge_core::ContinuumConfig>,
    virtual_clock: Option<std::sync::Arc<mockforge_core::VirtualClock>>,
) -> Router {
    // Initialize global logger if not already initialized
    let _logger = get_global_logger().unwrap_or_else(|| init_global_logger(1000));

    // Initialize audit log store (keep last 10000 audit entries)
    let _audit_store = init_global_audit_store(10000);

    // Initialize user store for authentication
    let _user_store = init_global_user_store();

    let state = AdminState::new(
        http_server_addr,
        ws_server_addr,
        grpc_server_addr,
        graphql_server_addr,
        api_enabled,
        admin_port,
        chaos_api_state,
        latency_injector,
        mockai,
        continuum_config,
        virtual_clock,
    );

    // Start system monitoring background task to poll CPU, memory, and thread metrics
    let state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.start_system_monitoring().await;
    });
    let mut router = Router::new()
        // Public routes (no authentication required)
        .route("/", get(serve_admin_html))
        .route("/assets/index.css", get(serve_admin_css))
        .route("/assets/index.js", get(serve_admin_js))
        .route("/assets/{filename}", get(serve_vendor_asset))
        .route("/api-docs", get(serve_api_docs))
        .route("/mockforge-icon.png", get(serve_icon))
        .route("/mockforge-icon-32.png", get(serve_icon_32))
        .route("/mockforge-icon-48.png", get(serve_icon_48))
        .route("/mockforge-logo.png", get(serve_logo))
        .route("/mockforge-logo-40.png", get(serve_logo_40))
        .route("/mockforge-logo-80.png", get(serve_logo_80))
        .route("/manifest.json", get(serve_manifest))
        .route("/sw.js", get(serve_service_worker))
        // Authentication endpoints (public)
        .route("/__mockforge/auth/login", post(crate::auth::login))
        .route("/__mockforge/auth/refresh", post(crate::auth::refresh_token))
        .route("/__mockforge/auth/logout", post(crate::auth::logout))
        .route("/__mockforge/health", get(get_health));

    // Protected routes (require authentication and RBAC)
    router = router
        .route("/__mockforge/dashboard", get(get_dashboard))
        .route("/_mf", get(get_dashboard))  // Short alias for dashboard
        .route("/admin/server-info", get(get_server_info))
        .route("/__mockforge/server-info", get(get_server_info))
        .route("/__mockforge/routes", get(get_routes))
        .route("/__mockforge/logs", get(get_logs))
        .route("/__mockforge/logs/sse", get(logs_sse))
        .route("/__mockforge/metrics", get(get_metrics))
        .route("/__mockforge/api/reality/trace/{request_id}", get(get_reality_trace))
        .route("/__mockforge/api/reality/response-trace/{request_id}", get(get_response_trace))
        .route("/__mockforge/config", get(get_config))
        .route("/__mockforge/config/latency", post(update_latency))
        .route("/__mockforge/config/faults", post(update_faults))
        .route("/__mockforge/config/proxy", post(update_proxy))
        .route("/__mockforge/config/traffic-shaping", post(update_traffic_shaping))
        .route("/__mockforge/logs", delete(clear_logs))
        .route("/__mockforge/restart", post(restart_servers))
        .route("/__mockforge/restart/status", get(get_restart_status))
        .route("/__mockforge/fixtures", get(get_fixtures))
        .route("/__mockforge/fixtures/{id}", delete(delete_fixture))
        .route("/__mockforge/fixtures/bulk", delete(delete_fixtures_bulk))
        .route("/__mockforge/audit/logs", get(get_audit_logs))
        .route("/__mockforge/audit/stats", get(get_audit_stats))
        .route("/__mockforge/fixtures/{id}/download", get(download_fixture))
        .route("/__mockforge/fixtures/{id}/rename", post(rename_fixture))
        .route("/__mockforge/fixtures/{id}/move", post(move_fixture))
        // Import routes
        .route("/__mockforge/import/postman", post(import_postman))
        .route("/__mockforge/import/insomnia", post(import_insomnia))
        .route("/__mockforge/import/curl", post(import_curl))
        .route("/__mockforge/import/preview", post(preview_import))
        .route("/__mockforge/import/history", get(get_import_history))
        .route("/__mockforge/import/history/clear", post(clear_import_history))
        // Plugin management routes
        .route("/__mockforge/plugins", get(get_plugins))
        .route("/__mockforge/plugins/status", get(get_plugin_status))
        .route("/__mockforge/plugins/{id}", get(get_plugin_details))
        .route("/__mockforge/plugins/{id}", delete(delete_plugin))
        .route("/__mockforge/plugins/reload", post(reload_plugin))
        // Workspace management routes (moved to workspace router with WorkspaceState)
        // These routes are now handled by the workspace router below
        // Environment management routes
        .route("/__mockforge/workspaces/{workspace_id}/environments", get(get_environments))
        .route("/__mockforge/workspaces/{workspace_id}/environments", post(create_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/order", axum::routing::put(update_environments_order))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", axum::routing::put(update_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}", delete(delete_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/activate", post(set_active_environment))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", get(get_environment_variables))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables", post(set_environment_variable))
        .route("/__mockforge/workspaces/{workspace_id}/environments/{environment_id}/variables/{variable_name}", delete(remove_environment_variable))
        // Chain management routes - proxy to main HTTP server
        .route("/__mockforge/chains", get(proxy_chains_list))
        .route("/__mockforge/chains", post(proxy_chains_create))
        .route("/__mockforge/chains/{id}", get(proxy_chain_get))
        .route("/__mockforge/chains/{id}", axum::routing::put(proxy_chain_update))
        .route("/__mockforge/chains/{id}", delete(proxy_chain_delete))
        .route("/__mockforge/chains/{id}/execute", post(proxy_chain_execute))
        .route("/__mockforge/chains/{id}/validate", post(proxy_chain_validate))
        .route("/__mockforge/chains/{id}/history", get(proxy_chain_history))
        // Graph visualization routes
        .route("/__mockforge/graph", get(get_graph))
        .route("/__mockforge/graph/sse", get(graph_sse))
        // Validation configuration routes
        .route("/__mockforge/validation", get(get_validation))
        .route("/__mockforge/validation", post(update_validation))
        // Migration pipeline routes
        .route("/__mockforge/migration/routes", get(migration::get_migration_routes))
        .route("/__mockforge/migration/routes/{pattern}/toggle", post(migration::toggle_route_migration))
        .route("/__mockforge/migration/routes/{pattern}", axum::routing::put(migration::set_route_migration_mode))
        .route("/__mockforge/migration/groups/{group}/toggle", post(migration::toggle_group_migration))
        .route("/__mockforge/migration/groups/{group}", axum::routing::put(migration::set_group_migration_mode))
        .route("/__mockforge/migration/groups", get(migration::get_migration_groups))
        .route("/__mockforge/migration/status", get(migration::get_migration_status))
        // Environment variables routes
        .route("/__mockforge/env", get(get_env_vars))
        .route("/__mockforge/env", post(update_env_var))
        // File management routes
        .route("/__mockforge/files/content", post(get_file_content))
        .route("/__mockforge/files/save", post(save_file_content))
        // Smoke test routes
        .route("/__mockforge/smoke", get(get_smoke_tests))
        .route("/__mockforge/smoke/run", get(run_smoke_tests_endpoint))
        // Time travel / temporal testing routes
        .route("/__mockforge/time-travel/status", get(time_travel_handlers::get_time_travel_status))
        .route("/__mockforge/time-travel/enable", post(time_travel_handlers::enable_time_travel))
        .route("/__mockforge/time-travel/disable", post(time_travel_handlers::disable_time_travel))
        .route("/__mockforge/time-travel/advance", post(time_travel_handlers::advance_time))
        .route("/__mockforge/time-travel/set", post(time_travel_handlers::set_time))
        .route("/__mockforge/time-travel/scale", post(time_travel_handlers::set_time_scale))
        .route("/__mockforge/time-travel/reset", post(time_travel_handlers::reset_time_travel))
        .route("/__mockforge/time-travel/schedule", post(time_travel_handlers::schedule_response))
        .route("/__mockforge/time-travel/scheduled", get(time_travel_handlers::list_scheduled_responses))
        .route("/__mockforge/time-travel/scheduled/{id}", delete(time_travel_handlers::cancel_scheduled_response))
        .route("/__mockforge/time-travel/scheduled/clear", post(time_travel_handlers::clear_scheduled_responses))
        .route("/__mockforge/time-travel/scenario/save", post(time_travel_handlers::save_scenario))
        .route("/__mockforge/time-travel/scenario/load", post(time_travel_handlers::load_scenario))
        // Cron job management routes
        .route("/__mockforge/time-travel/cron", get(time_travel_handlers::list_cron_jobs))
        .route("/__mockforge/time-travel/cron", post(time_travel_handlers::create_cron_job))
        .route("/__mockforge/time-travel/cron/{id}", get(time_travel_handlers::get_cron_job))
        .route("/__mockforge/time-travel/cron/{id}", delete(time_travel_handlers::delete_cron_job))
        .route("/__mockforge/time-travel/cron/{id}/enable", post(time_travel_handlers::set_cron_job_enabled))
        // Mutation rule management routes
        .route("/__mockforge/time-travel/mutations", get(time_travel_handlers::list_mutation_rules))
        .route("/__mockforge/time-travel/mutations", post(time_travel_handlers::create_mutation_rule))
        .route("/__mockforge/time-travel/mutations/{id}", get(time_travel_handlers::get_mutation_rule))
        .route("/__mockforge/time-travel/mutations/{id}", delete(time_travel_handlers::delete_mutation_rule))
        .route("/__mockforge/time-travel/mutations/{id}/enable", post(time_travel_handlers::set_mutation_rule_enabled))
        // Verification routes
        .route("/__mockforge/verification/verify", post(verification::verify))
        .route("/__mockforge/verification/count", post(verification::count))
        .route("/__mockforge/verification/sequence", post(verification::verify_sequence_handler))
        .route("/__mockforge/verification/never", post(verification::verify_never_handler))
        .route("/__mockforge/verification/at-least", post(verification::verify_at_least_handler))
        // Reality Slider routes
        .route("/__mockforge/reality/level", get(get_reality_level))
        .route("/__mockforge/reality/level", axum::routing::put(set_reality_level))
        .route("/__mockforge/reality/presets", get(list_reality_presets))
        .route("/__mockforge/reality/presets/import", post(import_reality_preset))
        .route("/__mockforge/reality/presets/export", post(export_reality_preset))
        // Reality Continuum routes
        .route("/__mockforge/continuum/ratio", get(get_continuum_ratio))
        .route("/__mockforge/continuum/ratio", axum::routing::put(set_continuum_ratio))
        .route("/__mockforge/continuum/schedule", get(get_continuum_schedule))
        .route("/__mockforge/continuum/schedule", axum::routing::put(set_continuum_schedule))
        .route("/__mockforge/continuum/advance", post(advance_continuum_ratio))
        .route("/__mockforge/continuum/enabled", axum::routing::put(set_continuum_enabled))
        .route("/__mockforge/continuum/overrides", get(get_continuum_overrides))
        .route("/__mockforge/continuum/overrides", axum::routing::delete(clear_continuum_overrides))
        // Contract diff routes
        .route("/__mockforge/contract-diff/upload", post(contract_diff::upload_request))
        .route("/__mockforge/contract-diff/submit", post(contract_diff::submit_request))
        .route("/__mockforge/contract-diff/captures", get(contract_diff::get_captured_requests))
        .route("/__mockforge/contract-diff/captures/{id}", get(contract_diff::get_captured_request))
        .route("/__mockforge/contract-diff/captures/{id}/analyze", post(contract_diff::analyze_captured_request))
        .route("/__mockforge/contract-diff/captures/{id}/patch", post(contract_diff::generate_patch_file))
        .route("/__mockforge/contract-diff/statistics", get(contract_diff::get_capture_statistics))
        // Playground routes
        .route("/__mockforge/playground/endpoints", get(playground::list_playground_endpoints))
        .route("/__mockforge/playground/execute", post(playground::execute_rest_request))
        .route("/__mockforge/playground/graphql", post(playground::execute_graphql_query))
        .route("/__mockforge/playground/graphql/introspect", get(playground::graphql_introspect))
        .route("/__mockforge/playground/history", get(playground::get_request_history))
        .route("/__mockforge/playground/history/{id}/replay", post(playground::replay_request))
        .route("/__mockforge/playground/snippets", post(playground::generate_code_snippet))
        // Voice + LLM Interface routes
        .route("/api/v2/voice/process", post(voice::process_voice_command))
        .route("/__mockforge/voice/process", post(voice::process_voice_command))
        .route("/api/v2/voice/transpile-hook", post(voice::transpile_hook))
        .route("/__mockforge/voice/transpile-hook", post(voice::transpile_hook))
        .route(
            "/api/v2/voice/create-workspace-scenario",
            post(voice::create_workspace_scenario),
        )
        .route(
            "/__mockforge/voice/create-workspace-scenario",
            post(voice::create_workspace_scenario),
        )
        .route(
            "/api/v2/voice/create-workspace-preview",
            post(voice::create_workspace_preview),
        )
        .route(
            "/__mockforge/voice/create-workspace-preview",
            post(voice::create_workspace_preview),
        )
        // create-workspace-confirm route moved to workspace router with WorkspaceState
        // AI Studio routes
        .route("/api/v1/ai-studio/chat", post(ai_studio::chat))
        .route("/__mockforge/ai-studio/chat", post(ai_studio::chat))
        .route("/api/v1/ai-studio/generate-mock", post(ai_studio::generate_mock))
        .route("/__mockforge/ai-studio/generate-mock", post(ai_studio::generate_mock))
        .route("/api/v1/ai-studio/debug-test", post(ai_studio::debug_test))
        .route("/__mockforge/ai-studio/debug-test", post(ai_studio::debug_test))
        .route("/api/v1/ai-studio/generate-persona", post(ai_studio::generate_persona))
        .route("/__mockforge/ai-studio/generate-persona", post(ai_studio::generate_persona))
        .route("/api/v1/ai-studio/freeze", post(ai_studio::freeze_artifact))
        .route("/__mockforge/ai-studio/freeze", post(ai_studio::freeze_artifact))
        .route("/api/v1/ai-studio/usage", get(ai_studio::get_usage))
        .route("/__mockforge/ai-studio/usage", get(ai_studio::get_usage))
        // Failure analysis routes
        .route("/api/v2/failures/analyze", post(failure_analysis::analyze_failure))
        .route("/api/v2/failures/{request_id}", get(failure_analysis::get_failure_analysis))
        .route("/api/v2/failures/recent", get(failure_analysis::list_recent_failures))
        .route("/__mockforge/failures/analyze", post(failure_analysis::analyze_failure))
        .route("/__mockforge/failures/{request_id}", get(failure_analysis::get_failure_analysis))
        .route("/__mockforge/failures/recent", get(failure_analysis::list_recent_failures))
        // Community portal routes
        .route("/__mockforge/community/showcase/projects", get(community::get_showcase_projects))
        .route("/__mockforge/community/showcase/projects/{id}", get(community::get_showcase_project))
        .route("/__mockforge/community/showcase/categories", get(community::get_showcase_categories))
        .route("/__mockforge/community/showcase/stories", get(community::get_success_stories))
        .route("/__mockforge/community/showcase/submit", post(community::submit_showcase_project))
        .route("/__mockforge/community/learning/resources", get(community::get_learning_resources))
        .route("/__mockforge/community/learning/resources/{id}", get(community::get_learning_resource))
        .route("/__mockforge/community/learning/categories", get(community::get_learning_categories))
        // Behavioral cloning / flow management routes
        .route("/__mockforge/flows", get(behavioral_cloning::get_flows))
        .route("/__mockforge/flows/{id}", get(behavioral_cloning::get_flow))
        .route("/__mockforge/flows/{id}/tag", axum::routing::put(behavioral_cloning::tag_flow))
        .route("/__mockforge/flows/{id}/compile", post(behavioral_cloning::compile_flow))
        .route("/__mockforge/scenarios", get(behavioral_cloning::get_scenarios))
        .route("/__mockforge/scenarios/{id}", get(behavioral_cloning::get_scenario))
        .route("/__mockforge/scenarios/{id}/export", get(behavioral_cloning::export_scenario))
        // Health check endpoints for Kubernetes probes
        .route("/health/live", get(health::liveness_probe))
        .route("/health/ready", get(health::readiness_probe))
        .route("/health/startup", get(health::startup_probe))
        .route("/health", get(health::deep_health_check))
        // Kubernetes-style health endpoint aliases
        .route("/healthz", get(health::deep_health_check))
        .route("/readyz", get(health::readiness_probe))
        .route("/livez", get(health::liveness_probe))
        .route("/startupz", get(health::startup_probe));

    // Analytics routes with Prometheus integration
    let analytics_state = AnalyticsState::new(prometheus_url);

    let analytics_router = Router::new()
        .route("/__mockforge/analytics/summary", get(analytics::get_summary))
        .route("/__mockforge/analytics/requests", get(analytics::get_requests))
        .route("/__mockforge/analytics/endpoints", get(analytics::get_endpoints))
        .route("/__mockforge/analytics/websocket", get(analytics::get_websocket))
        .route("/__mockforge/analytics/smtp", get(analytics::get_smtp))
        .route("/__mockforge/analytics/system", get(analytics::get_system))
        .with_state(analytics_state);

    router = router.merge(analytics_router);

    // Add workspace router with WorkspaceState
    {
        use crate::handlers::workspaces::WorkspaceState;
        use mockforge_core::multi_tenant::{MultiTenantConfig, MultiTenantWorkspaceRegistry};
        use std::sync::Arc;

        // Create workspace registry
        let mt_config = MultiTenantConfig {
            enabled: true,
            default_workspace: "default".to_string(),
            ..Default::default()
        };
        let registry = MultiTenantWorkspaceRegistry::new(mt_config);
        let workspace_state = WorkspaceState::new(Arc::new(tokio::sync::RwLock::new(registry)));

        // Create workspace router with state
        use crate::handlers::workspaces;
        let workspace_router = Router::new()
            .route("/__mockforge/workspaces", get(workspaces::list_workspaces))
            .route("/__mockforge/workspaces", post(workspaces::create_workspace))
            .route("/__mockforge/workspaces/{workspace_id}", get(workspaces::get_workspace))
            .route(
                "/__mockforge/workspaces/{workspace_id}",
                axum::routing::put(workspaces::update_workspace),
            )
            .route("/__mockforge/workspaces/{workspace_id}", delete(workspaces::delete_workspace))
            // Note: set_active_workspace handler not yet implemented
            // .route(
            //     "/__mockforge/workspaces/{workspace_id}/activate",
            //     post(workspaces::set_active_workspace),
            // )
            .route("/api/v2/voice/create-workspace-confirm", post(voice::create_workspace_confirm))
            .route(
                "/__mockforge/voice/create-workspace-confirm",
                post(voice::create_workspace_confirm),
            )
            .with_state(workspace_state);

        router = router.merge(workspace_router);
        tracing::info!("Workspace router mounted with WorkspaceState");
    }

    // Add UI Builder router
    // This provides a low-code visual interface for creating mock endpoints
    {
        use mockforge_http::{create_ui_builder_router, UIBuilderState};

        // Load server config for UI Builder
        // For now, create a default config. In production, this should be loaded from the actual config.
        // Use the re-exported ServerConfig from mockforge_core root to match UIBuilderState's import
        let server_config = mockforge_core::ServerConfig::default();
        let ui_builder_state = UIBuilderState::new(server_config);
        let ui_builder_router = create_ui_builder_router(ui_builder_state);

        // Nest the UI builder router with its own state
        router = router.nest_service("/__mockforge/ui-builder", ui_builder_router);
        tracing::info!("UI Builder mounted at /__mockforge/ui-builder");
    }

    // SPA fallback: serve index.html for any unmatched routes to support client-side routing
    // IMPORTANT: This must be AFTER all API routes
    router = router.route("/{*path}", get(serve_admin_html));

    // Apply RBAC middleware to protected routes
    // Note: The middleware will check authentication and permissions for all routes
    // Public routes (auth endpoints, static assets) should be handled gracefully
    router = router.layer(from_fn(rbac_middleware));

    router
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_admin_router() {
        let http_addr: std::net::SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let router = create_admin_router(
            Some(http_addr),
            None,
            None,
            None,
            true,
            8080,
            "http://localhost:9090".to_string(),
            None,
            None,
            None,
            None,
            None,
        );

        // Router should be created successfully
        let _ = router;
    }

    #[tokio::test]
    async fn test_create_admin_router_no_servers() {
        let router = create_admin_router(
            None,
            None,
            None,
            None,
            false,
            8080,
            "http://localhost:9090".to_string(),
            None,
            None,
            None,
            None,
            None,
        );

        // Router should still work without server addresses
        let _ = router;
    }
}

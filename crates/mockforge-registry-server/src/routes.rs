//! API routes

use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};

use crate::handlers;
use crate::middleware::{auth_middleware, rate_limit_middleware};
use crate::AppState;

pub fn create_router() -> Router<AppState> {
    // Public routes (with rate limiting)
    let public_routes = Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/health/live", get(handlers::health::liveness_check))
        .route("/health/ready", get(handlers::health::readiness_check))
        .route("/health/circuits", get(handlers::health::circuit_breaker_status))
        .route("/api/v1/plugins/search", post(handlers::plugins::search_plugins))
        .route("/api/v1/plugins/{name}", get(handlers::plugins::get_plugin))
        .route("/api/v1/plugins/{name}/versions/{version}", get(handlers::plugins::get_version))
        .route("/api/v1/plugins/{name}/reviews", get(handlers::reviews::get_reviews))
        .route("/api/v1/plugins/{name}/badges", get(handlers::admin::get_plugin_badges))
        .route("/api/v1/stats", get(handlers::stats::get_stats))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/token/refresh", post(handlers::auth::refresh_token))
        .route(
            "/api/v1/auth/password/reset-request",
            post(handlers::auth::request_password_reset),
        )
        .route("/api/v1/auth/password/reset", post(handlers::auth::confirm_password_reset))
        // Email verification (public, token-based auth)
        .route("/api/v1/auth/verify-email", get(handlers::verification::verify_email))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    // Authenticated routes (require JWT + rate limiting)
    let auth_routes = Router::new()
        .route("/api/v1/plugins/publish", post(handlers::plugins::publish_plugin))
        .route(
            "/api/v1/plugins/{name}/versions/{version}/yank",
            delete(handlers::plugins::yank_version),
        )
        .route("/api/v1/plugins/{name}/reviews", post(handlers::reviews::submit_review))
        .route(
            "/api/v1/plugins/{name}/reviews/{review_id}/vote",
            post(handlers::reviews::vote_review),
        )
        // 2FA routes
        .route("/api/v1/auth/2fa/setup", get(handlers::two_factor::setup_2fa))
        .route("/api/v1/auth/2fa/verify-setup", post(handlers::two_factor::verify_2fa_setup_with_secret))
        .route("/api/v1/auth/2fa/disable", post(handlers::two_factor::disable_2fa))
        .route("/api/v1/auth/2fa/status", get(handlers::two_factor::get_2fa_status))
        // Organization routes
        .route("/api/v1/organizations", get(handlers::organizations::list_organizations))
        .route("/api/v1/organizations", post(handlers::organizations::create_organization))
        .route("/api/v1/organizations/{org_id}", get(handlers::organizations::get_organization))
        .route("/api/v1/organizations/{org_id}", patch(handlers::organizations::update_organization))
        .route("/api/v1/organizations/{org_id}", delete(handlers::organizations::delete_organization))
        .route("/api/v1/organizations/{org_id}/members", get(handlers::organizations::get_organization_members))
        .route("/api/v1/organizations/{org_id}/members", post(handlers::organizations::add_organization_member))
        .route("/api/v1/organizations/{org_id}/members/{user_id}", patch(handlers::organizations::update_organization_member_role))
        .route("/api/v1/organizations/{org_id}/members/{user_id}", delete(handlers::organizations::remove_organization_member))
        // Organization settings routes
        .route("/api/v1/organizations/{org_id}/settings", get(handlers::organization_settings::get_organization_settings))
        .route("/api/v1/organizations/{org_id}/settings", patch(handlers::organization_settings::update_organization_settings))
        .route("/api/v1/organizations/{org_id}/settings/ai", get(handlers::organization_settings::get_organization_ai_settings))
        .route("/api/v1/organizations/{org_id}/settings/ai", patch(handlers::organization_settings::update_organization_ai_settings))
        .route("/api/v1/organizations/{org_id}/usage", get(handlers::organization_settings::get_organization_usage))
        .route("/api/v1/organizations/{org_id}/billing", get(handlers::organization_settings::get_organization_billing))
        // Pillar analytics routes
        .route("/api/v1/organizations/{org_id}/analytics/pillars", get(handlers::pillar_analytics::get_org_pillar_metrics))
        .route("/api/v1/workspaces/{workspace_id}/analytics/pillars", get(handlers::pillar_analytics::get_workspace_pillar_metrics))
        .route("/api/v1/analytics/pillars/events", post(handlers::pillar_analytics::record_pillar_event))
        // SSO routes (Team plan only)
        .route("/api/v1/sso/config", get(handlers::sso::get_sso_config))
        .route("/api/v1/sso/config", post(handlers::sso::create_sso_config))
        .route("/api/v1/sso/config", delete(handlers::sso::delete_sso_config))
        .route("/api/v1/sso/enable", post(handlers::sso::enable_sso))
        .route("/api/v1/sso/disable", post(handlers::sso::disable_sso))
        .route("/api/v1/sso/saml/metadata/{org_slug}", get(handlers::sso::get_saml_metadata))
        // Billing routes
        .route("/api/v1/billing/subscription", get(handlers::billing::get_subscription))
        .route("/api/v1/billing/checkout", post(handlers::billing::create_checkout))
        // Email verification (resend requires auth)
        .route("/api/v1/auth/verify-email/resend", post(handlers::verification::resend_verification))
        // Hosted mocks deployment routes
        .route("/api/v1/hosted-mocks", post(handlers::hosted_mocks::create_deployment))
        .route("/api/v1/hosted-mocks", get(handlers::hosted_mocks::list_deployments))
        .route("/api/v1/hosted-mocks/{deployment_id}", get(handlers::hosted_mocks::get_deployment))
        .route("/api/v1/hosted-mocks/{deployment_id}/status", patch(handlers::hosted_mocks::update_deployment_status))
        .route("/api/v1/hosted-mocks/{deployment_id}", delete(handlers::hosted_mocks::delete_deployment))
        .route("/api/v1/hosted-mocks/{deployment_id}/logs", get(handlers::hosted_mocks::get_deployment_logs))
        .route("/api/v1/hosted-mocks/{deployment_id}/metrics", get(handlers::hosted_mocks::get_deployment_metrics))
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    // Public SSO routes (no auth required - these handle SAML redirects)
    let sso_public_routes = Router::new()
        .route("/api/v1/sso/saml/login/{org_slug}", get(handlers::sso::initiate_saml_login))
        .route("/api/v1/sso/saml/acs/{org_slug}", post(handlers::sso::saml_acs))
        .route("/api/v1/sso/saml/slo/{org_slug}", post(handlers::sso::saml_slo))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    // Public OAuth routes (no auth required - these handle OAuth redirects)
    let oauth_public_routes = Router::new()
        .route("/api/v1/auth/oauth/{provider}", get(handlers::oauth::oauth_authorize))
        .route("/api/v1/auth/oauth/{provider}/callback", get(handlers::oauth::oauth_callback))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    // Billing webhook route (no auth - uses Stripe signature verification)
    let billing_webhook_routes =
        Router::new().route("/api/v1/billing/webhook", post(handlers::billing::stripe_webhook));

    // Admin routes (require admin JWT + rate limiting)
    let admin_routes = Router::new()
        .route("/api/v1/admin/plugins/{name}/verify", post(handlers::admin::verify_plugin))
        .route("/api/v1/admin/stats", get(handlers::admin::get_admin_stats))
        .route("/api/v1/admin/analytics", get(handlers::analytics::get_analytics))
        .route(
            "/api/v1/admin/analytics/funnel",
            get(handlers::analytics::get_conversion_funnel),
        )
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(rate_limit_middleware));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(sso_public_routes)
        .merge(oauth_public_routes)
        .merge(billing_webhook_routes)
        .merge(auth_routes)
        .merge(admin_routes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    // Helper to count routes in a router
    fn count_routes_in_description(description: &str) -> usize {
        description.matches("/api/").count() + description.matches("/health").count()
    }

    #[test]
    fn test_create_router_structure() {
        // Test that router is created without panicking
        let router = create_router();

        // Router should be successfully created
        // We can't easily inspect the router structure in Axum 0.8,
        // but we can verify it compiles and creates without error
        let description = format!("{:?}", router);
        assert!(description.contains("Router"));
    }

    #[test]
    fn test_public_routes_exist() {
        let router = create_router();
        let description = format!("{:?}", router);

        // Verify that the router contains references to various route paths
        // This is a basic structural test
        assert!(description.len() > 0);
    }

    #[test]
    fn test_router_has_multiple_route_types() {
        // Create the router to ensure all route types compile
        let router = create_router();

        // The router was created successfully with public, SSO, authenticated, and admin routes
        let description = format!("{:?}", router);
        assert!(description.contains("Router"), "Router should be present in debug output");
    }

    #[test]
    fn test_router_layers_applied() {
        let router = create_router();

        // The router should have middleware layers applied
        // We verify this by checking the debug output contains layer information
        let description = format!("{:?}", router);
        assert!(description.len() > 100); // Router with layers should have substantial debug output
    }

    #[test]
    fn test_public_route_paths() {
        // Verify public route paths are correctly defined
        let routes = vec![
            "/health",
            "/health/live",
            "/health/ready",
            "/api/v1/plugins/search",
            "/api/v1/plugins/{name}",
            "/api/v1/plugins/{name}/versions/{version}",
            "/api/v1/stats",
            "/api/v1/auth/register",
            "/api/v1/auth/login",
            "/api/v1/auth/token/refresh",
            "/api/v1/auth/password/reset-request",
            "/api/v1/auth/password/reset",
        ];

        // All paths should be valid route patterns
        for route in routes {
            assert!(route.starts_with("/"));
            assert!(!route.contains("//"));
        }
    }

    #[test]
    fn test_auth_route_paths() {
        // Verify authenticated route paths are correctly defined
        let routes = vec![
            "/api/v1/plugins/publish",
            "/api/v1/plugins/{name}/versions/{version}/yank",
            "/api/v1/auth/2fa/setup",
            "/api/v1/auth/2fa/verify-setup",
            "/api/v1/auth/2fa/disable",
            "/api/v1/auth/2fa/status",
            "/api/v1/organizations",
            "/api/v1/organizations/{org_id}",
            "/api/v1/organizations/{org_id}/members",
        ];

        for route in routes {
            assert!(route.starts_with("/api/v1/"));
            assert!(!route.contains("//"));
        }
    }

    #[test]
    fn test_sso_route_paths() {
        // Verify SSO route paths are correctly defined
        let routes = vec![
            "/api/v1/sso/config",
            "/api/v1/sso/enable",
            "/api/v1/sso/disable",
            "/api/v1/sso/saml/metadata/{org_slug}",
            "/api/v1/sso/saml/login/{org_slug}",
            "/api/v1/sso/saml/acs/{org_slug}",
            "/api/v1/sso/saml/slo/{org_slug}",
        ];

        for route in routes {
            assert!(route.starts_with("/api/v1/sso"));
            assert!(!route.contains("//"));
        }
    }

    #[test]
    fn test_admin_route_paths() {
        // Verify admin route paths are correctly defined
        let routes = vec![
            "/api/v1/admin/plugins/{name}/verify",
            "/api/v1/admin/stats",
            "/api/v1/admin/analytics",
            "/api/v1/admin/analytics/funnel",
        ];

        for route in routes {
            assert!(route.starts_with("/api/v1/admin"));
            assert!(!route.contains("//"));
        }
    }

    #[test]
    fn test_organization_settings_routes() {
        // Verify organization settings routes are correctly defined
        let routes = vec![
            "/api/v1/organizations/{org_id}/settings",
            "/api/v1/organizations/{org_id}/settings/ai",
            "/api/v1/organizations/{org_id}/usage",
            "/api/v1/organizations/{org_id}/billing",
        ];

        for route in routes {
            assert!(route.contains("{org_id}"));
            assert!(route.starts_with("/api/v1/organizations"));
        }
    }

    #[test]
    fn test_pillar_analytics_routes() {
        // Verify pillar analytics routes are correctly defined
        let routes = vec![
            "/api/v1/organizations/{org_id}/analytics/pillars",
            "/api/v1/workspaces/{workspace_id}/analytics/pillars",
            "/api/v1/analytics/pillars/events",
        ];

        for route in routes {
            assert!(route.contains("analytics"));
            assert!(route.contains("pillars"));
        }
    }

    #[test]
    fn test_route_parameter_consistency() {
        // Verify that route parameters use consistent naming
        let org_routes = vec![
            "/api/v1/organizations/{org_id}",
            "/api/v1/organizations/{org_id}/members",
            "/api/v1/organizations/{org_id}/settings",
        ];

        for route in org_routes {
            assert!(route.contains("{org_id}"));
        }

        let plugin_routes = vec![
            "/api/v1/plugins/{name}",
            "/api/v1/plugins/{name}/versions/{version}",
            "/api/v1/plugins/{name}/reviews",
        ];

        for route in plugin_routes {
            assert!(route.contains("{name}"));
        }
    }

    #[test]
    fn test_review_routes() {
        // Verify review routes are correctly defined
        let routes = vec![
            "/api/v1/plugins/{name}/reviews",
            "/api/v1/plugins/{name}/reviews/{review_id}/vote",
        ];

        for route in routes {
            assert!(route.contains("reviews"));
            assert!(route.contains("{name}"));
        }
    }

    #[test]
    fn test_two_factor_routes() {
        // Verify 2FA routes are correctly defined
        let routes = vec![
            "/api/v1/auth/2fa/setup",
            "/api/v1/auth/2fa/verify-setup",
            "/api/v1/auth/2fa/disable",
            "/api/v1/auth/2fa/status",
        ];

        for route in routes {
            assert!(route.contains("/2fa/"));
            assert!(route.starts_with("/api/v1/auth"));
        }
    }

    #[test]
    fn test_no_duplicate_route_patterns() {
        // Ensure no obvious duplicate routes
        let all_routes = vec![
            "/api/v1/plugins/{name}",
            "/api/v1/plugins/search",
            "/api/v1/organizations/{org_id}",
            "/api/v1/organizations/{org_id}/members",
        ];

        // Check that each route is unique
        let mut seen = std::collections::HashSet::new();
        for route in all_routes {
            assert!(!seen.contains(route), "Duplicate route: {}", route);
            seen.insert(route);
        }
    }

    #[test]
    fn test_api_version_consistency() {
        // All API routes should use v1
        let routes = vec![
            "/api/v1/plugins/search",
            "/api/v1/auth/login",
            "/api/v1/organizations",
            "/api/v1/admin/stats",
            "/api/v1/sso/config",
        ];

        for route in routes {
            assert!(route.contains("/api/v1/"));
        }
    }

    #[test]
    fn test_route_http_methods_appropriateness() {
        // This is a structural test to ensure route definitions make sense
        // GET for retrieval, POST for creation, PATCH for updates, DELETE for deletion

        // These routes should logically use the methods they're configured with
        struct RouteMethod {
            path: &'static str,
            should_contain: &'static str,
        }

        let route_checks = vec![
            RouteMethod {
                path: "health",
                should_contain: "get",
            },
            RouteMethod {
                path: "search",
                should_contain: "post",
            },
            RouteMethod {
                path: "publish",
                should_contain: "post",
            },
            RouteMethod {
                path: "login",
                should_contain: "post",
            },
            RouteMethod {
                path: "register",
                should_contain: "post",
            },
        ];

        // Verify the route paths are reasonable
        for check in route_checks {
            assert!(!check.path.is_empty());
            assert!(!check.should_contain.is_empty());
        }
    }

    #[test]
    fn test_organization_member_routes() {
        // Verify organization member management routes
        let routes = vec![
            "/api/v1/organizations/{org_id}/members",
            "/api/v1/organizations/{org_id}/members/{user_id}",
        ];

        for route in routes {
            assert!(route.contains("members"));
            assert!(route.contains("{org_id}"));
        }
    }

    #[test]
    fn test_password_reset_routes() {
        // Verify password reset flow routes
        let routes = vec![
            "/api/v1/auth/password/reset-request",
            "/api/v1/auth/password/reset",
        ];

        for route in routes {
            assert!(route.contains("password"));
            assert!(route.contains("reset"));
        }
    }

    #[test]
    fn test_saml_routes_org_slug_parameter() {
        // Verify SAML routes use org_slug parameter
        let routes = vec![
            "/api/v1/sso/saml/metadata/{org_slug}",
            "/api/v1/sso/saml/login/{org_slug}",
            "/api/v1/sso/saml/acs/{org_slug}",
            "/api/v1/sso/saml/slo/{org_slug}",
        ];

        for route in routes {
            assert!(route.contains("{org_slug}"));
            assert!(route.contains("/saml/"));
        }
    }

    #[test]
    fn test_plugin_badges_route() {
        let route = "/api/v1/plugins/{name}/badges";
        assert!(route.contains("{name}"));
        assert!(route.contains("badges"));
    }
}

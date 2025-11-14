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
        .route("/api/v1/plugins/search", post(handlers::plugins::search_plugins))
        .route("/api/v1/plugins/:name", get(handlers::plugins::get_plugin))
        .route("/api/v1/plugins/:name/versions/:version", get(handlers::plugins::get_version))
        .route("/api/v1/plugins/:name/reviews", get(handlers::reviews::get_reviews))
        .route("/api/v1/plugins/:name/badges", get(handlers::admin::get_plugin_badges))
        .route("/api/v1/stats", get(handlers::stats::get_stats))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/token/refresh", post(handlers::auth::refresh_token))
        .route("/api/v1/auth/password/reset-request", post(handlers::auth::request_password_reset))
        .route("/api/v1/auth/password/reset", post(handlers::auth::confirm_password_reset))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Authenticated routes (require JWT + rate limiting)
    let auth_routes = Router::new()
        .route("/api/v1/plugins/publish", post(handlers::plugins::publish_plugin))
        .route(
            "/api/v1/plugins/:name/versions/:version/yank",
            delete(handlers::plugins::yank_version),
        )
        .route("/api/v1/plugins/:name/reviews", post(handlers::reviews::submit_review))
        .route(
            "/api/v1/plugins/:name/reviews/:review_id/vote",
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
        .route("/api/v1/organizations/:org_id", get(handlers::organizations::get_organization))
        .route("/api/v1/organizations/:org_id", patch(handlers::organizations::update_organization))
        .route("/api/v1/organizations/:org_id", delete(handlers::organizations::delete_organization))
        .route("/api/v1/organizations/:org_id/members", get(handlers::organizations::get_organization_members))
        .route("/api/v1/organizations/:org_id/members", post(handlers::organizations::add_organization_member))
        .route("/api/v1/organizations/:org_id/members/:user_id", patch(handlers::organizations::update_organization_member_role))
        .route("/api/v1/organizations/:org_id/members/:user_id", delete(handlers::organizations::remove_organization_member))
        // Organization settings routes
        .route("/api/v1/organizations/:org_id/settings", get(handlers::organization_settings::get_organization_settings))
        .route("/api/v1/organizations/:org_id/settings", patch(handlers::organization_settings::update_organization_settings))
        .route("/api/v1/organizations/:org_id/usage", get(handlers::organization_settings::get_organization_usage))
        .route("/api/v1/organizations/:org_id/billing", get(handlers::organization_settings::get_organization_billing))
        // SSO routes (Team plan only)
        .route("/api/v1/sso/config", get(handlers::sso::get_sso_config))
        .route("/api/v1/sso/config", post(handlers::sso::create_sso_config))
        .route("/api/v1/sso/config", delete(handlers::sso::delete_sso_config))
        .route("/api/v1/sso/enable", post(handlers::sso::enable_sso))
        .route("/api/v1/sso/disable", post(handlers::sso::disable_sso))
        .route("/api/v1/sso/saml/metadata/:org_slug", get(handlers::sso::get_saml_metadata))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Public SSO routes (no auth required - these handle SAML redirects)
    let sso_public_routes = Router::new()
        .route("/api/v1/sso/saml/login/:org_slug", get(handlers::sso::initiate_saml_login))
        .route("/api/v1/sso/saml/acs/:org_slug", post(handlers::sso::saml_acs))
        .route("/api/v1/sso/saml/slo/:org_slug", post(handlers::sso::saml_slo))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Admin routes (require admin JWT + rate limiting)
    let admin_routes = Router::new()
        .route("/api/v1/admin/plugins/:name/verify", post(handlers::admin::verify_plugin))
        .route("/api/v1/admin/stats", get(handlers::admin::get_admin_stats))
        .route("/api/v1/admin/analytics", get(handlers::analytics::get_analytics))
        .route("/api/v1/admin/analytics/funnel", get(handlers::analytics::get_conversion_funnel))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(sso_public_routes)
        .merge(auth_routes)
        .merge(admin_routes)
}

//! API routes

use axum::{
    middleware,
    routing::{get, post, delete},
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
        .layer(middleware::from_fn(rate_limit_middleware));

    // Authenticated routes (require JWT + rate limiting)
    let auth_routes = Router::new()
        .route("/api/v1/plugins/publish", post(handlers::plugins::publish_plugin))
        .route("/api/v1/plugins/:name/versions/:version/yank", delete(handlers::plugins::yank_version))
        .route("/api/v1/plugins/:name/reviews", post(handlers::reviews::submit_review))
        .route("/api/v1/plugins/:name/reviews/:review_id/vote", post(handlers::reviews::vote_review))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Admin routes (require admin JWT + rate limiting)
    let admin_routes = Router::new()
        .route("/api/v1/admin/plugins/:name/verify", post(handlers::admin::verify_plugin))
        .route("/api/v1/admin/stats", get(handlers::admin::get_admin_stats))
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(auth_routes)
        .merge(admin_routes)
}

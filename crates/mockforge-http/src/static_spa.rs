//! Serves a built Vite SPA from admin-ui/dist at `/__admin/*` with SPA fallback.
use axum::Router;
use tower_http::services::ServeDir;

pub fn service() -> Router {
    Router::new().fallback_service(ServeDir::new("admin-ui/dist"))
}

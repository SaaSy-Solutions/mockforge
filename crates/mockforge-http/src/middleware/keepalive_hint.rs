//! Connection: keep-alive hint middleware.
//!
//! Adds explicit `Connection: keep-alive` and `Keep-Alive: timeout=N, max=M`
//! response headers when enabled via the `MOCKFORGE_HTTP_KEEPALIVE_HINT`
//! environment variable (or the `--http-keepalive-hint` CLI flag, when wired).
//!
//! This is a workaround for proxies that:
//! - Speak HTTP/1.0 upstream by default (hyper closes the connection after one
//!   response unless the request carried `Connection: keep-alive`).
//! - Cache the keep-alive policy from the response headers rather than the
//!   HTTP version. F5/Avi/HAProxy in some configurations look at the `Keep-
//!   Alive` response header to decide whether to pool the upstream socket.
//!
//! Issue #79 — Srikanth's round-3 reply: proxy observed FIN from MockForge
//! after every 200 response, then RST when it reused the socket. Root cause is
//! upstream HTTP/1.1 not being negotiated. We can't force hyper to keep the
//! connection alive after an HTTP/1.0 request, but we can advertise our
//! preferred policy in the response so proxies that read it adjust.

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

/// Default idle timeout advertised by the `Keep-Alive` header, in seconds.
/// Picked to match hyper's documented behavior — long enough that even a
/// slowly-draining proxy pool reuses the socket before MockForge closes it.
const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// Default max requests per connection advertised by the `Keep-Alive` header.
const DEFAULT_MAX_REQUESTS: u64 = 1000;

/// Is the keepalive hint enabled? Reads `MOCKFORGE_HTTP_KEEPALIVE_HINT` once
/// per startup process. Truthy values: `1`, `true`, `yes`, `on`.
pub fn is_keepalive_hint_enabled() -> bool {
    std::env::var("MOCKFORGE_HTTP_KEEPALIVE_HINT")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Read the advertised timeout in seconds. Falls back to the default.
fn keepalive_timeout_secs() -> u64 {
    std::env::var("MOCKFORGE_HTTP_KEEPALIVE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

/// Read the advertised max requests per connection. Falls back to the default.
fn keepalive_max_requests() -> u64 {
    std::env::var("MOCKFORGE_HTTP_KEEPALIVE_MAX_REQUESTS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_REQUESTS)
}

/// Middleware: stamp `Connection: keep-alive` and `Keep-Alive: timeout=…,
/// max=…` on every response. Does NOT override an upstream-set
/// `Connection: close` header.
pub async fn keepalive_hint_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;

    // Don't undo an explicit close; if downstream code already decided to
    // close, leave it alone.
    let already_close = response
        .headers()
        .get(http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_ascii_lowercase().contains("close"))
        .unwrap_or(false);
    if already_close {
        return response;
    }

    response
        .headers_mut()
        .insert(http::header::CONNECTION, HeaderValue::from_static("keep-alive"));

    let header_value =
        format!("timeout={}, max={}", keepalive_timeout_secs(), keepalive_max_requests());
    if let Ok(v) = HeaderValue::from_str(&header_value) {
        // `Keep-Alive` (case-sensitive in HeaderName) is a hop-by-hop header
        // some intermediaries strip, but the ones we care about (F5, Avi,
        // nginx) preserve it for their pool decisions.
        response.headers_mut().insert("keep-alive", v);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn middleware_adds_keepalive_headers() {
        let app: Router = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(keepalive_hint_middleware));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.headers().get(http::header::CONNECTION).unwrap(), "keep-alive");
        let ka = res.headers().get("keep-alive").unwrap().to_str().unwrap();
        assert!(ka.contains("timeout="));
        assert!(ka.contains("max="));
    }

    #[tokio::test]
    async fn middleware_respects_existing_close_header() {
        let app: Router = Router::new()
            .route(
                "/",
                get(|| async {
                    let mut res = Response::new(Body::from("bye"));
                    res.headers_mut()
                        .insert(http::header::CONNECTION, HeaderValue::from_static("close"));
                    res
                }),
            )
            .layer(axum::middleware::from_fn(keepalive_hint_middleware));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.headers().get(http::header::CONNECTION).unwrap(), "close");
        assert!(res.headers().get("keep-alive").is_none());
    }
}

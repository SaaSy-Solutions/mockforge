//! Connection diagnostics middleware.
//!
//! Logs the HTTP version and `Connection` header value MockForge sees on every
//! incoming request — the exact information needed to debug "why does
//! MockForge close the connection after each response?" scenarios.
//!
//! Issue #79 — Srikanth's round-5 reply: his PCAP showed HTTP/1.1 from the
//! proxy with no `Connection` header, but MockForge was sending FIN after each
//! response. The only way to confirm what MockForge actually sees on the wire
//! is to log the version + headers from hyper's parsed view of the request.
//!
//! Enabled by `MOCKFORGE_HTTP_LOG_CONN=1` (and convenience aliases
//! `true|yes|on`). Off by default — the per-request log line is too noisy for
//! normal operation.
//!
//! The emitted log uses INFO level so it surfaces under the default subscriber
//! filter; the env var is the on/off switch.

use axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response};
use std::net::SocketAddr;

/// Is the connection-diagnostics log enabled? Reads
/// `MOCKFORGE_HTTP_LOG_CONN` once per call (cheap — `env::var` is a hash
/// lookup; we don't bother caching).
pub fn is_conn_log_enabled() -> bool {
    std::env::var("MOCKFORGE_HTTP_LOG_CONN")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Render `hyper::Version` as a stable string (`HTTP/1.0`, `HTTP/1.1`,
/// `HTTP/2.0`, `HTTP/3.0`, …) for log output. The `Debug` impl already
/// produces this shape, but `{:?}` is documented as "not stable", so we
/// match explicitly for the versions we care about and fall back to Debug
/// for anything new.
fn version_str(v: http::Version) -> String {
    match v {
        http::Version::HTTP_09 => "HTTP/0.9".to_string(),
        http::Version::HTTP_10 => "HTTP/1.0".to_string(),
        http::Version::HTTP_11 => "HTTP/1.1".to_string(),
        http::Version::HTTP_2 => "HTTP/2.0".to_string(),
        http::Version::HTTP_3 => "HTTP/3.0".to_string(),
        other => format!("{:?}", other),
    }
}

/// Middleware: when enabled, log the request's HTTP version + `Connection`
/// header + selected hop-by-hop headers, then the response's `Connection`
/// header (which is what determines whether hyper will FIN the socket).
///
/// Output (one line per request, tracing target = `mockforge_http::conn_diag`):
///
/// ```text
/// http_conn_diag method=GET path=/v1.0/users version=HTTP/1.1 \
///   req_connection="keep-alive" req_keep_alive="timeout=120" \
///   req_host="192.168.2.86" peer=172.18.0.248:54321 \
///   resp_status=200 resp_connection="keep-alive" \
///   close_decision="keep-alive (HTTP/1.1, no Connection: close)"
/// ```
pub async fn conn_diag_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    if !is_conn_log_enabled() {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let version = req.version();
    let version_label = version_str(version);

    let req_connection = req
        .headers()
        .get(http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let req_keep_alive = req
        .headers()
        .get("keep-alive")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let req_host = req
        .headers()
        .get(http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let req_te = req
        .headers()
        .get(http::header::TE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let req_upgrade = req
        .headers()
        .get(http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    // Pre-compute the close decision hyper will make once the response goes
    // back: HTTP/1.0 without `Connection: keep-alive` → close; HTTP/1.1 with
    // `Connection: close` → close; everything else → keep-alive.
    let close_decision = match version {
        http::Version::HTTP_10 => {
            if req_connection.to_ascii_lowercase().contains("keep-alive") {
                "keep-alive (HTTP/1.0, explicit Connection: keep-alive)"
            } else {
                "close (HTTP/1.0 default — no Connection: keep-alive header)"
            }
        }
        http::Version::HTTP_11 => {
            if req_connection.to_ascii_lowercase().contains("close") {
                "close (HTTP/1.1 with Connection: close)"
            } else {
                "keep-alive (HTTP/1.1 default — no Connection: close)"
            }
        }
        _ => "n/a (HTTP/2+)",
    };

    let response = next.run(req).await;
    let resp_status = response.status().as_u16();
    let resp_connection = response
        .headers()
        .get(http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();
    let resp_keep_alive = response
        .headers()
        .get("keep-alive")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<absent>")
        .to_string();

    tracing::info!(
        target: "mockforge_http::conn_diag",
        method = %method,
        path = %path,
        version = %version_label,
        req_connection = %req_connection,
        req_keep_alive = %req_keep_alive,
        req_host = %req_host,
        req_te = %req_te,
        req_upgrade = %req_upgrade,
        peer = %peer,
        resp_status = resp_status,
        resp_connection = %resp_connection,
        resp_keep_alive = %resp_keep_alive,
        close_decision = %close_decision,
        "http_conn_diag",
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use http::HeaderValue;
    use tower::ServiceExt;

    fn isolate_env<F: FnOnce()>(value: Option<&str>, body: F) {
        // Tests can't run in true parallel for env-var coverage; use a process-
        // wide mutex held in the suite to serialize. Here we just save + set.
        let prev = std::env::var("MOCKFORGE_HTTP_LOG_CONN").ok();
        match value {
            Some(v) => std::env::set_var("MOCKFORGE_HTTP_LOG_CONN", v),
            None => std::env::remove_var("MOCKFORGE_HTTP_LOG_CONN"),
        }
        body();
        match prev {
            Some(p) => std::env::set_var("MOCKFORGE_HTTP_LOG_CONN", p),
            None => std::env::remove_var("MOCKFORGE_HTTP_LOG_CONN"),
        }
    }

    #[test]
    fn enabled_flag_truthy_values() {
        isolate_env(Some("1"), || assert!(is_conn_log_enabled()));
        isolate_env(Some("true"), || assert!(is_conn_log_enabled()));
        isolate_env(Some("on"), || assert!(is_conn_log_enabled()));
        isolate_env(Some("yes"), || assert!(is_conn_log_enabled()));
        isolate_env(Some("0"), || assert!(!is_conn_log_enabled()));
        isolate_env(None, || assert!(!is_conn_log_enabled()));
    }

    #[tokio::test]
    async fn middleware_is_transparent_when_disabled() {
        isolate_env(None, || {});
        let app: Router = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(conn_diag_middleware));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn middleware_passes_through_when_enabled() {
        // Just confirm we don't drop the response. The actual log assertion
        // is covered by inspecting tracing in higher-level integration tests.
        let prev = std::env::var("MOCKFORGE_HTTP_LOG_CONN").ok();
        std::env::set_var("MOCKFORGE_HTTP_LOG_CONN", "1");

        let app: Router = Router::new()
            .route("/x", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(conn_diag_middleware));

        let req = Request::builder()
            .uri("/x")
            .header(http::header::CONNECTION, HeaderValue::from_static("keep-alive"))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);

        match prev {
            Some(p) => std::env::set_var("MOCKFORGE_HTTP_LOG_CONN", p),
            None => std::env::remove_var("MOCKFORGE_HTTP_LOG_CONN"),
        }
    }

    #[test]
    fn version_str_renders_known_versions() {
        assert_eq!(version_str(http::Version::HTTP_10), "HTTP/1.0");
        assert_eq!(version_str(http::Version::HTTP_11), "HTTP/1.1");
        assert_eq!(version_str(http::Version::HTTP_2), "HTTP/2.0");
    }
}

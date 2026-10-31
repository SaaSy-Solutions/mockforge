use super::*;
use axum::http::HeaderMap;
use axum::routing::get;
use std::net::IpAddr;
use std::net::{Ipv4Addr, Ipv6Addr};
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef0123456789abcdef-test";

fn app(policy: ControlPlanePolicy) -> Router {
    let ok = || async { "ok" };
    let control = protect(
        Router::new()
            .route("/api/v1/auth/tokens/revoke", get(ok).post(ok))
            .route("/api/chaos/config", get(ok).post(ok)),
    );
    let reserved_unprotected = Router::new().route("/__mockforge/late", get(ok));
    let mocks = Router::new()
        .route("/api/v1/auth/login", get(ok).post(ok))
        .route("/__mockforgeX/ping", get(ok))
        .route("/__mockforge/docs", get(ok));
    install(control.merge(reserved_unprotected).merge(mocks), policy)
}

async fn status(app: Router, method: Method, path: &str, headers: &[(&str, &str)]) -> u16 {
    let mut req = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap().status().as_u16()
}

mod boundaries;
mod configuration;
mod credentials;

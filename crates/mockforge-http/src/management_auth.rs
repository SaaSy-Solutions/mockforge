//! Token gate for control-plane writes on a hosted mock (#1085).
//!
//! A hosted mock serves the customer's API and MockForge's own management
//! routes on the same public port. When `MOCKFORGE_MANAGEMENT_TOKEN` is set
//! (the registry sets it on every hosted machine), any request that can
//! change state on a control-plane route must carry that token. Reads stay
//! open because the cloud UI fetches the spec, route list, docs and live
//! stream straight from the browser. Self-hosted servers don't set the
//! variable, so nothing changes for them.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// Environment variable holding the deployment's management token
pub const MANAGEMENT_TOKEN_ENV: &str = "MOCKFORGE_MANAGEMENT_TOKEN";
/// Header that carries the token (`Authorization: Bearer` also works)
pub const MANAGEMENT_TOKEN_HEADER: &str = "x-mockforge-management-token";

/// Route prefixes served by MockForge itself rather than the user's spec.
/// A request matches a prefix when its path equals it or continues with `/`.
const CONTROL_PLANE_PREFIXES: &[&str] = &[
    "/__mockforge",
    "/api/chaos",
    "/api/recorder",
    "/api/performance",
    "/api/world-state",
    "/api/verification",
    "/api/v1/ab-tests",
    "/api/v1/auth",
    "/api/v1/behavioral-cloning",
    "/api/v1/change-management",
    "/api/v1/consistency",
    "/api/v1/consumers",
    "/api/v1/contract-health",
    "/api/v1/contracts",
    "/api/v1/drift",
    "/api/v1/forecasts",
    "/api/v1/pipelines",
    "/api/v1/pr",
    "/api/v1/scenario-studio",
    "/api/v1/security",
    "/api/v1/semantic-drift",
    "/api/v1/snapshots",
    "/api/v1/threats",
    "/api/v1/workspace",
    "/api/v1/xray",
];

/// Whether `path` belongs to a MockForge control-plane route
pub fn is_control_plane(path: &str) -> bool {
    CONTROL_PLANE_PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

fn is_read(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn presented_token(req: &Request<Body>) -> Option<&str> {
    let headers = req.headers();
    headers.get(MANAGEMENT_TOKEN_HEADER).and_then(|v| v.to_str().ok()).or_else(|| {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
    })
}

async fn require_management_token(
    State(expected): State<Arc<str>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if is_read(req.method()) || !is_control_plane(req.uri().path()) {
        return next.run(req).await;
    }
    let authorized = presented_token(&req)
        .is_some_and(|token| bool::from(token.as_bytes().ct_eq(expected.as_bytes())));
    if authorized {
        return next.run(req).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "This hosted mock's management API requires its management token \
                      (X-MockForge-Management-Token). Find it on the Hosted Mocks page."
        })),
    )
        .into_response()
}

/// Wrap `app` in the token gate when `MOCKFORGE_MANAGEMENT_TOKEN` is set.
/// Apply it last, after every router is merged, so no route escapes it.
pub fn gate_control_plane_writes(app: Router) -> Router {
    match std::env::var(MANAGEMENT_TOKEN_ENV) {
        Ok(token) if !token.is_empty() => gate_with_token(app, token),
        _ => app,
    }
}

fn gate_with_token(app: Router, token: String) -> Router {
    let expected: Arc<str> = Arc::from(token);
    app.layer(axum::middleware::from_fn_with_state(expected, require_management_token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use tower::ServiceExt;

    fn app() -> Router {
        let ok = || async { "ok" };
        gate_with_token(
            Router::new()
                .route("/__mockforge/api/mocks", get(ok).post(ok))
                .route("/api/chaos/config", get(ok).post(ok))
                .route("/api/chaosmonkey", get(ok).post(ok))
                .route("/api/v1/users", get(ok).post(ok)),
            "secret-token".to_string(),
        )
    }

    async fn status(method: Method, path: &str, header: Option<(&str, &str)>) -> StatusCode {
        let mut req = Request::builder().method(method).uri(path);
        if let Some((name, value)) = header {
            req = req.header(name, value);
        }
        app().oneshot(req.body(Body::empty()).unwrap()).await.unwrap().status()
    }

    #[test]
    fn control_plane_prefixes_match_whole_segments() {
        assert!(is_control_plane("/__mockforge"));
        assert!(is_control_plane("/__mockforge/time-travel/advance"));
        assert!(is_control_plane("/api/v1/pr/generate"));
        assert!(!is_control_plane("/api/chaosmonkey"));
        assert!(!is_control_plane("/api/v1/prices"));
        assert!(!is_control_plane("/api/v1/users"));
    }

    #[tokio::test]
    async fn writes_need_the_token_and_reads_do_not() {
        assert_eq!(
            status(Method::POST, "/__mockforge/api/mocks", None).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(status(Method::POST, "/api/chaos/config", None).await, StatusCode::UNAUTHORIZED);
        assert_eq!(
            status(
                Method::POST,
                "/__mockforge/api/mocks",
                Some((MANAGEMENT_TOKEN_HEADER, "wrong"))
            )
            .await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status(
                Method::POST,
                "/__mockforge/api/mocks",
                Some((MANAGEMENT_TOKEN_HEADER, "secret-token"))
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            status(
                Method::POST,
                "/api/chaos/config",
                Some(("authorization", "Bearer secret-token"))
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(status(Method::GET, "/__mockforge/api/mocks", None).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_customers_own_routes_are_untouched() {
        assert_eq!(status(Method::POST, "/api/v1/users", None).await, StatusCode::OK);
        assert_eq!(status(Method::POST, "/api/chaosmonkey", None).await, StatusCode::OK);
    }
}

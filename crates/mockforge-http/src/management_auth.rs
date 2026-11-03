//! Authentication for MockForge's control plane on the HTTP data port (#1011).
//!
//! The data port serves the user's mocks and, beside them, MockForge's own
//! control plane: the management API, chains, fixtures, time travel, chaos,
//! the recorder, verification and the `/api/v1/*` feature routers. Mock
//! traffic stays public. Every control-plane request, read or write, needs
//! a management credential:
//!
//! - the deployment's management token (`MOCKFORGE_MANAGEMENT_TOKEN`, which
//!   the registry sets on every hosted mock), sent as
//!   `X-MockForge-Management-Token` or `Authorization: Bearer`; browsers
//!   opening a WebSocket send it as the subprotocol `mockforge-token.<token>`;
//! - or the admin user's HTTP Basic credentials from the admin config.
//!
//! With neither configured the control plane is locked: it answers 401 to
//! everyone. `MOCKFORGE_INSECURE_MANAGEMENT=1` turns authentication off, but
//! only when the data port is bound to a loopback address; on any other bind
//! the server refuses to start.
//!
//! Two layers enforce this:
//!
//! - [`protect`] wraps each control router before it is mounted. It runs
//!   only when axum routes a request to one of that router's handlers, so
//!   path tricks cannot reach a handler around it, and a user's mock that
//!   shares a prefix such as `/api/v1/auth` is not affected.
//! - [`install`] is applied last, around the whole app. It hands the policy
//!   to the inner layers and also guards the reserved `/__mockforge`
//!   namespace by normalised path, so a route mounted there without
//!   [`protect`] still needs the credential.
//!
//! If [`install`] is missing, [`protect`] falls back to the token from the
//! environment, or locks the route. It never falls back to open.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use std::sync::Arc;
use tracing::warn;

mod credentials;
mod namespace;
mod policy;

use credentials::strip_token;
pub use namespace::is_reserved_path;
pub use policy::{
    is_strict_loopback, AdminCredential, ControlPlaneConfigError, ControlPlanePolicy,
};

/// Environment variable holding the deployment's management token
pub const MANAGEMENT_TOKEN_ENV: &str = "MOCKFORGE_MANAGEMENT_TOKEN";
/// Header that carries the token (`Authorization: Bearer` also works)
pub const MANAGEMENT_TOKEN_HEADER: &str = "x-mockforge-management-token";
/// Development opt-out; honoured only on a loopback bind
pub const INSECURE_MANAGEMENT_ENV: &str = "MOCKFORGE_INSECURE_MANAGEMENT";
/// WebSocket subprotocol the server selects for management streams
pub const WS_PROTOCOL: &str = "mockforge-management";
/// Prefix of the WebSocket subprotocol that carries the token
pub const WS_TOKEN_PROTOCOL_PREFIX: &str = "mockforge-token.";
/// Shortest management token accepted
pub const MIN_TOKEN_LEN: usize = 32;

/// Reserved first path segment for MockForge's own routes
const RESERVED_SEGMENT: &str = "__mockforge";
/// Paths in the reserved namespace that answer without a credential.
/// Exact raw-path matches, `GET`/`HEAD` only. `/__mockforge/docs` is a
/// static page; it fetches the spec with the token it is given.
const PUBLIC_RESERVED_PATHS: &[&str] = &["/__mockforge/docs"];

#[derive(Debug, Clone, Copy)]
enum Denial {
    Locked,
    MissingCredential,
    NonLoopbackHost,
}

impl IntoResponse for Denial {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Denial::Locked => (
                StatusCode::UNAUTHORIZED,
                "MockForge's control plane is locked because no management credential is \
                 configured. Set MOCKFORGE_MANAGEMENT_TOKEN (32+ characters) on the server and \
                 send it as X-MockForge-Management-Token.",
            ),
            Denial::MissingCredential => (
                StatusCode::UNAUTHORIZED,
                "MockForge's control plane requires the management token \
                 (X-MockForge-Management-Token). For a hosted mock, find it on the Hosted \
                 Mocks page.",
            ),
            Denial::NonLoopbackHost => (
                StatusCode::FORBIDDEN,
                "MockForge's unauthenticated control plane only answers requests addressed \
                 to a loopback host.",
            ),
        };
        let mut response = (status, Json(serde_json::json!({ "error": message }))).into_response();
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"mockforge-management\""),
            );
        }
        response
    }
}

/// Marker the outer layer leaves on a request it already authorised, so
/// the inner layer does not verify (and possibly bcrypt) twice. Only server
/// code can insert request extensions.
#[derive(Clone, Copy)]
struct Authorized;

fn is_cors_preflight(req: &Request<Body>) -> bool {
    req.method() == Method::OPTIONS
        && req.headers().contains_key(header::ORIGIN)
        && req.headers().contains_key(header::ACCESS_CONTROL_REQUEST_METHOD)
}

fn is_public_reserved(req: &Request<Body>) -> bool {
    matches!(*req.method(), Method::GET | Method::HEAD)
        && PUBLIC_RESERVED_PATHS.contains(&req.uri().path())
}

async fn outer_gate(
    axum::extract::State(policy): axum::extract::State<Arc<ControlPlanePolicy>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    req.extensions_mut().remove::<Authorized>();
    req.extensions_mut().insert(policy.clone());
    // Authenticate management credentials before removing them, so recording
    // and handlers never receive them. Unrelated mock Basic auth is preserved.
    let admin_matches = policy.admin_matches(req.headers());
    if policy.token_matches(req.headers()) || admin_matches {
        req.extensions_mut().insert(Authorized);
    }
    strip_token(req.headers_mut(), &policy);
    if admin_matches {
        req.headers_mut().remove(header::AUTHORIZATION);
    }
    if req.extensions().get::<Authorized>().is_none()
        && is_reserved_path(req.uri().path())
        && !is_public_reserved(&req)
        && !is_cors_preflight(&req)
    {
        if let Err(denial) = policy.authorize(req.headers()) {
            warn!(path = %req.uri().path(), "control-plane request refused");
            return denial.into_response();
        }
        req.extensions_mut().insert(Authorized);
    }
    next.run(req).await
}

async fn inner_gate(req: Request<Body>, next: Next) -> Response {
    if req.extensions().get::<Authorized>().is_some() {
        return next.run(req).await;
    }
    let policy = req
        .extensions()
        .get::<Arc<ControlPlanePolicy>>()
        .cloned()
        .unwrap_or_else(|| Arc::new(ControlPlanePolicy::fallback_from_env()));
    if let Err(denial) = policy.authorize(req.headers()) {
        warn!(path = %req.uri().path(), "control-plane request refused");
        return denial.into_response();
    }
    next.run(req).await
}

/// Require a management credential on every route of `router`. Apply it to
/// each control router before mounting it.
pub fn protect<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !router.has_routes() {
        return router;
    }
    router.route_layer(axum::middleware::from_fn(inner_gate))
}

/// Apply `policy` to the whole app. Call it last, after every router and
/// layer is added, so it is the outermost layer.
pub fn install(app: Router, policy: ControlPlanePolicy) -> Router {
    app.layer(axum::middleware::from_fn_with_state(Arc::new(policy), outer_gate))
}

#[cfg(test)]
mod tests;

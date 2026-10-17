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
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tracing::warn;

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

/// The admin user's HTTP Basic credentials. The password may be plaintext or
/// a bcrypt hash.
#[derive(Clone)]
pub struct AdminCredential {
    username: String,
    password: String,
}

impl AdminCredential {
    /// Both values must be non-empty.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Option<Self> {
        let (username, password) = (username.into(), password.into());
        if username.is_empty() || password.is_empty() {
            return None;
        }
        Some(Self { username, password })
    }
}

impl fmt::Debug for AdminCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminCredential").field("username", &self.username).finish()
    }
}

/// Who may use the control plane on the data port
#[derive(Clone)]
pub struct ControlPlanePolicy {
    mode: Mode,
}

#[derive(Clone)]
enum Mode {
    /// A credential is required. With neither set, the control plane is locked.
    Credentials {
        token: Option<Arc<str>>,
        admin: Option<AdminCredential>,
    },
    /// No authentication. Constructed only for a loopback bind.
    UnauthenticatedLoopback,
}

impl fmt::Debug for ControlPlanePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.mode {
            Mode::Credentials { token, admin } => f
                .debug_struct("Credentials")
                .field("token", &token.as_ref().map(|_| "<redacted>"))
                .field("admin", admin)
                .finish(),
            Mode::UnauthenticatedLoopback => f.write_str("UnauthenticatedLoopback"),
        }
    }
}

/// Why a control-plane policy cannot be built
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlPlaneConfigError {
    /// The token is shorter than [`MIN_TOKEN_LEN`]
    #[error("{MANAGEMENT_TOKEN_ENV} must be at least {MIN_TOKEN_LEN} characters")]
    TokenTooShort,
    /// The opt-out was requested on a non-loopback bind
    #[error(
        "Refusing to start: {INSECURE_MANAGEMENT_ENV} turns off control-plane authentication \
         and is allowed only when the HTTP server is bound to a loopback address, but it is \
         bound to {0}. Unset it, or set {MANAGEMENT_TOKEN_ENV} instead."
    )]
    InsecureOnNonLoopback(IpAddr),
}

impl ControlPlanePolicy {
    /// Every control-plane request is refused.
    pub fn locked() -> Self {
        Self {
            mode: Mode::Credentials {
                token: None,
                admin: None,
            },
        }
    }

    /// Require `token` (at least [`MIN_TOKEN_LEN`] characters).
    pub fn with_token(token: impl Into<String>) -> Result<Self, ControlPlaneConfigError> {
        let token = token.into();
        if token.chars().count() < MIN_TOKEN_LEN {
            return Err(ControlPlaneConfigError::TokenTooShort);
        }
        Ok(Self {
            mode: Mode::Credentials {
                token: Some(Arc::from(token)),
                admin: None,
            },
        })
    }

    /// Also accept the admin user's Basic credentials. Has no effect on an
    /// unauthenticated policy.
    pub fn with_admin(mut self, admin: Option<AdminCredential>) -> Self {
        if let Mode::Credentials { admin: slot, .. } = &mut self.mode {
            if admin.is_some() {
                *slot = admin;
            }
        }
        self
    }

    /// No authentication, for a server bound to `bind`. Refused unless
    /// `bind` is a loopback address (`127.0.0.0/8` or `::1`). Wildcards and
    /// IPv4-mapped IPv6 addresses are refused.
    pub fn unauthenticated_loopback(bind: IpAddr) -> Result<Self, ControlPlaneConfigError> {
        if !is_strict_loopback(bind) {
            return Err(ControlPlaneConfigError::InsecureOnNonLoopback(bind));
        }
        Ok(Self {
            mode: Mode::UnauthenticatedLoopback,
        })
    }

    /// The policy for a data port bound to `bind`, from
    /// `MOCKFORGE_MANAGEMENT_TOKEN` and `MOCKFORGE_INSECURE_MANAGEMENT`.
    pub fn from_env(
        bind: IpAddr,
        admin: Option<AdminCredential>,
    ) -> Result<Self, ControlPlaneConfigError> {
        Self::from_values(
            bind,
            std::env::var(MANAGEMENT_TOKEN_ENV).ok().as_deref(),
            std::env::var(INSECURE_MANAGEMENT_ENV).ok().as_deref(),
            admin,
        )
    }

    /// [`Self::from_env`] with the variables passed in.
    ///
    /// A configured credential always wins over the opt-out, and the opt-out
    /// on a non-loopback bind is an error even when a credential is set, so
    /// a misconfigured deployment fails loudly.
    pub fn from_values(
        bind: IpAddr,
        token: Option<&str>,
        insecure: Option<&str>,
        admin: Option<AdminCredential>,
    ) -> Result<Self, ControlPlaneConfigError> {
        let insecure = insecure.is_some_and(is_truthy);
        if insecure && !is_strict_loopback(bind) {
            return Err(ControlPlaneConfigError::InsecureOnNonLoopback(bind));
        }
        let token = token.filter(|t| !t.is_empty());
        let policy = match token {
            Some(token) => Self::with_token(token)?.with_admin(admin),
            None if admin.is_some() => Self::locked().with_admin(admin),
            None if insecure => Self::unauthenticated_loopback(bind)?,
            None => Self::locked(),
        };
        Ok(policy)
    }

    /// The policy [`protect`] uses when [`install`] was not applied: the
    /// environment's token if it is valid, otherwise locked.
    pub fn fallback_from_env() -> Self {
        std::env::var(MANAGEMENT_TOKEN_ENV)
            .ok()
            .and_then(|token| Self::with_token(token).ok())
            .unwrap_or_else(Self::locked)
    }

    /// Whether no credential can open the control plane
    pub fn is_locked(&self) -> bool {
        matches!(
            self.mode,
            Mode::Credentials {
                token: None,
                admin: None
            }
        )
    }

    /// Whether the control plane is open without a credential
    pub fn is_unauthenticated(&self) -> bool {
        matches!(self.mode, Mode::UnauthenticatedLoopback)
    }

    fn authorize(&self, headers: &HeaderMap) -> Result<(), Denial> {
        match &self.mode {
            Mode::UnauthenticatedLoopback => {
                // A loopback bind is still reachable from a browser through
                // DNS rebinding; insist on a loopback Host header.
                if host_is_loopback(headers) {
                    Ok(())
                } else {
                    Err(Denial::NonLoopbackHost)
                }
            }
            Mode::Credentials {
                token: None,
                admin: None,
            } => Err(Denial::Locked),
            Mode::Credentials { token, admin } => {
                let token_ok = token.as_ref().is_some_and(|expected| {
                    presented_tokens(headers).any(|presented| {
                        bool::from(presented.as_bytes().ct_eq(expected.as_bytes()))
                    })
                });
                if token_ok {
                    return Ok(());
                }
                let admin_ok = admin.as_ref().is_some_and(|admin| {
                    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).is_some_and(
                        |value| {
                            crate::auth::admin_auth::basic_credentials_match(
                                value,
                                &admin.username,
                                &admin.password,
                            )
                        },
                    )
                });
                if admin_ok {
                    Ok(())
                } else {
                    Err(Denial::MissingCredential)
                }
            }
        }
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// `127.0.0.0/8` or `::1`. IPv4-mapped IPv6 (`::ffff:127.0.0.1`) and
/// wildcard addresses are not loopback for this purpose.
pub fn is_strict_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn host_is_loopback(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        // HTTP/1.0 or a test client without Host: nothing to rebind.
        return true;
    };
    let host = host.trim();
    let name = if let Some(rest) = host.strip_prefix('[') {
        match rest.split_once(']') {
            Some((inner, _)) => inner,
            None => return false,
        }
    } else {
        host.rsplit_once(':').map_or(host, |(name, _)| name)
    };
    if name.eq_ignore_ascii_case("localhost") {
        return true;
    }
    name.parse::<IpAddr>().is_ok_and(is_strict_loopback)
}

/// Tokens presented on a request: the management header, a Bearer token,
/// and on a WebSocket upgrade the `mockforge-token.` subprotocol.
fn presented_tokens(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    let from_header = headers
        .get_all(MANAGEMENT_TOKEN_HEADER)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(str::trim);
    let from_bearer = headers
        .get_all(header::AUTHORIZATION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|v| v.strip_prefix("Bearer "))
        .map(str::trim);
    let upgrade = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let from_protocol = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter(move |_| upgrade)
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|p| p.trim().strip_prefix(WS_TOKEN_PROTOCOL_PREFIX));
    from_header.chain(from_bearer).chain(from_protocol).filter(|t| !t.is_empty())
}

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

/// Whether `path` lies in the reserved `/__mockforge` namespace after
/// percent-decoding, slash collapsing, dot-segment resolution, `;` parameter
/// stripping and case folding. `/__mockforgeX` is not in the namespace.
pub fn is_reserved_path(path: &str) -> bool {
    let mut decoded = path.to_string();
    for _ in 0..3 {
        let next = urlencoding::decode(&decoded)
            .map(|d| d.into_owned())
            .unwrap_or_else(|_| decoded.clone());
        if next == decoded {
            break;
        }
        decoded = next;
    }
    let decoded = decoded.replace('\\', "/");
    let mut segments: Vec<&str> = Vec::new();
    for segment in decoded.split('/') {
        let segment = segment.split(';').next().unwrap_or("").trim();
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case(RESERVED_SEGMENT))
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
    if is_reserved_path(req.uri().path()) && !is_public_reserved(&req) && !is_cors_preflight(&req) {
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
mod tests {
    use super::*;
    use axum::routing::get;
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

    #[test]
    fn reserved_namespace_is_normalised() {
        for path in [
            "/__mockforge",
            "/__mockforge/api/mocks",
            "//__mockforge/api/mocks",
            "/__mockforge//api",
            "/%5F_mockforge/api/mocks",
            "/%5f%5fmockforge/api",
            "/%255F_mockforge/api",
            "/__MOCKFORGE/api",
            "/x/../__mockforge/api",
            "/./__mockforge/api",
            "/__mockforge;jsessionid=1/api",
            "\\__mockforge\\api",
        ] {
            assert!(is_reserved_path(path), "{path}");
        }
        for path in [
            "/__mockforgeX/api",
            "/api/__mockforge",
            "/",
            "/health",
            "/__mock",
        ] {
            assert!(!is_reserved_path(path), "{path}");
        }
    }

    #[test]
    fn loopback_is_strict() {
        assert!(is_strict_loopback(Ipv4Addr::LOCALHOST.into()));
        assert!(is_strict_loopback(Ipv4Addr::new(127, 1, 2, 3).into()));
        assert!(is_strict_loopback(Ipv6Addr::LOCALHOST.into()));
        for ip in [
            IpAddr::from(Ipv4Addr::UNSPECIFIED),
            IpAddr::from(Ipv6Addr::UNSPECIFIED),
            IpAddr::from(Ipv4Addr::LOCALHOST.to_ipv6_mapped()),
            IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
            "fdaa::1".parse().unwrap(),
        ] {
            assert!(!is_strict_loopback(ip), "{ip}");
        }
    }

    #[test]
    fn insecure_opt_out_is_refused_off_loopback() {
        for bind in ["0.0.0.0", "::", "::ffff:127.0.0.1", "10.1.2.3", "fdaa::3"] {
            let bind: IpAddr = bind.parse().unwrap();
            assert_eq!(
                ControlPlanePolicy::from_values(bind, None, Some("1"), None).unwrap_err(),
                ControlPlaneConfigError::InsecureOnNonLoopback(bind),
            );
            // Even with a token set, the opt-out off loopback fails loudly.
            assert!(ControlPlanePolicy::from_values(bind, Some(TOKEN), Some("true"), None).is_err());
            assert!(ControlPlanePolicy::unauthenticated_loopback(bind).is_err());
        }
        for bind in ["127.0.0.1", "::1"] {
            let policy =
                ControlPlanePolicy::from_values(bind.parse().unwrap(), None, Some("1"), None)
                    .unwrap();
            assert!(policy.is_unauthenticated());
        }
    }

    #[test]
    fn policy_from_values() {
        let any: IpAddr = Ipv4Addr::UNSPECIFIED.into();
        assert!(ControlPlanePolicy::from_values(any, None, None, None).unwrap().is_locked());
        assert!(ControlPlanePolicy::from_values(any, Some(""), Some("0"), None)
            .unwrap()
            .is_locked());
        assert_eq!(
            ControlPlanePolicy::from_values(any, Some("short"), None, None).unwrap_err(),
            ControlPlaneConfigError::TokenTooShort
        );
        let admin = AdminCredential::new("admin", "a-long-admin-password");
        let policy = ControlPlanePolicy::from_values(any, None, None, admin).unwrap();
        assert!(!policy.is_locked() && !policy.is_unauthenticated());
        // A credential wins over the loopback opt-out.
        let policy = ControlPlanePolicy::from_values(
            Ipv4Addr::LOCALHOST.into(),
            Some(TOKEN),
            Some("1"),
            None,
        )
        .unwrap();
        assert!(!policy.is_unauthenticated());
        assert!(AdminCredential::new("", "x").is_none());
        assert!(AdminCredential::new("x", "").is_none());
    }

    #[tokio::test]
    async fn bad_credentials_are_refused_and_the_token_is_accepted() {
        let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
        let basic_wrong = format!(
            "Basic {}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "admin:wrong")
        );
        let bearer_prefix = format!("Bearer {}", &TOKEN[..TOKEN.len() - 1]);
        let protocol_no_upgrade = format!("{WS_TOKEN_PROTOCOL_PREFIX}{TOKEN}");
        let refused: Vec<Vec<(&str, &str)>> = vec![
            vec![],
            vec![(MANAGEMENT_TOKEN_HEADER, "")],
            vec![(MANAGEMENT_TOKEN_HEADER, "wrong")],
            vec![(MANAGEMENT_TOKEN_HEADER, &TOKEN[..10])],
            vec![("authorization", "Bearer ")],
            vec![("authorization", "Bearer wrong")],
            vec![("authorization", &bearer_prefix)],
            vec![("authorization", TOKEN)],
            vec![("authorization", "Basic !!!not-base64")],
            vec![("authorization", &basic_wrong)],
            // The subprotocol form only counts on a WebSocket upgrade.
            vec![("sec-websocket-protocol", &protocol_no_upgrade)],
        ];
        for path in [
            "/__mockforge/late",
            "/api/chaos/config",
            "/api/v1/auth/tokens/revoke",
        ] {
            for method in [Method::GET, Method::POST] {
                for headers in &refused {
                    assert_eq!(
                        status(app(policy()), method.clone(), path, headers).await,
                        401,
                        "{method} {path} {headers:?}"
                    );
                }
            }
            let bearer = format!("Bearer {TOKEN}");
            for headers in [
                vec![(MANAGEMENT_TOKEN_HEADER, TOKEN)],
                vec![("authorization", bearer.as_str())],
                vec![
                    ("upgrade", "websocket"),
                    ("sec-websocket-protocol", &format!("{WS_PROTOCOL}, {protocol_no_upgrade}")),
                ],
            ] {
                assert_eq!(status(app(policy()), Method::GET, path, &headers).await, 200, "{path}");
            }
        }
    }

    #[tokio::test]
    async fn locked_policy_refuses_everything_on_the_control_plane() {
        for path in [
            "/__mockforge/late",
            "/api/chaos/config",
            "/api/v1/auth/tokens/revoke",
        ] {
            let headers = [(MANAGEMENT_TOKEN_HEADER, TOKEN)];
            assert_eq!(
                status(app(ControlPlanePolicy::locked()), Method::GET, path, &headers).await,
                401
            );
        }
    }

    #[tokio::test]
    async fn admin_basic_credentials_are_accepted() {
        let admin = AdminCredential::new("admin", "correct horse battery staple");
        let policy = ControlPlanePolicy::locked().with_admin(admin);
        let good = format!(
            "Basic {}",
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                "admin:correct horse battery staple"
            )
        );
        let headers = [("authorization", good.as_str())];
        assert_eq!(
            status(app(policy.clone()), Method::POST, "/api/chaos/config", &headers).await,
            200
        );
        assert_eq!(status(app(policy), Method::GET, "/__mockforge/late", &headers).await, 200);
    }

    #[tokio::test]
    async fn mock_traffic_and_public_paths_stay_open() {
        let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
        // A user mock that shares a control router's prefix is untouched.
        assert_eq!(status(app(policy()), Method::POST, "/api/v1/auth/login", &[]).await, 200);
        assert_eq!(status(app(policy()), Method::GET, "/__mockforgeX/ping", &[]).await, 200);
        assert_eq!(status(app(policy()), Method::GET, "/__mockforge/docs", &[]).await, 200);
        // Only the exact docs path is public.
        assert_eq!(status(app(policy()), Method::GET, "/__mockforge/docs/", &[]).await, 401);
        assert_eq!(status(app(policy()), Method::POST, "/__mockforge/docs", &[]).await, 401);
    }

    #[tokio::test]
    async fn path_tricks_do_not_reach_control_handlers() {
        let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
        for path in [
            "/__mockforge/late/",
            "//__mockforge/late",
            "/__mockforge//late",
            "/%5F_mockforge/late",
            "/__MOCKFORGE/late",
            "/__mockforge/x/../late",
            "/api/../__mockforge/late",
            "/api//chaos/config",
            "/api/chaos/config/",
            "/API/chaos/config",
            "/api/%63haos/config",
            "/api/chaos/./config",
        ] {
            let code = status(app(policy()), Method::GET, path, &[]).await;
            assert!(code == 401 || code == 404, "{path} answered {code}");
        }
    }

    #[tokio::test]
    async fn cors_preflight_passes_the_outer_gate_only() {
        let headers = [
            ("origin", "https://app.mockforge.dev"),
            ("access-control-request-method", "GET"),
        ];
        let policy = ControlPlanePolicy::with_token(TOKEN).unwrap();
        // The reserved route has no OPTIONS handler: 405 from the router,
        // not a handler run.
        assert_eq!(
            status(app(policy.clone()), Method::OPTIONS, "/__mockforge/late", &headers).await,
            405
        );
        // A protected route refuses even a preflight-shaped request.
        let any_router =
            protect(Router::new().route("/api/x", axum::routing::any(|| async { "ran" })));
        assert_eq!(
            status(install(any_router, policy), Method::OPTIONS, "/api/x", &headers).await,
            401
        );
    }

    #[tokio::test]
    async fn unauthenticated_loopback_checks_the_host_header() {
        let policy =
            || ControlPlanePolicy::unauthenticated_loopback(Ipv4Addr::LOCALHOST.into()).unwrap();
        for host in [
            "localhost:3000",
            "127.0.0.1:3000",
            "[::1]:3000",
            "LOCALHOST",
        ] {
            assert_eq!(
                status(app(policy()), Method::POST, "/api/chaos/config", &[("host", host)]).await,
                200,
                "{host}"
            );
        }
        for host in [
            "evil.example",
            "evil.example:3000",
            "0.0.0.0:3000",
            "[::]:3000",
            "[::ffff:127.0.0.1]:3000",
            "127.0.0.1.evil.example",
        ] {
            assert_eq!(
                status(app(policy()), Method::POST, "/api/chaos/config", &[("host", host)]).await,
                403,
                "{host}"
            );
        }
    }

    #[tokio::test]
    async fn protect_without_install_falls_back_closed() {
        // No install(): the env token decides, and with none set the route is locked.
        let router = protect(Router::new().route("/api/chaos/config", get(|| async { "ok" })));
        let code = status(router, Method::GET, "/api/chaos/config", &[]).await;
        assert_eq!(code, 401);
    }
}

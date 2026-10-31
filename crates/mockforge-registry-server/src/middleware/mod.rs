//! HTTP middleware

pub mod api_token_auth;
pub mod csrf;
pub mod org_context;
pub mod org_rate_limit;
pub mod past_due_writes;
pub mod permission_check;
// `permissions` module now lives in mockforge-registry-core. Re-exported
// here so existing `crate::middleware::permissions::*` paths keep working.
pub use mockforge_registry_core::permissions;
pub mod rate_limit;
pub mod request_id;
pub mod rls_org_scope;
// `scope_check` now lives in mockforge-registry-core. Re-exported so
// existing `crate::middleware::scope_check::*` paths keep working.
pub use mockforge_registry_core::scope_check;
pub mod token_scope;
pub mod trusted_proxy;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::verify_token;
use crate::middleware::api_token_auth::authenticate_api_token;
use crate::AppState;

pub use org_context::{resolve_org_context, OrgContext};
pub use rate_limit::rate_limit_middleware;
pub use rls_org_scope::rls_org_scope_middleware;
pub use scope_check::{AuthType, ScopedAuth};
pub use token_scope::token_scope_middleware;

/// JSON error response for authentication failures
fn auth_error_response(message: &str, hint: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": message,
            "error_code": "AUTH_REQUIRED",
            "status": 401,
            "details": { "hint": hint }
        })),
    )
        .into_response()
}

/// Extractor for authenticated user ID from JWT middleware
#[derive(Debug, Clone)]
pub struct AuthUser(pub Uuid);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get user_id from request extensions (set by auth_middleware)
        let user_id_str = parts.extensions.get::<String>().ok_or_else(|| {
            auth_error_response(
                "Authentication required",
                "Include a valid Authorization header with your request",
            )
        })?;

        // Parse UUID
        let user_id = Uuid::parse_str(user_id_str).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "error_code": "INTERNAL_ERROR",
                    "status": 500
                })),
            )
                .into_response()
        })?;

        Ok(AuthUser(user_id))
    }
}

/// Optional authenticated user extractor
/// Returns None if no authentication is present, Some(user_id) if authenticated
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Uuid>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Fast path: user_id already resolved by auth_middleware (for routes that
        // run it) and stashed in request extensions.
        if let Some(user_id_str) = parts.extensions.get::<String>() {
            if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                return Ok(OptionalAuthUser(Some(user_id)));
            }
        }

        // Fallback: parse and verify the bearer token ourselves. Public routes
        // (e.g. marketplace search) carry `OptionalAuthUser` but do NOT run
        // `auth_middleware`, so without this an authenticated request would be
        // treated as anonymous — which previously hid the caller's own
        // org-scoped published items from search results (issue #724).
        if let Some(token) = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
        {
            // JWT_SECRET is a required env var at startup (see config.rs), so it
            // is always present here; this mirrors auth_middleware's verification.
            if let Ok(secret) = std::env::var("JWT_SECRET") {
                if let Ok(claims) = verify_token(token, &secret) {
                    if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                        return Ok(OptionalAuthUser(Some(user_id)));
                    }
                }
            }
        }

        Ok(OptionalAuthUser(None))
    }
}

/// Tiny percent-decoder for the `?token=` query path. We accept ASCII
/// percent-encoded JWTs which is what browsers emit; anything malformed
/// returns `None` and falls back to "no auth", letting the standard error
/// path produce a 401.
fn percent_decode_token(raw: &str) -> Option<String> {
    let mut out = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push(((hi as u8) << 4) | lo as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Cookie name issued by the auth handlers (`handlers::auth::SESSION_COOKIE`).
pub const SESSION_COOKIE: &str = "mockforge_session";

/// Pull the session JWT out of the `Cookie` header, if present.
///
/// Priority in [`auth_middleware`] is `Authorization` header → session cookie
/// → `?token=` query (SSE routes only). The cookie is HttpOnly, so unlike the query parameter it
/// is not readable by page JavaScript and does not leak via referrer/logs.
fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == SESSION_COOKIE).then(|| value.to_string())
    })
}

/// Route templates, exactly as registered in `routes.rs`, whose handlers serve
/// Server-Sent Events. Browsers cannot set headers on `EventSource`, so these
/// are the only routes that accept `?token=`.
///
/// The check uses axum's [`MatchedPath`](axum::extract::MatchedPath) (the route template the router
/// actually dispatched to), not the raw request path. Trailing slashes, extra
/// segments, `..`, or percent-encoded variants of an SSE URL either match a
/// different route or none, so they never widen the exception.
pub const SSE_QUERY_TOKEN_ROUTES: &[&str] = &[
    "/api/v1/hosted-mocks/{deployment_id}/runtime-logs/stream",
    "/api/v1/test-runs/{id}/stream",
    "/api/v1/workspaces/{workspace_id}/test-generation/jobs/{job_id}/stream",
];

/// Whether this request may authenticate with `?token=`: a GET to one of the
/// [`SSE_QUERY_TOKEN_ROUTES`].
fn query_token_allowed(request: &Request) -> bool {
    request.method() == axum::http::Method::GET
        && request
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .is_some_and(|matched| SSE_QUERY_TOKEN_ROUTES.contains(&matched.as_str()))
}

/// Pull `token=...` out of the query string without dragging in the `url`
/// crate. Long-lived `mfx_` API tokens are never accepted from a URL: API
/// clients can send headers, and only short-lived session JWTs may appear in
/// an `EventSource` URL.
fn extract_query_token(request: &Request) -> Option<String> {
    request.uri().query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if key == "token" {
                percent_decode_token(value).filter(|token| !token.starts_with("mfx_"))
            } else {
                None
            }
        })
    })
}

/// Credential for this request. Precedence: `Authorization` header → session
/// cookie → `?token=`, the last only on [`SSE_QUERY_TOKEN_ROUTES`].
fn extract_request_token(headers: &HeaderMap, request: &Request) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| extract_session_cookie(headers))
        .or_else(|| query_token_allowed(request).then(|| extract_query_token(request)).flatten())
}

/// Extract and verify JWT token or API token.
///
/// Tokens are normally read from the `Authorization: Bearer <token>` header.
/// Browser clients authenticated via `login`/`register`/`refresh` also carry an
/// HttpOnly session cookie (`SESSION_COOKIE`), which is accepted as a second
/// source so page JavaScript does not need to hold (or be able to steal) the
/// JWT. Browsers cannot send custom headers on `EventSource` connections, so
/// the SSE routes in [`SSE_QUERY_TOKEN_ROUTES`] additionally accept a session
/// JWT as `?token=`. Every other route ignores the query string, and request
/// logs redact it (`request_id::redact_uri`). The query path is deprecated in
/// favour of the cookie. Precedence: header → cookie → query.
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let owned_token = extract_request_token(&headers, &request).ok_or_else(|| {
        auth_error_response(
            "Authentication required",
            "Include an Authorization: Bearer <token> header (or an authenticated session \
             cookie; ?token= is accepted only on SSE stream endpoints).",
        )
    })?;

    let token = owned_token.as_str();

    // Check if this is an API token (starts with mfx_)
    if token.starts_with("mfx_") {
        match authenticate_api_token(&state, token).await.map_err(|_| {
            auth_error_response(
                "Authentication failed",
                "API token validation error. Please try again.",
            )
        })? {
            Some(auth_result) => {
                request.extensions_mut().insert(auth_result.user_id.to_string());
                request.extensions_mut().insert(AuthType::ApiToken);
                request.extensions_mut().insert(auth_result.token);
                return Ok(next.run(request).await);
            }
            None => {
                return Err(auth_error_response(
                    "Invalid API token",
                    "The API token is invalid or has been revoked. Generate a new one at https://app.mockforge.dev/settings/tokens",
                ));
            }
        }
    }

    // JWT authentication
    let claims = verify_token(token, &state.config.jwt_secret).map_err(|_| {
        auth_error_response(
            "Invalid or expired token",
            "Your session has expired. Please run 'mockforge cloud login' to re-authenticate.",
        )
    })?;

    // Add user_id to request extensions
    request.extensions_mut().insert(claims.sub.clone());
    request.extensions_mut().insert(AuthType::Jwt);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod session_cookie_tests {
    use super::*;

    fn headers_with_cookie(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers
            .insert(axum::http::header::COOKIE, axum::http::HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn extracts_session_cookie() {
        let headers = headers_with_cookie("other=1; mockforge_session=jwt-value; x=y");
        assert_eq!(extract_session_cookie(&headers).as_deref(), Some("jwt-value"));
    }

    #[test]
    fn returns_none_without_session_cookie() {
        let headers = headers_with_cookie("other=1; x=y");
        assert_eq!(extract_session_cookie(&headers), None);
    }

    #[test]
    fn returns_none_without_cookie_header() {
        assert_eq!(extract_session_cookie(&HeaderMap::new()), None);
    }
}

/// Regression tests for #1018: `?token=` is a credential only on the SSE
/// routes. They drive [`extract_request_token`] through real axum routing so
/// `MatchedPath` is set exactly as it is for `auth_middleware`.
#[cfg(test)]
mod query_token_tests {
    use super::*;
    use axum::{body::Body, routing::get, Router};
    use tower::ServiceExt;

    const JWT: &str = "eyJhbGciOiJIUzI1NiJ9.session.sig";

    async fn ok() -> &'static str {
        "handler"
    }

    /// 401 without a credential, else 200 echoing the credential found.
    async fn probe_auth(headers: HeaderMap, request: Request, next: Next) -> Response {
        match extract_request_token(&headers, &request) {
            Some(token) => {
                let _ = next.run(request).await;
                (StatusCode::OK, token).into_response()
            }
            None => StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    /// The real SSE templates plus ordinary authenticated routes, including a
    /// catch-all so URL variants still land on an authenticated route.
    fn app() -> Router {
        let mut router = Router::new();
        for template in SSE_QUERY_TOKEN_ROUTES {
            router = router.route(template, get(ok).post(ok));
        }
        router
            .route("/api/v1/plugins/{name}", get(ok))
            .route("/api/v1/hosted-mocks/{deployment_id}", get(ok))
            .route("/api/v1/{*rest}", get(ok))
            .route_layer(axum::middleware::from_fn(probe_auth))
    }

    async fn send(method: &str, uri: &str, header: Option<(&str, &str)>) -> (StatusCode, String) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some((name, value)) = header {
            builder = builder.header(name, value);
        }
        let response = app()
            .oneshot(builder.body(Body::empty()).expect("valid request"))
            .await
            .expect("infallible router");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("readable body");
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    fn concrete(template: &str) -> String {
        template
            .split('/')
            .map(|seg| if seg.starts_with('{') { "abc-123" } else { seg })
            .collect::<Vec<_>>()
            .join("/")
    }

    #[tokio::test]
    async fn query_token_rejected_on_normal_routes() {
        for path in [
            "/api/v1/plugins/foo",
            "/api/v1/hosted-mocks/abc-123",
            "/api/v1/organizations/abc-123/members",
        ] {
            let (status, _) = send("GET", &format!("{path}?token={JWT}"), None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        }
    }

    #[tokio::test]
    async fn query_token_accepted_on_every_sse_route() {
        for template in SSE_QUERY_TOKEN_ROUTES {
            let uri = format!("{}?foo=1&token={JWT}", concrete(template));
            let (status, body) = send("GET", &uri, None).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert_eq!(body, JWT, "{uri}");
        }
    }

    #[tokio::test]
    async fn percent_encoded_query_token_is_decoded_on_sse_route() {
        let uri = format!("{}?token=a%2Eb%2Ec", concrete(SSE_QUERY_TOKEN_ROUTES[1]));
        let (status, body) = send("GET", &uri, None).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "a.b.c"));
    }

    #[tokio::test]
    async fn header_auth_works_on_normal_and_sse_routes() {
        let bearer = format!("Bearer {JWT}");
        let mut paths = vec!["/api/v1/plugins/foo".to_string()];
        paths.extend(SSE_QUERY_TOKEN_ROUTES.iter().map(|t| concrete(t)));
        for path in paths {
            let (status, body) = send("GET", &path, Some(("Authorization", bearer.as_str()))).await;
            assert_eq!((status, body.as_str()), (StatusCode::OK, JWT), "{path}");
        }
    }

    #[tokio::test]
    async fn session_cookie_still_works_on_normal_route() {
        let cookie = format!("{SESSION_COOKIE}={JWT}");
        let (status, body) =
            send("GET", "/api/v1/plugins/foo", Some(("Cookie", cookie.as_str()))).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, JWT));
    }

    #[tokio::test]
    async fn header_takes_precedence_over_query_on_sse_route() {
        let uri = format!("{}?token=from-query", concrete(SSE_QUERY_TOKEN_ROUTES[0]));
        let bearer = format!("Bearer {JWT}");
        let (status, body) = send("GET", &uri, Some(("Authorization", bearer.as_str()))).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, JWT));
    }

    #[tokio::test]
    async fn api_tokens_are_never_accepted_from_the_query() {
        let uri = format!("{}?token=mfx_longlivedsecret", concrete(SSE_QUERY_TOKEN_ROUTES[0]));
        let (status, _) = send("GET", &uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, body) = send(
            "GET",
            &concrete(SSE_QUERY_TOKEN_ROUTES[0]),
            Some(("Authorization", "Bearer mfx_longlivedsecret")),
        )
        .await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "mfx_longlivedsecret"));
    }

    #[tokio::test]
    async fn non_get_on_sse_route_rejects_query_token() {
        let uri = format!("{}?token={JWT}", concrete(SSE_QUERY_TOKEN_ROUTES[1]));
        let (status, _) = send("POST", &uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn sse_path_variants_do_not_widen_the_exception() {
        let base = concrete("/api/v1/test-runs/{id}/stream");
        let variants = [
            format!("{base}/"),
            format!("{base}/extra"),
            format!("{base}/../../plugins/foo"),
            format!("{base}%2F"),
            format!("{base};x=1"),
            base.replace("/stream", "/%73tream"),
            base.replace("/stream", "/STREAM"),
            base.replace("/api/v1/", "/api/v1//"),
            "/api/v1/plugins/stream".to_string(),
            "/api/v1/stream".to_string(),
            "/api/v1/hosted-mocks/abc-123/runtime-logs/stream/x".to_string(),
            "/api/v1/workspaces/w/test-generation/jobs/stream".to_string(),
        ];
        for path in variants {
            let uri = format!("{path}?token={JWT}");
            let (status, body) = send("GET", &uri, None).await;
            assert_ne!(status, StatusCode::OK, "{uri} authenticated via query");
            assert!(!body.contains(JWT), "{uri}");
        }
    }

    /// Every `.../stream` route registered in `routes.rs` is in the allowlist
    /// and vice versa, so a new SSE route is a deliberate decision.
    #[test]
    fn allowlist_matches_registered_stream_routes() {
        let routes = include_str!("../routes.rs");
        let mut registered: Vec<&str> = routes
            .match_indices("/stream\"")
            .filter_map(|(end, _)| {
                let start = routes[..end].rfind('"')? + 1;
                Some(&routes[start..end + "/stream".len()])
            })
            .collect();
        registered.sort_unstable();
        registered.dedup();
        let mut allowed = SSE_QUERY_TOKEN_ROUTES.to_vec();
        allowed.sort_unstable();
        assert_eq!(registered, allowed);
    }
}

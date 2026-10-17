//! Central scope enforcement for `mfx_` API tokens.
//!
//! `auth_middleware` accepts two credentials on the authenticated route
//! stack: a session JWT (a logged-in user) and an `mfx_` personal access
//! token. Before this module only the plugin-publish handler checked token
//! scopes, so any valid token could deploy, update or delete hosted mocks,
//! manage tokens, run tests and so on regardless of the scopes it was issued
//! with.
//!
//! [`token_scope_middleware`] runs right after `auth_middleware` on every
//! authenticated router. For token requests it:
//!
//! 1. Looks up the matched route template in [`TOKEN_ROUTE_POLICY`] and
//!    requires the declared [`TokenAccess`]. The lookup **fails closed**: a
//!    route with no entry rejects token auth, so a newly added route is
//!    session-only until someone declares a scope for it. The
//!    `every_authenticated_route_is_classified` test additionally fails when a
//!    route in `routes.rs` and the manifest `token_scope_routes.txt` disagree.
//! 2. Binds the request to the token's organization: an `{org_id}` path
//!    parameter or `X-Organization-Id` header naming another org is rejected,
//!    `X-Organization-Slug` is rejected, and `X-Organization-Id` is then set
//!    to the token's org. Handlers resolve their org from that header, so a
//!    token issued for org A cannot act on org B even when its user belongs
//!    to both (and no longer falls back to the user's default org).
//!
//! Session (JWT) requests pass through unchanged.
//!
//! Scope strings are compared exactly against the canonical names
//! (`deploy:mocks`, ...). There is no case folding, trimming, wildcard or
//! implication between scopes (`admin:org` does not imply `deploy:mocks`).
//! Tokens created before this change keep the scopes they were issued with;
//! the column default is `read:packages`, so legacy tokens that relied on the
//! missing check now get a 403 naming the scope they need.

use axum::{
    extract::{MatchedPath, Request},
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::AuthType;
use crate::models::{ApiToken, TokenScope};

/// What an API token needs to call a route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenAccess {
    /// The token must carry this scope.
    Scope(TokenScope),
    /// Any valid token for the org (identity lookups such as `auth/me`).
    AnyToken,
    /// Interactive-session only: API tokens are rejected. Account security,
    /// token management, SSO, GDPR, platform admin and human approvals live
    /// here so a leaked token cannot escalate or take over the account.
    SessionOnly,
}

impl TokenAccess {
    /// Stable name used by the route manifest.
    pub fn manifest_name(&self) -> String {
        match self {
            TokenAccess::Scope(scope) => scope.to_string(),
            TokenAccess::AnyToken => "any-token".to_string(),
            TokenAccess::SessionOnly => "session-only".to_string(),
        }
    }
}

/// Which HTTP methods a policy rule applies to.
#[derive(Debug, Clone, Copy)]
enum Methods {
    Any,
    /// GET and HEAD.
    Read,
    /// Everything except GET and HEAD.
    Write,
    Only(&'static str),
}

impl Methods {
    fn matches(self, method: &Method) -> bool {
        let is_read = method == Method::GET || method == Method::HEAD;
        match self {
            Methods::Any => true,
            Methods::Read => is_read,
            Methods::Write => !is_read,
            Methods::Only(name) => method.as_str() == name,
        }
    }
}

/// How a policy rule matches a route template.
#[derive(Debug, Clone, Copy)]
enum Pattern {
    /// The whole template.
    Exact(&'static str),
    /// The template or anything below it (segment boundary, not substring).
    Prefix(&'static str),
    /// The template ends with this suffix, starting at a segment boundary.
    Suffix(&'static str),
}

impl Pattern {
    fn matches(self, template: &str) -> bool {
        match self {
            Pattern::Exact(p) => template == p,
            Pattern::Prefix(p) => {
                template == p
                    || (template.starts_with(p) && template.as_bytes().get(p.len()) == Some(&b'/'))
            }
            Pattern::Suffix(s) => template.ends_with(s),
        }
    }
}

use Methods::{Any, Only, Read, Write};
use Pattern::{Exact, Prefix, Suffix};
use TokenAccess::{AnyToken, SessionOnly};

const fn scope(s: TokenScope) -> TokenAccess {
    TokenAccess::Scope(s)
}

const DEPLOY: TokenAccess = scope(TokenScope::DeployMocks);
const ADMIN_ORG: TokenAccess = scope(TokenScope::AdminOrg);
const READ_USAGE: TokenAccess = scope(TokenScope::ReadUsage);
const BILLING: TokenAccess = scope(TokenScope::ManageBilling);
const READ_PKG: TokenAccess = scope(TokenScope::ReadPackages);
const PUBLISH_PKG: TokenAccess = scope(TokenScope::PublishPackages);

const ORG: &str = "/api/v1/organizations/{org_id}";

/// Ordered policy table; the first matching rule wins. Anything that matches
/// no rule is rejected for API tokens (see [`classify`]).
static TOKEN_ROUTE_POLICY: &[(Methods, Pattern, TokenAccess)] = &[
    // --- Platform admin, account security, token management: session only.
    (Any, Prefix("/api/v1/admin"), SessionOnly),
    (Only("GET"), Exact("/api/v1/tokens/scopes"), AnyToken),
    (Any, Prefix("/api/v1/tokens"), SessionOnly),
    (Only("GET"), Exact("/api/v1/auth/verify"), AnyToken),
    (Only("GET"), Exact("/api/v1/auth/me"), AnyToken),
    (Any, Prefix("/api/v1/auth"), SessionOnly),
    (Any, Prefix("/api/v1/users"), SessionOnly),
    (Any, Prefix("/api/v1/invitations"), SessionOnly),
    (Any, Prefix("/api/v1/sso"), SessionOnly),
    (Any, Prefix("/api/v1/gdpr"), SessionOnly),
    (Any, Prefix("/api/v1/security"), SessionOnly),
    (Any, Prefix("/api/v1/settings"), SessionOnly),
    (Any, Prefix("/api/v1/analytics"), SessionOnly),
    (Any, Prefix("/api/v1/showcase"), SessionOnly),
    (Any, Prefix("/api/v1/learning"), SessionOnly),
    (Any, Prefix("/api/v1/cloud-plugins"), SessionOnly),
    // --- Billing and usage.
    (Any, Prefix("/api/v1/billing"), BILLING),
    (Only("POST"), Exact("/api/v1/usage/ai-tokens"), DEPLOY),
    (Read, Prefix("/api/v1/usage"), READ_USAGE),
    (Any, Prefix("/api/v1/usage"), SessionOnly),
    (Only("GET"), Exact("/api/v1/ai-studio/quota"), READ_USAGE),
    // --- Packages: plugins and marketplace.
    (Only("POST"), Exact("/api/v1/plugins/publish"), PUBLISH_PKG),
    (Only("POST"), Exact("/api/v1/marketplace/scenarios/publish"), PUBLISH_PKG),
    (Only("POST"), Exact("/api/v1/marketplace/templates/publish"), PUBLISH_PKG),
    (Only("DELETE"), Suffix("/versions/{version}/yank"), PUBLISH_PKG),
    (Any, Suffix("/reviews/{review_id}/respond"), PUBLISH_PKG),
    (Read, Suffix("/star"), READ_PKG),
    // Reviews, votes and stars act as the person, not the publisher.
    (Any, Prefix("/api/v1/plugins"), SessionOnly),
    (Any, Prefix("/api/v1/marketplace"), SessionOnly),
    // --- Organizations: path org is pinned to the token's org (see below).
    (Only("GET"), Exact("/api/v1/organizations"), AnyToken),
    (Any, Exact("/api/v1/organizations"), SessionOnly),
    (Any, Prefix("/api/v1/organizations/slug"), SessionOnly),
    (Only("GET"), Exact(ORG), AnyToken),
    (Only("PATCH"), Exact(ORG), ADMIN_ORG),
    (Any, Exact(ORG), SessionOnly),
    (Read, Exact("/api/v1/organizations/{org_id}/public-keys"), READ_PKG),
    (Read, Prefix("/api/v1/organizations/{org_id}/trust-roots"), READ_PKG),
    (Any, Prefix("/api/v1/organizations/{org_id}/trust-roots"), ADMIN_ORG),
    (Any, Prefix("/api/v1/organizations/{org_id}/members"), ADMIN_ORG),
    (Any, Prefix("/api/v1/organizations/{org_id}/invitations"), ADMIN_ORG),
    (Any, Prefix("/api/v1/organizations/{org_id}/settings"), ADMIN_ORG),
    (Any, Prefix("/api/v1/organizations/{org_id}/audit-logs"), ADMIN_ORG),
    (Read, Exact("/api/v1/organizations/{org_id}/quota"), READ_USAGE),
    (Any, Exact("/api/v1/organizations/{org_id}/quota"), SessionOnly),
    (Read, Prefix("/api/v1/organizations/{org_id}/usage"), READ_USAGE),
    (Read, Prefix("/api/v1/organizations/{org_id}/analytics"), READ_USAGE),
    (Any, Prefix("/api/v1/organizations/{org_id}/billing"), BILLING),
    (Any, Prefix("/api/v1/organizations/{org_id}/mockai"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/test-runs"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/tunnels"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/observability"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/incidents"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/notification-channels"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/routing-rules"), DEPLOY),
    (Any, Prefix("/api/v1/organizations/{org_id}/templates"), DEPLOY),
    // --- Workspaces. Promotion approval is a human review step.
    (Read, Exact("/api/v1/workspaces/{workspace_id}/analytics/pillars"), READ_USAGE),
    (Write, Suffix("/promotions/{promotion_id}/approve"), SessionOnly),
    (Write, Suffix("/promotions/{promotion_id}/reject"), SessionOnly),
    (Any, Prefix("/api/v1/workspaces"), DEPLOY),
    // --- Hosted mocks and the rest of the cloud mock surface.
    (Any, Prefix("/api/v1/hosted-mocks"), DEPLOY),
    (Any, Prefix("/api/v1/ai-studio"), DEPLOY),
    (Any, Prefix("/api/v1/test-suites"), DEPLOY),
    (Any, Prefix("/api/v1/test-runs"), DEPLOY),
    (Any, Prefix("/api/v1/test-schedules"), DEPLOY),
    (Any, Prefix("/api/v1/tunnels"), DEPLOY),
    (Any, Prefix("/api/v1/snapshots"), DEPLOY),
    (Any, Prefix("/api/v1/consistency"), DEPLOY),
    (Any, Prefix("/api/v1/flows"), DEPLOY),
    (Any, Prefix("/api/v1/flow-versions"), DEPLOY),
    (Any, Prefix("/api/v1/observability"), DEPLOY),
    (Any, Prefix("/api/v1/chaos-campaigns"), DEPLOY),
    (Any, Prefix("/api/v1/monitored-services"), DEPLOY),
    (Any, Prefix("/api/v1/contract-diff-runs"), DEPLOY),
    (Any, Prefix("/api/v1/fitness-functions"), DEPLOY),
    (Any, Prefix("/api/v1/verification-suites"), DEPLOY),
    (Any, Prefix("/api/v1/capture-sessions"), DEPLOY),
    (Any, Prefix("/api/v1/clone-models"), DEPLOY),
    (Any, Prefix("/api/v1/incidents"), DEPLOY),
    (Any, Prefix("/api/v1/services"), DEPLOY),
    (Any, Prefix("/api/v1/fixtures"), DEPLOY),
    (Any, Prefix("/api/v1/federation"), DEPLOY),
    (Any, Prefix("/api/v1/scenarios"), DEPLOY),
    (Any, Prefix("/api/v1/import"), DEPLOY),
    (Any, Prefix("/api/v1/dashboard"), DEPLOY),
    (Any, Prefix("/api/v1/projects"), DEPLOY),
];

/// Classify a route for API-token callers. `None` means the route is not
/// declared, which the middleware treats exactly like
/// [`TokenAccess::SessionOnly`] (fail closed).
pub fn classify(method: &Method, template: &str) -> Option<TokenAccess> {
    TOKEN_ROUTE_POLICY
        .iter()
        .find(|(methods, pattern, _)| methods.matches(method) && pattern.matches(template))
        .map(|(_, _, access)| access.clone())
}

fn forbidden(error_code: &str, message: String, hint: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": message,
            "error_code": error_code,
            "status": 403,
            "details": { "hint": hint }
        })),
    )
        .into_response()
}

fn token_not_allowed(method: &Method, template: &str) -> Response {
    forbidden(
        "TOKEN_NOT_ALLOWED",
        format!("API tokens cannot call {} {}", method, template),
        "This operation requires an interactive session. Sign in to the MockForge \
         dashboard or run 'mockforge cloud login'.",
    )
}

fn org_mismatch(message: String) -> Response {
    forbidden(
        "TOKEN_ORG_MISMATCH",
        message,
        "An API token can only act on the organization it was created in.",
    )
}

/// Values of every `{org_id}` parameter in `path`, aligned against the
/// route template.
fn org_id_path_params<'a>(template: &str, path: &'a str) -> Vec<&'a str> {
    template
        .split('/')
        .zip(path.split('/'))
        .filter(|(t, _)| *t == "{org_id}")
        .map(|(_, p)| p)
        .collect()
}

/// Bind a token request to the token's organization. Returns the rejection
/// response if the request names any other org.
fn pin_token_org(request: &mut Request, template: &str, token_org: Uuid) -> Result<(), Response> {
    for raw in org_id_path_params(template, request.uri().path()) {
        if Uuid::parse_str(raw).ok() != Some(token_org) {
            return Err(org_mismatch(format!(
                "This API token belongs to organization {token_org} and cannot access \
                 organization {raw}"
            )));
        }
    }

    let headers = request.headers_mut();
    if headers.contains_key("X-Organization-Slug") {
        return Err(org_mismatch(
            "X-Organization-Slug is not accepted with API tokens; the token is bound to \
             its organization"
                .to_string(),
        ));
    }
    if let Some(value) = headers.get("X-Organization-Id") {
        let requested = value.to_str().ok().and_then(|s| Uuid::parse_str(s.trim()).ok());
        if requested != Some(token_org) {
            return Err(org_mismatch(format!(
                "This API token belongs to organization {token_org}; X-Organization-Id names \
                 a different organization"
            )));
        }
    }
    let pinned = HeaderValue::from_str(&token_org.to_string())
        .map_err(|_| org_mismatch("Invalid token organization".to_string()))?;
    headers.insert("X-Organization-Id", pinned);
    Ok(())
}

/// Enforce [`TOKEN_ROUTE_POLICY`] and org binding for `mfx_` token requests.
///
/// Must be layered *inside* `auth_middleware` (it reads the `AuthType` and
/// `ApiToken` extensions auth sets) and *outside* `rls_org_scope_middleware`
/// (so the org binding it applies is what RLS and the handlers see). Mounted
/// with `route_layer`, so `MatchedPath` is available.
pub async fn token_scope_middleware(mut request: Request, next: Next) -> Response {
    let is_token = matches!(request.extensions().get::<AuthType>(), Some(AuthType::ApiToken));
    if !is_token {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let template = match request.extensions().get::<MatchedPath>() {
        Some(matched) => matched.as_str().to_string(),
        // Without a matched route there is nothing to classify: fail closed.
        None => return token_not_allowed(&method, request.uri().path()),
    };

    let Some(token) = request.extensions().get::<ApiToken>().cloned() else {
        // AuthType says token but no token was attached: fail closed.
        return token_not_allowed(&method, &template);
    };

    match classify(&method, &template) {
        Some(TokenAccess::Scope(required)) => {
            if !token.has_scope(&required) {
                return ApiError::InsufficientScope {
                    required: required.to_string(),
                    scopes: token.scopes.clone(),
                }
                .into_response();
            }
        }
        Some(TokenAccess::AnyToken) => {}
        Some(TokenAccess::SessionOnly) | None => {
            tracing::debug!(%method, %template, "rejecting API token on session-only route");
            return token_not_allowed(&method, &template);
        }
    }

    if let Err(rejection) = pin_token_org(&mut request, &template, token.org_id) {
        return rejection;
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::HeaderMap,
        middleware,
        routing::{delete, get, patch, post},
        Router,
    };
    use std::collections::BTreeMap;
    use tower::ServiceExt;

    const ROUTES_RS: &str = include_str!("../routes.rs");
    const MANIFEST: &str = include_str!("token_scope_routes.txt");

    /// Extract `(METHOD, path)` for every `.route(...)` between `start` and
    /// `end` markers of `routes.rs`.
    fn parse_routes(start: &str, end: &str) -> Vec<(String, String)> {
        let from = ROUTES_RS.find(start).expect("start marker present in routes.rs");
        let to = from + ROUTES_RS[from..].find(end).expect("end marker present in routes.rs");
        let body: String = ROUTES_RS[from..to]
            .lines()
            .map(|l| l.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let bytes = body.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while let Some(rel) = body[i..].find(".route(") {
            let open = i + rel + ".route(".len();
            let mut depth = 1;
            let mut p = open;
            while depth > 0 {
                match bytes[p] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                p += 1;
            }
            let inner = &body[open..p - 1];
            let q1 = inner.find('"').expect("route path literal");
            let q2 = q1 + 1 + inner[q1 + 1..].find('"').expect("route path literal end");
            let path = &inner[q1 + 1..q2];
            let rest = &inner[q2 + 1..];
            for m in ["get", "post", "put", "patch", "delete"] {
                let needle = format!("{m}(");
                let mut j = 0;
                while let Some(k) = rest[j..].find(&needle) {
                    let at = j + k;
                    let prev = rest[..at].chars().last().unwrap_or(' ');
                    if !(prev.is_alphanumeric() || prev == '_') {
                        out.push((m.to_uppercase(), path.to_string()));
                    }
                    j = at + needle.len();
                }
            }
            i = p;
        }
        out
    }

    fn authenticated_routes() -> Vec<(String, String)> {
        let mut routes = parse_routes("let auth_routes = Router::new()", "let sso_public_routes");
        routes.extend(parse_routes("let admin_routes = Router::new()", "// Combine all routes"));
        routes
    }

    fn manifest() -> BTreeMap<(String, String), String> {
        MANIFEST
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                assert_eq!(parts.len(), 3, "manifest line must be 'METHOD PATH ACCESS': {l}");
                ((parts[0].to_string(), parts[1].to_string()), parts[2].to_string())
            })
            .collect()
    }

    /// Every authenticated route must be in the manifest with the access the
    /// policy table actually grants, and the manifest must list nothing that
    /// is not a route. Adding a route without declaring its token access
    /// fails here (and the route rejects tokens at runtime until declared).
    #[test]
    fn every_authenticated_route_is_classified() {
        let routes = authenticated_routes();
        assert!(routes.len() >= 360, "route parser found only {} routes", routes.len());
        let manifest = manifest();
        let mut problems = Vec::new();
        for (method, path) in &routes {
            let m = Method::from_bytes(method.as_bytes()).expect("valid method");
            let actual = classify(&m, path).map(|a| a.manifest_name());
            match manifest.get(&(method.clone(), path.clone())) {
                None => problems.push(format!(
                    "{method} {path}: not in token_scope_routes.txt (policy says {actual:?})"
                )),
                Some(declared) if actual.as_deref() != Some(declared.as_str()) => problems.push(
                    format!("{method} {path}: manifest says {declared}, policy says {actual:?}"),
                ),
                Some(_) => {}
            }
        }
        let route_set: std::collections::BTreeSet<_> = routes.iter().cloned().collect();
        for key in manifest.keys() {
            if !route_set.contains(key) {
                problems.push(format!("{} {}: in manifest but not a route", key.0, key.1));
            }
        }
        assert!(problems.is_empty(), "token scope policy drift:\n{}", problems.join("\n"));
    }

    /// The sensitive families named in the security report map to the scope
    /// they need, never to something a publish/read token holds.
    #[test]
    fn sensitive_routes_map_to_expected_access() {
        let cases: &[(&str, &str, TokenAccess)] = &[
            ("POST", "/api/v1/hosted-mocks", DEPLOY),
            ("PATCH", "/api/v1/hosted-mocks/{deployment_id}/status", DEPLOY),
            ("DELETE", "/api/v1/hosted-mocks/{deployment_id}", DEPLOY),
            ("POST", "/api/v1/hosted-mocks/{deployment_id}/redeploy", DEPLOY),
            ("GET", "/api/v1/hosted-mocks/{deployment_id}/captures/export/har", DEPLOY),
            ("POST", "/api/v1/test-suites/{id}/runs", DEPLOY),
            ("POST", "/api/v1/capture-sessions/{id}/replay", DEPLOY),
            ("POST", "/api/v1/tokens", SessionOnly),
            ("DELETE", "/api/v1/tokens/{token_id}", SessionOnly),
            ("POST", "/api/v1/tokens/{token_id}/rotate", SessionOnly),
            ("POST", "/api/v1/auth/change-password", SessionOnly),
            ("POST", "/api/v1/auth/2fa/disable", SessionOnly),
            ("DELETE", "/api/v1/gdpr/erase", SessionOnly),
            ("POST", "/api/v1/sso/config", SessionOnly),
            ("DELETE", "/api/v1/organizations/{org_id}", SessionOnly),
            ("POST", "/api/v1/organizations/{org_id}/members", ADMIN_ORG),
            ("POST", "/api/v1/billing/checkout", BILLING),
            ("GET", "/api/v1/usage", READ_USAGE),
            ("POST", "/api/v1/plugins/publish", PUBLISH_PKG),
            ("POST", "/api/v1/admin/plugins/{name}/takedown", SessionOnly),
            ("GET", "/api/v1/auth/me", AnyToken),
        ];
        for (method, path, expected) in cases {
            let m = Method::from_bytes(method.as_bytes()).expect("valid method");
            assert_eq!(classify(&m, path).as_ref(), Some(expected), "{method} {path}");
        }
    }

    #[test]
    fn unknown_routes_are_unclassified() {
        assert_eq!(classify(&Method::POST, "/api/v1/brand-new-feature"), None);
        assert_eq!(classify(&Method::GET, "/api/v1/hosted-mocksX"), None);
        assert_eq!(classify(&Method::GET, "/api/v1/tokens-extra"), None);
    }

    // ---- Middleware behaviour -------------------------------------------

    const OK_HANDLER_TAG: &str = "handler-ran";

    fn org_a() -> Uuid {
        Uuid::from_u128(0xA)
    }
    fn org_b() -> Uuid {
        Uuid::from_u128(0xB)
    }

    fn token(org_id: Uuid, scopes: &[&str]) -> ApiToken {
        let now = chrono::Utc::now();
        ApiToken {
            id: Uuid::new_v4(),
            org_id,
            user_id: Some(Uuid::from_u128(0x1)),
            name: "t".into(),
            token_prefix: "mfx_test1234".into(),
            hashed_token: String::new(),
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
            last_used_at: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[derive(Clone)]
    enum Caller {
        Token(ApiToken),
        Session,
    }

    async fn echo_org(headers: HeaderMap) -> String {
        format!(
            "{OK_HANDLER_TAG} org={}",
            headers.get("X-Organization-Id").and_then(|v| v.to_str().ok()).unwrap_or("-")
        )
    }

    /// A router with real route templates from `routes.rs`, the scope layer,
    /// and a stand-in for `auth_middleware` that attaches `caller`.
    fn app(caller: Caller) -> Router {
        let fake_auth = move |mut req: Request, next: Next| {
            let caller = caller.clone();
            async move {
                match caller {
                    Caller::Token(t) => {
                        req.extensions_mut().insert(AuthType::ApiToken);
                        req.extensions_mut().insert(t);
                    }
                    Caller::Session => {
                        req.extensions_mut().insert(AuthType::Jwt);
                    }
                }
                next.run(req).await
            }
        };
        Router::new()
            .route("/api/v1/hosted-mocks", post(echo_org).get(echo_org))
            .route("/api/v1/hosted-mocks/{deployment_id}", delete(echo_org))
            .route("/api/v1/hosted-mocks/{deployment_id}/status", patch(echo_org))
            .route("/api/v1/hosted-mocks/{deployment_id}/redeploy", post(echo_org))
            .route("/api/v1/tokens", post(echo_org).get(echo_org))
            .route("/api/v1/tokens/{token_id}", delete(echo_org))
            .route("/api/v1/test-suites/{id}/runs", post(echo_org))
            .route("/api/v1/organizations/{org_id}/members", post(echo_org))
            .route("/api/v1/organizations/{org_id}/tunnels", get(echo_org))
            .route("/api/v1/plugins/publish", post(echo_org))
            .route("/api/v1/auth/me", get(echo_org))
            .route("/api/v1/brand-new-feature", post(echo_org))
            .route_layer(middleware::from_fn(token_scope_middleware))
            .route_layer(middleware::from_fn(fake_auth))
    }

    async fn call(
        caller: Caller,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
    ) -> (StatusCode, String) {
        let mut builder = axum::http::Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let req = builder.body(Body::empty()).expect("valid request");
        let res = app(caller).oneshot(req).await.expect("infallible router");
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), 1 << 20).await.expect("body");
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    fn dep() -> String {
        Uuid::from_u128(0xD).to_string()
    }

    /// (method, uri) of the mutating cloud routes the report called out.
    fn mock_and_admin_ops() -> Vec<(Method, String)> {
        vec![
            (Method::POST, "/api/v1/hosted-mocks".into()),
            (Method::PATCH, format!("/api/v1/hosted-mocks/{}/status", dep())),
            (Method::DELETE, format!("/api/v1/hosted-mocks/{}", dep())),
            (Method::POST, format!("/api/v1/hosted-mocks/{}/redeploy", dep())),
            (Method::POST, format!("/api/v1/test-suites/{}/runs", dep())),
            (Method::POST, "/api/v1/tokens".into()),
            (Method::GET, "/api/v1/tokens".into()),
            (Method::DELETE, format!("/api/v1/tokens/{}", dep())),
            (Method::POST, format!("/api/v1/organizations/{}/members", org_a())),
        ]
    }

    #[tokio::test]
    async fn publish_or_read_token_cannot_touch_mocks_tokens_tests_or_members() {
        for scopes in [
            vec!["publish:packages"],
            vec!["read:packages"],
            vec![
                "read:packages",
                "publish:packages",
                "read:usage",
                "manage:billing",
            ],
            vec![],
        ] {
            let t = token(org_a(), &scopes);
            for (method, uri) in mock_and_admin_ops() {
                let (status, body) =
                    call(Caller::Token(t.clone()), method.clone(), &uri, &[]).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{scopes:?} {method} {uri}: {body}");
                assert!(
                    !body.contains(OK_HANDLER_TAG),
                    "handler ran for {scopes:?} {method} {uri}"
                );
            }
        }
    }

    #[tokio::test]
    async fn missing_scope_error_names_the_required_scope() {
        let t = token(org_a(), &["publish:packages"]);
        let (status, body) =
            call(Caller::Token(t), Method::POST, "/api/v1/hosted-mocks", &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body.contains("INSUFFICIENT_SCOPE"), "{body}");
        assert!(body.contains("deploy:mocks"), "{body}");
    }

    #[tokio::test]
    async fn scope_string_variants_do_not_grant_access() {
        for variant in [
            "DEPLOY:MOCKS",
            "Deploy:Mocks",
            " deploy:mocks",
            "deploy:mocks ",
            "deploy: mocks",
            "deploy:mocks\n",
            "deploy:*",
            "*",
            "deploy_mocks",
            "deploymocks",
            "admin:org",
        ] {
            let t = token(org_a(), &[variant]);
            let (status, body) =
                call(Caller::Token(t), Method::POST, "/api/v1/hosted-mocks", &[]).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "variant {variant:?} granted access: {body}");
        }
    }

    #[tokio::test]
    async fn correct_scope_still_works_and_pins_org() {
        let t = token(org_a(), &["deploy:mocks"]);
        for (method, uri) in [
            (Method::POST, "/api/v1/hosted-mocks".to_string()),
            (Method::GET, "/api/v1/hosted-mocks".to_string()),
            (Method::PATCH, format!("/api/v1/hosted-mocks/{}/status", dep())),
            (Method::DELETE, format!("/api/v1/hosted-mocks/{}", dep())),
            (Method::POST, format!("/api/v1/hosted-mocks/{}/redeploy", dep())),
            (Method::POST, format!("/api/v1/test-suites/{}/runs", dep())),
            (Method::GET, format!("/api/v1/organizations/{}/tunnels", org_a())),
        ] {
            let (status, body) = call(Caller::Token(t.clone()), method.clone(), &uri, &[]).await;
            assert_eq!(status, StatusCode::OK, "{method} {uri}: {body}");
            assert_eq!(body, format!("{OK_HANDLER_TAG} org={}", org_a()));
        }

        let publisher = token(org_a(), &["publish:packages"]);
        let (status, _) =
            call(Caller::Token(publisher), Method::POST, "/api/v1/plugins/publish", &[]).await;
        assert_eq!(status, StatusCode::OK);

        let admin = token(org_a(), &["admin:org"]);
        let uri = format!("/api/v1/organizations/{}/members", org_a());
        let (status, _) = call(Caller::Token(admin), Method::POST, &uri, &[]).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn token_management_is_session_only_even_with_every_scope() {
        let all = token(
            org_a(),
            &[
                "read:packages",
                "publish:packages",
                "deploy:mocks",
                "admin:org",
                "read:usage",
                "manage:billing",
            ],
        );
        for (method, uri) in [
            (Method::POST, "/api/v1/tokens".to_string()),
            (Method::GET, "/api/v1/tokens".to_string()),
            (Method::DELETE, format!("/api/v1/tokens/{}", dep())),
        ] {
            let (status, body) = call(Caller::Token(all.clone()), method.clone(), &uri, &[]).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
            assert!(body.contains("TOKEN_NOT_ALLOWED"), "{body}");
        }
    }

    #[tokio::test]
    async fn undeclared_route_rejects_tokens_but_not_sessions() {
        let all = token(org_a(), &["deploy:mocks", "admin:org"]);
        let (status, body) =
            call(Caller::Token(all), Method::POST, "/api/v1/brand-new-feature", &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(body.contains("TOKEN_NOT_ALLOWED"));

        let (status, _) =
            call(Caller::Session, Method::POST, "/api/v1/brand-new-feature", &[]).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn session_auth_is_unchanged() {
        for (method, uri) in mock_and_admin_ops() {
            let (status, body) = call(Caller::Session, method.clone(), &uri, &[]).await;
            assert_eq!(status, StatusCode::OK, "{method} {uri}: {body}");
        }
        // Session requests keep their own org selection.
        let other = org_b().to_string();
        let (status, body) = call(
            Caller::Session,
            Method::POST,
            "/api/v1/hosted-mocks",
            &[("X-Organization-Id", &other)],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, format!("{OK_HANDLER_TAG} org={other}"));
    }

    #[tokio::test]
    async fn token_cannot_act_on_another_org() {
        let t = token(org_a(), &["deploy:mocks", "admin:org"]);
        let other = org_b().to_string();

        // Another org in the path.
        for uri in [
            format!("/api/v1/organizations/{other}/members"),
            format!("/api/v1/organizations/{other}/tunnels"),
            "/api/v1/organizations/not-a-uuid/tunnels".to_string(),
        ] {
            let method = if uri.ends_with("members") {
                Method::POST
            } else {
                Method::GET
            };
            let (status, body) = call(Caller::Token(t.clone()), method, &uri, &[]).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
            assert!(body.contains("TOKEN_ORG_MISMATCH"), "{body}");
        }

        // Another org via header, including case/whitespace/garbage variants.
        let upper = other.to_uppercase();
        for (name, value) in [
            ("X-Organization-Id", other.as_str()),
            ("x-organization-id", other.as_str()),
            ("X-Organization-Id", upper.as_str()),
            ("X-Organization-Id", "not-a-uuid"),
            ("X-Organization-Id", ""),
            ("X-Organization-Slug", "someone-else"),
            ("x-organization-slug", "my-own-org"),
        ] {
            let (status, body) = call(
                Caller::Token(t.clone()),
                Method::DELETE,
                &format!("/api/v1/hosted-mocks/{}", dep()),
                &[(name, value)],
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{name}: {value:?} -> {body}");
            assert!(!body.contains(OK_HANDLER_TAG));
        }

        // Matching header is accepted (and stays pinned).
        let own = org_a().to_string();
        let (status, body) = call(
            Caller::Token(t.clone()),
            Method::POST,
            "/api/v1/hosted-mocks",
            &[("X-Organization-Id", &own)],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, format!("{OK_HANDLER_TAG} org={own}"));

        // A token of org B gets org B pinned, never org A.
        let tb = token(org_b(), &["deploy:mocks"]);
        let (_, body) = call(Caller::Token(tb), Method::GET, "/api/v1/hosted-mocks", &[]).await;
        assert_eq!(body, format!("{OK_HANDLER_TAG} org={other}"));
    }

    #[tokio::test]
    async fn empty_scope_token_gets_identity_only() {
        let t = token(org_a(), &[]);
        let (status, _) = call(Caller::Token(t.clone()), Method::GET, "/api/v1/auth/me", &[]).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(Caller::Token(t), Method::GET, "/api/v1/hosted-mocks", &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn org_id_params_align_with_template() {
        assert_eq!(
            org_id_path_params(
                "/api/v1/organizations/{org_id}/members/{user_id}",
                "/api/v1/organizations/x/members/y"
            ),
            vec!["x"]
        );
        assert!(org_id_path_params(
            "/api/v1/hosted-mocks/{deployment_id}",
            "/api/v1/hosted-mocks/x"
        )
        .is_empty());
    }
}

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
//! 1. Looks up the matched route template in `policy::TOKEN_ROUTE_POLICY` and
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
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::error::ApiError;
use crate::middleware::AuthType;
use crate::models::{ApiToken, TokenScope};

mod organization;
mod policy;
use organization::pin_token_org;
pub use policy::classify;

#[cfg(test)]
mod tests;

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

/// Enforce `policy::TOKEN_ROUTE_POLICY` and org binding for `mfx_` token requests.
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

//! Ordered route access rules for API token principals.
use super::TokenAccess;
use crate::models::TokenScope;
use axum::http::Method;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

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
            Any => true,
            Read => is_read,
            Write => !is_read,
            Only(name) => method.as_str() == name,
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
            Exact(p) => template == p,
            Prefix(p) => {
                template == p
                    || (template.starts_with(p) && template.as_bytes().get(p.len()) == Some(&b'/'))
            }
            Suffix(s) => template.ends_with(s),
        }
    }
}

use Methods::{Any, Only, Read, Write};
use Pattern::{Exact, Prefix, Suffix};
use TokenAccess::{AnyToken, SessionOnly};

const fn scope(s: TokenScope) -> TokenAccess {
    TokenAccess::Scope(s)
}

pub(super) const DEPLOY: TokenAccess = scope(TokenScope::DeployMocks);
pub(super) const ADMIN_ORG: TokenAccess = scope(TokenScope::AdminOrg);
pub(super) const READ_USAGE: TokenAccess = scope(TokenScope::ReadUsage);
pub(super) const BILLING: TokenAccess = scope(TokenScope::ManageBilling);
pub(super) const READ_PKG: TokenAccess = scope(TokenScope::ReadPackages);
pub(super) const PUBLISH_PKG: TokenAccess = scope(TokenScope::PublishPackages);

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

/// Exact declared method/path pairs. Prefix rules classify only routes that
/// were explicitly reviewed in the manifest; new descendants fail closed.
static DECLARED_ROUTES: LazyLock<HashMap<&'static str, HashSet<&'static str>>> =
    LazyLock::new(|| {
        let mut routes: HashMap<&str, HashSet<&str>> = HashMap::new();
        for line in include_str!("../token_scope_routes.txt").lines() {
            let mut fields = line.split_whitespace();
            let (Some(method), Some(path), Some(_access)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if method.starts_with('#') || fields.next().is_some() {
                continue;
            }
            routes.entry(method).or_default().insert(path);
        }
        routes
    });

fn is_declared(method: &Method, template: &str) -> bool {
    // Axum automatically serves HEAD through GET handlers. Keep the original
    // policy's method checks after this declaration check.
    let method = if method == Method::HEAD {
        "GET"
    } else {
        method.as_str()
    };
    DECLARED_ROUTES.get(method).is_some_and(|routes| routes.contains(template))
}

/// Classify a route for API-token callers. `None` means the route is not
/// declared, which the middleware treats exactly like
/// [`TokenAccess::SessionOnly`] (fail closed).
pub fn classify(method: &Method, template: &str) -> Option<TokenAccess> {
    if !is_declared(method, template) {
        return None;
    }
    TOKEN_ROUTE_POLICY
        .iter()
        .find(|(methods, pattern, _)| methods.matches(method) && pattern.matches(template))
        .map(|(_, _, access)| access.clone())
}

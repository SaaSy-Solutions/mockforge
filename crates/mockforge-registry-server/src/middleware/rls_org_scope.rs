//! Request-scoped tenant binding for the Postgres RLS backstop (#832 / #960).
//!
//! The RLS policies from migration `20250101000082_rls_tenant_isolation`
//! fail-close every covered-table row (`projects`, `audit_logs`,
//! `hosted_mocks`, `templates`, `scenarios`) unless the connection has
//! `app.current_org_id` bound. Binding it per call site does not scale: the
//! store delegates ~20 covered-table queries to model methods that take an
//! id and no `org_id`, so they have nothing to bind.
//!
//! This middleware resolves the request's org **once**, right after
//! `auth_middleware`, and scopes the whole downstream in the
//! [`CURRENT_ORG`](mockforge_registry_core::store::CURRENT_ORG) task-local.
//! [`with_current_org`](mockforge_registry_core::store::with_current_org) then
//! reads it and binds the GUC on every covered-table query without threading
//! `org_id` through dozens of signatures.
//!
//! ## Mounting
//!
//! Must run **after** `auth_middleware` (it reads the `user_id` that auth
//! stamps into request extensions). On axum's `route_layer` stack, later
//! layers are outer, so this is registered *before* the `auth_middleware`
//! line and therefore executes after it. See `routes.rs`.
//!
//! ## Fail-open on resolution, fail-closed at the database
//!
//! When no org resolves — an unauthenticated route, or a legitimately
//! org-less request like "list the orgs I belong to" — the request proceeds
//! with no task-local set. That is deliberate and safe: `with_current_org`
//! then runs the query with the GUC unbound, and under the `NOBYPASSRLS`
//! runtime role a covered-table query with no org context returns zero rows
//! rather than another tenant's data. The failure mode is an empty result,
//! never a leak.
//!
//! Resolution errors are likewise not turned into HTTP errors here. The
//! handler's own `AuthUser` extractor and permission checks remain the
//! authoritative authorization gate; this middleware only supplies database
//! scoping. Rejecting here would change the status codes of existing
//! endpoints for reasons unrelated to their contract.

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{middleware::resolve_org_context, store::CURRENT_ORG, AppState};

/// Bind the request's organization into the [`CURRENT_ORG`] task-local for the
/// duration of the downstream handler, so RLS-covered queries are org-scoped
/// at the database.
///
/// Also stashes the resolved [`OrgContext`](crate::middleware::OrgContext) in
/// request extensions. `resolve_org_context` checks for it first, so handlers
/// that already call it reuse this resolution instead of repeating the
/// org + membership lookups — this middleware pays for itself rather than
/// adding a round-trip to every authenticated request.
pub async fn rls_org_scope_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // `auth_middleware` stamps the user id as a `String` extension. No user =>
    // nothing to resolve an org from; let the request through unscoped.
    let Some(user_id) = request.extensions().get::<String>().and_then(|s| Uuid::parse_str(s).ok())
    else {
        return next.run(request).await;
    };

    let resolved = resolve_org_context(&state, user_id, &headers, Some(request.extensions())).await;

    match resolved {
        Ok(org_ctx) => {
            let org_id = org_ctx.org_id;
            // Cache for downstream `resolve_org_context` callers.
            request.extensions_mut().insert(org_ctx);
            CURRENT_ORG.scope(org_id, next.run(request)).await
        }
        Err(status) => {
            // Common and expected on org-less routes (e.g. GET
            // /api/v1/organizations, which lists the caller's orgs and has no
            // single org to bind). Debug, not warn: this is not an error path.
            tracing::debug!(
                %user_id,
                path = request.uri().path(),
                ?status,
                "rls_org_scope: no org bound for this request",
            );
            next.run(request).await
        }
    }
}

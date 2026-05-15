//! Past-due "read-only mode" middleware (#449 acceptance criterion 6b).
//!
//! When an organization's subscription has been `past_due` for more than the
//! 24-hour grace window, mutating requests outside the billing/auth allowlist
//! return 402 PaymentRequired. Reads (GET/HEAD/OPTIONS) and the allowlisted
//! "fix your billing" paths stay reachable so the customer can recover without
//! contacting support.
//!
//! Mounted on the authenticated route stack after `auth_middleware` so
//! `request.extensions()` carries the user_id by the time we resolve the org.
//!
//! Defense-in-depth: the deploy handler in `handlers/hosted_mocks.rs` has its
//! own inline check (added in PR #507) that this middleware does not replace.
//! The handler-level check stays as the authoritative deploy gate; this
//! middleware extends the same policy to *every* hot-write route so a future
//! handler can't accidentally serve writes during past_due.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    middleware::resolve_org_context,
    models::{Subscription, SubscriptionStatus},
    AppState,
};

/// Grace period before past_due writes are blocked. Mirrors the constant in
/// `handlers/hosted_mocks.rs::create_deployment` (PR #507) — kept in sync
/// here so the route-wide gate uses the same window as the deploy gate.
const PAST_DUE_GRACE_SECONDS: i64 = 24 * 60 * 60;

/// Path prefixes that remain writable during past_due read-only mode.
///
/// Anything under these prefixes can still receive POST/PUT/PATCH/DELETE
/// while the org is past_due, because they're how the customer recovers:
/// updating payment methods, talking to Stripe, rotating credentials,
/// resetting their password if billing email is locked out, etc. We
/// deliberately *don't* allowlist `/api/v1/users/me` mutations — name
/// changes can wait.
const WRITE_ALLOWLIST_PREFIXES: &[&str] = &[
    "/api/v1/billing/",  // subscription / portal / invoices
    "/api/v1/auth/",     // login, password reset, 2FA setup, change-password
    "/api/v1/support/",  // contact form
    "/api/v1/legal/",    // accept new ToS/DPA before paying
    "/api/v1/waitlist/", // unsubscribe etc.
];

/// Read-only HTTP methods. Always allowed regardless of past_due state.
fn is_read_method(method: &Method) -> bool {
    matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS)
}

/// True when the path is on the recovery allowlist and stays writable.
fn is_write_allowlisted(path: &str) -> bool {
    WRITE_ALLOWLIST_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

/// Block mutating requests when the org's subscription has been past_due for
/// longer than [`PAST_DUE_GRACE_SECONDS`].
///
/// Fail-open on infrastructure errors (DB lookup failures, missing org
/// context): a transient hiccup must not lock paying customers out of their
/// dashboard. The deploy-handler inline check in `hosted_mocks.rs` is the
/// last-resort gate for the high-stakes path.
pub async fn past_due_writes_blocked_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Reads always pass through. Saves us a DB round-trip on the dashboard's
    // overwhelming-majority GET traffic.
    if is_read_method(request.method()) {
        return Ok(next.run(request).await);
    }

    // Recovery paths (billing, auth, support) always pass through. Same
    // rationale as the read shortcut — we want a fast, allocation-free
    // bypass for the hottest exempt routes.
    if is_write_allowlisted(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    // Need a user_id to look up the org. If auth_middleware didn't stamp one,
    // let the request through — the underlying handler will reject it via
    // its own `AuthUser` extractor.
    let Some(user_id) = request.extensions().get::<String>().and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(next.run(request).await);
    };

    let Ok(org_ctx) =
        resolve_org_context(&state, user_id, &headers, Some(request.extensions())).await
    else {
        // No org context resolvable — let the handler's own auth/permission
        // checks surface the right error.
        return Ok(next.run(request).await);
    };

    let pool = state.db.pool();
    let subscription = match Subscription::find_by_org(pool, org_ctx.org_id).await {
        Ok(Some(sub)) => sub,
        Ok(None) => return Ok(next.run(request).await), // free orgs: no subscription, never past_due
        Err(e) => {
            tracing::error!(
                org_id = %org_ctx.org_id,
                "past_due middleware: subscription lookup failed: {}",
                e,
            );
            return Ok(next.run(request).await); // fail open on DB error
        }
    };

    if subscription.status() != SubscriptionStatus::PastDue {
        return Ok(next.run(request).await);
    }

    let elapsed = (chrono::Utc::now() - subscription.updated_at).num_seconds();
    if elapsed <= PAST_DUE_GRACE_SECONDS {
        // Within the 24h grace window — let writes through.
        return Ok(next.run(request).await);
    }

    tracing::warn!(
        org_id = %org_ctx.org_id,
        method = %request.method(),
        path = request.uri().path(),
        past_due_seconds = elapsed,
        "blocking write: subscription past_due beyond 24h grace",
    );

    Err(ApiError::PaymentRequired(
        "Subscription is past due. Update your payment method in the billing portal to resume \
         deploys and other write operations. Reads remain available."
            .to_string(),
    )
    .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_methods_pass() {
        assert!(is_read_method(&Method::GET));
        assert!(is_read_method(&Method::HEAD));
        assert!(is_read_method(&Method::OPTIONS));
    }

    #[test]
    fn write_methods_blocked_unless_allowlisted() {
        assert!(!is_read_method(&Method::POST));
        assert!(!is_read_method(&Method::PUT));
        assert!(!is_read_method(&Method::PATCH));
        assert!(!is_read_method(&Method::DELETE));
    }

    #[test]
    fn billing_path_is_allowlisted() {
        assert!(is_write_allowlisted("/api/v1/billing/checkout"));
        assert!(is_write_allowlisted("/api/v1/billing/portal"));
    }

    #[test]
    fn auth_path_is_allowlisted() {
        assert!(is_write_allowlisted("/api/v1/auth/2fa/setup"));
        assert!(is_write_allowlisted("/api/v1/auth/change-password"));
    }

    #[test]
    fn deploy_path_not_allowlisted() {
        assert!(!is_write_allowlisted("/api/v1/hosted-mocks"));
        assert!(!is_write_allowlisted("/api/v1/workspaces"));
        assert!(!is_write_allowlisted("/api/v1/organizations/abc/members"));
    }

    #[test]
    fn billing_lookalike_not_allowlisted() {
        // Prefix-style allowlist must not be string-contains. A handler at
        // `/api/v1/orgs/{id}/billing-summary` shouldn't bypass the gate.
        assert!(!is_write_allowlisted("/api/v1/orgs/123/billing-summary"));
    }
}

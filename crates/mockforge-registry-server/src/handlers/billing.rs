//! Billing and subscription handlers

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use stripe::{
    BillingPortalSession, CheckoutSession, CheckoutSessionMode,
    CheckoutSessionPaymentMethodCollection, Client, CreateBillingPortalSession,
    CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionSubscriptionData,
    CreateCheckoutSessionSubscriptionDataTrialSettings as CheckoutTrialSettings,
    CreateCheckoutSessionSubscriptionDataTrialSettingsEndBehavior as CheckoutTrialEndBehavior,
    CreateCheckoutSessionSubscriptionDataTrialSettingsEndBehaviorMissingPaymentMethod as MissingTrialPaymentMethod,
    Invoice, ListInvoices,
};
use uuid::Uuid;

use super::stripe_webhook::{
    self, parse_object, Claim, PriceIds, StripeCheckoutSession, StripeInvoice, StripeRef,
    StripeSubscription, WebhookError, WebhookEvent,
};
use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        AuditEventType, Organization, Plan, Subscription, SubscriptionStatus, UsageCounter, User,
    },
    store::with_org_context,
    AppState,
};

/// Public billing-config response: thin shape that the unauthenticated
/// pricing page can use to render trial copy without inheriting the rest
/// of the auth-gated billing surface.
#[derive(Serialize)]
pub struct BillingConfigResponse {
    /// Free-trial length in days for new Pro/Team subscriptions. `0` means
    /// trials are disabled and checkout charges immediately — UI should
    /// hide trial-related copy in that case.
    pub trial_period_days: u32,
    /// Whether annual billing is offered (i.e. at least one annual Stripe
    /// price ID is configured). The pricing/billing UI uses this to decide
    /// whether to render the monthly/annual toggle.
    pub annual_billing_available: bool,
}

/// Public billing config (no auth required).
///
/// Returns just enough metadata for the marketing / pricing UI to render
/// dynamic trial copy. Plan prices stay hard-coded on the client side
/// because they're already tied to operator-set STRIPE_PRICE_ID_* env vars
/// — exposing them here would duplicate the source of truth.
pub async fn get_billing_config(
    State(state): State<AppState>,
) -> ApiResult<Json<BillingConfigResponse>> {
    Ok(Json(BillingConfigResponse {
        trial_period_days: state.config.stripe_trial_period_days,
        annual_billing_available: state.config.stripe_price_id_pro_annual.is_some()
            || state.config.stripe_price_id_team_annual.is_some(),
    }))
}

/// Get current subscription status
pub async fn get_subscription(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<SubscriptionResponse>> {
    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Subscription + current usage, on the request-path (RLS) pool bound to
    // the resolved org.
    let org_id = org_ctx.org_id;
    let (subscription, usage) = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            let subscription = Subscription::find_by_org(&mut **tx, org_id).await?;
            let usage = UsageCounter::get_or_create_current(&mut **tx, org_id).await?;
            Ok((subscription, usage))
        })
    })
    .await?;

    // Effective limits = plan defaults + custom org quota overrides
    let limits = super::usage::effective_limits(&state, &org_ctx.org).await?;

    Ok(Json(SubscriptionResponse {
        org_id: org_ctx.org_id,
        plan: org_ctx.org.plan().to_string(),
        // `plan` above is authoritative. The fields below describe the Stripe
        // subscription and are empty when there is none (Free, or a paid plan
        // assigned administratively); reporting "free" / a fabricated renewal
        // date there contradicted `plan` in the billing UI.
        status: subscription
            .as_ref()
            .map(|s| s.status().to_string())
            .unwrap_or_else(|| NO_SUBSCRIPTION_STATUS.to_string()),
        // Derived from the stored Stripe price ID (no dedicated column) — annual
        // subs are recognised by matching the configured annual price IDs.
        billing_interval: subscription.as_ref().map(|s| {
            interval_label_from_price_id(
                &s.price_id,
                state.config.stripe_price_id_pro_annual.as_deref(),
                state.config.stripe_price_id_team_annual.as_deref(),
            )
            .to_string()
        }),
        cancel_at_period_end: subscription
            .as_ref()
            .map(|s| s.cancel_at_period_end)
            .unwrap_or(false),
        current_period_start: subscription.as_ref().map(|s| s.current_period_start),
        current_period_end: subscription.as_ref().map(|s| s.current_period_end),
        usage: UsageStats {
            requests: usage.requests,
            requests_limit: limits
                .get("requests_per_30d")
                .and_then(|v| v.as_i64())
                .unwrap_or(10000),
            storage_bytes: usage.storage_bytes,
            storage_limit_bytes: limits.get("storage_gb").and_then(|v| v.as_i64()).unwrap_or(1)
                * 1_000_000_000, // Convert GB to bytes
            egress_bytes: usage.egress_bytes,
            // -1 = no plan-defined cap (egress is tracked but not capped today)
            egress_limit_bytes: limits
                .get("egress_gb")
                .and_then(|v| v.as_i64())
                .map(|gb| gb * 1_000_000_000)
                .unwrap_or(-1),
            ai_tokens_used: usage.ai_tokens_used,
            ai_tokens_limit: limits
                .get("ai_tokens_per_month")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        },
        limits,
    }))
}

/// `status` reported by [`get_subscription`] when the org has no Stripe
/// subscription row.
pub const NO_SUBSCRIPTION_STATUS: &str = "none";

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub org_id: Uuid,
    /// The org's plan. Authoritative; `status` and the period fields only
    /// describe the Stripe subscription backing it, if any.
    pub plan: String,
    /// Stripe subscription status, or [`NO_SUBSCRIPTION_STATUS`].
    pub status: String,
    /// Billing cadence of the Stripe subscription: "month" or "year".
    /// `None` when no subscription exists.
    pub billing_interval: Option<String>,
    pub cancel_at_period_end: bool,
    pub current_period_start: Option<chrono::DateTime<chrono::Utc>>,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub usage: UsageStats,
    pub limits: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub requests: i64,
    pub requests_limit: i64,
    pub storage_bytes: i64,
    pub storage_limit_bytes: i64,
    pub egress_bytes: i64,
    pub egress_limit_bytes: i64,
    pub ai_tokens_used: i64,
    pub ai_tokens_limit: i64,
}

/// Billing cadence requested at checkout. The annual price carries the
/// "2 months free" discount (encoded in the Stripe annual price = 10× monthly),
/// so nothing in code computes a discount — we only route to the right price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingInterval {
    Monthly,
    Annual,
}

impl BillingInterval {
    /// Parse the optional `billing_interval` request field. Absent → Monthly
    /// (keeps existing clients working). Returns `None` for unrecognised
    /// values so the caller can 400 rather than guess.
    fn parse(s: Option<&str>) -> Option<Self> {
        match s {
            None | Some("month") | Some("monthly") => Some(Self::Monthly),
            Some("year") | Some("annual") | Some("yearly") => Some(Self::Annual),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Monthly => "month",
            Self::Annual => "year",
        }
    }
}

/// Pick the Stripe price ID for the requested cadence from a plan's
/// (monthly, annual) pair. Returns `None` when the requested cadence isn't
/// configured — the caller turns that into a clear error so an annual checkout
/// never silently falls back to (and charges) the monthly price.
fn select_price_id<'a>(
    interval: BillingInterval,
    monthly: Option<&'a str>,
    annual: Option<&'a str>,
) -> Option<&'a str> {
    match interval {
        BillingInterval::Monthly => monthly,
        BillingInterval::Annual => annual,
    }
}

/// Session metadata is not inherited by the Subscription. Include ownership on
/// both objects so subscription webhooks can resolve the organization even when
/// trials are disabled.
fn subscription_checkout_params<'a>(
    org_id: &'a str,
    plan: &str,
    interval: BillingInterval,
    price_id: String,
    success_url: &'a str,
    cancel_url: &'a str,
    trial_period_days: u32,
) -> CreateCheckoutSession<'a> {
    let metadata = std::collections::HashMap::from([
        ("org_id".to_string(), org_id.to_string()),
        ("plan".to_string(), plan.to_string()),
        ("billing_interval".to_string(), interval.as_str().to_string()),
    ]);
    let mut params = CreateCheckoutSession::new();
    params.success_url = Some(success_url);
    params.cancel_url = Some(cancel_url);
    params.mode = Some(CheckoutSessionMode::Subscription);
    // A positive trial starts without a card. Paid checkout without a
    // trial still collects one; this policy is explicit in both cases.
    let has_trial = trial_period_days > 0;
    params.payment_method_collection = Some(if has_trial {
        CheckoutSessionPaymentMethodCollection::IfRequired
    } else {
        CheckoutSessionPaymentMethodCollection::Always
    });
    params.client_reference_id = Some(org_id);
    params.metadata = Some(metadata.clone());
    params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        price: Some(price_id),
        quantity: Some(1),
        ..Default::default()
    }]);
    params.subscription_data = Some(CreateCheckoutSessionSubscriptionData {
        metadata: Some(metadata),
        // Stripe requires a positive trial length; omit the field for zero.
        trial_period_days: has_trial.then_some(trial_period_days),
        // No payment method at trial end cancels the subscription instead
        // of creating an invoice. Existing subscription-deleted handling
        // then returns the organization to its Free entitlements.
        trial_settings: has_trial.then_some(CheckoutTrialSettings {
            end_behavior: CheckoutTrialEndBehavior {
                missing_payment_method: MissingTrialPaymentMethod::Cancel,
            },
        }),
        ..Default::default()
    });
    params
}

/// Create Stripe checkout session
/// This would typically redirect to Stripe Checkout
#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String, // "pro" or "team"
    /// Billing cadence: "month" (default) or "year". Absent = monthly.
    pub billing_interval: Option<String>,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    pub checkout_url: String,
    pub session_id: String,
}

pub async fn create_checkout(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateCheckoutRequest>,
) -> ApiResult<Json<CreateCheckoutResponse>> {
    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Validate plan
    let plan = match request.plan.as_str() {
        "pro" => Plan::Pro,
        "team" => Plan::Team,
        _ => {
            return Err(ApiError::InvalidRequest(
                "Invalid plan. Must be 'pro' or 'team'".to_string(),
            ))
        }
    };

    // Get Stripe client
    let stripe_secret = state
        .config
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| ApiError::InvalidRequest("Stripe not configured".to_string()))?;
    let client = Client::new(stripe_secret);

    // Resolve the requested billing cadence (monthly default).
    let interval =
        BillingInterval::parse(request.billing_interval.as_deref()).ok_or_else(|| {
            ApiError::InvalidRequest(
                "Invalid billing_interval. Must be 'month' or 'year'".to_string(),
            )
        })?;

    // Get the (monthly, annual) price-ID pair for the plan, then pick by cadence.
    let (monthly, annual) = match plan {
        Plan::Pro => (
            state.config.stripe_price_id_pro.as_deref(),
            state.config.stripe_price_id_pro_annual.as_deref(),
        ),
        Plan::Team => (
            state.config.stripe_price_id_team.as_deref(),
            state.config.stripe_price_id_team_annual.as_deref(),
        ),
        Plan::Free => {
            return Err(ApiError::InvalidRequest(
                "Cannot create checkout for free plan".to_string(),
            ))
        }
    };
    // `None` here means the requested cadence isn't configured. Erroring (rather
    // than falling back to the other cadence) guarantees we never charge a
    // customer monthly when they asked for annual, or vice versa.
    let price_id = select_price_id(interval, monthly, annual)
        .ok_or_else(|| {
            ApiError::InvalidRequest(format!(
                "{} billing is not configured for the {} plan",
                interval.as_str(),
                plan
            ))
        })?
        .to_string();

    // Build success and cancel URLs
    let success_url = request.success_url.unwrap_or_else(|| {
        format!(
            "{}/billing/success?session_id={{CHECKOUT_SESSION_ID}}",
            state.config.app_base_url
        )
    });
    let cancel_url = request
        .cancel_url
        .unwrap_or_else(|| format!("{}/billing/cancel", state.config.app_base_url));

    // Create checkout session
    let org_id_str = org_ctx.org_id.to_string();
    let plan_str = plan.to_string();

    let checkout_params = subscription_checkout_params(
        &org_id_str,
        &plan_str,
        interval,
        price_id,
        &success_url,
        &cancel_url,
        state.config.stripe_trial_period_days,
    );

    // Create the checkout session
    let session = CheckoutSession::create(&client, checkout_params)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Stripe error: {}", e)))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::BillingCheckout,
            format!("Checkout session created for {} plan ({})", request.plan, interval.as_str()),
            Some(serde_json::json!({
                "plan": request.plan,
                "billing_interval": interval.as_str(),
                "session_id": session.id.to_string(),
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(CreateCheckoutResponse {
        checkout_url: session
            .url
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Stripe session missing URL")))?
            .to_string(),
        session_id: session.id.to_string(),
    }))
}

/// Create Stripe Customer Portal session
/// Allows users to manage subscription, payment methods, and view invoices
#[derive(Debug, Deserialize)]
pub struct CreatePortalRequest {
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatePortalResponse {
    pub portal_url: String,
}

pub async fn create_portal_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreatePortalRequest>,
) -> ApiResult<Json<CreatePortalResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Only org owners can access the billing portal
    if org_ctx.org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // Require a Stripe customer ID
    let stripe_customer_id = org_ctx.org.stripe_customer_id.as_ref().ok_or_else(|| {
        ApiError::InvalidRequest(
            "No billing account found. Please subscribe to a plan first.".to_string(),
        )
    })?;

    // Get Stripe client
    let stripe_secret = state
        .config
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| ApiError::InvalidRequest("Stripe not configured".to_string()))?;
    let client = Client::new(stripe_secret);

    let return_url = request
        .return_url
        .unwrap_or_else(|| format!("{}/billing", state.config.app_base_url));

    let customer_id = stripe_customer_id
        .parse()
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Invalid Stripe customer ID")))?;

    let mut params = CreateBillingPortalSession::new(customer_id);
    params.return_url = Some(&return_url);

    let session = BillingPortalSession::create(&client, params)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Stripe portal error: {}", e)))?;

    Ok(Json(CreatePortalResponse {
        portal_url: session.url,
    }))
}

/// Single invoice line in the API response (subset of Stripe's full Invoice).
#[derive(Debug, Serialize)]
pub struct InvoiceItem {
    pub id: String,
    pub number: Option<String>,
    pub status: Option<String>,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub currency: Option<String>,
    pub created: Option<i64>,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListInvoicesResponse {
    pub org_id: Uuid,
    pub invoices: Vec<InvoiceItem>,
}

/// GET /api/v1/billing/invoices
///
/// Lists the org's invoices via Stripe. Returns an empty list if the org has
/// no Stripe customer ID yet (e.g. free tier never upgraded).
pub async fn list_invoices(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<ListInvoicesResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Owner-only — invoices contain financial info
    if org_ctx.org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // No customer yet → empty list (not an error). Same for un-configured Stripe.
    let stripe_customer_id = match org_ctx.org.stripe_customer_id.as_ref() {
        Some(id) => id,
        None => {
            return Ok(Json(ListInvoicesResponse {
                org_id: org_ctx.org_id,
                invoices: vec![],
            }));
        }
    };
    let stripe_secret = match state.config.stripe_secret_key.as_ref() {
        Some(s) => s,
        None => {
            return Ok(Json(ListInvoicesResponse {
                org_id: org_ctx.org_id,
                invoices: vec![],
            }));
        }
    };

    let customer_id = stripe_customer_id
        .parse()
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Invalid Stripe customer ID")))?;
    let client = Client::new(stripe_secret);

    let mut params = ListInvoices::new();
    params.customer = Some(customer_id);
    params.limit = Some(20);

    let list = Invoice::list(&client, &params)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Stripe invoices error: {}", e)))?;

    let invoices = list
        .data
        .into_iter()
        .map(|inv| InvoiceItem {
            id: inv.id.to_string(),
            number: inv.number,
            status: inv.status.map(|s| s.as_str().to_string()),
            amount_due: inv.amount_due.unwrap_or(0),
            amount_paid: inv.amount_paid.unwrap_or(0),
            currency: inv.currency.map(|c| c.to_string()),
            created: inv.created,
            period_start: inv.period_start,
            period_end: inv.period_end,
            hosted_invoice_url: inv.hosted_invoice_url,
            invoice_pdf: inv.invoice_pdf,
        })
        .collect();

    Ok(Json(ListInvoicesResponse {
        org_id: org_ctx.org_id,
        invoices,
    }))
}

/// Stripe webhook handler (#1456).
///
/// Status codes:
/// - 400: the request is not authenticated (missing or bad signature).
/// - 200: the event was applied, was already processed (Stripe redelivers),
///   or can never be acted on (unknown type, missing `org_id`, unknown
///   subscription). A non-2xx for those would make Stripe retry for days.
/// - 422: the event was rejected and alerted, e.g. an unknown price id. Stripe
///   retries it, so it applies once the operator configures the price.
/// - 500: a transient failure; Stripe retries.
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let webhook_secret = state.config.stripe_webhook_secret.as_ref().ok_or_else(|| {
        ApiError::InvalidRequest("Stripe webhook secret not configured".to_string())
    })?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::InvalidRequest("Missing stripe-signature header".to_string()))?;

    let body_str = std::str::from_utf8(&body)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid UTF-8 in webhook body: {}", e)))?;

    // Verify the signature ourselves rather than via `stripe::Webhook`, which
    // also deserializes the whole event into types pinned to one API version.
    stripe_webhook::verify_signature(
        body_str,
        signature,
        webhook_secret,
        chrono::Utc::now().timestamp(),
    )
    .map_err(|e| ApiError::InvalidRequest(format!("Webhook signature verification failed: {e}")))?;

    let event: WebhookEvent = match serde_json::from_str(body_str) {
        Ok(event) => event,
        Err(e) => {
            tracing::error!("Signed Stripe webhook body is not an event: {e}");
            return Ok(acknowledge(
                serde_json::json!({ "received": true, "ignored": "not an event" }),
            ));
        }
    };

    let pool = state.db.pool();
    if stripe_webhook::claim_event(pool, &event.id, &event.event_type).await? == Claim::Duplicate {
        tracing::info!(
            event_id = %event.id,
            event_type = %event.event_type,
            "Stripe event already processed or in flight; skipping"
        );
        return Ok(acknowledge(serde_json::json!({ "received": true, "duplicate": true })));
    }

    let event_id = event.id;
    let event_type = event.event_type;
    let api_version = event.api_version.unwrap_or_default();
    match dispatch_event(pool, &state.config, &event_type, event.data.object).await {
        Ok(()) => {
            stripe_webhook::finish_event(pool, &event_id, "applied").await?;
            Ok(acknowledge(serde_json::json!({ "received": true })))
        }
        Err(WebhookError::Ignored(reason)) => {
            tracing::warn!(
                event_id = %event_id,
                event_type = %event_type,
                api_version = %api_version,
                "Stripe event not actionable, acknowledged: {reason}"
            );
            stripe_webhook::finish_event(pool, &event_id, &format!("ignored: {reason}")).await?;
            Ok(acknowledge(serde_json::json!({ "received": true, "ignored": reason })))
        }
        Err(WebhookError::Rejected(reason)) => {
            tracing::error!(
                alert = "billing",
                event_id = %event_id,
                event_type = %event_type,
                "Stripe event rejected: {reason}"
            );
            release_claim(pool, &event_id).await;
            Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "received": false, "error": reason })),
            )
                .into_response())
        }
        Err(WebhookError::Failed(e)) => {
            tracing::error!(
                event_id = %event_id,
                event_type = %event_type,
                "Stripe event failed, Stripe will retry: {e}"
            );
            release_claim(pool, &event_id).await;
            Err(e)
        }
    }
}

fn acknowledge(body: serde_json::Value) -> Response {
    (StatusCode::OK, Json(body)).into_response()
}

/// Release a claim after a failed attempt. If this fails too, the claim
/// expires after five minutes and the retry after that is processed.
async fn release_claim(pool: &sqlx::PgPool, event_id: &str) {
    if let Err(e) = stripe_webhook::release_event(pool, event_id).await {
        tracing::warn!(event_id = %event_id, "Could not release Stripe event claim: {e}");
    }
}

async fn dispatch_event(
    pool: &sqlx::PgPool,
    config: &crate::config::Config,
    event_type: &str,
    object: serde_json::Value,
) -> Result<(), WebhookError> {
    match event_type {
        "checkout.session.completed" => {
            handle_checkout_completed(pool, &parse_object(object)?).await
        }
        "customer.subscription.created" | "customer.subscription.updated" => {
            handle_subscription_event(pool, &parse_object(object)?, config).await
        }
        // Stripe sends this three days before a trial ends (#714).
        "customer.subscription.trial_will_end" => {
            handle_trial_will_end(pool, &parse_object(object)?).await
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(pool, &parse_object(object)?).await
        }
        "invoice.payment_succeeded" => handle_payment_succeeded(pool, &parse_object(object)?).await,
        "invoice.payment_failed" => handle_payment_failed(pool, &parse_object(object)?).await,
        other => Err(WebhookError::Ignored(format!("unhandled event type {other}"))),
    }
}

fn unix_to_utc(ts: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
}

/// Turn a failed owner/org lookup into a webhook outcome: a database error is
/// retried, a missing org or owner can never be fixed by a retry.
fn lookup_error(e: ApiError) -> WebhookError {
    match e {
        ApiError::Database(_) => WebhookError::Failed(e),
        other => WebhookError::Ignored(other.to_string()),
    }
}

async fn find_org(pool: &sqlx::PgPool, org_id: Uuid) -> Result<Organization, WebhookError> {
    Organization::find_by_id(pool, org_id)
        .await?
        .ok_or_else(|| WebhookError::Ignored(format!("organization {org_id} not found")))
}

/// Handle checkout session completed
async fn handle_checkout_completed(
    pool: &sqlx::PgPool,
    session: &StripeCheckoutSession,
) -> Result<(), WebhookError> {
    // Extract org_id from client_reference_id
    let org_id_str = session.client_reference_id.as_deref().ok_or_else(|| {
        WebhookError::Ignored("missing client_reference_id in checkout session".to_string())
    })?;

    let org_id = Uuid::parse_str(org_id_str).map_err(|_| {
        WebhookError::Ignored(format!("invalid org_id {org_id_str:?} in checkout session"))
    })?;

    let subscription_id = session.subscription.as_ref().map(StripeRef::id).ok_or_else(|| {
        WebhookError::Ignored("missing subscription in checkout session".to_string())
    })?;

    tracing::info!("Checkout completed: org_id={}, subscription_id={}", org_id, subscription_id);

    // The subscription will be handled by the subscription.created/updated webhook
    // But we can update the org's stripe_customer_id here if needed
    if let Some(customer) = &session.customer {
        Organization::update_stripe_customer_id(pool, org_id, Some(customer.id())).await?;
    }

    Ok(())
}

/// Get organization owner's email for sending notifications.
///
/// Returns `(email, username, email_notifications)`. Callers should skip
/// non-critical sends when `email_notifications` is false.
async fn get_org_owner_email(
    pool: &sqlx::PgPool,
    org_id: Uuid,
) -> Result<(String, String, bool), ApiError> {
    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get owner user
    let owner = User::find_by_id(pool, org.owner_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Organization owner not found".to_string()))?;

    Ok((owner.email, owner.username, owner.email_notifications))
}

async fn handle_subscription_event(
    pool: &sqlx::PgPool,
    subscription: &StripeSubscription,
    config: &crate::config::Config,
) -> Result<(), WebhookError> {
    let stripe_sub_id = subscription.id.as_str();
    let stripe_customer_id = subscription.customer.id();

    // Subscription ownership is carried by subscription_data.metadata at checkout.
    let org_id = subscription
        .metadata
        .get("org_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            WebhookError::Ignored(format!(
                "missing or invalid org_id in metadata of subscription {stripe_sub_id}"
            ))
        })?;

    let price_id = subscription.price_id().ok_or_else(|| {
        WebhookError::Ignored(format!("subscription {stripe_sub_id} has no price"))
    })?;

    // Exact match only (#1456): an unknown price must not grant a plan by
    // guessing from its text, nor silently drop the org to Free.
    let plan = PriceIds::from_config(config).plan_for(price_id).ok_or_else(|| {
        WebhookError::Rejected(format!(
            "unknown price id {price_id} on subscription {stripe_sub_id} (org {org_id}); \
             configure it in STRIPE_PRICE_ID_* or fix the Stripe product"
        ))
    })?;

    // Read from the subscription or its items, whichever the API version uses.
    let (period_start, period_end) = subscription.period().ok_or_else(|| {
        WebhookError::Ignored(format!(
            "subscription {stripe_sub_id} has no current period on it or its items"
        ))
    })?;
    let (current_period_start, current_period_end) =
        match (unix_to_utc(period_start), unix_to_utc(period_end)) {
            (Some(start), Some(end)) => (start, end),
            _ => {
                return Err(WebhookError::Ignored(format!(
                    "subscription {stripe_sub_id} has an out-of-range period"
                )))
            }
        };

    let status = SubscriptionStatus::from_string(&subscription.status);
    let cancel_at_period_end = subscription.cancel_at_period_end;
    let canceled_at = subscription.canceled_at.and_then(unix_to_utc);

    // Get old plan before updating; an org that no longer exists is ignored.
    let old_plan = find_org(pool, org_id).await?.plan();

    // Upsert subscription
    Subscription::upsert_from_stripe(
        pool,
        org_id,
        stripe_sub_id,
        stripe_customer_id,
        price_id,
        plan,
        status,
        current_period_start,
        current_period_end,
        cancel_at_period_end,
        canceled_at,
    )
    .await?;

    // Update organization plan
    Organization::update_plan(pool, org_id, plan).await?;

    // Update stripe_customer_id on org
    Organization::update_stripe_customer_id(pool, org_id, Some(stripe_customer_id)).await?;

    tracing::info!("Subscription updated: org_id={}, plan={:?}", org_id, plan);

    // Record audit log (webhook event, so no user_id or IP).
    //
    // #873 note: `OrgPlanChanged` is intentionally NOT emitted here. Every
    // plan transition on this webhook path is already logged as the more
    // specific `BillingUpgrade` / `BillingDowngrade` below, so adding
    // `OrgPlanChanged` would double-log the same event. `OrgPlanChanged`
    // remains available for non-billing plan mutations should any ever exist.
    if old_plan != plan {
        // Determine if this is an upgrade or downgrade based on plan ordering
        // Free < Pro < Team
        let plan_order = |p: Plan| -> u8 {
            match p {
                Plan::Free => 0,
                Plan::Pro => 1,
                Plan::Team => 2,
            }
        };
        let event_type = if plan_order(plan) > plan_order(old_plan) {
            AuditEventType::BillingUpgrade
        } else {
            AuditEventType::BillingDowngrade
        };

        crate::models::record_audit_event(
            pool,
            org_id,
            None, // Webhook event, no user context
            event_type,
            format!("Subscription plan changed from {:?} to {:?}", old_plan, plan),
            Some(serde_json::json!({
                "old_plan": old_plan.to_string(),
                "new_plan": plan.to_string(),
                "subscription_id": stripe_sub_id,
                "status": status.to_string(),
            })),
            None, // Webhook event, no IP
            None, // Webhook event, no user agent
        )
        .await;
    }

    // Send subscription confirmation email (non-blocking, gated on owner pref)
    if let Ok((email_addr, username, email_notifications)) = get_org_owner_email(pool, org_id).await
    {
        if email_notifications {
            if let Ok(email_service) = EmailService::from_env() {
                // Convert cents to dollars
                let amount = subscription.unit_amount().map(|amount| amount as f64 / 100.0);

                let plan_str = plan.to_string();
                let email_msg = EmailService::generate_subscription_confirmation(
                    &username,
                    &email_addr,
                    &plan_str,
                    amount,
                    Some(current_period_end),
                );

                tokio::spawn(async move {
                    if let Err(e) = email_service.send(email_msg).await {
                        tracing::warn!("Failed to send subscription confirmation email: {}", e);
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_subscription_deleted(
    pool: &sqlx::PgPool,
    subscription: &StripeSubscription,
) -> Result<(), WebhookError> {
    let stripe_sub_id = subscription.id.as_str();

    // Find subscription
    let subscription_record = Subscription::find_by_stripe_subscription_id(pool, stripe_sub_id)
        .await?
        .ok_or_else(|| {
            WebhookError::Ignored(format!("subscription {stripe_sub_id} not found in database"))
        })?;

    // Get the organization before downgrading to log the old plan
    let old_plan = find_org(pool, subscription_record.org_id).await?.plan();

    // Update status to canceled
    Subscription::update_status(pool, subscription_record.id, SubscriptionStatus::Canceled).await?;

    // TODO(#870-followup): cancellation fairness. This downgrades the org to
    // Free immediately on `customer.subscription.deleted`, ignoring
    // `cancel_at_period_end` / `current_period_end` — a user who cancels
    // mid-period loses paid features before the period they already paid for
    // ends. The fix is to keep the paid plan until period end, then downgrade.
    // Tracked separately from the entitlement-integrity work (#870); out of
    // scope for the effective-plan gating change.
    // Downgrade org to free plan
    Organization::update_plan(pool, subscription_record.org_id, Plan::Free).await?;

    tracing::info!("Subscription canceled: org_id={}", subscription_record.org_id);

    // Record audit log (webhook event, so no user_id or IP)
    crate::models::record_audit_event(
        pool,
        subscription_record.org_id,
        None, // Webhook event, no user context
        AuditEventType::BillingCanceled,
        format!("Subscription canceled for plan: {:?}", old_plan),
        Some(serde_json::json!({
            "subscription_id": stripe_sub_id,
            "plan": old_plan.to_string(),
        })),
        None, // Webhook event, no IP
        None, // Webhook event, no user agent
    )
    .await;

    // Send subscription canceled email (non-blocking, gated on owner pref)
    if let Ok((email_addr, username, email_notifications)) =
        get_org_owner_email(pool, subscription_record.org_id).await
    {
        if email_notifications {
            if let Ok(email_service) = EmailService::from_env() {
                let plan_str = old_plan.to_string();
                let email_msg = EmailService::generate_subscription_canceled(
                    &username,
                    &email_addr,
                    &plan_str,
                    Some(subscription_record.current_period_end),
                );

                tokio::spawn(async move {
                    if let Err(e) = email_service.send(email_msg).await {
                        tracing::warn!("Failed to send subscription canceled email: {}", e);
                    }
                });
            }
        }
    }

    Ok(())
}

/// Email the org owner that their trial ends soon (#714).
///
/// A subscription we have no record of is acknowledged rather than failed: a
/// retry would not create the record, and Stripe would keep redelivering a
/// reminder nobody can receive.
async fn handle_trial_will_end(
    pool: &sqlx::PgPool,
    subscription: &StripeSubscription,
) -> Result<(), WebhookError> {
    let stripe_sub_id = subscription.id.as_str();
    let record = Subscription::find_by_stripe_subscription_id(pool, stripe_sub_id)
        .await?
        .ok_or_else(|| {
            WebhookError::Ignored(format!(
                "trial_will_end for unknown subscription {stripe_sub_id}"
            ))
        })?;

    let (email_addr, username, email_notifications) =
        get_org_owner_email(pool, record.org_id).await.map_err(lookup_error)?;
    tracing::info!("Trial ending soon: org_id={}", record.org_id);
    if !email_notifications {
        return Ok(());
    }
    let Ok(email_service) = EmailService::from_env() else {
        return Ok(());
    };

    let org = find_org(pool, record.org_id).await?;
    let amount = subscription.unit_amount().map(|cents| cents as f64 / 100.0);
    let trial_end = subscription.trial_end.and_then(unix_to_utc);

    let plan_name = match org.plan() {
        Plan::Free => "Free",
        Plan::Pro => "Pro",
        Plan::Team => "Team",
    };
    let email_msg =
        EmailService::generate_trial_ending(&username, &email_addr, plan_name, amount, trial_end);
    tokio::spawn(async move {
        if let Err(e) = email_service.send(email_msg).await {
            tracing::warn!("Failed to send trial ending email: {}", e);
        }
    });

    Ok(())
}

/// The newest subscription for an invoice's customer. An invoice with no
/// customer, or a customer we hold no subscription for, is not actionable.
async fn subscription_for_invoice(
    pool: &sqlx::PgPool,
    invoice: &StripeInvoice,
) -> Result<Subscription, WebhookError> {
    let customer_id =
        invoice.customer.as_ref().map(StripeRef::id).ok_or_else(|| {
            WebhookError::Ignored(format!("invoice {} has no customer", invoice.id))
        })?;

    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE stripe_customer_id = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        WebhookError::Ignored(format!("no subscription for customer {customer_id}"))
    })
}

/// Handle payment succeeded
async fn handle_payment_succeeded(
    pool: &sqlx::PgPool,
    invoice: &StripeInvoice,
) -> Result<(), WebhookError> {
    let subscription = subscription_for_invoice(pool, invoice).await?;

    // Update status to active if it was past_due
    if subscription.status() == SubscriptionStatus::PastDue {
        Subscription::update_status(pool, subscription.id, SubscriptionStatus::Active).await?;
    }

    tracing::info!("Payment succeeded: org_id={}", subscription.org_id);

    Ok(())
}

async fn handle_payment_failed(
    pool: &sqlx::PgPool,
    invoice: &StripeInvoice,
) -> Result<(), WebhookError> {
    let subscription = subscription_for_invoice(pool, invoice).await?;

    // Update status to past_due
    Subscription::update_status(pool, subscription.id, SubscriptionStatus::PastDue).await?;

    tracing::info!("Payment failed: org_id={}", subscription.org_id);

    // Record audit log (webhook event, so no user_id or IP).
    // #873: use the dedicated PaymentFailed event instead of reusing
    // BillingCanceled — a failed payment puts the sub in `past_due` dunning,
    // which is distinct from an actual cancellation.
    crate::models::record_audit_event(
        pool,
        subscription.org_id,
        None, // Webhook event, no user context
        AuditEventType::PaymentFailed,
        format!("Payment failed for subscription: {}", invoice.id),
        Some(serde_json::json!({
            "invoice_id": invoice.id,
            "amount_due": invoice.amount_due,
            "subscription_id": subscription.stripe_subscription_id,
        })),
        None, // Webhook event, no IP
        None, // Webhook event, no user agent
    )
    .await;

    // Send payment failed email (non-blocking, gated on owner pref)
    if let Ok((email_addr, username, email_notifications)) =
        get_org_owner_email(pool, subscription.org_id).await
    {
        if email_notifications {
            if let Ok(email_service) = EmailService::from_env() {
                let org = find_org(pool, subscription.org_id).await?;

                let amount = invoice.amount_due.map(|a| a as f64 / 100.0).unwrap_or(0.0);
                let retry_date = invoice.next_payment_attempt.and_then(unix_to_utc);

                let plan_str = org.plan().to_string();
                let email_msg = EmailService::generate_payment_failed(
                    &username,
                    &email_addr,
                    &plan_str,
                    amount,
                    retry_date,
                );

                tokio::spawn(async move {
                    if let Err(e) = email_service.send(email_msg).await {
                        tracing::warn!("Failed to send payment failed email: {}", e);
                    }
                });
            }
        }
    }

    Ok(())
}

/// Derive the billing cadence label ("month" / "year") of an active
/// subscription from its stored Stripe price ID. Annual subscriptions are
/// identified by matching the configured annual price IDs; everything else
/// (including unknown / legacy IDs) is treated as monthly. This avoids
/// persisting a redundant column — the stored `price_id` is the source of truth.
fn interval_label_from_price_id(
    price_id: &str,
    pro_annual: Option<&str>,
    team_annual: Option<&str>,
) -> &'static str {
    if Some(price_id) == pro_annual || Some(price_id) == team_annual {
        BillingInterval::Annual.as_str()
    } else {
        BillingInterval::Monthly.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_serializes_subscription_ownership_with_and_without_trial() {
        let org_id = "04caad30-8aae-4506-a73c-3f306c0bbc0e";
        for trial_days in [0, 14] {
            for (plan, interval, price) in [
                ("pro", BillingInterval::Monthly, "price_pro_month"),
                ("team", BillingInterval::Annual, "price_team_year"),
            ] {
                let params = subscription_checkout_params(
                    org_id,
                    plan,
                    interval,
                    price.to_string(),
                    "https://app.example.test/billing/success",
                    "https://app.example.test/billing/cancel",
                    trial_days,
                );
                let body = serde_json::to_value(params).unwrap();
                assert_eq!(body["mode"], "subscription");
                assert_eq!(body["client_reference_id"], org_id);
                assert_eq!(body["metadata"]["org_id"], org_id);
                assert_eq!(body["subscription_data"]["metadata"]["org_id"], org_id);
                assert_eq!(body["subscription_data"]["metadata"]["plan"], plan);
                assert_eq!(
                    body["subscription_data"]["metadata"]["billing_interval"],
                    interval.as_str()
                );
                assert_eq!(body["line_items"][0]["price"], price);
                assert_eq!(body["line_items"][0]["quantity"], 1);
                if trial_days == 0 {
                    assert!(body["subscription_data"].get("trial_period_days").is_none());
                } else {
                    assert_eq!(body["subscription_data"]["trial_period_days"], trial_days);
                }
            }
        }
    }

    #[tokio::test]
    async fn checkout_sdk_sends_cardless_trial_and_safe_end_policy() -> anyhow::Result<()> {
        use anyhow::Context;
        use axum::{http::StatusCode, routing::post, Router};
        use std::{collections::HashMap, time::Duration};
        use tokio::sync::mpsc;

        async fn capture(
            State(sender): State<mpsc::UnboundedSender<String>>,
            body: String,
        ) -> (StatusCode, Json<serde_json::Value>) {
            let _ = sender.send(body);
            // Only capture the actual SDK request. Never create a real
            // subscription or pretend the fixture is a successful checkout.
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": {"type": "invalid_request_error", "message": "local fixture"}
                })),
            )
        }

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let app = Router::new().route("/v1/checkout/sessions", post(capture)).with_state(sender);
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        struct ServerGuard(tokio::task::JoinHandle<std::io::Result<()>>);
        impl Drop for ServerGuard {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let _server = ServerGuard(server);
        let base = format!("http://{address}/");
        let client = Client::from_url(base.as_str(), "local-fixture-not-a-provider-key");
        let org_id = "04caad30-8aae-4506-a73c-3f306c0bbc0e";
        for trial_days in [14, 0, 30] {
            for (plan, interval, price) in [
                ("pro", BillingInterval::Monthly, "price_pro_month"),
                ("pro", BillingInterval::Annual, "price_pro_year"),
                ("team", BillingInterval::Monthly, "price_team_month"),
                ("team", BillingInterval::Annual, "price_team_year"),
            ] {
                let params = subscription_checkout_params(
                    org_id,
                    plan,
                    interval,
                    price.to_string(),
                    "https://app.example.test/billing/success",
                    "https://app.example.test/billing/cancel",
                    trial_days,
                );
                assert!(CheckoutSession::create(&client, params).await.is_err());
                let body = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
                    .await?
                    .context("Stripe SDK must send its request to the owned fixture")?;
                let form_url = reqwest::Url::parse(&format!("http://fixture.invalid/?{body}"))?;
                let fields: HashMap<_, _> = form_url.query_pairs().into_owned().collect();
                assert_eq!(fields.get("mode").map(String::as_str), Some("subscription"));
                assert_eq!(fields.get("client_reference_id").map(String::as_str), Some(org_id));
                assert_eq!(
                    fields.get("subscription_data[metadata][org_id]").map(String::as_str),
                    Some(org_id)
                );
                assert_eq!(fields.get("line_items[0][price]").map(String::as_str), Some(price));
                let card_policy = fields.get("payment_method_collection").map(String::as_str);
                let end_policy = fields
                    .get("subscription_data[trial_settings][end_behavior][missing_payment_method]")
                    .map(String::as_str);
                if trial_days > 0 {
                    assert_eq!(card_policy, Some("if_required"), "trials must allow no card");
                    assert_eq!(end_policy, Some("cancel"), "no-card trials must not create debt");
                    assert_eq!(
                        fields.get("subscription_data[trial_period_days]"),
                        Some(&trial_days.to_string())
                    );
                } else {
                    assert_eq!(card_policy, Some("always"), "paid checkout still collects a card");
                    assert_eq!(end_policy, None);
                    assert!(!fields.contains_key("subscription_data[trial_period_days]"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn billing_interval_parses_aliases_and_defaults_to_monthly() {
        assert_eq!(BillingInterval::parse(None), Some(BillingInterval::Monthly));
        assert_eq!(BillingInterval::parse(Some("month")), Some(BillingInterval::Monthly));
        assert_eq!(BillingInterval::parse(Some("monthly")), Some(BillingInterval::Monthly));
        assert_eq!(BillingInterval::parse(Some("year")), Some(BillingInterval::Annual));
        assert_eq!(BillingInterval::parse(Some("annual")), Some(BillingInterval::Annual));
        assert_eq!(BillingInterval::parse(Some("yearly")), Some(BillingInterval::Annual));
        // Unrecognised values are rejected (caller 400s) rather than guessed.
        assert_eq!(BillingInterval::parse(Some("biennial")), None);
        assert_eq!(BillingInterval::parse(Some("")), None);
    }

    #[test]
    fn select_price_id_routes_by_cadence() {
        let monthly = Some("price_monthly");
        let annual = Some("price_annual");
        assert_eq!(
            select_price_id(BillingInterval::Monthly, monthly, annual),
            Some("price_monthly")
        );
        assert_eq!(select_price_id(BillingInterval::Annual, monthly, annual), Some("price_annual"));
    }

    #[test]
    fn select_price_id_annual_unconfigured_returns_none_not_monthly() {
        // The critical safety property: requesting annual when it isn't
        // configured must NOT silently fall back to the monthly price.
        let monthly = Some("price_monthly");
        assert_eq!(select_price_id(BillingInterval::Annual, monthly, None), None);
        // ...and vice versa.
        assert_eq!(select_price_id(BillingInterval::Monthly, None, Some("price_annual")), None);
    }

    #[test]
    fn interval_label_recognises_annual_price_ids() {
        let pro_annual = Some("price_pro_year");
        let team_annual = Some("price_team_year");
        assert_eq!(interval_label_from_price_id("price_pro_year", pro_annual, team_annual), "year");
        assert_eq!(
            interval_label_from_price_id("price_team_year", pro_annual, team_annual),
            "year"
        );
        // Monthly / unknown / legacy IDs → monthly.
        assert_eq!(
            interval_label_from_price_id("price_pro_month", pro_annual, team_annual),
            "month"
        );
        assert_eq!(interval_label_from_price_id("price_legacy", None, None), "month");
    }
}

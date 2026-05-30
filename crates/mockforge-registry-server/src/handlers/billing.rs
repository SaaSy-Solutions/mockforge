//! Billing and subscription handlers

use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use stripe::{
    BillingPortalSession, CheckoutSession, CheckoutSessionMode, Client, CreateBillingPortalSession,
    CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionSubscriptionData,
    EventObject, EventType, Invoice, ListInvoices,
};
use uuid::Uuid;

use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        AuditEventType, Organization, Plan, Subscription, SubscriptionStatus, UsageCounter, User,
    },
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
    let pool = state.db.pool();

    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get subscription
    let subscription = Subscription::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    // Get current usage
    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    // Effective limits = plan defaults + custom org quota overrides
    let limits = super::usage::effective_limits(&state, &org_ctx.org).await?;

    Ok(Json(SubscriptionResponse {
        org_id: org_ctx.org_id,
        plan: org_ctx.org.plan().to_string(),
        status: subscription
            .as_ref()
            .map(|s| s.status().to_string())
            .unwrap_or_else(|| "free".to_string()),
        // Derived from the stored Stripe price ID (no dedicated column) — annual
        // subs are recognised by matching the configured annual price IDs.
        billing_interval: subscription
            .as_ref()
            .map(|s| {
                interval_label_from_price_id(
                    &s.price_id,
                    state.config.stripe_price_id_pro_annual.as_deref(),
                    state.config.stripe_price_id_team_annual.as_deref(),
                )
                .to_string()
            })
            .unwrap_or_else(|| BillingInterval::Monthly.as_str().to_string()),
        cancel_at_period_end: subscription
            .as_ref()
            .map(|s| s.cancel_at_period_end)
            .unwrap_or(false),
        current_period_start: subscription.as_ref().map(|s| s.current_period_start),
        current_period_end: subscription.as_ref().map(|s| s.current_period_end).or_else(|| {
            // For free plan, return None or far future
            Some(chrono::Utc::now() + chrono::Duration::days(365))
        }),
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

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub org_id: Uuid,
    pub plan: String,
    pub status: String,
    /// Billing cadence of the active subscription: "month" or "year".
    /// Defaults to "month" for free orgs / when no subscription exists.
    pub billing_interval: String,
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

    let mut checkout_params = CreateCheckoutSession::new();
    checkout_params.success_url = Some(&success_url);
    checkout_params.cancel_url = Some(&cancel_url);
    checkout_params.mode = Some(CheckoutSessionMode::Subscription);
    checkout_params.client_reference_id = Some(&org_id_str);

    // Add metadata for org_id (as backup)
    checkout_params.metadata = Some(std::collections::HashMap::from([
        ("org_id".to_string(), org_id_str.clone()),
        ("plan".to_string(), plan_str.clone()),
        ("billing_interval".to_string(), interval.as_str().to_string()),
    ]));

    // Add line item with price
    checkout_params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        price: Some(price_id.clone()),
        quantity: Some(1),
        ..Default::default()
    }]);

    // Apply a free trial when one is configured. Default is 14 days (set in
    // Config::from_env); STRIPE_TRIAL_PERIOD_DAYS=0 disables it entirely.
    // The marketing pricing page ("Start Pro/Team Trial") promises this;
    // without it the CTA copy is a bait-and-switch — checkout would charge
    // immediately. Standard B2B dev-tool convention at this price point.
    if state.config.stripe_trial_period_days > 0 {
        checkout_params.subscription_data = Some(CreateCheckoutSessionSubscriptionData {
            trial_period_days: Some(state.config.stripe_trial_period_days),
            ..Default::default()
        });
    }

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

/// Stripe webhook handler
/// This receives events from Stripe and updates subscriptions
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    // Get webhook secret
    let webhook_secret = state.config.stripe_webhook_secret.as_ref().ok_or_else(|| {
        ApiError::InvalidRequest("Stripe webhook secret not configured".to_string())
    })?;

    // Get signature from headers
    let signature = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::InvalidRequest("Missing stripe-signature header".to_string()))?;

    // Verify webhook signature
    let body_str = std::str::from_utf8(&body)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid UTF-8 in webhook body: {}", e)))?;
    let event =
        stripe::Webhook::construct_event(body_str, signature, webhook_secret).map_err(|e| {
            ApiError::InvalidRequest(format!("Webhook signature verification failed: {}", e))
        })?;

    let pool = state.db.pool();

    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            // Handle checkout completion
            if let EventObject::CheckoutSession(session) = event.data.object {
                handle_checkout_completed(pool, &session).await?;
            }
        }
        EventType::CustomerSubscriptionCreated | EventType::CustomerSubscriptionUpdated => {
            // Handle subscription creation/update
            if let EventObject::Subscription(subscription) = event.data.object {
                handle_subscription_event(pool, &subscription, &state.config).await?;
            }
        }
        EventType::CustomerSubscriptionDeleted => {
            // Handle subscription cancellation
            if let EventObject::Subscription(subscription) = event.data.object {
                handle_subscription_deleted(pool, &subscription).await?;
            }
        }
        EventType::InvoicePaymentSucceeded => {
            // Payment succeeded - subscription is active
            if let EventObject::Invoice(invoice) = event.data.object {
                handle_payment_succeeded(pool, &invoice).await?;
            }
        }
        EventType::InvoicePaymentFailed => {
            // Payment failed - mark subscription as past_due
            if let EventObject::Invoice(invoice) = event.data.object {
                handle_payment_failed(pool, &invoice).await?;
            }
        }
        _ => {
            tracing::debug!("Unhandled Stripe event: {:?}", event.type_);
        }
    }

    Ok(Json(serde_json::json!({ "received": true })))
}

/// Handle checkout session completed
async fn handle_checkout_completed(
    pool: &sqlx::PgPool,
    session: &CheckoutSession,
) -> Result<(), ApiError> {
    // Extract org_id from client_reference_id
    let org_id_str = session.client_reference_id.as_ref().ok_or_else(|| {
        ApiError::InvalidRequest("Missing client_reference_id in checkout session".to_string())
    })?;

    let org_id = Uuid::parse_str(org_id_str)
        .map_err(|_| ApiError::InvalidRequest("Invalid org_id in checkout session".to_string()))?;

    // Get subscription ID from session
    let subscription_id = session
        .subscription
        .as_ref()
        .and_then(|s| match s {
            stripe::Expandable::Id(id) => Some(id.clone()),
            stripe::Expandable::Object(_) => None, // Would need to expand
        })
        .ok_or_else(|| {
            ApiError::InvalidRequest("Missing subscription in checkout session".to_string())
        })?;

    tracing::info!("Checkout completed: org_id={}, subscription_id={}", org_id, subscription_id);

    // The subscription will be handled by the subscription.created/updated webhook
    // But we can update the org's stripe_customer_id here if needed
    if let Some(customer_id) = &session.customer {
        let customer_id_str = match customer_id {
            stripe::Expandable::Id(id) => id.to_string(),
            stripe::Expandable::Object(customer) => customer.id.to_string(),
        };

        Organization::update_stripe_customer_id(pool, org_id, Some(&customer_id_str))
            .await
            .map_err(ApiError::Database)?;
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
    subscription: &stripe::Subscription,
    config: &crate::config::Config,
) -> Result<(), ApiError> {
    let stripe_sub_id = subscription.id.to_string();

    let stripe_customer_id = match &subscription.customer {
        stripe::Expandable::Id(id) => id.to_string(),
        stripe::Expandable::Object(customer) => customer.id.to_string(),
    };

    // Get price_id from subscription items
    let price_id = subscription
        .items
        .data
        .first()
        .and_then(|item| item.price.as_ref())
        .map(|price| price.id.to_string())
        .ok_or_else(|| ApiError::InvalidRequest("Missing price in subscription".to_string()))?;

    // Determine plan from price_id (using config for exact matching)
    let plan = determine_plan_from_price_id(&price_id, config);

    let status = SubscriptionStatus::from_string(subscription.status.as_ref());

    let current_period_start =
        chrono::DateTime::<chrono::Utc>::from_timestamp(subscription.current_period_start, 0)
            .unwrap_or_else(chrono::Utc::now);

    let current_period_end =
        chrono::DateTime::<chrono::Utc>::from_timestamp(subscription.current_period_end, 0)
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30));

    let cancel_at_period_end = subscription.cancel_at_period_end;

    let canceled_at = subscription
        .canceled_at
        .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0));

    // Get org_id from metadata or find by customer_id
    let org_id = subscription
        .metadata
        .get("org_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .or({
            // Fallback: find org by stripe_customer_id
            // This requires async, so we'll need to handle it differently
            // For now, require metadata
            None
        })
        .ok_or_else(|| {
            ApiError::InvalidRequest("Missing org_id in subscription metadata".to_string())
        })?;

    // Upsert subscription
    let _subscription_record = Subscription::upsert_from_stripe(
        pool,
        org_id,
        &stripe_sub_id,
        &stripe_customer_id,
        &price_id,
        plan,
        status,
        current_period_start,
        current_period_end,
        cancel_at_period_end,
        canceled_at,
    )
    .await
    .map_err(ApiError::Database)?;

    // Get old plan before updating
    let old_org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let old_plan = old_org.plan();

    // Update organization plan
    Organization::update_plan(pool, org_id, plan)
        .await
        .map_err(ApiError::Database)?;

    // Update stripe_customer_id on org
    Organization::update_stripe_customer_id(pool, org_id, Some(&stripe_customer_id))
        .await
        .map_err(ApiError::Database)?;

    tracing::info!("Subscription updated: org_id={}, plan={:?}", org_id, plan);

    // Record audit log (webhook event, so no user_id or IP)
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
                let amount = subscription
                    .items
                    .data
                    .first()
                    .and_then(|item| item.price.as_ref())
                    .and_then(|price| price.unit_amount)
                    .map(|amount| amount as f64 / 100.0); // Convert cents to dollars

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
    subscription: &stripe::Subscription,
) -> Result<(), ApiError> {
    let stripe_sub_id = subscription.id.to_string();

    // Find subscription
    let subscription_record = Subscription::find_by_stripe_subscription_id(pool, &stripe_sub_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            ApiError::InvalidRequest("Subscription not found in database".to_string())
        })?;

    // Update status to canceled
    Subscription::update_status(pool, subscription_record.id, SubscriptionStatus::Canceled)
        .await
        .map_err(ApiError::Database)?;

    // Get the organization before downgrading to log the old plan
    let org = Organization::find_by_id(pool, subscription_record.org_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let old_plan = org.plan();

    // Downgrade org to free plan
    Organization::update_plan(pool, subscription_record.org_id, Plan::Free)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!("Subscription canceled: org_id={}", subscription_record.org_id);

    // Record audit log (webhook event, so no user_id or IP)
    crate::models::record_audit_event(
        pool,
        subscription_record.org_id,
        None, // Webhook event, no user context
        AuditEventType::BillingCanceled,
        format!("Subscription canceled for plan: {:?}", old_plan),
        Some(serde_json::json!({
            "subscription_id": subscription.id.to_string(),
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

/// Handle payment succeeded
async fn handle_payment_succeeded(pool: &sqlx::PgPool, invoice: &Invoice) -> Result<(), ApiError> {
    let customer_id = match &invoice.customer {
        Some(stripe::Expandable::Id(id)) => id.to_string(),
        Some(stripe::Expandable::Object(customer)) => customer.id.to_string(),
        None => return Err(ApiError::InvalidRequest("Invoice missing customer".to_string())),
    };

    // Find subscription by customer_id
    let subscription = sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE stripe_customer_id = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&customer_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Subscription not found".to_string()))?;

    // Update status to active if it was past_due
    if subscription.status() == SubscriptionStatus::PastDue {
        Subscription::update_status(pool, subscription.id, SubscriptionStatus::Active)
            .await
            .map_err(ApiError::Database)?;
    }

    tracing::info!("Payment succeeded: org_id={}", subscription.org_id);

    Ok(())
}

async fn handle_payment_failed(pool: &sqlx::PgPool, invoice: &Invoice) -> Result<(), ApiError> {
    let customer_id = match &invoice.customer {
        Some(stripe::Expandable::Id(id)) => id.to_string(),
        Some(stripe::Expandable::Object(customer)) => customer.id.to_string(),
        None => return Err(ApiError::InvalidRequest("Invoice missing customer".to_string())),
    };

    // Find subscription by customer_id
    let subscription = sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE stripe_customer_id = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&customer_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Subscription not found".to_string()))?;

    // Update status to past_due
    Subscription::update_status(pool, subscription.id, SubscriptionStatus::PastDue)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!("Payment failed: org_id={}", subscription.org_id);

    // Record audit log (webhook event, so no user_id or IP)
    crate::models::record_audit_event(
        pool,
        subscription.org_id,
        None,                            // Webhook event, no user context
        AuditEventType::BillingCanceled, // Using canceled as closest match
        format!("Payment failed for subscription: {}", invoice.id),
        Some(serde_json::json!({
            "invoice_id": invoice.id.to_string(),
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
                let org = Organization::find_by_id(pool, subscription.org_id)
                    .await
                    .map_err(ApiError::Database)?
                    .ok_or_else(|| {
                        ApiError::InvalidRequest("Organization not found".to_string())
                    })?;

                let amount = invoice.amount_due.map(|a| a as f64 / 100.0).unwrap_or(0.0);
                let retry_date = invoice
                    .next_payment_attempt
                    .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0));

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

/// Determine plan from Stripe price_id
/// Maps Stripe price IDs to internal plans using exact matching from config
fn determine_plan_from_price_id(price_id: &str, config: &crate::config::Config) -> Plan {
    // Exact match against configured price IDs
    if let Some(pro_price_id) = &config.stripe_price_id_pro {
        if price_id == pro_price_id {
            return Plan::Pro;
        }
    }

    if let Some(team_price_id) = &config.stripe_price_id_team {
        if price_id == team_price_id {
            return Plan::Team;
        }
    }

    // Annual price IDs map to the same plan as their monthly counterpart.
    if let Some(pro_annual) = &config.stripe_price_id_pro_annual {
        if price_id == pro_annual {
            return Plan::Pro;
        }
    }
    if let Some(team_annual) = &config.stripe_price_id_team_annual {
        if price_id == team_annual {
            return Plan::Team;
        }
    }

    // Fallback: heuristic matching (for development/testing)
    if price_id.contains("pro") || price_id.contains("Pro") {
        tracing::warn!("Using heuristic matching for price_id: {} (Pro)", price_id);
        return Plan::Pro;
    }

    if price_id.contains("team") || price_id.contains("Team") {
        tracing::warn!("Using heuristic matching for price_id: {} (Team)", price_id);
        return Plan::Team;
    }

    // Default to Free if unknown
    tracing::warn!("Unknown price_id: {}, defaulting to Free", price_id);
    Plan::Free
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

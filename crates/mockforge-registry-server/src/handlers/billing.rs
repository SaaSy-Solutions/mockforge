//! Billing and subscription handlers

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, Event, EventObject, EventType,
};
use uuid::Uuid;

use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser, OrgContext},
    models::{
        record_audit_event, AuditEventType, Organization, Plan, Subscription, SubscriptionStatus,
        UsageCounter, User,
    },
    AppState,
};

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
        .map_err(|e| ApiError::Database(e))?;

    // Get current usage
    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get plan limits
    let limits = org_ctx.org.limits_json.clone();

    Ok(Json(SubscriptionResponse {
        org_id: org_ctx.org_id,
        plan: org_ctx.org.plan().to_string(),
        status: subscription
            .as_ref()
            .map(|s| s.status().to_string())
            .unwrap_or_else(|| "free".to_string()),
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
    pub ai_tokens_used: i64,
    pub ai_tokens_limit: i64,
}

/// Create Stripe checkout session
/// This would typically redirect to Stripe Checkout
#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String, // "pro" or "team"
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

    // Get price ID for the plan
    let price_id = match plan {
        Plan::Pro => state.config.stripe_price_id_pro.as_ref().ok_or_else(|| {
            ApiError::InvalidRequest("Stripe Pro price ID not configured".to_string())
        })?,
        Plan::Team => state.config.stripe_price_id_team.as_ref().ok_or_else(|| {
            ApiError::InvalidRequest("Stripe Team price ID not configured".to_string())
        })?,
        Plan::Free => {
            return Err(ApiError::InvalidRequest(
                "Cannot create checkout for free plan".to_string(),
            ))
        }
    };

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
    ]));

    // Add line item with price
    checkout_params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        price: Some(price_id.clone()),
        quantity: Some(1),
        ..Default::default()
    }]);

    // Create the checkout session
    let session = CheckoutSession::create(&client, checkout_params)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Stripe error: {}", e)))?;

    // Record audit log
    let pool = state.db.pool();
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::BillingCheckout,
        format!("Checkout session created for {} plan", request.plan),
        Some(serde_json::json!({
            "plan": request.plan,
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
            .map_err(|e| ApiError::Database(e))?;
    }

    Ok(())
}

/// Get organization owner's email for sending notifications
async fn get_org_owner_email(
    pool: &sqlx::PgPool,
    org_id: Uuid,
) -> Result<(String, String), ApiError> {
    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get owner user
    let owner = User::find_by_id(pool, org.owner_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization owner not found".to_string()))?;

    Ok((owner.email, owner.username))
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

    let status = SubscriptionStatus::from_string(&subscription.status.to_string());

    let current_period_start =
        chrono::DateTime::<chrono::Utc>::from_timestamp(subscription.current_period_start, 0)
            .unwrap_or_else(chrono::Utc::now);

    let current_period_end =
        chrono::DateTime::<chrono::Utc>::from_timestamp(subscription.current_period_end, 0)
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30));

    let cancel_at_period_end = subscription.cancel_at_period_end;

    let canceled_at = subscription
        .canceled_at
        .map(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0))
        .flatten();

    // Get org_id from metadata or find by customer_id
    let org_id = subscription
        .metadata
        .get("org_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            // Fallback: find org by stripe_customer_id
            // This requires async, so we'll need to handle it differently
            // For now, require metadata
            None
        })
        .ok_or_else(|| {
            ApiError::InvalidRequest("Missing org_id in subscription metadata".to_string())
        })?;

    // Upsert subscription
    let subscription_record = Subscription::upsert_from_stripe(
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
    .map_err(|e| ApiError::Database(e))?;

    // Get old plan before updating
    let old_org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let old_plan = old_org.plan();

    // Update organization plan
    Organization::update_plan(pool, org_id, plan)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Update stripe_customer_id on org
    Organization::update_stripe_customer_id(pool, org_id, Some(&stripe_customer_id))
        .await
        .map_err(|e| ApiError::Database(e))?;

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

        record_audit_event(
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

    // Send subscription confirmation email (non-blocking)
    if let Ok((email_addr, username)) = get_org_owner_email(pool, org_id).await {
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
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::InvalidRequest("Subscription not found in database".to_string())
        })?;

    // Update status to canceled
    Subscription::update_status(pool, subscription_record.id, SubscriptionStatus::Canceled)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get the organization before downgrading to log the old plan
    let org = Organization::find_by_id(pool, subscription_record.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let old_plan = org.plan();

    // Downgrade org to free plan
    Organization::update_plan(pool, subscription_record.org_id, Plan::Free)
        .await
        .map_err(|e| ApiError::Database(e))?;

    tracing::info!("Subscription canceled: org_id={}", subscription_record.org_id);

    // Record audit log (webhook event, so no user_id or IP)
    record_audit_event(
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

    // Send subscription canceled email (non-blocking)
    if let Ok((email_addr, username)) = get_org_owner_email(pool, subscription_record.org_id).await
    {
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

    Ok(())
}

/// Handle payment succeeded
async fn handle_payment_succeeded(
    pool: &sqlx::PgPool,
    invoice: &stripe::Invoice,
) -> Result<(), ApiError> {
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
    .map_err(|e| ApiError::Database(e))?
    .ok_or_else(|| ApiError::InvalidRequest("Subscription not found".to_string()))?;

    // Update status to active if it was past_due
    if subscription.status() == SubscriptionStatus::PastDue {
        Subscription::update_status(pool, subscription.id, SubscriptionStatus::Active)
            .await
            .map_err(|e| ApiError::Database(e))?;
    }

    tracing::info!("Payment succeeded: org_id={}", subscription.org_id);

    Ok(())
}

async fn handle_payment_failed(
    pool: &sqlx::PgPool,
    invoice: &stripe::Invoice,
) -> Result<(), ApiError> {
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
    .map_err(|e| ApiError::Database(e))?
    .ok_or_else(|| ApiError::InvalidRequest("Subscription not found".to_string()))?;

    // Update status to past_due
    Subscription::update_status(pool, subscription.id, SubscriptionStatus::PastDue)
        .await
        .map_err(|e| ApiError::Database(e))?;

    tracing::info!("Payment failed: org_id={}", subscription.org_id);

    // Record audit log (webhook event, so no user_id or IP)
    record_audit_event(
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

    // Send payment failed email (non-blocking)
    if let Ok((email_addr, username)) = get_org_owner_email(pool, subscription.org_id).await {
        if let Ok(email_service) = EmailService::from_env() {
            let org = Organization::find_by_id(pool, subscription.org_id)
                .await
                .map_err(|e| ApiError::Database(e))?
                .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

            let amount = invoice.amount_due.map(|a| a as f64 / 100.0).unwrap_or(0.0); // Convert cents to dollars
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

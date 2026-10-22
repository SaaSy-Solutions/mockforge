//! Durable platform-credit admission and exact-period settlement.
use crate::{
    ai::{call_llm, LlmCall, LlmResult, ProviderSelection},
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    models::{Organization, Plan, UsageCounter},
    store::{with_org_context, with_org_transaction},
    AppState,
};
use chrono::Datelike;
use uuid::Uuid;

#[derive(Clone, Copy)]
struct Reservation {
    id: Uuid,
    org_id: Uuid,
    counter_id: Uuid,
    free_credit_period_id: Option<Uuid>,
}

/// Provenance for a platform-funded call. Worker actors are read from the
/// persisted running job, never supplied independently by the caller.
#[derive(Clone, Copy)]
pub enum CallOrigin {
    /// Compatibility for callers predating durable attribution.
    Legacy,
    /// Studio or internal scoring with its already resolved organization actor.
    OrganizationCompletion {
        actor_user_id: Option<Uuid>,
    },
    TestGeneration {
        job_id: Uuid,
    },
}

enum Admission {
    Denied,
    Exhausted,
    Ready(Reservation, i64),
}

async fn reserve(
    state: &AppState,
    org: &Organization,
    call: &mut LlmCall,
    origin: CallOrigin,
) -> ApiResult<Reservation> {
    // UTF-8 bytes conservatively bound text tokens, with framing allowance.
    let input = call.system.len().saturating_add(call.user.len()).saturating_add(512);
    if input > 32 * 1024 || call.max_tokens == 0 || call.max_tokens > 8192 {
        return Err(ApiError::InvalidRequest(
            "AI input or output budget exceeds the supported limit".into(),
        ));
    }
    let limit = effective_limits(state, org)
        .await?
        .get("ai_tokens_per_month")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let free = crate::handlers::entitlements::effective_plan(state, org).await? == Plan::Free;
    let org_id = org.id;
    let today = chrono::Utc::now().date_naive();
    let period = today.with_day(1).unwrap_or(today);
    let requested_output = i64::from(call.max_tokens);
    let input = input as i64;
    let admitted = with_org_context(state.db.runtime_pool(), org_id, |tx| Box::pin(async move {
        // The upsert takes the counter row lock. All admissions and settlements
        // use this same lock, so concurrent callers cannot reuse available credit.
        let (call_origin, actor_user_id, job_id) = match origin {
            CallOrigin::Legacy => ("legacy", None, None),
            CallOrigin::OrganizationCompletion { actor_user_id } =>
                ("organization_completion", actor_user_id, None),
            CallOrigin::TestGeneration { job_id } => {
                let actor: Option<Option<Uuid>> = sqlx::query_scalar(
                    "SELECT j.created_by FROM cloud_test_generation_jobs j JOIN workspaces w ON w.id=j.workspace_id AND w.org_id=j.org_id WHERE j.id=$1 AND j.org_id=$2 AND j.status='running'"
                ).bind(job_id).bind(org_id).fetch_optional(&mut **tx).await?;
                let Some(actor) = actor else { return Ok(Admission::Denied); };
                ("test_generation", actor, Some(job_id))
            }
        };
        if let Some(actor) = actor_user_id {
            let member: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM org_members WHERE org_id=$1 AND user_id=$2)"
            ).bind(org_id).bind(actor).fetch_one(&mut **tx).await?;
            if !member { return Ok(Admission::Denied); }
        }
        // A shared account row is locked before the organization counter. Every
        // Free admission/settlement/recovery uses this ordering across tenants.
        let shared: Option<(Uuid, i64, i64, i64)> = if free {
            let owner: Option<Uuid> = sqlx::query_scalar("SELECT o.owner_id FROM organizations o JOIN users u ON u.id=o.owner_id WHERE o.id=$1 AND u.is_verified")
                .bind(org_id).fetch_optional(&mut **tx).await?;
            let Some(owner) = owner else { return Ok(Admission::Denied); };
            Some(sqlx::query_as("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens) VALUES($1,$2,$3) ON CONFLICT(owner_user_id,period_start) DO UPDATE SET granted_tokens=CASE WHEN free_ai_credit_periods.granted_tokens=-1 OR EXCLUDED.granted_tokens=-1 THEN -1 ELSE GREATEST(free_ai_credit_periods.granted_tokens,EXCLUDED.granted_tokens) END RETURNING id,granted_tokens,used_tokens,reserved_tokens")
                .bind(owner).bind(period).bind(if limit < 0 { -1 } else { limit }).fetch_one(&mut **tx).await?)
        } else { None };
        let counter = UsageCounter::get_or_create_for_period(&mut **tx, org_id, period).await?;
        let held: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(budget),0)::bigint FROM ai_token_reservations WHERE usage_counter_id=$1 AND org_id=$2 AND state <> 'completed'")
            .bind(counter.id).bind(org_id).fetch_one(&mut **tx).await?;
        let mut output = if limit < 0 { requested_output } else {
            requested_output.min(limit.saturating_sub(counter.ai_tokens_used).saturating_sub(held).saturating_sub(input))
        };
        if let Some((_, granted, used, reserved)) = shared {
            if granted >= 0 { output = output.min(granted.saturating_sub(used).saturating_sub(reserved).saturating_sub(input)); }
        }
        if output <= 0 { return Ok(Admission::Exhausted); }
        let reservation = Reservation { id: Uuid::new_v4(), org_id, counter_id: counter.id, free_credit_period_id: shared.map(|row| row.0) };
        if let Some(id) = reservation.free_credit_period_id {
            sqlx::query("UPDATE free_ai_credit_periods SET reserved_tokens=reserved_tokens+$2 WHERE id=$1")
                .bind(id).bind(input+output).execute(&mut **tx).await?;
        }
        sqlx::query("INSERT INTO ai_token_reservations(id,org_id,usage_counter_id,budget,call_origin,actor_user_id,test_generation_job_id,free_credit_period_id,execution_fenced) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,TRUE)")
            .bind(reservation.id).bind(org_id).bind(counter.id).bind(input + output)
            .bind(call_origin).bind(actor_user_id).bind(job_id).bind(reservation.free_credit_period_id).execute(&mut **tx).await?;
        Ok(Admission::Ready(reservation, output))
    })).await?;
    let (reservation, output) =
        match admitted {
            Admission::Denied => return Err(ApiError::PermissionDenied),
            Admission::Exhausted => return Err(ApiError::ResourceLimitExceeded(
                "AI credits are exhausted or reserved by outstanding calls; add BYOK or upgrade"
                    .into(),
            )),
            Admission::Ready(reservation, output) => (reservation, output),
        };
    call.max_tokens = output as u32;
    Ok(reservation)
}

async fn settle(state: &AppState, reservation: Reservation, actual: Option<u64>) -> ApiResult<()> {
    let actual = actual
        .map(i64::try_from)
        .transpose()
        .map_err(|_| ApiError::InvalidRequest("Invalid provider token usage".into()))?;
    with_org_context(state.db.runtime_pool(), reservation.org_id, |tx| Box::pin(async move {
        if let Some(id) = reservation.free_credit_period_id {
            sqlx::query("SELECT id FROM free_ai_credit_periods WHERE id=$1 FOR UPDATE")
                .bind(id).fetch_one(&mut **tx).await?;
        }
        sqlx::query("SELECT id FROM usage_counters WHERE id=$1 AND org_id=$2 FOR UPDATE")
            .bind(reservation.counter_id).bind(reservation.org_id).fetch_one(&mut **tx).await?;
        let changed = sqlx::query("UPDATE ai_token_reservations SET state=CASE WHEN $3::bigint IS NULL THEN 'uncertain' ELSE 'completed' END, actual_tokens=$3, updated_at=NOW() WHERE id=$1 AND org_id=$2 AND state <> 'completed'")
            .bind(reservation.id).bind(reservation.org_id).bind(actual).execute(&mut **tx).await?.rows_affected();
        if changed > 0 {
            if let Some(tokens) = actual {
                if let Some(id) = reservation.free_credit_period_id {
                    sqlx::query("UPDATE free_ai_credit_periods SET used_tokens=used_tokens+$2,reserved_tokens=reserved_tokens-(SELECT budget FROM ai_token_reservations WHERE id=$3 AND org_id=$4) WHERE id=$1")
                        .bind(id).bind(tokens).bind(reservation.id).bind(reservation.org_id).execute(&mut **tx).await?;
                }
                // Charge the admitted period, including calls finishing after rollover.
                sqlx::query("UPDATE usage_counters SET ai_tokens_used=ai_tokens_used+$3, updated_at=NOW() WHERE id=$1 AND org_id=$2")
                    .bind(reservation.counter_id).bind(reservation.org_id).bind(tokens).execute(&mut **tx).await?;
            }
        }
        Ok(())
    })).await?;
    Ok(())
}

/// Shared by Studio and background workers. Cancellation of the caller must not
/// cancel settlement after the provider may have consumed platform credits.
pub async fn call_with_budget(
    state: &AppState,
    org: &Organization,
    selection: ProviderSelection,
    call: LlmCall,
) -> ApiResult<LlmResult> {
    call_with_budget_attributed(state, org, selection, call, CallOrigin::Legacy).await
}

/// Record validated provenance before a platform provider can consume credits.
pub async fn call_with_budget_attributed(
    state: &AppState,
    org: &Organization,
    selection: ProviderSelection,
    mut call: LlmCall,
    origin: CallOrigin,
) -> ApiResult<LlmResult> {
    match selection {
        ProviderSelection::Disabled => return Err(ApiError::PermissionDenied),
        ProviderSelection::Byok => return call_llm(call).await,
        ProviderSelection::Platform => {}
    }
    crate::handlers::entitlements::require_verified_free_owner(state, org).await?;
    let reservation = reserve(state, org, &mut call, origin).await?;
    let state = state.clone();
    tokio::spawn(async move {
        // Keep a transaction-scoped execution fence for the provider task.
        // A recovery racing task startup wins before the provider is contacted;
        // the state check below prevents that task from executing afterward.
        // The fence transaction only reads, so it ends (and releases its
        // connection) before settlement acquires shared accounting locks,
        // avoiding pool starvation and reversed lock order.
        let (reservation_id, org_id) = (reservation.id, reservation.org_id);
        let result = with_org_transaction(state.db.ai_execution_pool(), org_id, |tx| Box::pin(async move {
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,107))")
                .bind(reservation_id.to_string()).execute(&mut **tx).await?;
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ai_token_reservations WHERE id=$1 AND org_id=$2 AND state='pending' AND execution_fenced)")
                .bind(reservation_id).bind(org_id).fetch_one(&mut **tx).await?;
            if !pending {
                return Err(ApiError::InvalidRequest("AI call was recovered before execution".into()));
            }
            Ok(call_llm(call).await)
        })).await?;
        let actual = result.as_ref().ok().map(LlmResult::total_tokens);
        settle(&state, reservation, actual).await?;
        result
    })
    .await
    .map_err(|_| {
        ApiError::Internal(anyhow::anyhow!("AI call interrupted; credits remain reserved"))
    })?
}

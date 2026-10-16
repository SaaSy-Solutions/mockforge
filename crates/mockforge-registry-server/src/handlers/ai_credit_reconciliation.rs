//! Operator recovery for provider calls with unknown usage. Never releases a
//! reservation merely because it is old, and never settles a running call.
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{AuditEventType, AuditLog},
    store::with_org_transaction,
    AppState,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileRequest {
    /// Total tokens established from authoritative provider records; zero is
    /// valid only when those records confirm no provider charge.
    pub actual_tokens: u64,
    /// An opaque internal evidence/ticket identifier, never credentials or a URL.
    pub evidence_reference: String,
}

#[derive(Serialize)]
pub struct ReconcileResponse {
    pub reservation_id: Uuid,
    pub actual_tokens: i64,
    pub already_reconciled: bool,
}

pub async fn reconcile(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path((org_id, reservation_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ReconcileRequest>,
) -> ApiResult<Json<ReconcileResponse>> {
    let actual = i64::try_from(request.actual_tokens)
        .map_err(|_| ApiError::InvalidRequest("Invalid provider token total".into()))?;
    let reference = request.evidence_reference;
    if reference.is_empty()
        || reference.len() > 128
        || !reference.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(ApiError::InvalidRequest("Provide an opaque evidence reference of 1–128 letters, digits, dots, underscores or hyphens".into()));
    }
    let response = with_org_transaction(state.db.runtime_pool(), org_id, |tx| Box::pin(async move {
        // Hold the administrator row against concurrent revocation until commit.
        let admin: Option<bool> = sqlx::query_scalar("SELECT is_admin FROM users WHERE id=$1 FOR SHARE")
            .bind(actor).fetch_optional(&mut **tx).await?;
        if admin != Some(true) { return Err(ApiError::PermissionDenied); }
        // Completed accounting is immutable. An exact replay needs no access
        // to the old owner's shared pool after a settled ownership transfer.
        let completed: Option<(i64, Option<String>)> = sqlx::query_as("SELECT actual_tokens,reconciliation_reference FROM ai_token_reservations WHERE id=$1 AND org_id=$2 AND state='completed'")
            .bind(reservation_id).bind(org_id).fetch_optional(&mut **tx).await?;
        if let Some((tokens, evidence)) = completed {
            if tokens == actual && evidence.as_deref() == Some(reference.as_str()) {
                return Ok(ReconcileResponse { reservation_id, actual_tokens: actual, already_reconciled: true });
            }
            return Err(ApiError::InvalidRequest("Completed calls cannot be overwritten".into()));
        }
        let fenced: Option<bool> = sqlx::query_scalar("SELECT execution_fenced FROM ai_token_reservations WHERE id=$1 AND org_id=$2")
            .bind(reservation_id).bind(org_id).fetch_optional(&mut **tx).await?;
        if fenced == Some(true) {
            let stopped: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1,107))")
                .bind(reservation_id.to_string()).fetch_one(&mut **tx).await?;
            if !stopped {
                return Err(ApiError::InvalidRequest("The provider task is still executing; recovery requires final provider usage evidence after execution stops".into()));
            }
        }
        let counter: Option<(Uuid, Option<Uuid>, i64)> = sqlx::query_as("SELECT usage_counter_id,free_credit_period_id,budget FROM ai_token_reservations WHERE id=$1 AND org_id=$2")
            .bind(reservation_id).bind(org_id).fetch_optional(&mut **tx).await?;
        let (counter, shared, budget) = counter.ok_or_else(|| ApiError::InvalidRequest("Reservation not found".into()))?;
        // Shared account, organization counter, then reservation: the same
        // lock ordering as admission and automatic settlement.
        if let Some(id) = shared {
            sqlx::query("SELECT id FROM free_ai_credit_periods WHERE id=$1 FOR UPDATE")
                .bind(id).fetch_one(&mut **tx).await?;
        }
        sqlx::query("SELECT id FROM usage_counters WHERE id=$1 AND org_id=$2 FOR UPDATE")
            .bind(counter).bind(org_id).fetch_one(&mut **tx).await?;
        let (status, tokens, evidence): (String, Option<i64>, Option<String>) = sqlx::query_as("SELECT state,actual_tokens,reconciliation_reference FROM ai_token_reservations WHERE id=$1 AND org_id=$2 FOR UPDATE")
            .bind(reservation_id).bind(org_id).fetch_one(&mut **tx).await?;
        if status == "completed" && tokens == Some(actual) && evidence.as_deref() == Some(reference.as_str()) {
            return Ok(ReconcileResponse { reservation_id, actual_tokens: actual, already_reconciled: true });
        }
        if status != "uncertain" && !(status == "pending" && fenced == Some(true)) {
            return Err(ApiError::InvalidRequest("Only uncertain or stopped execution-fenced pending calls may be reconciled; legacy pending and completed calls cannot be overwritten".into()));
        }
        sqlx::query("UPDATE ai_token_reservations SET state='completed',actual_tokens=$3,reconciled_by=$4,reconciliation_reference=$5,reconciled_at=NOW(),updated_at=NOW() WHERE id=$1 AND org_id=$2")
            .bind(reservation_id).bind(org_id).bind(actual).bind(actor).bind(&reference).execute(&mut **tx).await?;
        if let Some(id) = shared {
            sqlx::query("UPDATE free_ai_credit_periods SET used_tokens=used_tokens+$2,reserved_tokens=reserved_tokens-$3 WHERE id=$1")
                .bind(id).bind(actual).bind(budget).execute(&mut **tx).await?;
        }
        sqlx::query("UPDATE usage_counters SET ai_tokens_used=ai_tokens_used+$3,updated_at=NOW() WHERE id=$1 AND org_id=$2")
            .bind(counter).bind(org_id).bind(actual).execute(&mut **tx).await?;
        // Audit failure rolls accounting back; a recovery must leave evidence.
        AuditLog::create(&mut *tx, org_id, Some(actor), AuditEventType::AdminImpersonation,
            "Reconciled uncertain platform AI usage".into(),
            Some(serde_json::json!({"action":"ai_credit_reconciliation", "reservation_id":reservation_id,"actual_tokens":actual,"evidence_reference":reference})), None, None).await?;
        Ok::<_, ApiError>(ReconcileResponse { reservation_id, actual_tokens: actual, already_reconciled: false })
    })).await?;
    Ok(Json(response))
}

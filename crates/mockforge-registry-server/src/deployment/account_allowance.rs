//! Atomic Free HTTP admission against both account and organization budgets.
use crate::{error::StoreResult, models::UsageCounter};
use chrono::{Datelike, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn admit_request(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    limit: i64,
) -> StoreResult<(bool, i64)> {
    let today = Utc::now().date_naive();
    let period = today.with_day(1).unwrap_or(today);
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM organizations WHERE id=$1")
        .bind(org_id)
        .fetch_one(&mut **tx)
        .await?;
    // Shared account before org counter: the same ordering across all proxy routes.
    let (id,granted,used):(Uuid,i64,i64)=sqlx::query_as("INSERT INTO free_hosted_request_periods(owner_user_id,period_start,granted_requests) VALUES($1,$2,$3) ON CONFLICT(owner_user_id,period_start) DO UPDATE SET granted_requests=CASE WHEN free_hosted_request_periods.granted_requests=-1 OR EXCLUDED.granted_requests=-1 THEN -1 ELSE GREATEST(free_hosted_request_periods.granted_requests,EXCLUDED.granted_requests) END RETURNING id,granted_requests,requests")
    .bind(owner).bind(period).bind(limit).fetch_one(&mut **tx).await?;
    let counter = UsageCounter::get_or_create_for_period(&mut **tx, org_id, period).await?;
    if limit >= 0 && counter.requests >= limit {
        return Ok((false, counter.requests));
    }
    if granted >= 0 && used >= granted {
        // Report this org's exhausted effective allowance, without exposing sibling
        // organization IDs or their individual traffic counts to a public caller.
        return Ok((false, if limit >= 0 { limit } else { used }));
    }
    sqlx::query("UPDATE free_hosted_request_periods SET requests=requests+1 WHERE id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE usage_counters SET requests=requests+1,updated_at=NOW() WHERE id=$1 AND org_id=$2",
    )
    .bind(counter.id)
    .bind(org_id)
    .execute(&mut **tx)
    .await?;
    Ok((true, counter.requests + 1))
}

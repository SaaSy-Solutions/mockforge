//! Internal endpoints for the Cloudflare hosted-mocks Worker.
//!
//! - `GET  /api/v1/internal/hosted-mocks/runtime/{org_id}/{slug}`: the
//!   environment and limits a mock's container starts with.
//! - `POST /api/v1/internal/hosted-mocks/{deployment_id}/usage`: request and
//!   egress counts the Worker has served since its last report.
//!
//! Auth: the shared `MOCKFORGE_INTERNAL_API_TOKEN` bearer token. The Worker is
//! a separate trust principal with no user, so both run on the owner pool.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    deployment::{cloudflare, machine_config::hosted_mock_runtime_env, router},
    error::{ApiError, ApiResult},
    handlers::internal_test_runs::require_internal_auth,
    models::{DeploymentStatus, HostedMock},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct RuntimeLimits {
    pub max_body_bytes: usize,
    pub rps: i64,
}

#[derive(Debug, Serialize)]
pub struct RuntimeResponse {
    pub deployment_id: Uuid,
    pub env: HashMap<String, String>,
    pub limits: RuntimeLimits,
    /// The org has reached its opt-in overage ceiling; serve 429.
    pub blocked: bool,
}

/// `GET /api/v1/internal/hosted-mocks/runtime/{org_id}/{slug}`
///
/// 404 unless the mock exists, is active and is served from Cloudflare, so a
/// deleted or moved mock stops starting at the edge.
pub async fn get_runtime(
    State(state): State<AppState>,
    Path((org_id, slug)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<RuntimeResponse>> {
    require_internal_auth(&headers)?;
    let pool = state.db.pool();

    let deployment = HostedMock::find_by_slug(pool, org_id, &slug)
        .await
        .map_err(ApiError::Database)?
        .filter(|d| matches!(d.status(), DeploymentStatus::Active) && cloudflare::is_cloudflare(d))
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;

    cloudflare::require_paid_plan(pool, org_id).await?;
    let env = hosted_mock_runtime_env(&deployment, None).map_err(ApiError::Internal)?;
    let limits = router::resolve_proxy_limits(pool, org_id).await;
    let blocked = router::quota_ceiling_reached(pool, org_id).await.is_some();

    Ok(Json(RuntimeResponse {
        deployment_id: deployment.id,
        env,
        limits: RuntimeLimits {
            max_body_bytes: limits.max_body_bytes,
            rps: limits.rps,
        },
        blocked,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UsageReport {
    pub batch_id: Uuid,
    pub period_start: NaiveDate,
    pub requests: i64,
    #[serde(default)]
    pub egress_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub blocked: bool,
}

/// `POST /api/v1/internal/hosted-mocks/{deployment_id}/usage`
///
/// Adds the reported counts to the deployment's org and returns whether the
/// org is now over its overage ceiling. The org comes from the deployment
/// row, never from the request.
pub async fn report_usage(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(report): Json<UsageReport>,
) -> ApiResult<Json<UsageResponse>> {
    require_internal_auth(&headers)?;
    if report.requests < 0 || report.egress_bytes < 0 {
        return Err(ApiError::InvalidRequest("counts must be non-negative".into()));
    }
    if report.period_start.day() != 1 || report.period_start > Utc::now().date_naive() {
        return Err(ApiError::InvalidRequest(
            "period_start must be a current or past month start".into(),
        ));
    }
    let pool = state.db.pool();

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;
    let org_id = deployment.org_id;

    apply_usage_batch(pool, deployment_id, org_id, &report).await?;

    let blocked = cloudflare::require_paid_plan(pool, org_id).await.is_err()
        || router::quota_ceiling_reached(pool, org_id).await.is_some();
    Ok(Json(UsageResponse { blocked }))
}

/// Only the authenticated internal callback calls this with an org resolved
/// from its deployment. The receipt and both increments share one transaction.
async fn apply_usage_batch(
    pool: &sqlx::PgPool,
    deployment_id: Uuid,
    org_id: Uuid,
    report: &UsageReport,
) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO hosted_mock_usage_batches (deployment_id, batch_id, org_id, period_start, requests, egress_bytes)
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (deployment_id, batch_id) DO NOTHING",
    )
    .bind(deployment_id).bind(report.batch_id).bind(org_id).bind(report.period_start)
    .bind(report.requests).bind(report.egress_bytes).execute(&mut *tx).await?.rows_affected();
    if inserted == 0 {
        let previous: (Uuid, NaiveDate, i64, i64) = sqlx::query_as(
            "SELECT org_id, period_start, requests, egress_bytes FROM hosted_mock_usage_batches
             WHERE deployment_id = $1 AND batch_id = $2",
        )
        .bind(deployment_id)
        .bind(report.batch_id)
        .fetch_one(&mut *tx)
        .await?;
        if previous != (org_id, report.period_start, report.requests, report.egress_bytes) {
            return Err(ApiError::InvalidRequest(
                "batch_id was already used for different counts".into(),
            ));
        }
    } else {
        sqlx::query(
            "INSERT INTO usage_counters (org_id, period_start, requests, egress_bytes)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (org_id, period_start) DO UPDATE SET
             requests = usage_counters.requests + EXCLUDED.requests,
             egress_bytes = usage_counters.egress_bytes + EXCLUDED.egress_bytes,
             updated_at = NOW()",
        )
        .bind(org_id)
        .bind(report.period_start)
        .bind(report.requests)
        .bind(report.egress_bytes)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires isolated PostgreSQL via TEST_DATABASE_URL"]
    async fn usage_batches_replay_once_and_roll_back_receipt_on_counter_failure() {
        let url = std::env::var("TEST_DATABASE_URL").expect("isolated TEST_DATABASE_URL required");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let user = Uuid::new_v4();
        let org = Uuid::new_v4();
        let deployment = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, 'unusable-fixture')")
            .bind(user).bind(format!("usage-{user}")).bind(format!("usage-{user}@example.invalid"))
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO organizations (id, name, slug, owner_id) VALUES ($1, 'usage test', $2, $3)")
            .bind(org).bind(format!("usage-{org}")).bind(user).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO hosted_mocks (id, org_id, name, slug, config_json) VALUES ($1, $2, 'usage test', 'usage', '{}')")
            .bind(deployment).bind(org).execute(&pool).await.unwrap();
        let mut report = UsageReport {
            batch_id: Uuid::new_v4(),
            period_start: Utc::now().date_naive().with_day(1).unwrap(),
            requests: 3,
            egress_bytes: 7,
        };
        // Concurrent replays model both duplicate delivery and a lost acknowledgement.
        let (first, replay) = tokio::join!(
            apply_usage_batch(&pool, deployment, org, &report),
            apply_usage_batch(&pool, deployment, org, &report),
        );
        first.unwrap();
        replay.unwrap();
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT requests, egress_bytes FROM usage_counters WHERE org_id=$1 AND period_start=$2",
        )
        .bind(org)
        .bind(report.period_start)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(counts, (3, 7));
        report.requests = 4;
        assert!(
            apply_usage_batch(&pool, deployment, org, &report).await.is_err(),
            "changed payload reused a receipt"
        );
        report.batch_id = Uuid::new_v4();
        sqlx::query("UPDATE usage_counters SET egress_bytes=$1 WHERE org_id=$2")
            .bind(i64::MAX)
            .bind(org)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            apply_usage_batch(&pool, deployment, org, &report).await.is_err(),
            "counter overflow must fail"
        );
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT requests, egress_bytes FROM usage_counters WHERE org_id=$1 AND period_start=$2",
        )
        .bind(org)
        .bind(report.period_start)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(counts, (3, i64::MAX), "request increment must roll back with egress failure");
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM hosted_mock_usage_batches WHERE deployment_id=$1 AND batch_id=$2",
        )
        .bind(deployment)
        .bind(report.batch_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(receipts, 0, "failed count update must not retain a receipt");
        sqlx::query("UPDATE usage_counters SET egress_bytes=7 WHERE org_id=$1")
            .bind(org)
            .execute(&pool)
            .await
            .unwrap();
        apply_usage_batch(&pool, deployment, org, &report).await.unwrap();
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT requests, egress_bytes FROM usage_counters WHERE org_id=$1 AND period_start=$2",
        )
        .bind(org)
        .bind(report.period_start)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(counts, (7, 14));
        // Only the synthetic fixture rows created above are removed.
        sqlx::query("DELETE FROM organizations WHERE id=$1")
            .bind(org)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id=$1")
            .bind(user)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
}

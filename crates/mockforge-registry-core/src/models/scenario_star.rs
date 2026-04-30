//! Scenario star model — per-user favorite marker for marketplace scenarios.
//!
//! Mirrors `TemplateStar`. Counts are always computed from the `scenario_stars`
//! table at read time; we intentionally do not denormalize into `scenarios.*`
//! because toggling would otherwise take a row lock on the parent scenario on
//! every click and make popular scenarios a write-contention hotspot.

#[cfg(feature = "postgres")]
use std::collections::HashMap;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
pub struct ScenarioStar;

#[cfg(feature = "postgres")]
impl ScenarioStar {
    /// Toggle a star for (scenario_id, user_id).
    ///
    /// Returns `(now_starred, new_count)` where `now_starred` is `true` when
    /// the row was inserted by this call and `false` when it was removed. The
    /// count is authoritative — computed in the same transaction as the
    /// insert/delete so it never lies about the state the client just set.
    pub async fn toggle(
        pool: &PgPool,
        scenario_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<(bool, i64)> {
        let mut tx = pool.begin().await?;

        let already: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM scenario_stars WHERE scenario_id = $1 AND user_id = $2")
                .bind(scenario_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let now_starred = if already.is_some() {
            sqlx::query("DELETE FROM scenario_stars WHERE scenario_id = $1 AND user_id = $2")
                .bind(scenario_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            false
        } else {
            sqlx::query(
                "INSERT INTO scenario_stars (scenario_id, user_id) VALUES ($1, $2)
                 ON CONFLICT (scenario_id, user_id) DO NOTHING",
            )
            .bind(scenario_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            true
        };

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scenario_stars WHERE scenario_id = $1")
                .bind(scenario_id)
                .fetch_one(&mut *tx)
                .await?;

        tx.commit().await?;
        Ok((now_starred, count))
    }

    /// Whether `user_id` has starred `scenario_id`.
    pub async fn is_starred_by(
        pool: &PgPool,
        scenario_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM scenario_stars WHERE scenario_id = $1 AND user_id = $2")
                .bind(scenario_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
        Ok(row.is_some())
    }

    /// Count stars for a single scenario.
    pub async fn count_for_scenario(pool: &PgPool, scenario_id: Uuid) -> sqlx::Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scenario_stars WHERE scenario_id = $1")
                .bind(scenario_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Batch-count stars for a list of scenarios. Scenarios with zero stars
    /// are omitted from the returned map — callers should default to 0.
    pub async fn counts_for_scenarios(
        pool: &PgPool,
        scenario_ids: &[Uuid],
    ) -> sqlx::Result<HashMap<Uuid, i64>> {
        if scenario_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT scenario_id, COUNT(*)::bigint
             FROM scenario_stars
             WHERE scenario_id = ANY($1)
             GROUP BY scenario_id",
        )
        .bind(scenario_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().collect())
    }
}

//! Template star model — per-user favorite marker for marketplace templates.
//!
//! Stars are tracked as rows in `template_stars` (one row per user-template
//! pair). The star **count** is always computed from this table at read time
//! — we intentionally do not denormalize into `templates.stats_json.stars`,
//! because toggling would then take a row lock on the parent template on every
//! click and make popular templates a write-contention hotspot.
//!
//! Read paths use [`TemplateStar::count_for_template`] for a single row or
//! [`TemplateStar::counts_for_templates`] for a batch (one `GROUP BY` query,
//! rather than N round-trips).

#[cfg(feature = "postgres")]
use std::collections::HashMap;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
pub struct TemplateStar;

#[cfg(feature = "postgres")]
impl TemplateStar {
    /// Toggle a star for (template_id, user_id).
    ///
    /// Returns `(now_starred, new_count)` where `now_starred` is `true` when
    /// the row was inserted by this call and `false` when it was removed. The
    /// count is authoritative — computed in the same transaction as the
    /// insert/delete so it never lies about the state the client just set.
    pub async fn toggle(
        pool: &PgPool,
        template_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<(bool, i64)> {
        let mut tx = pool.begin().await?;

        let already: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM template_stars WHERE template_id = $1 AND user_id = $2")
                .bind(template_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let now_starred = if already.is_some() {
            sqlx::query("DELETE FROM template_stars WHERE template_id = $1 AND user_id = $2")
                .bind(template_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            false
        } else {
            sqlx::query(
                "INSERT INTO template_stars (template_id, user_id) VALUES ($1, $2)
                 ON CONFLICT (template_id, user_id) DO NOTHING",
            )
            .bind(template_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            true
        };

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM template_stars WHERE template_id = $1")
                .bind(template_id)
                .fetch_one(&mut *tx)
                .await?;

        tx.commit().await?;
        Ok((now_starred, count))
    }

    /// Whether `user_id` has starred `template_id`.
    pub async fn is_starred_by(
        pool: &PgPool,
        template_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM template_stars WHERE template_id = $1 AND user_id = $2")
                .bind(template_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
        Ok(row.is_some())
    }

    /// Count stars for a single template.
    pub async fn count_for_template(pool: &PgPool, template_id: Uuid) -> sqlx::Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM template_stars WHERE template_id = $1")
                .bind(template_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Batch-count stars for a list of templates. Templates with zero stars
    /// are omitted from the returned map — callers should default to 0.
    pub async fn counts_for_templates(
        pool: &PgPool,
        template_ids: &[Uuid],
    ) -> sqlx::Result<HashMap<Uuid, i64>> {
        if template_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT template_id, COUNT(*)::bigint
             FROM template_stars
             WHERE template_id = ANY($1)
             GROUP BY template_id",
        )
        .bind(template_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().collect())
    }
}

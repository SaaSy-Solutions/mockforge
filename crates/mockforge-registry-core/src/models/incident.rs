//! Incident management domain model (cloud-enablement task #3 / Phase 1).
//!
//! See `docs/cloud/CLOUD_INCIDENTS_DESIGN.md` for the full design.
//! Schema lives in migration 20250101000060_incidents.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

/// Persistent incident row. `dedupe_key` is source-scoped — the noisy
/// sources (drift detection, observability alerts) collapse repeat fires
/// onto a single open row via the partial-unique index in the migration.
#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub org_id: Uuid,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    pub source: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    pub dedupe_key: String,
    pub severity: String,
    pub status: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub postmortem_url: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<Uuid>,
    #[serde(default)]
    pub acknowledged_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub acknowledged_by: Option<Uuid>,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inputs for raising a new incident through the IncidentBus.
#[cfg(feature = "postgres")]
pub struct RaiseIncidentInput<'a> {
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub source: &'a str,
    pub source_ref: Option<&'a str>,
    pub dedupe_key: &'a str,
    pub severity: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
}

/// Append-only timeline of what happened on an incident.
#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentEvent {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub event_type: String,
    #[serde(default)]
    pub actor_id: Option<Uuid>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl Incident {
    pub async fn list_by_org(
        pool: &PgPool,
        org_id: Uuid,
        status_filter: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        match status_filter {
            Some(status) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM incidents WHERE org_id = $1 AND status = $2 \
                 ORDER BY created_at DESC LIMIT $3",
                )
                .bind(org_id)
                .bind(status)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM incidents WHERE org_id = $1 ORDER BY created_at DESC LIMIT $2",
                )
                .bind(org_id)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM incidents WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Insert a new incident, or no-op if there's already an open one with
    /// the same (org_id, source, dedupe_key). Returns the incident in either
    /// case, so callers always have an id to reference.
    ///
    /// Relies on the partial-unique index `idx_incidents_open_dedupe` —
    /// `ON CONFLICT DO NOTHING` against that index keeps repeated fires
    /// idempotent without needing application-side coordination.
    pub async fn raise(pool: &PgPool, input: RaiseIncidentInput<'_>) -> sqlx::Result<Self> {
        let mut tx = pool.begin().await?;

        // Try to insert. The partial unique index makes this fail silently
        // when there's already an open incident matching the dedupe key.
        let inserted: Option<Self> = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO incidents
                (org_id, workspace_id, source, source_ref, dedupe_key,
                 severity, status, title, description)
            VALUES ($1, $2, $3, $4, $5, $6, 'open', $7, $8)
            ON CONFLICT (org_id, source, dedupe_key)
                WHERE status != 'resolved'
                DO NOTHING
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.workspace_id)
        .bind(input.source)
        .bind(input.source_ref)
        .bind(input.dedupe_key)
        .bind(input.severity)
        .bind(input.title)
        .bind(input.description)
        .fetch_optional(&mut *tx)
        .await?;

        let incident = match inserted {
            Some(row) => {
                // Newly created — log the 'created' event in the same tx.
                sqlx::query(
                    "INSERT INTO incident_events (incident_id, event_type) VALUES ($1, 'created')",
                )
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
                row
            }
            None => {
                // Already existed; fetch the open row.
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM incidents \
                     WHERE org_id = $1 AND source = $2 AND dedupe_key = $3 AND status != 'resolved' \
                     LIMIT 1",
                )
                .bind(input.org_id)
                .bind(input.source)
                .bind(input.dedupe_key)
                .fetch_one(&mut *tx)
                .await?
            }
        };

        tx.commit().await?;
        Ok(incident)
    }

    /// Mark all open incidents matching (org_id, source, dedupe_key) as
    /// resolved. Used by sources that auto-resolve when the underlying
    /// signal recovers (e.g., next clean drift check).
    pub async fn auto_resolve(
        pool: &PgPool,
        org_id: Uuid,
        source: &str,
        dedupe_key: &str,
    ) -> sqlx::Result<u64> {
        let mut tx = pool.begin().await?;

        let rows = sqlx::query(
            r#"
            UPDATE incidents SET
                status = 'resolved',
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE org_id = $1
              AND source = $2
              AND dedupe_key = $3
              AND status != 'resolved'
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(source)
        .bind(dedupe_key)
        .fetch_all(&mut *tx)
        .await?;

        for row in &rows {
            let id: Uuid = sqlx::Row::get(row, "id");
            sqlx::query(
                "INSERT INTO incident_events (incident_id, event_type, payload) \
                 VALUES ($1, 'resolved', '{\"auto\":true}'::jsonb)",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(rows.len() as u64)
    }

    pub async fn acknowledge(
        pool: &PgPool,
        id: Uuid,
        actor_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        let mut tx = pool.begin().await?;

        let updated: Option<Self> = sqlx::query_as::<_, Self>(
            r#"
            UPDATE incidents SET
                status = CASE WHEN status = 'open' THEN 'acknowledged' ELSE status END,
                acknowledged_at = COALESCE(acknowledged_at, NOW()),
                acknowledged_by = COALESCE(acknowledged_by, $2),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;

        if updated.is_some() {
            sqlx::query(
                "INSERT INTO incident_events (incident_id, event_type, actor_id) \
                 VALUES ($1, 'acknowledged', $2)",
            )
            .bind(id)
            .bind(actor_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn resolve(pool: &PgPool, id: Uuid, actor_id: Uuid) -> sqlx::Result<Option<Self>> {
        let mut tx = pool.begin().await?;

        let updated: Option<Self> = sqlx::query_as::<_, Self>(
            r#"
            UPDATE incidents SET
                status = 'resolved',
                resolved_at = COALESCE(resolved_at, NOW()),
                resolved_by = COALESCE(resolved_by, $2),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;

        if updated.is_some() {
            sqlx::query(
                "INSERT INTO incident_events (incident_id, event_type, actor_id) \
                 VALUES ($1, 'resolved', $2)",
            )
            .bind(id)
            .bind(actor_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn list_events(pool: &PgPool, incident_id: Uuid) -> sqlx::Result<Vec<IncidentEvent>> {
        sqlx::query_as::<_, IncidentEvent>(
            "SELECT * FROM incident_events WHERE incident_id = $1 ORDER BY created_at ASC",
        )
        .bind(incident_id)
        .fetch_all(pool)
        .await
    }

    /// Open incidents that haven't been dispatched to notification channels
    /// yet. The dispatcher worker polls this; once it inserts a
    /// `notification_dispatched` incident_event for an incident, that
    /// incident drops out of the list.
    ///
    /// Capped at `limit` rows so a backlog can't OOM the worker.
    pub async fn list_pending_dispatch(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT i.*
              FROM incidents i
             WHERE i.status = 'open'
               AND NOT EXISTS (
                   SELECT 1 FROM incident_events e
                    WHERE e.incident_id = i.id
                      AND e.event_type = 'notification_dispatched'
               )
             ORDER BY i.created_at ASC
             LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Worker-callback: record one dispatch attempt per (incident, channel).
    /// `result` is freeform JSON — typically `{"ok": true, "status": 204}`
    /// for success or `{"ok": false, "error": "..."}` for failure.
    /// Multiple rows per incident are expected (one per channel).
    pub async fn record_notification_attempt(
        pool: &PgPool,
        incident_id: Uuid,
        channel_id: Uuid,
        result: &serde_json::Value,
    ) -> sqlx::Result<()> {
        let mut payload = result.clone();
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("channel_id".into(), serde_json::json!(channel_id));
        }
        sqlx::query(
            "INSERT INTO incident_events (incident_id, event_type, payload) \
             VALUES ($1, 'notification_sent', $2)",
        )
        .bind(incident_id)
        .bind(payload)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Worker-callback: incident has been processed by the dispatcher.
    /// Inserting this row removes the incident from
    /// `list_pending_dispatch`. Idempotent: a duplicate insert is fine —
    /// the next poll just finds zero pending rows.
    pub async fn mark_dispatched(
        pool: &PgPool,
        incident_id: Uuid,
        summary: &serde_json::Value,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO incident_events (incident_id, event_type, payload) \
             VALUES ($1, 'notification_dispatched', $2)",
        )
        .bind(incident_id)
        .bind(summary)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Resolved incidents that have NOT yet had a resolve-side
    /// notification fanout. Symmetric to `list_pending_dispatch` but
    /// gated on:
    ///
    ///   1. status = 'resolved'  — the incident has been closed
    ///   2. `notification_dispatched` exists — we previously sent a
    ///      "trigger" alert, so there's a paired alert to close
    ///   3. `notification_resolution_dispatched` does NOT exist —
    ///      we haven't already sent the resolve
    ///
    /// Condition (2) prevents a resolve fanout for incidents that were
    /// auto-resolved before they ever fired (created and resolved within
    /// the dispatcher's poll window, or filtered to zero channels) —
    /// users don't expect "your alert is resolved" emails for alerts
    /// they never received.
    pub async fn list_pending_resolution_dispatch(
        pool: &PgPool,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT i.*
              FROM incidents i
             WHERE i.status = 'resolved'
               AND EXISTS (
                   SELECT 1 FROM incident_events e
                    WHERE e.incident_id = i.id
                      AND e.event_type = 'notification_dispatched'
               )
               AND NOT EXISTS (
                   SELECT 1 FROM incident_events e
                    WHERE e.incident_id = i.id
                      AND e.event_type = 'notification_resolution_dispatched'
               )
             ORDER BY i.resolved_at ASC NULLS LAST
             LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Worker-callback: record one resolution-dispatch attempt per
    /// (incident, channel). Symmetric to `record_notification_attempt`
    /// but uses a distinct event_type so trigger-side and resolve-side
    /// attempts don't collide in `incident_events`.
    pub async fn record_resolution_attempt(
        pool: &PgPool,
        incident_id: Uuid,
        channel_id: Uuid,
        result: &serde_json::Value,
    ) -> sqlx::Result<()> {
        let mut payload = result.clone();
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("channel_id".into(), serde_json::json!(channel_id));
        }
        sqlx::query(
            "INSERT INTO incident_events (incident_id, event_type, payload) \
             VALUES ($1, 'notification_resolution_sent', $2)",
        )
        .bind(incident_id)
        .bind(payload)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Worker-callback: the resolve-side fanout has completed for this
    /// incident. Inserting this row removes the incident from
    /// `list_pending_resolution_dispatch`. Idempotent for the same
    /// reason `mark_dispatched` is.
    pub async fn mark_resolution_dispatched(
        pool: &PgPool,
        incident_id: Uuid,
        summary: &serde_json::Value,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO incident_events (incident_id, event_type, payload) \
             VALUES ($1, 'notification_resolution_dispatched', $2)",
        )
        .bind(incident_id)
        .bind(summary)
        .execute(pool)
        .await?;
        Ok(())
    }
}

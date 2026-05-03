//! Routing rules for the incidents subsystem (cloud-enablement task #3 /
//! Phase 1, follow-up slice).
//!
//! Each row maps an incoming incident's (severity × source × workspace)
//! to a list of notification channels. Rules are evaluated in priority
//! order (lower priority number = evaluated first); when the dispatcher
//! finds a matching rule, it stops and uses that rule's `channel_ids`.
//!
//! Empty `match_severity` / `match_source` arrays act as wildcards.
//! `match_workspace_id` is a single value (NULL = wildcard) since most
//! customers want at most one workspace-scoped rule per incident.
//!
//! Schema lives in migration 20250101000060_incidents.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: Uuid,
    pub org_id: Uuid,
    pub priority: i32,
    pub match_severity: Vec<String>,
    pub match_source: Vec<String>,
    #[serde(default)]
    pub match_workspace_id: Option<Uuid>,
    pub channel_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateRoutingRule<'a> {
    pub org_id: Uuid,
    pub priority: i32,
    pub match_severity: &'a [String],
    pub match_source: &'a [String],
    pub match_workspace_id: Option<Uuid>,
    pub channel_ids: &'a [Uuid],
}

#[cfg(feature = "postgres")]
impl RoutingRule {
    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM routing_rules WHERE org_id = $1 ORDER BY priority ASC, created_at ASC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM routing_rules WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateRoutingRule<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO routing_rules
                (org_id, priority, match_severity, match_source,
                 match_workspace_id, channel_ids)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.priority)
        .bind(input.match_severity)
        .bind(input.match_source)
        .bind(input.match_workspace_id)
        .bind(input.channel_ids)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        priority: Option<i32>,
        match_severity: Option<&[String]>,
        match_source: Option<&[String]>,
        channel_ids: Option<&[Uuid]>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE routing_rules SET
                priority = COALESCE($2, priority),
                match_severity = COALESCE($3, match_severity),
                match_source = COALESCE($4, match_source),
                channel_ids = COALESCE($5, channel_ids),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(priority)
        .bind(match_severity)
        .bind(match_source)
        .bind(channel_ids)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM routing_rules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }

    /// Returns true if this rule matches the given incident dimensions.
    /// Used by the dispatcher worker (separate slice) when a new incident
    /// is raised.
    pub fn matches(&self, severity: &str, source: &str, workspace_id: Option<Uuid>) -> bool {
        if !self.match_severity.is_empty() && !self.match_severity.iter().any(|s| s == severity) {
            return false;
        }
        if !self.match_source.is_empty() && !self.match_source.iter().any(|s| s == source) {
            return false;
        }
        if let Some(rule_ws) = self.match_workspace_id {
            if Some(rule_ws) != workspace_id {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #![cfg(feature = "postgres")]
    use super::*;
    use chrono::Utc;

    fn rule_with(
        match_severity: Vec<&str>,
        match_source: Vec<&str>,
        match_workspace_id: Option<Uuid>,
    ) -> RoutingRule {
        RoutingRule {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            priority: 100,
            match_severity: match_severity.into_iter().map(String::from).collect(),
            match_source: match_source.into_iter().map(String::from).collect(),
            match_workspace_id,
            channel_ids: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_match_lists_are_wildcards() {
        let rule = rule_with(vec![], vec![], None);
        assert!(rule.matches("critical", "drift", None));
        assert!(rule.matches("low", "external", Some(Uuid::new_v4())));
    }

    #[test]
    fn severity_filter_excludes_others() {
        let rule = rule_with(vec!["critical", "high"], vec![], None);
        assert!(rule.matches("critical", "drift", None));
        assert!(rule.matches("high", "drift", None));
        assert!(!rule.matches("medium", "drift", None));
        assert!(!rule.matches("low", "drift", None));
    }

    #[test]
    fn source_filter_excludes_others() {
        let rule = rule_with(vec![], vec!["drift", "observability"], None);
        assert!(rule.matches("low", "drift", None));
        assert!(!rule.matches("low", "external", None));
    }

    #[test]
    fn workspace_filter_requires_exact_match() {
        let ws = Uuid::new_v4();
        let other = Uuid::new_v4();
        let rule = rule_with(vec![], vec![], Some(ws));
        assert!(rule.matches("low", "drift", Some(ws)));
        assert!(!rule.matches("low", "drift", Some(other)));
        assert!(!rule.matches("low", "drift", None)); // None workspace doesn't match scoped rule
    }

    #[test]
    fn all_filters_must_match() {
        let ws = Uuid::new_v4();
        let rule = rule_with(vec!["critical"], vec!["drift"], Some(ws));
        assert!(rule.matches("critical", "drift", Some(ws)));
        assert!(!rule.matches("critical", "external", Some(ws)));
        assert!(!rule.matches("low", "drift", Some(ws)));
        assert!(!rule.matches("critical", "drift", None));
    }
}

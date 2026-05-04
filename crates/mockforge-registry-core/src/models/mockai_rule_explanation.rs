//! MockAI rule explanations stored on the cloud registry.
//!
//! Replaces the local-only `/__mockforge/api/mockai/rules/*` surface
//! when the UI is in cloud mode. Each row is the LLM-generated
//! explanation of a rule learned from example request/response pairs.
//! Schema lives in migration `20250101000072_mockai_rule_explanations.sql`
//! (cloud-enablement task #353).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockaiRuleExplanation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub rule_id: String,
    pub rule_type: String,
    pub confidence: f32,
    pub source_examples: serde_json::Value,
    pub reasoning: String,
    pub pattern_matches: serde_json::Value,
    pub generated_at: DateTime<Utc>,
}

/// Inputs for inserting/upserting a rule explanation row.
#[cfg(feature = "postgres")]
pub struct UpsertMockaiRuleExplanation<'a> {
    pub workspace_id: Uuid,
    pub rule_id: &'a str,
    pub rule_type: &'a str,
    pub confidence: f32,
    pub source_examples: &'a serde_json::Value,
    pub reasoning: &'a str,
    pub pattern_matches: &'a serde_json::Value,
}

#[cfg(feature = "postgres")]
impl MockaiRuleExplanation {
    /// List rule explanations for a workspace, optionally filtering by
    /// rule_type and a minimum confidence.
    pub async fn list_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        rule_type: Option<&str>,
        min_confidence: Option<f32>,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, workspace_id, rule_id, rule_type, confidence,
                   source_examples, reasoning, pattern_matches, generated_at
            FROM cloud_mockai_rule_explanations
            WHERE workspace_id = $1
              AND ($2::text IS NULL OR rule_type = $2)
              AND ($3::real IS NULL OR confidence >= $3)
            ORDER BY generated_at DESC
            "#,
        )
        .bind(workspace_id)
        .bind(rule_type)
        .bind(min_confidence)
        .fetch_all(pool)
        .await
    }

    /// Get a single rule explanation by (workspace_id, rule_id).
    pub async fn get_by_rule_id(
        pool: &PgPool,
        workspace_id: Uuid,
        rule_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, workspace_id, rule_id, rule_type, confidence,
                   source_examples, reasoning, pattern_matches, generated_at
            FROM cloud_mockai_rule_explanations
            WHERE workspace_id = $1 AND rule_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(rule_id)
        .fetch_optional(pool)
        .await
    }

    /// Upsert a rule explanation. Conflict on (workspace_id, rule_id)
    /// rewrites — the LLM is free to revise the explanation as more
    /// examples arrive.
    pub async fn upsert(
        pool: &PgPool,
        input: UpsertMockaiRuleExplanation<'_>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO cloud_mockai_rule_explanations
                (workspace_id, rule_id, rule_type, confidence,
                 source_examples, reasoning, pattern_matches)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (workspace_id, rule_id) DO UPDATE SET
                rule_type = EXCLUDED.rule_type,
                confidence = EXCLUDED.confidence,
                source_examples = EXCLUDED.source_examples,
                reasoning = EXCLUDED.reasoning,
                pattern_matches = EXCLUDED.pattern_matches,
                generated_at = NOW()
            RETURNING id, workspace_id, rule_id, rule_type, confidence,
                      source_examples, reasoning, pattern_matches, generated_at
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.rule_id)
        .bind(input.rule_type)
        .bind(input.confidence)
        .bind(input.source_examples)
        .bind(input.reasoning)
        .bind(input.pattern_matches)
        .fetch_one(pool)
        .await
    }
}

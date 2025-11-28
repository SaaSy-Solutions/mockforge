//! Organization model for multi-tenancy

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Organization plan type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
    Team,
}

impl Default for Plan {
    fn default() -> Self {
        Plan::Free
    }
}

/// Organization model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub plan: String, // Stored as VARCHAR, converted via methods
    pub limits_json: serde_json::Value,
    pub stripe_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
}

impl Default for OrgRole {
    fn default() -> Self {
        OrgRole::Member
    }
}

/// Organization member
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrgMember {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: String, // Stored as VARCHAR, converted via methods
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Get plan as enum
    pub fn plan(&self) -> Plan {
        match self.plan.as_str() {
            "free" => Plan::Free,
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            _ => Plan::Free,
        }
    }

    /// Create a new organization
    /// Also creates the owner membership automatically
    pub async fn create(
        pool: &sqlx::PgPool,
        name: &str,
        slug: &str,
        owner_id: Uuid,
        plan: Plan,
    ) -> sqlx::Result<Self> {
        let limits = get_default_limits(plan);

        // Use a transaction to create org and owner membership atomically
        let mut tx = pool.begin().await?;

        let org = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO organizations (name, slug, owner_id, plan, limits_json)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(owner_id)
        .bind(plan.to_string())
        .bind(serde_json::to_value(limits).unwrap())
        .fetch_one(&mut *tx)
        .await?;

        // Create owner membership
        sqlx::query(
            r#"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, 'owner')
            ON CONFLICT (org_id, user_id) DO NOTHING
            "#,
        )
        .bind(org.id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(org)
    }

    /// Find organization by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find organization by slug
    pub async fn find_by_slug(pool: &sqlx::PgPool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    /// Get organizations for a user (as owner or member)
    pub async fn find_by_user(pool: &sqlx::PgPool, user_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT DISTINCT o.*
            FROM organizations o
            LEFT JOIN org_members om ON o.id = om.org_id
            WHERE o.owner_id = $1 OR om.user_id = $1
            ORDER BY o.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /// Get or create user's personal organization
    /// This ensures every user has at least one org (backward compatibility)
    pub async fn get_or_create_personal_org(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        username: &str,
    ) -> sqlx::Result<Self> {
        // Try to find existing personal org
        if let Some(org) = sqlx::query_as::<_, Self>(
            "SELECT * FROM organizations WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        {
            return Ok(org);
        }

        // Create new personal org
        let slug = format!(
            "org-{}",
            username
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .trim_matches('-')
                .replace("--", "-")
        );

        // Ensure slug is unique
        let mut final_slug = slug.clone();
        let mut counter = 1;
        while Organization::find_by_slug(pool, &final_slug)
            .await?
            .is_some()
        {
            final_slug = format!("{}-{}", slug, counter);
            counter += 1;
        }

        Self::create(pool, &format!("{}'s Organization", username), &final_slug, user_id, Plan::Free)
            .await
    }

    /// Update organization plan
    pub async fn update_plan(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        plan: Plan,
    ) -> sqlx::Result<()> {
        let limits = get_default_limits(plan);

        sqlx::query(
            r#"
            UPDATE organizations
            SET plan = $1, limits_json = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(plan.to_string())
        .bind(serde_json::to_value(limits).unwrap())
        .bind(org_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update Stripe customer ID
    pub async fn update_stripe_customer_id(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        stripe_customer_id: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE organizations SET stripe_customer_id = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(stripe_customer_id)
        .bind(org_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl Plan {
    pub fn to_string(&self) -> String {
        match self {
            Plan::Free => "free".to_string(),
            Plan::Pro => "pro".to_string(),
            Plan::Team => "team".to_string(),
        }
    }
}

impl OrgMember {
    /// Get role as enum
    pub fn role(&self) -> OrgRole {
        match self.role.as_str() {
            "owner" => OrgRole::Owner,
            "admin" => OrgRole::Admin,
            "member" => OrgRole::Member,
            _ => OrgRole::Member,
        }
    }

    /// Add a member to an organization
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role.to_string())
        .fetch_one(pool)
        .await
    }

    /// Find member by org and user
    pub async fn find(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_members WHERE org_id = $1 AND user_id = $2",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Get all members of an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_members WHERE org_id = $1 ORDER BY created_at ASC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Update member role
    pub async fn update_role(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE org_members SET role = $1, updated_at = NOW() WHERE org_id = $2 AND user_id = $3",
        )
        .bind(role.to_string())
        .bind(org_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove member from organization
    pub async fn delete(pool: &sqlx::PgPool, org_id: Uuid, user_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM org_members WHERE org_id = $1 AND user_id = $2")
            .bind(org_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl OrgRole {
    pub fn to_string(&self) -> String {
        match self {
            OrgRole::Owner => "owner".to_string(),
            OrgRole::Admin => "admin".to_string(),
            OrgRole::Member => "member".to_string(),
        }
    }

    pub fn can_manage_members(&self) -> bool {
        matches!(self, OrgRole::Owner | OrgRole::Admin)
    }

    pub fn can_manage_billing(&self) -> bool {
        matches!(self, OrgRole::Owner)
    }
}

/// Get default limits for a plan
fn get_default_limits(plan: Plan) -> serde_json::Value {
    match plan {
        Plan::Free => serde_json::json!({
            "max_projects": 1,
            "max_collaborators": 1,
            "max_environments": 1,
            "requests_per_30d": 10000,
            "storage_gb": 1,
            "max_plugins_published": 1,
            "max_templates_published": 3,
            "max_scenarios_published": 1,
            "ai_tokens_per_month": 0, // BYOK only
            "hosted_mocks": false
        }),
        Plan::Pro => serde_json::json!({
            "max_projects": 10,
            "max_collaborators": 5,
            "max_environments": 3,
            "requests_per_30d": 250000,
            "storage_gb": 20,
            "max_plugins_published": 10,
            "max_templates_published": 50,
            "max_scenarios_published": 20,
            "ai_tokens_per_month": 100000,
            "hosted_mocks": true
        }),
        Plan::Team => serde_json::json!({
            "max_projects": -1, // unlimited
            "max_collaborators": 20,
            "max_environments": 10,
            "requests_per_30d": 1000000,
            "storage_gb": 100,
            "max_plugins_published": -1, // unlimited
            "max_templates_published": -1, // unlimited
            "max_scenarios_published": -1, // unlimited
            "ai_tokens_per_month": 1000000,
            "hosted_mocks": true
        }),
    }
}

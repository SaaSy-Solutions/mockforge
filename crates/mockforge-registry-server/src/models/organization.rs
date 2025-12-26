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
        .bind(serde_json::to_value(&limits).map_err(|e| sqlx::Error::Protocol(e.to_string()))?)
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
        while Organization::find_by_slug(pool, &final_slug).await?.is_some() {
            final_slug = format!("{}-{}", slug, counter);
            counter += 1;
        }

        Self::create(
            pool,
            &format!("{}'s Organization", username),
            &final_slug,
            user_id,
            Plan::Free,
        )
        .await
    }

    /// Update organization plan
    pub async fn update_plan(pool: &sqlx::PgPool, org_id: Uuid, plan: Plan) -> sqlx::Result<()> {
        let limits = get_default_limits(plan);

        sqlx::query(
            r#"
            UPDATE organizations
            SET plan = $1, limits_json = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(plan.to_string())
        .bind(serde_json::to_value(&limits).map_err(|e| sqlx::Error::Protocol(e.to_string()))?)
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
        sqlx::query_as::<_, Self>("SELECT * FROM org_members WHERE org_id = $1 AND user_id = $2")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_default() {
        assert_eq!(Plan::default(), Plan::Free);
    }

    #[test]
    fn test_plan_to_string() {
        assert_eq!(Plan::Free.to_string(), "free");
        assert_eq!(Plan::Pro.to_string(), "pro");
        assert_eq!(Plan::Team.to_string(), "team");
    }

    #[test]
    fn test_plan_serialization() {
        let plan = Plan::Free;
        let json = serde_json::to_string(&plan).unwrap();
        assert_eq!(json, "\"free\"");

        let plan = Plan::Pro;
        let json = serde_json::to_string(&plan).unwrap();
        assert_eq!(json, "\"pro\"");

        let plan = Plan::Team;
        let json = serde_json::to_string(&plan).unwrap();
        assert_eq!(json, "\"team\"");
    }

    #[test]
    fn test_plan_deserialization() {
        let plan: Plan = serde_json::from_str("\"free\"").unwrap();
        assert_eq!(plan, Plan::Free);

        let plan: Plan = serde_json::from_str("\"pro\"").unwrap();
        assert_eq!(plan, Plan::Pro);

        let plan: Plan = serde_json::from_str("\"team\"").unwrap();
        assert_eq!(plan, Plan::Team);
    }

    #[test]
    fn test_plan_equality() {
        assert_eq!(Plan::Free, Plan::Free);
        assert_ne!(Plan::Free, Plan::Pro);
        assert_ne!(Plan::Pro, Plan::Team);
    }

    #[test]
    fn test_org_role_default() {
        assert_eq!(OrgRole::default(), OrgRole::Member);
    }

    #[test]
    fn test_org_role_to_string() {
        assert_eq!(OrgRole::Owner.to_string(), "owner");
        assert_eq!(OrgRole::Admin.to_string(), "admin");
        assert_eq!(OrgRole::Member.to_string(), "member");
    }

    #[test]
    fn test_org_role_serialization() {
        let role = OrgRole::Owner;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"owner\"");

        let role = OrgRole::Admin;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"admin\"");

        let role = OrgRole::Member;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"member\"");
    }

    #[test]
    fn test_org_role_deserialization() {
        let role: OrgRole = serde_json::from_str("\"owner\"").unwrap();
        assert_eq!(role, OrgRole::Owner);

        let role: OrgRole = serde_json::from_str("\"admin\"").unwrap();
        assert_eq!(role, OrgRole::Admin);

        let role: OrgRole = serde_json::from_str("\"member\"").unwrap();
        assert_eq!(role, OrgRole::Member);
    }

    #[test]
    fn test_org_role_can_manage_members() {
        assert!(OrgRole::Owner.can_manage_members());
        assert!(OrgRole::Admin.can_manage_members());
        assert!(!OrgRole::Member.can_manage_members());
    }

    #[test]
    fn test_org_role_can_manage_billing() {
        assert!(OrgRole::Owner.can_manage_billing());
        assert!(!OrgRole::Admin.can_manage_billing());
        assert!(!OrgRole::Member.can_manage_billing());
    }

    #[test]
    fn test_organization_plan_method() {
        let org = Organization {
            id: Uuid::new_v4(),
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            owner_id: Uuid::new_v4(),
            plan: "free".to_string(),
            limits_json: serde_json::json!({}),
            stripe_customer_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(org.plan(), Plan::Free);

        let org = Organization {
            plan: "pro".to_string(),
            ..org
        };
        assert_eq!(org.plan(), Plan::Pro);

        let org = Organization {
            plan: "team".to_string(),
            ..org
        };
        assert_eq!(org.plan(), Plan::Team);

        // Invalid plan should default to Free
        let org = Organization {
            plan: "invalid".to_string(),
            ..org
        };
        assert_eq!(org.plan(), Plan::Free);
    }

    #[test]
    fn test_org_member_role_method() {
        let member = OrgMember {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: "owner".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(member.role(), OrgRole::Owner);

        let member = OrgMember {
            role: "admin".to_string(),
            ..member
        };
        assert_eq!(member.role(), OrgRole::Admin);

        let member = OrgMember {
            role: "member".to_string(),
            ..member
        };
        assert_eq!(member.role(), OrgRole::Member);

        // Invalid role should default to Member
        let member = OrgMember {
            role: "invalid".to_string(),
            ..member
        };
        assert_eq!(member.role(), OrgRole::Member);
    }

    #[test]
    fn test_get_default_limits_free() {
        let limits = get_default_limits(Plan::Free);

        assert_eq!(limits["max_projects"], 1);
        assert_eq!(limits["max_collaborators"], 1);
        assert_eq!(limits["max_environments"], 1);
        assert_eq!(limits["requests_per_30d"], 10000);
        assert_eq!(limits["storage_gb"], 1);
        assert_eq!(limits["max_plugins_published"], 1);
        assert_eq!(limits["max_templates_published"], 3);
        assert_eq!(limits["max_scenarios_published"], 1);
        assert_eq!(limits["ai_tokens_per_month"], 0);
        assert_eq!(limits["hosted_mocks"], false);
    }

    #[test]
    fn test_get_default_limits_pro() {
        let limits = get_default_limits(Plan::Pro);

        assert_eq!(limits["max_projects"], 10);
        assert_eq!(limits["max_collaborators"], 5);
        assert_eq!(limits["max_environments"], 3);
        assert_eq!(limits["requests_per_30d"], 250000);
        assert_eq!(limits["storage_gb"], 20);
        assert_eq!(limits["max_plugins_published"], 10);
        assert_eq!(limits["max_templates_published"], 50);
        assert_eq!(limits["max_scenarios_published"], 20);
        assert_eq!(limits["ai_tokens_per_month"], 100000);
        assert_eq!(limits["hosted_mocks"], true);
    }

    #[test]
    fn test_get_default_limits_team() {
        let limits = get_default_limits(Plan::Team);

        assert_eq!(limits["max_projects"], -1); // unlimited
        assert_eq!(limits["max_collaborators"], 20);
        assert_eq!(limits["max_environments"], 10);
        assert_eq!(limits["requests_per_30d"], 1000000);
        assert_eq!(limits["storage_gb"], 100);
        assert_eq!(limits["max_plugins_published"], -1); // unlimited
        assert_eq!(limits["max_templates_published"], -1); // unlimited
        assert_eq!(limits["max_scenarios_published"], -1); // unlimited
        assert_eq!(limits["ai_tokens_per_month"], 1000000);
        assert_eq!(limits["hosted_mocks"], true);
    }

    #[test]
    fn test_organization_serialization() {
        let org = Organization {
            id: Uuid::new_v4(),
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            owner_id: Uuid::new_v4(),
            plan: "free".to_string(),
            limits_json: serde_json::json!({"max_projects": 1}),
            stripe_customer_id: Some("cus_123".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&org).unwrap();
        assert!(json.contains("Test Org"));
        assert!(json.contains("test-org"));
        assert!(json.contains("free"));
        assert!(json.contains("cus_123"));
    }

    #[test]
    fn test_org_member_serialization() {
        let member = OrgMember {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: "admin".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("admin"));
    }

    #[test]
    fn test_plan_copy_and_clone() {
        let plan1 = Plan::Pro;
        let plan2 = plan1;
        let plan3 = plan1.clone();

        assert_eq!(plan1, plan2);
        assert_eq!(plan1, plan3);
    }

    #[test]
    fn test_org_role_copy_and_clone() {
        let role1 = OrgRole::Admin;
        let role2 = role1;
        let role3 = role1.clone();

        assert_eq!(role1, role2);
        assert_eq!(role1, role3);
    }

    #[test]
    fn test_organization_clone() {
        let org = Organization {
            id: Uuid::new_v4(),
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            owner_id: Uuid::new_v4(),
            plan: "free".to_string(),
            limits_json: serde_json::json!({}),
            stripe_customer_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = org.clone();
        assert_eq!(org.id, cloned.id);
        assert_eq!(org.name, cloned.name);
        assert_eq!(org.slug, cloned.slug);
    }
}

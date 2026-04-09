//! SQLite implementation of [`RegistryStore`].
//!
//! This backend powers the single-tenant OSS admin server embedded in
//! `mockforge-ui`. It targets a subset of the domain — authentication,
//! users, organizations, API tokens, audit logs, settings — and returns
//! sensible empty defaults (or [`StoreError::NotFound`]) for SaaS-only
//! features like the plugin/template/scenario marketplace, SSO/SAML,
//! Stripe billing, hosted mocks, federations, and cloud workspaces.
//!
//! The migrations for this backend live in `../../migrations-sqlite/` and
//! are applied at connect time via [`SqliteRegistryStore::connect`].
//!
//! This is the Phase 2b skeleton — the struct, the connect/migrate entry
//! points, and a `RegistryStore` impl whose methods are generated stubs.
//! Follow-up commits will fill in real SQLite queries for the
//! OSS-essential paths (users, api tokens, orgs, audit logging).

#![cfg(feature = "sqlite")]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

use super::{
    AdminAnalyticsSnapshot, ConversionFunnelSnapshot, OrgSettingRow, ProjectRow, RegistryStore,
    SubscriptionRow, UserSettingRow,
};
use crate::error::{StoreError, StoreResult};
use crate::models::api_token::{ApiToken, TokenScope};
use crate::models::audit_log::{AuditEventType, AuditLog};
use crate::models::cloud_fixture::CloudFixture;
use crate::models::cloud_service::CloudService;
use crate::models::cloud_workspace::Workspace as CloudWorkspace;
use crate::models::feature_usage::FeatureType;
#[allow(unused_imports)]
use crate::models::feature_usage::FeatureUsage;
use crate::models::federation::Federation;
use crate::models::hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
use crate::models::org_template::OrgTemplate;
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::plugin::{Plugin, PluginVersion};
use crate::models::review::Review;
use crate::models::saml_assertion::SAMLAssertionId;
use crate::models::scenario::Scenario;
use crate::models::scenario_review::ScenarioReview;
use crate::models::settings::OrgSetting;
use crate::models::sso::{SSOConfiguration, SSOProvider};
use crate::models::subscription::UsageCounter;
use crate::models::suspicious_activity::{SuspiciousActivity, SuspiciousActivityType};
use crate::models::template::{Template, TemplateCategory};
use crate::models::template_review::TemplateReview;
use crate::models::user::User;
use crate::models::verification_token::VerificationToken;
use crate::models::waitlist::WaitlistSubscriber;

/// SQLite-backed [`RegistryStore`] implementation for the OSS admin UI.
#[derive(Clone)]
pub struct SqliteRegistryStore {
    pool: SqlitePool,
}

impl SqliteRegistryStore {
    /// Wrap an existing [`SqlitePool`] in a registry store.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Open (and create if missing) a SQLite database at the given URL and
    /// run the bundled OSS migrations.
    ///
    /// The URL must be in sqlx form, e.g. `sqlite://./mockforge.db` or
    /// `sqlite::memory:` for an in-process database.
    pub async fn connect(database_url: &str) -> StoreResult<Self> {
        let pool = SqlitePoolOptions::new().max_connections(5).connect(database_url).await?;
        let this = Self { pool };
        this.migrate().await?;
        Ok(this)
    }

    /// Run the bundled SQLite migrations (subset of the Postgres schema).
    pub async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("../mockforge-registry-server/migrations-sqlite")
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Hash(format!("migrate: {}", e)))?;
        Ok(())
    }

    /// Borrow the underlying pool. Exposed for tests and advanced wiring.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// --- row <-> model helpers -------------------------------------------------
//
// SQLite lacks native UUID / array column types. We store UUIDs as TEXT and
// Vec<String> values (e.g. 2FA backup codes) as JSON-encoded TEXT, then map
// them back into our strongly-typed model structs via these helpers.

fn parse_uuid(s: &str) -> StoreResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| StoreError::Hash(format!("invalid uuid '{}': {}", s, e)))
}

fn parse_dt(s: &str) -> StoreResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // sqlite's datetime('now') returns `YYYY-MM-DD HH:MM:SS` (no T, no tz)
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
        .map_err(|e| StoreError::Hash(format!("bad datetime '{}': {}", s, e)))
}

fn row_to_api_token(row: &sqlx::sqlite::SqliteRow) -> StoreResult<ApiToken> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: String = row.try_get("org_id")?;
    let user_id_str: Option<String> = row.try_get("user_id")?;
    let scopes_json: String = row.try_get("scopes")?;
    let last_used_at_str: Option<String> = row.try_get("last_used_at")?;
    let expires_at_str: Option<String> = row.try_get("expires_at")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let scopes: Vec<String> = serde_json::from_str(&scopes_json)
        .map_err(|e| StoreError::Hash(format!("bad scopes json: {}", e)))?;

    Ok(ApiToken {
        id: parse_uuid(&id_str)?,
        org_id: parse_uuid(&org_id_str)?,
        user_id: user_id_str.as_deref().map(parse_uuid).transpose()?,
        name: row.try_get("name")?,
        token_prefix: row.try_get("token_prefix")?,
        hashed_token: row.try_get("hashed_token")?,
        scopes,
        last_used_at: last_used_at_str.as_deref().map(parse_dt).transpose()?,
        expires_at: expires_at_str.as_deref().map(parse_dt).transpose()?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_organization(row: &sqlx::sqlite::SqliteRow) -> StoreResult<Organization> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let owner_id_str: String = row.try_get("owner_id")?;
    let limits_json_str: String = row.try_get("limits_json")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let limits_json: serde_json::Value = serde_json::from_str(&limits_json_str)
        .map_err(|e| StoreError::Hash(format!("bad limits_json: {}", e)))?;

    Ok(Organization {
        id: parse_uuid(&id_str)?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        owner_id: parse_uuid(&owner_id_str)?,
        plan: row.try_get("plan")?,
        limits_json,
        stripe_customer_id: row.try_get("stripe_customer_id")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_org_member(row: &sqlx::sqlite::SqliteRow) -> StoreResult<OrgMember> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: String = row.try_get("org_id")?;
    let user_id_str: String = row.try_get("user_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(OrgMember {
        id: parse_uuid(&id_str)?,
        org_id: parse_uuid(&org_id_str)?,
        user_id: parse_uuid(&user_id_str)?,
        role: row.try_get("role")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_org_setting(row: &sqlx::sqlite::SqliteRow) -> StoreResult<OrgSetting> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: String = row.try_get("org_id")?;
    let setting_value_str: String = row.try_get("setting_value")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let setting_value: serde_json::Value = serde_json::from_str(&setting_value_str)
        .map_err(|e| StoreError::Hash(format!("bad setting_value: {}", e)))?;

    Ok(OrgSetting {
        id: parse_uuid(&id_str)?,
        org_id: parse_uuid(&org_id_str)?,
        setting_key: row.try_get("setting_key")?,
        setting_value,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_audit_log(row: &sqlx::sqlite::SqliteRow) -> StoreResult<AuditLog> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: String = row.try_get("org_id")?;
    let user_id_str: Option<String> = row.try_get("user_id")?;
    let event_type_str: String = row.try_get("event_type")?;
    let metadata_str: Option<String> = row.try_get("metadata")?;
    let created_at_str: String = row.try_get("created_at")?;

    let event_type: AuditEventType =
        serde_json::from_value(serde_json::Value::String(event_type_str.clone()))
            .map_err(|e| StoreError::Hash(format!("bad event_type '{}': {}", event_type_str, e)))?;
    let metadata: Option<serde_json::Value> = metadata_str
        .as_deref()
        .map(|s| {
            serde_json::from_str(s).map_err(|e| StoreError::Hash(format!("bad metadata: {}", e)))
        })
        .transpose()?;

    Ok(AuditLog {
        id: parse_uuid(&id_str)?,
        org_id: parse_uuid(&org_id_str)?,
        user_id: user_id_str.as_deref().map(parse_uuid).transpose()?,
        event_type,
        description: row.try_get("description")?,
        metadata,
        ip_address: row.try_get("ip_address")?,
        user_agent: row.try_get("user_agent")?,
        created_at: parse_dt(&created_at_str)?,
    })
}

fn row_to_verification_token(row: &sqlx::sqlite::SqliteRow) -> StoreResult<VerificationToken> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let user_id_str: String = row.try_get("user_id")?;
    let expires_at_str: String = row.try_get("expires_at")?;
    let used_at_str: Option<String> = row.try_get("used_at")?;
    let created_at_str: String = row.try_get("created_at")?;

    Ok(VerificationToken {
        id: parse_uuid(&id_str)?,
        user_id: parse_uuid(&user_id_str)?,
        token: row.try_get("token")?,
        expires_at: parse_dt(&expires_at_str)?,
        used_at: used_at_str.as_deref().map(parse_dt).transpose()?,
        created_at: parse_dt(&created_at_str)?,
    })
}

fn audit_event_type_to_str(et: &AuditEventType) -> String {
    // AuditEventType derives Serialize with `rename_all = "snake_case"`, so
    // serde_json::to_value yields a JSON string like "member_added".
    serde_json::to_value(et)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> StoreResult<User> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let two_factor_verified_at_str: Option<String> = row.try_get("two_factor_verified_at")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(User {
        id: parse_uuid(&id_str)?,
        username: row.try_get("username")?,
        email: row.try_get("email")?,
        password_hash: row.try_get("password_hash")?,
        api_token: row.try_get("api_token")?,
        is_verified: row.try_get("is_verified")?,
        is_admin: row.try_get("is_admin")?,
        two_factor_enabled: row.try_get("two_factor_enabled")?,
        two_factor_secret: row.try_get("two_factor_secret")?,
        // 2FA backup codes aren't needed by the OSS admin flow; we skip
        // loading them for now. A future commit can JSON-decode the column.
        two_factor_backup_codes: None,
        two_factor_verified_at: two_factor_verified_at_str.as_deref().map(parse_dt).transpose()?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

#[async_trait]
impl RegistryStore for SqliteRegistryStore {
    // Generated by /tmp/gen_sqlite_stub.py — do not edit by hand.
    // Contains 151 method stubs.

    async fn health_check(&self) -> StoreResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn create_api_token(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name: &str,
        scopes: &[TokenScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<(String, ApiToken)> {
        // Generate a 32-byte random token, base64-encode, prefix with `mfx_`.
        use base64::{engine::general_purpose, Engine as _};
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        let full_token = format!("mfx_{}", general_purpose::URL_SAFE_NO_PAD.encode(buf));
        let token_prefix: String = full_token.chars().take(12).collect();
        let hashed_token = bcrypt::hash(&full_token, bcrypt::DEFAULT_COST)
            .map_err(|e| StoreError::Hash(format!("bcrypt: {}", e)))?;

        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let scopes_json: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
        let scopes_json = serde_json::to_string(&scopes_json)
            .map_err(|e| StoreError::Hash(format!("encode scopes: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO api_tokens (
                id, org_id, user_id, name, token_prefix, hashed_token,
                scopes, expires_at, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(user_id.map(|u| u.to_string()))
        .bind(name)
        .bind(&token_prefix)
        .bind(&hashed_token)
        .bind(&scopes_json)
        .bind(expires_at.map(|d| d.to_rfc3339()))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let token = self.find_api_token_by_id(id).await?.ok_or(StoreError::NotFound)?;
        Ok((full_token, token))
    }

    async fn find_api_token_by_id(&self, token_id: Uuid) -> StoreResult<Option<ApiToken>> {
        let row = sqlx::query("SELECT * FROM api_tokens WHERE id = ?")
            .bind(token_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_api_token).transpose()
    }

    async fn list_api_tokens_by_org(&self, org_id: Uuid) -> StoreResult<Vec<ApiToken>> {
        let rows =
            sqlx::query("SELECT * FROM api_tokens WHERE org_id = ? ORDER BY created_at DESC")
                .bind(org_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(row_to_api_token).collect()
    }

    async fn find_api_token_by_prefix(
        &self,
        org_id: Uuid,
        prefix: &str,
    ) -> StoreResult<Option<ApiToken>> {
        let row = sqlx::query("SELECT * FROM api_tokens WHERE org_id = ? AND token_prefix = ?")
            .bind(org_id.to_string())
            .bind(prefix)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_api_token).transpose()
    }

    async fn verify_api_token(&self, token: &str) -> StoreResult<Option<ApiToken>> {
        // Match the prefix, then bcrypt-verify candidates. Matches the
        // Postgres impl's behavior (no expires_at filter — caller checks).
        let token_prefix: String = token.chars().take(12).collect();
        let rows = sqlx::query(
            "SELECT * FROM api_tokens WHERE token_prefix = ? AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(&token_prefix)
        .bind(Utc::now().to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        for row in &rows {
            let candidate = row_to_api_token(row)?;
            if bcrypt::verify(token, &candidate.hashed_token).unwrap_or(false) {
                // Best-effort last_used_at touch — ignore failure.
                let _ = sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
                    .bind(Utc::now().to_rfc3339())
                    .bind(candidate.id.to_string())
                    .execute(&self.pool)
                    .await;
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    async fn delete_api_token(&self, token_id: Uuid) -> StoreResult<()> {
        sqlx::query("DELETE FROM api_tokens WHERE id = ?")
            .bind(token_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn rotate_api_token(
        &self,
        token_id: Uuid,
        new_name: Option<&str>,
        delete_old: bool,
    ) -> StoreResult<(String, ApiToken, Option<ApiToken>)> {
        Err(StoreError::Hash(
            "rotate_api_token: not yet implemented in SQLite backend".into(),
        ))
    }

    #[allow(unused_variables)]
    async fn find_api_tokens_needing_rotation(
        &self,
        org_id: Option<Uuid>,
        days_old: i64,
    ) -> StoreResult<Vec<ApiToken>> {
        Ok(Vec::new())
    }

    async fn get_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<Option<OrgSetting>> {
        let row = sqlx::query("SELECT * FROM org_settings WHERE org_id = ? AND setting_key = ?")
            .bind(org_id.to_string())
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_org_setting).transpose()
    }

    async fn set_org_setting(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> StoreResult<OrgSetting> {
        let value_str = value.to_string();
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO org_settings (id, org_id, setting_key, setting_value, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(org_id, setting_key) DO UPDATE SET
                setting_value = excluded.setting_value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(key)
        .bind(&value_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.get_org_setting(org_id, key).await?.ok_or(StoreError::NotFound)
    }

    async fn delete_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<()> {
        sqlx::query("DELETE FROM org_settings WHERE org_id = ? AND setting_key = ?")
            .bind(org_id.to_string())
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        owner_id: Uuid,
        plan: Plan,
    ) -> StoreResult<Organization> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let plan_str = serde_json::to_value(&plan)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "free".to_string());
        sqlx::query(
            r#"
            INSERT INTO organizations (
                id, name, slug, owner_id, plan, limits_json, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, '{}', ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(name)
        .bind(slug)
        .bind(owner_id.to_string())
        .bind(&plan_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_organization_by_id(id).await?.ok_or(StoreError::NotFound)
    }

    async fn find_organization_by_id(&self, org_id: Uuid) -> StoreResult<Option<Organization>> {
        let row = sqlx::query("SELECT * FROM organizations WHERE id = ?")
            .bind(org_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_organization).transpose()
    }

    async fn find_organization_by_slug(&self, slug: &str) -> StoreResult<Option<Organization>> {
        let row = sqlx::query("SELECT * FROM organizations WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_organization).transpose()
    }

    async fn list_organizations_by_user(&self, user_id: Uuid) -> StoreResult<Vec<Organization>> {
        // Orgs where the user is owner OR a member.
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT o.* FROM organizations o
            LEFT JOIN org_members m ON m.org_id = o.id
            WHERE o.owner_id = ? OR m.user_id = ?
            ORDER BY o.created_at ASC
            "#,
        )
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_organization).collect()
    }

    async fn update_organization_name(&self, org_id: Uuid, name: &str) -> StoreResult<()> {
        sqlx::query("UPDATE organizations SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(Utc::now().to_rfc3339())
            .bind(org_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_organization_slug(&self, org_id: Uuid, slug: &str) -> StoreResult<()> {
        sqlx::query("UPDATE organizations SET slug = ?, updated_at = ? WHERE id = ?")
            .bind(slug)
            .bind(Utc::now().to_rfc3339())
            .bind(org_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_organization_plan(&self, org_id: Uuid, plan: Plan) -> StoreResult<()> {
        let plan_str = serde_json::to_value(&plan)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "free".to_string());
        sqlx::query("UPDATE organizations SET plan = ?, updated_at = ? WHERE id = ?")
            .bind(&plan_str)
            .bind(Utc::now().to_rfc3339())
            .bind(org_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn organization_has_active_subscription(&self, org_id: Uuid) -> StoreResult<bool> {
        // OSS admin has no Stripe integration.
        Ok(false)
    }

    async fn delete_organization(&self, org_id: Uuid) -> StoreResult<()> {
        sqlx::query("DELETE FROM organizations WHERE id = ?")
            .bind(org_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<OrgMember> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let role_str = serde_json::to_value(&role)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "member".to_string());
        sqlx::query(
            "INSERT INTO org_members (id, org_id, user_id, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(user_id.to_string())
        .bind(&role_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.find_org_member(org_id, user_id).await?.ok_or(StoreError::NotFound)
    }

    async fn find_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<Option<OrgMember>> {
        let row = sqlx::query("SELECT * FROM org_members WHERE org_id = ? AND user_id = ?")
            .bind(org_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_org_member).transpose()
    }

    async fn list_org_members(&self, org_id: Uuid) -> StoreResult<Vec<OrgMember>> {
        let rows =
            sqlx::query("SELECT * FROM org_members WHERE org_id = ? ORDER BY created_at ASC")
                .bind(org_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(row_to_org_member).collect()
    }

    async fn update_org_member_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<()> {
        let role_str = serde_json::to_value(&role)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "member".to_string());
        sqlx::query(
            "UPDATE org_members SET role = ?, updated_at = ? WHERE org_id = ? AND user_id = ?",
        )
        .bind(&role_str)
        .bind(Utc::now().to_rfc3339())
        .bind(org_id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<()> {
        sqlx::query("DELETE FROM org_members WHERE org_id = ? AND user_id = ?")
            .bind(org_id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn record_audit_event(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event_type: AuditEventType,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) {
        // Best-effort: never fail the caller on an audit insert error.
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let event_type_str = audit_event_type_to_str(&event_type);
        let metadata_str = metadata.as_ref().map(|v| v.to_string());
        let result = sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, org_id, user_id, event_type, description,
                metadata, ip_address, user_agent, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(user_id.map(|u| u.to_string()))
        .bind(&event_type_str)
        .bind(&description)
        .bind(metadata_str)
        .bind(ip_address)
        .bind(user_agent)
        .bind(&now)
        .execute(&self.pool)
        .await;
        if let Err(e) = result {
            tracing::warn!("failed to record audit event: {}", e);
        }
    }

    async fn list_audit_logs(
        &self,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<Vec<AuditLog>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let rows = if let Some(et) = event_type {
            let et_str = audit_event_type_to_str(&et);
            sqlx::query(
                "SELECT * FROM audit_logs WHERE org_id = ? AND event_type = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(org_id.to_string())
            .bind(&et_str)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT * FROM audit_logs WHERE org_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(org_id.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        rows.iter().map(row_to_audit_log).collect()
    }

    async fn count_audit_logs(
        &self,
        org_id: Uuid,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<i64> {
        use sqlx::Row;
        let row = if let Some(et) = event_type {
            let et_str = audit_event_type_to_str(&et);
            sqlx::query("SELECT COUNT(*) as c FROM audit_logs WHERE org_id = ? AND event_type = ?")
                .bind(org_id.to_string())
                .bind(&et_str)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query("SELECT COUNT(*) as c FROM audit_logs WHERE org_id = ?")
                .bind(org_id.to_string())
                .fetch_one(&self.pool)
                .await?
        };
        Ok(row.try_get::<i64, _>("c")?)
    }

    #[allow(unused_variables)]
    async fn record_feature_usage(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        feature: FeatureType,
        metadata: Option<serde_json::Value>,
    ) {
    }

    #[allow(unused_variables)]
    async fn count_feature_usage_by_org(
        &self,
        org_id: Uuid,
        feature: FeatureType,
        days: i64,
    ) -> StoreResult<i64> {
        Ok(0)
    }

    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> StoreResult<User> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO users (
                id, username, email, password_hash,
                is_verified, is_admin, two_factor_enabled,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, 0, 0, 0, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.find_user_by_id(id).await?.ok_or(StoreError::NotFound)
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> StoreResult<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_user_by_email(&self, email: &str) -> StoreResult<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_user_by_username(&self, username: &str) -> StoreResult<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[allow(unused_variables)]
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> StoreResult<Vec<User>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn set_user_api_token(&self, user_id: Uuid, token: &str) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn enable_user_2fa(
        &self,
        user_id: Uuid,
        secret: &str,
        backup_codes: &[String],
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn disable_user_2fa(&self, user_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn update_user_2fa_verified(&self, user_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn remove_user_backup_code(&self, user_id: Uuid, code_index: usize) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn find_user_by_github_id(&self, github_id: &str) -> StoreResult<Option<User>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn find_user_by_google_id(&self, google_id: &str) -> StoreResult<Option<User>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn link_user_github_account(
        &self,
        user_id: Uuid,
        github_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn link_user_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_oauth_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        auth_provider: &str,
        github_id: Option<&str>,
        google_id: Option<&str>,
        avatar_url: Option<&str>,
    ) -> StoreResult<User> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn get_or_create_personal_org(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> StoreResult<Organization> {
        Err(StoreError::NotFound)
    }

    async fn update_user_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> StoreResult<()> {
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(password_hash)
            .bind(Utc::now().to_rfc3339())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_user_verified(&self, user_id: Uuid) -> StoreResult<()> {
        sqlx::query("UPDATE users SET is_verified = 1, updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_verification_token(&self, user_id: Uuid) -> StoreResult<VerificationToken> {
        use base64::{engine::general_purpose, Engine as _};
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        let token = general_purpose::URL_SAFE_NO_PAD.encode(buf);
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(24);
        sqlx::query(
            "INSERT INTO verification_tokens (id, user_id, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(&token)
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let row = sqlx::query("SELECT * FROM verification_tokens WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await?;
        row_to_verification_token(&row)
    }

    async fn set_verification_token_expiry_hours(
        &self,
        token_id: Uuid,
        hours: i64,
    ) -> StoreResult<()> {
        let new_expiry = Utc::now() + chrono::Duration::hours(hours);
        sqlx::query("UPDATE verification_tokens SET expires_at = ? WHERE id = ?")
            .bind(new_expiry.to_rfc3339())
            .bind(token_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_verification_token_by_token(
        &self,
        token: &str,
    ) -> StoreResult<Option<VerificationToken>> {
        let row = sqlx::query("SELECT * FROM verification_tokens WHERE token = ?")
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_verification_token).transpose()
    }

    async fn mark_verification_token_used(&self, token_id: Uuid) -> StoreResult<()> {
        sqlx::query("UPDATE verification_tokens SET used_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(token_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn record_suspicious_activity(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        activity_type: SuspiciousActivityType,
        severity: &str,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) {
    }

    #[allow(unused_variables)]
    async fn create_federation(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> StoreResult<Federation> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_federation_by_id(&self, id: Uuid) -> StoreResult<Option<Federation>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_federations_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Federation>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_federation(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> StoreResult<Option<Federation>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn delete_federation(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn list_unresolved_suspicious_activities(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> StoreResult<Vec<SuspiciousActivity>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_unresolved_suspicious_activities(&self, org_id: Uuid) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn resolve_suspicious_activity(
        &self,
        activity_id: Uuid,
        resolved_by: Uuid,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_cloud_workspace(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
    ) -> StoreResult<CloudWorkspace> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_cloud_workspace_by_id(&self, id: Uuid) -> StoreResult<Option<CloudWorkspace>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_cloud_workspaces_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudWorkspace>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_cloud_workspace(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudWorkspace>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn delete_cloud_workspace(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_cloud_service(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        base_url: &str,
    ) -> StoreResult<CloudService> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_cloud_service_by_id(&self, id: Uuid) -> StoreResult<Option<CloudService>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_cloud_services_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudService>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_cloud_service(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        base_url: Option<&str>,
        enabled: Option<bool>,
        tags: Option<&serde_json::Value>,
        routes: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudService>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn delete_cloud_service(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_cloud_fixture(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        path: &str,
        method: &str,
        content: Option<&serde_json::Value>,
    ) -> StoreResult<CloudFixture> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_cloud_fixture_by_id(&self, id: Uuid) -> StoreResult<Option<CloudFixture>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_cloud_fixtures_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudFixture>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_cloud_fixture(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        path: Option<&str>,
        method: Option<&str>,
        content: Option<&serde_json::Value>,
        tags: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudFixture>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn delete_cloud_fixture(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_hosted_mock(
        &self,
        org_id: Uuid,
        project_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: Option<&str>,
        config_json: serde_json::Value,
        openapi_spec_url: Option<&str>,
        region: Option<&str>,
    ) -> StoreResult<HostedMock> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_hosted_mock_by_id(&self, id: Uuid) -> StoreResult<Option<HostedMock>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn find_hosted_mock_by_slug(
        &self,
        org_id: Uuid,
        slug: &str,
    ) -> StoreResult<Option<HostedMock>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_hosted_mocks_by_org(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_hosted_mock_status(
        &self,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn update_hosted_mock_urls(
        &self,
        id: Uuid,
        deployment_url: Option<&str>,
        internal_url: Option<&str>,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn update_hosted_mock_health(
        &self,
        id: Uuid,
        health_status: HealthStatus,
        health_check_url: Option<&str>,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn delete_hosted_mock(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn subscribe_waitlist(
        &self,
        email: &str,
        source: &str,
    ) -> StoreResult<WaitlistSubscriber> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn unsubscribe_waitlist_by_token(&self, token: Uuid) -> StoreResult<bool> {
        Ok(false)
    }

    #[allow(unused_variables)]
    async fn get_or_create_current_usage_counter(&self, org_id: Uuid) -> StoreResult<UsageCounter> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn list_usage_counters_by_org(&self, org_id: Uuid) -> StoreResult<Vec<UsageCounter>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn find_sso_config_by_org(&self, org_id: Uuid) -> StoreResult<Option<SSOConfiguration>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn upsert_sso_config(
        &self,
        org_id: Uuid,
        provider: SSOProvider,
        saml_entity_id: Option<&str>,
        saml_sso_url: Option<&str>,
        saml_slo_url: Option<&str>,
        saml_x509_cert: Option<&str>,
        saml_name_id_format: Option<&str>,
        attribute_mapping: Option<serde_json::Value>,
        require_signed_assertions: bool,
        require_signed_responses: bool,
        allow_unsolicited_responses: bool,
    ) -> StoreResult<SSOConfiguration> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn enable_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn disable_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn delete_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn is_saml_assertion_used(&self, assertion_id: &str, org_id: Uuid) -> StoreResult<bool> {
        Ok(false)
    }

    #[allow(unused_variables)]
    async fn record_saml_assertion_used(
        &self,
        assertion_id: &str,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name_id: Option<&str>,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> StoreResult<SAMLAssertionId> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn create_org_template(
        &self,
        org_id: Uuid,
        name: &str,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        created_by: Uuid,
        is_default: bool,
    ) -> StoreResult<OrgTemplate> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_org_template_by_id(&self, id: Uuid) -> StoreResult<Option<OrgTemplate>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_org_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<OrgTemplate>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn update_org_template(
        &self,
        template: &OrgTemplate,
        name: Option<&str>,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        is_default: Option<bool>,
    ) -> StoreResult<OrgTemplate> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn delete_org_template(&self, id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn create_template(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        version: &str,
        category: TemplateCategory,
        content_json: serde_json::Value,
    ) -> StoreResult<Template> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_template_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> StoreResult<Option<Template>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Template>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Template>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn create_scenario(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        current_version: &str,
        category: &str,
        license: &str,
        manifest_json: serde_json::Value,
    ) -> StoreResult<Scenario> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn find_scenario_by_name(&self, name: &str) -> StoreResult<Option<Scenario>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn list_scenarios_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Scenario>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Scenario>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Plugin>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
    ) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn find_plugin_by_name(&self, name: &str) -> StoreResult<Option<Plugin>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn get_plugin_tags(&self, plugin_id: Uuid) -> StoreResult<Vec<String>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn create_plugin(
        &self,
        name: &str,
        description: &str,
        version: &str,
        category: &str,
        license: &str,
        repository: Option<&str>,
        homepage: Option<&str>,
        author_id: Uuid,
    ) -> StoreResult<Plugin> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn list_plugin_versions(&self, plugin_id: Uuid) -> StoreResult<Vec<PluginVersion>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn find_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
    ) -> StoreResult<Option<PluginVersion>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn create_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
        download_url: &str,
        checksum: &str,
        file_size: i64,
        min_mockforge_version: Option<&str>,
    ) -> StoreResult<PluginVersion> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn yank_plugin_version(&self, version_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn get_plugin_version_dependencies(
        &self,
        version_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<String, String>> {
        Ok(std::collections::HashMap::new())
    }

    #[allow(unused_variables)]
    async fn add_plugin_version_dependency(
        &self,
        version_id: Uuid,
        plugin_name: &str,
        version_req: &str,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn get_plugin_reviews(
        &self,
        plugin_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Review>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_plugin_reviews(&self, plugin_id: Uuid) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn create_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
        version: &str,
        rating: i16,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<Review> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn get_plugin_review_stats(&self, plugin_id: Uuid) -> StoreResult<(f64, i64)> {
        Ok((0.0, 0))
    }

    #[allow(unused_variables)]
    async fn get_plugin_review_distribution(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<i16, i64>> {
        Ok(std::collections::HashMap::new())
    }

    #[allow(unused_variables)]
    async fn find_existing_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn update_plugin_rating_stats(
        &self,
        plugin_id: Uuid,
        avg: f64,
        count: i32,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn increment_plugin_review_vote(
        &self,
        plugin_id: Uuid,
        review_id: Uuid,
        helpful: bool,
    ) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn get_user_public_info(&self, user_id: Uuid) -> StoreResult<Option<(String, String)>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn get_template_reviews(
        &self,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<TemplateReview>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_template_reviews(&self, template_id: Uuid) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn create_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<TemplateReview> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn update_template_review_stats(&self, template_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn find_existing_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn get_scenario_reviews(
        &self,
        scenario_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<ScenarioReview>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn count_scenario_reviews(&self, scenario_id: Uuid) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn create_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<ScenarioReview> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn update_scenario_review_stats(&self, scenario_id: Uuid) -> StoreResult<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn find_existing_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    async fn get_admin_analytics_snapshot(&self) -> StoreResult<AdminAnalyticsSnapshot> {
        Ok(AdminAnalyticsSnapshot {
            total_users: 0,
            verified_users: 0,
            auth_providers: Vec::new(),
            new_users_7d: 0,
            new_users_30d: 0,
            total_orgs: 0,
            plan_distribution: Vec::new(),
            active_subs: 0,
            trial_orgs: 0,
            total_requests: None,
            total_storage: None,
            total_ai_tokens: None,
            top_orgs: Vec::new(),
            hosted_mocks_count: 0,
            hosted_mocks_orgs: 0,
            hosted_mocks_30d: 0,
            plugins_count: 0,
            plugins_orgs: 0,
            plugins_30d: 0,
            templates_count: 0,
            templates_orgs: 0,
            templates_30d: 0,
            scenarios_count: 0,
            scenarios_orgs: 0,
            scenarios_30d: 0,
            api_tokens_count: 0,
            api_tokens_orgs: 0,
            api_tokens_30d: 0,
            user_growth_30d: Vec::new(),
            org_growth_30d: Vec::new(),
            logins_24h: 0,
            logins_7d: 0,
            api_requests_24h: 0,
            api_requests_7d: 0,
        })
    }

    #[allow(unused_variables)]
    async fn get_conversion_funnel_snapshot(
        &self,
        interval: &str,
    ) -> StoreResult<ConversionFunnelSnapshot> {
        Ok(ConversionFunnelSnapshot {
            signups: 0,
            verified: 0,
            logged_in: 0,
            org_created: 0,
            feature_users: 0,
            checkout_initiated: 0,
            paid_subscribers: 0,
            time_to_convert_days: None,
        })
    }

    #[allow(unused_variables)]
    async fn list_user_settings_raw(&self, user_id: Uuid) -> StoreResult<Vec<UserSettingRow>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn list_user_api_tokens(&self, user_id: Uuid) -> StoreResult<Vec<ApiToken>> {
        Ok(Vec::new())
    }

    async fn get_org_membership_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<String>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT role FROM org_members WHERE org_id = ? AND user_id = ?")
            .bind(org_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(|r| r.try_get::<String, _>("role")).transpose()?)
    }

    #[allow(unused_variables)]
    async fn list_org_settings_raw(&self, org_id: Uuid) -> StoreResult<Vec<OrgSettingRow>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn list_org_projects_raw(&self, org_id: Uuid) -> StoreResult<Vec<ProjectRow>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn list_org_subscriptions_raw(&self, org_id: Uuid) -> StoreResult<Vec<SubscriptionRow>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn list_org_hosted_mocks_raw(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>> {
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
    async fn delete_user_data_cascade(&self, user_id: Uuid) -> StoreResult<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open an in-memory SQLite database, run migrations, and return the
    /// resulting store. Shared by every test below.
    async fn memory_store() -> SqliteRegistryStore {
        SqliteRegistryStore::connect("sqlite::memory:")
            .await
            .expect("connect and migrate in-memory sqlite")
    }

    #[tokio::test]
    async fn test_connect_and_migrate_in_memory() {
        let store = memory_store().await;
        // Health check should succeed against a fresh in-memory database.
        store.health_check().await.expect("health_check");
    }

    #[tokio::test]
    async fn test_migrations_create_core_tables() {
        let store = memory_store().await;
        // Every table we care about for the OSS admin should exist. We do
        // a cheap `SELECT COUNT(*)` against each — a missing table surfaces
        // as a sqlx error and fails the test.
        for table in [
            "users",
            "organizations",
            "org_members",
            "api_tokens",
            "user_settings",
            "org_settings",
            "audit_logs",
            "token_revocations",
            "verification_tokens",
            "login_attempts",
        ] {
            let query = format!("SELECT COUNT(*) FROM {}", table);
            sqlx::query(&query)
                .fetch_one(store.pool())
                .await
                .unwrap_or_else(|e| panic!("table `{}` missing or broken: {}", table, e));
        }
    }

    #[tokio::test]
    async fn test_empty_store_returns_expected_defaults() {
        let store = memory_store().await;
        let fake_user = Uuid::new_v4();
        let fake_org = Uuid::new_v4();

        // Lookups against an empty store return None, never an error.
        assert!(store.find_user_by_id(fake_user).await.unwrap().is_none());
        assert!(store.find_user_by_email("nobody@example.com").await.unwrap().is_none());
        assert!(store.find_organization_by_id(fake_org).await.unwrap().is_none());
        assert!(store.find_organization_by_slug("nope").await.unwrap().is_none());

        // List endpoints return empty vectors.
        assert!(store.list_api_tokens_by_org(fake_org).await.unwrap().is_empty());
        assert!(store.list_org_members(fake_org).await.unwrap().is_empty());
        assert!(store.list_organizations_by_user(fake_user).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_admin_analytics_snapshot_is_zeroed() {
        let store = memory_store().await;
        let snap = store.get_admin_analytics_snapshot().await.unwrap();
        assert_eq!(snap.total_users, 0);
        assert_eq!(snap.total_orgs, 0);
        assert_eq!(snap.plugins_count, 0);
        assert!(snap.user_growth_30d.is_empty());
    }

    #[tokio::test]
    async fn test_conversion_funnel_snapshot_is_zeroed() {
        let store = memory_store().await;
        let funnel = store.get_conversion_funnel_snapshot("30 days").await.unwrap();
        assert_eq!(funnel.signups, 0);
        assert_eq!(funnel.paid_subscribers, 0);
        assert!(funnel.time_to_convert_days.is_none());
    }

    #[tokio::test]
    async fn test_create_and_find_user_roundtrip() {
        let store = memory_store().await;
        let created = store
            .create_user("alice", "alice@example.com", "bcrypt_hash_placeholder")
            .await
            .expect("create_user");

        assert_eq!(created.username, "alice");
        assert_eq!(created.email, "alice@example.com");
        assert_eq!(created.password_hash, "bcrypt_hash_placeholder");
        assert!(!created.is_verified);
        assert!(!created.is_admin);
        assert!(!created.two_factor_enabled);
        assert!(created.two_factor_backup_codes.is_none());

        let by_id = store.find_user_by_id(created.id).await.unwrap().expect("by id");
        assert_eq!(by_id.id, created.id);
        assert_eq!(by_id.email, "alice@example.com");

        let by_email =
            store.find_user_by_email("alice@example.com").await.unwrap().expect("by email");
        assert_eq!(by_email.id, created.id);

        let by_username = store.find_user_by_username("alice").await.unwrap().expect("by username");
        assert_eq!(by_username.id, created.id);

        assert!(store.find_user_by_email("missing@example.com").await.unwrap().is_none());
        assert!(store.find_user_by_username("nobody").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_health_check_pings_database() {
        let store = memory_store().await;
        store.health_check().await.expect("health_check ping");
    }

    #[tokio::test]
    async fn test_create_and_find_organization_roundtrip() {
        let store = memory_store().await;
        let owner = store.create_user("bob", "bob@example.com", "hash").await.unwrap();

        let org = store
            .create_organization("Bob's Org", "bobs-org", owner.id, Plan::Free)
            .await
            .unwrap();
        assert_eq!(org.name, "Bob's Org");
        assert_eq!(org.slug, "bobs-org");
        assert_eq!(org.owner_id, owner.id);
        assert_eq!(org.plan, "free");

        // Find by id + slug
        let by_id = store.find_organization_by_id(org.id).await.unwrap().expect("by id");
        assert_eq!(by_id.id, org.id);
        let by_slug = store.find_organization_by_slug("bobs-org").await.unwrap().expect("by slug");
        assert_eq!(by_slug.id, org.id);

        // list_organizations_by_user finds the org via owner_id
        let mine = store.list_organizations_by_user(owner.id).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].id, org.id);

        // Update plan
        store.update_organization_plan(org.id, Plan::Pro).await.unwrap();
        let reloaded = store.find_organization_by_id(org.id).await.unwrap().unwrap();
        assert_eq!(reloaded.plan, "pro");

        // Delete
        store.delete_organization(org.id).await.unwrap();
        assert!(store.find_organization_by_id(org.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_org_member_crud() {
        let store = memory_store().await;
        let owner = store.create_user("carol", "carol@example.com", "hash").await.unwrap();
        let org = store
            .create_organization("Carol Org", "carol-org", owner.id, Plan::Free)
            .await
            .unwrap();
        let member_user = store.create_user("dave", "dave@example.com", "hash").await.unwrap();

        // Create member
        let member =
            store.create_org_member(org.id, member_user.id, OrgRole::Member).await.unwrap();
        assert_eq!(member.org_id, org.id);
        assert_eq!(member.user_id, member_user.id);
        assert_eq!(member.role, "member");

        // Find
        let found = store.find_org_member(org.id, member_user.id).await.unwrap().expect("found");
        assert_eq!(found.id, member.id);

        // get_org_membership_role
        let role = store.get_org_membership_role(org.id, member_user.id).await.unwrap();
        assert_eq!(role, Some("member".to_string()));

        // List
        let members = store.list_org_members(org.id).await.unwrap();
        assert_eq!(members.len(), 1);

        // Update role
        store
            .update_org_member_role(org.id, member_user.id, OrgRole::Admin)
            .await
            .unwrap();
        let updated = store.find_org_member(org.id, member_user.id).await.unwrap().unwrap();
        assert_eq!(updated.role, "admin");

        // Delete
        store.delete_org_member(org.id, member_user.id).await.unwrap();
        assert!(store.find_org_member(org.id, member_user.id).await.unwrap().is_none());
        assert!(store.list_org_members(org.id).await.unwrap().is_empty());

        // list_organizations_by_user still finds the org through member path
        // (removed above, so now empty for member_user but populated for owner)
        assert!(store.list_organizations_by_user(member_user.id).await.unwrap().is_empty());
        assert_eq!(store.list_organizations_by_user(owner.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_org_setting_upsert() {
        let store = memory_store().await;
        let owner = store.create_user("eve", "eve@example.com", "hash").await.unwrap();
        let org = store
            .create_organization("Eve Org", "eve-org", owner.id, Plan::Free)
            .await
            .unwrap();

        // Missing key returns None
        assert!(store.get_org_setting(org.id, "retention_days").await.unwrap().is_none());

        // Set a value
        let v1 = serde_json::json!({"days": 30});
        let s1 = store.set_org_setting(org.id, "retention_days", v1.clone()).await.unwrap();
        assert_eq!(s1.setting_value, v1);

        // Update the same key — should upsert
        let v2 = serde_json::json!({"days": 60});
        let s2 = store.set_org_setting(org.id, "retention_days", v2.clone()).await.unwrap();
        assert_eq!(s2.id, s1.id, "upsert should preserve id");
        assert_eq!(s2.setting_value, v2);

        // Read it back
        let got = store.get_org_setting(org.id, "retention_days").await.unwrap().unwrap();
        assert_eq!(got.setting_value, v2);

        // Delete
        store.delete_org_setting(org.id, "retention_days").await.unwrap();
        assert!(store.get_org_setting(org.id, "retention_days").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_audit_event_roundtrip() {
        let store = memory_store().await;
        let owner = store.create_user("frank", "frank@example.com", "hash").await.unwrap();
        let org = store
            .create_organization("Frank Org", "frank-org", owner.id, Plan::Free)
            .await
            .unwrap();

        // Record two events
        store
            .record_audit_event(
                org.id,
                Some(owner.id),
                AuditEventType::OrgCreated,
                "bootstrap".to_string(),
                Some(serde_json::json!({"source": "test"})),
                Some("127.0.0.1"),
                Some("test/1.0"),
            )
            .await;

        store
            .record_audit_event(
                org.id,
                Some(owner.id),
                AuditEventType::ApiTokenCreated,
                "created ci-token".to_string(),
                None,
                None,
                None,
            )
            .await;

        // Count and list
        assert_eq!(store.count_audit_logs(org.id, None).await.unwrap(), 2);
        assert_eq!(
            store.count_audit_logs(org.id, Some(AuditEventType::OrgCreated)).await.unwrap(),
            1
        );

        let logs = store.list_audit_logs(org.id, None, None, None).await.unwrap();
        assert_eq!(logs.len(), 2);
        // most recent first
        assert_eq!(logs[0].event_type, AuditEventType::ApiTokenCreated);
        assert_eq!(logs[1].event_type, AuditEventType::OrgCreated);
        assert_eq!(logs[1].ip_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(logs[1].metadata.as_ref().unwrap(), &serde_json::json!({"source": "test"}));

        // Filtered list
        let org_created = store
            .list_audit_logs(org.id, None, None, Some(AuditEventType::OrgCreated))
            .await
            .unwrap();
        assert_eq!(org_created.len(), 1);
        assert_eq!(org_created[0].event_type, AuditEventType::OrgCreated);
    }

    #[tokio::test]
    async fn test_user_update_flows() {
        let store = memory_store().await;
        let user = store.create_user("gina", "gina@example.com", "hash_v1").await.unwrap();
        assert!(!user.is_verified);

        // mark_user_verified
        store.mark_user_verified(user.id).await.unwrap();
        let reloaded = store.find_user_by_id(user.id).await.unwrap().unwrap();
        assert!(reloaded.is_verified);

        // update_user_password_hash
        store.update_user_password_hash(user.id, "hash_v2").await.unwrap();
        let reloaded2 = store.find_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(reloaded2.password_hash, "hash_v2");
    }

    #[tokio::test]
    async fn test_verification_token_lifecycle() {
        let store = memory_store().await;
        let user = store.create_user("harry", "harry@example.com", "hash").await.unwrap();

        let vt = store.create_verification_token(user.id).await.unwrap();
        assert_eq!(vt.user_id, user.id);
        assert!(vt.used_at.is_none());
        assert!(!vt.token.is_empty());
        assert!(vt.expires_at > Utc::now());

        // Find by token
        let found = store
            .find_verification_token_by_token(&vt.token)
            .await
            .unwrap()
            .expect("should find");
        assert_eq!(found.id, vt.id);

        // Mark used
        store.mark_verification_token_used(vt.id).await.unwrap();
        let used = store.find_verification_token_by_token(&vt.token).await.unwrap().unwrap();
        assert!(used.used_at.is_some());

        // Extend expiry
        store.set_verification_token_expiry_hours(vt.id, 72).await.unwrap();
        let extended = store.find_verification_token_by_token(&vt.token).await.unwrap().unwrap();
        assert!(extended.expires_at > vt.expires_at);
    }

    /// Insert a minimal `organizations` row so tests can satisfy the FK
    /// constraint on api_tokens.org_id without dragging in the full
    /// create_organization / create_user plumbing.
    async fn seed_org(store: &SqliteRegistryStore, user_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        // FK on owner_id -> users.id, so seed the user too.
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, is_verified, is_admin, two_factor_enabled, created_at, updated_at) VALUES (?, ?, ?, ?, 0, 0, 0, ?, ?)",
        )
        .bind(user_id.to_string())
        .bind(format!("u-{}", user_id))
        .bind(format!("u-{}@example.com", user_id))
        .bind("hash")
        .bind(&now)
        .bind(&now)
        .execute(store.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO organizations (id, name, slug, owner_id, plan, limits_json, created_at, updated_at) VALUES (?, ?, ?, ?, 'free', '{}', ?, ?)",
        )
        .bind(id.to_string())
        .bind("Test Org")
        .bind(format!("test-{}", id))
        .bind(user_id.to_string())
        .bind(&now)
        .bind(&now)
        .execute(store.pool())
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn test_create_and_verify_api_token_roundtrip() {
        let store = memory_store().await;
        let user_id_value = Uuid::new_v4();
        let org_id = seed_org(&store, user_id_value).await;
        let user_id = Some(user_id_value);
        let scopes = vec![TokenScope::ReadPackages, TokenScope::PublishPackages];

        let (plaintext, created) = store
            .create_api_token(org_id, user_id, "ci-token", &scopes, None)
            .await
            .expect("create_api_token");

        assert!(plaintext.starts_with("mfx_"));
        assert_eq!(created.token_prefix, plaintext.chars().take(12).collect::<String>());
        assert_eq!(created.org_id, org_id);
        assert_eq!(created.user_id, user_id);
        assert_eq!(created.scopes.len(), 2);
        assert!(created.scopes.contains(&"read:packages".to_string()));
        assert!(created.scopes.contains(&"publish:packages".to_string()));
        assert!(created.has_scope(&TokenScope::ReadPackages));
        assert!(!created.has_scope(&TokenScope::AdminOrg));

        // find_api_token_by_id round trip
        let by_id = store.find_api_token_by_id(created.id).await.unwrap().expect("by id");
        assert_eq!(by_id.id, created.id);
        assert_eq!(by_id.hashed_token, created.hashed_token);

        // list_api_tokens_by_org finds it
        let listed = store.list_api_tokens_by_org(org_id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        // find_api_token_by_prefix
        let by_prefix = store
            .find_api_token_by_prefix(org_id, &created.token_prefix)
            .await
            .unwrap()
            .expect("by prefix");
        assert_eq!(by_prefix.id, created.id);

        // verify_api_token with the plaintext returns Some
        let verified = store.verify_api_token(&plaintext).await.unwrap().expect("verified");
        assert_eq!(verified.id, created.id);

        // verify_api_token with a bogus token returns None
        let bogus = store.verify_api_token("mfx_nope_nope_nope_nope").await.unwrap();
        assert!(bogus.is_none());

        // delete_api_token removes it
        store.delete_api_token(created.id).await.unwrap();
        assert!(store.find_api_token_by_id(created.id).await.unwrap().is_none());
        assert!(store.list_api_tokens_by_org(org_id).await.unwrap().is_empty());
    }
}

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

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

use super::{
    AdminAnalyticsSnapshot, ConversionFunnelSnapshot, OrgSettingRow, ProjectRow, RegistryStore,
    SubscriptionRow, UserSettingRow,
};
use crate::error::{StoreError, StoreResult};
use crate::models::api_token::{ApiToken, TokenScope};
use crate::models::attestation::UserPublicKey;
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
use crate::models::osv::{OsvImportRecord, OsvMatch};
use crate::models::plugin::{PendingScanJob, Plugin, PluginSecurityScan, PluginVersion};
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
    /// Accepts these URL forms:
    ///   * `sqlite::memory:`               — in-process only
    ///   * `sqlite://./mockforge.db`       — relative path (two slashes)
    ///   * `sqlite:///var/lib/forge.db`    — absolute path (three slashes)
    ///   * `/absolute/path/mockforge.db`   — bare absolute path
    ///   * `./relative/mockforge.db`       — bare relative path
    ///
    /// Always sets `create_if_missing(true)` so a fresh container with an
    /// empty volume mount bootstraps its own database file instead of
    /// erroring with SQLITE_CANTOPEN.
    pub async fn connect(database_url: &str) -> StoreResult<Self> {
        // Explicitly build connect options so we can set
        // create_if_missing — SqlitePoolOptions::connect(url) alone
        // defaults to create_if_missing(false), which is what we hit on
        // the first Fly deploy (empty /data volume → SQLITE_CANTOPEN).
        let opts: SqliteConnectOptions = database_url
            .parse::<SqliteConnectOptions>()
            .map_err(|e| StoreError::Hash(format!("parse sqlite url '{}': {}", database_url, e)))?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await?;
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

fn row_to_template(row: &sqlx::sqlite::SqliteRow) -> StoreResult<Template> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: Option<String> = row.try_get("org_id")?;
    let author_id_str: String = row.try_get("author_id")?;
    let tags_json: String = row.try_get("tags")?;
    let content_json_str: String = row.try_get("content_json")?;
    let requirements_json: String = row.try_get("requirements")?;
    let compat_json_str: String = row.try_get("compatibility_json")?;
    let stats_json_str: String = row.try_get("stats_json")?;
    let verified_at_str: Option<String> = row.try_get("verified_at")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| StoreError::Hash(format!("bad template tags: {}", e)))?;
    let requirements: Vec<String> = serde_json::from_str(&requirements_json)
        .map_err(|e| StoreError::Hash(format!("bad template requirements: {}", e)))?;
    let content_json: serde_json::Value = serde_json::from_str(&content_json_str)
        .map_err(|e| StoreError::Hash(format!("bad template content_json: {}", e)))?;
    let compatibility_json: serde_json::Value = serde_json::from_str(&compat_json_str)
        .map_err(|e| StoreError::Hash(format!("bad template compatibility_json: {}", e)))?;
    let stats_json: serde_json::Value = serde_json::from_str(&stats_json_str)
        .map_err(|e| StoreError::Hash(format!("bad template stats_json: {}", e)))?;

    Ok(Template {
        id: parse_uuid(&id_str)?,
        org_id: org_id_str.as_deref().map(parse_uuid).transpose()?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        description: row.try_get("description")?,
        author_id: parse_uuid(&author_id_str)?,
        version: row.try_get("version")?,
        category: row.try_get("category")?,
        tags,
        content_json,
        readme: row.try_get("readme")?,
        example_usage: row.try_get("example_usage")?,
        requirements,
        compatibility_json,
        stats_json,
        published: row.try_get("published")?,
        verified_at: verified_at_str.as_deref().map(parse_dt).transpose()?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_federation(row: &sqlx::sqlite::SqliteRow) -> StoreResult<Federation> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: String = row.try_get("org_id")?;
    let created_by_str: String = row.try_get("created_by")?;
    let services_str: String = row.try_get("services")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;
    let services: serde_json::Value = serde_json::from_str(&services_str)
        .map_err(|e| StoreError::Hash(format!("bad federation services: {}", e)))?;

    Ok(Federation {
        id: parse_uuid(&id_str)?,
        org_id: parse_uuid(&org_id_str)?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        services,
        created_by: parse_uuid(&created_by_str)?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_scenario_review(row: &sqlx::sqlite::SqliteRow) -> StoreResult<ScenarioReview> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let scenario_id_str: String = row.try_get("scenario_id")?;
    let reviewer_id_str: String = row.try_get("reviewer_id")?;
    let rating_i64: i64 = row.try_get("rating")?;
    let helpful_i64: i64 = row.try_get("helpful_count")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(ScenarioReview {
        id: parse_uuid(&id_str)?,
        scenario_id: parse_uuid(&scenario_id_str)?,
        reviewer_id: parse_uuid(&reviewer_id_str)?,
        rating: rating_i64 as i32,
        title: row.try_get("title")?,
        comment: row.try_get("comment")?,
        helpful_count: helpful_i64 as i32,
        verified_purchase: row.try_get("verified_purchase")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_template_review(row: &sqlx::sqlite::SqliteRow) -> StoreResult<TemplateReview> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let template_id_str: String = row.try_get("template_id")?;
    let reviewer_id_str: String = row.try_get("reviewer_id")?;
    let rating_i64: i64 = row.try_get("rating")?;
    let helpful_i64: i64 = row.try_get("helpful_count")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(TemplateReview {
        id: parse_uuid(&id_str)?,
        template_id: parse_uuid(&template_id_str)?,
        reviewer_id: parse_uuid(&reviewer_id_str)?,
        rating: rating_i64 as i32,
        title: row.try_get("title")?,
        comment: row.try_get("comment")?,
        helpful_count: helpful_i64 as i32,
        verified_use: row.try_get("verified_use")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

fn row_to_scenario(row: &sqlx::sqlite::SqliteRow) -> StoreResult<Scenario> {
    use sqlx::Row;
    let id_str: String = row.try_get("id")?;
    let org_id_str: Option<String> = row.try_get("org_id")?;
    let author_id_str: String = row.try_get("author_id")?;
    let tags_json: String = row.try_get("tags")?;
    let manifest_str: String = row.try_get("manifest_json")?;
    let verified_at_str: Option<String> = row.try_get("verified_at")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| StoreError::Hash(format!("bad scenario tags: {}", e)))?;
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest_str)
        .map_err(|e| StoreError::Hash(format!("bad scenario manifest: {}", e)))?;
    let rating_f64: f64 = row.try_get("rating_avg")?;
    let rating_count_i64: i64 = row.try_get("rating_count")?;
    let downloads_total_i64: i64 = row.try_get("downloads_total")?;

    Ok(Scenario {
        id: parse_uuid(&id_str)?,
        org_id: org_id_str.as_deref().map(parse_uuid).transpose()?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        description: row.try_get("description")?,
        author_id: parse_uuid(&author_id_str)?,
        current_version: row.try_get("current_version")?,
        category: row.try_get("category")?,
        tags,
        license: row.try_get("license")?,
        repository: row.try_get("repository")?,
        homepage: row.try_get("homepage")?,
        manifest_json,
        downloads_total: downloads_total_i64,
        rating_avg: rust_decimal::Decimal::from_f64_retain(rating_f64).unwrap_or_default(),
        rating_count: rating_count_i64 as i32,
        verified_at: verified_at_str.as_deref().map(parse_dt).transpose()?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
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
        let plan_str = serde_json::to_value(plan)
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
        let plan_str = serde_json::to_value(plan)
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
        let role_str = serde_json::to_value(role)
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
        let role_str = serde_json::to_value(role)
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

    async fn create_federation(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> StoreResult<Federation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let services_str = serde_json::to_string(services)
            .map_err(|e| StoreError::Hash(format!("encode services: {}", e)))?;
        sqlx::query(
            r#"
            INSERT INTO federations
                (id, org_id, name, description, services, created_by,
                 created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(name)
        .bind(description)
        .bind(&services_str)
        .bind(created_by.to_string())
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        Ok(Federation {
            id,
            org_id,
            name: name.to_string(),
            description: description.to_string(),
            services: services.clone(),
            created_by,
            created_at: now,
            updated_at: now,
        })
    }

    async fn find_federation_by_id(&self, id: Uuid) -> StoreResult<Option<Federation>> {
        let row = sqlx::query("SELECT * FROM federations WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(row_to_federation(&r)?)),
            None => Ok(None),
        }
    }

    async fn list_federations_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Federation>> {
        let rows =
            sqlx::query("SELECT * FROM federations WHERE org_id = ? ORDER BY created_at DESC")
                .bind(org_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.iter().map(row_to_federation).collect()
    }

    async fn update_federation(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> StoreResult<Option<Federation>> {
        // SQLite doesn't support Postgres's COALESCE-against-bind pattern
        // as cleanly, so we compose the UPDATE with only the columns the
        // caller supplied. Keeps the SQL obvious and avoids binding NULL
        // for "leave alone" semantics.
        let mut sets: Vec<&str> = Vec::new();
        if name.is_some() {
            sets.push("name = ?");
        }
        if description.is_some() {
            sets.push("description = ?");
        }
        if services.is_some() {
            sets.push("services = ?");
        }
        sets.push("updated_at = datetime('now')");
        let sql = format!("UPDATE federations SET {} WHERE id = ?", sets.join(", "));
        let mut q = sqlx::query(&sql);
        if let Some(n) = name {
            q = q.bind(n);
        }
        if let Some(d) = description {
            q = q.bind(d);
        }
        let svc_str;
        if let Some(s) = services {
            svc_str = serde_json::to_string(s)
                .map_err(|e| StoreError::Hash(format!("encode services: {}", e)))?;
            q = q.bind(&svc_str);
        }
        q = q.bind(id.to_string());
        let res = q.execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            return Ok(None);
        }
        self.find_federation_by_id(id).await
    }

    async fn delete_federation(&self, id: Uuid) -> StoreResult<()> {
        sqlx::query("DELETE FROM federations WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
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
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let content_str = serde_json::to_string(&content_json)
            .map_err(|e| StoreError::Hash(format!("encode content_json: {}", e)))?;
        sqlx::query(
            r#"
            INSERT INTO templates (
                id, org_id, name, slug, description, author_id, version,
                category, tags, content_json, requirements,
                compatibility_json, stats_json, published,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, '[]', '{}',
                    '{"downloads":0,"stars":0,"forks":0,"rating":0.0,"rating_count":0}',
                    0, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.map(|u| u.to_string()))
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(author_id.to_string())
        .bind(version)
        .bind(category.to_string())
        .bind(&content_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query("SELECT * FROM templates WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await?;
        row_to_template(&row)
    }

    async fn find_template_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> StoreResult<Option<Template>> {
        let row = sqlx::query("SELECT * FROM templates WHERE name = ? AND version = ?")
            .bind(name)
            .bind(version)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(row_to_template(&r)?)),
            None => Ok(None),
        }
    }

    async fn list_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Template>> {
        let rows = sqlx::query("SELECT * FROM templates WHERE org_id = ? ORDER BY created_at DESC")
            .bind(org_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_template).collect()
    }

    async fn search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Template>> {
        // Mirror of `search_scenarios`: build the filter clauses
        // additively. Tag matching uses LIKE against the JSON array
        // column — exact-keyword only, which is fine for the OSS admin
        // data volumes.
        let mut sql = String::from("SELECT * FROM templates WHERE 1=1");
        if query.is_some() {
            sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
        }
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        if org_id.is_some() {
            sql.push_str(" AND org_id = ?");
        }
        for _ in tags {
            sql.push_str(" AND tags LIKE ?");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query(&sql);
        let like = |s: &str| format!("%{}%", s);
        if let Some(qs) = query {
            q = q.bind(like(qs)).bind(like(qs));
        }
        if let Some(cat) = category {
            q = q.bind(cat.to_string());
        }
        if let Some(oid) = org_id {
            q = q.bind(oid.to_string());
        }
        for t in tags {
            q = q.bind(format!("%\"{}\"%", t));
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        rows.iter().map(row_to_template).collect()
    }

    async fn count_search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM templates WHERE 1=1");
        if query.is_some() {
            sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
        }
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        if org_id.is_some() {
            sql.push_str(" AND org_id = ?");
        }
        for _ in tags {
            sql.push_str(" AND tags LIKE ?");
        }

        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        let like = |s: &str| format!("%{}%", s);
        if let Some(qs) = query {
            q = q.bind(like(qs)).bind(like(qs));
        }
        if let Some(cat) = category {
            q = q.bind(cat.to_string());
        }
        if let Some(oid) = org_id {
            q = q.bind(oid.to_string());
        }
        for t in tags {
            q = q.bind(format!("%\"{}\"%", t));
        }

        let (count,) = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

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
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let manifest_str = serde_json::to_string(&manifest_json)
            .map_err(|e| StoreError::Hash(format!("encode manifest: {}", e)))?;
        sqlx::query(
            r#"
            INSERT INTO scenarios (
                id, org_id, name, slug, description, author_id,
                current_version, category, tags, license, manifest_json,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(org_id.map(|u| u.to_string()))
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(author_id.to_string())
        .bind(current_version)
        .bind(category)
        .bind(license)
        .bind(&manifest_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Re-read so we pick up DEFAULT-populated columns (rating_avg,
        // downloads_total, etc.) without duplicating defaults here.
        let row = sqlx::query("SELECT * FROM scenarios WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await?;
        row_to_scenario(&row)
    }

    async fn find_scenario_by_name(&self, name: &str) -> StoreResult<Option<Scenario>> {
        let row = sqlx::query("SELECT * FROM scenarios WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(row_to_scenario(&r)?)),
            None => Ok(None),
        }
    }

    async fn list_scenarios_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Scenario>> {
        let rows = sqlx::query("SELECT * FROM scenarios WHERE org_id = ? ORDER BY created_at DESC")
            .bind(org_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_scenario).collect()
    }

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
        let mut sql = String::from("SELECT * FROM scenarios WHERE 1=1");
        if query.is_some() {
            sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
        }
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        if org_id.is_some() {
            sql.push_str(" AND org_id = ?");
        }
        // Tag filter uses LIKE against the JSON array — imprecise but the
        // tag-scan test in Postgres uses a materialized join; on SQLite
        // with small OSS datasets this is fine.
        for _ in tags {
            sql.push_str(" AND tags LIKE ?");
        }
        match sort {
            "downloads" => sql.push_str(" ORDER BY downloads_total DESC"),
            "rating" => sql.push_str(" ORDER BY rating_avg DESC"),
            "recent" => sql.push_str(" ORDER BY created_at DESC"),
            "name" => sql.push_str(" ORDER BY name ASC"),
            _ => sql.push_str(" ORDER BY downloads_total DESC"),
        }
        sql.push_str(" LIMIT ? OFFSET ?");

        let mut q = sqlx::query(&sql);
        let like = |s: &str| format!("%{}%", s);
        if let Some(qs) = query {
            q = q.bind(like(qs)).bind(like(qs));
        }
        if let Some(cat) = category {
            q = q.bind(cat.to_string());
        }
        if let Some(oid) = org_id {
            q = q.bind(oid.to_string());
        }
        for t in tags {
            q = q.bind(format!("%\"{}\"%", t));
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        rows.iter().map(row_to_scenario).collect()
    }

    async fn count_search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM scenarios WHERE 1=1");
        if query.is_some() {
            sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
        }
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        if org_id.is_some() {
            sql.push_str(" AND org_id = ?");
        }
        for _ in tags {
            sql.push_str(" AND tags LIKE ?");
        }

        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        let like = |s: &str| format!("%{}%", s);
        if let Some(qs) = query {
            q = q.bind(like(qs)).bind(like(qs));
        }
        if let Some(cat) = category {
            q = q.bind(cat.to_string());
        }
        if let Some(oid) = org_id {
            q = q.bind(oid.to_string());
        }
        for t in tags {
            q = q.bind(format!("%\"{}\"%", t));
        }

        let (count,) = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

    #[allow(unused_variables)]
    async fn search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        language: Option<&str>,
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
        language: Option<&str>,
        tags: &[String],
    ) -> StoreResult<i64> {
        Ok(0)
    }

    #[allow(unused_variables)]
    async fn find_plugin_by_name(&self, name: &str) -> StoreResult<Option<Plugin>> {
        Ok(None)
    }

    async fn get_plugin_tags(&self, plugin_id: Uuid) -> StoreResult<Vec<String>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT t.name
            FROM tags t
            INNER JOIN plugin_tags pt ON pt.tag_id = t.id
            WHERE pt.plugin_id = ?
            ORDER BY t.name
            "#,
        )
        .bind(plugin_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| r.try_get::<String, _>("name").map_err(Into::into))
            .collect()
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
        language: &str,
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
        sbom_json: Option<&serde_json::Value>,
    ) -> StoreResult<PluginVersion> {
        Err(StoreError::NotFound)
    }

    #[allow(unused_variables)]
    async fn get_plugin_version_sbom(
        &self,
        plugin_version_id: Uuid,
    ) -> StoreResult<Option<serde_json::Value>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT sbom_json FROM plugin_versions WHERE id = ?")
            .bind(plugin_version_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else { return Ok(None) };
        let sbom_str: Option<String> = row.try_get("sbom_json")?;
        Ok(match sbom_str {
            Some(s) => Some(
                serde_json::from_str(&s)
                    .map_err(|e| StoreError::Hash(format!("bad sbom_json: {}", e)))?,
            ),
            None => None,
        })
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

    async fn upsert_plugin_security_scan(
        &self,
        plugin_version_id: Uuid,
        status: &str,
        score: i16,
        findings: &serde_json::Value,
        scanner_version: Option<&str>,
    ) -> StoreResult<()> {
        let findings_json = serde_json::to_string(findings)
            .map_err(|e| StoreError::Hash(format!("encode findings: {}", e)))?;
        let now = Utc::now().to_rfc3339();
        let new_id = Uuid::new_v4().to_string();

        // SQLite ON CONFLICT on the unique index (plugin_version_id) matches
        // the Postgres upsert semantics: latest scan wins, other fields are
        // overwritten, id stays stable after the first write.
        sqlx::query(
            r#"
            INSERT INTO plugin_security_scans
                (id, plugin_version_id, status, score, findings, scanner_version, scanned_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(plugin_version_id) DO UPDATE SET
                status = excluded.status,
                score = excluded.score,
                findings = excluded.findings,
                scanner_version = excluded.scanner_version,
                scanned_at = excluded.scanned_at
            "#,
        )
        .bind(new_id)
        .bind(plugin_version_id.to_string())
        .bind(status)
        .bind(score as i64)
        .bind(findings_json)
        .bind(scanner_version)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn latest_security_scan_for_plugin(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<Option<PluginSecurityScan>> {
        use sqlx::Row;
        let row = sqlx::query(
            r#"
            SELECT s.id, s.plugin_version_id, s.status, s.score, s.findings,
                   s.scanner_version, s.scanned_at
            FROM plugin_security_scans s
            INNER JOIN plugin_versions v ON v.id = s.plugin_version_id
            INNER JOIN plugins p ON p.id = v.plugin_id AND p.current_version = v.version
            WHERE v.plugin_id = ?
            ORDER BY s.scanned_at DESC
            LIMIT 1
            "#,
        )
        .bind(plugin_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else { return Ok(None) };

        let id_str: String = row.try_get("id")?;
        let version_id_str: String = row.try_get("plugin_version_id")?;
        let findings_json: String = row.try_get("findings")?;
        let scanned_at_str: String = row.try_get("scanned_at")?;
        let score_i64: i64 = row.try_get("score")?;
        let findings: serde_json::Value = serde_json::from_str(&findings_json)
            .map_err(|e| StoreError::Hash(format!("bad findings json: {}", e)))?;

        Ok(Some(PluginSecurityScan {
            id: parse_uuid(&id_str)?,
            plugin_version_id: parse_uuid(&version_id_str)?,
            status: row.try_get("status")?,
            score: score_i64 as i16,
            findings,
            scanner_version: row.try_get("scanner_version")?,
            scanned_at: parse_dt(&scanned_at_str)?,
        }))
    }

    async fn list_pending_security_scans(&self, limit: i64) -> StoreResult<Vec<PendingScanJob>> {
        use sqlx::Row;
        // Oldest pending scans first so a burst of publishes drains in
        // order. Same shape as the Postgres query in `postgres.rs`.
        let rows = sqlx::query(
            r#"
            SELECT
                v.id AS plugin_version_id,
                p.name AS plugin_name,
                v.version AS version,
                v.file_size AS file_size,
                v.checksum AS checksum
            FROM plugin_security_scans s
            INNER JOIN plugin_versions v ON v.id = s.plugin_version_id
            INNER JOIN plugins p ON p.id = v.plugin_id
            WHERE s.status = 'pending'
            ORDER BY s.scanned_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut jobs = Vec::with_capacity(rows.len());
        for row in rows {
            let version_id_str: String = row.try_get("plugin_version_id")?;
            jobs.push(PendingScanJob {
                plugin_version_id: parse_uuid(&version_id_str)?,
                plugin_name: row.try_get("plugin_name")?,
                version: row.try_get("version")?,
                file_size: row.try_get("file_size")?,
                checksum: row.try_get("checksum")?,
            });
        }
        Ok(jobs)
    }

    async fn find_osv_matches(
        &self,
        ecosystem: &str,
        package_name: &str,
        version: &str,
    ) -> StoreResult<Vec<OsvMatch>> {
        use sqlx::Row;
        let eco = ecosystem.to_ascii_lowercase();
        let name = package_name.to_ascii_lowercase();
        let rows = sqlx::query(
            r#"
            SELECT advisory_id, severity, summary, affected_versions
            FROM osv_vulnerabilities
            WHERE ecosystem = ? AND LOWER(package_name) = ?
            "#,
        )
        .bind(&eco)
        .bind(&name)
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::new();
        for row in rows {
            let advisory_id: String = row.try_get("advisory_id")?;
            let severity: String = row.try_get("severity")?;
            let summary: String = row.try_get("summary")?;
            let affected_str: String = row.try_get("affected_versions")?;
            let affected: serde_json::Value = serde_json::from_str(&affected_str)
                .map_err(|e| StoreError::Hash(format!("bad affected_versions: {}", e)))?;
            if crate::store::version_affected_in_ecosystem(&affected, version, ecosystem) {
                hits.push(OsvMatch {
                    advisory_id,
                    severity,
                    summary,
                });
            }
        }
        Ok(hits)
    }

    async fn upsert_osv_advisory(&self, record: &OsvImportRecord) -> StoreResult<usize> {
        let severity = record.severity_bucket().to_string();
        let summary = record.human_summary();
        let modified = crate::store::parse_modified_str(record.modified.as_deref());
        let extra = serde_json::to_string(record)
            .map_err(|e| StoreError::Hash(format!("encode extra_json: {}", e)))?;

        let mut imported = 0usize;
        for affected in &record.affected {
            let ecosystem = affected.package.ecosystem.to_ascii_lowercase();
            let package_name = affected.package.name.clone();
            let affected_json = serde_json::json!({
                "ranges": affected.ranges,
                "versions": affected.versions,
            });
            let affected_str = serde_json::to_string(&affected_json)
                .map_err(|e| StoreError::Hash(format!("encode affected_versions: {}", e)))?;
            let new_id = Uuid::new_v4().to_string();

            sqlx::query(
                r#"
                INSERT INTO osv_vulnerabilities
                    (id, advisory_id, ecosystem, package_name, severity,
                     summary, affected_versions, extra_json, modified_at,
                     imported_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
                ON CONFLICT (advisory_id, ecosystem, package_name) DO UPDATE SET
                    severity = excluded.severity,
                    summary = excluded.summary,
                    affected_versions = excluded.affected_versions,
                    extra_json = excluded.extra_json,
                    modified_at = excluded.modified_at,
                    imported_at = datetime('now')
                "#,
            )
            .bind(new_id)
            .bind(&record.id)
            .bind(&ecosystem)
            .bind(&package_name)
            .bind(&severity)
            .bind(&summary)
            .bind(&affected_str)
            .bind(&extra)
            .bind(modified)
            .execute(&self.pool)
            .await?;
            imported += 1;
        }
        Ok(imported)
    }

    async fn count_osv_advisories(&self) -> StoreResult<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM osv_vulnerabilities")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn list_user_public_keys(&self, user_id: Uuid) -> StoreResult<Vec<UserPublicKey>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, algorithm, public_key_b64, label,
                   created_at, revoked_at
            FROM user_public_keys
            WHERE user_id = ? AND revoked_at IS NULL
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let user_id_str: String = row.try_get("user_id")?;
            let created_at_str: String = row.try_get("created_at")?;
            let revoked_at_str: Option<String> = row.try_get("revoked_at")?;
            out.push(UserPublicKey {
                id: parse_uuid(&id_str)?,
                user_id: parse_uuid(&user_id_str)?,
                algorithm: row.try_get("algorithm")?,
                public_key_b64: row.try_get("public_key_b64")?,
                label: row.try_get("label")?,
                created_at: parse_dt(&created_at_str)?,
                revoked_at: revoked_at_str.as_deref().map(parse_dt).transpose()?,
            });
        }
        Ok(out)
    }

    async fn create_user_public_key(
        &self,
        user_id: Uuid,
        algorithm: &str,
        public_key_b64: &str,
        label: &str,
    ) -> StoreResult<UserPublicKey> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO user_public_keys
                (id, user_id, algorithm, public_key_b64, label, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(algorithm)
        .bind(public_key_b64)
        .bind(label)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        Ok(UserPublicKey {
            id,
            user_id,
            algorithm: algorithm.to_string(),
            public_key_b64: public_key_b64.to_string(),
            label: label.to_string(),
            created_at: now,
            revoked_at: None,
        })
    }

    async fn revoke_user_public_key(&self, user_id: Uuid, key_id: Uuid) -> StoreResult<bool> {
        let res = sqlx::query(
            r#"
            UPDATE user_public_keys
            SET revoked_at = datetime('now')
            WHERE id = ? AND user_id = ? AND revoked_at IS NULL
            "#,
        )
        .bind(key_id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn record_plugin_version_attestation(
        &self,
        plugin_version_id: Uuid,
        key_id: Option<Uuid>,
    ) -> StoreResult<()> {
        // SQLite doesn't have a `NOW()` equivalent inside a CASE so we
        // just conditionally run one of two statements; the branches are
        // tiny.
        if let Some(k) = key_id {
            sqlx::query(
                "UPDATE plugin_versions SET sbom_signed_key_id = ?, sbom_signed_at = datetime('now') WHERE id = ?",
            )
            .bind(k.to_string())
            .bind(plugin_version_id.to_string())
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE plugin_versions SET sbom_signed_key_id = NULL, sbom_signed_at = NULL WHERE id = ?",
            )
            .bind(plugin_version_id.to_string())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn get_plugin_version_attestation(
        &self,
        plugin_version_id: Uuid,
    ) -> StoreResult<Option<(Uuid, chrono::DateTime<chrono::Utc>)>> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT sbom_signed_key_id, sbom_signed_at FROM plugin_versions WHERE id = ?",
        )
        .bind(plugin_version_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        let key_str: Option<String> = row.try_get("sbom_signed_key_id")?;
        let ts_str: Option<String> = row.try_get("sbom_signed_at")?;
        match (key_str, ts_str) {
            (Some(k), Some(t)) => Ok(Some((parse_uuid(&k)?, parse_dt(&t)?))),
            _ => Ok(None),
        }
    }

    async fn get_plugin_reviews(
        &self,
        plugin_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Review>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, plugin_id, version, user_id, rating, title, comment,
                   helpful_count, unhelpful_count, verified,
                   created_at, updated_at
            FROM reviews
            WHERE plugin_id = ?
            ORDER BY helpful_count DESC, created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(plugin_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let plugin_id_str: String = row.try_get("plugin_id")?;
            let user_id_str: String = row.try_get("user_id")?;
            let rating_i64: i64 = row.try_get("rating")?;
            let helpful: i64 = row.try_get("helpful_count")?;
            let unhelpful: i64 = row.try_get("unhelpful_count")?;
            let created_at_str: String = row.try_get("created_at")?;
            let updated_at_str: String = row.try_get("updated_at")?;
            out.push(Review {
                id: parse_uuid(&id_str)?,
                plugin_id: parse_uuid(&plugin_id_str)?,
                version: row.try_get("version")?,
                user_id: parse_uuid(&user_id_str)?,
                rating: rating_i64 as i16,
                title: row.try_get("title")?,
                comment: row.try_get("comment")?,
                helpful_count: helpful as i32,
                unhelpful_count: unhelpful as i32,
                verified: row.try_get("verified")?,
                created_at: parse_dt(&created_at_str)?,
                updated_at: parse_dt(&updated_at_str)?,
            });
        }
        Ok(out)
    }

    async fn count_plugin_reviews(&self, plugin_id: Uuid) -> StoreResult<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reviews WHERE plugin_id = ?")
            .bind(plugin_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn create_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
        version: &str,
        rating: i16,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<Review> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO reviews (
                id, plugin_id, version, user_id, rating, title, comment,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(plugin_id.to_string())
        .bind(version)
        .bind(user_id.to_string())
        .bind(rating as i64)
        .bind(title)
        .bind(comment)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(Review {
            id,
            plugin_id,
            version: version.to_string(),
            user_id,
            rating,
            title: title.map(str::to_string),
            comment: comment.to_string(),
            helpful_count: 0,
            unhelpful_count: 0,
            verified: false,
            created_at: parse_dt(&now)?,
            updated_at: parse_dt(&now)?,
        })
    }

    async fn get_plugin_review_stats(&self, plugin_id: Uuid) -> StoreResult<(f64, i64)> {
        use sqlx::Row;
        // COALESCE(AVG(...), 0) — SQLite returns NULL from AVG on empty sets.
        let row = sqlx::query(
            "SELECT COALESCE(AVG(rating), 0.0) AS avg, COUNT(*) AS cnt
             FROM reviews WHERE plugin_id = ?",
        )
        .bind(plugin_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        let avg: f64 = row.try_get("avg")?;
        let cnt: i64 = row.try_get("cnt")?;
        Ok((avg, cnt))
    }

    async fn get_plugin_review_distribution(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<i16, i64>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT rating, COUNT(*) AS cnt FROM reviews
             WHERE plugin_id = ? GROUP BY rating",
        )
        .bind(plugin_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut out = std::collections::HashMap::new();
        for row in rows {
            let rating: i64 = row.try_get("rating")?;
            let cnt: i64 = row.try_get("cnt")?;
            out.insert(rating as i16, cnt);
        }
        Ok(out)
    }

    async fn find_existing_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT id FROM reviews WHERE plugin_id = ? AND user_id = ?")
            .bind(plugin_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let id_str: String = r.try_get("id")?;
                Ok(Some(parse_uuid(&id_str)?))
            }
            None => Ok(None),
        }
    }

    async fn update_plugin_rating_stats(
        &self,
        plugin_id: Uuid,
        avg: f64,
        count: i32,
    ) -> StoreResult<()> {
        sqlx::query("UPDATE plugins SET rating_avg = ?, rating_count = ? WHERE id = ?")
            .bind(avg)
            .bind(count as i64)
            .bind(plugin_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn increment_plugin_review_vote(
        &self,
        plugin_id: Uuid,
        review_id: Uuid,
        helpful: bool,
    ) -> StoreResult<()> {
        // Column name is selected from a whitelist — no user input reaches
        // the SQL string, so string interpolation here is safe.
        let field = if helpful {
            "helpful_count"
        } else {
            "unhelpful_count"
        };
        let sql = format!("UPDATE reviews SET {0} = {0} + 1 WHERE id = ? AND plugin_id = ?", field);
        sqlx::query(&sql)
            .bind(review_id.to_string())
            .bind(plugin_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_user_public_info(&self, user_id: Uuid) -> StoreResult<Option<(String, String)>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT id, username FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let id: String = r.try_get("id")?;
                let username: String = r.try_get("username")?;
                Ok(Some((id, username)))
            }
            None => Ok(None),
        }
    }

    async fn get_template_reviews(
        &self,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<TemplateReview>> {
        let rows = sqlx::query(
            r#"SELECT * FROM template_reviews
               WHERE template_id = ?
               ORDER BY helpful_count DESC, created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(template_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_template_review).collect()
    }

    async fn count_template_reviews(&self, template_id: Uuid) -> StoreResult<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM template_reviews WHERE template_id = ?")
                .bind(template_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    async fn create_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<TemplateReview> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"INSERT INTO template_reviews
                   (id, template_id, reviewer_id, rating, title, comment,
                    created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(template_id.to_string())
        .bind(reviewer_id.to_string())
        .bind(rating as i64)
        .bind(title)
        .bind(comment)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        Ok(TemplateReview {
            id,
            template_id,
            reviewer_id,
            rating,
            title: title.map(str::to_string),
            comment: comment.to_string(),
            helpful_count: 0,
            verified_use: false,
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_template_review_stats(&self, template_id: Uuid) -> StoreResult<()> {
        use sqlx::Row;
        // Recompute AVG(rating) + COUNT(*) from template_reviews and push
        // them into the template's `stats_json`. Unlike the Postgres impl
        // (which has typed rating columns on the templates row), SQLite
        // stores template stats inside a JSON blob so we rewrite the
        // `rating` + `rating_count` fields while preserving the rest.
        let row = sqlx::query(
            "SELECT COALESCE(AVG(rating), 0.0) AS avg, COUNT(*) AS cnt
                         FROM template_reviews WHERE template_id = ?",
        )
        .bind(template_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        let avg: f64 = row.try_get("avg")?;
        let cnt: i64 = row.try_get("cnt")?;

        let existing_stats_row = sqlx::query("SELECT stats_json FROM templates WHERE id = ?")
            .bind(template_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let Some(existing_stats_row) = existing_stats_row else {
            return Ok(());
        };
        let existing: String = existing_stats_row.try_get("stats_json")?;
        let mut stats: serde_json::Value =
            serde_json::from_str(&existing).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = stats.as_object_mut() {
            obj.insert("rating".to_string(), serde_json::json!(avg));
            obj.insert("rating_count".to_string(), serde_json::json!(cnt));
        }
        let stats_str = serde_json::to_string(&stats)
            .map_err(|e| StoreError::Hash(format!("encode stats_json: {}", e)))?;

        sqlx::query("UPDATE templates SET stats_json = ? WHERE id = ?")
            .bind(&stats_str)
            .bind(template_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_existing_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT id FROM template_reviews WHERE template_id = ? AND reviewer_id = ?",
        )
        .bind(template_id.to_string())
        .bind(reviewer_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => {
                let id_str: String = r.try_get("id")?;
                Ok(Some(parse_uuid(&id_str)?))
            }
            None => Ok(None),
        }
    }

    async fn toggle_template_star(
        &self,
        template_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<(bool, i64)> {
        use sqlx::Row;
        // Wrap the flip + recount in a transaction so the count we return
        // reflects the state the caller just set, without racing other
        // concurrent toggles. Postgres uses the same pattern.
        let mut tx = self.pool.begin().await?;

        let already = sqlx::query(
            "SELECT 1 AS present FROM template_stars
             WHERE template_id = ? AND user_id = ?",
        )
        .bind(template_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        let now_starred = if already {
            sqlx::query("DELETE FROM template_stars WHERE template_id = ? AND user_id = ?")
                .bind(template_id.to_string())
                .bind(user_id.to_string())
                .execute(&mut *tx)
                .await?;
            false
        } else {
            sqlx::query(
                "INSERT INTO template_stars (template_id, user_id) VALUES (?, ?)
                 ON CONFLICT (template_id, user_id) DO NOTHING",
            )
            .bind(template_id.to_string())
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
            true
        };

        let count_row =
            sqlx::query("SELECT COUNT(*) AS cnt FROM template_stars WHERE template_id = ?")
                .bind(template_id.to_string())
                .fetch_one(&mut *tx)
                .await?;
        let count: i64 = count_row.try_get("cnt")?;

        tx.commit().await?;
        Ok((now_starred, count))
    }

    async fn is_template_starred_by(&self, template_id: Uuid, user_id: Uuid) -> StoreResult<bool> {
        let row = sqlx::query("SELECT 1 FROM template_stars WHERE template_id = ? AND user_id = ?")
            .bind(template_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    async fn count_template_stars(&self, template_id: Uuid) -> StoreResult<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM template_stars WHERE template_id = ?")
                .bind(template_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    async fn count_template_stars_batch(
        &self,
        template_ids: &[Uuid],
    ) -> StoreResult<std::collections::HashMap<Uuid, i64>> {
        use sqlx::Row;
        if template_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // SQLite caps bound parameters at `SQLITE_MAX_VARIABLE_NUMBER`
        // (default 999 in the embedded build, 250000 on the modern
        // CLI default). We deliberately stay below the *conservative*
        // 999 cap so the code works regardless of how libsqlite3-sys
        // was compiled downstream. Chunks of 900 leave headroom for
        // `PRAGMA`-introduced slots and future additions without a
        // rebuild.
        //
        // The chunked path produces identical output to the single-query
        // variant because the grouping is per-template and the HashMap
        // is keyed on the template id — no row appears in more than one
        // chunk.
        const CHUNK: usize = 900;

        let mut out = std::collections::HashMap::new();
        for chunk in template_ids.chunks(CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let sql = format!(
                "SELECT template_id, COUNT(*) AS cnt
                 FROM template_stars
                 WHERE template_id IN ({})
                 GROUP BY template_id",
                placeholders
            );
            let mut q = sqlx::query(&sql);
            for id in chunk {
                q = q.bind(id.to_string());
            }
            let rows = q.fetch_all(&self.pool).await?;
            for row in rows {
                let id_str: String = row.try_get("template_id")?;
                let cnt: i64 = row.try_get("cnt")?;
                out.insert(parse_uuid(&id_str)?, cnt);
            }
        }
        Ok(out)
    }

    async fn get_scenario_reviews(
        &self,
        scenario_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<ScenarioReview>> {
        let rows = sqlx::query(
            r#"SELECT * FROM scenario_reviews
               WHERE scenario_id = ?
               ORDER BY helpful_count DESC, created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(scenario_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_scenario_review).collect()
    }

    async fn count_scenario_reviews(&self, scenario_id: Uuid) -> StoreResult<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scenario_reviews WHERE scenario_id = ?")
                .bind(scenario_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    async fn create_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<ScenarioReview> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"INSERT INTO scenario_reviews
                   (id, scenario_id, reviewer_id, rating, title, comment,
                    created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(scenario_id.to_string())
        .bind(reviewer_id.to_string())
        .bind(rating as i64)
        .bind(title)
        .bind(comment)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        Ok(ScenarioReview {
            id,
            scenario_id,
            reviewer_id,
            rating,
            title: title.map(str::to_string),
            comment: comment.to_string(),
            helpful_count: 0,
            verified_purchase: false,
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_scenario_review_stats(&self, scenario_id: Uuid) -> StoreResult<()> {
        use sqlx::Row;
        // Mirror of the Postgres implementation: recompute AVG + COUNT
        // from scenario_reviews and push them into the scenarios row.
        // SQLite stores rating_avg as REAL, so no decimal conversion is
        // needed.
        let row = sqlx::query(
            "SELECT COALESCE(AVG(rating), 0.0) AS avg, COUNT(*) AS cnt
             FROM scenario_reviews WHERE scenario_id = ?",
        )
        .bind(scenario_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        let avg: f64 = row.try_get("avg")?;
        let cnt: i64 = row.try_get("cnt")?;

        sqlx::query("UPDATE scenarios SET rating_avg = ?, rating_count = ? WHERE id = ?")
            .bind(avg)
            .bind(cnt)
            .bind(scenario_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_existing_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT id FROM scenario_reviews WHERE scenario_id = ? AND reviewer_id = ?",
        )
        .bind(scenario_id.to_string())
        .bind(reviewer_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => {
                let id_str: String = r.try_get("id")?;
                Ok(Some(parse_uuid(&id_str)?))
            }
            None => Ok(None),
        }
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
            "plugins",
            "plugin_versions",
            "plugin_security_scans",
            "reviews",
            "tags",
            "plugin_tags",
            "osv_vulnerabilities",
            "templates",
            "template_versions",
            "template_reviews",
            "template_tags",
            "template_stars",
            "scenarios",
            "scenario_versions",
            "scenario_reviews",
            "scenario_tags",
            "federations",
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

    /// Confirm `count_template_stars_batch` honours the chunk boundary.
    /// We feed 1800 random ids — twice the internal CHUNK of 900 —
    /// and verify (a) the call doesn't trip SQLite's variable cap, and
    /// (b) the result is correct across chunk boundaries. Only a handful
    /// of ids are actual templates with stars; the rest are noise so the
    /// batch has to traverse every chunk just to get "no rows" answers.
    #[tokio::test]
    async fn test_template_star_batch_chunks_past_sqlite_cap() {
        use crate::models::template::TemplateCategory;
        let store = memory_store().await;
        let pool = store.pool();

        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'chunk-user', 'chunk@example.com', 'x',
                       datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // Two real templates — star both.
        let real1 = store
            .create_template(
                None,
                "chunked-1",
                "chunked-1",
                "",
                user_id,
                "1.0.0",
                TemplateCategory::CustomScenario,
                serde_json::json!({}),
            )
            .await
            .unwrap();
        let real2 = store
            .create_template(
                None,
                "chunked-2",
                "chunked-2",
                "",
                user_id,
                "1.0.0",
                TemplateCategory::CustomScenario,
                serde_json::json!({}),
            )
            .await
            .unwrap();
        store.toggle_template_star(real1.id, user_id).await.unwrap();
        store.toggle_template_star(real2.id, user_id).await.unwrap();

        // 1800 ids total: 2 real (somewhere in the middle of the list)
        // and 1798 garbage. Crosses CHUNK=900 twice.
        let mut ids = (0..898).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
        ids.push(real1.id);
        ids.extend((0..899).map(|_| Uuid::new_v4()));
        ids.push(real2.id);
        ids.extend((0..1).map(|_| Uuid::new_v4()));
        assert_eq!(ids.len(), 1800);

        let counts = store.count_template_stars_batch(&ids).await.unwrap();
        // Only the real ones show up; everything else is absent.
        assert_eq!(counts.len(), 2);
        assert_eq!(counts.get(&real1.id), Some(&1));
        assert_eq!(counts.get(&real2.id), Some(&1));
    }

    /// Template star lifecycle on SQLite. Exercises each public method
    /// once: toggle creates + increments count, toggle again deletes +
    /// decrements, is_starred_by reflects state, count_template_stars
    /// agrees with toggle's returned count, and the batch variant
    /// returns only templates with at least one star.
    #[tokio::test]
    async fn test_template_star_lifecycle() {
        use crate::models::template::TemplateCategory;
        let store = memory_store().await;
        let pool = store.pool();

        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        for (uid, uname, email) in [
            (user_a, "star-a", "a@example.com"),
            (user_b, "star-b", "b@example.com"),
        ] {
            sqlx::query(
                r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
                   VALUES (?, ?, ?, 'x', datetime('now'), datetime('now'))"#,
            )
            .bind(uid.to_string())
            .bind(uname)
            .bind(email)
            .execute(pool)
            .await
            .unwrap();
        }

        let t1 = store
            .create_template(
                None,
                "starry-1",
                "starry-1",
                "",
                user_a,
                "1.0.0",
                TemplateCategory::CustomScenario,
                serde_json::json!({}),
            )
            .await
            .unwrap();
        let t2 = store
            .create_template(
                None,
                "starry-2",
                "starry-2",
                "",
                user_a,
                "1.0.0",
                TemplateCategory::CustomScenario,
                serde_json::json!({}),
            )
            .await
            .unwrap();

        // First star → true/1. Second star by the same user → off, count 0.
        let (now, cnt) = store.toggle_template_star(t1.id, user_a).await.unwrap();
        assert!(now);
        assert_eq!(cnt, 1);
        assert!(store.is_template_starred_by(t1.id, user_a).await.unwrap());
        let (now, cnt) = store.toggle_template_star(t1.id, user_a).await.unwrap();
        assert!(!now);
        assert_eq!(cnt, 0);
        assert!(!store.is_template_starred_by(t1.id, user_a).await.unwrap());

        // Two users both star t1, one stars t2. Counts agree with toggle's
        // in-transaction recount.
        assert_eq!(store.toggle_template_star(t1.id, user_a).await.unwrap(), (true, 1));
        assert_eq!(store.toggle_template_star(t1.id, user_b).await.unwrap(), (true, 2));
        assert_eq!(store.toggle_template_star(t2.id, user_a).await.unwrap(), (true, 1));

        assert_eq!(store.count_template_stars(t1.id).await.unwrap(), 2);
        assert_eq!(store.count_template_stars(t2.id).await.unwrap(), 1);

        // Batch: starred-but-zero templates get omitted from the map.
        let t3 = store
            .create_template(
                None,
                "starry-empty",
                "starry-empty",
                "",
                user_a,
                "1.0.0",
                TemplateCategory::CustomScenario,
                serde_json::json!({}),
            )
            .await
            .unwrap();
        let batch = store.count_template_stars_batch(&[t1.id, t2.id, t3.id]).await.unwrap();
        assert_eq!(batch.get(&t1.id), Some(&2));
        assert_eq!(batch.get(&t2.id), Some(&1));
        assert!(batch.get(&t3.id).is_none(), "zero-star templates omitted");

        // Empty input → empty map (fast path, no SQL).
        let empty = store.count_template_stars_batch(&[]).await.unwrap();
        assert!(empty.is_empty());
    }

    /// Templates CRUD on SQLite: create + find_by_name_version +
    /// list_by_org + search (query/category/tags/org filter combos) +
    /// count_search. Mirrors the scenario test shape; same reasoning
    /// about what the marketplace page depends on.
    #[tokio::test]
    async fn test_templates_crud_roundtrip() {
        use crate::models::template::TemplateCategory;
        let store = memory_store().await;
        let pool = store.pool();

        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'tmpl-author', 'tmpl@example.com', 'x',
                       datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let org_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO organizations (id, name, slug, owner_id, plan, limits_json,
                                          stripe_customer_id, created_at, updated_at)
               VALUES (?, 'TmplOrg', 'tmpl-org', ?, 'free', '{}', NULL,
                       datetime('now'), datetime('now'))"#,
        )
        .bind(org_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let t1 = store
            .create_template(
                Some(org_id),
                "chaos-toolkit-demo",
                "chaos-toolkit-demo",
                "Introduces network jitter",
                user_id,
                "1.0.0",
                TemplateCategory::NetworkChaos,
                serde_json::json!({"delay": "250ms"}),
            )
            .await
            .unwrap();
        let t2 = store
            .create_template(
                None,
                "resilience-probe",
                "resilience-probe",
                "Probes retry logic",
                user_id,
                "0.3.1",
                TemplateCategory::ResilienceTesting,
                serde_json::json!({"probes": ["status"]}),
            )
            .await
            .unwrap();
        assert_ne!(t1.id, t2.id);
        assert_eq!(t1.category, "network-chaos");

        // Tag one template (stored as a JSON array column).
        sqlx::query("UPDATE templates SET tags = ? WHERE id = ?")
            .bind(r#"["chaos","demo"]"#)
            .bind(t1.id.to_string())
            .execute(pool)
            .await
            .unwrap();

        // find_template_by_name_version
        let found = store.find_template_by_name_version("resilience-probe", "0.3.1").await.unwrap();
        assert_eq!(found.unwrap().id, t2.id);
        assert!(store.find_template_by_name_version("nope", "1.0.0").await.unwrap().is_none());

        // list_templates_by_org scopes to org_id.
        let listed = store.list_templates_by_org(org_id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, t1.id);

        // search: no filters returns both.
        assert_eq!(store.search_templates(None, None, &[], None, 10, 0).await.unwrap().len(), 2);
        assert_eq!(store.count_search_templates(None, None, &[], None).await.unwrap(), 2);

        // category filter.
        let chaos_only = store
            .search_templates(None, Some("network-chaos"), &[], None, 10, 0)
            .await
            .unwrap();
        assert_eq!(chaos_only.len(), 1);
        assert_eq!(chaos_only[0].id, t1.id);

        // query filter against description.
        let q_hits = store.search_templates(Some("retry"), None, &[], None, 10, 0).await.unwrap();
        assert_eq!(q_hits.len(), 1);
        assert_eq!(q_hits[0].id, t2.id);

        // tag filter against JSON column.
        let tag_hits = store
            .search_templates(None, None, &["demo".to_string()], None, 10, 0)
            .await
            .unwrap();
        assert_eq!(tag_hits.len(), 1);
        assert_eq!(tag_hits[0].id, t1.id);

        // combined filters.
        assert_eq!(
            store
                .count_search_templates(Some("jitter"), Some("network-chaos"), &[], Some(org_id))
                .await
                .unwrap(),
            1
        );
    }

    /// Federations CRUD round-trip on SQLite. Covers create + find_by_id
    /// + list_by_org + update (partial — only the `description` field) +
    /// delete.
    #[tokio::test]
    async fn test_federations_crud_roundtrip() {
        let store = memory_store().await;
        let pool = store.pool();

        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'fed-owner', 'fed@example.com', 'x', datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let org_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO organizations (id, name, slug, owner_id, plan, limits_json,
                                          stripe_customer_id, created_at, updated_at)
               VALUES (?, 'FedOrg', 'fed-org', ?, 'free', '{}', NULL,
                       datetime('now'), datetime('now'))"#,
        )
        .bind(org_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let services_a = serde_json::json!([{"name": "svc-a", "url": "https://a.example"}]);
        let fed = store
            .create_federation(org_id, user_id, "prod-edge", "initial", &services_a)
            .await
            .unwrap();
        assert_eq!(fed.name, "prod-edge");
        assert_eq!(fed.description, "initial");

        // Round-trip by id.
        let got = store.find_federation_by_id(fed.id).await.unwrap().unwrap();
        assert_eq!(got.id, fed.id);
        assert_eq!(got.services, services_a);

        // Org scan.
        let listed = store.list_federations_by_org(org_id).await.unwrap();
        assert_eq!(listed.len(), 1);

        // Partial update: description only; name + services stay put.
        let updated = store
            .update_federation(fed.id, None, Some("revised"), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "prod-edge");
        assert_eq!(updated.description, "revised");
        assert_eq!(updated.services, services_a);

        // Full update.
        let services_b = serde_json::json!([{"name": "svc-b", "url": "https://b.example"}]);
        let updated = store
            .update_federation(fed.id, Some("prod-edge-v2"), None, Some(&services_b))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "prod-edge-v2");
        assert_eq!(updated.services, services_b);

        // Update targeting a missing federation returns None, not an error.
        let nope = store.update_federation(Uuid::new_v4(), Some("x"), None, None).await.unwrap();
        assert!(nope.is_none());

        // Delete and confirm it's gone.
        store.delete_federation(fed.id).await.unwrap();
        assert!(store.find_federation_by_id(fed.id).await.unwrap().is_none());
        assert!(store.list_federations_by_org(org_id).await.unwrap().is_empty());
    }

    /// Scenario + template reviews CRUD on SQLite. Exercises each
    /// method once per domain: create, count, list-ordering,
    /// find_existing, update_*_review_stats propagation. The two
    /// domains share the shape so one combined test keeps things
    /// compact without leaving holes.
    #[tokio::test]
    async fn test_scenario_and_template_reviews_roundtrip() {
        let store = memory_store().await;
        let pool = store.pool();

        // Seed a user (both reviewer + template/scenario author).
        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'rev-user', 'rev@example.com', 'x', datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // --- Scenario reviews ----
        let scenario = store
            .create_scenario(
                None,
                "net-flap",
                "net-flap",
                "random link flap",
                user_id,
                "1.0.0",
                "chaos",
                "MIT",
                serde_json::json!({"steps": []}),
            )
            .await
            .unwrap();

        assert_eq!(store.count_scenario_reviews(scenario.id).await.unwrap(), 0);
        let sr = store
            .create_scenario_review(scenario.id, user_id, 4, Some("ok"), "works on my laptop")
            .await
            .unwrap();
        assert_eq!(sr.rating, 4);
        assert_eq!(store.count_scenario_reviews(scenario.id).await.unwrap(), 1);

        let existing = store.find_existing_scenario_review(scenario.id, user_id).await.unwrap();
        assert_eq!(existing, Some(sr.id));

        store.update_scenario_review_stats(scenario.id).await.unwrap();
        let (avg, cnt): (f64, i64) =
            sqlx::query_as("SELECT rating_avg, rating_count FROM scenarios WHERE id = ?")
                .bind(scenario.id.to_string())
                .fetch_one(pool)
                .await
                .unwrap();
        assert!((avg - 4.0).abs() < 1e-6);
        assert_eq!(cnt, 1);

        let listed = store.get_scenario_reviews(scenario.id, 10, 0).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, sr.id);

        // --- Template reviews ----
        let template_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO templates
                   (id, name, slug, description, author_id, version, category,
                    content_json)
               VALUES (?, 't1', 't1', 'desc', ?, '1.0.0', 'network-chaos', '{}')"#,
        )
        .bind(template_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        assert_eq!(store.count_template_reviews(template_id).await.unwrap(), 0);
        let tr = store
            .create_template_review(template_id, user_id, 5, None, "excellent template")
            .await
            .unwrap();
        assert_eq!(tr.rating, 5);
        assert_eq!(store.count_template_reviews(template_id).await.unwrap(), 1);

        let existing_t = store.find_existing_template_review(template_id, user_id).await.unwrap();
        assert_eq!(existing_t, Some(tr.id));

        store.update_template_review_stats(template_id).await.unwrap();
        let stats_str: String = sqlx::query_scalar("SELECT stats_json FROM templates WHERE id = ?")
            .bind(template_id.to_string())
            .fetch_one(pool)
            .await
            .unwrap();
        let stats: serde_json::Value = serde_json::from_str(&stats_str).unwrap();
        assert_eq!(stats["rating"], serde_json::json!(5.0));
        assert_eq!(stats["rating_count"], serde_json::json!(1));

        let listed_t = store.get_template_reviews(template_id, 10, 0).await.unwrap();
        assert_eq!(listed_t.len(), 1);
        assert_eq!(listed_t[0].id, tr.id);
    }

    /// Full attestation flow against the SQLite backend: register a
    /// key, sign an SBOM locally with the matching private half, run
    /// the verifier (the exact function the publish handler calls) and
    /// confirm the outcome flows end to end. Tampering with the SBOM
    /// after signing must reject; using a revoked key must reject.
    ///
    /// This closes the coverage gap called out in the prior pass —
    /// the per-piece SQLite tests all pass individually, but nothing
    /// tied them together on the SQLite path the OSS admin actually
    /// hits.
    #[tokio::test]
    async fn test_sbom_attestation_flow_on_sqlite() {
        use crate::models::attestation::{
            verify_sbom_attestation, SbomAttestationInput, SbomVerifyOutcome,
        };
        use base64::Engine;
        use ed25519_dalek::{Signer, SigningKey};
        use rand::RngCore;
        use sha2::{Digest, Sha256};

        let store = memory_store().await;
        let pool = store.pool();

        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'att-user', 'att@example.com', 'x',
                       datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // 1. Generate a keypair locally — the server only ever sees the
        //    public half. Same `from_bytes(random)` shape the CLI uses.
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        let signing = SigningKey::from_bytes(&secret);
        let public_b64 =
            base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());

        // 2. Register via the SQLite store.
        let registered = store
            .create_user_public_key(user_id, "ed25519", &public_b64, "laptop")
            .await
            .unwrap();
        assert!(registered.is_active());

        // 3. Fabricate a plugin checksum + SBOM and sign them exactly
        //    the way the CLI does.
        let checksum_bytes: [u8; 32] = Sha256::digest(b"fake-wasm-bytes").into();
        let checksum_hex: String = checksum_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let sbom = br#"{"components":[{"name":"leftpad","version":"1.0"}]}"#;

        let mut msg_hasher = Sha256::new();
        msg_hasher.update(hex::decode(&checksum_hex).unwrap());
        msg_hasher.update(sbom);
        let msg: [u8; 32] = msg_hasher.finalize().into();
        let sig = signing.sign(&msg);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        // 4. Verify via the same surface the publish handler uses.
        let keys = store.list_user_public_keys(user_id).await.unwrap();
        let outcome = verify_sbom_attestation(
            &keys,
            &SbomAttestationInput {
                artifact_checksum: &checksum_hex,
                sbom_canonical: sbom,
                signature_b64: &sig_b64,
            },
        );
        match outcome {
            SbomVerifyOutcome::Verified { key_id } => assert_eq!(key_id, registered.id),
            other => panic!("expected Verified, got {:?}", other),
        }

        // 5. Verifier must reject a tampered SBOM under the same
        //    signature. This is the property the attestation exists to
        //    guarantee.
        let tampered = br#"{"components":[{"name":"evil","version":"1.0"}]}"#;
        let outcome = verify_sbom_attestation(
            &keys,
            &SbomAttestationInput {
                artifact_checksum: &checksum_hex,
                sbom_canonical: tampered,
                signature_b64: &sig_b64,
            },
        );
        assert!(matches!(outcome, SbomVerifyOutcome::Invalid));

        // 6. Revoke the key; list_user_public_keys stops returning it,
        //    so the verifier reports NoKeys rather than falling back to
        //    an Invalid verdict.
        assert!(store.revoke_user_public_key(user_id, registered.id).await.unwrap());
        let keys_after = store.list_user_public_keys(user_id).await.unwrap();
        assert!(keys_after.is_empty());
        let outcome = verify_sbom_attestation(
            &keys_after,
            &SbomAttestationInput {
                artifact_checksum: &checksum_hex,
                sbom_canonical: sbom,
                signature_b64: &sig_b64,
            },
        );
        assert!(matches!(outcome, SbomVerifyOutcome::NoKeys));

        // 7. And an audit trail survives: the scan-row-recording path
        //    still works after revocation so historical attestations
        //    stay readable.
        let version_id = Uuid::new_v4();
        // Need a plugin_versions row to FK to.
        let plugin_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO plugins (id, name, description, current_version, category,
                                    license, author_id)
               VALUES (?, 'att-plugin', 'd', '1.0.0', 'other', 'MIT', ?)"#,
        )
        .bind(plugin_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO plugin_versions
                   (id, plugin_id, version, download_url, checksum, file_size)
               VALUES (?, ?, '1.0.0', 'https://example.invalid', ?, 0)"#,
        )
        .bind(version_id.to_string())
        .bind(plugin_id.to_string())
        .bind(&checksum_hex)
        .execute(pool)
        .await
        .unwrap();
        store
            .record_plugin_version_attestation(version_id, Some(registered.id))
            .await
            .unwrap();
        let read_back = store
            .get_plugin_version_attestation(version_id)
            .await
            .unwrap()
            .expect("attestation row present");
        assert_eq!(read_back.0, registered.id);
    }

    /// End-to-end exercise of the publisher-key lifecycle on SQLite:
    /// create two keys, list returns both, revoke one, list returns only
    /// the active key, re-revoking returns false. Mirrors the contract
    /// the REST handler relies on.
    #[tokio::test]
    async fn test_user_public_key_lifecycle() {
        use base64::Engine;
        let store = memory_store().await;
        let pool = store.pool();

        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'pk-user', 'pk@example.com', 'x', datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // Start empty.
        assert!(store.list_user_public_keys(user_id).await.unwrap().is_empty());

        // Create two keys — real 32-byte payloads, base64-encoded.
        let key_a_bytes = [0x11u8; 32];
        let key_b_bytes = [0x22u8; 32];
        let b64 = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
        let a = store
            .create_user_public_key(user_id, "ed25519", &b64(&key_a_bytes), "laptop")
            .await
            .unwrap();
        let b = store
            .create_user_public_key(user_id, "ed25519", &b64(&key_b_bytes), "ci")
            .await
            .unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(a.label, "laptop");
        assert!(a.is_active());

        let listed = store.list_user_public_keys(user_id).await.unwrap();
        assert_eq!(listed.len(), 2);

        // Revoke the first. Second revocation is a no-op.
        assert!(store.revoke_user_public_key(user_id, a.id).await.unwrap());
        assert!(!store.revoke_user_public_key(user_id, a.id).await.unwrap());

        let after = store.list_user_public_keys(user_id).await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, b.id);

        // Trying to revoke someone else's key (or a random id) returns false.
        let someone_else = Uuid::new_v4();
        assert!(!store.revoke_user_public_key(someone_else, b.id).await.unwrap());
    }

    /// End-to-end scenarios CRUD on SQLite. Seeds a user + org, creates
    /// two scenarios with different categories/tags/org ownership, then
    /// exercises each reader (find, list_by_org, search with query /
    /// category / tags / sort combos, count_search_scenarios). Serves as
    /// the template for porting the templates + federations domains.
    #[tokio::test]
    async fn test_scenarios_crud_roundtrip() {
        let store = memory_store().await;
        let pool = store.pool();

        // Seed user + org (scenarios reference both).
        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'scenario-author', 'sa@example.com', 'x', datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let org_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO organizations (id, name, slug, owner_id, plan, limits_json,
                                          stripe_customer_id, created_at, updated_at)
               VALUES (?, 'TestOrg', 'test-org', ?, 'free', '{}', NULL,
                       datetime('now'), datetime('now'))"#,
        )
        .bind(org_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // Create two scenarios — one org-scoped, one public (org_id = None).
        let s1 = store
            .create_scenario(
                Some(org_id),
                "network-failure-demo",
                "network-failure-demo",
                "Drops every third packet to test retry logic",
                user_id,
                "1.0.0",
                "chaos",
                "MIT",
                serde_json::json!({"steps": []}),
            )
            .await
            .unwrap();
        assert_eq!(s1.name, "network-failure-demo");
        assert_eq!(s1.category, "chaos");

        let s2 = store
            .create_scenario(
                None,
                "auth-token-expiry",
                "auth-token-expiry",
                "Expires tokens at a configurable interval",
                user_id,
                "0.1.0",
                "auth",
                "Apache-2.0",
                serde_json::json!({"steps": [{"at": "5s"}]}),
            )
            .await
            .unwrap();
        assert_ne!(s1.id, s2.id);

        // Attach a tag to s1 so search-by-tag has something to hit. We
        // store tags as a JSON array on the scenarios row directly (the
        // Postgres schema also has a separate scenario_tags junction
        // table; SQLite matches via the JSON column to keep the OSS
        // admin simple).
        sqlx::query("UPDATE scenarios SET tags = ? WHERE id = ?")
            .bind(r#"["chaos","demo"]"#)
            .bind(s1.id.to_string())
            .execute(pool)
            .await
            .unwrap();

        // find_scenario_by_name
        let found = store.find_scenario_by_name("auth-token-expiry").await.unwrap();
        assert_eq!(found.expect("found").id, s2.id);
        assert!(store.find_scenario_by_name("nonexistent-scenario").await.unwrap().is_none());

        // list_scenarios_by_org returns only org-scoped scenarios.
        let org_listed = store.list_scenarios_by_org(org_id).await.unwrap();
        assert_eq!(org_listed.len(), 1);
        assert_eq!(org_listed[0].id, s1.id);

        // search: no filters → both results.
        assert_eq!(
            store
                .search_scenarios(None, None, &[], None, "recent", 10, 0)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(store.count_search_scenarios(None, None, &[], None).await.unwrap(), 2);

        // search: by category.
        let auth_only = store
            .search_scenarios(None, Some("auth"), &[], None, "name", 10, 0)
            .await
            .unwrap();
        assert_eq!(auth_only.len(), 1);
        assert_eq!(auth_only[0].id, s2.id);

        // search: by query against name.
        let by_name = store
            .search_scenarios(Some("token"), None, &[], None, "name", 10, 0)
            .await
            .unwrap();
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].id, s2.id);

        // search: by tag (LIKE match against the JSON column).
        let by_tag = store
            .search_scenarios(None, None, &["demo".to_string()], None, "name", 10, 0)
            .await
            .unwrap();
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].id, s1.id);

        // search: combined filters.
        let combined = store
            .search_scenarios(Some("failure"), Some("chaos"), &[], Some(org_id), "name", 10, 0)
            .await
            .unwrap();
        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].id, s1.id);

        // count_search_scenarios mirrors search under every filter combo.
        assert_eq!(
            store
                .count_search_scenarios(Some("failure"), Some("chaos"), &[], Some(org_id))
                .await
                .unwrap(),
            1
        );
    }

    /// OSV vulnerability cache round-trip on SQLite: import an advisory,
    /// query it back by `(ecosystem, name, version)`, confirm version
    /// matching honours the "introduced=0 + no fixed = all versions"
    /// heuristic and exact-pin matches.
    #[tokio::test]
    async fn test_osv_cache_roundtrip() {
        use crate::models::osv::{OsvAffected, OsvImportRecord, OsvPackage, OsvSeverity};

        let store = memory_store().await;

        assert_eq!(store.count_osv_advisories().await.unwrap(), 0);

        // Advisory A: npm event-stream 3.3.6 (exact pin via versions[]).
        let advisory_a = OsvImportRecord {
            id: "GHSA-test-event-stream".to_string(),
            modified: Some("2018-11-26T00:00:00Z".to_string()),
            summary: Some("event-stream bitcoin wallet exfil".to_string()),
            details: None,
            affected: vec![OsvAffected {
                package: OsvPackage {
                    ecosystem: "npm".to_string(),
                    name: "event-stream".to_string(),
                },
                ranges: vec![],
                versions: vec!["3.3.6".to_string()],
            }],
            severity: vec![OsvSeverity {
                kind: "CVSS_V3".to_string(),
                score: "9.8".to_string(),
            }],
            database_specific: None,
        };

        // Advisory B: all-versions match via introduced=0 range.
        let advisory_b = OsvImportRecord {
            id: "GHSA-test-ctx".to_string(),
            modified: None,
            summary: Some("pypi ctx hijack".to_string()),
            details: None,
            affected: vec![OsvAffected {
                package: OsvPackage {
                    ecosystem: "PyPI".to_string(),
                    name: "ctx".to_string(),
                },
                ranges: vec![serde_json::json!({
                    "type": "ECOSYSTEM",
                    "events": [{"introduced": "0"}],
                })],
                versions: vec![],
            }],
            severity: vec![],
            database_specific: None,
        };

        assert_eq!(store.upsert_osv_advisory(&advisory_a).await.unwrap(), 1);
        assert_eq!(store.upsert_osv_advisory(&advisory_b).await.unwrap(), 1);
        assert_eq!(store.count_osv_advisories().await.unwrap(), 2);

        // Exact-pin match for advisory A.
        let hits = store.find_osv_matches("npm", "event-stream", "3.3.6").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].advisory_id, "GHSA-test-event-stream");
        assert_eq!(hits[0].severity, "critical");

        // Different version of the same package should not match.
        assert!(store.find_osv_matches("npm", "event-stream", "3.3.7").await.unwrap().is_empty());

        // Advisory B matches any version of pypi ctx.
        assert_eq!(store.find_osv_matches("pypi", "ctx", "0.2.2").await.unwrap().len(), 1);

        // Re-importing advisory A is idempotent — row count stays at 2.
        assert_eq!(store.upsert_osv_advisory(&advisory_a).await.unwrap(), 1);
        assert_eq!(store.count_osv_advisories().await.unwrap(), 2);
    }

    /// End-to-end round-trip of the plugin review + tag surface on SQLite.
    /// Seeds a plugin + author, attaches two tags, creates two reviews with
    /// different ratings + helpful counts, then exercises each reader
    /// (`get_plugin_reviews` ordering, `get_plugin_review_stats`,
    /// `get_plugin_review_distribution`, `find_existing_plugin_review`, and
    /// `increment_plugin_review_vote`). Mirrors what the registry search
    /// handler actually does per request.
    #[tokio::test]
    async fn test_plugin_reviews_and_tags_roundtrip() {
        let store = memory_store().await;
        let pool = store.pool();

        let author_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();
        for (uid, uname, email) in [
            (author_id, "alice-reviews", "alice-r@example.com"),
            (reviewer_id, "bob-reviews", "bob-r@example.com"),
        ] {
            sqlx::query(
                r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
                   VALUES (?, ?, ?, 'x', datetime('now'), datetime('now'))"#,
            )
            .bind(uid.to_string())
            .bind(uname)
            .bind(email)
            .execute(pool)
            .await
            .unwrap();
        }

        let plugin_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO plugins (id, name, description, current_version, category, license, author_id)
               VALUES (?, 'reviewed-plugin', 'demo', '1.0.0', 'other', 'MIT', ?)"#,
        )
        .bind(plugin_id.to_string())
        .bind(author_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        for tag in ["auth", "security"] {
            sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
                .bind(tag)
                .execute(pool)
                .await
                .unwrap();
            let (tag_id,): (i64,) = sqlx::query_as("SELECT id FROM tags WHERE name = ?")
                .bind(tag)
                .fetch_one(pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO plugin_tags (plugin_id, tag_id) VALUES (?, ?)")
                .bind(plugin_id.to_string())
                .bind(tag_id)
                .execute(pool)
                .await
                .unwrap();
        }

        let tags = store.get_plugin_tags(plugin_id).await.unwrap();
        assert_eq!(tags, vec!["auth".to_string(), "security".to_string()]);

        assert_eq!(store.count_plugin_reviews(plugin_id).await.unwrap(), 0);
        let (avg, cnt) = store.get_plugin_review_stats(plugin_id).await.unwrap();
        assert_eq!(cnt, 0);
        assert_eq!(avg, 0.0);
        assert!(store
            .find_existing_plugin_review(plugin_id, reviewer_id)
            .await
            .unwrap()
            .is_none());

        let r1 = store
            .create_plugin_review(
                plugin_id,
                reviewer_id,
                "1.0.0",
                5,
                Some("love it"),
                "great plugin, works well",
            )
            .await
            .unwrap();
        let r2 = store
            .create_plugin_review(
                plugin_id,
                author_id,
                "1.0.0",
                3,
                None,
                "self-review from the author for test coverage",
            )
            .await
            .unwrap();

        store.increment_plugin_review_vote(plugin_id, r1.id, true).await.unwrap();
        store.increment_plugin_review_vote(plugin_id, r1.id, true).await.unwrap();

        assert_eq!(store.count_plugin_reviews(plugin_id).await.unwrap(), 2);
        let (avg, cnt) = store.get_plugin_review_stats(plugin_id).await.unwrap();
        assert_eq!(cnt, 2);
        assert!((avg - 4.0).abs() < 1e-6, "expected avg 4.0, got {}", avg);

        let dist = store.get_plugin_review_distribution(plugin_id).await.unwrap();
        assert_eq!(dist.get(&5), Some(&1));
        assert_eq!(dist.get(&3), Some(&1));
        assert_eq!(dist.get(&4), None);

        let existing = store
            .find_existing_plugin_review(plugin_id, reviewer_id)
            .await
            .unwrap()
            .expect("reviewer's review is visible");
        assert_eq!(existing, r1.id);

        let listed = store.get_plugin_reviews(plugin_id, 10, 0).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, r1.id);
        assert_eq!(listed[0].rating, 5);
        assert_eq!(listed[0].helpful_count, 2);
        assert_eq!(listed[1].id, r2.id);
        assert_eq!(listed[1].rating, 3);

        store.update_plugin_rating_stats(plugin_id, avg, cnt as i32).await.unwrap();
        let (stored_avg, stored_cnt): (f64, i64) =
            sqlx::query_as("SELECT rating_avg, rating_count FROM plugins WHERE id = ?")
                .bind(plugin_id.to_string())
                .fetch_one(pool)
                .await
                .unwrap();
        assert!((stored_avg - 4.0).abs() < 1e-6);
        assert_eq!(stored_cnt, 2);

        let pi = store.get_user_public_info(reviewer_id).await.unwrap().expect("reviewer exists");
        assert_eq!(pi.1, "bob-reviews");
    }

    /// End-to-end round-trip of the plugin security scan surface on SQLite:
    /// seed a plugin + version, enqueue a `"pending"` scan, drain it via
    /// `list_pending_security_scans`, upsert a `"pass"` verdict, and read it
    /// back via `latest_security_scan_for_plugin`. Mirrors the shape of the
    /// Postgres-backed worker flow.
    #[tokio::test]
    async fn test_plugin_security_scan_roundtrip() {
        let store = memory_store().await;
        let pool = store.pool();

        // Seed a user/author.
        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
               VALUES (?, 'author', 'author@example.com', 'x', datetime('now'), datetime('now'))"#,
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let plugin_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO plugins (id, name, description, current_version, category, license, author_id)
               VALUES (?, 'demo-plugin', 'demo', '1.0.0', 'other', 'MIT', ?)"#,
        )
        .bind(plugin_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        let version_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO plugin_versions (id, plugin_id, version, download_url, checksum, file_size)
               VALUES (?, ?, '1.0.0', 'https://example.invalid/1.wasm', 'deadbeef', 42)"#,
        )
        .bind(version_id.to_string())
        .bind(plugin_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // Enqueue as pending, drain via the worker query, confirm the row we
        // get back carries all the context the scanner needs.
        let pending = serde_json::json!([{ "severity": "info", "title": "queued" }]);
        store
            .upsert_plugin_security_scan(version_id, "pending", 50, &pending, None)
            .await
            .unwrap();

        let jobs = store.list_pending_security_scans(10).await.unwrap();
        assert_eq!(jobs.len(), 1);
        let job = &jobs[0];
        assert_eq!(job.plugin_version_id, version_id);
        assert_eq!(job.plugin_name, "demo-plugin");
        assert_eq!(job.version, "1.0.0");
        assert_eq!(job.file_size, 42);
        assert_eq!(job.checksum, "deadbeef");

        // Overwrite with a verdict and confirm the next poll sees nothing.
        let verdict = serde_json::json!([]);
        store
            .upsert_plugin_security_scan(version_id, "pass", 95, &verdict, Some("test-1.0"))
            .await
            .unwrap();
        assert!(store.list_pending_security_scans(10).await.unwrap().is_empty());

        // Latest lookup should surface the pass row, and scanner_version must
        // have been updated by the upsert (not left at the earlier NULL).
        let latest = store
            .latest_security_scan_for_plugin(plugin_id)
            .await
            .unwrap()
            .expect("scan row present");
        assert_eq!(latest.status, "pass");
        assert_eq!(latest.score, 95);
        assert_eq!(latest.scanner_version.as_deref(), Some("test-1.0"));
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

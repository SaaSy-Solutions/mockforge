//! SSO (Single Sign-On) configuration model
//!
//! Supports SAML 2.0 SSO for Team plan organizations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// SSO provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SSOProvider {
    Saml,
    Oidc,
}

impl std::fmt::Display for SSOProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SSOProvider::Saml => write!(f, "saml"),
            SSOProvider::Oidc => write!(f, "oidc"),
        }
    }
}

impl SSOProvider {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "saml" => Some(SSOProvider::Saml),
            "oidc" => Some(SSOProvider::Oidc),
            _ => None,
        }
    }
}

/// SSO configuration for an organization
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SSOConfiguration {
    pub id: Uuid,
    pub org_id: Uuid,
    pub provider: String, // "saml" or "oidc"
    pub enabled: bool,

    // SAML 2.0 fields
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_slo_url: Option<String>,
    pub saml_x509_cert: Option<String>,
    pub saml_name_id_format: Option<String>,

    // OIDC fields (for future use)
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,

    // Email-domain discovery
    pub email_domain: Option<String>,

    // Attribute mapping
    pub attribute_mapping: serde_json::Value,

    // Security settings
    pub require_signed_assertions: bool,
    pub require_signed_responses: bool,
    pub allow_unsolicited_responses: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SSO session
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SSOSession {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub session_index: Option<String>,
    pub name_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl SSOConfiguration {
    /// Get provider as enum
    pub fn provider(&self) -> SSOProvider {
        SSOProvider::from_str(&self.provider).unwrap_or(SSOProvider::Saml)
    }

    /// Find SSO configuration by organization ID
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM sso_configurations WHERE org_id = $1")
            .bind(org_id)
            .fetch_optional(pool)
            .await
    }

    /// Create or update SSO configuration
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        pool: &sqlx::PgPool,
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
        oidc_issuer_url: Option<&str>,
        oidc_client_id: Option<&str>,
        oidc_client_secret: Option<&str>,
        email_domain: Option<&str>,
    ) -> sqlx::Result<Self> {
        let attribute_mapping = attribute_mapping.unwrap_or_else(|| serde_json::json!({}));

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sso_configurations (
                org_id, provider, saml_entity_id, saml_sso_url, saml_slo_url,
                saml_x509_cert, saml_name_id_format, attribute_mapping,
                require_signed_assertions, require_signed_responses, allow_unsolicited_responses,
                oidc_issuer_url, oidc_client_id, oidc_client_secret, email_domain
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (org_id) DO UPDATE SET
                provider = EXCLUDED.provider,
                saml_entity_id = EXCLUDED.saml_entity_id,
                saml_sso_url = EXCLUDED.saml_sso_url,
                saml_slo_url = EXCLUDED.saml_slo_url,
                saml_x509_cert = EXCLUDED.saml_x509_cert,
                saml_name_id_format = EXCLUDED.saml_name_id_format,
                attribute_mapping = EXCLUDED.attribute_mapping,
                require_signed_assertions = EXCLUDED.require_signed_assertions,
                require_signed_responses = EXCLUDED.require_signed_responses,
                allow_unsolicited_responses = EXCLUDED.allow_unsolicited_responses,
                oidc_issuer_url = EXCLUDED.oidc_issuer_url,
                oidc_client_id = EXCLUDED.oidc_client_id,
                oidc_client_secret = EXCLUDED.oidc_client_secret,
                email_domain = EXCLUDED.email_domain,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(provider.to_string())
        .bind(saml_entity_id)
        .bind(saml_sso_url)
        .bind(saml_slo_url)
        .bind(saml_x509_cert)
        .bind(saml_name_id_format)
        .bind(&attribute_mapping)
        .bind(require_signed_assertions)
        .bind(require_signed_responses)
        .bind(allow_unsolicited_responses)
        .bind(oidc_issuer_url)
        .bind(oidc_client_id)
        .bind(oidc_client_secret)
        .bind(email_domain)
        .fetch_one(pool)
        .await
    }

    /// Look up (org_slug, provider) by email domain for SSO discovery.
    ///
    /// Matches `lower(email_domain) = lower($1)` and `enabled = true` so
    /// only active configurations are returned.
    pub async fn find_org_slug_by_email_domain(
        pool: &sqlx::PgPool,
        domain: &str,
    ) -> sqlx::Result<Option<(String, String)>> {
        let row: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT o.slug, sc.provider
            FROM sso_configurations sc
            JOIN organizations o ON o.id = sc.org_id
            WHERE lower(sc.email_domain) = lower($1)
              AND sc.enabled = TRUE
            LIMIT 1
            "#,
        )
        .bind(domain)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Enable SSO for an organization
    pub async fn enable(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE sso_configurations SET enabled = TRUE, updated_at = NOW() WHERE org_id = $1",
        )
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Disable SSO for an organization
    pub async fn disable(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE sso_configurations SET enabled = FALSE, updated_at = NOW() WHERE org_id = $1",
        )
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete SSO configuration
    pub async fn delete(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM sso_configurations WHERE org_id = $1")
            .bind(org_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Extract the lowercased domain from an email address, or None if malformed.
pub fn normalize_email_domain(email: &str) -> Option<String> {
    let (_, domain) = email.split_once('@')?;
    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

#[cfg(feature = "postgres")]
impl SSOSession {
    /// Create a new SSO session
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
        session_index: Option<&str>,
        name_id: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sso_sessions (org_id, user_id, session_index, name_id, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(session_index)
        .bind(name_id)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    /// Find active session by org and user
    pub async fn find_active(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sso_sessions
            WHERE org_id = $1 AND user_id = $2 AND expires_at > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Delete expired sessions
    pub async fn cleanup_expired(pool: &sqlx::PgPool) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM sso_sessions WHERE expires_at < NOW()")
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete session by ID
    pub async fn delete(pool: &sqlx::PgPool, session_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM sso_sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_email_domain;

    #[test]
    fn email_domain_normalization() {
        assert_eq!(normalize_email_domain("Jo@Acme.com"), Some("acme.com".to_string()));
        assert_eq!(normalize_email_domain("no-at-sign"), None);
        assert_eq!(normalize_email_domain("a@"), None);
    }
}

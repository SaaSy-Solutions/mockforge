//! Startup bootstrap for site admins and billing-exempt organizations.
//!
//! Deploys run without anyone touching the production database, so these
//! flags are seeded from the deployment environment instead:
//!
//! - `MOCKFORGE_SITE_ADMIN_EMAILS`: comma-separated user emails (matched
//!   case-insensitively) that get `users.is_admin`.
//! - `MOCKFORGE_BILLING_EXEMPT_TENANT_SLUGS`: comma-separated organization
//!   slugs that get `organizations.billing_exempt`.
//!
//! The bootstrap only ever sets flags: it never clears one, an empty or
//! missing variable changes nothing, and a second run is a no-op. Clearing an
//! exemption goes through `PUT /api/v1/admin/organizations/{org_id}/billing-exempt`.
//! Every change, and every entry it could not match, is logged; failures
//! never stop startup.

use sqlx::PgPool;

pub const SITE_ADMIN_EMAILS_ENV: &str = "MOCKFORGE_SITE_ADMIN_EMAILS";
pub const BILLING_EXEMPT_SLUGS_ENV: &str = "MOCKFORGE_BILLING_EXEMPT_TENANT_SLUGS";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootstrapConfig {
    pub site_admin_emails: Vec<String>,
    pub billing_exempt_slugs: Vec<String>,
}

/// Split a comma-separated value, trimming entries and dropping empty or
/// repeated ones. A missing value is an empty list.
pub fn parse_list(value: Option<&str>) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    for entry in value.unwrap_or_default().split(',').map(str::trim) {
        if !entry.is_empty() && !entries.iter().any(|e| e == entry) {
            entries.push(entry.to_string());
        }
    }
    entries
}

impl BootstrapConfig {
    pub fn from_env() -> Self {
        Self {
            site_admin_emails: parse_list(std::env::var(SITE_ADMIN_EMAILS_ENV).ok().as_deref()),
            billing_exempt_slugs: parse_list(
                std::env::var(BILLING_EXEMPT_SLUGS_ENV).ok().as_deref(),
            ),
        }
    }
}

/// What one run did. Entries already flagged appear nowhere.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootstrapReport {
    pub site_admins_granted: Vec<String>,
    pub site_admins_not_found: Vec<String>,
    pub orgs_exempted: Vec<String>,
    pub orgs_not_found: Vec<String>,
}

impl BootstrapReport {
    pub fn changed_nothing(&self) -> bool {
        self.site_admins_granted.is_empty() && self.orgs_exempted.is_empty()
    }
}

/// Apply `config` on the owner pool, setting flags that are not yet set.
pub async fn apply(pool: &PgPool, config: &BootstrapConfig) -> sqlx::Result<BootstrapReport> {
    let mut report = BootstrapReport::default();

    for email in &config.site_admin_emails {
        let granted = sqlx::query(
            "UPDATE users SET is_admin = TRUE WHERE lower(email) = lower($1) AND NOT is_admin",
        )
        .bind(email)
        .execute(pool)
        .await?
        .rows_affected();
        if granted > 0 {
            report.site_admins_granted.push(email.clone());
            continue;
        }
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1))")
                .bind(email)
                .fetch_one(pool)
                .await?;
        if !exists {
            report.site_admins_not_found.push(email.clone());
        }
    }

    for slug in &config.billing_exempt_slugs {
        let exempted = sqlx::query(
            "UPDATE organizations SET billing_exempt = TRUE, updated_at = NOW() \
             WHERE slug = $1 AND NOT billing_exempt",
        )
        .bind(slug)
        .execute(pool)
        .await?
        .rows_affected();
        if exempted > 0 {
            report.orgs_exempted.push(slug.clone());
            continue;
        }
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)")
                .bind(slug)
                .fetch_one(pool)
                .await?;
        if !exists {
            report.orgs_not_found.push(slug.clone());
        }
    }

    Ok(report)
}

/// Run from startup, after migrations. Logs and never fails.
pub async fn run_at_startup(pool: &PgPool) {
    let config = BootstrapConfig::from_env();
    if config == BootstrapConfig::default() {
        return;
    }
    let report = match apply(pool, &config).await {
        Ok(report) => report,
        Err(e) => {
            tracing::error!(error = %e, "billing-exempt bootstrap failed; flags unchanged");
            return;
        }
    };
    for email in &report.site_admins_granted {
        tracing::info!(email = email.as_str(), "bootstrap: granted site admin");
    }
    for email in &report.site_admins_not_found {
        tracing::warn!(email = email.as_str(), "bootstrap: no user with this site-admin email");
    }
    for slug in &report.orgs_exempted {
        tracing::info!(slug = slug.as_str(), "bootstrap: organization is now billing-exempt");
    }
    for slug in &report.orgs_not_found {
        tracing::warn!(slug = slug.as_str(), "bootstrap: no organization with this slug");
    }
    if report.changed_nothing() {
        tracing::info!("bootstrap: site admins and billing-exempt orgs already up to date");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_trims_and_skips_empty_entries() {
        assert!(parse_list(None).is_empty());
        assert!(parse_list(Some("")).is_empty());
        assert!(parse_list(Some(" , ,")).is_empty());
        assert_eq!(
            parse_list(Some(" admin@example.invalid ,ops@example.invalid,admin@example.invalid,")),
            vec!["admin@example.invalid", "ops@example.invalid"]
        );
    }
}

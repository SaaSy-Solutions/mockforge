//! Hosted Mock deployment models
//!
//! Handles cloud-hosted mock service deployments with lifecycle management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Hosted mock deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Pending,
    Deploying,
    Active,
    Stopped,
    Failed,
    Deleting,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStatus::Pending => write!(f, "pending"),
            DeploymentStatus::Deploying => write!(f, "deploying"),
            DeploymentStatus::Active => write!(f, "active"),
            DeploymentStatus::Stopped => write!(f, "stopped"),
            DeploymentStatus::Failed => write!(f, "failed"),
            DeploymentStatus::Deleting => write!(f, "deleting"),
        }
    }
}

impl DeploymentStatus {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(DeploymentStatus::Pending),
            "deploying" => Some(DeploymentStatus::Deploying),
            "active" => Some(DeploymentStatus::Active),
            "stopped" => Some(DeploymentStatus::Stopped),
            "failed" => Some(DeploymentStatus::Failed),
            "deleting" => Some(DeploymentStatus::Deleting),
            _ => None,
        }
    }
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl HealthStatus {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "healthy" => Some(HealthStatus::Healthy),
            "unhealthy" => Some(HealthStatus::Unhealthy),
            "unknown" => Some(HealthStatus::Unknown),
            _ => None,
        }
    }
}

/// Protocols that can be enabled on a hosted mock deployment.
///
/// Tracks two things at once: what protocol crate the deployed
/// `mockforge-cli` will serve, and how Fly.io should expose it. Some
/// protocols ride on the HTTP port (WS upgrade / GraphQL POST) so the Fly
/// service config doesn't change; others need their own TCP service entry
/// and may need TLS handlers.
///
/// Plan gating in [`Protocol::min_plan`] keeps Free deployments to HTTP-only,
/// Pro adds gRPC, and Team unlocks the full broker set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    #[serde(alias = "ws")]
    WebSocket,
    #[serde(alias = "gql")]
    GraphQL,
    #[serde(alias = "grpc")]
    Grpc,
    Smtp,
    Mqtt,
    Kafka,
    Amqp,
    Tcp,
}

impl Protocol {
    /// Lowest plan tier that may enable this protocol.
    pub fn min_plan(self) -> &'static str {
        match self {
            Protocol::Http | Protocol::WebSocket | Protocol::GraphQL => "free",
            Protocol::Grpc => "pro",
            Protocol::Smtp | Protocol::Mqtt | Protocol::Kafka | Protocol::Amqp | Protocol::Tcp => {
                "team"
            }
        }
    }

    /// Internal port the protocol binds inside the container.
    /// `None` for protocols that share the HTTP listener (handled by the
    /// in-process router merge done in mockforge-cli's `serve` command).
    pub fn internal_port(self) -> Option<u16> {
        match self {
            Protocol::Http | Protocol::WebSocket | Protocol::GraphQL => None,
            Protocol::Grpc => Some(50051),
            Protocol::Smtp => Some(1025),
            Protocol::Mqtt => Some(1883),
            Protocol::Kafka => Some(9092),
            Protocol::Amqp => Some(5672),
            Protocol::Tcp => Some(9999),
        }
    }

    /// Public port to expose on Fly. Same as `internal_port` by default;
    /// SMTP runs on 2525 publicly because many ISPs block outbound 25.
    pub fn public_port(self) -> Option<u16> {
        match self {
            Protocol::Smtp => Some(2525),
            other => other.internal_port(),
        }
    }

    /// Fly TLS handlers to attach. `["tls"]` for binary protocols that
    /// terminate TLS at Fly's edge; `[]` for plain TCP; `["h2"]` for gRPC.
    pub fn fly_handlers(self) -> &'static [&'static str] {
        match self {
            Protocol::Grpc => &["tls", "h2"],
            Protocol::Smtp | Protocol::Mqtt | Protocol::Amqp => &["tls"],
            Protocol::Kafka | Protocol::Tcp => &[],
            Protocol::Http | Protocol::WebSocket | Protocol::GraphQL => &[],
        }
    }

    /// Env var pair set on the Fly machine to enable this protocol in the
    /// `mockforge-cli serve` runtime. `None` for protocols that are always
    /// on (HTTP) or merged into HTTP (WS/GraphQL).
    pub fn enable_env(self) -> Option<(&'static str, String)> {
        match self {
            Protocol::Grpc => Some(("MOCKFORGE_GRPC_ENABLED", "true".to_string())),
            Protocol::Smtp => Some(("MOCKFORGE_SMTP_ENABLED", "true".to_string())),
            Protocol::Mqtt => Some(("MOCKFORGE_MQTT_ENABLED", "true".to_string())),
            Protocol::Kafka => Some(("MOCKFORGE_KAFKA_ENABLED", "true".to_string())),
            Protocol::Amqp => Some(("MOCKFORGE_AMQP_ENABLED", "true".to_string())),
            Protocol::Tcp => Some(("MOCKFORGE_TCP_ENABLED", "true".to_string())),
            Protocol::Http | Protocol::WebSocket | Protocol::GraphQL => None,
        }
    }
}

/// `true` if every protocol in `requested` is allowed on the given plan.
/// Plan ordering is `free < pro < team`.
pub fn protocols_allowed_on_plan(requested: &[Protocol], plan: &str) -> bool {
    let plan_rank = plan_rank(plan);
    requested.iter().all(|p| plan_rank >= plan_rank_str(p.min_plan()))
}

fn plan_rank(plan: &str) -> u8 {
    plan_rank_str(plan)
}

fn plan_rank_str(plan: &str) -> u8 {
    match plan {
        "team" | "enterprise" => 3,
        "pro" => 2,
        "free" => 1,
        _ => 1,
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[test]
    fn http_is_free_tier() {
        assert_eq!(Protocol::Http.min_plan(), "free");
        assert_eq!(Protocol::WebSocket.min_plan(), "free");
        assert_eq!(Protocol::GraphQL.min_plan(), "free");
    }

    #[test]
    fn grpc_is_pro_tier() {
        assert_eq!(Protocol::Grpc.min_plan(), "pro");
    }

    #[test]
    fn brokers_are_team_tier() {
        for p in [
            Protocol::Smtp,
            Protocol::Mqtt,
            Protocol::Kafka,
            Protocol::Amqp,
            Protocol::Tcp,
        ] {
            assert_eq!(p.min_plan(), "team", "{:?} should be team-tier", p);
        }
    }

    #[test]
    fn plan_gate_rejects_lower_tier() {
        // Free user requesting gRPC → blocked.
        assert!(!protocols_allowed_on_plan(&[Protocol::Grpc], "free"));
        // Free user with HTTP only → fine.
        assert!(protocols_allowed_on_plan(&[Protocol::Http], "free"));
        // Pro user with gRPC → fine.
        assert!(protocols_allowed_on_plan(&[Protocol::Http, Protocol::Grpc], "pro"));
        // Pro user with Kafka (team-only) → blocked.
        assert!(!protocols_allowed_on_plan(&[Protocol::Kafka], "pro"));
        // Team user with everything → fine.
        assert!(protocols_allowed_on_plan(
            &[
                Protocol::Http,
                Protocol::Grpc,
                Protocol::Kafka,
                Protocol::Smtp
            ],
            "team"
        ));
    }

    #[test]
    fn http_merged_protocols_have_no_internal_port() {
        assert!(Protocol::Http.internal_port().is_none());
        assert!(Protocol::WebSocket.internal_port().is_none());
        assert!(Protocol::GraphQL.internal_port().is_none());
    }

    #[test]
    fn smtp_public_port_dodges_isp_blocks() {
        // Public port should be 2525 (not 25/465) to avoid outbound-25 blocks.
        assert_eq!(Protocol::Smtp.public_port(), Some(2525));
        // Internal port stays 1025 because that's what mockforge-smtp binds.
        assert_eq!(Protocol::Smtp.internal_port(), Some(1025));
    }

    #[test]
    fn grpc_uses_h2_handler() {
        assert!(Protocol::Grpc.fly_handlers().contains(&"h2"));
        assert!(Protocol::Grpc.fly_handlers().contains(&"tls"));
    }

    #[test]
    fn enable_env_skips_http_family() {
        assert!(Protocol::Http.enable_env().is_none());
        assert!(Protocol::WebSocket.enable_env().is_none());
        assert!(Protocol::GraphQL.enable_env().is_none());
        assert_eq!(
            Protocol::Grpc.enable_env(),
            Some(("MOCKFORGE_GRPC_ENABLED", "true".to_string()))
        );
    }
}

/// Hosted mock deployment
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HostedMock {
    pub id: Uuid,
    pub org_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub config_json: serde_json::Value,
    pub openapi_spec_url: Option<String>,
    pub status: String, // Stored as VARCHAR, converted via methods
    pub deployment_url: Option<String>,
    pub internal_url: Option<String>,
    pub region: String,
    pub instance_type: String,
    pub health_check_url: Option<String>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub health_status: String, // Stored as VARCHAR, converted via methods
    pub error_message: Option<String>,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl HostedMock {
    /// Get status as enum
    pub fn status(&self) -> DeploymentStatus {
        DeploymentStatus::from_str(&self.status).unwrap_or(DeploymentStatus::Pending)
    }

    /// Get health status as enum
    pub fn health_status(&self) -> HealthStatus {
        HealthStatus::from_str(&self.health_status).unwrap_or(HealthStatus::Unknown)
    }

    /// Protocols enabled on this deployment. Persisted inside `config_json`
    /// (under the `"enabled_protocols"` key) so we don't need a schema
    /// migration on first land. Defaults to `[Protocol::Http]` when missing
    /// or malformed — every deployment gets HTTP, and that's the only
    /// protocol guaranteed today.
    pub fn enabled_protocols(&self) -> Vec<Protocol> {
        self.config_json
            .get("enabled_protocols")
            .and_then(|v| serde_json::from_value::<Vec<Protocol>>(v.clone()).ok())
            .filter(|v| !v.is_empty())
            .map(|mut v| {
                if !v.contains(&Protocol::Http) {
                    v.insert(0, Protocol::Http);
                }
                v
            })
            .unwrap_or_else(|| vec![Protocol::Http])
    }

    /// Optional upstream URL the deployment proxies to when the reality
    /// slider is > 0 (#222). Persisted inside `config_json` under the
    /// `"upstream_url"` key so we don't need a schema migration.
    /// Returns `None` when no upstream is configured — in that case the
    /// reality slider is a no-op and responses always come from the mock.
    pub fn upstream_url(&self) -> Option<String> {
        self.config_json
            .get("upstream_url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// Compute the Fly.io app name used at deploy time.
    ///
    /// Mirrors the format built in
    /// `mockforge-registry-server::deployment::orchestrator` — the orchestrator
    /// is the single source of truth for the on-wire name, but we need the
    /// same value in other subsystems (e.g. Fly Prometheus metric queries in
    /// `fly_metrics`). Keep this method in lockstep with the orchestrator.
    pub fn fly_app_name(&self) -> String {
        format!(
            "mockforge-{}-{}",
            self.org_id.to_string().replace('-', "").chars().take(8).collect::<String>(),
            self.slug
        )
    }

    /// Create a new hosted mock deployment
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        project_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: Option<&str>,
        config_json: serde_json::Value,
        openapi_spec_url: Option<&str>,
        region: Option<&str>,
    ) -> sqlx::Result<Self> {
        let region = region.unwrap_or("iad");
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO hosted_mocks (
                org_id, project_id, name, slug, description,
                config_json, openapi_spec_url, region, status, health_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', 'unknown')
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(project_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(config_json)
        .bind(openapi_spec_url)
        .bind(region)
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM hosted_mocks WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find by slug and org
    pub async fn find_by_slug(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE org_id = $1 AND slug = $2 AND deleted_at IS NULL",
        )
        .bind(org_id)
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    /// Find an active deployment by slug (across all orgs).
    /// Used for custom domain routing where only the slug is known from the hostname.
    pub async fn find_active_by_slug(
        pool: &sqlx::PgPool,
        slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE slug = $1 AND status = 'active' AND deleted_at IS NULL LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    /// Find all mocks for an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE org_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Find all mocks for a project
    pub async fn find_by_project(pool: &sqlx::PgPool, project_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE project_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    /// Update deployment status
    pub async fn update_status(
        pool: &sqlx::PgPool,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET status = $1, error_message = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(status.to_string())
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update deployment URLs
    pub async fn update_urls(
        pool: &sqlx::PgPool,
        id: Uuid,
        deployment_url: Option<&str>,
        internal_url: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET deployment_url = $1, internal_url = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(deployment_url)
        .bind(internal_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update health status
    pub async fn update_health(
        pool: &sqlx::PgPool,
        id: Uuid,
        health_status: HealthStatus,
        health_check_url: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET health_status = $1, health_check_url = $2, last_health_check = NOW(), updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(health_status.to_string())
        .bind(health_check_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Soft delete (mark as deleted)
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE hosted_mocks SET deleted_at = NOW(), status = 'deleting', updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

/// Deployment log entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DeploymentLog {
    pub id: Uuid,
    pub hosted_mock_id: Uuid,
    pub level: String,
    pub message: String,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl DeploymentLog {
    /// Create a new log entry
    pub async fn create(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
        level: &str,
        message: &str,
        metadata: Option<serde_json::Value>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO deployment_logs (hosted_mock_id, level, message, metadata_json)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(hosted_mock_id)
        .bind(level)
        .bind(message)
        .bind(metadata.unwrap_or_else(|| serde_json::json!({})))
        .fetch_one(pool)
        .await
    }

    /// Get logs for a deployment
    pub async fn find_by_mock(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
        limit: Option<i64>,
    ) -> sqlx::Result<Vec<Self>> {
        let limit = limit.unwrap_or(100);
        sqlx::query_as::<_, Self>(
            "SELECT * FROM deployment_logs WHERE hosted_mock_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(hosted_mock_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Deployment metrics
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    pub id: Uuid,
    pub hosted_mock_id: Uuid,
    pub period_start: chrono::NaiveDate,
    pub requests: i64,
    pub requests_2xx: i64,
    pub requests_4xx: i64,
    pub requests_5xx: i64,
    pub egress_bytes: i64,
    pub avg_response_time_ms: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl DeploymentMetrics {
    /// Get or create metrics for current period
    pub async fn get_or_create_current(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
    ) -> sqlx::Result<Self> {
        use chrono::Datelike;
        let now = chrono::Utc::now().date_naive();
        let period_start =
            chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);

        // Try to get existing
        if let Some(metrics) = sqlx::query_as::<_, Self>(
            "SELECT * FROM deployment_metrics WHERE hosted_mock_id = $1 AND period_start = $2",
        )
        .bind(hosted_mock_id)
        .bind(period_start)
        .fetch_optional(pool)
        .await?
        {
            return Ok(metrics);
        }

        // Create new
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO deployment_metrics (hosted_mock_id, period_start)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(hosted_mock_id)
        .bind(period_start)
        .fetch_one(pool)
        .await
    }

    /// Increment request counters
    pub async fn increment_requests(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
        status_code: u16,
        response_time_ms: u64,
    ) -> sqlx::Result<()> {
        let metrics = Self::get_or_create_current(pool, hosted_mock_id).await?;

        let (increment_2xx, increment_4xx, increment_5xx) = if (200..300).contains(&status_code) {
            (1, 0, 0)
        } else if (400..500).contains(&status_code) {
            (0, 1, 0)
        } else if status_code >= 500 {
            (0, 0, 1)
        } else {
            (0, 0, 0)
        };

        // Update average response time (simple moving average)
        let new_avg = if metrics.requests > 0 {
            ((metrics.avg_response_time_ms as f64 * metrics.requests as f64
                + response_time_ms as f64)
                / (metrics.requests + 1) as f64) as i64
        } else {
            response_time_ms as i64
        };

        sqlx::query(
            r#"
            UPDATE deployment_metrics
            SET
                requests = requests + 1,
                requests_2xx = requests_2xx + $1,
                requests_4xx = requests_4xx + $2,
                requests_5xx = requests_5xx + $3,
                avg_response_time_ms = $4,
                updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(increment_2xx)
        .bind(increment_4xx)
        .bind(increment_5xx)
        .bind(new_avg)
        .bind(metrics.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increment egress bytes
    pub async fn increment_egress(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
        bytes: i64,
    ) -> sqlx::Result<()> {
        let metrics = Self::get_or_create_current(pool, hosted_mock_id).await?;

        sqlx::query(
            r#"
            UPDATE deployment_metrics
            SET egress_bytes = egress_bytes + $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(bytes)
        .bind(metrics.id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

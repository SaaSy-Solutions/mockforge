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

impl DeploymentStatus {
    pub fn to_string(&self) -> String {
        match self {
            DeploymentStatus::Pending => "pending".to_string(),
            DeploymentStatus::Deploying => "deploying".to_string(),
            DeploymentStatus::Active => "active".to_string(),
            DeploymentStatus::Stopped => "stopped".to_string(),
            DeploymentStatus::Failed => "failed".to_string(),
            DeploymentStatus::Deleting => "deleting".to_string(),
        }
    }

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

impl HealthStatus {
    pub fn to_string(&self) -> String {
        match self {
            HealthStatus::Healthy => "healthy".to_string(),
            HealthStatus::Unhealthy => "unhealthy".to_string(),
            HealthStatus::Unknown => "unknown".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "healthy" => Some(HealthStatus::Healthy),
            "unhealthy" => Some(HealthStatus::Unhealthy),
            "unknown" => Some(HealthStatus::Unknown),
            _ => None,
        }
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

impl HostedMock {
    /// Get status as enum
    pub fn status(&self) -> DeploymentStatus {
        DeploymentStatus::from_str(&self.status).unwrap_or(DeploymentStatus::Pending)
    }

    /// Get health status as enum
    pub fn health_status(&self) -> HealthStatus {
        HealthStatus::from_str(&self.health_status).unwrap_or(HealthStatus::Unknown)
    }

    /// Create a new hosted mock deployment
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        project_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: Option<&str>,
        config_json: serde_json::Value,
        openapi_spec_url: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO hosted_mocks (
                org_id, project_id, name, slug, description,
                config_json, openapi_spec_url, status, health_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', 'unknown')
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
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE id = $1 AND deleted_at IS NULL",
        )
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

    /// Find all mocks for an organization
    pub async fn find_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM hosted_mocks WHERE org_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Find all mocks for a project
    pub async fn find_by_project(
        pool: &sqlx::PgPool,
        project_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
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

impl DeploymentMetrics {
    /// Get or create metrics for current period
    pub async fn get_or_create_current(
        pool: &sqlx::PgPool,
        hosted_mock_id: Uuid,
    ) -> sqlx::Result<Self> {
        let period_start = chrono::Utc::now().date_naive();
        let period_start = period_start.with_day(1).unwrap_or(period_start);

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

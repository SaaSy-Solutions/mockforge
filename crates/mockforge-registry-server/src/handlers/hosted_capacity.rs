//! Serialize hosted-mock admission against the owning organization.
//!
//! Pending deployments hold a slot before the background deployer sees them.
//! Creation and restart take the same organization-row lock, so concurrent
//! callers cannot each pass a stale count and exceed the plan's capacity.

use chrono::Utc;
use serde_json::Value;
use sqlx::{Connection, PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult, StoreResult},
    models::{HostedMock, OrgSetting, Organization, Plan, Subscription},
    store::with_org_context,
    AppState,
};

use super::{entitlements::resolve_effective_plan, hosted_mocks::CreateDeploymentRequest};

async fn has_capacity(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    existing: Option<Uuid>,
) -> StoreResult<bool> {
    let org =
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1 FOR UPDATE")
            .bind(org_id)
            .fetch_one(&mut **tx)
            .await?;
    let subscription = Subscription::find_by_org(&mut **tx, org_id).await?;
    let plan = resolve_effective_plan(org.plan(), subscription.as_ref(), Utc::now());
    let mut limits = if plan == org.plan() {
        org.limits_json
    } else {
        plan.default_limits()
    };
    if let Some(setting) = OrgSetting::get(&mut **tx, org_id, "quota").await? {
        super::usage::merge_quota_overrides(&mut limits, &setting.setting_value);
    }
    if limits.get("hosted_mocks").and_then(Value::as_bool) == Some(false) {
        return Ok(false);
    }
    let maximum = limits.get("max_hosted_mocks").and_then(Value::as_i64).unwrap_or(0);
    let org_has_capacity = if maximum < 0 {
        true
    } else {
        let occupied: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM hosted_mocks WHERE org_id = $1 AND capacity_reserved AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(org_id)
    .bind(existing)
    .fetch_one(&mut **tx)
    .await?;
        occupied < maximum
    };
    if !org_has_capacity {
        return Ok(false);
    }
    if plan == Plan::Free {
        return Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,$2)")
            .bind(org_id)
            .bind(existing)
            .fetch_one(&mut **tx)
            .await?);
    }
    Ok(true)
}

fn limit_error() -> ApiError {
    ApiError::ResourceLimitExceeded(
        "Hosted mock limit reached. Delete an existing deployment and wait for cleanup, or upgrade your plan.".into(),
    )
}

pub(super) async fn create(
    state: &AppState,
    org_id: Uuid,
    request: &CreateDeploymentRequest,
    slug: &str,
    config: Value,
) -> ApiResult<HostedMock> {
    let org = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(Organization::find_by_id(&mut **tx, org_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;
    super::entitlements::require_verified_free_owner(state, &org).await?;
    let deployment = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            if !has_capacity(tx, org_id, None).await? {
                return Ok(None);
            }
            HostedMock::create_scoped(
                &mut **tx,
                mockforge_registry_core::models::hosted_mock::HostedMockCreation {
                    org_id,
                    project_id: request.project_id,
                    workspace_id: request.workspace_id,
                    name: &request.name,
                    slug,
                    description: request.description.as_deref(),
                    config_json: config,
                    openapi_spec_url: request.openapi_spec_url.as_deref(),
                    region: request.region.as_deref(),
                },
            )
            .await
            .map(Some)
            .map_err(Into::into)
        })
    })
    .await
    .map_err(|error| {
        if let crate::error::StoreError::Database(ref database) = error {
            if database.as_database_error().and_then(|error| error.constraint())
                == Some("hosted_mocks_project_org_fkey")
            {
                return ApiError::InvalidRequest("Project not found in this organization".into());
            }
            if database.as_database_error().and_then(|error| error.code()).as_deref()
                == Some("23505")
            {
                return ApiError::Conflict(
                    "Hosted slug is unavailable. Choose a different slug.".into(),
                );
            }
        }
        ApiError::from(error)
    })?;
    deployment.ok_or_else(limit_error)
}

/// A dedicated connection owns the session lock for the entire provider
/// operation. It is detached from the pool so cancellation closes the socket
/// instead of returning a still-locked connection to another request.
pub(crate) struct LifecycleGuard {
    connection: PgConnection,
    org_id: Uuid,
    deployment_id: Uuid,
}

impl LifecycleGuard {
    pub(crate) async fn acquire(
        pool: &PgPool,
        org_id: Uuid,
        deployment_id: Uuid,
    ) -> ApiResult<Self> {
        let mut connection = pool.acquire().await?.detach();
        sqlx::query("SELECT set_config('app.current_org_id', $1, false)")
            .bind(org_id.to_string())
            .execute(&mut connection)
            .await?;
        let acquired: Option<bool> = sqlx::query_scalar("SELECT pg_try_advisory_lock($1, hashtext($2)) FROM hosted_mocks WHERE id = $3 AND org_id = $4")
            .bind(0x4d46_i32)
            .bind(deployment_id.to_string())
            .bind(deployment_id)
            .bind(org_id)
            .fetch_optional(&mut connection)
            .await?;
        let acquired =
            acquired.ok_or_else(|| ApiError::InvalidRequest("Deployment not found".into()))?;
        if !acquired {
            return Err(ApiError::Conflict(
                "A deployment operation is already in progress. Retry shortly.".into(),
            ));
        }
        Ok(Self {
            connection,
            org_id,
            deployment_id,
        })
    }

    pub(crate) async fn find(&mut self) -> ApiResult<Option<HostedMock>> {
        Ok(self
            .find_for_cleanup()
            .await?
            .filter(|deployment| deployment.deleted_at.is_none()))
    }

    pub(crate) async fn find_for_cleanup(&mut self) -> ApiResult<Option<HostedMock>> {
        Ok(sqlx::query_as::<_, HostedMock>(
            "SELECT * FROM hosted_mocks WHERE id = $1 AND org_id = $2",
        )
        .bind(self.deployment_id)
        .bind(self.org_id)
        .fetch_optional(&mut self.connection)
        .await?)
    }

    pub(crate) async fn deletion_requested(&mut self) -> ApiResult<bool> {
        Ok(sqlx::query_scalar("SELECT deletion_requested_at IS NOT NULL FROM hosted_mocks WHERE id = $1 AND org_id = $2")
            .bind(self.deployment_id).bind(self.org_id).fetch_one(&mut self.connection).await?)
    }

    pub(crate) async fn ensure_not_deleting(&mut self) -> ApiResult<()> {
        if self.deletion_requested().await? {
            return Err(ApiError::Conflict("Deployment is awaiting cleanup".into()));
        }
        Ok(())
    }

    /// Only an authorized delete operation may create this durable intent;
    /// changing the display status cannot enqueue destructive cleanup.
    pub(crate) async fn request_deletion(&mut self) -> ApiResult<()> {
        sqlx::query("UPDATE hosted_mocks SET status = 'deleting', deletion_requested_at = COALESCE(deletion_requested_at, NOW()), updated_at = NOW() WHERE id = $1 AND org_id = $2")
            .bind(self.deployment_id).bind(self.org_id).execute(&mut self.connection).await?;
        Ok(())
    }

    /// Persist the reservation before any provider call. Cancellation cannot
    /// roll it back and falsely free a slot after an uncertain remote start.
    pub(crate) async fn reserve_restart(&mut self) -> ApiResult<()> {
        let mut tx = self.connection.begin().await?;
        if !has_capacity(&mut tx, self.org_id, Some(self.deployment_id)).await? {
            return Err(limit_error());
        }
        let changed = sqlx::query(
            "UPDATE hosted_mocks SET status = 'pending', capacity_reserved = TRUE, updated_at = NOW() WHERE id = $1 AND org_id = $2 AND status IN ('stopped', 'pending') AND deleted_at IS NULL AND deletion_requested_at IS NULL AND deployment_url IS NOT NULL",
        )
        .bind(self.deployment_id)
        .bind(self.org_id)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(ApiError::Conflict(
                "Deployment is no longer stopped or awaiting restart".into(),
            ));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Hide the deployment and release capacity only after the caller confirms
    /// provider app absence or the registry-recorded local backend. The held
    /// operation lock prevents a competing provision/start during cleanup.
    pub(crate) async fn complete_deletion(&mut self) -> ApiResult<()> {
        let changed = sqlx::query(
            "UPDATE hosted_mocks SET status = 'deleting', deleted_at = COALESCE(deleted_at, NOW()), capacity_reserved = FALSE, updated_at = NOW() WHERE id = $1 AND org_id = $2 AND deletion_requested_at IS NOT NULL AND (capacity_reserved OR deleted_at IS NULL)",
        )
        .bind(self.deployment_id)
        .bind(self.org_id)
        .execute(&mut self.connection)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(ApiError::Conflict("Deployment changed during cleanup".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod lifecycle_lock_tests {
    use super::*;
    use crate::{
        database::Database,
        models::Plan,
        store::{PgRegistryStore, RegistryStore},
    };

    async fn fixture() -> (Database, Uuid, Uuid) {
        let url = std::env::var("TEST_DATABASE_URL").expect("isolated Postgres required");
        let db = Database::connect(&url).await.unwrap();
        db.migrate().await.unwrap();
        let store = PgRegistryStore::new(db.pool().clone());
        let tag = Uuid::new_v4().simple().to_string();
        let user = store
            .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-test-account")
            .await
            .unwrap();
        let org = store
            .create_organization("Lock fixture", &tag, user.id, Plan::Free)
            .await
            .unwrap();
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, deployment_url, capacity_reserved) VALUES ($1, $2, 'Lock fixture', $3, '{}'::jsonb, 'stopped', 'https://fixture.example.invalid', FALSE)")
            .bind(id).bind(org.id).bind(id.simple().to_string()).execute(db.pool()).await.unwrap();
        (db, org.id, id)
    }

    async fn acquire_after_close(pool: &PgPool, org: Uuid, id: Uuid) -> LifecycleGuard {
        for _ in 0..100 {
            match LifecycleGuard::acquire(pool, org, id).await {
                Ok(guard) => return guard,
                Err(ApiError::Conflict(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await
                }
                Err(error) => panic!("unexpected lock error: {error}"),
            }
        }
        panic!("canceled operation retained its database session lock");
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres with NOBYPASSRLS APP_DATABASE_URL"]
    async fn overlapping_operations_conflict_and_dropped_guards_do_not_leak_locks() {
        let (db, org, id) = fixture().await;
        let mut guard = LifecycleGuard::acquire(db.runtime_pool(), org, id).await.unwrap();
        assert!(guard.find().await.unwrap().is_some());
        assert!(matches!(
            LifecycleGuard::acquire(db.runtime_pool(), org, id).await,
            Err(ApiError::Conflict(_))
        ));
        assert!(matches!(
            LifecycleGuard::acquire(db.runtime_pool(), Uuid::new_v4(), id).await,
            Err(ApiError::InvalidRequest(_))
        ));
        drop(guard);
        let mut next = acquire_after_close(db.runtime_pool(), org, id).await;
        assert!(next.find().await.unwrap().is_some());
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres with NOBYPASSRLS APP_DATABASE_URL"]
    async fn canceling_a_restart_releases_the_operation_lock_but_keeps_its_reservation() {
        let (db, org, id) = fixture().await;
        let pool = db.runtime_pool().clone();
        let (ready, observed) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut guard = LifecycleGuard::acquire(&pool, org, id).await.unwrap();
            guard.reserve_restart().await.unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
            drop(guard);
        });
        observed.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let mut next = acquire_after_close(db.runtime_pool(), org, id).await;
        assert_eq!(
            next.find().await.unwrap().unwrap().status(),
            crate::models::DeploymentStatus::Pending
        );
        let reserved: bool =
            sqlx::query_scalar("SELECT capacity_reserved FROM hosted_mocks WHERE id = $1")
                .bind(id)
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert!(reserved, "a canceled remote start must not release its committed slot");
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres with NOBYPASSRLS APP_DATABASE_URL"]
    async fn editing_display_status_cannot_cancel_an_authorized_deletion_intent() {
        let (db, org, id) = fixture().await;
        let mut guard = LifecycleGuard::acquire(db.runtime_pool(), org, id).await.unwrap();
        guard.request_deletion().await.unwrap();
        sqlx::query("UPDATE hosted_mocks SET status = 'stopped' WHERE id = $1")
            .bind(id)
            .execute(db.pool())
            .await
            .unwrap();
        assert!(guard.deletion_requested().await.unwrap());
        assert!(matches!(guard.ensure_not_deleting().await, Err(ApiError::Conflict(_))));
        assert!(matches!(guard.reserve_restart().await, Err(ApiError::Conflict(_))));
        let status: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id = $1")
            .bind(id)
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(status, "stopped");
    }
}

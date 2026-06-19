//! Transaction-scoped tenant context for the Postgres Row-Level-Security
//! backstop (#832).
//!
//! The RLS policies added in migration `20250101000079_rls_tenant_isolation`
//! constrain every org-scoped row to the org named by the
//! `app.current_org_id` GUC on the current connection. This module provides
//! [`with_org_context`], the single chokepoint that binds that GUC for the
//! lifetime of a transaction so handler queries are isolated even if they
//! forget their `WHERE org_id` clause.
//!
//! ## Why a transaction
//!
//! `set_config(key, value, is_local => true)` makes the GUC **transaction
//! local**: it is automatically reset when the transaction commits or rolls
//! back. That is essential with a pooled connection — without `is_local` the
//! setting would leak onto the next checkout of the same physical connection
//! and bind the *next* request to the *previous* request's org. So every unit
//! of work that relies on RLS must run inside a transaction that first sets
//! the GUC.
//!
//! ## Usage
//!
//! ```ignore
//! let projects = with_org_context(pool, org_id, |tx| {
//!     Box::pin(async move {
//!         // No `WHERE org_id` needed for correctness — RLS enforces it.
//!         // We keep it anyway as defense-in-depth at the call sites.
//!         let rows = sqlx::query_as::<_, (Uuid, String)>(
//!             "SELECT id, name FROM projects",
//!         )
//!         .fetch_all(&mut **tx)
//!         .await?;
//!         Ok(rows)
//!     })
//! })
//! .await?;
//! ```

use std::future::Future;
use std::pin::Pin;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{StoreError, StoreResult};

/// Run `f` inside a transaction that has `app.current_org_id` bound to
/// `org_id`, then commit.
///
/// The GUC is set with `set_config(..., is_local => true)` so it is scoped to
/// the transaction and never leaks back into the connection pool. Any error
/// returned by `f` aborts the transaction (rolled back on drop) and is
/// propagated; on success the transaction is committed and the value returned.
///
/// Callers that need cross-org access (platform admin, background sweeps,
/// migrations) must NOT route through this helper — they run as an elevated
/// role or manage the GUC themselves. See the migration header for the role
/// requirements.
pub async fn with_org_context<'a, T, F>(pool: &'a PgPool, org_id: Uuid, f: F) -> StoreResult<T>
where
    F: for<'t> FnOnce(
        &'t mut Transaction<'a, Postgres>,
    ) -> Pin<Box<dyn Future<Output = StoreResult<T>> + Send + 't>>,
{
    let mut tx = pool.begin().await.map_err(StoreError::Database)?;

    // Bind the tenant for the lifetime of this transaction. `true` =>
    // is_local, so the setting resets on COMMIT/ROLLBACK and cannot bleed
    // onto the next checkout of this pooled connection.
    sqlx::query("SELECT set_config('app.current_org_id', $1, true)")
        .bind(org_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(StoreError::Database)?;

    let result = f(&mut tx).await;

    match result {
        Ok(value) => {
            tx.commit().await.map_err(StoreError::Database)?;
            Ok(value)
        }
        Err(err) => {
            // Best-effort rollback; the transaction also rolls back on drop.
            let _ = tx.rollback().await;
            Err(err)
        }
    }
}

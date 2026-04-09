//! Registry admin integration for the OSS admin UI.
//!
//! This module wires the shared [`mockforge_registry_core::store::SqliteRegistryStore`]
//! into `mockforge-ui` so the embedded admin server can manage users,
//! organizations, API tokens, and audit logs against a local SQLite
//! database — reusing the same `RegistryStore` trait and query paths that
//! power the multi-tenant SaaS `mockforge-registry-server` binary.
//!
//! This is the Phase 5a entry point (task #16). Follow-up work will add
//! the axum routes that call into the store; for now the module exposes
//! `init_sqlite_registry_store(db_url)` plus a shared [`CoreAppState`]
//! struct so any future handler can take `State<CoreAppState>` and reach
//! the store through a stable `Arc<dyn RegistryStore>` dispatch.

#![cfg(feature = "registry-admin")]

use std::sync::Arc;

use mockforge_registry_core::error::StoreResult;
use mockforge_registry_core::store::{RegistryStore, SqliteRegistryStore};

/// Minimal app state for the registry-admin subsystem.
///
/// Kept intentionally small — just the backend-agnostic `Arc<dyn
/// RegistryStore>`. The UI's main `AppState` (in `routes.rs`) can hold
/// one of these inside an `Option` and only construct it when the user
/// opts into the OSS admin backend.
#[derive(Clone)]
pub struct CoreAppState {
    pub store: Arc<dyn RegistryStore>,
}

impl CoreAppState {
    /// Wrap an arbitrary [`RegistryStore`] implementation. Useful for
    /// tests that want to hand in a mock or in-memory store.
    pub fn new(store: Arc<dyn RegistryStore>) -> Self {
        Self { store }
    }
}

/// Bootstrap a SQLite-backed registry store from a connection URL,
/// running the bundled OSS migrations. Returns the concrete store so
/// callers can also reach the pool if they need to run raw SQL during
/// setup; most callers should wrap it in a [`CoreAppState`] via
/// [`CoreAppState::new`] + `Arc::new`.
///
/// Example URLs:
///   * `sqlite::memory:`               — in-process, discarded on exit
///   * `sqlite://./mockforge.db`       — on-disk file in the cwd
///   * `sqlite:///var/lib/mockforge.db` — absolute path
pub async fn init_sqlite_registry_store(database_url: &str) -> StoreResult<SqliteRegistryStore> {
    SqliteRegistryStore::connect(database_url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_registry_core::models::organization::Plan;

    /// Smoke test that proves `mockforge-ui` can actually reach into
    /// `mockforge-registry-core`: open an in-memory SQLite store, run
    /// the migrations, create a user + org, and hit the store through
    /// the `dyn RegistryStore` trait object inside `CoreAppState`.
    #[tokio::test]
    async fn test_init_sqlite_registry_store_end_to_end() {
        let store = init_sqlite_registry_store("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");

        // Exercise the concrete store first — this is what the main
        // admin server init path would do.
        let user = store
            .create_user("ui-admin", "ui-admin@example.com", "bcrypt_hash")
            .await
            .expect("create user");
        let org = store
            .create_organization("UI Org", "ui-org", user.id, Plan::Free)
            .await
            .expect("create org");
        assert_eq!(org.owner_id, user.id);

        // Wrap in CoreAppState and round-trip the lookups through the
        // Arc<dyn RegistryStore> dispatch — this is the shape the UI
        // handlers will use.
        let state = CoreAppState::new(Arc::new(store));
        let reloaded_user = state
            .store
            .find_user_by_email("ui-admin@example.com")
            .await
            .expect("find user")
            .expect("user exists");
        assert_eq!(reloaded_user.id, user.id);

        let reloaded_org = state
            .store
            .find_organization_by_slug("ui-org")
            .await
            .expect("find org")
            .expect("org exists");
        assert_eq!(reloaded_org.id, org.id);
    }
}

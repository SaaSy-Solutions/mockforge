//! Snapshot diff HTTP behavior, independent of the consuming management state.

use async_trait::async_trait;
mod comparison;
mod models;
mod routes;

pub use models::*;
pub use routes::snapshot_diff_router;

/// Current mock projection supplied by the consuming application.
#[async_trait]
pub trait SnapshotStateProvider: Clone + Send + Sync + 'static {
    /// Capture the application's current mocks using the public snapshot format.
    async fn snapshot_items(&self) -> Vec<MockSnapshotItem>;
}

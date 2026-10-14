//! Snapshot diff compatibility surface and management-state projection.
//!
//! Public DTOs and router entry point retain their original HTTP paths.

use crate::management::ManagementState;
use async_trait::async_trait;
pub use mockforge_intelligence::handlers::snapshot_diff::*;

#[async_trait]
impl SnapshotStateProvider for ManagementState {
    async fn snapshot_items(&self) -> Vec<MockSnapshotItem> {
        self.mocks
            .read()
            .await
            .iter()
            .map(|mock| MockSnapshotItem {
                id: mock.id.clone(),
                method: mock.method.clone(),
                path: mock.path.clone(),
                status_code: mock.status_code.unwrap_or(200),
                response_body: mock.response.body.clone(),
                response_headers: mock.response.headers.clone(),
                config: serde_json::to_value(mock).unwrap_or_default(),
            })
            .collect()
    }
}

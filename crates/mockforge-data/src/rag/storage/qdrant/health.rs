use super::*;

impl QdrantStorage {
    pub(super) async fn collection_health(&self) -> Result<StorageHealth> {
        let result = async {
            self.initialize().await?;
            Self::result(self.request(Method::GET, &[])).await
        }
        .await;
        let mut details = HashMap::new();
        details.insert("backend".into(), "qdrant".into());
        let status = match result {
            Ok(info) => match info["status"].as_str() {
                Some("green") => HealthStatus::Healthy,
                Some("yellow" | "grey") => HealthStatus::Warning,
                _ => HealthStatus::Unhealthy,
            },
            Err(error) => {
                details.insert("error".into(), error.to_string());
                HealthStatus::Unavailable
            }
        };
        Ok(StorageHealth {
            status,
            checked_at: chrono::Utc::now(),
            details,
            metrics: None,
        })
    }
}

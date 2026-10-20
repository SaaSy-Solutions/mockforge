use super::*;

#[async_trait::async_trait]
impl DocumentStorage for QdrantStorage {
    async fn store_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<()> {
        // Validate the complete input before making any writes.
        for chunk in &chunks {
            self.validate_vector(&chunk.embedding)?;
            if chunk.id.is_empty() || serde_json::to_vec(chunk)?.len() > 1024 * 1024 {
                return Err(Error::validation(
                    "Qdrant chunks require a nonempty ID and at most 1 MiB of JSON",
                ));
            }
        }
        self.initialize().await?;
        for batch in chunks.chunks(PAGE_SIZE) {
            let points: Vec<_> = batch
                .iter()
                .map(|chunk| {
                    let pairs: Vec<_> = chunk
                        .metadata
                        .iter()
                        .map(|(key, value)| json!({"key":key,"value":value}))
                        .collect();
                    json!({"id":Self::point_id(&chunk.id),"vector":chunk.embedding,
                    "payload":{"mockforge_chunk":chunk,"metadata_pairs":pairs}})
                })
                .collect();
            Self::result(
                self.request(Method::PUT, &["points"])
                    .query(&[("wait", true)])
                    .json(&json!({"points":points})),
            )
            .await?;
        }
        Ok(())
    }

    async fn search_similar(&self, vector: &[f32], top_k: usize) -> Result<Vec<DocumentChunk>> {
        self.search(vector, top_k, None, json!({}), true).await
    }

    async fn search_with_params(
        &self,
        vector: &[f32],
        params: SearchParams,
    ) -> Result<Vec<DocumentChunk>> {
        let distance = match params.search_method {
            SearchMethod::Cosine => "Cosine",
            SearchMethod::Euclidean => "Euclid",
            SearchMethod::DotProduct => "Dot",
            SearchMethod::Manhattan => "Manhattan",
        };
        if distance != self.distance || !params.threshold.is_finite() {
            return Err(Error::validation(
                "Search metric must match the Qdrant collection and threshold must be finite",
            ));
        }
        let mut conditions = Vec::new();
        if let Some(id) = params.document_filter {
            conditions.push(json!({"key":"mockforge_chunk.document_id","match":{"value":id}}));
        }
        if let Some(metadata) = params.metadata_filter {
            for (key, value) in metadata {
                conditions.push(json!({"nested":{"key":"metadata_pairs","filter":{"must":[
                    {"key":"key","match":{"value":key}}, {"key":"value","match":{"value":value}}
                ]}}}));
            }
        }
        self.search(
            vector,
            params.top_k,
            Some(params.threshold),
            json!({"must":conditions}),
            params.include_metadata,
        )
        .await
    }

    async fn get_chunk(&self, id: &str) -> Result<Option<DocumentChunk>> {
        self.initialize().await?;
        let points = Self::result(
            self.request(Method::POST, &["points"])
                .json(&json!({"ids":[Self::point_id(id)],"with_payload":true,"with_vector":false})),
        )
        .await?;
        let points = points
            .as_array()
            .ok_or_else(|| Error::generic("Invalid Qdrant retrieve response"))?;
        points
            .first()
            .map(Self::decode)
            .transpose()?
            .map(|chunk| {
                if chunk.id != id {
                    Err(Error::generic("Qdrant point ID does not match its chunk"))
                } else {
                    Ok(chunk)
                }
            })
            .transpose()
    }

    async fn delete_chunk(&self, id: &str) -> Result<bool> {
        if self.get_chunk(id).await?.is_none() {
            return Ok(false);
        }
        Self::result(
            self.request(Method::POST, &["points", "delete"])
                .query(&[("wait", true)])
                .json(&json!({"points":[Self::point_id(id)]})),
        )
        .await?;
        Ok(true)
    }

    async fn get_chunks_by_document(&self, id: &str) -> Result<Vec<DocumentChunk>> {
        self.initialize().await?;
        let mut chunks = Vec::new();
        self.scan(Self::document_filter(id), false, |point| {
            chunks.push(Self::decode(&point)?);
            Ok(())
        })
        .await?;
        chunks.sort_by_key(|chunk| chunk.position);
        Ok(chunks)
    }

    async fn delete_document(&self, id: &str) -> Result<usize> {
        self.initialize().await?;
        let filter = Self::document_filter(id);
        let count = self.count(filter.clone()).await?;
        Self::result(
            self.request(Method::POST, &["points", "delete"])
                .query(&[("wait", true)])
                .json(&json!({"filter":filter})),
        )
        .await?;
        Ok(count)
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let total_chunks = self.get_total_chunks().await?;
        let total_documents = self.list_documents().await?.len();
        Ok(StorageStats {
            total_documents,
            total_chunks,
            backend_type: "qdrant".into(),
            last_updated: chrono::Utc::now(),
            index_size_bytes: 0,
            available_space_bytes: 0,
            used_space_bytes: 0,
        })
    }

    async fn list_documents(&self) -> Result<Vec<String>> {
        self.initialize().await?;
        let mut documents = std::collections::BTreeSet::new();
        self.scan(json!({}), true, |point| {
            let id = point["payload"]["mockforge_chunk"]["document_id"]
                .as_str()
                .ok_or_else(|| Error::generic("Qdrant point omitted document ID"))?;
            documents.insert(id.to_owned());
            Ok(())
        })
        .await?;
        Ok(documents.into_iter().collect())
    }

    async fn get_total_chunks(&self) -> Result<usize> {
        self.initialize().await?;
        self.count(json!({})).await
    }

    async fn clear(&self) -> Result<()> {
        self.initialize().await?;
        Self::result(
            self.request(Method::POST, &["points", "delete"])
                .query(&[("wait", true)])
                .json(&json!({"filter":{}})),
        )
        .await?;
        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        // Qdrant continuously optimizes segments/HNSW in the background.
        // Report a failed optimizer instead of claiming compaction succeeded.
        self.initialize().await?;
        let info = Self::result(self.request(Method::GET, &[])).await?;
        if info["optimizer_status"] != "ok" {
            return Err(Error::generic("Qdrant optimizer is not healthy"));
        }
        Ok(())
    }

    async fn create_backup(&self, path: &str) -> Result<()> {
        self.download_snapshot(path).await
    }
    async fn restore_backup(&self, path: &str) -> Result<()> {
        self.upload_snapshot(path).await
    }

    async fn health_check(&self) -> Result<StorageHealth> {
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

impl QdrantStorage {
    async fn search(
        &self,
        vector: &[f32],
        limit: usize,
        threshold: Option<f32>,
        filter: Value,
        metadata: bool,
    ) -> Result<Vec<DocumentChunk>> {
        self.validate_vector(vector)?;
        if limit > 10000 {
            return Err(Error::validation("Qdrant search top_k must be at most 10000"));
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.initialize().await?;
        let mut body = json!({"query":vector,"limit":limit,"filter":filter,"with_payload":true,"with_vector":false});
        if let Some(threshold) = threshold {
            body["score_threshold"] = json!(threshold);
        }
        let result =
            Self::result(self.request(Method::POST, &["points", "query"]).json(&body)).await?;
        result["points"]
            .as_array()
            .ok_or_else(|| Error::generic("Invalid Qdrant query response"))?
            .iter()
            .map(|point| {
                let mut chunk = Self::decode(point)?;
                if !metadata {
                    chunk.metadata.clear();
                }
                Ok(chunk)
            })
            .collect()
    }
}

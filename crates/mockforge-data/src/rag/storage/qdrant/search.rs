use super::*;

impl QdrantStorage {
    pub(super) async fn filtered_search(
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

    pub(super) async fn search(
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

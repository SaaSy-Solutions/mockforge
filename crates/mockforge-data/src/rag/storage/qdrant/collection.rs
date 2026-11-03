use super::*;

impl QdrantStorage {
    pub(super) async fn initialize(&self) -> Result<()> {
        self.ready
            .get_or_try_init(|| async {
                let response = self.collection_response(false).await?;
                if response.status() == StatusCode::NOT_FOUND {
                    let response = self
                        .request(Method::PUT, &[])
                        .json(&json!({"vectors":{"size":self.dimensions,"distance":self.distance}}))
                        .send()
                        .await
                        .map_err(|_| Error::generic("Qdrant collection creation failed"))?;
                    // Another process may have created this collection concurrently.
                    if !response.status().is_success() && response.status() != StatusCode::CONFLICT
                    {
                        return Err(Error::generic(format!(
                            "Qdrant collection creation returned HTTP {}",
                            response.status().as_u16()
                        )));
                    }
                } else if !response.status().is_success() {
                    return Err(Error::generic(format!(
                        "Qdrant collection lookup returned HTTP {}",
                        response.status().as_u16()
                    )));
                }
                let info = Self::response_result(self.collection_response(true).await?).await?;
                let vectors = &info["config"]["params"]["vectors"];
                if vectors["size"].as_u64() != Some(self.dimensions as u64)
                    || vectors["distance"].as_str() != Some(self.distance.as_str())
                {
                    return Err(Error::config(
                        "Qdrant collection vector configuration does not match",
                    ));
                }
                Self::result(self.request(Method::PUT, &["index"]).query(&[("wait", true)]).json(
                    &json!({"field_name":"mockforge_chunk.document_id","field_schema":"keyword"}),
                ))
                .await?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    // Qdrant can publish collection metadata before its first shard is ready:
    // concurrent creators then see HTTP 500 ("0 of 0 read operations failed").
    // Wait only on collection-read readiness; authentication and configuration
    // errors still fail immediately, and writes are never retried blindly.
    pub(super) async fn collection_response(&self, retry_missing: bool) -> Result<Response> {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let response = self
                    .request(Method::GET, &[])
                    .send()
                    .await
                    .map_err(|_| Error::generic("Qdrant collection lookup failed"))?;
                if matches!(
                    response.status(),
                    StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE
                ) || (retry_missing && response.status() == StatusCode::NOT_FOUND)
                {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                return Ok(response);
            }
        })
        .await
        .map_err(|_| {
            Error::generic("Qdrant collection did not become readable within 10 seconds")
        })?
    }
}

//! Persistent vector search through Qdrant's REST API.
//!
//! Configuration keys: `provider=qdrant`, `url`, `collection`, `dimensions`,
//! optional `api_key`, `distance` (Cosine/Euclid/Dot/Manhattan), and
//! `timeout_seconds` (default 60). Connections are established on first use.

use super::*;
use crate::Error;
use reqwest::{Method, Response, StatusCode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::OnceCell;

mod backend;
mod snapshots;
#[cfg(test)]
mod tests;

const PAGE_SIZE: usize = 32;

/// Persistent Qdrant collection implementing [`DocumentStorage`].
/// Each collection belongs to this store; do not mix unrelated point formats.
/// Debug output deliberately omits connection configuration and API keys.
pub struct QdrantStorage {
    client: reqwest::Client,
    base: url::Url,
    collection: String,
    dimensions: usize,
    distance: String,
    ready: OnceCell<()>,
}

impl QdrantStorage {
    /// Build storage without network I/O. Existing collections must use the
    /// configured dimensions/distance; incompatible collections are rejected.
    pub fn new(config: &HashMap<String, String>) -> Result<Self> {
        let required = |key| {
            config
                .get(key)
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| Error::config(format!("Qdrant requires {key}")))
        };
        let base =
            url::Url::parse(required("url")?).map_err(|_| Error::config("Invalid Qdrant URL"))?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Error::config(
                "Qdrant URL requires HTTP(S), without credentials, query or fragment",
            ));
        }
        let collection = required("collection")?.clone();
        if collection.len() > 128
            || !collection.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(Error::config(
                "Qdrant collection requires 1-128 letters, digits, underscores or hyphens",
            ));
        }
        let dimensions = required("dimensions")?
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0 && *n <= 65536)
            .ok_or_else(|| Error::config("Qdrant dimensions must be between 1 and 65536"))?;
        let distance = config.get("distance").map(String::as_str).unwrap_or("Cosine");
        if !matches!(distance, "Cosine" | "Euclid" | "Dot" | "Manhattan") {
            return Err(Error::config("Unsupported Qdrant distance"));
        }
        let timeout = config
            .get("timeout_seconds")
            .map(|v| v.parse::<u64>())
            .transpose()
            .map_err(|_| Error::config("Invalid Qdrant timeout_seconds"))?
            .unwrap_or(60);
        if !(1..=3600).contains(&timeout) {
            return Err(Error::config("Qdrant timeout_seconds must be between 1 and 3600"));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = config.get("api_key") {
            let mut value = reqwest::header::HeaderValue::from_str(key)
                .map_err(|_| Error::config("Invalid Qdrant API key header"))?;
            value.set_sensitive(true);
            headers.insert("api-key", value);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|_| Error::config("Could not create Qdrant client"))?;
        Ok(Self {
            client,
            base,
            collection,
            dimensions,
            distance: distance.into(),
            ready: OnceCell::new(),
        })
    }

    fn endpoint(&self, suffix: &[&str]) -> url::Url {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .expect("validated HTTP URL")
            .pop_if_empty()
            .extend(["collections", self.collection.as_str()])
            .extend(suffix.iter().copied());
        url
    }

    fn request(&self, method: Method, suffix: &[&str]) -> reqwest::RequestBuilder {
        self.client.request(method, self.endpoint(suffix))
    }

    async fn send(request: reqwest::RequestBuilder) -> Result<Response> {
        let response = request.send().await.map_err(|_| Error::generic("Qdrant request failed"))?;
        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Qdrant returned HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(response)
    }

    async fn result(request: reqwest::RequestBuilder) -> Result<Value> {
        Self::response_result(Self::send(request).await?).await
    }

    async fn response_result(mut response: Response) -> Result<Value> {
        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Qdrant returned HTTP {}",
                response.status().as_u16()
            )));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::generic("Qdrant response interrupted"))?
        {
            if bytes.len().saturating_add(chunk.len()) > 64 * 1024 * 1024 {
                return Err(Error::generic("Qdrant JSON response exceeds 64 MiB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Error::generic("Invalid Qdrant JSON response"))?;
        body.get_mut("result")
            .map(Value::take)
            .ok_or_else(|| Error::generic("Qdrant response omitted result"))
    }

    async fn initialize(&self) -> Result<()> {
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
    async fn collection_response(&self, retry_missing: bool) -> Result<Response> {
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

    fn validate_vector(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dimensions || !vector.iter().all(|v| v.is_finite()) {
            return Err(Error::validation(
                "Qdrant vector dimensions must match and values must be finite",
            ));
        }
        Ok(())
    }

    fn point_id(id: &str) -> String {
        let digest = Sha256::digest(id.as_bytes());
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        uuid::Uuid::from_bytes(bytes).to_string()
    }

    fn decode(point: &Value) -> Result<DocumentChunk> {
        serde_json::from_value(point["payload"]["mockforge_chunk"].clone())
            .map_err(|_| Error::generic("Qdrant point has invalid MockForge chunk payload"))
    }

    fn document_filter(id: &str) -> Value {
        json!({"must":[{"key":"mockforge_chunk.document_id","match":{"value":id}}]})
    }

    async fn count(&self, filter: Value) -> Result<usize> {
        let result = Self::result(
            self.request(Method::POST, &["points", "count"])
                .json(&json!({"filter":filter,"exact":true})),
        )
        .await?;
        result["count"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Error::generic("Invalid Qdrant count response"))
    }

    async fn scan<F>(&self, filter: Value, documents_only: bool, mut visit: F) -> Result<()>
    where
        F: FnMut(Value) -> Result<()> + Send,
    {
        let mut offset = Value::Null;
        loop {
            let payload = if documents_only {
                json!({"include":["mockforge_chunk.document_id"]})
            } else {
                json!(true)
            };
            let result = Self::result(self.request(Method::POST, &["points","scroll"])
                .json(&json!({"filter":filter,"limit":PAGE_SIZE,"offset":offset,"with_payload":payload,"with_vector":false}))).await?;
            for point in result["points"]
                .as_array()
                .ok_or_else(|| Error::generic("Invalid Qdrant scroll response"))?
            {
                visit(point.clone())?;
            }
            let next = result["next_page_offset"].clone();
            if next.is_null() {
                break;
            }
            if next == offset {
                return Err(Error::generic("Qdrant scroll did not advance"));
            }
            offset = next;
        }
        Ok(())
    }
}

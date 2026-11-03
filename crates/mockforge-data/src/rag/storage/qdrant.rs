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
mod client;
mod collection;
mod config;
mod health;
mod search;
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

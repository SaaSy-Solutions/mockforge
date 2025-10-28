//! Core recording functionality

use crate::{database::RecorderDatabase, models::*, scrubbing::*, Result};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

/// Recorder for capturing API requests and responses
#[derive(Clone)]
pub struct Recorder {
    db: Arc<RecorderDatabase>,
    enabled: Arc<RwLock<bool>>,
    scrubber: Arc<Scrubber>,
    filter: Arc<CaptureFilter>,
}

impl Recorder {
    /// Create a new recorder
    pub fn new(db: RecorderDatabase) -> Self {
        Self {
            db: Arc::new(db),
            enabled: Arc::new(RwLock::new(true)),
            scrubber: Scrubber::global(),
            filter: CaptureFilter::global(),
        }
    }

    /// Create a new recorder with custom scrubber and filter
    pub fn with_scrubbing(db: RecorderDatabase, scrubber: Scrubber, filter: CaptureFilter) -> Self {
        Self {
            db: Arc::new(db),
            enabled: Arc::new(RwLock::new(true)),
            scrubber: Arc::new(scrubber),
            filter: Arc::new(filter),
        }
    }

    /// Get the scrubber
    pub fn scrubber(&self) -> &Arc<Scrubber> {
        &self.scrubber
    }

    /// Get the filter
    pub fn filter(&self) -> &Arc<CaptureFilter> {
        &self.filter
    }

    /// Check if recording is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Enable recording
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        debug!("Recording enabled");
    }

    /// Disable recording
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        debug!("Recording disabled");
    }

    /// Record a request
    pub async fn record_request(&self, mut request: RecordedRequest) -> Result<String> {
        if !self.is_enabled().await {
            return Ok(request.id);
        }

        // Apply scrubbing
        self.scrubber.scrub_request(&mut request);

        let request_id = request.id.clone();
        self.db.insert_request(&request).await?;
        Ok(request_id)
    }

    /// Record a response
    pub async fn record_response(&self, mut response: RecordedResponse) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        // Apply scrubbing
        self.scrubber.scrub_response(&mut response);

        self.db.insert_response(&response).await?;
        Ok(())
    }

    /// Record an HTTP request
    pub async fn record_http_request(
        &self,
        method: &str,
        path: &str,
        query_params: Option<&str>,
        headers: &std::collections::HashMap<String, String>,
        body: Option<&[u8]>,
        context: &crate::models::RequestContext,
    ) -> Result<String> {
        let request_id = Uuid::new_v4().to_string();

        let (body_str, body_encoding) = encode_body(body);

        let request = RecordedRequest {
            id: request_id.clone(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: method.to_string(),
            path: path.to_string(),
            query_params: query_params.map(|q| q.to_string()),
            headers: serde_json::to_string(&headers)?,
            body: body_str,
            body_encoding,
            client_ip: context.client_ip.clone(),
            trace_id: context.trace_id.clone(),
            span_id: context.span_id.clone(),
            duration_ms: None,
            status_code: None,
            tags: None,
        };

        self.record_request(request).await
    }

    /// Record an HTTP response
    pub async fn record_http_response(
        &self,
        request_id: &str,
        status_code: i32,
        headers: &std::collections::HashMap<String, String>,
        body: Option<&[u8]>,
        duration_ms: i64,
    ) -> Result<()> {
        // Check filter with status code now that we have it
        // Get the request to check path and method
        if let Some(request) = self.db.get_request(request_id).await? {
            let should_capture = self.filter.should_capture(
                &request.method,
                &request.path,
                Some(status_code as u16),
            );

            if !should_capture {
                // Delete the request since it doesn't match the filter
                // (We don't have a delete method, so we just skip the response)
                debug!("Skipping response recording due to filter");
                return Ok(());
            }
        }

        let (body_str, body_encoding) = encode_body(body);
        let size_bytes = body.map(|b| b.len()).unwrap_or(0) as i64;

        let response = RecordedResponse {
            request_id: request_id.to_string(),
            status_code,
            headers: serde_json::to_string(&headers)?,
            body: body_str,
            body_encoding,
            size_bytes,
            timestamp: Utc::now(),
        };

        self.record_response(response).await?;

        // Update request with duration and status
        self.update_request_completion(request_id, status_code, duration_ms).await?;

        Ok(())
    }

    /// Update request with completion data
    async fn update_request_completion(
        &self,
        _request_id: &str,
        _status_code: i32,
        _duration_ms: i64,
    ) -> Result<()> {
        // Note: This would need to access the pool through a public method
        // For now, we'll skip this optimization and rely on separate inserts
        Ok(())
    }

    /// Get database reference
    pub fn database(&self) -> &Arc<RecorderDatabase> {
        &self.db
    }
}

/// Encode body for storage (binary data as base64)
fn encode_body(body: Option<&[u8]>) -> (Option<String>, String) {
    match body {
        None => (None, "utf8".to_string()),
        Some(bytes) => {
            // Try to parse as UTF-8 first
            if let Ok(text) = std::str::from_utf8(bytes) {
                (Some(text.to_string()), "utf8".to_string())
            } else {
                // Binary data, encode as base64
                let encoded =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);
                (Some(encoded), "base64".to_string())
            }
        }
    }
}

/// Decode body from storage
pub fn decode_body(body: Option<&str>, encoding: &str) -> Option<Vec<u8>> {
    body.map(|b| {
        if encoding == "base64" {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b)
                .unwrap_or_else(|_| b.as_bytes().to_vec())
        } else {
            b.as_bytes().to_vec()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;

    #[tokio::test]
    async fn test_recorder_enable_disable() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        assert!(recorder.is_enabled().await);

        recorder.disable().await;
        assert!(!recorder.is_enabled().await);

        recorder.enable().await;
        assert!(recorder.is_enabled().await);
    }

    #[tokio::test]
    async fn test_record_http_exchange() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let headers = std::collections::HashMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]);

        let context = RequestContext::new(Some("127.0.0.1"), None, None);
        let request_id = recorder
            .record_http_request("GET", "/api/test", Some("foo=bar"), &headers, None, &context)
            .await
            .unwrap();

        let body = b"{\"result\":\"ok\"}";
        recorder
            .record_http_response(&request_id, 200, &headers, Some(body), 42)
            .await
            .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&request_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.path, "/api/test");
        assert_eq!(exchange.response.unwrap().status_code, 200);
    }

    #[test]
    fn test_body_encoding() {
        // UTF-8 text
        let text = b"Hello, World!";
        let (encoded, encoding) = encode_body(Some(text));
        assert_eq!(encoding, "utf8");
        assert_eq!(encoded, Some("Hello, World!".to_string()));

        // Binary data
        let binary = &[0xFF, 0xFE, 0xFD];
        let (encoded, encoding) = encode_body(Some(binary));
        assert_eq!(encoding, "base64");
        assert!(encoded.is_some());

        // Decode back
        let decoded = decode_body(encoded.as_deref(), &encoding);
        assert_eq!(decoded, Some(binary.to_vec()));
    }
}

//! Priority-based HTTP request handler implementing the full priority chain:
//! Replay → Fail → Proxy → Mock → Record

use crate::{
    Error, FailureInjector, ProxyHandler, RecordReplayHandler, RequestFingerprint,
    ResponsePriority, ResponseSource, Result,
};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use std::collections::HashMap;

/// Priority-based HTTP request handler
pub struct PriorityHttpHandler {
    /// Record/replay handler
    record_replay: RecordReplayHandler,
    /// Failure injector
    failure_injector: Option<FailureInjector>,
    /// Proxy handler
    proxy_handler: Option<ProxyHandler>,
    /// Mock response generator (from OpenAPI spec)
    mock_generator: Option<Box<dyn MockGenerator + Send + Sync>>,
    /// OpenAPI spec for tag extraction
    openapi_spec: Option<crate::openapi::spec::OpenApiSpec>,
}

/// Trait for mock response generation
pub trait MockGenerator {
    /// Generate a mock response for the given request
    fn generate_mock_response(
        &self,
        fingerprint: &RequestFingerprint,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<Option<MockResponse>>;
}

/// Mock response
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// Response status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Content type
    pub content_type: String,
}

impl PriorityHttpHandler {
    /// Create a new priority HTTP handler
    pub fn new(
        record_replay: RecordReplayHandler,
        failure_injector: Option<FailureInjector>,
        proxy_handler: Option<ProxyHandler>,
        mock_generator: Option<Box<dyn MockGenerator + Send + Sync>>,
    ) -> Self {
        Self {
            record_replay,
            failure_injector,
            proxy_handler,
            mock_generator,
            openapi_spec: None,
        }
    }

    /// Create a new priority HTTP handler with OpenAPI spec
    pub fn new_with_openapi(
        record_replay: RecordReplayHandler,
        failure_injector: Option<FailureInjector>,
        proxy_handler: Option<ProxyHandler>,
        mock_generator: Option<Box<dyn MockGenerator + Send + Sync>>,
        openapi_spec: Option<crate::openapi::spec::OpenApiSpec>,
    ) -> Self {
        Self {
            record_replay,
            failure_injector,
            proxy_handler,
            mock_generator,
            openapi_spec,
        }
    }

    /// Process a request through the priority chain
    pub async fn process_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<PriorityResponse> {
        let fingerprint = RequestFingerprint::new(method.clone(), uri, headers, body);

        // 1. REPLAY: Check if we have a recorded fixture
        if let Some(recorded_request) =
            self.record_replay.replay_handler().load_fixture(&fingerprint).await?
        {
            let content_type = recorded_request
                .response_headers
                .get("content-type")
                .unwrap_or(&"application/json".to_string())
                .clone();

            return Ok(PriorityResponse {
                source: ResponseSource::new(ResponsePriority::Replay, "replay".to_string())
                    .with_metadata("fixture_path".to_string(), "recorded".to_string()),
                status_code: recorded_request.status_code,
                headers: recorded_request.response_headers,
                body: recorded_request.response_body.into_bytes(),
                content_type,
            });
        }

        // 2. FAIL: Check for failure injection
        if let Some(ref failure_injector) = self.failure_injector {
            let tags = if let Some(ref spec) = self.openapi_spec {
                fingerprint.openapi_tags(spec).unwrap_or_else(|| fingerprint.tags())
            } else {
                fingerprint.tags()
            };
            if let Some((status_code, error_message)) = failure_injector.process_request(&tags) {
                let error_response = serde_json::json!({
                    "error": error_message,
                    "injected_failure": true,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                return Ok(PriorityResponse {
                    source: ResponseSource::new(
                        ResponsePriority::Fail,
                        "failure_injection".to_string(),
                    )
                    .with_metadata("error_message".to_string(), error_message),
                    status_code,
                    headers: HashMap::new(),
                    body: serde_json::to_string(&error_response)?.into_bytes(),
                    content_type: "application/json".to_string(),
                });
            }
        }

        // 3. PROXY: Check if request should be proxied (respecting migration mode)
        if let Some(ref proxy_handler) = self.proxy_handler {
            // Check migration mode first
            let migration_mode = if proxy_handler.config.migration_enabled {
                proxy_handler.config.get_effective_migration_mode(uri.path())
            } else {
                None
            };

            // If migration mode is Mock, skip proxy and continue to mock generator
            if let Some(crate::proxy::config::MigrationMode::Mock) = migration_mode {
                // Force mock mode - skip proxy
            } else if proxy_handler.config.should_proxy(method, uri.path()) {
                // Check if this is shadow mode (proxy + generate mock for comparison)
                let is_shadow = proxy_handler.config.should_shadow(uri.path());

                match proxy_handler.proxy_request(method, uri, headers, body).await {
                    Ok(proxy_response) => {
                        let mut response_headers = HashMap::new();
                        for (key, value) in proxy_response.headers.iter() {
                            let key_str = key.as_str();
                            if let Ok(value_str) = value.to_str() {
                                response_headers.insert(key_str.to_string(), value_str.to_string());
                            }
                        }

                        let content_type = response_headers
                            .get("content-type")
                            .unwrap_or(&"application/json".to_string())
                            .clone();

                        // If shadow mode, also generate mock response for comparison
                        if is_shadow {
                            if let Some(ref mock_generator) = self.mock_generator {
                                if let Ok(Some(mock_response)) = mock_generator
                                    .generate_mock_response(&fingerprint, headers, body)
                                {
                                    // Log comparison between real and mock
                                    tracing::info!(
                                        path = %uri.path(),
                                        real_status = proxy_response.status_code,
                                        mock_status = mock_response.status_code,
                                        "Shadow mode: comparing real and mock responses"
                                    );

                                    // Compare response bodies (basic comparison)
                                    let real_body_bytes = proxy_response.body.clone().unwrap_or_default();
                                    let real_body = String::from_utf8_lossy(&real_body_bytes);
                                    let mock_body = &mock_response.body;

                                    if real_body != *mock_body {
                                        tracing::warn!(
                                            path = %uri.path(),
                                            "Shadow mode: real and mock responses differ"
                                        );
                                    }
                                }
                            }
                        }

                        let mut source = ResponseSource::new(
                            ResponsePriority::Proxy,
                            if is_shadow {
                                "shadow".to_string()
                            } else {
                                "proxy".to_string()
                            },
                        )
                        .with_metadata(
                            "upstream_url".to_string(),
                            proxy_handler.config.get_upstream_url(uri.path()),
                        );

                        if let Some(mode) = migration_mode {
                            source = source
                                .with_metadata("migration_mode".to_string(), format!("{:?}", mode));
                        }

                        return Ok(PriorityResponse {
                            source,
                            status_code: proxy_response.status_code,
                            headers: response_headers,
                            body: proxy_response.body.unwrap_or_default(),
                            content_type,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Proxy request failed: {}", e);
                        // If migration mode is Real, fail hard (don't fall back to mock)
                        if let Some(crate::proxy::config::MigrationMode::Real) = migration_mode {
                            return Err(Error::generic(format!(
                                "Proxy request failed in real mode: {}",
                                e
                            )));
                        }
                        // Continue to next handler for other modes
                    }
                }
            }
        }

        // 4. MOCK: Generate mock response from OpenAPI spec
        if let Some(ref mock_generator) = self.mock_generator {
            // Check if we're in mock mode (forced by migration)
            let migration_mode = if let Some(ref proxy_handler) = self.proxy_handler {
                if proxy_handler.config.migration_enabled {
                    proxy_handler.config.get_effective_migration_mode(uri.path())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(mock_response) =
                mock_generator.generate_mock_response(&fingerprint, headers, body)?
            {
                let mut source = ResponseSource::new(ResponsePriority::Mock, "mock".to_string())
                    .with_metadata("generated_from".to_string(), "openapi_spec".to_string());

                if let Some(mode) = migration_mode {
                    source =
                        source.with_metadata("migration_mode".to_string(), format!("{:?}", mode));
                }

                return Ok(PriorityResponse {
                    source,
                    status_code: mock_response.status_code,
                    headers: mock_response.headers,
                    body: mock_response.body.into_bytes(),
                    content_type: mock_response.content_type,
                });
            }
        }

        // 5. RECORD: Record the request for future replay
        if self.record_replay.record_handler().should_record(method) {
            // For now, return a default response and record it
            let default_response = serde_json::json!({
                "message": "Request recorded for future replay",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "fingerprint": fingerprint.to_hash()
            });

            let response_body = serde_json::to_string(&default_response)?;
            let status_code = 200;

            // Record the request
            self.record_replay
                .record_handler()
                .record_request(&fingerprint, status_code, headers, &response_body, None)
                .await?;

            return Ok(PriorityResponse {
                source: ResponseSource::new(ResponsePriority::Record, "record".to_string())
                    .with_metadata("recorded".to_string(), "true".to_string()),
                status_code,
                headers: HashMap::new(),
                body: response_body.into_bytes(),
                content_type: "application/json".to_string(),
            });
        }

        // If we reach here, no handler could process the request
        Err(Error::generic("No handler could process the request".to_string()))
    }
}

/// Priority response
#[derive(Debug, Clone)]
pub struct PriorityResponse {
    /// Response source information
    pub source: ResponseSource,
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Content type
    pub content_type: String,
}

impl PriorityResponse {
    /// Convert to Axum response
    pub fn to_axum_response(self) -> axum::response::Response {
        let mut response = axum::response::Response::new(axum::body::Body::from(self.body));
        *response.status_mut() = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::OK);

        // Add headers
        for (key, value) in self.headers {
            if let (Ok(header_name), Ok(header_value)) =
                (key.parse::<axum::http::HeaderName>(), value.parse::<axum::http::HeaderValue>())
            {
                response.headers_mut().insert(header_name, header_value);
            }
        }

        // Set content type if not already set
        if !response.headers().contains_key("content-type") {
            if let Ok(header_value) = self.content_type.parse::<axum::http::HeaderValue>() {
                response.headers_mut().insert("content-type", header_value);
            }
        }

        response
    }
}

/// Simple mock generator for testing
pub struct SimpleMockGenerator {
    /// Default status code
    pub default_status: u16,
    /// Default response body
    pub default_body: String,
}

impl SimpleMockGenerator {
    /// Create a new simple mock generator
    pub fn new(default_status: u16, default_body: String) -> Self {
        Self {
            default_status,
            default_body,
        }
    }
}

impl MockGenerator for SimpleMockGenerator {
    fn generate_mock_response(
        &self,
        _fingerprint: &RequestFingerprint,
        _headers: &HeaderMap,
        _body: Option<&[u8]>,
    ) -> Result<Option<MockResponse>> {
        Ok(Some(MockResponse {
            status_code: self.default_status,
            headers: HashMap::new(),
            body: self.default_body.clone(),
            content_type: "application/json".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_priority_chain() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let record_replay = RecordReplayHandler::new(fixtures_dir, true, true, false);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock response"}"#.to_string()));

        let handler = PriorityHttpHandler::new_with_openapi(
            record_replay,
            None, // No failure injection
            None, // No proxy
            Some(mock_generator),
            None, // No OpenAPI spec
        );

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "mock");
    }
}

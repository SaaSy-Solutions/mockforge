//! Priority-based HTTP request handler implementing the full priority chain:
//! Custom Fixtures → Replay → Stateful → Route Chaos (per-route fault/latency) → Global Fail → Proxy → Mock → Record

use crate::stateful_handler::StatefulResponseHandler;
use crate::{
    CustomFixtureLoader, Error, FailureInjector, ProxyHandler, RealityContinuumEngine,
    RecordReplayHandler, RequestFingerprint, ResponsePriority, ResponseSource, Result,
    RouteChaosInjector,
};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for behavioral scenario replay engines
#[async_trait]
pub trait BehavioralScenarioReplay: Send + Sync {
    /// Try to replay a request against active scenarios
    async fn try_replay(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
        session_id: Option<&str>,
    ) -> Result<Option<BehavioralReplayResponse>>;
}

/// Response from behavioral scenario replay
#[derive(Debug, Clone)]
pub struct BehavioralReplayResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Timing delay in milliseconds
    pub timing_ms: Option<u64>,
    /// Content type
    pub content_type: String,
}

/// Priority-based HTTP request handler
pub struct PriorityHttpHandler {
    /// Custom fixture loader (simple format fixtures)
    custom_fixture_loader: Option<Arc<CustomFixtureLoader>>,
    /// Record/replay handler
    record_replay: RecordReplayHandler,
    /// Behavioral scenario replay engine (for journey-level simulations)
    behavioral_scenario_replay: Option<Arc<dyn BehavioralScenarioReplay + Send + Sync>>,
    /// Stateful response handler
    stateful_handler: Option<Arc<StatefulResponseHandler>>,
    /// Per-route chaos injector (fault injection and latency)
    route_chaos_injector: Option<Arc<RouteChaosInjector>>,
    /// Failure injector (global/tag-based)
    failure_injector: Option<FailureInjector>,
    /// Proxy handler
    proxy_handler: Option<ProxyHandler>,
    /// Mock response generator (from OpenAPI spec)
    mock_generator: Option<Box<dyn MockGenerator + Send + Sync>>,
    /// OpenAPI spec for tag extraction
    openapi_spec: Option<crate::openapi::spec::OpenApiSpec>,
    /// Reality Continuum engine for blending mock and real responses
    continuum_engine: Option<Arc<RealityContinuumEngine>>,
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
            custom_fixture_loader: None,
            record_replay,
            behavioral_scenario_replay: None,
            stateful_handler: None,
            route_chaos_injector: None,
            failure_injector,
            proxy_handler,
            mock_generator,
            openapi_spec: None,
            continuum_engine: None,
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
            custom_fixture_loader: None,
            record_replay,
            behavioral_scenario_replay: None,
            stateful_handler: None,
            route_chaos_injector: None,
            failure_injector,
            proxy_handler,
            mock_generator,
            openapi_spec,
            continuum_engine: None,
        }
    }

    /// Set custom fixture loader
    pub fn with_custom_fixture_loader(mut self, loader: Arc<CustomFixtureLoader>) -> Self {
        self.custom_fixture_loader = Some(loader);
        self
    }

    /// Set stateful response handler
    pub fn with_stateful_handler(mut self, handler: Arc<StatefulResponseHandler>) -> Self {
        self.stateful_handler = Some(handler);
        self
    }

    /// Set per-route chaos injector
    pub fn with_route_chaos_injector(mut self, injector: Arc<RouteChaosInjector>) -> Self {
        self.route_chaos_injector = Some(injector);
        self
    }

    /// Set Reality Continuum engine
    pub fn with_continuum_engine(mut self, engine: Arc<RealityContinuumEngine>) -> Self {
        self.continuum_engine = Some(engine);
        self
    }

    /// Set behavioral scenario replay engine
    pub fn with_behavioral_scenario_replay(
        mut self,
        replay_engine: Arc<dyn BehavioralScenarioReplay + Send + Sync>,
    ) -> Self {
        self.behavioral_scenario_replay = Some(replay_engine);
        self
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

        // 0. CUSTOM FIXTURES: Check if we have a custom fixture (highest priority)
        if let Some(ref custom_loader) = self.custom_fixture_loader {
            if let Some(custom_fixture) = custom_loader.load_fixture(&fingerprint) {
                // Apply delay if specified
                if custom_fixture.delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        custom_fixture.delay_ms,
                    ))
                    .await;
                }

                // Convert response to JSON string if it's not already a string
                let response_body = if custom_fixture.response.is_string() {
                    custom_fixture.response.as_str().unwrap().to_string()
                } else {
                    serde_json::to_string(&custom_fixture.response).map_err(|e| {
                        Error::generic(format!("Failed to serialize custom fixture response: {}", e))
                    })?
                };

                // Determine content type
                let content_type = custom_fixture
                    .headers
                    .get("content-type")
                    .cloned()
                    .unwrap_or_else(|| "application/json".to_string());

                return Ok(PriorityResponse {
                    source: ResponseSource::new(
                        ResponsePriority::Replay,
                        "custom_fixture".to_string(),
                    )
                    .with_metadata("fixture_path".to_string(), custom_fixture.path.clone()),
                    status_code: custom_fixture.status,
                    headers: custom_fixture.headers.clone(),
                    body: response_body.into_bytes(),
                    content_type,
                });
            }
        }

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

        // 1.5. BEHAVIORAL SCENARIO REPLAY: Check for active behavioral scenarios
        if let Some(ref scenario_replay) = self.behavioral_scenario_replay {
            // Extract session ID from headers or cookies
            let session_id = headers
                .get("x-session-id")
                .or_else(|| headers.get("session-id"))
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            if let Ok(Some(replay_response)) = scenario_replay
                .try_replay(method, uri, headers, body, session_id.as_deref())
                .await
            {
                // Apply timing delay if specified
                if let Some(timing_ms) = replay_response.timing_ms {
                    tokio::time::sleep(tokio::time::Duration::from_millis(timing_ms)).await;
                }
                return Ok(PriorityResponse {
                    source: ResponseSource::new(
                        ResponsePriority::Replay,
                        "behavioral_scenario".to_string(),
                    )
                    .with_metadata("replay_type".to_string(), "scenario".to_string()),
                    status_code: replay_response.status_code,
                    headers: replay_response.headers,
                    body: replay_response.body,
                    content_type: replay_response.content_type,
                });
            }
        }

        // 2. STATEFUL: Check for stateful response handling
        if let Some(ref stateful_handler) = self.stateful_handler {
            if let Some(stateful_response) =
                stateful_handler.process_request(method, uri, headers, body).await?
            {
                return Ok(PriorityResponse {
                    source: ResponseSource::new(ResponsePriority::Stateful, "stateful".to_string())
                        .with_metadata("state".to_string(), stateful_response.state)
                        .with_metadata("resource_id".to_string(), stateful_response.resource_id),
                    status_code: stateful_response.status_code,
                    headers: stateful_response.headers,
                    body: stateful_response.body.into_bytes(),
                    content_type: stateful_response.content_type,
                });
            }
        }

        // 2.5. ROUTE CHAOS: Check for per-route fault injection and latency
        if let Some(ref route_chaos) = self.route_chaos_injector {
            // Inject latency first (before fault injection)
            if let Err(e) = route_chaos.inject_latency(method, uri).await {
                tracing::warn!("Failed to inject per-route latency: {}", e);
            }

            // Check for per-route fault injection
            if let Some(fault_response) = route_chaos.get_fault_response(method, uri) {
                let error_response = serde_json::json!({
                    "error": fault_response.error_message,
                    "injected_failure": true,
                    "fault_type": fault_response.fault_type,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                return Ok(PriorityResponse {
                    source: ResponseSource::new(
                        ResponsePriority::Fail,
                        "route_fault_injection".to_string(),
                    )
                    .with_metadata("fault_type".to_string(), fault_response.fault_type)
                    .with_metadata("error_message".to_string(), fault_response.error_message),
                    status_code: fault_response.status_code,
                    headers: HashMap::new(),
                    body: serde_json::to_string(&error_response)?.into_bytes(),
                    content_type: "application/json".to_string(),
                });
            }
        }

        // 3. FAIL: Check for global/tag-based failure injection
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

        // Check if Reality Continuum is enabled and should blend responses
        let should_blend = if let Some(ref continuum_engine) = self.continuum_engine {
            continuum_engine.is_enabled().await
        } else {
            false
        };

        // 4. PROXY: Check if request should be proxied (respecting migration mode)
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
            } else if proxy_handler.config.should_proxy_with_condition(method, uri, headers, body) {
                // Check if this is shadow mode (proxy + generate mock for comparison)
                let is_shadow = proxy_handler.config.should_shadow(uri.path());

                // If continuum is enabled, we need both mock and real responses
                if should_blend {
                    // Fetch both responses in parallel
                    let proxy_future = proxy_handler.proxy_request(method, uri, headers, body);
                    let mock_result = if let Some(ref mock_generator) = self.mock_generator {
                        mock_generator.generate_mock_response(&fingerprint, headers, body)
                    } else {
                        Ok(None)
                    };

                    // Wait for proxy response
                    let proxy_result = proxy_future.await;

                    // Handle blending
                    match (proxy_result, mock_result) {
                        (Ok(proxy_response), Ok(Some(mock_response))) => {
                            // Both succeeded - blend them
                            if let Some(ref continuum_engine) = self.continuum_engine {
                                let blend_ratio =
                                    continuum_engine.get_blend_ratio(uri.path()).await;
                                let blender = continuum_engine.blender();

                                // Parse JSON bodies
                                let mock_body_str = &mock_response.body;
                                let real_body_bytes =
                                    proxy_response.body.clone().unwrap_or_default();
                                let real_body_str = String::from_utf8_lossy(&real_body_bytes);

                                let mock_json: serde_json::Value =
                                    serde_json::from_str(mock_body_str)
                                        .unwrap_or_else(|_| serde_json::json!({}));
                                let real_json: serde_json::Value =
                                    serde_json::from_str(&real_body_str)
                                        .unwrap_or_else(|_| serde_json::json!({}));

                                // Blend the JSON responses
                                let blended_json =
                                    blender.blend_responses(&mock_json, &real_json, blend_ratio);
                                let blended_body = serde_json::to_string(&blended_json)
                                    .unwrap_or_else(|_| real_body_str.to_string());

                                // Blend status codes
                                let blended_status = blender.blend_status_code(
                                    mock_response.status_code,
                                    proxy_response.status_code,
                                    blend_ratio,
                                );

                                // Blend headers
                                let mut proxy_headers = HashMap::new();
                                for (key, value) in proxy_response.headers.iter() {
                                    if let Ok(value_str) = value.to_str() {
                                        proxy_headers.insert(
                                            key.as_str().to_string(),
                                            value_str.to_string(),
                                        );
                                    }
                                }
                                let blended_headers = blender.blend_headers(
                                    &mock_response.headers,
                                    &proxy_headers,
                                    blend_ratio,
                                );

                                let content_type = blended_headers
                                    .get("content-type")
                                    .cloned()
                                    .or_else(|| {
                                        proxy_response
                                            .headers
                                            .get("content-type")
                                            .and_then(|v| v.to_str().ok())
                                            .map(|s| s.to_string())
                                    })
                                    .unwrap_or_else(|| "application/json".to_string());

                                tracing::info!(
                                    path = %uri.path(),
                                    blend_ratio = blend_ratio,
                                    "Reality Continuum: blended mock and real responses"
                                );

                                let mut source = ResponseSource::new(
                                    ResponsePriority::Proxy,
                                    "continuum".to_string(),
                                )
                                .with_metadata("blend_ratio".to_string(), blend_ratio.to_string())
                                .with_metadata(
                                    "upstream_url".to_string(),
                                    proxy_handler.config.get_upstream_url(uri.path()),
                                );

                                if let Some(mode) = migration_mode {
                                    source = source.with_metadata(
                                        "migration_mode".to_string(),
                                        format!("{:?}", mode),
                                    );
                                }

                                return Ok(PriorityResponse {
                                    source,
                                    status_code: blended_status,
                                    headers: blended_headers,
                                    body: blended_body.into_bytes(),
                                    content_type,
                                });
                            }
                        }
                        (Ok(proxy_response), Ok(None)) => {
                            // Only proxy succeeded - use it (fallback behavior)
                            tracing::debug!(
                                path = %uri.path(),
                                "Continuum: mock generation failed, using real response"
                            );
                            // Fall through to normal proxy handling
                        }
                        (Ok(proxy_response), Err(_)) => {
                            // Only proxy succeeded - use it (fallback behavior)
                            tracing::debug!(
                                path = %uri.path(),
                                "Continuum: mock generation failed, using real response"
                            );
                            // Fall through to normal proxy handling
                        }
                        (Err(e), Ok(Some(mock_response))) => {
                            // Only mock succeeded - use it (fallback behavior)
                            tracing::debug!(
                                path = %uri.path(),
                                error = %e,
                                "Continuum: proxy failed, using mock response"
                            );
                            // Fall through to normal mock handling below
                            let mut source = ResponseSource::new(
                                ResponsePriority::Mock,
                                "continuum_fallback".to_string(),
                            )
                            .with_metadata("generated_from".to_string(), "openapi_spec".to_string())
                            .with_metadata(
                                "fallback_reason".to_string(),
                                "proxy_failed".to_string(),
                            );

                            if let Some(mode) = migration_mode {
                                source = source.with_metadata(
                                    "migration_mode".to_string(),
                                    format!("{:?}", mode),
                                );
                            }

                            return Ok(PriorityResponse {
                                source,
                                status_code: mock_response.status_code,
                                headers: mock_response.headers,
                                body: mock_response.body.into_bytes(),
                                content_type: mock_response.content_type,
                            });
                        }
                        (Err(e), _) => {
                            // Both failed
                            tracing::warn!(
                                path = %uri.path(),
                                error = %e,
                                "Continuum: both proxy and mock failed"
                            );
                            // If migration mode is Real, fail hard
                            if let Some(crate::proxy::config::MigrationMode::Real) = migration_mode
                            {
                                return Err(Error::generic(format!(
                                    "Proxy request failed in real mode: {}",
                                    e
                                )));
                            }
                            // Continue to next handler
                        }
                    }
                }

                // Normal proxy handling (when continuum is not enabled or blending failed)
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
                                    let real_body_bytes =
                                        proxy_response.body.clone().unwrap_or_default();
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

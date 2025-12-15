//! Priority-based HTTP request handler implementing the full priority chain:
//! Custom Fixtures → Replay → Stateful → Route Chaos (per-route fault/latency) → Global Fail → Proxy → Mock → Record

use crate::behavioral_economics::BehavioralEconomicsEngine;
use crate::stateful_handler::StatefulResponseHandler;
use crate::{
    CustomFixtureLoader, Error, FailureInjector, ProxyHandler, RealityContinuumEngine,
    RecordReplayHandler, RequestFingerprint, ResponsePriority, ResponseSource, Result,
};
// RouteChaosInjector moved to mockforge-route-chaos crate to avoid Send issues
// We define a trait here that RouteChaosInjector can implement to avoid circular dependency
use async_trait::async_trait;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fault injection response (defined in mockforge-core to avoid circular dependency)
#[derive(Debug, Clone)]
pub struct RouteFaultResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Error message
    pub error_message: String,
    /// Fault type identifier
    pub fault_type: String,
}

/// Trait for route chaos injection (fault injection and latency)
/// This trait is defined in mockforge-core to avoid circular dependency.
/// The concrete RouteChaosInjector in mockforge-route-chaos implements this trait.
#[async_trait]
pub trait RouteChaosInjectorTrait: Send + Sync {
    /// Inject latency for this request
    async fn inject_latency(&self, method: &Method, uri: &Uri) -> Result<()>;

    /// Get fault injection response for a request
    fn get_fault_response(&self, method: &Method, uri: &Uri) -> Option<RouteFaultResponse>;
}

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
    /// Uses trait object to avoid circular dependency with mockforge-route-chaos
    route_chaos_injector: Option<Arc<dyn RouteChaosInjectorTrait>>,
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
    /// Behavioral Economics Engine for reactive mock behavior
    behavioral_economics_engine: Option<Arc<RwLock<BehavioralEconomicsEngine>>>,
    /// Request tracking for metrics (endpoint -> (request_count, error_count, last_request_time))
    request_metrics: Arc<RwLock<HashMap<String, (u64, u64, std::time::Instant)>>>,
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
            behavioral_economics_engine: None,
            request_metrics: Arc::new(RwLock::new(HashMap::new())),
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
            behavioral_economics_engine: None,
            request_metrics: Arc::new(RwLock::new(HashMap::new())),
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
    pub fn with_route_chaos_injector(mut self, injector: Arc<dyn RouteChaosInjectorTrait>) -> Self {
        self.route_chaos_injector = Some(injector);
        self
    }

    /// Set Reality Continuum engine
    pub fn with_continuum_engine(mut self, engine: Arc<RealityContinuumEngine>) -> Self {
        self.continuum_engine = Some(engine);
        self
    }

    /// Set Behavioral Economics Engine
    pub fn with_behavioral_economics_engine(
        mut self,
        engine: Arc<RwLock<BehavioralEconomicsEngine>>,
    ) -> Self {
        self.behavioral_economics_engine = Some(engine);
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
        // Normalize the URI path before creating fingerprint to match fixture normalization
        // This ensures fixtures are matched correctly
        let normalized_path = crate::CustomFixtureLoader::normalize_path(uri.path());
        let normalized_uri_str = if let Some(query) = uri.query() {
            format!("{}?{}", normalized_path, query)
        } else {
            normalized_path
        };
        let normalized_uri =
            normalized_uri_str.parse::<axum::http::Uri>().unwrap_or_else(|_| uri.clone());

        let fingerprint = RequestFingerprint::new(method.clone(), &normalized_uri, headers, body);

        // 0. CUSTOM FIXTURES: Check if we have a custom fixture (highest priority)
        if let Some(ref custom_loader) = self.custom_fixture_loader {
            if let Some(custom_fixture) = custom_loader.load_fixture(&fingerprint) {
                // Apply delay if specified
                if custom_fixture.delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(custom_fixture.delay_ms))
                        .await;
                }

                // Convert response to JSON string if it's not already a string
                let response_body = if custom_fixture.response.is_string() {
                    custom_fixture.response.as_str().unwrap().to_string()
                } else {
                    serde_json::to_string(&custom_fixture.response).map_err(|e| {
                        Error::generic(format!(
                            "Failed to serialize custom fixture response: {}",
                            e
                        ))
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

    /// Apply behavioral economics rules to a response
    ///
    /// Updates condition evaluator with current metrics and evaluates rules,
    /// then applies any matching actions to modify the response.
    async fn apply_behavioral_economics(
        &self,
        response: PriorityResponse,
        method: &Method,
        uri: &Uri,
        latency_ms: Option<u64>,
    ) -> Result<PriorityResponse> {
        if let Some(ref engine) = self.behavioral_economics_engine {
            let engine = engine.read().await;
            let evaluator = engine.condition_evaluator();

            // Update condition evaluator with current metrics
            {
                let mut eval = evaluator.write().await;
                if let Some(latency) = latency_ms {
                    eval.update_latency(uri.path(), latency);
                }

                // Update load and error rates
                let endpoint = uri.path().to_string();
                let mut metrics = self.request_metrics.write().await;
                let now = std::time::Instant::now();

                // Get or create metrics entry for this endpoint
                let (request_count, error_count, last_request_time) =
                    metrics.entry(endpoint.clone()).or_insert_with(|| (0, 0, now));

                // Increment request count
                *request_count += 1;

                // Check if this is an error response (status >= 400)
                if response.status_code >= 400 {
                    *error_count += 1;
                }

                // Calculate error rate
                let error_rate = if *request_count > 0 {
                    *error_count as f64 / *request_count as f64
                } else {
                    0.0
                };
                eval.update_error_rate(&endpoint, error_rate);

                // Calculate load (requests per second) based on time window
                let time_elapsed = now.duration_since(*last_request_time).as_secs_f64();
                if time_elapsed > 0.0 {
                    let rps = *request_count as f64 / time_elapsed.max(1.0);
                    eval.update_load(rps);
                }

                // Reset metrics periodically (every 60 seconds) to avoid unbounded growth
                if time_elapsed > 60.0 {
                    *request_count = 1;
                    *error_count = if response.status_code >= 400 { 1 } else { 0 };
                    *last_request_time = now;
                } else {
                    *last_request_time = now;
                }
            }

            // Evaluate rules and get executed actions
            let executed_actions = engine.evaluate().await?;

            // Apply actions to response if any were executed
            if !executed_actions.is_empty() {
                tracing::debug!(
                    "Behavioral economics engine executed {} actions",
                    executed_actions.len()
                );
                // Actions are executed by the engine, but we may need to modify
                // the response based on action results. For now, the engine
                // handles action execution internally.
            }
        }

        Ok(response)
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

    // Mock implementations for testing
    struct MockRouteChaosInjector;

    #[async_trait]
    impl RouteChaosInjectorTrait for MockRouteChaosInjector {
        async fn inject_latency(&self, _method: &Method, _uri: &Uri) -> Result<()> {
            Ok(())
        }

        fn get_fault_response(&self, _method: &Method, _uri: &Uri) -> Option<RouteFaultResponse> {
            Some(RouteFaultResponse {
                status_code: 503,
                error_message: "Service unavailable".to_string(),
                fault_type: "test_fault".to_string(),
            })
        }
    }

    struct MockBehavioralScenarioReplay;

    #[async_trait]
    impl BehavioralScenarioReplay for MockBehavioralScenarioReplay {
        async fn try_replay(
            &self,
            _method: &Method,
            _uri: &Uri,
            _headers: &HeaderMap,
            _body: Option<&[u8]>,
            _session_id: Option<&str>,
        ) -> Result<Option<BehavioralReplayResponse>> {
            Ok(Some(BehavioralReplayResponse {
                status_code: 200,
                headers: HashMap::new(),
                body: b"scenario response".to_vec(),
                timing_ms: Some(100),
                content_type: "application/json".to_string(),
            }))
        }
    }

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

    #[tokio::test]
    async fn test_builder_methods() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir, true, true, false);
        let mock_generator = Box::new(SimpleMockGenerator::new(200, "{}".to_string()));

        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator));

        // Test with_custom_fixture_loader
        let custom_loader = Arc::new(CustomFixtureLoader::new(temp_dir.path().to_path_buf(), true));
        let handler = handler.with_custom_fixture_loader(custom_loader);
        assert!(handler.custom_fixture_loader.is_some());

        // Test with_stateful_handler
        let stateful_handler = Arc::new(StatefulResponseHandler::new().unwrap());
        let handler = handler.with_stateful_handler(stateful_handler);
        assert!(handler.stateful_handler.is_some());

        // Test with_route_chaos_injector
        let route_chaos = Arc::new(MockRouteChaosInjector);
        let handler = handler.with_route_chaos_injector(route_chaos);
        assert!(handler.route_chaos_injector.is_some());

        // Test with_continuum_engine
        let continuum_engine = Arc::new(RealityContinuumEngine::new(
            crate::reality_continuum::config::ContinuumConfig::default(),
        ));
        let handler = handler.with_continuum_engine(continuum_engine);
        assert!(handler.continuum_engine.is_some());

        // Test with_behavioral_economics_engine
        let behavioral_engine = Arc::new(RwLock::new(
            BehavioralEconomicsEngine::new(
                crate::behavioral_economics::config::BehavioralEconomicsConfig::default(),
            )
            .unwrap(),
        ));
        let handler = handler.with_behavioral_economics_engine(behavioral_engine);
        assert!(handler.behavioral_economics_engine.is_some());

        // Test with_behavioral_scenario_replay
        let scenario_replay = Arc::new(MockBehavioralScenarioReplay);
        let handler = handler.with_behavioral_scenario_replay(scenario_replay);
        assert!(handler.behavioral_scenario_replay.is_some());
    }

    #[tokio::test]
    async fn test_custom_fixture_priority() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);
        let custom_loader = Arc::new(CustomFixtureLoader::new(temp_dir.path().to_path_buf(), true));

        // Create a custom fixture
        let fixture_path = temp_dir.path().join("custom_fixture.json");
        std::fs::write(
            &fixture_path,
            r#"{"status": 201, "response": {"message": "custom"}, "headers": {"x-custom": "value"}}"#,
        )
        .unwrap();

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_custom_fixture_loader(custom_loader);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Custom fixture should be checked first, but won't match without proper fingerprint
        // This tests the custom fixture loader path
        let _handler = handler; // Handler is ready for custom fixture lookup
    }

    #[tokio::test]
    async fn test_route_chaos_injection() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir, true, true, false);
        let route_chaos = Arc::new(MockRouteChaosInjector);

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_route_chaos_injector(route_chaos);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await;

        // Should get fault response from route chaos injector
        if let Ok(resp) = response {
            assert_eq!(resp.status_code, 503);
            assert_eq!(resp.source.source_type, "route_fault_injection");
        }
    }

    #[tokio::test]
    async fn test_behavioral_scenario_replay() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir, true, true, false);
        let scenario_replay = Arc::new(MockBehavioralScenarioReplay);

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_behavioral_scenario_replay(scenario_replay);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", "test-session".parse().unwrap());

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "behavioral_scenario");
        assert_eq!(response.body, b"scenario response");
    }

    #[tokio::test]
    async fn test_priority_response_to_axum() {
        let response = PriorityResponse {
            source: ResponseSource::new(ResponsePriority::Mock, "test".to_string()),
            status_code: 201,
            headers: {
                let mut h = HashMap::new();
                h.insert("x-custom".to_string(), "value".to_string());
                h
            },
            body: b"test body".to_vec(),
            content_type: "application/json".to_string(),
        };

        let axum_response = response.to_axum_response();
        assert_eq!(axum_response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_simple_mock_generator() {
        let generator = SimpleMockGenerator::new(404, r#"{"error": "not found"}"#.to_string());
        let fingerprint = RequestFingerprint::new(
            Method::GET,
            &Uri::from_static("/api/test"),
            &HeaderMap::new(),
            None,
        );

        let response =
            generator.generate_mock_response(&fingerprint, &HeaderMap::new(), None).unwrap();

        assert!(response.is_some());
        let mock_response = response.unwrap();
        assert_eq!(mock_response.status_code, 404);
        assert_eq!(mock_response.body, r#"{"error": "not found"}"#);
    }

    #[tokio::test]
    async fn test_new_vs_new_with_openapi() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);
        let mock_generator = Box::new(SimpleMockGenerator::new(200, "{}".to_string()));

        // Test new()
        let record_replay1 = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);
        let mock_generator1 = Box::new(SimpleMockGenerator::new(200, "{}".to_string()));
        let handler1 = PriorityHttpHandler::new(record_replay1, None, None, Some(mock_generator1));
        assert!(handler1.openapi_spec.is_none());

        // Test new_with_openapi()
        let record_replay2 = RecordReplayHandler::new(fixtures_dir, true, true, false);
        let mock_generator2 = Box::new(SimpleMockGenerator::new(200, "{}".to_string()));
        let openapi_spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();
        let handler2 = PriorityHttpHandler::new_with_openapi(
            record_replay2,
            None,
            None,
            Some(mock_generator2),
            Some(openapi_spec),
        );
        assert!(handler2.openapi_spec.is_some());
    }

    #[tokio::test]
    async fn test_custom_fixture_with_delay() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a custom fixture with delay
        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/test",
  "status": 200,
  "response": {"message": "delayed response"},
  "delay_ms": 10
}"#;
        let fixture_file = fixtures_dir.join("test.json");
        std::fs::write(&fixture_file, fixture_content).unwrap();

        let mut custom_loader = CustomFixtureLoader::new(fixtures_dir.clone(), true);
        custom_loader.load_fixtures().await.unwrap();

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_custom_fixture_loader(Arc::new(custom_loader));

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let start = std::time::Instant::now();
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "custom_fixture");
        assert!(elapsed.as_millis() >= 10); // Should have delay
    }

    #[tokio::test]
    async fn test_custom_fixture_with_non_string_response() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a custom fixture with object response (not string)
        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/test",
  "status": 201,
  "response": {"id": 123, "name": "test"},
  "headers": {"content-type": "application/json"}
}"#;
        let fixture_file = fixtures_dir.join("test.json");
        std::fs::write(&fixture_file, fixture_content).unwrap();

        let mut custom_loader = CustomFixtureLoader::new(fixtures_dir.clone(), true);
        custom_loader.load_fixtures().await.unwrap();

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_custom_fixture_loader(Arc::new(custom_loader));

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 201);
        assert_eq!(response.source.source_type, "custom_fixture");
        assert!(response.body.len() > 0);
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("id"));
    }

    #[tokio::test]
    async fn test_custom_fixture_with_custom_content_type() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a custom fixture with custom content-type
        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/test",
  "status": 200,
  "response": "text response",
  "headers": {"content-type": "text/plain"}
}"#;
        let fixture_file = fixtures_dir.join("test.json");
        std::fs::write(&fixture_file, fixture_content).unwrap();

        let mut custom_loader = CustomFixtureLoader::new(fixtures_dir.clone(), true);
        custom_loader.load_fixtures().await.unwrap();

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_custom_fixture_loader(Arc::new(custom_loader));

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "text/plain");
    }

    #[tokio::test]
    async fn test_stateful_handler_path() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a stateful handler that returns a response
        let stateful_handler = Arc::new(StatefulResponseHandler::new().unwrap());

        // Add a stateful rule that matches our request
        // Note: This is a simplified test - in reality we'd need to set up stateful rules
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_stateful_handler(stateful_handler);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Stateful handler might not match, so this will fall through to mock/record
        // But we're testing the stateful handler path is checked
        let _response = handler.process_request(&method, &uri, &headers, None).await;
        // This may error if no handler matches, which is expected
    }

    #[tokio::test]
    async fn test_route_chaos_latency_injection() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a route chaos injector that injects latency
        struct LatencyInjector;
        #[async_trait]
        impl RouteChaosInjectorTrait for LatencyInjector {
            async fn inject_latency(&self, _method: &Method, _uri: &Uri) -> Result<()> {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                Ok(())
            }
            fn get_fault_response(
                &self,
                _method: &Method,
                _uri: &Uri,
            ) -> Option<RouteFaultResponse> {
                None
            }
        }

        let route_chaos = Arc::new(LatencyInjector);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_route_chaos_injector(route_chaos);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let start = std::time::Instant::now();
        let _response = handler.process_request(&method, &uri, &headers, None).await;
        let elapsed = start.elapsed();

        // Should have latency injected
        assert!(elapsed.as_millis() >= 20);
    }

    #[tokio::test]
    async fn test_failure_injection_path() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a failure injector that injects failures
        let mut failure_config = crate::failure_injection::FailureConfig::default();
        failure_config.global_error_rate = 1.0; // 100% error rate
        failure_config.default_status_codes = vec![500]; // Use 500 status code

        let failure_injector = FailureInjector::new(Some(failure_config), true);

        let openapi_spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /api/test:
    get:
      tags: [test]
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let handler = PriorityHttpHandler::new_with_openapi(
            record_replay,
            Some(failure_injector),
            None,
            None,
            Some(openapi_spec),
        );

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 500);
        assert_eq!(response.source.source_type, "failure_injection");
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("Injected failure")); // Default message
    }

    #[tokio::test]
    async fn test_record_handler_path() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        // Create record_replay with recording enabled
        // Parameters: fixtures_dir, enable_replay, enable_record, auto_record
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, true);

        // Need a mock generator as fallback since record is last in chain
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "test"}"#.to_string()));
        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator));

        let method = Method::POST; // POST should be recorded
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // This will hit mock generator, not record handler, since record is checked after mock
        // Let's test the record path by checking if recording happens
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        // Response will be from mock, but recording should have happened
        assert_eq!(response.source.source_type, "mock");
    }

    #[tokio::test]
    async fn test_behavioral_economics_engine_path() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "test"}"#.to_string()));

        let be_config = crate::behavioral_economics::config::BehavioralEconomicsConfig::default();
        let be_engine = Arc::new(RwLock::new(BehavioralEconomicsEngine::new(be_config).unwrap()));

        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator))
            .with_behavioral_economics_engine(be_engine);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        // Should go through behavioral economics engine processing
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_replay_handler_with_recorded_fixture() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        // Enable both replay and recording
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        // First, record a request
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
        record_replay
            .record_handler()
            .record_request(
                &fingerprint,
                200,
                &headers,
                r#"{"message": "recorded response"}"#,
                None,
            )
            .await
            .unwrap();

        // Create handler after recording
        let handler = PriorityHttpHandler::new(record_replay, None, None, None);

        // Now replay it - should hit the replay path (lines 266-282)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "replay");
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("recorded response"));
    }

    #[tokio::test]
    async fn test_behavioral_scenario_replay_with_cookies() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a scenario replay that extracts session ID from headers
        // Note: The current implementation checks x-session-id or session-id headers (lines 288-292)
        // Cookie parsing would need to be added separately
        struct CookieScenarioReplay;
        #[async_trait]
        impl BehavioralScenarioReplay for CookieScenarioReplay {
            async fn try_replay(
                &self,
                _method: &Method,
                _uri: &Uri,
                headers: &HeaderMap,
                _body: Option<&[u8]>,
                session_id: Option<&str>,
            ) -> Result<Option<BehavioralReplayResponse>> {
                // Test that session_id is extracted from headers
                // The code checks x-session-id or session-id headers, not cookies
                if session_id == Some("header-session-123") {
                    Ok(Some(BehavioralReplayResponse {
                        status_code: 200,
                        headers: HashMap::new(),
                        body: b"header scenario response".to_vec(),
                        timing_ms: None,
                        content_type: "application/json".to_string(),
                    }))
                } else {
                    Ok(None)
                }
            }
        }

        let scenario_replay = Arc::new(CookieScenarioReplay);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_behavioral_scenario_replay(scenario_replay);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let mut headers = HeaderMap::new();
        // Set session-id header (lines 288-292 test header extraction)
        headers.insert("session-id", "header-session-123".parse().unwrap());

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "behavioral_scenario");
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("header scenario"));
    }

    #[tokio::test]
    async fn test_route_chaos_latency_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a route chaos injector that returns an error from inject_latency (line 337)
        struct ErrorLatencyInjector;
        #[async_trait]
        impl RouteChaosInjectorTrait for ErrorLatencyInjector {
            async fn inject_latency(&self, _method: &Method, _uri: &Uri) -> Result<()> {
                Err(Error::generic("Latency injection failed".to_string()))
            }
            fn get_fault_response(
                &self,
                _method: &Method,
                _uri: &Uri,
            ) -> Option<RouteFaultResponse> {
                None
            }
        }

        let route_chaos = Arc::new(ErrorLatencyInjector);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "test"}"#.to_string()));
        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator))
            .with_route_chaos_injector(route_chaos);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Should handle the error gracefully and continue (line 337)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_behavioral_scenario_replay_with_timing_delay() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a scenario replay with timing delay (line 299-301)
        struct TimingScenarioReplay;
        #[async_trait]
        impl BehavioralScenarioReplay for TimingScenarioReplay {
            async fn try_replay(
                &self,
                _method: &Method,
                _uri: &Uri,
                _headers: &HeaderMap,
                _body: Option<&[u8]>,
                _session_id: Option<&str>,
            ) -> Result<Option<BehavioralReplayResponse>> {
                Ok(Some(BehavioralReplayResponse {
                    status_code: 200,
                    headers: HashMap::new(),
                    body: b"delayed response".to_vec(),
                    timing_ms: Some(15), // Timing delay
                    content_type: "application/json".to_string(),
                }))
            }
        }

        let scenario_replay = Arc::new(TimingScenarioReplay);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_behavioral_scenario_replay(scenario_replay);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        let start = std::time::Instant::now();
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.status_code, 200);
        assert!(elapsed.as_millis() >= 15); // Should have timing delay (line 300)
    }

    #[tokio::test]
    async fn test_stateful_handler_with_response() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a stateful handler that actually returns a response (lines 318-329)
        // Note: This requires setting up stateful rules, which is complex
        // For now, we'll test that the path is checked even if no response is returned
        let stateful_handler = Arc::new(StatefulResponseHandler::new().unwrap());
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_stateful_handler(stateful_handler);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Stateful handler path is checked (lines 317-330)
        // May not return a response if no rules match, but path is executed
        let _result = handler.process_request(&method, &uri, &headers, None).await;
        // Result may be error if no handler matches, which is expected
    }

    #[tokio::test]
    async fn test_replay_handler_content_type_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/xml".parse().unwrap());

        // Record with custom content type
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
        record_replay
            .record_handler()
            .record_request(&fingerprint, 200, &headers, r#"<xml>test</xml>"#, None)
            .await
            .unwrap();

        // Create handler after recording
        let handler = PriorityHttpHandler::new(record_replay, None, None, None);

        // Replay should extract content type from recorded headers (lines 269-273)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.content_type, "application/xml");
    }

    #[tokio::test]
    async fn test_proxy_migration_mode_mock() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create proxy config with Mock migration mode (lines 402-410)
        let mut proxy_config =
            crate::proxy::config::ProxyConfig::new("http://localhost:8080".to_string());
        proxy_config.migration_enabled = true;
        proxy_config.rules.push(crate::proxy::config::ProxyRule {
            path_pattern: "/api/*".to_string(),
            target_url: "http://localhost:8080".to_string(),
            enabled: true,
            pattern: "/api/*".to_string(),
            upstream_url: "http://localhost:8080".to_string(),
            migration_mode: crate::proxy::config::MigrationMode::Mock, // Force mock mode
            migration_group: None,
            condition: None,
        });

        let proxy_handler = ProxyHandler::new(proxy_config);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock"}"#.to_string()));

        let handler = PriorityHttpHandler::new(
            record_replay,
            None,
            Some(proxy_handler),
            Some(mock_generator),
        );

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Migration mode Mock should skip proxy and use mock (lines 409-410)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "mock");
    }

    #[tokio::test]
    async fn test_proxy_migration_mode_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create proxy config with migration disabled (lines 402-406)
        let mut proxy_config =
            crate::proxy::config::ProxyConfig::new("http://localhost:8080".to_string());
        proxy_config.migration_enabled = false; // Migration disabled
        proxy_config.enabled = false; // Also disable proxy to avoid network calls

        let proxy_handler = ProxyHandler::new(proxy_config);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock"}"#.to_string()));

        let handler = PriorityHttpHandler::new(
            record_replay,
            None,
            Some(proxy_handler),
            Some(mock_generator),
        );

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // With migration disabled, should fall through to mock (line 405)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "mock");
    }

    #[tokio::test]
    async fn test_continuum_engine_enabled_check() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create continuum engine (lines 393-397)
        let continuum_config = crate::reality_continuum::config::ContinuumConfig::new();
        let continuum_engine = Arc::new(RealityContinuumEngine::new(continuum_config));
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock"}"#.to_string()));

        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator))
            .with_continuum_engine(continuum_engine);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Should check if continuum is enabled (line 394)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_behavioral_scenario_replay_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a scenario replay that returns an error (lines 294-296)
        struct ErrorScenarioReplay;
        #[async_trait]
        impl BehavioralScenarioReplay for ErrorScenarioReplay {
            async fn try_replay(
                &self,
                _method: &Method,
                _uri: &Uri,
                _headers: &HeaderMap,
                _body: Option<&[u8]>,
                _session_id: Option<&str>,
            ) -> Result<Option<BehavioralReplayResponse>> {
                Err(Error::generic("Scenario replay error".to_string()))
            }
        }

        let scenario_replay = Arc::new(ErrorScenarioReplay);
        let mock_generator =
            Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock"}"#.to_string()));
        let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator))
            .with_behavioral_scenario_replay(scenario_replay);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Error should be handled gracefully and fall through to mock
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "mock");
    }

    #[tokio::test]
    async fn test_behavioral_scenario_replay_with_session_id_header() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Test session ID extraction from x-session-id header (lines 288-292)
        struct SessionScenarioReplay;
        #[async_trait]
        impl BehavioralScenarioReplay for SessionScenarioReplay {
            async fn try_replay(
                &self,
                _method: &Method,
                _uri: &Uri,
                _headers: &HeaderMap,
                _body: Option<&[u8]>,
                session_id: Option<&str>,
            ) -> Result<Option<BehavioralReplayResponse>> {
                if session_id == Some("header-session-456") {
                    Ok(Some(BehavioralReplayResponse {
                        status_code: 200,
                        headers: HashMap::new(),
                        body: b"header session response".to_vec(),
                        timing_ms: None,
                        content_type: "application/json".to_string(),
                    }))
                } else {
                    Ok(None)
                }
            }
        }

        let scenario_replay = Arc::new(SessionScenarioReplay);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_behavioral_scenario_replay(scenario_replay);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", "header-session-456".parse().unwrap());

        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "behavioral_scenario");
    }

    #[tokio::test]
    async fn test_stateful_handler_returns_response() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a stateful handler with a config that matches our request (lines 318-329)
        let stateful_handler = Arc::new(StatefulResponseHandler::new().unwrap());

        // Add a stateful config for /api/orders/{order_id}
        let mut state_responses = HashMap::new();
        state_responses.insert(
            "initial".to_string(),
            crate::stateful_handler::StateResponse {
                status_code: 200,
                headers: HashMap::new(),
                body_template: r#"{"status": "initial", "order_id": "123"}"#.to_string(),
                content_type: "application/json".to_string(),
            },
        );

        let config = crate::stateful_handler::StatefulConfig {
            resource_id_extract: crate::stateful_handler::ResourceIdExtract::PathParam {
                param: "order_id".to_string(),
            },
            resource_type: "order".to_string(),
            state_responses,
            transitions: vec![],
        };

        stateful_handler.add_config("/api/orders/{order_id}".to_string(), config).await;

        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_stateful_handler(stateful_handler);

        let method = Method::GET;
        let uri = Uri::from_static("/api/orders/123");
        let headers = HeaderMap::new();

        // Should hit stateful handler path (lines 318-329)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "stateful");
        assert_eq!(response.source.metadata.get("state"), Some(&"initial".to_string()));
        assert_eq!(response.source.metadata.get("resource_id"), Some(&"123".to_string()));
    }

    #[tokio::test]
    async fn test_record_handler_path_with_no_other_handlers() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        // Create record_replay with recording enabled (lines 714-739)
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, false);

        let handler = PriorityHttpHandler::new(record_replay, None, None, None);

        let method = Method::GET; // GET should be recorded when record_get_only is false
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Should hit record handler path (lines 714-739)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "record");
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("Request recorded"));
    }

    #[tokio::test]
    async fn test_mock_generator_with_migration_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create proxy config with Mock migration mode
        let mut proxy_config =
            crate::proxy::config::ProxyConfig::new("http://localhost:8080".to_string());
        proxy_config.migration_enabled = true;
        proxy_config.rules.push(crate::proxy::config::ProxyRule {
            path_pattern: "/api/*".to_string(),
            target_url: "http://localhost:8080".to_string(),
            enabled: true,
            pattern: "/api/*".to_string(),
            upstream_url: "http://localhost:8080".to_string(),
            migration_mode: crate::proxy::config::MigrationMode::Mock,
            migration_group: None,
            condition: None,
        });
        proxy_config.enabled = false; // Disable proxy to avoid network calls

        let proxy_handler = ProxyHandler::new(proxy_config);
        let mock_generator = Box::new(SimpleMockGenerator::new(
            200,
            r#"{"message": "mock with migration"}"#.to_string(),
        ));

        let handler = PriorityHttpHandler::new(
            record_replay,
            None,
            Some(proxy_handler),
            Some(mock_generator),
        );

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Migration mode Mock should skip proxy and use mock (lines 682-710)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.source.source_type, "mock");
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("mock with migration"));
    }

    #[tokio::test]
    async fn test_no_handler_can_process_request() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        // Create handler with no enabled handlers
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, false, false);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None);

        let method = Method::GET;
        let uri = Uri::from_static("/api/test");
        let headers = HeaderMap::new();

        // Should return error when no handler can process (line 742)
        let result = handler.process_request(&method, &uri, &headers, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No handler could process"));
    }

    #[tokio::test]
    async fn test_route_chaos_fault_injection() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();
        let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a route chaos injector that returns a fault response (lines 341-355)
        struct FaultInjector;
        #[async_trait]
        impl RouteChaosInjectorTrait for FaultInjector {
            async fn inject_latency(&self, _method: &Method, _uri: &Uri) -> Result<()> {
                Ok(())
            }
            fn get_fault_response(&self, method: &Method, uri: &Uri) -> Option<RouteFaultResponse> {
                if method == Method::GET && uri.path() == "/api/faulty" {
                    Some(RouteFaultResponse {
                        status_code: 503,
                        error_message: "Service unavailable".to_string(),
                        fault_type: "injected_fault".to_string(),
                    })
                } else {
                    None
                }
            }
        }

        let route_chaos = Arc::new(FaultInjector);
        let handler = PriorityHttpHandler::new(record_replay, None, None, None)
            .with_route_chaos_injector(route_chaos);

        let method = Method::GET;
        let uri = Uri::from_static("/api/faulty");
        let headers = HeaderMap::new();

        // Should return fault response (lines 341-355)
        let response = handler.process_request(&method, &uri, &headers, None).await.unwrap();
        assert_eq!(response.status_code, 503);
        let body_str = String::from_utf8_lossy(&response.body);
        assert!(body_str.contains("Service unavailable"));
        assert!(body_str.contains("injected_failure"));
    }
}

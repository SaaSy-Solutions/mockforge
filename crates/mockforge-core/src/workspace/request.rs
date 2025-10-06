//! Mock request handling and processing
//!
//! This module provides functionality for processing mock requests,
//! including request matching, response generation, and request execution.

use crate::cache::{Cache, ResponseCache, CachedResponse};
use crate::performance::PerformanceMonitor;
use crate::templating::TemplateEngine;
use crate::workspace::core::{EntityId, Folder, MockRequest, MockResponse, Workspace};
use crate::{
    routing::{HttpMethod, Route, RouteRegistry},
    Error, Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Request execution result
#[derive(Debug, Clone)]
pub struct RequestExecutionResult {
    /// Request ID that was executed
    pub request_id: EntityId,
    /// Response that was returned
    pub response: Option<MockResponse>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Request matching criteria
#[derive(Debug, Clone)]
pub struct RequestMatchCriteria {
    /// HTTP method
    pub method: HttpMethod,
    /// Request path/URL
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body content (optional)
    pub body: Option<String>,
}

/// Request processor for handling mock request execution
#[derive(Debug, Clone)]
pub struct RequestProcessor {
    /// Template engine for variable substitution
    _template_engine: TemplateEngine,
    /// Environment manager for variable resolution
    environment_manager: Option<crate::workspace::environment::EnvironmentManager>,
    /// Performance monitoring
    performance_monitor: Arc<PerformanceMonitor>,
    /// Response cache for frequently accessed responses
    response_cache: Arc<ResponseCache>,
    /// Request validation cache
    validation_cache: Arc<Cache<String, RequestValidationResult>>,
    /// Enable performance optimizations
    optimizations_enabled: bool,
}

/// Request validation result
#[derive(Debug, Clone)]
pub struct RequestValidationResult {
    /// Whether the request is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Request execution context
#[derive(Debug, Clone)]
pub struct RequestExecutionContext {
    /// Workspace ID
    pub workspace_id: EntityId,
    /// Environment variables
    pub environment_variables: HashMap<String, String>,
    /// Global headers
    pub global_headers: HashMap<String, String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Whether SSL verification is enabled
    pub ssl_verify: bool,
}

/// Request metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    /// Total requests executed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Most popular requests
    pub popular_requests: Vec<(EntityId, u64)>,
    /// Last execution timestamp
    pub last_execution: Option<DateTime<Utc>>,
}

impl RequestProcessor {
    /// Create a new request processor
    pub fn new() -> Self {
        Self {
            _template_engine: TemplateEngine::new(),
            environment_manager: None,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            response_cache: Arc::new(ResponseCache::new(1000, Duration::from_secs(300))), // 5 min TTL
            validation_cache: Arc::new(Cache::with_ttl(500, Duration::from_secs(60))), // 1 min TTL
            optimizations_enabled: true,
        }
    }

    /// Create a new request processor with environment manager
    pub fn with_environment_manager(
        environment_manager: crate::workspace::environment::EnvironmentManager,
    ) -> Self {
        Self {
            _template_engine: TemplateEngine::new(),
            environment_manager: Some(environment_manager),
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            response_cache: Arc::new(ResponseCache::new(1000, Duration::from_secs(300))),
            validation_cache: Arc::new(Cache::with_ttl(500, Duration::from_secs(60))),
            optimizations_enabled: true,
        }
    }

    /// Create a request processor with custom performance settings
    pub fn with_performance_config(
        environment_manager: Option<crate::workspace::environment::EnvironmentManager>,
        cache_size: usize,
        cache_ttl: Duration,
        enable_optimizations: bool,
    ) -> Self {
        Self {
            _template_engine: TemplateEngine::new(),
            environment_manager,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            response_cache: Arc::new(ResponseCache::new(cache_size, cache_ttl)),
            validation_cache: Arc::new(Cache::with_ttl(cache_size / 2, Duration::from_secs(60))),
            optimizations_enabled: enable_optimizations,
        }
    }

    /// Get performance monitor
    pub fn performance_monitor(&self) -> Arc<PerformanceMonitor> {
        self.performance_monitor.clone()
    }

    /// Enable or disable performance optimizations
    pub fn set_optimizations_enabled(&mut self, enabled: bool) {
        self.optimizations_enabled = enabled;
    }

    /// Find a request that matches the given criteria
    pub fn find_matching_request(
        &self,
        workspace: &Workspace,
        criteria: &RequestMatchCriteria,
    ) -> Option<EntityId> {
        // Search root requests
        for request in &workspace.requests {
            if self.request_matches(request, criteria) {
                return Some(request.id.clone());
            }
        }

        // Search folder requests
        if let Some(request_id) =
            self.find_matching_request_in_folders(&workspace.folders, criteria)
        {
            return Some(request_id);
        }

        None
    }

    /// Check if a request matches the given criteria
    fn request_matches(&self, request: &MockRequest, criteria: &RequestMatchCriteria) -> bool {
        // Check HTTP method
        if request.method != criteria.method {
            return false;
        }

        // Check URL pattern matching
        if !self.url_matches_pattern(&request.url, &criteria.path) {
            return false;
        }

        // Check query parameters
        for (key, expected_value) in &criteria.query_params {
            if let Some(actual_value) = request.query_params.get(key) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check headers (basic implementation)
        for (key, expected_value) in &criteria.headers {
            if let Some(actual_value) = request.headers.get(key) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Check if URL matches pattern
    pub fn url_matches_pattern(&self, pattern: &str, url: &str) -> bool {
        // Exact match
        if pattern == url {
            return true;
        }

        // Handle special case for root wildcard
        if pattern == "*" {
            return true;
        }

        // Handle wildcard patterns
        if pattern.contains('*') {
            return self.matches_path_pattern(pattern, url);
        }

        false
    }

    /// Check if a URL path matches a pattern with wildcards
    fn matches_path_pattern(&self, pattern: &str, path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        self.match_segments(&pattern_parts, &path_parts, 0, 0)
    }

    /// Recursive function to match path segments with wildcards
    fn match_segments(
        &self,
        pattern_parts: &[&str],
        path_parts: &[&str],
        pattern_idx: usize,
        path_idx: usize,
    ) -> bool {
        // If we've consumed both patterns and paths, it's a match
        if pattern_idx == pattern_parts.len() && path_idx == path_parts.len() {
            return true;
        }

        // If we've consumed the pattern but not the path, no match
        if pattern_idx == pattern_parts.len() {
            return false;
        }

        let current_pattern = pattern_parts[pattern_idx];

        match current_pattern {
            "*" => {
                // Single wildcard: try matching with current path segment
                if path_idx < path_parts.len() {
                    // Try consuming one segment
                    if self.match_segments(pattern_parts, path_parts, pattern_idx + 1, path_idx + 1)
                    {
                        return true;
                    }
                }
                false
            }
            "**" => {
                // Double wildcard: can match zero or more segments
                // Try matching zero segments (skip this pattern)
                if self.match_segments(pattern_parts, path_parts, pattern_idx + 1, path_idx) {
                    return true;
                }
                // Try matching one or more segments
                if path_idx < path_parts.len()
                    && self.match_segments(pattern_parts, path_parts, pattern_idx, path_idx + 1)
                {
                    return true;
                }
                false
            }
            _ => {
                // Exact match required
                if path_idx < path_parts.len() && current_pattern == path_parts[path_idx] {
                    return self.match_segments(
                        pattern_parts,
                        path_parts,
                        pattern_idx + 1,
                        path_idx + 1,
                    );
                }
                false
            }
        }
    }

    /// Find matching request in folder hierarchy
    fn find_matching_request_in_folders(
        &self,
        folders: &[Folder],
        criteria: &RequestMatchCriteria,
    ) -> Option<EntityId> {
        for folder in folders {
            // Search folder requests
            for request in &folder.requests {
                if self.request_matches(request, criteria) {
                    return Some(request.id.clone());
                }
            }

            // Search subfolders
            if let Some(request_id) =
                self.find_matching_request_in_folders(&folder.folders, criteria)
            {
                return Some(request_id);
            }
        }

        None
    }

    /// Execute a mock request
    pub async fn execute_request(
        &self,
        workspace: &mut Workspace,
        request_id: &EntityId,
        context: &RequestExecutionContext,
    ) -> Result<RequestExecutionResult> {
        // Start performance tracking
        let _perf_guard = if self.optimizations_enabled {
            self.performance_monitor.start_tracking_named("execute_request")
        } else {
            None
        };

        // Generate cache key for response caching if optimizations are enabled
        let cache_key = if self.optimizations_enabled {
            self.generate_response_cache_key(request_id, context)
        } else {
            String::new()
        };

        // Check response cache first
        if self.optimizations_enabled && !cache_key.is_empty() {
            if let Some(cached_response) = self.response_cache.get_response(&cache_key).await {
                self.performance_monitor.record_cache_hit();
                return Ok(RequestExecutionResult {
                    request_id: request_id.clone(),
                    response: Some(self.convert_cached_response_to_mock_response(cached_response)),
                    duration_ms: 1, // Cached responses are nearly instant
                    success: true,
                    error: None,
                });
            } else {
                self.performance_monitor.record_cache_miss();
            }
        }

        // Find the request
        let request = self
            .find_request_in_workspace(workspace, request_id)
            .ok_or_else(|| {
                if self.optimizations_enabled {
                    self.performance_monitor.record_error();
                }
                format!("Request with ID {} not found", request_id)
            })?;

        let start_time = std::time::Instant::now();

        // Validate request with caching
        let validation = self.validate_request_cached(request, context).await?;
        if !validation.is_valid {
            if self.optimizations_enabled {
                self.performance_monitor.record_error();
            }
            return Err(Error::Validation {
                message: format!("Request validation failed: {:?}", validation.errors),
            });
        }

        // Get active response
        let response = request
            .active_response()
            .ok_or_else(|| {
                if self.optimizations_enabled {
                    self.performance_monitor.record_error();
                }
                Error::generic("No active response found for request")
            })?;

        // Apply variable substitution
        let processed_response = self.process_response(response, context).await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Cache the response if optimizations are enabled
        if self.optimizations_enabled && !cache_key.is_empty() {
            let cached_response = self.convert_mock_response_to_cached_response(&processed_response);
            self.response_cache.cache_response(cache_key, cached_response).await;
        }

        // Record response usage
        if let Some(request_mut) = self.find_request_in_workspace_mut(workspace, request_id) {
            if let Some(response_mut) = request_mut.active_response_mut() {
                response_mut.record_usage(request_id.clone(), duration_ms);
            }
        }

        Ok(RequestExecutionResult {
            request_id: request_id.clone(),
            response: Some(processed_response),
            duration_ms,
            success: true,
            error: None,
        })
    }

    /// Find request in workspace (mutable)
    fn find_request_in_workspace_mut<'a>(
        &self,
        workspace: &'a mut Workspace,
        request_id: &EntityId,
    ) -> Option<&'a mut MockRequest> {
        // Search root requests
        for request in &mut workspace.requests {
            if &request.id == request_id {
                return Some(request);
            }
        }

        // Search folder requests
        self.find_request_in_folders_mut(&mut workspace.folders, request_id)
    }

    /// Find request in folder hierarchy (mutable)
    fn find_request_in_folders_mut<'a>(
        &self,
        folders: &'a mut [Folder],
        request_id: &EntityId,
    ) -> Option<&'a mut MockRequest> {
        for folder in folders {
            // Search folder requests
            for request in &mut folder.requests {
                if &request.id == request_id {
                    return Some(request);
                }
            }

            // Search subfolders
            if let Some(request) = self.find_request_in_folders_mut(&mut folder.folders, request_id)
            {
                return Some(request);
            }
        }

        None
    }

    /// Find request in workspace (immutable)
    fn find_request_in_workspace<'a>(
        &self,
        workspace: &'a Workspace,
        request_id: &EntityId,
    ) -> Option<&'a MockRequest> {
        // Search root requests
        workspace
            .requests
            .iter()
            .find(|r| &r.id == request_id)
            .or_else(|| self.find_request_in_folders(&workspace.folders, request_id))
    }

    /// Find request in folder hierarchy (immutable)
    fn find_request_in_folders<'a>(
        &self,
        folders: &'a [Folder],
        request_id: &EntityId,
    ) -> Option<&'a MockRequest> {
        for folder in folders {
            // Search folder requests
            if let Some(request) = folder.requests.iter().find(|r| &r.id == request_id) {
                return Some(request);
            }

            // Search subfolders
            if let Some(request) = self.find_request_in_folders(&folder.folders, request_id) {
                return Some(request);
            }
        }

        None
    }

    /// Validate a request
    pub fn validate_request(
        &self,
        request: &MockRequest,
        _context: &RequestExecutionContext,
    ) -> RequestValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if request is enabled
        if !request.enabled {
            errors.push("Request is disabled".to_string());
        }

        // Validate URL
        if request.url.is_empty() {
            errors.push("Request URL cannot be empty".to_string());
        }

        // Validate method
        match request.method {
            HttpMethod::GET
            | HttpMethod::POST
            | HttpMethod::PUT
            | HttpMethod::DELETE
            | HttpMethod::PATCH
            | HttpMethod::HEAD
            | HttpMethod::OPTIONS => {
                // Valid methods
            }
        }

        // Check for active response
        if request.active_response().is_none() {
            warnings.push("No active response configured".to_string());
        }

        // Validate responses
        for response in &request.responses {
            if response.status_code < 100 || response.status_code > 599 {
                errors.push(format!("Invalid status code: {}", response.status_code));
            }

            if response.body.is_empty() {
                warnings.push(format!("Response '{}' has empty body", response.name));
            }
        }

        RequestValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Process response with variable substitution and delays
    async fn process_response(
        &self,
        response: &MockResponse,
        context: &RequestExecutionContext,
    ) -> Result<MockResponse> {
        // Apply delay if configured
        if response.delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(response.delay)).await;
        }

        // Create processed response
        let mut processed_response = response.clone();

        // Apply environment variable substitution
        if let Some(env_manager) = &self.environment_manager {
            if let Some(_env_vars) = self.get_environment_variables(context) {
                processed_response.body = env_manager.substitute_variables(&response.body).value;
            }
        }

        Ok(processed_response)
    }

    /// Get environment variables for context
    fn get_environment_variables(
        &self,
        context: &RequestExecutionContext,
    ) -> Option<HashMap<String, String>> {
        if let Some(env_manager) = &self.environment_manager {
            if let Some(active_env) = env_manager.get_active_environment() {
                return Some(active_env.variables.clone());
            }
        }

        Some(context.environment_variables.clone())
    }

    /// Get request metrics for a workspace
    pub fn get_request_metrics(&self, workspace: &Workspace) -> RequestMetrics {
        let mut total_requests = 0u64;
        let mut successful_requests = 0u64;
        let mut failed_requests = 0u64;
        let mut total_response_time = 0u64;
        let mut request_counts = HashMap::new();
        let mut last_execution: Option<DateTime<Utc>> = None;

        // Collect metrics from all requests
        for request in &workspace.requests {
            total_requests += 1;

            // Count executions from response history
            for response in &request.responses {
                let execution_count = response.history.len() as u64;
                *request_counts.entry(request.id.clone()).or_insert(0) += execution_count;

                for entry in &response.history {
                    total_response_time += entry.duration_ms;

                    // Update last execution timestamp
                    if let Some(current_last) = last_execution {
                        if entry.timestamp > current_last {
                            last_execution = Some(entry.timestamp);
                        }
                    } else {
                        last_execution = Some(entry.timestamp);
                    }

                    // Simple success determination (could be improved)
                    if entry.duration_ms < 5000 {
                        // Less than 5 seconds
                        successful_requests += 1;
                    } else {
                        failed_requests += 1;
                    }
                }
            }
        }

        // Also collect from folder requests
        self.collect_folder_request_metrics(
            &workspace.folders,
            &mut total_requests,
            &mut successful_requests,
            &mut failed_requests,
            &mut total_response_time,
            &mut request_counts,
            &mut last_execution,
        );

        let average_response_time = if total_requests > 0 {
            total_response_time as f64 / total_requests as f64
        } else {
            0.0
        };

        // Get popular requests (top 5)
        let mut popular_requests: Vec<_> = request_counts.into_iter().collect();
        popular_requests.sort_by(|a, b| b.1.cmp(&a.1));
        popular_requests.truncate(5);

        RequestMetrics {
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time_ms: average_response_time,
            popular_requests,
            last_execution,
        }
    }

    /// Collect metrics from folder requests
    fn collect_folder_request_metrics(
        &self,
        folders: &[Folder],
        total_requests: &mut u64,
        successful_requests: &mut u64,
        failed_requests: &mut u64,
        total_response_time: &mut u64,
        request_counts: &mut HashMap<EntityId, u64>,
        last_execution: &mut Option<DateTime<Utc>>,
    ) {
        for folder in folders {
            for request in &folder.requests {
                *total_requests += 1;

                // Count executions from response history
                for response in &request.responses {
                    let execution_count = response.history.len() as u64;
                    *request_counts.entry(request.id.clone()).or_insert(0) += execution_count;

                    for entry in &response.history {
                        *total_response_time += entry.duration_ms;

                        // Update last execution timestamp
                        if let Some(current_last) = *last_execution {
                            if entry.timestamp > current_last {
                                *last_execution = Some(entry.timestamp);
                            }
                        } else {
                            *last_execution = Some(entry.timestamp);
                        }

                        // Simple success determination
                        if entry.duration_ms < 5000 {
                            *successful_requests += 1;
                        } else {
                            *failed_requests += 1;
                        }
                    }
                }
            }

            // Recurse into subfolders
            self.collect_folder_request_metrics(
                &folder.folders,
                total_requests,
                successful_requests,
                failed_requests,
                total_response_time,
                request_counts,
                last_execution,
            );
        }
    }

    /// Create a route from a mock request
    pub fn create_route_from_request(&self, request: &MockRequest) -> Result<Route> {
        if !request.enabled {
            return Err(Error::validation("Request is disabled"));
        }

        let response = request
            .active_response()
            .ok_or_else(|| Error::validation("No active response found"))?;

        // Create route with request information
        let mut route = Route::new(request.method.clone(), request.url.clone());

        // Store additional data in metadata
        route.metadata.insert("id".to_string(), serde_json::json!(request.id));
        route.metadata.insert("response".to_string(), serde_json::json!(response.body));
        route
            .metadata
            .insert("status_code".to_string(), serde_json::json!(response.status_code));
        route.metadata.insert("headers".to_string(), serde_json::json!(request.headers));
        route
            .metadata
            .insert("query_params".to_string(), serde_json::json!(request.query_params));
        route.metadata.insert("enabled".to_string(), serde_json::json!(request.enabled));
        route
            .metadata
            .insert("created_at".to_string(), serde_json::json!(request.created_at));
        route
            .metadata
            .insert("updated_at".to_string(), serde_json::json!(request.updated_at));

        Ok(route)
    }

    /// Update route registry with workspace requests
    pub fn update_route_registry(
        &self,
        workspace: &Workspace,
        route_registry: &mut RouteRegistry,
    ) -> Result<()> {
        route_registry.clear();

        // Add root requests
        for request in &workspace.requests {
            if request.enabled {
                if let Ok(route) = self.create_route_from_request(request) {
                    let _ = route_registry.add_route(route);
                }
            }
        }

        // Add folder requests
        self.add_folder_routes_to_registry(&workspace.folders, route_registry)?;

        Ok(())
    }

    /// Add folder requests to route registry
    fn add_folder_routes_to_registry(
        &self,
        folders: &[Folder],
        route_registry: &mut RouteRegistry,
    ) -> Result<()> {
        for folder in folders {
            for request in &folder.requests {
                if request.enabled {
                    if let Ok(route) = self.create_route_from_request(request) {
                        let _ = route_registry.add_route(route);
                    }
                }
            }

            // Recurse into subfolders
            self.add_folder_routes_to_registry(&folder.folders, route_registry)?;
        }

        Ok(())
    }

    // Performance optimization helper methods

    /// Generate cache key for response caching
    fn generate_response_cache_key(&self, request_id: &EntityId, context: &RequestExecutionContext) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request_id.hash(&mut hasher);
        context.workspace_id.hash(&mut hasher);

        // Hash environment variables
        for (key, value) in &context.environment_variables {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }

        // Hash global headers
        for (key, value) in &context.global_headers {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }

        format!("req_{}_{}", hasher.finish(), request_id)
    }

    /// Validate request with caching
    async fn validate_request_cached(
        &self,
        request: &MockRequest,
        context: &RequestExecutionContext,
    ) -> Result<RequestValidationResult> {
        if !self.optimizations_enabled {
            return Ok(self.validate_request(request, context));
        }

        // Generate validation cache key
        let cache_key = format!("val_{}_{}", request.id, context.workspace_id);

        // Check cache first
        if let Some(cached_result) = self.validation_cache.get(&cache_key).await {
            return Ok(cached_result);
        }

        // Perform validation
        let result = self.validate_request(request, context);

        // Cache the result
        self.validation_cache.insert(cache_key, result.clone(), None).await;

        Ok(result)
    }

    /// Convert MockResponse to CachedResponse
    fn convert_mock_response_to_cached_response(&self, response: &MockResponse) -> CachedResponse {
        CachedResponse {
            status_code: response.status_code,
            headers: response.headers.clone(),
            body: response.body.clone(),
            content_type: response.headers.get("Content-Type").cloned(),
        }
    }

    /// Convert CachedResponse to MockResponse
    fn convert_cached_response_to_mock_response(&self, cached: CachedResponse) -> MockResponse {
        MockResponse {
            id: EntityId::new(),
            name: "Cached Response".to_string(),
            status_code: cached.status_code,
            headers: cached.headers,
            body: cached.body,
            delay: 0, // Cached responses have no additional delay
            active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            history: Vec::new(),
        }
    }

    /// Get performance summary
    pub async fn get_performance_summary(&self) -> crate::performance::PerformanceSummary {
        self.performance_monitor.get_summary().await
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (crate::cache::CacheStats, crate::cache::CacheStats) {
        let response_cache_stats = self.response_cache.stats().await;
        let validation_cache_stats = self.validation_cache.stats().await;
        (response_cache_stats, validation_cache_stats)
    }

    /// Clear all caches
    pub async fn clear_caches(&self) {
        self.response_cache.get_response("").await; // Dummy call to access underlying cache
        self.validation_cache.clear().await;
    }
}

impl Default for RequestProcessor {
    fn default() -> Self {
        Self::new()
    }
}

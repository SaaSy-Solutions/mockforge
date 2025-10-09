//! Unified middleware implementations for common patterns across protocols

use super::{Protocol, ProtocolMiddleware, ProtocolRequest, ProtocolResponse};
use crate::{request_logger::log_request_global, Result};
use std::time::Instant;

/// Logging middleware that works across all protocols
pub struct LoggingMiddleware {
    /// Middleware name
    name: String,
    /// Whether to log request bodies
    log_bodies: bool,
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    pub fn new(log_bodies: bool) -> Self {
        Self {
            name: "LoggingMiddleware".to_string(),
            log_bodies,
        }
    }
}

#[async_trait::async_trait]
impl ProtocolMiddleware for LoggingMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Add timestamp to request metadata
        let timestamp = chrono::Utc::now().to_rfc3339();
        request
            .metadata
            .insert("x-mockforge-request-time".to_string(), timestamp);

        // Store start time for duration calculation
        request.metadata.insert(
            "x-mockforge-request-start".to_string(),
            Instant::now().elapsed().as_millis().to_string(),
        );

        tracing::debug!(
            protocol = %request.protocol,
            operation = %request.operation,
            path = %request.path,
            "Processing request through logging middleware"
        );

        Ok(())
    }

    async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()> {
        let duration_ms = if let Some(start) = request.metadata.get("x-mockforge-request-start") {
            let start: u128 = start.parse().unwrap_or(0);
            Instant::now().elapsed().as_millis() - start
        } else {
            0
        };

        // Create appropriate log entry based on protocol
        let log_entry = match request.protocol {
            Protocol::Http => crate::create_http_log_entry(
                &request.operation,
                &request.path,
                response.status.as_code().unwrap_or(0) as u16,
                duration_ms as u64,
                request.client_ip.clone(),
                request.metadata.get("user-agent").cloned(),
                request.metadata.clone(),
                response.body.len() as u64,
                if !response.status.is_success() {
                    Some(format!("Error response: {:?}", response.status))
                } else {
                    None
                },
            ),
            Protocol::Grpc => {
                // Extract service and method from operation (e.g., "greeter.SayHello")
                let parts: Vec<&str> = request.operation.split('.').collect();
                let (service, method) = if parts.len() == 2 {
                    (parts[0], parts[1])
                } else {
                    ("unknown", request.operation.as_str())
                };
                crate::create_grpc_log_entry(
                    service,
                    method,
                    response.status.as_code().unwrap_or(0) as u16,
                    duration_ms as u64,
                    request.client_ip.clone(),
                    request.body.as_ref().map(|b| b.len() as u64).unwrap_or(0),
                    response.body.len() as u64,
                    if !response.status.is_success() {
                        Some(format!("Error response: {:?}", response.status))
                    } else {
                        None
                    },
                )
            }
            Protocol::GraphQL => crate::create_http_log_entry(
                "GraphQL",
                &request.path,
                if response.status.is_success() { 200 } else { 400 },
                duration_ms as u64,
                request.client_ip.clone(),
                request.metadata.get("user-agent").cloned(),
                request.metadata.clone(),
                response.body.len() as u64,
                None,
            ),
            Protocol::WebSocket => crate::create_websocket_log_entry(
                &request.operation,
                &request.path,
                response.status.as_code().unwrap_or(0) as u16,
                request.client_ip.clone(),
                response.body.len() as u64,
                if !response.status.is_success() {
                    Some(format!("Error response: {:?}", response.status))
                } else {
                    None
                },
            ),
            Protocol::Smtp => crate::create_http_log_entry(
                "SMTP",
                &request.path,
                response.status.as_code().unwrap_or(250) as u16,
                duration_ms as u64,
                request.client_ip.clone(),
                None,
                request.metadata.clone(),
                response.body.len() as u64,
                if !response.status.is_success() {
                    Some(format!("SMTP Error: {:?}", response.status))
                } else {
                    None
                },
            ),
        };

        // Log to centralized logger
        log_request_global(log_entry).await;

        tracing::debug!(
            protocol = %request.protocol,
            operation = %request.operation,
            path = %request.path,
            duration_ms = duration_ms,
            success = response.status.is_success(),
            "Request processed"
        );

        Ok(())
    }

    fn supports_protocol(&self, _protocol: Protocol) -> bool {
        // Logging middleware supports all protocols
        true
    }
}

/// Metrics middleware that collects metrics across all protocols
pub struct MetricsMiddleware {
    /// Middleware name
    name: String,
}

impl MetricsMiddleware {
    /// Create a new metrics middleware
    pub fn new() -> Self {
        Self {
            name: "MetricsMiddleware".to_string(),
        }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ProtocolMiddleware for MetricsMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Store start time for metrics calculation
        request.metadata.insert(
            "x-mockforge-metrics-start".to_string(),
            std::time::Instant::now().elapsed().as_millis().to_string(),
        );

        tracing::debug!(
            protocol = %request.protocol,
            operation = %request.operation,
            "Metrics: request started"
        );

        Ok(())
    }

    async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()> {
        let duration_ms = if let Some(start) = request.metadata.get("x-mockforge-metrics-start") {
            let start: u128 = start.parse().unwrap_or(0);
            Instant::now().elapsed().as_millis() - start
        } else {
            0
        };

        let status_code = response.status.as_code().unwrap_or(0);

        tracing::info!(
            protocol = %request.protocol,
            operation = %request.operation,
            status_code = status_code,
            duration_ms = duration_ms,
            response_size = response.body.len(),
            success = response.status.is_success(),
            "Metrics: request completed"
        );

        Ok(())
    }

    fn supports_protocol(&self, _protocol: Protocol) -> bool {
        // Metrics middleware supports all protocols
        true
    }
}

/// Latency injection middleware for simulating delays
pub struct LatencyMiddleware {
    /// Middleware name
    name: String,
    /// Latency injector
    injector: crate::latency::LatencyInjector,
}

impl LatencyMiddleware {
    /// Create a new latency middleware
    pub fn new(injector: crate::latency::LatencyInjector) -> Self {
        Self {
            name: "LatencyMiddleware".to_string(),
            injector,
        }
    }
}

#[async_trait::async_trait]
impl ProtocolMiddleware for LatencyMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Extract tags from request metadata
        let tags: Vec<String> = request
            .metadata
            .get("x-mockforge-tags")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        // Inject latency
        self.injector.inject_latency(&tags).await?;

        Ok(())
    }

    async fn process_response(
        &self,
        _request: &ProtocolRequest,
        _response: &mut ProtocolResponse,
    ) -> Result<()> {
        // No post-processing needed for latency
        Ok(())
    }

    fn supports_protocol(&self, _protocol: Protocol) -> bool {
        // Latency middleware supports all protocols
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_logging_middleware_creation() {
        let middleware = LoggingMiddleware::new(true);
        assert_eq!(middleware.name(), "LoggingMiddleware");
        assert!(middleware.supports_protocol(Protocol::Http));
        assert!(middleware.supports_protocol(Protocol::GraphQL));
        assert!(middleware.supports_protocol(Protocol::Grpc));
    }

    #[test]
    fn test_metrics_middleware_creation() {
        let middleware = MetricsMiddleware::new();
        assert_eq!(middleware.name(), "MetricsMiddleware");
        assert!(middleware.supports_protocol(Protocol::Http));
        assert!(middleware.supports_protocol(Protocol::GraphQL));
    }

    #[test]
    fn test_latency_middleware_creation() {
        let injector = crate::latency::LatencyInjector::default();
        let middleware = LatencyMiddleware::new(injector);
        assert_eq!(middleware.name(), "LatencyMiddleware");
        assert!(middleware.supports_protocol(Protocol::Http));
    }

    #[tokio::test]
    async fn test_logging_middleware_process_request() {
        let middleware = LoggingMiddleware::new(false);
        let mut request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let result = middleware.process_request(&mut request).await;
        assert!(result.is_ok());
        assert!(request.metadata.contains_key("x-mockforge-request-time"));
    }
}

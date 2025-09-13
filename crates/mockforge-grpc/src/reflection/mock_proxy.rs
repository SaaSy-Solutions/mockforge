//! Mock-enabled reflection proxy implementation
//!
//! This module provides a reflection proxy that can serve mock responses
//! instead of forwarding requests to other servers.

use crate::reflection::{
    cache::DescriptorCache,
    config::ProxyConfig,
    connection_pool::ConnectionPool,
};
use crate::dynamic::ServiceRegistry;
use prost_reflect::{DynamicMessage, ReflectMessage};
use prost_reflect::prost::Message;
use prost_types::Any;
use std::sync::Arc;
use std::time::Duration;
use tonic::{
    Request, Response, Status, Streaming,
};
use tracing::{debug, info, warn};

/// A mock-enabled reflection proxy that serves mock responses
pub struct MockReflectionProxy {
    /// Cache of service and method descriptors
    cache: DescriptorCache,
    /// Proxy configuration
    config: ProxyConfig,
    /// Timeout for requests
    timeout_duration: Duration,
    /// Connection pool for gRPC channels
    connection_pool: ConnectionPool,
    /// Registry of dynamic services for mock responses
    service_registry: Arc<ServiceRegistry>,
}

impl MockReflectionProxy {
    /// Create a new mock reflection proxy
    pub async fn new(
        config: ProxyConfig,
        service_registry: Arc<ServiceRegistry>,
    ) -> Result<Self, Status> {
        debug!("Creating mock reflection proxy with {} services", service_registry.service_names().len());

        let cache = DescriptorCache::new();

        // Populate cache from service registry's descriptor pool
        cache.populate_from_pool(service_registry.descriptor_pool()).await;

        Ok(Self {
            cache,
            config,
            timeout_duration: Duration::from_secs(30),
            connection_pool: ConnectionPool::new(),
            service_registry,
        })
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    /// Handle a unary request with mock response
    pub async fn handle_unary(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Handling unary request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // Get the dynamic service from our registry
        if let Some(dynamic_service) = self.service_registry.get(service_name) {
            // Convert DynamicMessage to Any for our service
            let request_any = self.convert_dynamic_message_to_any(&request.into_inner())?;
            let request_any = Request::new(request_any);

            // Handle with our dynamic service
            let response_any = dynamic_service.handle_unary(method_name, request_any).await?;

            // Convert back to DynamicMessage
            let response_dynamic = self.convert_any_to_dynamic_message(&response_any.into_inner())?;

            Ok(Response::new(response_dynamic))
        } else {
            // Service not found in our registry, return a generic mock response
            warn!("Service {} not found in registry, returning generic mock", service_name);
            self.generate_generic_mock_response(service_name, method_name).await
        }
    }

    /// Handle a server-streaming request with mock response
    pub async fn handle_server_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<DynamicMessage>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        debug!("Handling server streaming request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // Get the dynamic service from our registry
        if let Some(dynamic_service) = self.service_registry.get(service_name) {
            // Convert DynamicMessage to Any for our service
            let request_any = self.convert_dynamic_message_to_any(&request.into_inner())?;
            let request_any = Request::new(request_any);

            // Handle with our dynamic service
            let _response_stream = dynamic_service.handle_server_streaming(method_name, request_any).await?;

            // For now, return unimplemented - proper streaming implementation is complex
            Err(Status::unimplemented("Server streaming mock response not yet implemented"))
        } else {
            // Service not found in our registry, return a generic mock response
            warn!("Service {} not found in registry, returning generic mock for server streaming", service_name);
            Err(Status::unimplemented(format!("Server streaming mock response for {}.{} not yet implemented", service_name, method_name)))
        }
    }

    /// Handle a client-streaming request with mock response
    pub async fn handle_client_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        _request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Handling client streaming request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // For now, return an error since streaming is complex
        // In a full implementation, we would handle streaming properly
        Err(Status::unimplemented("Client streaming not yet implemented in mock proxy"))
    }

    /// Handle a bidirectional streaming request with mock response
    pub async fn handle_bidirectional_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        _request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        debug!("Handling bidirectional streaming request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // For now, return unimplemented - proper stream conversion is complex
        // TODO: Implement proper streaming conversion between DynamicMessage and Any
        Err(Status::unimplemented("Bidirectional streaming mock response not yet implemented"))
    }

    /// Convert DynamicMessage to Any for our service handlers
    fn convert_dynamic_message_to_any(&self, message: &DynamicMessage) -> Result<Any, Status> {
        // This is a simplified conversion - in a real implementation,
        // we would properly serialize the DynamicMessage
        let type_url = format!("type.googleapis.com/{}", message.descriptor().full_name());
        let value = message.encode_to_vec();

        Ok(Any {
            type_url,
            value,
        })
    }

    /// Convert Any back to DynamicMessage for responses
    fn convert_any_to_dynamic_message(&self, _any: &Any) -> Result<DynamicMessage, Status> {
        // This is a simplified conversion - in a real implementation,
        // we would properly deserialize the Any back to DynamicMessage
        // For now, we'll create a simple mock response
        Err(Status::unimplemented("DynamicMessage conversion not yet implemented"))
    }

    /// Generate mock stream messages for testing
    async fn generate_mock_stream_messages(&self, _count: usize) -> Result<Vec<DynamicMessage>, Status> {
        // For now, return an error - proper implementation needs correct MessageDescriptor
        Err(Status::unimplemented("Mock stream message generation not yet implemented"))
    }

    /// Generate a generic mock response when service is not found
    async fn generate_generic_mock_response(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<DynamicMessage>, Status> {
        info!("Generating generic mock response for {}.{}", service_name, method_name);

        // For now, return an error since DynamicMessage creation is complex
        // In a full implementation, we would create proper DynamicMessage responses
        Err(Status::unimplemented(format!("Mock response for {}.{} not yet implemented", service_name, method_name)))
    }

    /// Get the service registry
    pub fn service_registry(&self) -> &Arc<ServiceRegistry> {
        &self.service_registry
    }

    /// Get the configuration
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

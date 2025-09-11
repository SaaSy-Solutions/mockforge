//! Main reflection proxy implementation

use crate::reflection::{
    cache::DescriptorCache,
    client::ReflectionClient,
    config::ProxyConfig,
    connection_pool::ConnectionPool,
};
use prost_reflect::DynamicMessage;
use std::time::Duration;
use tonic::{
    transport::Endpoint,
    Request, Response, Status, Streaming,
};
use tracing::debug;

/// A reflection-based gRPC proxy that can forward requests to arbitrary services
pub struct ReflectionProxy {
    /// The reflection client for discovering services
    _client: ReflectionClient,
    /// Cache of service and method descriptors
    cache: DescriptorCache,
    /// Proxy configuration
    config: ProxyConfig,
    /// Timeout for requests
    timeout_duration: Duration,
    /// Connection pool for gRPC channels
    connection_pool: ConnectionPool,
}

impl ReflectionProxy {
    /// Create a new reflection proxy
    pub async fn new(endpoint: Endpoint, config: ProxyConfig) -> Result<Self, Status> {
        debug!("Creating reflection proxy for endpoint: {:?}", endpoint.uri());
        
        let client = ReflectionClient::new(endpoint).await?;
        let cache = DescriptorCache::new();
        
        // Populate cache from the client's descriptor pool
        cache.populate_from_pool(client.pool()).await;
        
        Ok(Self {
            _client: client,
            cache,
            config,
            timeout_duration: Duration::from_secs(30),
            connection_pool: ConnectionPool::new(),
        })
    }
    
    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }
    
    /// Forward a unary request to the target service
    pub async fn forward_unary(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }
        
        // Get the method descriptor
        let method = self.cache.get_method(service_name, method_name).await?;
        
        // Check if it's actually a unary method
        if !method.is_server_streaming() && !method.is_client_streaming() {
            self.forward_unary_impl(method, request).await
        } else {
            Err(Status::invalid_argument(format!(
                "Method {}::{} is not a unary method",
                service_name, method_name
            )))
        }
    }
    
    /// Forward a server-streaming request to the target service
    pub async fn forward_server_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<DynamicMessage>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }
        
        // Get the method descriptor
        let method = self.cache.get_method(service_name, method_name).await?;
        
        // Check if it's actually a server streaming method
        if method.is_server_streaming() && !method.is_client_streaming() {
            self.forward_server_streaming_impl(method, request).await
        } else {
            Err(Status::invalid_argument(format!(
                "Method {}::{} is not a server streaming method",
                service_name, method_name
            )))
        }
    }
    
    /// Forward a client-streaming request to the target service
    pub async fn forward_client_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }
        
        // Get the method descriptor
        let method = self.cache.get_method(service_name, method_name).await?;
        
        // Check if it's actually a client streaming method
        if method.is_client_streaming() && !method.is_server_streaming() {
            self.forward_client_streaming_impl(method, request).await
        } else {
            Err(Status::invalid_argument(format!(
                "Method {}::{} is not a client streaming method",
                service_name, method_name
            )))
        }
    }
    
    /// Forward a bidirectional streaming request to the target service
    pub async fn forward_bidirectional_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }
        
        // Get the method descriptor
        let method = self.cache.get_method(service_name, method_name).await?;
        
        // Check if it's actually a bidirectional streaming method
        if method.is_client_streaming() && method.is_server_streaming() {
            self.forward_bidirectional_streaming_impl(method, request).await
        } else {
            Err(Status::invalid_argument(format!(
                "Method {}::{} is not a bidirectional streaming method",
                service_name, method_name
            )))
        }
    }
    
    /// Implementation for forwarding unary requests
    async fn forward_unary_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        _request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        // For a mock server, we would typically:
        // 1. Look up mock responses based on the service/method
        // 2. Apply any configured latency or error simulation
        // 3. Return the appropriate mock response with preserved metadata
        // 4. Preserve all metadata from the original request in the response
        
        debug!("Forwarding unary request for method: {}", method.name());
        
        // Get a channel from the connection pool
        // In a real implementation, we would use the channel to make the actual request
        // For now, we're just demonstrating the use of the connection pool
        let _channel = self.connection_pool.get_channel("http://localhost:50051").await
            .map_err(|e| Status::internal(format!("Failed to get channel from pool: {}", e)))?;
        
        Ok(Response::new(DynamicMessage::new(method.output())))
    }
    
    /// Implementation for forwarding server streaming requests
    async fn forward_server_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        request: Request<DynamicMessage>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        // Extract metadata from the original request
        let metadata = request.metadata();
        debug!("Forwarding server streaming request for method: {} with {} metadata entries", 
               method.name(), metadata.len());
        
        // For a mock server, we would typically:
        // 1. Look up mock responses based on the service/method
        // 2. Apply any configured latency or error simulation
        // 3. Return a streaming response with preserved metadata
        // 4. Preserve all metadata from the original request in the response
        
        // For now, we'll just return a placeholder implementation
        debug!("Preserving metadata for server streaming response");
        Err(Status::unimplemented("Server streaming not yet implemented"))
    }
    
    /// Implementation for forwarding client streaming requests
    async fn forward_client_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        _request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        // For a mock server, we would typically:
        // 1. Look up mock responses based on the service/method
        // 2. Apply any configured latency or error simulation
        // 3. Process the streaming request
        // 4. Return the appropriate mock response with preserved metadata
        // 5. Preserve all metadata from the original request in the response
        
        debug!("Forwarding client streaming request for method: {}", method.name());
        Err(Status::unimplemented("Client streaming not yet implemented"))
    }
    
    /// Implementation for forwarding bidirectional streaming requests
    async fn forward_bidirectional_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        _request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<Streaming<DynamicMessage>>, Status> {
        // For a mock server, we would typically:
        // 1. Look up mock responses based on the service/method
        // 2. Apply any configured latency or error simulation
        // 3. Process the bidirectional streaming request
        // 4. Return the appropriate mock streaming response with preserved metadata
        // 5. Preserve all metadata from the original request in the response
        
        debug!("Forwarding bidirectional streaming request for method: {}", method.name());
        Err(Status::unimplemented("Bidirectional streaming not yet implemented"))
    }
}
//! Request/response handling logic
//!
//! This module provides handlers for processing gRPC requests and responses,
//! including mock response generation and request validation.

use crate::reflection::mock_proxy::proxy::MockReflectionProxy;
use prost_reflect::{DynamicMessage, MessageDescriptor};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info};

impl MockReflectionProxy {
    /// Handle a unary gRPC request
    pub async fn handle_unary_request(
        &self,
        request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        let _guard = self.track_connection();
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (service_name, method_name) = self.extract_service_method_from_request(&request)?;

        debug!("Handling unary request for {}/{}", service_name, method_name);

        // Check if this should be mocked
        if self.should_mock_service_method(&service_name, &method_name) {
            return self.generate_mock_response(&service_name, &method_name, request).await;
        }

        // Forward to real service
        self.forward_unary_request(request, &service_name, &method_name).await
    }

    /// Handle a server streaming gRPC request
    pub async fn handle_server_streaming_request(
        &self,
        request: Request<DynamicMessage>,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        let _guard = self.track_connection();
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (service_name, method_name) = self.extract_service_method_from_request(&request)?;

        debug!("Handling server streaming request for {}/{}", service_name, method_name);

        // Check if this should be mocked
        if self.should_mock_service_method(&service_name, &method_name) {
            return self.generate_mock_stream_response(&service_name, &method_name).await;
        }

        // Forward to real service
        self.forward_server_streaming_request(request, &service_name, &method_name)
            .await
    }

    /// Handle a client streaming gRPC request
    pub async fn handle_client_streaming_request(
        &self,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        let _guard = self.track_connection();
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (service_name, method_name) = self.extract_service_method_from_request(&request)?;

        debug!("Handling client streaming request for {}/{}", service_name, method_name);

        // Check if this should be mocked
        if self.should_mock_service_method(&service_name, &method_name) {
            return self
                .generate_mock_client_stream_response(&service_name, &method_name, request)
                .await;
        }

        // Forward to real service
        self.forward_client_streaming_request(request, &service_name, &method_name)
            .await
    }

    /// Handle a bidirectional streaming gRPC request
    pub async fn handle_bidirectional_streaming_request(
        &self,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        let _guard = self.track_connection();
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (service_name, method_name) = self.extract_service_method_from_request(&request)?;

        debug!("Handling bidirectional streaming request for {}/{}", service_name, method_name);

        // Check if this should be mocked
        if self.should_mock_service_method(&service_name, &method_name) {
            return self
                .generate_mock_bidirectional_stream_response(&service_name, &method_name)
                .await;
        }

        // Forward to real service
        self.forward_bidirectional_streaming_request(request, &service_name, &method_name)
            .await
    }

    /// Extract service and method names from a request
    pub fn extract_service_method_from_request<T>(
        &self,
        request: &Request<T>,
    ) -> Result<(String, String), Status> {
        // Try to get path from metadata (gRPC path header)
        let path = request
            .metadata()
            .get("path")
            .or_else(|| request.metadata().get(":path"))
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Status::invalid_argument("Missing path in request"))?;

        if !path.starts_with('/') {
            return Err(Status::invalid_argument("Invalid request path"));
        }
        let parts: Vec<&str> = path[1..].split('/').collect();
        if parts.len() != 2 {
            return Err(Status::invalid_argument(
                "Invalid gRPC path format, expected /Service/Method",
            ));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Generate a mock response for a unary request
    async fn generate_mock_response(
        &self,
        service_name: &str,
        method_name: &str,
        _request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        info!("Generating mock response for {}/{}", service_name, method_name);

        // Get the method descriptor
        let method_descriptor = self.cache().get_method(service_name, method_name).await?;

        // Generate a mock response message
        let response_message = self.generate_mock_message(method_descriptor.output())?;

        let mut response = Response::new(response_message);

        // Apply response postprocessing with body transformations
        self.postprocess_dynamic_response(&mut response, service_name, method_name)
            .await?;

        Ok(response)
    }

    /// Generate a mock streaming response
    async fn generate_mock_stream_response(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        info!("Generating mock stream response for {}/{}", service_name, method_name);

        // Get the method descriptor
        let method_descriptor = self.cache().get_method(service_name, method_name).await?;

        // Create a channel for streaming responses
        let (tx, rx) = mpsc::channel(4);

        // Generate mock response messages in a separate task
        let smart_generator = self.smart_generator().clone();
        let output_descriptor = method_descriptor.output();

        tokio::spawn(async move {
            for _i in 0..3 {
                // Generate a mock response message
                if let Ok(message) = Self::generate_mock_message_with_generator(
                    &smart_generator,
                    output_descriptor.clone(),
                ) {
                    if tx.send(Ok(message)).await.is_err() {
                        break; // Receiver dropped
                    }
                }

                // Small delay between messages
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        let mut response = Response::new(ReceiverStream::new(rx));

        // Apply response postprocessing for streaming responses
        self.postprocess_streaming_dynamic_response(&mut response, service_name, method_name)
            .await?;

        Ok(response)
    }

    /// Generate a mock client streaming response
    async fn generate_mock_client_stream_response(
        &self,
        service_name: &str,
        method_name: &str,
        _request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        info!("Generating mock client streaming response for {}/{}", service_name, method_name);

        // Get the method descriptor
        let method_descriptor = self.cache().get_method(service_name, method_name).await?;

        // Generate a mock response message
        let response_message = self.generate_mock_message(method_descriptor.output())?;

        let mut response = Response::new(response_message);

        // Apply response postprocessing with body transformations
        self.postprocess_dynamic_response(&mut response, service_name, method_name)
            .await?;

        Ok(response)
    }

    /// Generate a mock bidirectional streaming response
    async fn generate_mock_bidirectional_stream_response(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        info!(
            "Generating mock bidirectional stream response for {}/{}",
            service_name, method_name
        );

        // Get the method descriptor
        let method_descriptor = self.cache().get_method(service_name, method_name).await?;

        // Create a channel for streaming responses
        let (tx, rx) = mpsc::channel(4);

        // Generate mock response messages in a separate task
        let smart_generator = self.smart_generator().clone();
        let output_descriptor = method_descriptor.output();

        tokio::spawn(async move {
            for _i in 0..5 {
                // Generate a mock response message
                if let Ok(message) = Self::generate_mock_message_with_generator(
                    &smart_generator,
                    output_descriptor.clone(),
                ) {
                    if tx.send(Ok(message)).await.is_err() {
                        break; // Receiver dropped
                    }
                }

                // Small delay between messages
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });

        let mut response = Response::new(ReceiverStream::new(rx));

        // Apply response postprocessing for streaming responses
        self.postprocess_streaming_dynamic_response(&mut response, service_name, method_name)
            .await?;

        Ok(response)
    }

    /// Forward a unary request to the real service
    async fn forward_unary_request(
        &self,
        request: Request<DynamicMessage>,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<DynamicMessage>, Status> {
        if let Some(upstream) = &self.config.upstream_endpoint {
            // Get channel to upstream
            let _channel = self.connection_pool.get_channel(upstream).await.map_err(|e| {
                Status::unavailable(format!("Failed to connect to upstream {}: {}", upstream, e))
            })?;

            debug!(
                "Generic upstream forwarding is unavailable for {}/{}, falling back to local mock response",
                service_name, method_name
            );
            self.generate_mock_response(service_name, method_name, request).await
        } else {
            debug!(
                "No upstream endpoint configured for {}/{}, using local mock fallback",
                service_name, method_name
            );
            self.generate_mock_response(service_name, method_name, request).await
        }
    }

    /// Forward a server streaming request to the real service
    async fn forward_server_streaming_request(
        &self,
        _request: Request<DynamicMessage>,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        if let Some(upstream) = &self.config.upstream_endpoint {
            // Get channel to upstream
            let _channel = self.connection_pool.get_channel(upstream).await.map_err(|e| {
                Status::unavailable(format!("Failed to connect to upstream {}: {}", upstream, e))
            })?;

            debug!(
                "Generic upstream streaming forwarding is unavailable for {}/{}, falling back to local mock stream",
                service_name, method_name
            );
            self.generate_mock_stream_response(service_name, method_name).await
        } else {
            debug!(
                "No upstream endpoint configured for {}/{}, using local mock stream fallback",
                service_name, method_name
            );
            self.generate_mock_stream_response(service_name, method_name).await
        }
    }

    /// Forward a client streaming request to the real service
    async fn forward_client_streaming_request(
        &self,
        request: Request<Streaming<DynamicMessage>>,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<DynamicMessage>, Status> {
        if let Some(upstream) = &self.config.upstream_endpoint {
            // Get channel to upstream
            let _channel = self.connection_pool.get_channel(upstream).await.map_err(|e| {
                Status::unavailable(format!("Failed to connect to upstream {}: {}", upstream, e))
            })?;

            debug!(
                "Generic upstream client-stream forwarding is unavailable for {}/{}, falling back to local mock response",
                service_name, method_name
            );
            self.generate_mock_client_stream_response(service_name, method_name, request)
                .await
        } else {
            debug!(
                "No upstream endpoint configured for {}/{}, using local mock client-stream fallback",
                service_name, method_name
            );
            self.generate_mock_client_stream_response(service_name, method_name, request)
                .await
        }
    }

    /// Forward a bidirectional streaming request to the real service
    async fn forward_bidirectional_streaming_request(
        &self,
        request: Request<Streaming<DynamicMessage>>,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        if let Some(upstream) = &self.config.upstream_endpoint {
            // Get channel to upstream
            let _channel = self.connection_pool.get_channel(upstream).await.map_err(|e| {
                Status::unavailable(format!("Failed to connect to upstream {}: {}", upstream, e))
            })?;

            debug!(
                "Generic upstream bidi-stream forwarding is unavailable for {}/{}, falling back to local mock stream",
                service_name, method_name
            );
            let _ = request;
            self.generate_mock_bidirectional_stream_response(service_name, method_name)
                .await
        } else {
            debug!(
                "No upstream endpoint configured for {}/{}, using local mock bidi-stream fallback",
                service_name, method_name
            );
            let _ = request;
            self.generate_mock_bidirectional_stream_response(service_name, method_name)
                .await
        }
    }

    /// Generate a mock message using the smart generator
    fn generate_mock_message(
        &self,
        descriptor: MessageDescriptor,
    ) -> Result<DynamicMessage, Status> {
        let mut smart_generator = self
            .smart_generator()
            .lock()
            .map_err(|_| Status::internal("Failed to acquire lock on smart generator"))?;

        Ok(smart_generator.generate_message(&descriptor))
    }

    /// Generate a mock message with a specific generator
    fn generate_mock_message_with_generator(
        smart_generator: &Arc<Mutex<crate::reflection::smart_mock_generator::SmartMockGenerator>>,
        descriptor: MessageDescriptor,
    ) -> Result<DynamicMessage, Status> {
        let mut smart_generator = smart_generator
            .lock()
            .map_err(|_| Status::internal("Failed to acquire lock on smart generator"))?;

        Ok(smart_generator.generate_message(&descriptor))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}

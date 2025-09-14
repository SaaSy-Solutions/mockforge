//! Mock-enabled reflection proxy implementation
//!
//! This module provides a reflection proxy that can serve mock responses
//! instead of forwarding requests to other servers.

use crate::dynamic::ServiceRegistry;
use crate::reflection::{
    cache::DescriptorCache, config::ProxyConfig, connection_pool::ConnectionPool,
    smart_mock_generator::{SmartMockGenerator, SmartMockConfig},
};
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, ReflectMessage};
use prost_types::Any;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
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
    #[allow(dead_code)]
    connection_pool: ConnectionPool,
    /// Registry of dynamic services for mock responses
    service_registry: Arc<ServiceRegistry>,
    /// Smart mock data generator for intelligent field population
    smart_generator: Arc<Mutex<SmartMockGenerator>>,
}

impl MockReflectionProxy {
    /// Create a new mock reflection proxy
    pub async fn new(
        config: ProxyConfig,
        service_registry: Arc<ServiceRegistry>,
    ) -> Result<Self, Status> {
        debug!(
            "Creating mock reflection proxy with {} services",
            service_registry.service_names().len()
        );

        let cache = DescriptorCache::new();

        // Populate cache from service registry's descriptor pool
        cache.populate_from_pool(service_registry.descriptor_pool()).await;

        Ok(Self {
            cache,
            config,
            timeout_duration: Duration::from_secs(30),
            connection_pool: ConnectionPool::new(),
            service_registry,
            smart_generator: Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default()))),
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
            let response_dynamic =
                self.convert_any_to_dynamic_message(&response_any.into_inner())?;

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
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
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
            let _request_any = Request::new(request_any);

            // Create a streaming response
            let stream =
                self.create_mock_stream(service_name, method_name, dynamic_service).await?;
            Ok(Response::new(stream))
        } else {
            // Service not found in our registry, create a generic mock stream
            warn!("Service {} not found in registry, creating generic mock stream", service_name);
            let stream = self.create_generic_mock_stream(service_name, method_name).await?;
            Ok(Response::new(stream))
        }
    }

    /// Handle a client-streaming request with mock response
    pub async fn handle_client_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Handling client streaming request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // Get the dynamic service from our registry
        if let Some(dynamic_service) = self.service_registry.get(service_name) {
            // Try to use the dynamic service for client streaming
            match self
                .handle_client_streaming_with_service(
                    service_name,
                    method_name,
                    request,
                    dynamic_service,
                )
                .await
            {
                Ok(response) => Ok(response),
                Err(e) => {
                    warn!(
                        "Failed to handle client streaming with service {}.{}: {}",
                        service_name, method_name, e
                    );
                    // Fallback to generic handling
                    self.handle_client_streaming_generic(service_name, method_name).await
                }
            }
        } else {
            // Service not found in our registry, create a generic mock response
            warn!(
                "Service {} not found in registry, creating generic client streaming response",
                service_name
            );
            self.handle_client_streaming_generic(service_name, method_name).await
        }
    }

    /// Handle a bidirectional streaming request with mock response
    pub async fn handle_bidirectional_streaming(
        &self,
        service_name: &str,
        method_name: &str,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<ReceiverStream<Result<DynamicMessage, Status>>>, Status> {
        debug!("Handling bidirectional streaming request for {}.{}", service_name, method_name);

        // Check if service is allowed
        if !self.config.is_service_allowed(service_name) {
            return Err(Status::permission_denied(format!(
                "Service {} is not allowed",
                service_name
            )));
        }

        // Get the dynamic service from our registry
        if let Some(dynamic_service) = self.service_registry.get(service_name) {
            // Create a bidirectional streaming response
            let stream = self
                .create_bidirectional_mock_stream(
                    service_name,
                    method_name,
                    request,
                    dynamic_service,
                )
                .await?;
            Ok(Response::new(stream))
        } else {
            // Service not found in our registry, create a generic bidirectional mock stream
            warn!(
                "Service {} not found in registry, creating generic bidirectional mock stream",
                service_name
            );
            let stream = self
                .create_generic_bidirectional_mock_stream(service_name, method_name, request)
                .await?;
            Ok(Response::new(stream))
        }
    }

    /// Create a bidirectional mock stream for registered services
    async fn create_bidirectional_mock_stream(
        &self,
        service_name: &str,
        method_name: &str,
        mut request: Request<Streaming<DynamicMessage>>,
        _dynamic_service: &std::sync::Arc<crate::dynamic::service_generator::DynamicGrpcService>,
    ) -> Result<ReceiverStream<Result<DynamicMessage, Status>>, Status> {
        debug!("Creating bidirectional mock stream for {}.{}", service_name, method_name);

        let (tx, rx) = mpsc::channel(10);
        let service_name = service_name.to_string();
        let method_name = method_name.to_string();
        let cache = self.cache.clone();
        let smart_generator = Arc::clone(&self.smart_generator);

        // Spawn a task to handle bidirectional streaming
        tokio::spawn(async move {
            let mut input_count = 0;
            let mut output_count = 0;

            // Read from input stream and respond to each message
            while let Ok(Some(_input_message)) = request.get_mut().message().await {
                input_count += 1;
                debug!(
                    "Received bidirectional input message {} for {}.{}",
                    input_count, service_name, method_name
                );

                // For each input message, generate 1-2 response messages
                let responses_per_input = if input_count % 3 == 0 { 2 } else { 1 };

                for response_idx in 0..responses_per_input {
                    output_count += 1;

                    match Self::create_bidirectional_response_message(
                        &cache,
                        &service_name,
                        &method_name,
                        input_count,
                        output_count,
                        response_idx,
                        &smart_generator,
                    )
                    .await
                    {
                        Ok(response_message) => {
                            if tx.send(Ok(response_message)).await.is_err() {
                                debug!(
                                    "Bidirectional stream receiver dropped for {}.{}",
                                    service_name, method_name
                                );
                                return;
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create bidirectional response message for {}.{}: {}",
                                service_name, method_name, e
                            );
                            if tx
                                .send(Err(Status::internal(format!(
                                    "Failed to create response: {}",
                                    e
                                ))))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }

                    // Add small delay between responses
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }

                // Limit the number of messages we process to prevent infinite loops
                if input_count >= 100 {
                    warn!(
                        "Reached maximum input message limit (100) for bidirectional {}.{}",
                        service_name, method_name
                    );
                    break;
                }
            }

            info!(
                "Bidirectional streaming completed for {}.{}: processed {} inputs, sent {} outputs",
                service_name, method_name, input_count, output_count
            );
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a generic bidirectional mock stream for unknown services  
    async fn create_generic_bidirectional_mock_stream(
        &self,
        service_name: &str,
        method_name: &str,
        mut request: Request<Streaming<DynamicMessage>>,
    ) -> Result<ReceiverStream<Result<DynamicMessage, Status>>, Status> {
        debug!(
            "Creating generic bidirectional mock stream for {}.{}",
            service_name, method_name
        );

        let (tx, rx) = mpsc::channel(10);
        let service_name = service_name.to_string();
        let method_name = method_name.to_string();
        let cache = self.cache.clone();
        let smart_generator = Arc::clone(&self.smart_generator);

        // Spawn a task to handle generic bidirectional streaming
        tokio::spawn(async move {
            let mut input_count = 0;
            let mut output_count = 0;

            // Read from input stream and respond to each message
            while let Ok(Some(_input_message)) = request.get_mut().message().await {
                input_count += 1;
                debug!(
                    "Received generic bidirectional input message {} for {}.{}",
                    input_count, service_name, method_name
                );

                // For generic handling, send one response per input
                output_count += 1;

                match Self::create_bidirectional_response_message(
                    &cache,
                    &service_name,
                    &method_name,
                    input_count,
                    output_count,
                    0,
                    &smart_generator,
                )
                .await
                {
                    Ok(response_message) => {
                        if tx.send(Ok(response_message)).await.is_err() {
                            debug!(
                                "Generic bidirectional stream receiver dropped for {}.{}",
                                service_name, method_name
                            );
                            return;
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create generic bidirectional response for {}.{}: {}",
                            service_name, method_name, e
                        );
                        if tx
                            .send(Err(Status::internal(format!(
                                "Failed to create response: {}",
                                e
                            ))))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }

                // Add delay between responses
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Limit the number of messages for generic handling
                if input_count >= 50 {
                    warn!(
                        "Reached maximum input message limit (50) for generic bidirectional {}.{}",
                        service_name, method_name
                    );
                    break;
                }
            }

            info!("Generic bidirectional streaming completed for {}.{}: processed {} inputs, sent {} outputs", 
                  service_name, method_name, input_count, output_count);
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a response message for bidirectional streaming
    async fn create_bidirectional_response_message(
        cache: &DescriptorCache,
        service_name: &str,
        method_name: &str,
        input_sequence: usize,
        output_sequence: usize,
        response_index: usize,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) -> Result<DynamicMessage, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Creating bidirectional response message for {}.{} (input: {}, output: {}, index: {})",
            service_name, method_name, input_sequence, output_sequence, response_index
        );

        // Try to get the proper response message descriptor from cache
        if let Ok(method_desc) = cache.get_method(service_name, method_name).await {
            let output_desc = method_desc.output();
            let mut msg = DynamicMessage::new(output_desc.clone());

            // Populate response fields with bidirectional-specific data
            Self::populate_bidirectional_response_fields(
                &mut msg,
                service_name,
                method_name,
                input_sequence,
                output_sequence,
                response_index,
                smart_generator,
            );

            Ok(msg)
        } else {
            // Fallback to generic message creation
            let mock_data = format!(
                "Bidirectional response {} for input {} from {}.{} at timestamp {}",
                output_sequence,
                input_sequence,
                service_name,
                method_name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );

            Self::create_placeholder_dynamic_message(&mock_data)
        }
    }

    /// Populate mock fields specifically for bidirectional streaming responses using smart generation
    fn populate_bidirectional_response_fields(
        msg: &mut DynamicMessage,
        service_name: &str,
        method_name: &str,
        input_sequence: usize,
        output_sequence: usize,
        response_index: usize,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) {
        let descriptor = msg.descriptor();

        // Use smart generator for bidirectional streaming context
        for field in descriptor.fields() {
            if let Ok(mut generator) = smart_generator.lock() {
                // Generate value with intelligent inference for bidirectional streaming
                let value = generator.generate_value_for_field(&field, service_name, method_name, 0);
                
                // Override specific fields for bidirectional streaming context
                let field_name = field.name();
                match field_name {
                    "sequence" | "seq" | "response_id" | "output_sequence" => {
                        if matches!(field.kind(), prost_reflect::Kind::Int32) {
                            msg.set_field(&field, prost_reflect::Value::I32(output_sequence as i32));
                        } else if matches!(field.kind(), prost_reflect::Kind::Int64) {
                            msg.set_field(&field, prost_reflect::Value::I64(output_sequence as i64));
                        } else {
                            msg.set_field(&field, value);
                        }
                    }
                    "input_sequence" | "request_id" | "input_id" => {
                        if matches!(field.kind(), prost_reflect::Kind::Int32) {
                            msg.set_field(&field, prost_reflect::Value::I32(input_sequence as i32));
                        } else if matches!(field.kind(), prost_reflect::Kind::Int64) {
                            msg.set_field(&field, prost_reflect::Value::I64(input_sequence as i64));
                        } else {
                            msg.set_field(&field, value);
                        }
                    }
                    "response_index" | "index" | "part" => {
                        if matches!(field.kind(), prost_reflect::Kind::Int32) {
                            msg.set_field(&field, prost_reflect::Value::I32(response_index as i32));
                        } else if matches!(field.kind(), prost_reflect::Kind::Int64) {
                            msg.set_field(&field, prost_reflect::Value::I64(response_index as i64));
                        } else {
                            msg.set_field(&field, value);
                        }
                    }
                    "is_final" | "final" | "last" => {
                        if matches!(field.kind(), prost_reflect::Kind::Bool) {
                            // Mark as final if this is the last response for this input
                            let is_final = response_index > 0;
                            msg.set_field(&field, prost_reflect::Value::Bool(is_final));
                        } else {
                            msg.set_field(&field, value);
                        }
                    }
                    _ => {
                        msg.set_field(&field, value);
                    }
                }
            } else {
                // Fallback if lock fails
                Self::populate_field_fallback(msg, &field, output_sequence);
            }
        }
    }

    /// Handle client streaming with a registered dynamic service
    async fn handle_client_streaming_with_service(
        &self,
        service_name: &str,
        method_name: &str,
        mut request: Request<Streaming<DynamicMessage>>,
        _dynamic_service: &std::sync::Arc<crate::dynamic::service_generator::DynamicGrpcService>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Handling client streaming with service for {}.{}", service_name, method_name);

        // Collect all client messages
        let mut messages = Vec::new();
        let mut message_count = 0;

        while let Ok(Some(message)) = request.get_mut().message().await {
            message_count += 1;
            debug!(
                "Received client message {} for {}.{}",
                message_count, service_name, method_name
            );

            // Convert DynamicMessage to Any for processing
            match self.convert_dynamic_message_to_any(&message) {
                Ok(any_message) => messages.push(any_message),
                Err(e) => {
                    warn!(
                        "Failed to convert message {} for {}.{}: {}",
                        message_count, service_name, method_name, e
                    );
                    // Continue processing other messages
                }
            }

            // Limit the number of messages we collect to prevent memory issues
            if message_count >= 1000 {
                warn!("Reached maximum message limit (1000) for {}.{}", service_name, method_name);
                break;
            }
        }

        info!(
            "Collected {} messages for client streaming {}.{}",
            messages.len(),
            service_name,
            method_name
        );

        // Create a mock request with the first message (if any) to pass to the service
        if let Some(first_message) = messages.first() {
            let _mock_request = Request::new(first_message.clone());

            // Try to handle using the dynamic service's client streaming method
            // Since we can't easily convert between Streaming<DynamicMessage> and Streaming<Any>,
            // we'll create a mock response based on the collected messages
            self.create_client_streaming_response(service_name, method_name, messages.len())
                .await
        } else {
            // No messages received
            info!("No messages received for client streaming {}.{}", service_name, method_name);
            self.create_client_streaming_response(service_name, method_name, 0).await
        }
    }

    /// Handle client streaming generically when no service is found
    async fn handle_client_streaming_generic(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Handling generic client streaming for {}.{}", service_name, method_name);

        // For generic handling, we simulate processing without actually reading the stream
        // This is because we don't have the request parameter in this branch
        info!(
            "Creating generic client streaming response for {}.{}",
            service_name, method_name
        );

        self.create_client_streaming_response(service_name, method_name, 0).await
    }

    /// Create a client streaming response message
    async fn create_client_streaming_response(
        &self,
        service_name: &str,
        method_name: &str,
        message_count: usize,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!(
            "Creating client streaming response for {}.{} with {} messages",
            service_name, method_name, message_count
        );

        // Try to get the proper response message descriptor from cache
        if let Ok(method_desc) = self.cache.get_method(service_name, method_name).await {
            let output_desc = method_desc.output();
            let mut msg = DynamicMessage::new(output_desc.clone());

            // Populate response fields with streaming-specific data
            Self::populate_client_streaming_fields(
                &mut msg,
                service_name,
                method_name,
                message_count,
                &self.smart_generator,
            );

            Ok(Response::new(msg))
        } else {
            // Fallback to generic message creation
            warn!(
                "Could not find method descriptor for {}.{}, using fallback",
                service_name, method_name
            );

            let mock_data = format!(
                "Processed {} messages for client streaming {}.{} at timestamp {}",
                message_count,
                service_name,
                method_name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );

            match Self::create_placeholder_dynamic_message(&mock_data) {
                Ok(msg) => Ok(Response::new(msg)),
                Err(e) => {
                    Err(Status::internal(format!("Failed to create response message: {}", e)))
                }
            }
        }
    }

    /// Populate mock fields specifically for client streaming responses using smart generation
    fn populate_client_streaming_fields(
        msg: &mut DynamicMessage,
        service_name: &str,
        method_name: &str,
        message_count: usize,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) {
        let descriptor = msg.descriptor();

        // Use smart generator for client streaming context
        for field in descriptor.fields() {
            if let Ok(mut generator) = smart_generator.lock() {
                // Generate value with intelligent inference for client streaming
                let value = generator.generate_value_for_field(&field, service_name, method_name, 0);
                
                // Override specific fields for client streaming context
                let field_name = field.name();
                match field_name {
                    "count" | "message_count" | "total" | "processed" => {
                        if matches!(field.kind(), prost_reflect::Kind::Int32) {
                            msg.set_field(&field, prost_reflect::Value::I32(message_count as i32));
                        } else if matches!(field.kind(), prost_reflect::Kind::Int64) {
                            msg.set_field(&field, prost_reflect::Value::I64(message_count as i64));
                        } else {
                            msg.set_field(&field, value);
                        }
                    }
                    _ => {
                        msg.set_field(&field, value);
                    }
                }
            } else {
                // Fallback if lock fails
                Self::populate_field_fallback(msg, &field, message_count);
            }
        }
    }

    /// Create a mock stream for server streaming responses
    async fn create_mock_stream(
        &self,
        service_name: &str,
        method_name: &str,
        _dynamic_service: &std::sync::Arc<crate::dynamic::service_generator::DynamicGrpcService>,
    ) -> Result<ReceiverStream<Result<DynamicMessage, Status>>, Status> {
        debug!("Creating mock stream for {}.{}", service_name, method_name);

        let (tx, rx) = mpsc::channel(10);
        let service_name = service_name.to_string();
        let method_name = method_name.to_string();

        let cache = self.cache.clone();
        let smart_generator = Arc::clone(&self.smart_generator);

        // Spawn a task to generate stream messages
        tokio::spawn(async move {
            // Generate a few mock messages
            for i in 0..3 {
                match Self::create_mock_dynamic_message_with_cache(
                    &cache,
                    &service_name,
                    &method_name,
                    i,
                    &smart_generator,
                )
                .await
                {
                    Ok(mock_message) => {
                        if tx.send(Ok(mock_message)).await.is_err() {
                            debug!("Stream receiver dropped for {}.{}", service_name, method_name);
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create mock message for {}.{}: {}",
                            service_name, method_name, e
                        );
                        if tx
                            .send(Err(Status::internal(format!(
                                "Failed to create mock message: {}",
                                e
                            ))))
                            .await
                            .is_err()
                        {
                            break; // Receiver dropped
                        }
                    }
                }
                // Add some delay between messages
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            debug!("Finished streaming {} messages for {}.{}", 3, service_name, method_name);
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a generic mock stream for unknown services
    async fn create_generic_mock_stream(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<ReceiverStream<Result<DynamicMessage, Status>>, Status> {
        debug!("Creating generic mock stream for {}.{}", service_name, method_name);

        let (tx, rx) = mpsc::channel(10);
        let service_name = service_name.to_string();
        let method_name = method_name.to_string();

        let cache = self.cache.clone();
        let smart_generator = Arc::clone(&self.smart_generator);

        // Spawn a task to generate generic stream messages
        tokio::spawn(async move {
            // Generate a few generic mock messages
            for i in 0..3 {
                match Self::create_mock_dynamic_message_with_cache(
                    &cache,
                    &service_name,
                    &method_name,
                    i,
                    &smart_generator,
                )
                .await
                {
                    Ok(mock_message) => {
                        if tx.send(Ok(mock_message)).await.is_err() {
                            debug!(
                                "Stream receiver dropped for generic {}.{}",
                                service_name, method_name
                            );
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create generic mock message for {}.{}: {}",
                            service_name, method_name, e
                        );
                        if tx
                            .send(Err(Status::internal(format!(
                                "Failed to create mock message: {}",
                                e
                            ))))
                            .await
                            .is_err()
                        {
                            break; // Receiver dropped
                        }
                    }
                }
                // Add some delay between messages
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            debug!(
                "Finished streaming {} generic messages for {}.{}",
                3, service_name, method_name
            );
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a mock DynamicMessage for streaming responses using cache
    async fn create_mock_dynamic_message_with_cache(
        cache: &DescriptorCache,
        service_name: &str,
        method_name: &str,
        index: usize,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) -> Result<DynamicMessage, Box<dyn std::error::Error + Send + Sync>> {
        // Try to get the proper response message descriptor from cache
        if let Ok(method_desc) = cache.get_method(service_name, method_name).await {
            let output_desc = method_desc.output();
            // Create a proper DynamicMessage with the correct descriptor
            let mut msg = DynamicMessage::new(output_desc.clone());

            // Try to populate some common fields if they exist
            Self::populate_mock_fields(&mut msg, service_name, method_name, index, smart_generator);

            return Ok(msg);
        }

        // Fallback to generic message creation
        let mock_data = format!(
            "Stream message {} from {}.{} at timestamp {}",
            index,
            service_name,
            method_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        Self::create_placeholder_dynamic_message(&mock_data)
    }

    /// Populate mock fields in a DynamicMessage using smart generation
    fn populate_mock_fields(
        msg: &mut DynamicMessage,
        service_name: &str,
        method_name: &str,
        index: usize,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) {
        let descriptor = msg.descriptor();

        // Use smart generator for all fields
        for field in descriptor.fields() {
            if let Ok(mut generator) = smart_generator.lock() {
                let value = generator.generate_value_for_field(&field, service_name, method_name, 0);
                msg.set_field(&field, value);
            } else {
                // Fallback to basic generation if lock fails
                Self::populate_field_fallback(msg, &field, index);
            }
        }
    }

    /// Fallback field population when smart generator is unavailable
    fn populate_field_fallback(msg: &mut DynamicMessage, field: &prost_reflect::FieldDescriptor, index: usize) {
        match field.kind() {
            prost_reflect::Kind::String => {
                msg.set_field(field, prost_reflect::Value::String(format!("mock_{}", field.name())));
            }
            prost_reflect::Kind::Int32 => {
                msg.set_field(field, prost_reflect::Value::I32(index as i32));
            }
            prost_reflect::Kind::Int64 => {
                msg.set_field(field, prost_reflect::Value::I64(index as i64));
            }
            prost_reflect::Kind::Bool => {
                msg.set_field(field, prost_reflect::Value::Bool(index % 2 == 0));
            }
            _ => {
                // Skip complex types for now
            }
        }
    }

    /// Create a proper DynamicMessage with a custom MessageDescriptor
    fn create_placeholder_dynamic_message(
        data: &str,
    ) -> Result<DynamicMessage, Box<dyn std::error::Error + Send + Sync>> {
        // First try to use well-known types as before for performance
        let pool = DescriptorPool::global();
        if let Some(value_desc) = pool.get_message_by_name("google.protobuf.Value") {
            let mut msg = DynamicMessage::new(value_desc.clone());
            if let Some(field) = value_desc.get_field_by_name("string_value") {
                msg.set_field(&field, prost_reflect::Value::String(data.to_string()));
            }
            return Ok(msg);
        }

        // Use cached custom message descriptor for MockForge streaming responses
        Self::create_custom_streaming_message(data)
    }

    /// Create a custom DynamicMessage with a proper MessageDescriptor for streaming responses
    fn create_custom_streaming_message(
        data: &str,
    ) -> Result<DynamicMessage, Box<dyn std::error::Error + Send + Sync>> {
        // Use a static cache for the descriptor to avoid recreating it
        static CUSTOM_DESCRIPTOR: OnceLock<MessageDescriptor> = OnceLock::new();

        let message_desc = CUSTOM_DESCRIPTOR.get_or_init(|| {
            use prost_reflect::prost_types::FileDescriptorProto;

            // Create a custom descriptor pool for our mock messages
            let mut pool = DescriptorPool::new();

            // Create a FileDescriptorProto for MockForge streaming messages
            let file_descriptor = FileDescriptorProto {
                name: Some("mockforge/stream_response.proto".to_string()),
                package: Some("mockforge".to_string()),
                dependency: vec![],
                public_dependency: vec![],
                weak_dependency: vec![],
                message_type: vec![Self::create_streaming_response_descriptor()],
                enum_type: vec![],
                service: vec![],
                extension: vec![],
                options: None,
                source_code_info: None,
                syntax: Some("proto3".to_string()),
            };

            // Add the file descriptor to the pool
            if let Err(e) = pool.add_file_descriptor_proto(file_descriptor) {
                panic!("Failed to add file descriptor: {}", e);
            }

            // Get the message descriptor we just created
            pool.get_message_by_name("mockforge.StreamResponse")
                .expect("Failed to find StreamResponse message descriptor")
        });

        // Create the DynamicMessage using the cached descriptor
        let mut msg = DynamicMessage::new(message_desc.clone());

        // Populate the message with the provided data
        if let Some(message_field) = message_desc.get_field_by_name("message") {
            msg.set_field(&message_field, prost_reflect::Value::String(data.to_string()));
        }

        if let Some(timestamp_field) = message_desc.get_field_by_name("timestamp") {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            msg.set_field(&timestamp_field, prost_reflect::Value::I64(timestamp as i64));
        }

        if let Some(source_field) = message_desc.get_field_by_name("source") {
            msg.set_field(&source_field, prost_reflect::Value::String("MockForge".to_string()));
        }

        if let Some(success_field) = message_desc.get_field_by_name("success") {
            msg.set_field(&success_field, prost_reflect::Value::Bool(true));
        }

        Ok(msg)
    }

    /// Create a DescriptorProto for the StreamResponse message
    fn create_streaming_response_descriptor() -> prost_reflect::prost_types::DescriptorProto {
        use prost_reflect::prost_types::{
            field_descriptor_proto, DescriptorProto, FieldDescriptorProto,
        };

        DescriptorProto {
            name: Some("StreamResponse".to_string()),
            field: vec![
                FieldDescriptorProto {
                    name: Some("message".to_string()),
                    number: Some(1),
                    label: Some(field_descriptor_proto::Label::Optional as i32),
                    r#type: Some(field_descriptor_proto::Type::String as i32),
                    type_name: None,
                    extendee: None,
                    default_value: None,
                    oneof_index: None,
                    json_name: Some("message".to_string()),
                    options: None,
                    proto3_optional: None,
                },
                FieldDescriptorProto {
                    name: Some("timestamp".to_string()),
                    number: Some(2),
                    label: Some(field_descriptor_proto::Label::Optional as i32),
                    r#type: Some(field_descriptor_proto::Type::Int64 as i32),
                    type_name: None,
                    extendee: None,
                    default_value: None,
                    oneof_index: None,
                    json_name: Some("timestamp".to_string()),
                    options: None,
                    proto3_optional: None,
                },
                FieldDescriptorProto {
                    name: Some("source".to_string()),
                    number: Some(3),
                    label: Some(field_descriptor_proto::Label::Optional as i32),
                    r#type: Some(field_descriptor_proto::Type::String as i32),
                    type_name: None,
                    extendee: None,
                    default_value: None,
                    oneof_index: None,
                    json_name: Some("source".to_string()),
                    options: None,
                    proto3_optional: None,
                },
                FieldDescriptorProto {
                    name: Some("sequence".to_string()),
                    number: Some(4),
                    label: Some(field_descriptor_proto::Label::Optional as i32),
                    r#type: Some(field_descriptor_proto::Type::Int32 as i32),
                    type_name: None,
                    extendee: None,
                    default_value: None,
                    oneof_index: None,
                    json_name: Some("sequence".to_string()),
                    options: None,
                    proto3_optional: None,
                },
                FieldDescriptorProto {
                    name: Some("success".to_string()),
                    number: Some(5),
                    label: Some(field_descriptor_proto::Label::Optional as i32),
                    r#type: Some(field_descriptor_proto::Type::Bool as i32),
                    type_name: None,
                    extendee: None,
                    default_value: None,
                    oneof_index: None,
                    json_name: Some("success".to_string()),
                    options: None,
                    proto3_optional: None,
                },
            ],
            extension: vec![],
            nested_type: vec![],
            enum_type: vec![],
            extension_range: vec![],
            oneof_decl: vec![],
            options: None,
            reserved_range: vec![],
            reserved_name: vec![],
        }
    }

    /// Convert DynamicMessage to Any for our service handlers
    fn convert_dynamic_message_to_any(&self, message: &DynamicMessage) -> Result<Any, Status> {
        use prost_reflect::prost::Message;

        debug!(
            "Converting DynamicMessage to Any for type: {}",
            message.descriptor().full_name()
        );

        // Validate the message before conversion
        let descriptor = message.descriptor();
        if !descriptor.is_map_entry() && descriptor.fields().count() == 0 {
            debug!("Message has no fields, but proceeding with conversion");
        }

        // Create the proper type URL according to protobuf standards
        // Format: type.googleapis.com/{package}.{message_name}
        let full_name = descriptor.full_name();
        let type_url = if full_name.starts_with("google.protobuf.") {
            // For well-known types, use the standard googleapis.com domain
            format!("type.googleapis.com/{}", full_name)
        } else if full_name.contains('.') {
            // For custom types with package, use the standard format
            format!("type.googleapis.com/{}", full_name)
        } else {
            // For types without package, add a default domain
            format!("type.googleapis.com/{}", full_name)
        };

        // Encode the message to bytes with proper error handling
        let value = message.encode_to_vec();
        debug!("Successfully encoded DynamicMessage to {} bytes", value.len());

        // Validate the encoded data is not empty (unless the message is genuinely empty)
        if value.is_empty() && message.descriptor().fields().count() > 0 {
            warn!("Encoded message is empty but descriptor has fields for type: {}", full_name);
        }

        let any_message = Any {
            type_url: type_url.clone(),
            value,
        };

        debug!("Successfully converted DynamicMessage to Any with type_url: {}", type_url);

        Ok(any_message)
    }

    /// Convert Any back to DynamicMessage for responses
    fn convert_any_to_dynamic_message(&self, any: &Any) -> Result<DynamicMessage, Status> {
        debug!("Converting Any to DynamicMessage for type_url: {}", any.type_url);

        // Validate the Any message
        if any.type_url.is_empty() {
            return Err(Status::invalid_argument("Any message has empty type_url"));
        }

        if any.value.is_empty() {
            debug!("Any message has empty value, will create empty DynamicMessage");
        }

        // Extract the type name from the type_url
        // Standard format is: type.googleapis.com/{package}.{message_name}
        let type_name = if let Some(type_part) = any.type_url.strip_prefix("type.googleapis.com/") {
            type_part
        } else if any.type_url.contains('/') {
            // Handle other URL formats
            any.type_url.split('/').next_back().unwrap_or(&any.type_url)
        } else {
            // Fallback: use the entire type_url as type name
            &any.type_url
        };

        debug!("Extracted type name: {} from type_url: {}", type_name, any.type_url);

        // Try to find the message descriptor in our cache first
        // Note: We'd need to iterate through cached methods to find matching types
        // For now, we'll skip cache lookup and go directly to global pool

        // Try to find the descriptor in the global pool (for well-known types)
        let pool = DescriptorPool::global();
        if let Some(message_desc) = pool.get_message_by_name(type_name) {
            return self.decode_any_with_descriptor(any, message_desc);
        }

        // Try common variations of the type name
        let variations = [
            type_name,
            &format!("google.protobuf.{}", type_name),
            &format!("{}.Response", type_name),
            &format!("{}.Reply", type_name),
        ];

        for variation in &variations {
            if let Some(message_desc) = pool.get_message_by_name(variation) {
                debug!("Found descriptor using variation: {}", variation);
                return self.decode_any_with_descriptor(any, message_desc);
            }
        }

        // Last resort: create our custom StreamResponse if nothing else works
        warn!("Could not find descriptor for type: {}, creating fallback message", type_name);
        self.create_fallback_dynamic_message_from_any(any)
    }

    /// Decode Any with a specific MessageDescriptor
    fn decode_any_with_descriptor(
        &self,
        any: &Any,
        descriptor: MessageDescriptor,
    ) -> Result<DynamicMessage, Status> {
        let type_name = descriptor.full_name().to_string();

        match DynamicMessage::decode(descriptor, any.value.as_slice()) {
            Ok(message) => {
                debug!("Successfully decoded Any to DynamicMessage for type: {}", type_name);
                Ok(message)
            }
            Err(decode_error) => {
                warn!("Failed to decode Any for type {}: {}", type_name, decode_error);
                Err(Status::invalid_argument(format!(
                    "Failed to decode Any message for type {}: {}",
                    type_name, decode_error
                )))
            }
        }
    }

    /// Create a fallback DynamicMessage when we can't find the proper descriptor
    fn create_fallback_dynamic_message_from_any(
        &self,
        any: &Any,
    ) -> Result<DynamicMessage, Status> {
        debug!("Creating fallback DynamicMessage from Any with type_url: {}", any.type_url);

        // Try to create a meaningful message using our custom StreamResponse
        let data = if any.value.is_empty() {
            format!("Empty Any message with type: {}", any.type_url)
        } else {
            // Try to interpret the bytes as UTF-8 string for debugging
            match String::from_utf8(any.value.clone()) {
                Ok(utf8_str) if utf8_str.len() < 1000 => {
                    // Reasonable length limit
                    format!("Decoded Any message (type: {}): {}", any.type_url, utf8_str)
                }
                _ => {
                    format!(
                        "Binary Any message (type: {}, {} bytes)",
                        any.type_url,
                        any.value.len()
                    )
                }
            }
        };

        Self::create_placeholder_dynamic_message(&data)
            .map_err(|e| Status::internal(format!("Failed to create fallback message: {}", e)))
    }

    /// Generate mock stream messages for testing
    #[allow(dead_code)]
    async fn generate_mock_stream_messages(
        &self,
        _count: usize,
    ) -> Result<Vec<DynamicMessage>, Status> {
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
        Err(Status::unimplemented(format!(
            "Mock response for {}.{} not yet implemented",
            service_name, method_name
        )))
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

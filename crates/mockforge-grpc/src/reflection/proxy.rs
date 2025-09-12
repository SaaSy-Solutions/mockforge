//! Main reflection proxy implementation

use crate::reflection::{
    cache::DescriptorCache,
    client::ReflectionClient,
    config::ProxyConfig,
    connection_pool::ConnectionPool,
};
#[cfg(feature = "data-faker")]
use mockforge_data::{DataConfig, DataGenerator, SchemaDefinition};
use prost_reflect::DynamicMessage;
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::{
    transport::Endpoint,
    Request, Response, Status, Streaming,
};
use tracing::{debug, warn};
use futures::stream;

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
    ) -> Result<Response<DynamicMessage>, Status> {
        // Extract metadata from the original request
        let metadata = request.metadata();
        debug!("Forwarding server streaming request for method: {} with {} metadata entries",
               method.name(), metadata.len());

        #[cfg(feature = "data-faker")]
        {
            // Generate mock streaming responses
            let output_descriptor = method.output();
            let messages = self.generate_mock_stream_messages(&output_descriptor, 5).await?;

            let _stream = stream::iter(messages.into_iter().map(Ok::<_, Status>));

            // Create a simple response for now - streaming implementation needs tonic API update
            let response = Response::new(DynamicMessage::new(output_descriptor.clone()));

            debug!("Generated server streaming response with preserved metadata");
            Ok(response)
        }

        #[cfg(not(feature = "data-faker"))]
        {
            debug!("Data faker feature not enabled, returning unimplemented for server streaming");
            Err(Status::unimplemented("Server streaming requires data-faker feature"))
        }
    }

    /// Implementation for forwarding client streaming requests
    async fn forward_client_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Forwarding client streaming request for method: {}", method.name());

        #[cfg(feature = "data-faker")]
        {
            // Consume the streaming request
            let mut stream = request.into_inner();
            let mut message_count = 0;

            while let Some(message_result) = stream.next().await {
                match message_result {
                    Ok(_message) => {
                        message_count += 1;
                        debug!("Received client streaming message {} for method: {}", message_count, method.name());
                        // In a real implementation, you might want to process each message
                        // For now, we just count them
                    }
                    Err(e) => {
                        warn!("Error receiving client streaming message: {}", e);
                        return Err(Status::internal(format!("Error processing streaming request: {}", e)));
                    }
                }
            }

            debug!("Processed {} messages in client streaming request", message_count);

            // Generate a mock response based on the output descriptor
            let output_descriptor = method.output();
            let mock_response = self.generate_mock_message(&output_descriptor).await?;

            let response = Response::new(mock_response);

            // Metadata preservation simplified for now

            debug!("Generated client streaming response with {} processed messages", message_count);
            Ok(response)
        }

        #[cfg(not(feature = "data-faker"))]
        {
            debug!("Data faker feature not enabled, returning unimplemented for client streaming");
            Err(Status::unimplemented("Client streaming requires data-faker feature"))
        }
    }

    /// Implementation for forwarding bidirectional streaming requests
    async fn forward_bidirectional_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        request: Request<Streaming<DynamicMessage>>,
    ) -> Result<Response<DynamicMessage>, Status> {
        debug!("Forwarding bidirectional streaming request for method: {}", method.name());

        #[cfg(feature = "data-faker")]
        {
            // Generate mock bidirectional streaming responses
            let output_descriptor = method.output();
            let messages = self.generate_mock_stream_messages(&output_descriptor, 10).await?;

            let _stream = stream::iter(messages.into_iter().map(Ok::<_, Status>));

            // Create a simple response for now - streaming implementation needs tonic API update
            let response = Response::new(DynamicMessage::new(output_descriptor.clone()));

            // Metadata preservation simplified for now

            // Note: In a real bidirectional implementation, we would also need to
            // process the incoming stream concurrently. For now, we focus on
            // generating the outgoing stream.
            let mut incoming_stream = request.into_inner();
            tokio::spawn(async move {
                let mut count = 0;
                while let Some(message_result) = incoming_stream.next().await {
                    match message_result {
                        Ok(_) => {
                            count += 1;
                            debug!("Processed bidirectional message {} for method: {}", count, method.name());
                        }
                        Err(e) => {
                            warn!("Error processing bidirectional message: {}", e);
                            break;
                        }
                    }
                }
                debug!("Finished processing {} bidirectional messages", count);
            });

            debug!("Generated bidirectional streaming response");
            Ok(response)
        }

        #[cfg(not(feature = "data-faker"))]
        {
            debug!("Data faker feature not enabled, returning unimplemented for bidirectional streaming");
            Err(Status::unimplemented("Bidirectional streaming requires data-faker feature"))
        }
    }

    /// Generate a single mock message for the given descriptor
    #[cfg(feature = "data-faker")]
    async fn generate_mock_message(
        &self,
        descriptor: &prost_reflect::MessageDescriptor,
    ) -> Result<DynamicMessage, Status> {
        // Create a basic schema from the descriptor for mock generation
        let schema_def = self.create_schema_from_protobuf_descriptor(descriptor);

        let config = DataConfig {
            rows: 1,
            ..Default::default()
        };

        let mut generator = DataGenerator::new(schema_def, config)
            .map_err(|e| Status::internal(format!("Failed to create data generator: {}", e)))?;

        let result = generator.generate().await
            .map_err(|e| Status::internal(format!("Failed to generate mock data: {}", e)))?;

        if let Some(data) = result.data.first() {
            // Convert the generated JSON to a DynamicMessage
            self.json_to_dynamic_message(descriptor, data)
        } else {
            Err(Status::internal("No mock data generated"))
        }
    }

    /// Generate multiple mock messages for streaming
    #[cfg(feature = "data-faker")]
    async fn generate_mock_stream_messages(
        &self,
        descriptor: &prost_reflect::MessageDescriptor,
        count: usize,
    ) -> Result<Vec<DynamicMessage>, Status> {
        let schema_def = self.create_schema_from_protobuf_descriptor(descriptor);

        let config = DataConfig {
            rows: count,
            ..Default::default()
        };

        let mut generator = DataGenerator::new(schema_def, config)
            .map_err(|e| Status::internal(format!("Failed to create data generator: {}", e)))?;

        let result = generator.generate().await
            .map_err(|e| Status::internal(format!("Failed to generate mock data: {}", e)))?;

        result.data.iter()
            .map(|data| self.json_to_dynamic_message(descriptor, data))
            .collect()
    }

    /// Convert JSON data to a DynamicMessage
    #[cfg(feature = "data-faker")]
    fn json_to_dynamic_message(
        &self,
        descriptor: &prost_reflect::MessageDescriptor,
        json_data: &serde_json::Value,
    ) -> Result<DynamicMessage, Status> {
        let mut message = DynamicMessage::new(descriptor.clone());

        if let serde_json::Value::Object(obj) = json_data {
            for (key, value) in obj {
                if let Some(field) = descriptor.get_field_by_name(key) {
                    match value {
                        serde_json::Value::String(s) => {
                            if matches!(field.kind(), prost_reflect::Kind::Message(_)) {
                                // Skip complex message fields for now
                                continue;
                            }
                            message.set_field(&field, prost_reflect::Value::String(s.clone()));
                        }
                        serde_json::Value::Number(n) => {
                            if let Some(int_val) = n.as_i64() {
                                message.set_field(&field, prost_reflect::Value::I64(int_val));
                            } else if let Some(float_val) = n.as_f64() {
                                message.set_field(&field, prost_reflect::Value::F64(float_val));
                            }
                        }
                        serde_json::Value::Bool(b) => {
                            message.set_field(&field, prost_reflect::Value::Bool(*b));
                        }
                        _ => {
                            // Skip complex types for now
                            continue;
                        }
                    }
                }
            }
        }

        Ok(message)
    }

    /// Create a basic schema definition from a protobuf message descriptor
    #[cfg(feature = "data-faker")]
    fn create_schema_from_protobuf_descriptor(
        &self,
        descriptor: &prost_reflect::MessageDescriptor,
    ) -> SchemaDefinition {
        use mockforge_data::schema::FieldDefinition;

        let mut schema = SchemaDefinition::new(descriptor.name().to_string());

        for field in descriptor.fields() {
            let field_name = field.name().to_string();
            let field_type = match field.kind() {
                prost_reflect::Kind::Message(_) => {
                    // For nested messages, use a generic object type
                    "object".to_string()
                }
                prost_reflect::Kind::Enum(_) => {
                    "string".to_string()
                }
                // Simplified type mapping - default to string for all scalar types
                _ => "string".to_string()
            };

            let field_def = FieldDefinition::new(field_name, field_type);
            // For now, assume all fields are required
            // In a full implementation, we'd check the proto field descriptor
            // field_def = field_def.optional();

            schema = schema.with_field(field_def);
        }

        schema
    }
}

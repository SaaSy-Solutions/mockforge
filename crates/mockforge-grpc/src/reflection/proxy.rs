//! Main reflection proxy implementation

use crate::reflection::{
    cache::DescriptorCache, client::ReflectionClient, config::ProxyConfig,
    connection_pool::ConnectionPool,
};
use futures_util::Stream;
#[cfg(feature = "data-faker")]
use mockforge_data::{DataConfig, DataGenerator, SchemaDefinition};
use prost_reflect::{DynamicMessage, ReflectMessage};
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{transport::Endpoint, Request, Response, Status, Streaming};
use tracing::{debug, warn};

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
    #[allow(dead_code)]
    connection_pool: ConnectionPool,
}

impl ReflectionProxy {
    /// Create a new reflection proxy
    pub async fn new(endpoint: Endpoint, config: ProxyConfig) -> Result<Self, Status> {
        debug!("Creating reflection proxy for endpoint: {:?}", endpoint.uri());

        let client = ReflectionClient::new(endpoint).await?;
        let cache = DescriptorCache::new();

        // Populate cache from the client's descriptor pool
        cache.populate_from_pool(Some(client.pool())).await;

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
    ) -> Result<Response<Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>>, Status>
    {
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
    ) -> Result<Response<Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>>, Status>
    {
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
        request: Request<DynamicMessage>,
    ) -> Result<Response<DynamicMessage>, Status> {
        // Real implementation for mock server:
        // 1. Look up mock responses based on the service/method
        // 2. Apply any configured latency or error simulation
        // 3. Return the appropriate mock response with preserved metadata
        // 4. Preserve all metadata from the original request in the response

        debug!("Generating mock response for method: {}", method.name());

        // Extract service name from method descriptor
        let service_name = method.parent_service().name();
        let method_name = method.name();

        // Create a mock response based on the method
        let mock_response = self.generate_mock_response(service_name, method_name, &method).await?;

        // Create response with mock data and preserve metadata
        let mut response = Response::new(mock_response);

        // Preserve original request metadata in the response (ASCII only for simplicity)
        let request_metadata = request.metadata();
        for entry in request_metadata.iter() {
            if let tonic::metadata::KeyAndValueRef::Ascii(key, value) = entry {
                // Only preserve certain metadata keys, avoiding system headers
                if !key.as_str().starts_with(':')
                    && !key.as_str().starts_with("grpc-")
                    && !key.as_str().starts_with("te")
                    && !key.as_str().starts_with("content-type")
                {
                    response.metadata_mut().insert(key.clone(), value.clone());
                }
            }
        }

        // Add mock-specific metadata
        response
            .metadata_mut()
            .insert("x-mockforge-service", service_name.parse().unwrap());
        response
            .metadata_mut()
            .insert("x-mockforge-method", method_name.parse().unwrap());
        response
            .metadata_mut()
            .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());

        Ok(response)
    }

    /// Generate a mock response for a given service and method
    async fn generate_mock_response(
        &self,
        service_name: &str,
        method_name: &str,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> Result<DynamicMessage, Status> {
        debug!("Generating mock response for {}.{}", service_name, method_name);

        // Get the output message descriptor
        let output_descriptor = method_descriptor.output();

        // Create a new dynamic message with the output descriptor
        let mut response = DynamicMessage::new(output_descriptor.clone());

        // Generate mock data dynamically based on the proto structure
        self.populate_dynamic_mock_response(
            &mut response,
            service_name,
            method_name,
            &output_descriptor,
        )?;

        Ok(response)
    }

    /// Populate a dynamic mock response based on the proto structure
    fn populate_dynamic_mock_response(
        &self,
        response: &mut DynamicMessage,
        service_name: &str,
        method_name: &str,
        output_descriptor: &prost_reflect::MessageDescriptor,
    ) -> Result<(), Status> {
        debug!("Generating dynamic mock response for {}.{}", service_name, method_name);

        // Get all fields from the output message descriptor
        for field in output_descriptor.fields() {
            let field_name = field.name();
            let field_type = field.kind();

            debug!("Processing field: {} of type: {:?}", field_name, field_type);

            // Generate appropriate mock values based on field type
            let mock_value = self.generate_mock_value_for_field(&field, service_name, method_name);

            // Try to set the field (ignore errors if field doesn't exist or is wrong type)
            response.set_field(&field, mock_value);
        }

        // Always try to add some common metadata fields if they don't exist
        let metadata_fields = vec![
            ("mockforge_service", prost_reflect::Value::String(service_name.to_string())),
            ("mockforge_method", prost_reflect::Value::String(method_name.to_string())),
            (
                "mockforge_timestamp",
                prost_reflect::Value::String(chrono::Utc::now().to_rfc3339()),
            ),
            (
                "mockforge_source",
                prost_reflect::Value::String("MockForge Reflection Proxy".to_string()),
            ),
        ];

        for (field_name, value) in metadata_fields {
            response.set_field_by_name(field_name, value);
        }

        Ok(())
    }

    /// Generate a mock value for a specific field based on its type
    fn generate_mock_value_for_field(
        &self,
        field: &prost_reflect::FieldDescriptor,
        service_name: &str,
        method_name: &str,
    ) -> prost_reflect::Value {
        self.generate_mock_value_for_field_with_depth(field, service_name, method_name, 0)
    }

    /// Generate a mock value for a specific field with recursion depth limit
    fn generate_mock_value_for_field_with_depth(
        &self,
        field: &prost_reflect::FieldDescriptor,
        service_name: &str,
        method_name: &str,
        depth: usize,
    ) -> prost_reflect::Value {
        // Prevent infinite recursion with a reasonable depth limit
        const MAX_DEPTH: usize = 5;
        if depth >= MAX_DEPTH {
            return prost_reflect::Value::String(format!("max_depth_reached_{}", field.name()));
        }

        // Handle repeated fields (arrays)
        if field.is_list() {
            let mut list_values = Vec::new();
            // Generate 1-3 mock values for the list
            let field_name_lower = field.name().to_lowercase();
            let num_items =
                if field_name_lower.contains("list") || field_name_lower.contains("items") {
                    3
                } else {
                    1
                };

            for _ in 0..num_items {
                let item_value =
                    self.generate_single_field_value(field, service_name, method_name, depth);
                list_values.push(item_value);
            }

            return prost_reflect::Value::List(list_values);
        }

        self.generate_single_field_value(field, service_name, method_name, depth)
    }

    /// Generate a mock value for a single (non-repeated) field
    fn generate_single_field_value(
        &self,
        field: &prost_reflect::FieldDescriptor,
        service_name: &str,
        method_name: &str,
        depth: usize,
    ) -> prost_reflect::Value {
        let field_name = field.name().to_lowercase();
        let field_type = field.kind();

        // Generate contextual mock data based on field name patterns
        if field_name.contains("message")
            || field_name.contains("text")
            || field_name.contains("content")
        {
            return prost_reflect::Value::String(format!(
                "Mock response from {} for method {} at {}",
                service_name,
                method_name,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        if field_name.contains("id") {
            return prost_reflect::Value::String(format!(
                "mock_{}",
                chrono::Utc::now().timestamp()
            ));
        }

        if field_name.contains("status") || field_name.contains("state") {
            return prost_reflect::Value::String("success".to_string());
        }

        if field_name.contains("count") || field_name.contains("number") {
            return prost_reflect::Value::I64(42);
        }

        if field_name.contains("timestamp") || field_name.contains("time") {
            return prost_reflect::Value::String(chrono::Utc::now().to_rfc3339());
        }

        if field_name.contains("enabled") || field_name.contains("active") {
            return prost_reflect::Value::Bool(true);
        }

        // Default mock values based on field type
        match field_type {
            prost_reflect::Kind::String => {
                prost_reflect::Value::String(format!("mock_{}_{}", service_name, method_name))
            }
            prost_reflect::Kind::Int32 => prost_reflect::Value::I32(42),
            prost_reflect::Kind::Int64 => prost_reflect::Value::I64(42),
            prost_reflect::Kind::Float => prost_reflect::Value::F32(std::f32::consts::PI),
            prost_reflect::Kind::Double => prost_reflect::Value::F64(std::f64::consts::PI),
            prost_reflect::Kind::Bool => prost_reflect::Value::Bool(true),
            prost_reflect::Kind::Bytes => prost_reflect::Value::Bytes(b"mock_data".to_vec().into()),
            prost_reflect::Kind::Enum(enum_descriptor) => {
                // Try to get the first enum value, or use a default
                if let Some(first_value) = enum_descriptor.values().next() {
                    // Use the first enum value as the default
                    prost_reflect::Value::EnumNumber(first_value.number())
                } else {
                    // Fallback if no enum values are defined
                    prost_reflect::Value::EnumNumber(0)
                }
            }
            prost_reflect::Kind::Message(message_descriptor) => {
                // Recursively generate a mock message for nested types
                let mut nested_message = DynamicMessage::new(message_descriptor.clone());

                // Populate the nested message with mock values
                for nested_field in message_descriptor.fields() {
                    let mock_value = self.generate_mock_value_for_field_with_depth(
                        &nested_field,
                        service_name,
                        method_name,
                        depth + 1,
                    );
                    nested_message.set_field(&nested_field, mock_value);
                }

                prost_reflect::Value::Message(nested_message)
            }
            _ => prost_reflect::Value::String("mock_value".to_string()),
        }
    }

    /// Implementation for forwarding server streaming requests
    async fn forward_server_streaming_impl(
        &self,
        method: prost_reflect::MethodDescriptor,
        request: Request<DynamicMessage>,
    ) -> Result<Response<Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>>, Status>
    {
        // Extract metadata from the original request
        let metadata = request.metadata();
        debug!(
            "Forwarding server streaming request for method: {} with {} metadata entries",
            method.name(),
            metadata.len()
        );

        #[cfg(feature = "data-faker")]
        {
            // Generate mock streaming responses
            let output_descriptor = method.output();
            let messages = self.generate_mock_stream_messages(&output_descriptor, 5).await?;

            // Create a proper streaming response using ReceiverStream
            let (tx, rx) = mpsc::channel(32);
            let stream = Box::pin(ReceiverStream::new(rx))
                as Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>;

            // Spawn a task to send messages
            tokio::spawn(async move {
                for message in messages {
                    if tx.send(Ok(message)).await.is_err() {
                        break;
                    }
                }
            });

            // Preserve original request metadata in the response
            let mut response = Response::new(stream);

            // Copy relevant metadata from the original request to the response (ASCII only for simplicity)
            for entry in metadata.iter() {
                if let tonic::metadata::KeyAndValueRef::Ascii(key, value) = entry {
                    // Only preserve certain metadata keys, avoiding system headers
                    if !key.as_str().starts_with(':')
                        && !key.as_str().starts_with("grpc-")
                        && !key.as_str().starts_with("te")
                        && !key.as_str().starts_with("content-type")
                    {
                        response.metadata_mut().insert(key.clone(), value.clone());
                    }
                }
            }

            // Add mock-specific metadata
            response
                .metadata_mut()
                .insert("x-mockforge-service", method.parent_service().name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-method", method.name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());
            response.metadata_mut().insert("x-mockforge-stream-count", "5".parse().unwrap());

            debug!("Generated server streaming response with {} messages", 5);
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
            // Extract metadata from the original request before consuming it
            let request_metadata = request.metadata().clone();

            // Process the streaming request and extract message data
            let mut stream = request.into_inner();
            let mut message_count = 0;
            let mut processed_names = Vec::new();
            let mut user_ids = Vec::new();
            let mut all_tags = Vec::new();

            while let Some(message_result) = stream.next().await {
                match message_result {
                    Ok(message) => {
                        message_count += 1;
                        debug!(
                            "Processing client streaming message {} for method: {}",
                            message_count,
                            method.name()
                        );

                        // Extract data from the HelloRequest message
                        let input_descriptor = method.input();

                        // Extract the 'name' field
                        if let Some(name_field) = input_descriptor.get_field_by_name("name") {
                            let field_value = message.get_field(&name_field);
                            if let prost_reflect::Value::String(name) = field_value.into_owned() {
                                processed_names.push(name.clone());
                                debug!("  - Name: {}", name);
                            }
                        }

                        // Extract the 'user_info' field (nested message)
                        if let Some(user_info_field) =
                            input_descriptor.get_field_by_name("user_info")
                        {
                            let field_value = message.get_field(&user_info_field);
                            if let prost_reflect::Value::Message(user_info_msg) =
                                field_value.into_owned()
                            {
                                // Extract user_id from user_info
                                if let Some(user_id_field) =
                                    user_info_msg.descriptor().get_field_by_name("user_id")
                                {
                                    let user_id_value = user_info_msg.get_field(&user_id_field);
                                    if let prost_reflect::Value::String(user_id) =
                                        user_id_value.into_owned()
                                    {
                                        user_ids.push(user_id.clone());
                                        debug!("  - User ID: {}", user_id);
                                    }
                                }
                            }
                        }

                        // Extract the 'tags' field (repeated string)
                        if let Some(tags_field) = input_descriptor.get_field_by_name("tags") {
                            let field_value = message.get_field(&tags_field);
                            if let prost_reflect::Value::List(tags_list) = field_value.into_owned()
                            {
                                for tag_value in tags_list {
                                    if let prost_reflect::Value::String(tag) = tag_value {
                                        all_tags.push(tag.clone());
                                        debug!("  - Tag: {}", tag);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error receiving client streaming message: {}", e);
                        return Err(Status::internal(format!(
                            "Error processing streaming request: {}",
                            e
                        )));
                    }
                }
            }

            debug!("Processed {} messages in client streaming request", message_count);
            debug!(
                "Collected data - Names: {:?}, User IDs: {:?}, Tags: {:?}",
                processed_names, user_ids, all_tags
            );

            // Generate a mock response based on the output descriptor, but enhance it with processed data
            let output_descriptor = method.output();
            let mut mock_response = self.generate_mock_message(&output_descriptor).await?;

            // Enhance the response message with aggregated data from the stream
            if let Some(message_field) = output_descriptor.get_field_by_name("message") {
                // Create a personalized message based on the processed data
                let personalized_message = if !processed_names.is_empty() {
                    format!("Hello to all {} senders! Processed names: {}, with {} unique tags from {} users",
                           message_count, processed_names.join(", "), all_tags.len(), user_ids.len())
                } else {
                    format!(
                        "Hello! Processed {} messages with {} tags",
                        message_count,
                        all_tags.len()
                    )
                };

                // Update the message field in the response
                mock_response
                    .set_field(&message_field, prost_reflect::Value::String(personalized_message));
            }

            // Preserve original request metadata in the response
            let mut response = Response::new(mock_response);

            // Copy relevant metadata from the original request to the response (ASCII only for simplicity)
            for entry in request_metadata.iter() {
                if let tonic::metadata::KeyAndValueRef::Ascii(key, value) = entry {
                    // Only preserve certain metadata keys, avoiding system headers
                    if !key.as_str().starts_with(':')
                        && !key.as_str().starts_with("grpc-")
                        && !key.as_str().starts_with("te")
                        && !key.as_str().starts_with("content-type")
                    {
                        response.metadata_mut().insert(key.clone(), value.clone());
                    }
                }
            }

            // Add mock-specific metadata
            response
                .metadata_mut()
                .insert("x-mockforge-service", method.parent_service().name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-method", method.name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-message-count", message_count.to_string().parse().unwrap());

            let response = response;

            debug!(
                "Generated enhanced client streaming response with {} processed messages",
                message_count
            );
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
    ) -> Result<Response<Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>>, Status>
    {
        debug!("Forwarding bidirectional streaming request for method: {}", method.name());

        #[cfg(feature = "data-faker")]
        {
            // Extract metadata from the original request before consuming it
            let metadata = request.metadata();
            debug!("Forwarding bidirectional streaming request for method: {} with {} metadata entries",
                   method.name(), metadata.len());

            // Generate mock bidirectional streaming responses
            let output_descriptor = method.output();
            let messages = self.generate_mock_stream_messages(&output_descriptor, 10).await?;

            // Create streaming response using ReceiverStream
            let (tx, rx) = mpsc::channel(32);
            let stream = Box::pin(ReceiverStream::new(rx))
                as Pin<Box<dyn Stream<Item = Result<DynamicMessage, Status>> + Send>>;

            // Spawn a task to send messages
            tokio::spawn(async move {
                for message in messages {
                    if tx.send(Ok(message)).await.is_err() {
                        break;
                    }
                }
            });

            // Preserve original request metadata in the response
            let mut response = Response::new(stream);

            // Copy relevant metadata from the original request to the response (ASCII only for simplicity)
            for entry in metadata.iter() {
                if let tonic::metadata::KeyAndValueRef::Ascii(key, value) = entry {
                    // Only preserve certain metadata keys, avoiding system headers
                    if !key.as_str().starts_with(':')
                        && !key.as_str().starts_with("grpc-")
                        && !key.as_str().starts_with("te")
                        && !key.as_str().starts_with("content-type")
                    {
                        response.metadata_mut().insert(key.clone(), value.clone());
                    }
                }
            }

            // Add mock-specific metadata
            response
                .metadata_mut()
                .insert("x-mockforge-service", method.parent_service().name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-method", method.name().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());
            response
                .metadata_mut()
                .insert("x-mockforge-stream-count", "10".parse().unwrap());

            // Process incoming stream concurrently
            let mut incoming_stream = request.into_inner();
            tokio::spawn(async move {
                let mut count = 0;
                while let Some(message_result) = incoming_stream.next().await {
                    match message_result {
                        Ok(_) => {
                            count += 1;
                            debug!(
                                "Processed bidirectional message {} for method: {}",
                                count,
                                method.name()
                            );
                        }
                        Err(e) => {
                            warn!("Error processing bidirectional message: {}", e);
                            break;
                        }
                    }
                }
                debug!("Finished processing {} bidirectional messages", count);
            });

            debug!("Generated bidirectional streaming response with {} messages", 10);
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

        let result = generator
            .generate()
            .await
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

        let result = generator
            .generate()
            .await
            .map_err(|e| Status::internal(format!("Failed to generate mock data: {}", e)))?;

        result
            .data
            .iter()
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
                    let field_value = self.convert_json_value_to_protobuf_value(&field, value)?;
                    message.set_field(&field, field_value);
                }
            }
        }

        Ok(message)
    }

    /// Convert a JSON value to a protobuf Value based on the field descriptor
    #[cfg(feature = "data-faker")]
    fn convert_json_value_to_protobuf_value(
        &self,
        field: &prost_reflect::FieldDescriptor,
        json_value: &serde_json::Value,
    ) -> Result<prost_reflect::Value, Status> {
        use prost_reflect::Kind;

        match json_value {
            serde_json::Value::Null => {
                // Return default value for the field type
                match field.kind() {
                    Kind::Message(message_descriptor) => Ok(prost_reflect::Value::Message(
                        DynamicMessage::new(message_descriptor.clone()),
                    )),
                    Kind::Enum(enum_descriptor) => {
                        // Try to get the first enum value, or use 0 as default
                        if let Some(first_value) = enum_descriptor.values().next() {
                            Ok(prost_reflect::Value::EnumNumber(first_value.number()))
                        } else {
                            Ok(prost_reflect::Value::EnumNumber(0))
                        }
                    }
                    Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Ok(prost_reflect::Value::I32(0)),
                    Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Ok(prost_reflect::Value::I64(0)),
                    Kind::Uint32 | Kind::Fixed32 => Ok(prost_reflect::Value::U32(0)),
                    Kind::Uint64 | Kind::Fixed64 => Ok(prost_reflect::Value::U64(0)),
                    Kind::Float => Ok(prost_reflect::Value::F32(0.0)),
                    Kind::Double => Ok(prost_reflect::Value::F64(0.0)),
                    Kind::Bool => Ok(prost_reflect::Value::Bool(false)),
                    Kind::String => Ok(prost_reflect::Value::String(String::new())),
                    Kind::Bytes => Ok(prost_reflect::Value::Bytes(b"".to_vec().into())),
                }
            }
            serde_json::Value::Bool(b) => Ok(prost_reflect::Value::Bool(*b)),
            serde_json::Value::Number(n) => {
                match field.kind() {
                    Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                        if let Some(i) = n.as_i64() {
                            Ok(prost_reflect::Value::I32(i as i32))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to int32",
                                n
                            )))
                        }
                    }
                    Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                        if let Some(i) = n.as_i64() {
                            Ok(prost_reflect::Value::I64(i))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to int64",
                                n
                            )))
                        }
                    }
                    Kind::Uint32 | Kind::Fixed32 => {
                        if let Some(i) = n.as_u64() {
                            Ok(prost_reflect::Value::U32(i as u32))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to uint32",
                                n
                            )))
                        }
                    }
                    Kind::Uint64 | Kind::Fixed64 => {
                        if let Some(i) = n.as_u64() {
                            Ok(prost_reflect::Value::U64(i))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to uint64",
                                n
                            )))
                        }
                    }
                    Kind::Float => {
                        if let Some(f) = n.as_f64() {
                            Ok(prost_reflect::Value::F32(f as f32))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to float",
                                n
                            )))
                        }
                    }
                    Kind::Double => {
                        if let Some(f) = n.as_f64() {
                            Ok(prost_reflect::Value::F64(f))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to double",
                                n
                            )))
                        }
                    }
                    _ => {
                        // Fallback to int64 for unknown numeric types
                        if let Some(i) = n.as_i64() {
                            Ok(prost_reflect::Value::I64(i))
                        } else {
                            Err(Status::invalid_argument(format!(
                                "Cannot convert number {} to numeric type",
                                n
                            )))
                        }
                    }
                }
            }
            serde_json::Value::String(s) => {
                match field.kind() {
                    Kind::String => Ok(prost_reflect::Value::String(s.clone())),
                    Kind::Bytes => Ok(prost_reflect::Value::Bytes(s.as_bytes().to_vec().into())),
                    Kind::Enum(enum_descriptor) => {
                        // Try to convert string to enum value
                        if let Some(enum_value) = enum_descriptor.get_value_by_name(s) {
                            Ok(prost_reflect::Value::EnumNumber(enum_value.number()))
                        } else {
                            // Try to parse as number
                            if let Ok(num) = s.parse::<i32>() {
                                Ok(prost_reflect::Value::EnumNumber(num))
                            } else {
                                warn!(
                                    "Unknown enum value '{}' for field '{}', using default",
                                    s,
                                    field.name()
                                );
                                Ok(prost_reflect::Value::EnumNumber(0))
                            }
                        }
                    }
                    _ => {
                        // For other types, treat string as string
                        Ok(prost_reflect::Value::String(s.clone()))
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                let mut list_values = Vec::new();

                for item in arr {
                    let item_value = self.convert_json_value_to_protobuf_value(field, item)?;
                    list_values.push(item_value);
                }

                Ok(prost_reflect::Value::List(list_values))
            }
            serde_json::Value::Object(_obj) => match field.kind() {
                Kind::Message(message_descriptor) => self
                    .json_to_dynamic_message(&message_descriptor, json_value)
                    .map(prost_reflect::Value::Message),
                _ => Err(Status::invalid_argument(format!(
                    "Cannot convert object to field {} of type {:?}",
                    field.name(),
                    field.kind()
                ))),
            },
        }
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
                prost_reflect::Kind::Enum(_) => "string".to_string(),
                // Simplified type mapping - default to string for all scalar types
                _ => "string".to_string(),
            };

            let mut field_def = FieldDefinition::new(field_name, field_type);

            // Check if field is optional based on protobuf field properties
            // In proto3, all non-repeated fields are effectively optional
            // In proto2, only explicitly optional or required fields exist
            if field.supports_presence() && !field.is_list() {
                // Field supports presence detection and is not repeated, so it's optional
                field_def = field_def.optional();
            }

            schema = schema.with_field(field_def);
        }

        schema
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

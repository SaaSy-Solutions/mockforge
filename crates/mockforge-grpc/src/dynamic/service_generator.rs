//! Dynamic gRPC service generation
//!
//! This module generates actual gRPC service implementations from parsed proto definitions.

use crate::dynamic::proto_parser::{ProtoMethod, ProtoParser, ProtoService};
use crate::reflection::smart_mock_generator::{SmartMockConfig, SmartMockGenerator};
use mockforge_core::latency::LatencyInjector;
use prost_reflect::DescriptorPool;
use prost_types::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, warn};

/// Service factory for creating enhanced gRPC services from proto files
pub struct EnhancedServiceFactory;

impl EnhancedServiceFactory {
    /// Create services from a proto directory with enhanced capabilities
    pub async fn create_services_from_proto_dir(
        proto_dir: &str,
        latency_injector: Option<LatencyInjector>,
        smart_config: SmartMockConfig,
    ) -> Result<Vec<DynamicGrpcService>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Creating enhanced services from proto directory: {}", proto_dir);

        // Parse proto files with full protoc support
        let mut parser = ProtoParser::new();
        parser.parse_directory(proto_dir).await?;

        let mut services = Vec::new();

        // Store services info before consuming parser
        let services_info: Vec<(String, ProtoService)> = parser
            .services()
            .iter()
            .map(|(name, service)| (name.clone(), service.clone()))
            .collect();

        // Create enhanced services for each parsed service
        for (service_name, proto_service) in services_info {
            debug!("Creating enhanced service: {}", service_name);

            // Create a new parser instance for each service (we'll improve this later)
            let mut service_parser = ProtoParser::new();
            let _ = service_parser.parse_directory(proto_dir).await; // Re-parse for now

            let service = DynamicGrpcService::new_enhanced(
                proto_service,
                latency_injector.clone(),
                Some(service_parser),
                smart_config.clone(),
            );

            services.push(service);
        }

        info!("Created {} enhanced services", services.len());
        Ok(services)
    }

    /// Create a single service from proto service definition
    pub fn create_service_from_proto(
        proto_service: ProtoService,
        latency_injector: Option<LatencyInjector>,
        proto_parser: Option<ProtoParser>,
        smart_config: SmartMockConfig,
    ) -> DynamicGrpcService {
        if proto_parser.is_some() {
            info!("Creating enhanced service: {}", proto_service.name);
            DynamicGrpcService::new_enhanced(
                proto_service,
                latency_injector,
                proto_parser,
                smart_config,
            )
        } else {
            info!("Creating basic service: {}", proto_service.name);
            DynamicGrpcService::new(proto_service, latency_injector)
        }
    }
}

/// A dynamically generated gRPC service
pub struct DynamicGrpcService {
    /// The service definition
    service: ProtoService,
    /// Latency injector for simulating delays
    latency_injector: Option<LatencyInjector>,
    /// Mock responses for each method
    mock_responses: HashMap<String, MockResponse>,
    /// Proto parser with descriptor pool for advanced type support
    proto_parser: Option<ProtoParser>,
    /// Smart mock generator for intelligent data generation
    smart_generator: Arc<Mutex<SmartMockGenerator>>,
}

/// Configuration for mock responses
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// The response message as JSON
    pub response_json: String,
    /// Whether to simulate an error
    pub simulate_error: bool,
    /// Error message if simulating an error
    pub error_message: Option<String>,
    /// Error code if simulating an error
    pub error_code: Option<i32>,
}

impl DynamicGrpcService {
    /// Create a new dynamic gRPC service
    pub fn new(service: ProtoService, latency_injector: Option<LatencyInjector>) -> Self {
        let mut mock_responses = HashMap::new();

        // Generate default mock responses for each method
        for method in &service.methods {
            let response = Self::generate_mock_response(&method.name, &method.output_type);
            mock_responses.insert(method.name.clone(), response);
        }

        Self {
            service,
            latency_injector,
            mock_responses,
            proto_parser: None,
            smart_generator: Arc::new(Mutex::new(SmartMockGenerator::new(
                SmartMockConfig::default(),
            ))),
        }
    }

    /// Create a new enhanced dynamic gRPC service with proto parser and smart generator
    pub fn new_enhanced(
        service: ProtoService,
        latency_injector: Option<LatencyInjector>,
        proto_parser: Option<ProtoParser>,
        smart_config: SmartMockConfig,
    ) -> Self {
        let mut mock_responses = HashMap::new();
        let smart_generator = Arc::new(Mutex::new(SmartMockGenerator::new(smart_config)));

        // Generate enhanced mock responses for each method using smart generator
        for method in &service.methods {
            let response = if proto_parser.is_some() {
                Self::generate_enhanced_mock_response(
                    &method.name,
                    &method.output_type,
                    &service.name,
                    &smart_generator,
                )
            } else {
                Self::generate_mock_response(&method.name, &method.output_type)
            };
            mock_responses.insert(method.name.clone(), response);
        }

        Self {
            service,
            latency_injector,
            mock_responses,
            proto_parser,
            smart_generator,
        }
    }

    /// Generate a mock response for a method
    fn generate_mock_response(method_name: &str, output_type: &str) -> MockResponse {
        // Generate different responses based on method name
        let response_json = match method_name {
            "SayHello" | "SayHelloStream" | "SayHelloClientStream" | "Chat" => {
                r#"{"message": "Hello from MockForge!"}"#.to_string()
            }
            _ => {
                // Generic response for unknown methods
                format!(
                    r#"{{"result": "Mock response for {}", "type": "{}"}}"#,
                    method_name, output_type
                )
            }
        };

        MockResponse {
            response_json,
            simulate_error: false,
            error_message: None,
            error_code: None,
        }
    }

    /// Generate an enhanced mock response using smart generator
    fn generate_enhanced_mock_response(
        method_name: &str,
        output_type: &str,
        service_name: &str,
        smart_generator: &Arc<Mutex<SmartMockGenerator>>,
    ) -> MockResponse {
        debug!("Generating enhanced mock response for {}.{}", service_name, method_name);

        // Use smart generator for more realistic responses
        let response_json = if let Ok(mut generator) = smart_generator.lock() {
            // Create sample fields based on common gRPC response patterns
            let mut fields = HashMap::new();

            // Add common response fields based on method name
            match method_name.to_lowercase().as_str() {
                name if name.contains("hello") || name.contains("greet") => {
                    fields.insert("message".to_string(), "greeting".to_string());
                    fields.insert("name".to_string(), "user_name".to_string());
                    fields.insert("timestamp".to_string(), "timestamp".to_string());
                }
                name if name.contains("list") || name.contains("get") => {
                    fields.insert("id".to_string(), "identifier".to_string());
                    fields.insert("data".to_string(), "response_data".to_string());
                    fields.insert("count".to_string(), "total_count".to_string());
                }
                name if name.contains("create") || name.contains("add") => {
                    fields.insert("id".to_string(), "new_id".to_string());
                    fields.insert("status".to_string(), "status".to_string());
                    fields.insert("message".to_string(), "success_message".to_string());
                }
                name if name.contains("update") || name.contains("modify") => {
                    fields.insert("updated".to_string(), "updated_fields".to_string());
                    fields.insert("version".to_string(), "version_number".to_string());
                    fields.insert("status".to_string(), "status".to_string());
                }
                name if name.contains("delete") || name.contains("remove") => {
                    fields.insert("deleted".to_string(), "deleted_status".to_string());
                    fields.insert("message".to_string(), "confirmation_message".to_string());
                }
                _ => {
                    // Generic response structure
                    fields.insert("result".to_string(), "result_data".to_string());
                    fields.insert("status".to_string(), "status".to_string());
                    fields.insert("message".to_string(), "response_message".to_string());
                }
            }

            // Generate JSON response using field patterns
            let mut json_parts = Vec::new();
            for (field_name, field_type) in fields {
                let mock_value = match field_type.as_str() {
                    "greeting" => {
                        format!("\"Hello from enhanced MockForge service {}!\"", service_name)
                    }
                    "user_name" => "\"MockForge User\"".to_string(),
                    "timestamp" => format!(
                        "\"{}\"",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    ),
                    "identifier" | "new_id" => format!("{}", generator.next_sequence()),
                    "total_count" => "42".to_string(),
                    "status" => "\"success\"".to_string(),
                    "success_message" => {
                        format!("\"Successfully processed {} request\"", method_name)
                    }
                    "confirmation_message" => {
                        format!("\"Operation {} completed successfully\"", method_name)
                    }
                    "version_number" => "\"1.0.0\"".to_string(),
                    "updated_status" | "deleted_status" => "true".to_string(),
                    _ => format!("\"Enhanced mock data for {}\"", field_type),
                };
                json_parts.push(format!("\"{}\": {}", field_name, mock_value));
            }

            format!("{{{}}}", json_parts.join(", "))
        } else {
            // Fallback to basic response if generator lock fails
            format!(
                r#"{{"result": "Enhanced mock response for {}", "type": "{}"}}"#,
                method_name, output_type
            )
        };

        MockResponse {
            response_json,
            simulate_error: false,
            error_message: None,
            error_code: None,
        }
    }

    /// Get the descriptor pool if available
    pub fn descriptor_pool(&self) -> Option<&DescriptorPool> {
        self.proto_parser.as_ref().map(|parser| parser.pool())
    }

    /// Get the smart generator for external use
    pub fn smart_generator(&self) -> &Arc<Mutex<SmartMockGenerator>> {
        &self.smart_generator
    }

    /// Get the service definition
    pub fn service(&self) -> &ProtoService {
        &self.service
    }

    /// Handle a unary request
    pub async fn handle_unary(
        &self,
        method_name: &str,
        _request: Request<Any>,
    ) -> Result<Response<Any>, Status> {
        debug!("Handling unary request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // Get mock response for this method
        let mock_response = self
            .mock_responses
            .get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response
                .error_message
                .as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(tonic::Code::from_i32(error_code), error_message));
        }

        // Create response
        let response = Any {
            type_url: format!("type.googleapis.com/{}", self.get_output_type(method_name)),
            value: mock_response.response_json.as_bytes().to_vec(),
        };

        Ok(Response::new(response))
    }

    /// Handle a server streaming request
    pub async fn handle_server_streaming(
        &self,
        method_name: &str,
        request: Request<Any>,
    ) -> Result<Response<ReceiverStream<Result<Any, Status>>>, Status> {
        debug!("Handling server streaming request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // Get mock response for this method
        let mock_response = self
            .mock_responses
            .get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response
                .error_message
                .as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(tonic::Code::from_i32(error_code), error_message));
        }

        // Create a streaming response
        let stream = self
            .create_server_stream(method_name, &request.into_inner(), mock_response)
            .await?;
        Ok(Response::new(stream))
    }

    /// Create a server streaming response
    async fn create_server_stream(
        &self,
        method_name: &str,
        _request: &Any,
        mock_response: &MockResponse,
    ) -> Result<ReceiverStream<Result<Any, Status>>, Status> {
        debug!("Creating server stream for method: {}", method_name);

        let (tx, rx) = mpsc::channel(10);
        let method_name = method_name.to_string();
        let output_type = self.get_output_type(&method_name);
        let response_json = mock_response.response_json.clone();

        // Spawn a task to generate stream messages
        tokio::spawn(async move {
            // Generate multiple stream messages (3-5 messages per stream)
            let message_count = 3 + (method_name.len() % 3); // 3-5 messages based on method name

            for i in 0..message_count {
                // Create a mock response message
                let stream_response = Self::create_stream_response_message(
                    &method_name,
                    &output_type,
                    &response_json,
                    i,
                    message_count,
                );

                if tx.send(Ok(stream_response)).await.is_err() {
                    debug!("Stream receiver dropped for method: {}", method_name);
                    break; // Receiver dropped
                }

                // Add delay between messages to simulate realistic streaming
                let delay = Duration::from_millis(100 + (i as u64 * 50)); // Progressive delay
                tokio::time::sleep(delay).await;
            }

            info!(
                "Completed server streaming for method: {} with {} messages",
                method_name, message_count
            );
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a single stream response message
    fn create_stream_response_message(
        method_name: &str,
        output_type: &str,
        base_response: &str,
        index: usize,
        total: usize,
    ) -> Any {
        // Create a streaming-specific response by modifying the base response
        let stream_response = if base_response.starts_with('{') && base_response.ends_with('}') {
            // It's JSON, add streaming fields
            let mut response = base_response.trim_end_matches('}').to_string();
            response.push_str(&format!(
                r#", "stream_index": {}, "stream_total": {}, "is_final": {}, "timestamp": "{}""#,
                index,
                total,
                index == total - 1,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            response.push('}');
            response
        } else {
            // It's a simple string, create a structured response
            format!(
                r#"{{"message": "{}", "stream_index": {}, "stream_total": {}, "is_final": {}, "method": "{}"}}"#,
                base_response.replace('"', r#"\""#), // Escape quotes
                index,
                total,
                index == total - 1,
                method_name
            )
        };

        Any {
            type_url: format!("type.googleapis.com/{}", output_type),
            value: stream_response.as_bytes().to_vec(),
        }
    }

    /// Handle a client streaming request
    pub async fn handle_client_streaming(
        &self,
        method_name: &str,
        mut request: Request<Streaming<Any>>,
    ) -> Result<Response<Any>, Status> {
        debug!("Handling client streaming request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // Collect all client messages
        let mut messages = Vec::new();
        while let Ok(Some(message)) = request.get_mut().message().await {
            messages.push(message);
        }

        debug!("Received {} client messages", messages.len());

        // Get mock response for this method
        let mock_response = self
            .mock_responses
            .get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response
                .error_message
                .as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(tonic::Code::from_i32(error_code), error_message));
        }

        // Create response based on collected messages
        let response = Any {
            type_url: format!("type.googleapis.com/{}", self.get_output_type(method_name)),
            value: format!(
                r#"{{"message": "Processed {} messages from MockForge!"}}"#,
                messages.len()
            )
            .as_bytes()
            .to_vec(),
        };

        Ok(Response::new(response))
    }

    /// Handle a bidirectional streaming request
    pub async fn handle_bidirectional_streaming(
        &self,
        method_name: &str,
        request: Request<Streaming<Any>>,
    ) -> Result<Response<ReceiverStream<Result<Any, Status>>>, Status> {
        debug!("Handling bidirectional streaming request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // Get mock response for this method
        let mock_response = self
            .mock_responses
            .get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response
                .error_message
                .as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(tonic::Code::from_i32(error_code), error_message));
        }

        // Create a bidirectional streaming response
        let stream = self.create_bidirectional_stream(method_name, request, mock_response).await?;
        Ok(Response::new(stream))
    }

    /// Create a bidirectional streaming response
    async fn create_bidirectional_stream(
        &self,
        method_name: &str,
        mut request: Request<Streaming<Any>>,
        mock_response: &MockResponse,
    ) -> Result<ReceiverStream<Result<Any, Status>>, Status> {
        debug!("Creating bidirectional stream for method: {}", method_name);

        let (tx, rx) = mpsc::channel(10);
        let method_name = method_name.to_string();
        let output_type = self.get_output_type(&method_name);
        let response_json = mock_response.response_json.clone();

        // Spawn a task to handle bidirectional streaming
        tokio::spawn(async move {
            let mut input_count = 0;
            let mut output_count = 0;

            // Read from input stream and respond to each message
            while let Ok(Some(input_message)) = request.get_mut().message().await {
                input_count += 1;
                debug!(
                    "Received bidirectional input message {} for method: {}",
                    input_count, method_name
                );

                // For each input message, generate 1-2 response messages
                let responses_per_input = if input_count % 3 == 0 { 2 } else { 1 };

                for response_idx in 0..responses_per_input {
                    output_count += 1;

                    // Create a bidirectional response message
                    let response_message = Self::create_bidirectional_response_message(
                        &method_name,
                        &output_type,
                        &response_json,
                        &input_message,
                        input_count,
                        output_count,
                        response_idx,
                    );

                    if tx.send(Ok(response_message)).await.is_err() {
                        debug!("Bidirectional stream receiver dropped for method: {}", method_name);
                        return;
                    }

                    // Add small delay between responses
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }

                // Limit the number of messages we process to prevent infinite loops
                if input_count >= 100 {
                    warn!(
                        "Reached maximum input message limit (100) for bidirectional method: {}",
                        method_name
                    );
                    break;
                }
            }

            info!("Bidirectional streaming completed for method: {}: processed {} inputs, sent {} outputs",
                  method_name, input_count, output_count);
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Create a single bidirectional response message
    fn create_bidirectional_response_message(
        method_name: &str,
        output_type: &str,
        base_response: &str,
        input_message: &Any,
        input_sequence: usize,
        output_sequence: usize,
        response_index: usize,
    ) -> Any {
        // Try to extract some context from the input message
        let input_context = if let Ok(input_str) = String::from_utf8(input_message.value.clone()) {
            if input_str.len() < 200 {
                // Reasonable length limit
                input_str
            } else {
                format!("Large input ({} bytes)", input_message.value.len())
            }
        } else {
            format!("Binary input ({} bytes)", input_message.value.len())
        };

        // Create a bidirectional response
        let response_json = if base_response.starts_with('{') && base_response.ends_with('}') {
            // It's JSON, add bidirectional fields
            let mut response = base_response.trim_end_matches('}').to_string();
            response.push_str(&format!(
                r#", "input_sequence": {}, "output_sequence": {}, "response_index": {}, "input_context": "{}", "is_final": {}, "timestamp": "{}""#,
                input_sequence,
                output_sequence,
                response_index,
                input_context.replace('"', r#"\""#), // Escape quotes
                response_index > 0, // Mark as final if this is the second response
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            response.push('}');
            response
        } else {
            // It's a simple string, create a structured response
            format!(
                r#"{{"message": "{}", "input_sequence": {}, "output_sequence": {}, "response_index": {}, "input_context": "{}", "method": "{}"}}"#,
                base_response.replace('"', r#"\""#), // Escape quotes
                input_sequence,
                output_sequence,
                response_index,
                input_context.replace('"', r#"\""#), // Escape quotes
                method_name
            )
        };

        Any {
            type_url: format!("type.googleapis.com/{}", output_type),
            value: response_json.as_bytes().to_vec(),
        }
    }

    /// Get the output type for a method
    fn get_output_type(&self, method_name: &str) -> String {
        self.service
            .methods
            .iter()
            .find(|m| m.name == method_name)
            .map(|m| m.output_type.clone())
            .unwrap_or_else(|| "google.protobuf.Any".to_string())
    }

    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service.name
    }

    /// Set a custom mock response for a method
    pub fn set_mock_response(&mut self, method_name: &str, response: MockResponse) {
        self.mock_responses.insert(method_name.to_string(), response);
    }

    /// Set error simulation for a method
    pub fn set_error_simulation(
        &mut self,
        method_name: &str,
        error_message: &str,
        error_code: i32,
    ) {
        if let Some(mock_response) = self.mock_responses.get_mut(method_name) {
            mock_response.simulate_error = true;
            mock_response.error_message = Some(error_message.to_string());
            mock_response.error_code = Some(error_code);
        }
    }

    /// Get the service methods
    pub fn methods(&self) -> &Vec<ProtoMethod> {
        &self.service.methods
    }

    /// Get the service package
    pub fn package(&self) -> &str {
        &self.service.package
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test ProtoService
    fn create_test_proto_service() -> ProtoService {
        ProtoService {
            name: "test.package.TestService".to_string(),
            package: "test.package".to_string(),
            short_name: "TestService".to_string(),
            methods: vec![
                ProtoMethod {
                    name: "SayHello".to_string(),
                    input_type: "HelloRequest".to_string(),
                    output_type: "HelloResponse".to_string(),
                    client_streaming: false,
                    server_streaming: false,
                },
                ProtoMethod {
                    name: "GetUser".to_string(),
                    input_type: "GetUserRequest".to_string(),
                    output_type: "GetUserResponse".to_string(),
                    client_streaming: false,
                    server_streaming: false,
                },
                ProtoMethod {
                    name: "CreateItem".to_string(),
                    input_type: "CreateItemRequest".to_string(),
                    output_type: "CreateItemResponse".to_string(),
                    client_streaming: false,
                    server_streaming: false,
                },
            ],
        }
    }

    fn create_streaming_proto_service() -> ProtoService {
        ProtoService {
            name: "streaming.package.StreamingService".to_string(),
            package: "streaming.package".to_string(),
            short_name: "StreamingService".to_string(),
            methods: vec![
                ProtoMethod {
                    name: "ServerStream".to_string(),
                    input_type: "StreamRequest".to_string(),
                    output_type: "StreamResponse".to_string(),
                    client_streaming: false,
                    server_streaming: true,
                },
                ProtoMethod {
                    name: "ClientStream".to_string(),
                    input_type: "StreamRequest".to_string(),
                    output_type: "StreamResponse".to_string(),
                    client_streaming: true,
                    server_streaming: false,
                },
                ProtoMethod {
                    name: "BiDiStream".to_string(),
                    input_type: "StreamRequest".to_string(),
                    output_type: "StreamResponse".to_string(),
                    client_streaming: true,
                    server_streaming: true,
                },
            ],
        }
    }

    // ==================== MockResponse Tests ====================

    #[test]
    fn test_mock_response_creation() {
        let response = MockResponse {
            response_json: r#"{"message": "test"}"#.to_string(),
            simulate_error: false,
            error_message: None,
            error_code: None,
        };

        assert_eq!(response.response_json, r#"{"message": "test"}"#);
        assert!(!response.simulate_error);
        assert!(response.error_message.is_none());
        assert!(response.error_code.is_none());
    }

    #[test]
    fn test_mock_response_with_error() {
        let response = MockResponse {
            response_json: "{}".to_string(),
            simulate_error: true,
            error_message: Some("Test error".to_string()),
            error_code: Some(3),
        };

        assert!(response.simulate_error);
        assert_eq!(response.error_message, Some("Test error".to_string()));
        assert_eq!(response.error_code, Some(3));
    }

    #[test]
    fn test_mock_response_clone() {
        let response = MockResponse {
            response_json: r#"{"key": "value"}"#.to_string(),
            simulate_error: true,
            error_message: Some("error".to_string()),
            error_code: Some(5),
        };

        let cloned = response.clone();
        assert_eq!(cloned.response_json, response.response_json);
        assert_eq!(cloned.simulate_error, response.simulate_error);
        assert_eq!(cloned.error_message, response.error_message);
        assert_eq!(cloned.error_code, response.error_code);
    }

    // ==================== DynamicGrpcService Basic Tests ====================

    #[test]
    fn test_dynamic_grpc_service_new() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        assert_eq!(service.service_name(), "test.package.TestService");
        assert_eq!(service.package(), "test.package");
        assert_eq!(service.methods().len(), 3);
    }

    #[test]
    fn test_dynamic_grpc_service_with_latency_injector() {
        use mockforge_core::latency::{FaultConfig, LatencyProfile};

        let proto_service = create_test_proto_service();
        let latency_injector =
            LatencyInjector::new(LatencyProfile::default(), FaultConfig::default());
        let service = DynamicGrpcService::new(proto_service, Some(latency_injector));

        assert_eq!(service.service_name(), "test.package.TestService");
        assert!(service.latency_injector.is_some());
    }

    #[test]
    fn test_dynamic_grpc_service_service_accessor() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let accessed_service = service.service();
        assert_eq!(accessed_service.name, "test.package.TestService");
        assert_eq!(accessed_service.package, "test.package");
        assert_eq!(accessed_service.short_name, "TestService");
    }

    #[test]
    fn test_dynamic_grpc_service_methods_accessor() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let methods = service.methods();
        assert_eq!(methods.len(), 3);
        assert_eq!(methods[0].name, "SayHello");
        assert_eq!(methods[1].name, "GetUser");
        assert_eq!(methods[2].name, "CreateItem");
    }

    #[test]
    fn test_dynamic_grpc_service_package_accessor() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        assert_eq!(service.package(), "test.package");
    }

    // ==================== Mock Response Generation Tests ====================

    #[test]
    fn test_generate_mock_response_say_hello() {
        let response = DynamicGrpcService::generate_mock_response("SayHello", "HelloResponse");

        assert!(response.response_json.contains("Hello from MockForge"));
        assert!(!response.simulate_error);
    }

    #[test]
    fn test_generate_mock_response_say_hello_stream() {
        let response =
            DynamicGrpcService::generate_mock_response("SayHelloStream", "HelloResponse");

        assert!(response.response_json.contains("Hello from MockForge"));
    }

    #[test]
    fn test_generate_mock_response_client_stream() {
        let response =
            DynamicGrpcService::generate_mock_response("SayHelloClientStream", "HelloResponse");

        assert!(response.response_json.contains("Hello from MockForge"));
    }

    #[test]
    fn test_generate_mock_response_chat() {
        let response = DynamicGrpcService::generate_mock_response("Chat", "ChatResponse");

        assert!(response.response_json.contains("Hello from MockForge"));
    }

    #[test]
    fn test_generate_mock_response_generic() {
        let response = DynamicGrpcService::generate_mock_response("CustomMethod", "CustomResponse");

        assert!(response.response_json.contains("Mock response for CustomMethod"));
        assert!(response.response_json.contains("CustomResponse"));
    }

    // ==================== Enhanced Mock Response Tests ====================

    #[test]
    fn test_generate_enhanced_mock_response_hello() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "SayHello",
            "HelloResponse",
            "TestService",
            &smart_generator,
        );

        assert!(response.response_json.contains("message"));
        assert!(!response.simulate_error);
    }

    #[test]
    fn test_generate_enhanced_mock_response_list() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "ListUsers",
            "ListUsersResponse",
            "UserService",
            &smart_generator,
        );

        assert!(response.response_json.contains("id") || response.response_json.contains("data"));
    }

    #[test]
    fn test_generate_enhanced_mock_response_create() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "CreateUser",
            "CreateUserResponse",
            "UserService",
            &smart_generator,
        );

        assert!(
            response.response_json.contains("status") || response.response_json.contains("message")
        );
    }

    #[test]
    fn test_generate_enhanced_mock_response_update() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "UpdateUser",
            "UpdateUserResponse",
            "UserService",
            &smart_generator,
        );

        assert!(
            response.response_json.contains("status")
                || response.response_json.contains("version")
                || response.response_json.contains("updated")
        );
    }

    #[test]
    fn test_generate_enhanced_mock_response_delete() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "DeleteUser",
            "DeleteUserResponse",
            "UserService",
            &smart_generator,
        );

        assert!(
            response.response_json.contains("deleted")
                || response.response_json.contains("message")
        );
    }

    #[test]
    fn test_generate_enhanced_mock_response_generic() {
        let smart_generator =
            Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig::default())));

        let response = DynamicGrpcService::generate_enhanced_mock_response(
            "ProcessData",
            "ProcessDataResponse",
            "DataService",
            &smart_generator,
        );

        assert!(
            response.response_json.contains("result")
                || response.response_json.contains("status")
                || response.response_json.contains("message")
        );
    }

    // ==================== Set Mock Response Tests ====================

    #[test]
    fn test_set_mock_response() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        let custom_response = MockResponse {
            response_json: r#"{"custom": "response"}"#.to_string(),
            simulate_error: false,
            error_message: None,
            error_code: None,
        };

        service.set_mock_response("SayHello", custom_response.clone());

        let stored = service.mock_responses.get("SayHello").unwrap();
        assert_eq!(stored.response_json, r#"{"custom": "response"}"#);
    }

    #[test]
    fn test_set_mock_response_new_method() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        let custom_response = MockResponse {
            response_json: r#"{"new": "method"}"#.to_string(),
            simulate_error: false,
            error_message: None,
            error_code: None,
        };

        service.set_mock_response("NewMethod", custom_response);

        assert!(service.mock_responses.contains_key("NewMethod"));
    }

    // ==================== Error Simulation Tests ====================

    #[test]
    fn test_set_error_simulation() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        service.set_error_simulation("SayHello", "Test error message", 3);

        let response = service.mock_responses.get("SayHello").unwrap();
        assert!(response.simulate_error);
        assert_eq!(response.error_message, Some("Test error message".to_string()));
        assert_eq!(response.error_code, Some(3));
    }

    #[test]
    fn test_set_error_simulation_nonexistent_method() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        // Should not panic, just do nothing
        service.set_error_simulation("NonExistent", "Error", 5);

        assert!(!service.mock_responses.contains_key("NonExistent"));
    }

    // ==================== Output Type Tests ====================

    #[test]
    fn test_get_output_type_existing_method() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let output_type = service.get_output_type("SayHello");
        assert_eq!(output_type, "HelloResponse");
    }

    #[test]
    fn test_get_output_type_nonexistent_method() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let output_type = service.get_output_type("NonExistent");
        assert_eq!(output_type, "google.protobuf.Any");
    }

    // ==================== Stream Response Message Tests ====================

    #[test]
    fn test_create_stream_response_message_json() {
        let base_response = r#"{"message": "test"}"#;
        let response = DynamicGrpcService::create_stream_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            0,
            3,
        );

        assert!(response.type_url.contains("TestOutput"));
        let value_str = String::from_utf8(response.value.clone()).unwrap();
        assert!(value_str.contains("stream_index"));
        assert!(value_str.contains("stream_total"));
        assert!(value_str.contains("is_final"));
    }

    #[test]
    fn test_create_stream_response_message_first() {
        let base_response = r#"{"data": "value"}"#;
        let response = DynamicGrpcService::create_stream_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            0,
            5,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains(r#""stream_index": 0"#));
        assert!(value_str.contains(r#""is_final": false"#));
    }

    #[test]
    fn test_create_stream_response_message_last() {
        let base_response = r#"{"data": "value"}"#;
        let response = DynamicGrpcService::create_stream_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            4,
            5,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains(r#""stream_index": 4"#));
        assert!(value_str.contains(r#""is_final": true"#));
    }

    #[test]
    fn test_create_stream_response_message_non_json() {
        let base_response = "simple string";
        let response = DynamicGrpcService::create_stream_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            1,
            3,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains("simple string"));
        assert!(value_str.contains("stream_index"));
        assert!(value_str.contains("method"));
    }

    // ==================== Bidirectional Response Message Tests ====================

    #[test]
    fn test_create_bidirectional_response_message_json() {
        let base_response = r#"{"message": "hello"}"#;
        let input_message = Any {
            type_url: "type.googleapis.com/test".to_string(),
            value: b"input data".to_vec(),
        };

        let response = DynamicGrpcService::create_bidirectional_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            &input_message,
            1,
            1,
            0,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains("input_sequence"));
        assert!(value_str.contains("output_sequence"));
        assert!(value_str.contains("input_context"));
    }

    #[test]
    fn test_create_bidirectional_response_message_with_binary_input() {
        let base_response = r#"{"data": "test"}"#;
        let input_message = Any {
            type_url: "type.googleapis.com/test".to_string(),
            value: vec![0xFF, 0xFE, 0x00, 0x01], // Invalid UTF-8
        };

        let response = DynamicGrpcService::create_bidirectional_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            &input_message,
            2,
            3,
            1,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains("Binary input"));
    }

    #[test]
    fn test_create_bidirectional_response_message_large_input() {
        let base_response = r#"{"data": "test"}"#;
        let large_input = "x".repeat(300); // Larger than 200 char limit
        let input_message = Any {
            type_url: "type.googleapis.com/test".to_string(),
            value: large_input.as_bytes().to_vec(),
        };

        let response = DynamicGrpcService::create_bidirectional_response_message(
            "TestMethod",
            "TestOutput",
            base_response,
            &input_message,
            1,
            1,
            0,
        );

        let value_str = String::from_utf8(response.value).unwrap();
        assert!(value_str.contains("Large input"));
    }

    // ==================== Enhanced Service Tests ====================

    #[test]
    fn test_dynamic_grpc_service_new_enhanced() {
        let proto_service = create_test_proto_service();
        let smart_config = SmartMockConfig::default();

        let service = DynamicGrpcService::new_enhanced(proto_service, None, None, smart_config);

        assert_eq!(service.service_name(), "test.package.TestService");
        assert!(service.proto_parser.is_none());
    }

    #[test]
    fn test_smart_generator_accessor() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let generator = service.smart_generator();
        assert!(generator.lock().is_ok());
    }

    #[test]
    fn test_descriptor_pool_none() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        assert!(service.descriptor_pool().is_none());
    }

    // ==================== EnhancedServiceFactory Tests ====================

    #[test]
    fn test_create_service_from_proto_basic() {
        let proto_service = create_test_proto_service();
        let smart_config = SmartMockConfig::default();

        let service = EnhancedServiceFactory::create_service_from_proto(
            proto_service,
            None,
            None,
            smart_config,
        );

        assert_eq!(service.service_name(), "test.package.TestService");
        assert!(service.proto_parser.is_none());
    }

    #[test]
    fn test_create_service_from_proto_with_latency() {
        use mockforge_core::latency::{FaultConfig, LatencyProfile};

        let proto_service = create_test_proto_service();
        let latency_injector =
            LatencyInjector::new(LatencyProfile::default(), FaultConfig::default());
        let smart_config = SmartMockConfig::default();

        let service = EnhancedServiceFactory::create_service_from_proto(
            proto_service,
            Some(latency_injector),
            None,
            smart_config,
        );

        assert!(service.latency_injector.is_some());
    }

    // ==================== Streaming Service Tests ====================

    #[test]
    fn test_streaming_service_methods() {
        let proto_service = create_streaming_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let methods = service.methods();
        assert_eq!(methods.len(), 3);

        assert!(!methods[0].client_streaming && methods[0].server_streaming); // Server stream
        assert!(methods[1].client_streaming && !methods[1].server_streaming); // Client stream
        assert!(methods[2].client_streaming && methods[2].server_streaming); // BiDi
    }

    #[test]
    fn test_mock_responses_generated_for_all_methods() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        // All methods should have mock responses
        assert!(service.mock_responses.contains_key("SayHello"));
        assert!(service.mock_responses.contains_key("GetUser"));
        assert!(service.mock_responses.contains_key("CreateItem"));
    }

    // ==================== Async Handler Tests ====================

    #[tokio::test]
    async fn test_handle_unary_success() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/HelloRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_unary("SayHello", request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert!(response.type_url.contains("HelloResponse"));
    }

    #[tokio::test]
    async fn test_handle_unary_method_not_found() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/UnknownRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_unary("UnknownMethod", request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_handle_unary_with_error_simulation() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        service.set_error_simulation("SayHello", "Simulated error", 3);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/HelloRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_unary("SayHello", request).await;
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument); // Code 3
    }

    #[tokio::test]
    async fn test_handle_server_streaming_success() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/HelloRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_server_streaming("SayHello", request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_server_streaming_method_not_found() {
        let proto_service = create_test_proto_service();
        let service = DynamicGrpcService::new(proto_service, None);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/UnknownRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_server_streaming("UnknownMethod", request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_server_streaming_with_error_simulation() {
        let proto_service = create_test_proto_service();
        let mut service = DynamicGrpcService::new(proto_service, None);

        service.set_error_simulation("SayHello", "Stream error", 13);

        let request = Request::new(Any {
            type_url: "type.googleapis.com/HelloRequest".to_string(),
            value: b"{}".to_vec(),
        });

        let result = service.handle_server_streaming("SayHello", request).await;
        assert!(result.is_err());
    }
}

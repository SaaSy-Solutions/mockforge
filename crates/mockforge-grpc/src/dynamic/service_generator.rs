//! Dynamic gRPC service generation
//!
//! This module generates actual gRPC service implementations from parsed proto definitions.

use crate::dynamic::proto_parser::ProtoService;
use mockforge_core::latency::LatencyInjector;
use prost_types::Any;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, warn};

/// A dynamically generated gRPC service
pub struct DynamicGrpcService {
    /// The service definition
    service: ProtoService,
    /// Latency injector for simulating delays
    latency_injector: Option<LatencyInjector>,
    /// Mock responses for each method
    mock_responses: HashMap<String, MockResponse>,
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
    pub fn new(
        service: ProtoService,
        latency_injector: Option<LatencyInjector>,
    ) -> Self {
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
        }
    }

    /// Generate a mock response for a method
    fn generate_mock_response(method_name: &str, output_type: &str) -> MockResponse {
        // Generate different responses based on method name
        let response_json = match method_name {
            "SayHello" | "SayHelloStream" | "SayHelloClientStream" | "Chat" => {
                r#"{"message": "Hello from MockForge!"}"#.to_string()
            },
            _ => {
                // Generic response for unknown methods
                format!(r#"{{"result": "Mock response for {}", "type": "{}"}}"#, method_name, output_type)
            }
        };

        MockResponse {
            response_json,
            simulate_error: false,
            error_message: None,
            error_code: None,
        }
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
        let mock_response = self.mock_responses.get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response.error_message.as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(
                tonic::Code::from_i32(error_code),
                error_message,
            ));
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
        let mock_response = self.mock_responses.get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response.error_message.as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(
                tonic::Code::from_i32(error_code),
                error_message,
            ));
        }

        // Create a streaming response
        let stream = self.create_server_stream(method_name, &request.into_inner(), mock_response).await?;
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
                let stream_response = Self::create_stream_response_message(&method_name, &output_type, &response_json, i, message_count);
                
                if tx.send(Ok(stream_response)).await.is_err() {
                    debug!("Stream receiver dropped for method: {}", method_name);
                    break; // Receiver dropped
                }

                // Add delay between messages to simulate realistic streaming
                let delay = Duration::from_millis(100 + (i as u64 * 50)); // Progressive delay
                tokio::time::sleep(delay).await;
            }

            info!("Completed server streaming for method: {} with {} messages", method_name, message_count);
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
        let mock_response = self.mock_responses.get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response.error_message.as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(
                tonic::Code::from_i32(error_code),
                error_message,
            ));
        }

        // Create response based on collected messages
        let response = Any {
            type_url: format!("type.googleapis.com/{}", self.get_output_type(method_name)),
            value: format!(r#"{{"message": "Processed {} messages from MockForge!"}}"#, messages.len()).as_bytes().to_vec(),
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
        let mock_response = self.mock_responses.get(method_name)
            .ok_or_else(|| Status::not_found(format!("Method {} not found", method_name)))?;

        // Check if we should simulate an error
        if mock_response.simulate_error {
            let error_code = mock_response.error_code.unwrap_or(2); // UNKNOWN
            let error_message = mock_response.error_message.as_deref()
                .unwrap_or("Simulated error from MockForge");
            return Err(Status::new(
                tonic::Code::from_i32(error_code),
                error_message,
            ));
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
                debug!("Received bidirectional input message {} for method: {}", input_count, method_name);

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
                    warn!("Reached maximum input message limit (100) for bidirectional method: {}", method_name);
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
            if input_str.len() < 200 { // Reasonable length limit
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
        self.service.methods.iter()
            .find(|m| m.name == method_name)
            .map(|m| m.output_type.clone())
            .unwrap_or_else(|| "google.protobuf.Any".to_string())
    }

    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service.name
    }

    /// Get the service definition
    pub fn service(&self) -> &ProtoService {
        &self.service
    }

    /// Set a custom mock response for a method
    pub fn set_mock_response(&mut self, method_name: &str, response: MockResponse) {
        self.mock_responses.insert(method_name.to_string(), response);
    }

    /// Set error simulation for a method
    pub fn set_error_simulation(&mut self, method_name: &str, error_message: &str, error_code: i32) {
        if let Some(mock_response) = self.mock_responses.get_mut(method_name) {
            mock_response.simulate_error = true;
            mock_response.error_message = Some(error_message.to_string());
            mock_response.error_code = Some(error_code);
        }
    }
}

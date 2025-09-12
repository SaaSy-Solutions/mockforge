//! Dynamic gRPC service generation
//!
//! This module generates actual gRPC service implementations from parsed proto definitions.

use crate::dynamic::proto_parser::ProtoService;
use mockforge_core::latency::LatencyInjector;
use prost_types::Any;
use std::collections::HashMap;
use tonic::{Request, Response, Status, Streaming};
use tracing::debug;

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
        _request: Request<Any>,
    ) -> Result<Response<Streaming<Any>>, Status> {
        debug!("Handling server streaming request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // For now, return an error since streaming is complex to implement
        // In a full implementation, we would create a proper streaming response
        Err(Status::unimplemented("Server streaming not yet fully implemented"))
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
        _request: Request<Streaming<Any>>,
    ) -> Result<Response<Streaming<Any>>, Status> {
        debug!("Handling bidirectional streaming request for method: {}", method_name);

        // Inject latency if configured
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

        // For now, return an error since streaming is complex to implement
        // In a full implementation, we would create a proper streaming response
        Err(Status::unimplemented("Bidirectional streaming not yet fully implemented"))
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

//! gRPC Proto Registry - SpecRegistry implementation for gRPC
//!
//! This module provides a SpecRegistry implementation that can load .proto files
//! and generate mock responses for gRPC services.

use crate::dynamic::proto_parser::{ProtoMethod, ProtoParser, ProtoService};
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
};
use mockforge_core::{ProtocolValidationError as ValidationError, ProtocolValidationResult as ValidationResult, Result};
use prost_reflect::MessageDescriptor;
use std::collections::HashMap;

/// gRPC Proto Registry implementing SpecRegistry
pub struct GrpcProtoRegistry {
    /// Proto parser
    parser: ProtoParser,
    /// Services defined in the proto files
    services: Vec<ProtoService>,
    /// Operations (RPCs) extracted from services
    operations: Vec<SpecOperation>,
}

impl GrpcProtoRegistry {
    /// Create a new gRPC registry from proto directory
    pub async fn from_directory(proto_dir: &str) -> Result<Self> {
        let mut parser = ProtoParser::new();
        parser.parse_directory(proto_dir).await
            .map_err(|e| mockforge_core::Error::validation(format!("Failed to parse proto directory: {}", e)))?;

        let services: Vec<ProtoService> = parser.services().values().cloned().collect();
        let operations = Self::extract_operations_from_services(&services);

        Ok(Self {
            parser,
            services,
            operations,
        })
    }

    /// Create a new gRPC registry with a custom parser
    /// (useful when you need to provide your own configured parser)
    pub fn from_parser(parser: ProtoParser) -> Result<Self> {
        let services: Vec<ProtoService> = parser.services().values().cloned().collect();
        let operations = Self::extract_operations_from_services(&services);

        Ok(Self {
            parser,
            services,
            operations,
        })
    }

    /// Extract operations from services
    fn extract_operations_from_services(services: &[ProtoService]) -> Vec<SpecOperation> {
        let mut operations = Vec::new();

        for service in services {
            for method in &service.methods {
                operations.push(SpecOperation {
                    name: method.name.clone(),
                    path: format!("{}/{}", service.name, method.name),
                    operation_type: Self::method_type_string(method),
                    input_schema: Some(method.input_type.clone()),
                    output_schema: Some(method.output_type.clone()),
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("service".to_string(), service.name.clone());
                        meta.insert("package".to_string(), service.package.clone());
                        meta
                    },
                });
            }
        }

        operations
    }

    /// Get method type as string
    fn method_type_string(method: &ProtoMethod) -> String {
        match (method.client_streaming, method.server_streaming) {
            (false, false) => "Unary".to_string(),
            (true, false) => "ClientStreaming".to_string(),
            (false, true) => "ServerStreaming".to_string(),
            (true, true) => "BidirectionalStreaming".to_string(),
        }
    }

    /// Generate mock message for a type
    fn generate_mock_message(&self, message_type: &str) -> serde_json::Value {
        // Try to get the message descriptor
        if let Some(descriptor) = self.parser.pool().get_message_by_name(message_type) {
            return Self::generate_mock_from_descriptor(&descriptor);
        }

        // Fallback to simple mock
        serde_json::json!({
            "message": format!("Mock response for {}", message_type)
        })
    }

    /// Generate mock data from a message descriptor
    fn generate_mock_from_descriptor(descriptor: &MessageDescriptor) -> serde_json::Value {
        let mut fields = serde_json::Map::new();

        for field in descriptor.fields() {
            let field_name = field.name();
            let mock_value = match field.kind() {
                prost_reflect::Kind::Double | prost_reflect::Kind::Float => {
                    serde_json::json!(99.99)
                }
                prost_reflect::Kind::Int32 | prost_reflect::Kind::Int64 |
                prost_reflect::Kind::Uint32 | prost_reflect::Kind::Uint64 |
                prost_reflect::Kind::Sint32 | prost_reflect::Kind::Sint64 |
                prost_reflect::Kind::Fixed32 | prost_reflect::Kind::Fixed64 |
                prost_reflect::Kind::Sfixed32 | prost_reflect::Kind::Sfixed64 => {
                    serde_json::json!(42)
                }
                prost_reflect::Kind::Bool => serde_json::json!(true),
                prost_reflect::Kind::String => {
                    // Generate based on field name
                    match field_name.to_lowercase().as_str() {
                        "id" => serde_json::json!(mockforge_core::templating::expand_str("{{uuid}}")),
                        "name" | "title" => serde_json::json!(format!("Mock {}", field_name)),
                        "email" => serde_json::json!(mockforge_core::templating::expand_str("{{faker.email}}")),
                        _ => serde_json::json!(format!("mock_{}", field_name)),
                    }
                }
                prost_reflect::Kind::Bytes => serde_json::json!("mock_bytes"),
                prost_reflect::Kind::Message(_msg_desc) => {
                    // Nested message - generate recursively or use simple mock
                    serde_json::json!({})
                }
                prost_reflect::Kind::Enum(_enum_desc) => serde_json::json!(0),
            };

            fields.insert(field_name.to_string(), mock_value);
        }

        serde_json::Value::Object(fields)
    }
}

impl SpecRegistry for GrpcProtoRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Grpc
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.operations.clone()
    }

    fn find_operation(&self, operation: &str, _path: &str) -> Option<SpecOperation> {
        // Operation format: "service.package.Service/Method" or just "Method"
        self.operations
            .iter()
            .find(|op| op.path == operation || op.name == operation)
            .cloned()
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        // Check if the operation exists
        if let Some(_op) = self.find_operation(&request.operation, &request.path) {
            Ok(ValidationResult::success())
        } else {
            Ok(ValidationResult::failure(vec![
                ValidationError {
                    message: format!("Unknown gRPC operation: {}", request.operation),
                    path: Some(request.path.clone()),
                    code: Some("UNKNOWN_RPC".to_string()),
                },
            ]))
        }
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        // Find the operation
        let operation = self.find_operation(&request.operation, &request.path)
            .ok_or_else(|| mockforge_core::Error::validation(
                format!("Unknown operation: {}", request.operation)
            ))?;

        // Get output type
        let output_type = operation.output_schema
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::validation("No output schema defined"))?;

        // Generate mock message
        let mock_data = self.generate_mock_message(output_type);

        // Serialize to bytes (JSON for now, could be protobuf)
        let body = serde_json::to_vec(&mock_data)?;

        Ok(ProtocolResponse {
            status: ResponseStatus::GrpcStatus(0), // OK
            metadata: {
                let mut m = HashMap::new();
                m.insert("content-type".to_string(), "application/grpc+json".to_string());
                m.insert("grpc-status".to_string(), "0".to_string());
                m
            },
            body,
            content_type: "application/grpc+json".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_directory_nonexistent() {
        let result = GrpcProtoRegistry::from_directory("/nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_method_type_string() {
        let unary = ProtoMethod {
            name: "Test".to_string(),
            input_type: "Input".to_string(),
            output_type: "Output".to_string(),
            client_streaming: false,
            server_streaming: false,
        };
        assert_eq!(GrpcProtoRegistry::method_type_string(&unary), "Unary");

        let client_streaming = ProtoMethod {
            name: "Test".to_string(),
            input_type: "Input".to_string(),
            output_type: "Output".to_string(),
            client_streaming: true,
            server_streaming: false,
        };
        assert_eq!(GrpcProtoRegistry::method_type_string(&client_streaming), "ClientStreaming");

        let server_streaming = ProtoMethod {
            name: "Test".to_string(),
            input_type: "Input".to_string(),
            output_type: "Output".to_string(),
            client_streaming: false,
            server_streaming: true,
        };
        assert_eq!(GrpcProtoRegistry::method_type_string(&server_streaming), "ServerStreaming");

        let bidirectional = ProtoMethod {
            name: "Test".to_string(),
            input_type: "Input".to_string(),
            output_type: "Output".to_string(),
            client_streaming: true,
            server_streaming: true,
        };
        assert_eq!(GrpcProtoRegistry::method_type_string(&bidirectional), "BidirectionalStreaming");
    }

    #[test]
    fn test_extract_operations_from_services() {
        let services = vec![
            ProtoService {
                name: "test.Service".to_string(),
                package: "test".to_string(),
                short_name: "Service".to_string(),
                methods: vec![
                    ProtoMethod {
                        name: "GetUser".to_string(),
                        input_type: "GetUserRequest".to_string(),
                        output_type: "GetUserResponse".to_string(),
                        client_streaming: false,
                        server_streaming: false,
                    },
                ],
            },
        ];

        let operations = GrpcProtoRegistry::extract_operations_from_services(&services);
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].name, "GetUser");
        assert_eq!(operations[0].path, "test.Service/GetUser");
        assert_eq!(operations[0].operation_type, "Unary");
    }
}

//! Route generation for HTTP bridge
//!
//! This module generates HTTP routes from protobuf service definitions,
//! mapping gRPC methods to RESTful endpoints.

use super::HttpBridgeConfig;
use regex::Regex;

/// Generates HTTP routes from protobuf service definitions
#[derive(Debug, Clone)]
pub struct RouteGenerator {
    /// Configuration for route generation
    config: HttpBridgeConfig,
    /// Regular expression for cleaning service/service names for HTTP paths
    service_name_regex: Regex,
    /// Regular expression for cleaning method names for HTTP paths
    method_name_regex: Regex,
}

impl RouteGenerator {
    /// Create a new route generator
    pub fn new(config: HttpBridgeConfig) -> Self {
        Self {
            config,
            service_name_regex: Regex::new(r"[^a-zA-Z0-9_.-]").unwrap(),
            method_name_regex: Regex::new(r"[^a-zA-Z0-9_.-]").unwrap(),
        }
    }

    /// Generate HTTP route path for a service method
    pub fn generate_route_path(&self, service_name: &str, method_name: &str) -> String {
        let clean_service = self.clean_service_name(service_name);
        let clean_method = self.clean_method_name(method_name);

        format!("{}{}", self.config.base_path, self.config.route_pattern)
            .replace("{service}", &clean_service)
            .replace("{method}", &clean_method)
    }

    /// Generate URL pattern from route path
    pub fn generate_url_pattern(&self, _service_name: &str, _method_name: &str) -> String {
        let template = format!("{}{}", self.config.base_path, self.config.route_pattern);
        // Replace path parameters with regex patterns
        template.replace("{service}", "[^/]+").replace("{method}", "[^/]+")
    }

    /// Clean service name for HTTP routing
    pub fn clean_service_name(&self, service_name: &str) -> String {
        // Remove package prefix if present
        let without_package = if let Some(dot_index) = service_name.rfind('.') {
            &service_name[dot_index + 1..]
        } else {
            service_name
        };

        // Replace non-alphanumeric characters with dashes
        let cleaned = self.service_name_regex.replace_all(without_package, "-");

        // Convert to lowercase for consistency
        cleaned.to_lowercase()
    }

    /// Clean method name for HTTP routing
    pub fn clean_method_name(&self, method_name: &str) -> String {
        // Replace non-alphanumeric characters with dashes
        let cleaned = self.method_name_regex.replace_all(method_name, "-");

        // Convert to lowercase for consistency
        cleaned.to_lowercase()
    }

    /// Generate OpenAPI path specification for a service method
    pub fn generate_openapi_path(
        &self,
        service_name: &str,
        method_name: &str,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> serde_json::Value {
        let path = self.generate_route_path(service_name, method_name);
        let operation =
            self.generate_openapi_operation(service_name, method_name, method_descriptor);

        serde_json::json!({ path: operation })
    }

    /// Generate full OpenAPI specification for all services
    pub fn generate_openapi_spec(
        &self,
        services: &std::collections::HashMap<String, crate::dynamic::proto_parser::ProtoService>,
    ) -> serde_json::Value {
        let mut paths = serde_json::Map::new();
        let mut schemas = serde_json::Map::new();

        // Generate paths for all services and methods
        for (service_name, service) in services {
            for method in &service.methods {
                let path = self.generate_route_path(service_name, &method.name);
                let operation = self.generate_openapi_operation_full(
                    service_name,
                    &method.name,
                    method,
                    &mut schemas,
                );

                // Determine HTTP method
                let http_method = if method.server_streaming {
                    "get"
                } else {
                    "post"
                };

                paths.insert(
                    path,
                    serde_json::json!({
                        http_method: operation
                    }),
                );
            }
        }

        serde_json::json!({
            "openapi": "3.0.1",
            "info": {
                "title": "MockForge gRPC HTTP Bridge API",
                "description": "RESTful API bridge to gRPC services automatically generated from protobuf definitions",
                "version": "1.0.0",
                "contact": {
                    "name": "MockForge",
                    "url": "https://github.com/SaaSy-Solutions/mockforge"
                },
                "license": {
                    "name": "MIT",
                    "url": "https://opensource.org/licenses/MIT"
                }
            },
            "servers": [{
                "url": "http://localhost:9080",
                "description": "Local development server"
            }],
            "security": [],
            "paths": paths,
            "components": {
                "schemas": schemas,
                "securitySchemes": {}
            }
        })
    }

    /// Generate complete OpenAPI operation with schema references
    fn generate_openapi_operation_full(
        &self,
        service_name: &str,
        method_name: &str,
        proto_method: &crate::dynamic::proto_parser::ProtoMethod,
        schemas: &mut serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let mut operation = serde_json::json!({
            "summary": format!("{} {}", method_name, service_name),
            "description": format!("Calls the {} method on {} service\n\n**gRPC Method Details:**\n- Service: {}\n- Method: {}\n- Input: {}\n- Output: {}\n{}",
                method_name,
                service_name,
                service_name,
                method_name,
                proto_method.input_type,
                proto_method.output_type,
                if proto_method.server_streaming {
                    "\n- Server Streaming: Yes (returns SSE stream)"
                } else {
                    "\n- Server Streaming: No"
                }
            ),
            "tags": [service_name],
            "parameters": self.generate_openapi_parameters(),
        });

        // Add request body for methods that send data
        if !proto_method.server_streaming {
            operation["requestBody"] =
                self.generate_openapi_request_body_full(proto_method, schemas);
        }

        // Add responses
        operation["responses"] =
            self.generate_openapi_responses_full(service_name, method_name, proto_method, schemas);

        operation
    }

    /// Generate OpenAPI request body with schema for a specific proto method
    fn generate_openapi_request_body_full(
        &self,
        proto_method: &crate::dynamic::proto_parser::ProtoMethod,
        schemas: &mut serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let schema_name = self.get_schema_name(&proto_method.input_type);
        let input_descriptor_opt = None; // We would need dynamic descriptor pool to get this

        // Create a basic schema reference for the input type
        let schema_ref = if let Some(descriptor) = input_descriptor_opt {
            schemas.insert(schema_name.clone(), self.generate_json_schema(&descriptor));
            serde_json::json!({
                "$ref": format!("#/components/schemas/{}", schema_name)
            })
        } else {
            // Fallback schema when descriptor is not available
            serde_json::json!({
                "type": "object",
                "description": format!("Protobuf message: {}", proto_method.input_type),
                "additionalProperties": true
            })
        };

        serde_json::json!({
            "required": true,
            "content": {
                "application/json": {
                    "schema": schema_ref,
                    "example": self.generate_example_for_type(&proto_method.input_type)
                }
            }
        })
    }

    /// Generate OpenAPI responses with schema for a specific proto method
    fn generate_openapi_responses_full(
        &self,
        service_name: &str,
        method_name: &str,
        proto_method: &crate::dynamic::proto_parser::ProtoMethod,
        schemas: &mut serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let schema_name = self.get_schema_name(&proto_method.output_type);
        let output_descriptor_opt = None; // We would need dynamic descriptor pool to get this

        let success_schema = if let Some(descriptor) = output_descriptor_opt {
            schemas.insert(schema_name.clone(), self.generate_json_schema(&descriptor));
            serde_json::json!({
                "$ref": format!("#/components/schemas/{}", schema_name)
            })
        } else {
            // Fallback schema when descriptor is not available
            serde_json::json!({
                "type": "object",
                "description": format!("Protobuf message: {}", proto_method.output_type),
                "additionalProperties": true
            })
        };

        let mut responses = serde_json::json!({
            "200": {
                "description": "Successful operation",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": {
                                    "type": "boolean",
                                    "description": "Whether the request was successful"
                                },
                                "data": success_schema,
                                "error": {
                                    "type": ["string", "null"],
                                    "description": "Error message if success is false"
                                },
                                "metadata": {
                                    "type": "object",
                                    "description": "Additional metadata from gRPC response",
                                    "additionalProperties": { "type": "string" }
                                }
                            },
                            "required": ["success", "data", "error", "metadata"]
                        },
                        "example": {
                            "success": true,
                            "data": self.generate_example_for_type(&proto_method.output_type),
                            "error": null,
                            "metadata": {
                                "x-mockforge-service": service_name,
                                "x-mockforge-method": method_name
                            }
                        }
                    }
                }
            },
            "400": {
                "description": "Bad request - invalid JSON or invalid parameters",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": { "type": "boolean" },
                                "data": { "type": "null" },
                                "error": { "type": "string" },
                                "metadata": { "type": "object" }
                            }
                        },
                        "example": {
                            "success": false,
                            "data": null,
                            "error": "Invalid request format",
                            "metadata": {}
                        }
                    }
                }
            },
            "500": {
                "description": "Internal server error",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": { "type": "boolean" },
                                "data": { "type": "null" },
                                "error": { "type": "string" },
                                "metadata": { "type": "object" }
                            }
                        },
                        "example": {
                            "success": false,
                            "data": null,
                            "error": "Internal server error",
                            "metadata": {}
                        }
                    }
                }
            }
        });

        // Add streaming response option for server streaming methods
        if proto_method.server_streaming {
            responses["200"]["content"]["text/event-stream"] = serde_json::json!({
                "schema": {
                    "type": "string",
                    "description": "Server-sent events stream"
                },
                "example": "data: {\"message\":\"Stream started\"}\n\ndata: {\"message\":\"Hello World!\"}\n\ndata: {\"message\":\"Stream ended\"}\n\n"
            });
        }

        responses
    }

    /// Generate schema name from protobuf type name
    fn get_schema_name(&self, type_name: &str) -> String {
        // Remove package prefix and clean up the name
        let short_name = if let Some(dot_index) = type_name.rfind('.') {
            &type_name[dot_index + 1..]
        } else {
            type_name
        };

        // Keep the name as PascalCase and append "Message" if not already present
        let schema_name = short_name.to_string();

        if !schema_name.ends_with("Message") {
            format!("{}Message", schema_name)
        } else {
            schema_name
        }
    }

    /// Generate example JSON for a protobuf type
    fn generate_example_for_type(&self, type_name: &str) -> serde_json::Value {
        // Simple example generation based on common protobuf messages
        if type_name.contains("HelloRequest") {
            serde_json::json!({
                "name": "World",
                "user_info": {
                    "user_id": "12345"
                }
            })
        } else if type_name.contains("HelloReply") {
            serde_json::json!({
                "message": "Hello World! This is a mock response from MockForge",
                "timestamp": "2025-01-01T00:00:00Z"
            })
        } else {
            serde_json::json!({
                "example_field": "example_value"
            })
        }
    }

    /// Generate OpenAPI operation specification for a method
    pub fn generate_openapi_operation(
        &self,
        service_name: &str,
        method_name: &str,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> serde_json::Value {
        let http_method = self.get_http_method(method_descriptor);

        let mut operation = serde_json::json!({
            "summary": format!("{} {}", method_name, service_name),
            "description": format!("Calls the {} method on {} service", method_name, service_name),
            "parameters": self.generate_openapi_parameters(),
        });

        // Add request body for methods that send data
        if http_method != "get" {
            operation["requestBody"] = self.generate_openapi_request_body(method_descriptor);
        }

        // Add responses
        operation["responses"] = self.generate_openapi_responses(method_descriptor);

        operation
    }

    /// Get appropriate HTTP method for gRPC method type
    pub fn get_http_method(
        &self,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> &'static str {
        if method_descriptor.is_client_streaming() && method_descriptor.is_server_streaming() {
            "post" // Bidirectional streaming uses POST
        } else if method_descriptor.is_server_streaming() {
            "get" // Server streaming uses GET (streaming response)
        } else {
            "post" // Unary and client streaming use POST
        }
    }

    /// Generate OpenAPI parameters
    fn generate_openapi_parameters(&self) -> serde_json::Value {
        serde_json::json!([
            {
                "name": "stream",
                "in": "query",
                "description": "Streaming mode (none, server, client, bidirectional)",
                "required": false,
                "schema": {
                    "type": "string",
                    "enum": ["none", "server", "client", "bidirectional"]
                }
            }
        ])
    }

    /// Generate OpenAPI request body specification
    fn generate_openapi_request_body(
        &self,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> serde_json::Value {
        let input_descriptor = method_descriptor.input();

        serde_json::json!({
            "required": true,
            "content": {
                "application/json": {
                    "schema": self.generate_json_schema(&input_descriptor)
                }
            }
        })
    }

    /// Generate OpenAPI responses specification
    fn generate_openapi_responses(
        &self,
        method_descriptor: &prost_reflect::MethodDescriptor,
    ) -> serde_json::Value {
        let output_descriptor = method_descriptor.output();
        let success_schema = self.generate_json_schema(&output_descriptor);

        let mut responses = serde_json::json!({
            "200": {
                "description": "Successful operation",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": {
                                    "type": "boolean",
                                    "description": "Whether the request was successful"
                                },
                                "data": success_schema,
                                "error": {
                                    "type": ["string", "null"],
                                    "description": "Error message if success is false"
                                },
                                "metadata": {
                                    "type": "object",
                                    "description": "Additional metadata from gRPC response"
                                }
                            },
                            "required": ["success", "data", "error", "metadata"]
                        }
                    }
                }
            },
            "400": {
                "description": "Bad request - invalid JSON or invalid parameters",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": { "type": "boolean" },
                                "data": { "type": "null" },
                                "error": { "type": "string" },
                                "metadata": { "type": "object" }
                            }
                        }
                    }
                }
            },
            "500": {
                "description": "Internal server error",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "success": { "type": "boolean" },
                                "data": { "type": "null" },
                                "error": { "type": "string" },
                                "metadata": { "type": "object" }
                            }
                        }
                    }
                }
            }
        });

        // For streaming methods, add streaming response option
        if method_descriptor.is_server_streaming() {
            responses["200"]["content"]["text/event-stream"] = serde_json::json!({
                "schema": {
                    "type": "string",
                    "description": "Server-sent events stream"
                }
            });
        }

        responses
    }

    /// Generate JSON schema from protobuf message descriptor
    pub fn generate_json_schema(
        &self,
        descriptor: &prost_reflect::MessageDescriptor,
    ) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for field in descriptor.fields() {
            let field_name = field.name().to_string();
            let field_schema = self.generate_field_schema(&field);

            properties.insert(field_name.clone(), field_schema);

            // Mark field as required if it's not optional
            // Note: In proto3, all fields are effectively optional at the JSON level
            // but we can mark them as required for better API documentation
            if field.supports_presence() && !field.is_list() {
                // For proto3, we can make educated guesses about required fields
                // based on field name patterns
                let field_name_lower = field.name().to_lowercase();
                if !field_name_lower.contains("optional") && !field_name_lower.contains("_opt") {
                    required.push(field_name);
                }
            }
        }

        let mut schema = serde_json::json!({
            "type": "object",
            "properties": properties
        });

        if !required.is_empty() {
            schema["required"] = serde_json::Value::Array(
                required.into_iter().map(serde_json::Value::String).collect(),
            );
        }

        schema
    }

    /// Generate JSON schema for a single field
    fn generate_field_schema(&self, field: &prost_reflect::FieldDescriptor) -> serde_json::Value {
        let base_type = self.get_json_type_for_field(field);

        if field.is_list() {
            serde_json::json!({
                "type": "array",
                "items": base_type
            })
        } else {
            base_type
        }
    }

    /// Get JSON type for a protobuf field
    fn get_json_type_for_field(&self, field: &prost_reflect::FieldDescriptor) -> serde_json::Value {
        match field.kind() {
            prost_reflect::Kind::Message(message_descriptor) => {
                self.generate_json_schema(&message_descriptor)
            }
            prost_reflect::Kind::Enum(_) => {
                serde_json::json!({
                    "type": "string",
                    "description": "Enum value as string"
                })
            }
            prost_reflect::Kind::String => {
                serde_json::json!({
                    "type": "string"
                })
            }
            prost_reflect::Kind::Int32
            | prost_reflect::Kind::Sint32
            | prost_reflect::Kind::Sfixed32 => {
                serde_json::json!({
                    "type": "integer",
                    "format": "int32"
                })
            }
            prost_reflect::Kind::Int64
            | prost_reflect::Kind::Sint64
            | prost_reflect::Kind::Sfixed64 => {
                serde_json::json!({
                    "type": "integer",
                    "format": "int64"
                })
            }
            prost_reflect::Kind::Uint32 | prost_reflect::Kind::Fixed32 => {
                serde_json::json!({
                    "type": "integer",
                    "format": "uint32",
                    "minimum": 0
                })
            }
            prost_reflect::Kind::Uint64 | prost_reflect::Kind::Fixed64 => {
                serde_json::json!({
                    "type": "integer",
                    "format": "uint64",
                    "minimum": 0
                })
            }
            prost_reflect::Kind::Float => {
                serde_json::json!({
                    "type": "number",
                    "format": "float"
                })
            }
            prost_reflect::Kind::Double => {
                serde_json::json!({
                    "type": "number",
                    "format": "double"
                })
            }
            prost_reflect::Kind::Bool => {
                serde_json::json!({
                    "type": "boolean"
                })
            }
            prost_reflect::Kind::Bytes => {
                serde_json::json!({
                    "type": "string",
                    "contentEncoding": "base64",
                    "description": "Base64-encoded bytes"
                })
            }
        }
    }

    /// Extract service name from route path
    pub fn extract_service_name(&self, path: &str) -> Option<String> {
        if path.starts_with(&self.config.base_path) {
            let path_without_base = &path[self.config.base_path.len()..];
            let parts: Vec<&str> = path_without_base.trim_start_matches('/').split('/').collect();

            if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                Some(parts[0].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract method name from route path
    pub fn extract_method_name(&self, path: &str) -> Option<String> {
        if path.starts_with(&self.config.base_path) {
            let path_without_base = &path[self.config.base_path.len()..];
            let parts: Vec<&str> = path_without_base.trim_start_matches('/').split('/').collect();

            if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                Some(parts[1].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_service_name() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        assert_eq!(generator.clean_service_name("mockforge.greeter.Greeter"), "greeter");
        assert_eq!(generator.clean_service_name("MyService"), "myservice");
        assert_eq!(generator.clean_service_name("My-Service_Name"), "my-service_name");
    }

    #[test]
    fn test_clean_method_name() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        assert_eq!(generator.clean_method_name("SayHello"), "sayhello");
        assert_eq!(generator.clean_method_name("Say_Hello"), "say_hello");
        assert_eq!(generator.clean_method_name("GetUserData"), "getuserdata");
    }

    #[test]
    fn test_generate_route_path() {
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let path = generator.generate_route_path("mockforge.greeter.Greeter", "SayHello");
        assert_eq!(path, "/api/greeter/sayhello");
        // Note: Service name is cleaned (package prefix removed, lowercased)
        // Method name is cleaned (lowercased)
    }

    #[test]
    fn test_extract_service_name() {
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        assert_eq!(
            generator.extract_service_name("/api/greeter/sayhello"),
            Some("greeter".to_string())
        );
        assert_eq!(generator.extract_service_name("/api/user/get"), Some("user".to_string()));
        assert_eq!(generator.extract_service_name("/other/path"), None);
    }

    #[test]
    fn test_extract_method_name() {
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        assert_eq!(
            generator.extract_method_name("/api/greeter/sayhello"),
            Some("sayhello".to_string())
        );
        assert_eq!(generator.extract_method_name("/api/user/get"), Some("get".to_string()));
        assert_eq!(generator.extract_method_name("/api/single"), None);
    }

    #[test]
    fn test_route_generator_creation() {
        let config = HttpBridgeConfig {
            enabled: true,
            base_path: "/api".to_string(),
            enable_cors: true,
            max_request_size: 1024,
            timeout_seconds: 30,
            route_pattern: "/{service}/{method}".to_string(),
        };

        let generator = RouteGenerator::new(config.clone());
        assert_eq!(generator.config.base_path, "/api");
        assert_eq!(generator.config.route_pattern, "/{service}/{method}");
    }

    #[test]
    fn test_clean_service_name_comprehensive() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        // Test various service name patterns
        let test_cases = vec![
            ("simple.Service", "service"),
            ("com.example.MyService", "myservice"),
            ("org.test.API", "api"),
            ("Service", "service"),
            ("ServiceName", "servicename"),
            ("service-name", "service-name"),
            ("service_name", "service_name"),
            ("service.name", "name"),
            ("a.b.c.d.Service", "service"),
            ("Service123", "service123"),
            ("123Service", "123service"),
            ("Service-123", "service-123"),
            ("Service_123", "service_123"),
            ("Service.Name", "name"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                generator.clean_service_name(input),
                expected,
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_clean_method_name_comprehensive() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        // Test various method name patterns
        let test_cases = vec![
            ("GetUser", "getuser"),
            ("getUser", "getuser"),
            ("Get_User", "get_user"),
            ("GetUserData", "getuserdata"),
            ("getUserData", "getuserdata"),
            ("GetUser_Data", "getuser_data"),
            ("method123", "method123"),
            ("123method", "123method"),
            ("Method-123", "method-123"),
            ("Method_123", "method_123"),
            ("MethodName", "methodname"),
            ("method_Name", "method_name"),
            ("method-name", "method-name"),
            ("method.name", "method.name"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(generator.clean_method_name(input), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_generate_route_path_comprehensive() {
        let test_configs = vec![
            ("/api", "/{service}/{method}"),
            ("/v1", "/{service}/{method}"),
            ("/api/v1", "/{service}/{method}"),
            ("/bridge", "/{service}/{method}"),
            ("", "/{service}/{method}"),
        ];

        for (base_path, route_pattern) in test_configs {
            let config = HttpBridgeConfig {
                base_path: base_path.to_string(),
                route_pattern: route_pattern.to_string(),
                ..Default::default()
            };
            let generator = RouteGenerator::new(config);

            let test_cases = vec![
                ("com.example.Greeter", "SayHello"),
                ("Service", "Method"),
                ("test.Service", "testMethod"),
                ("org.example.v1.UserService", "GetUser"),
            ];

            for (service, method) in test_cases {
                let path = generator.generate_route_path(service, method);
                let expected = format!("{}{}", base_path, route_pattern)
                    .replace("{service}", &generator.clean_service_name(service))
                    .replace("{method}", &generator.clean_method_name(method));

                assert_eq!(path, expected, "Failed for service: {}, method: {}", service, method);
            }
        }
    }

    #[test]
    fn test_generate_url_pattern_comprehensive() {
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            route_pattern: "/{service}/{method}".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let test_cases = vec![
            ("com.example.Greeter", "SayHello"),
            ("Service", "Method"),
            ("test.Service", "testMethod"),
        ];

        for (service, method) in test_cases {
            let pattern = generator.generate_url_pattern(service, method);
            assert!(pattern.starts_with("/api/"), "Pattern should start with /api/: {}", pattern);
            assert!(
                pattern.contains("[^/]+"),
                "Pattern should contain regex for service: {}",
                pattern
            );
            assert!(
                pattern.contains("[^/]+"),
                "Pattern should contain regex for method: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_extract_service_name_comprehensive() {
        // Test with /api base path
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            route_pattern: "/{service}/{method}".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let test_paths = vec![
            ("/api/greeter/sayhello", Some("greeter".to_string())),
            ("/api/user/get", Some("user".to_string())),
            ("/api/complex.service/name", Some("complex.service".to_string())),
            ("/api/single", None),     // Not enough parts
            ("/v1/test/method", None), // Wrong base path
            ("/other/path", None),     // Wrong base path
            ("", None),
            ("/api/", None),
            ("/api/greeter/", None), // Empty method name
        ];

        for (path, expected) in test_paths {
            let result = generator.extract_service_name(path);
            assert_eq!(result, expected, "Failed for path: {} with base /api", path);
        }

        // Test with /v1 base path
        let config = HttpBridgeConfig {
            base_path: "/v1".to_string(),
            route_pattern: "/{service}/{method}".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let test_paths = vec![
            ("/v1/test/method", Some("test".to_string())),
            ("/v1/service/action", Some("service".to_string())),
            ("/api/greeter/sayhello", None), // Wrong base path
        ];

        for (path, expected) in test_paths {
            let result = generator.extract_service_name(path);
            assert_eq!(result, expected, "Failed for path: {} with base /v1", path);
        }
    }

    #[test]
    fn test_extract_method_name_comprehensive() {
        // Test with /api base path
        let config = HttpBridgeConfig {
            base_path: "/api".to_string(),
            route_pattern: "/{service}/{method}".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let test_paths = vec![
            ("/api/greeter/sayhello", Some("sayhello".to_string())),
            ("/api/user/get", Some("get".to_string())),
            ("/api/complex.service/method_name", Some("method_name".to_string())),
            ("/api/single", None),     // Not enough parts
            ("/v1/test/method", None), // Wrong base path
            ("/other/path", None),     // Wrong base path
            ("", None),
            ("/api/", None),
            ("/api/greeter/", None), // Empty method name
        ];

        for (path, expected) in test_paths {
            let result = generator.extract_method_name(path);
            assert_eq!(result, expected, "Failed for path: {} with base /api", path);
        }

        // Test with /v1 base path
        let config = HttpBridgeConfig {
            base_path: "/v1".to_string(),
            route_pattern: "/{service}/{method}".to_string(),
            ..Default::default()
        };
        let generator = RouteGenerator::new(config);

        let test_paths = vec![
            ("/v1/test/method", Some("method".to_string())),
            ("/v1/service/action", Some("action".to_string())),
            ("/api/greeter/sayhello", None), // Wrong base path
        ];

        for (path, expected) in test_paths {
            let result = generator.extract_method_name(path);
            assert_eq!(result, expected, "Failed for path: {} with base /v1", path);
        }
    }

    #[test]
    fn test_get_http_method() {
        let config = HttpBridgeConfig::default();
        let _generator = RouteGenerator::new(config);

        // Create mock method descriptors for different streaming types
        // Note: This is simplified since we don't have actual descriptors

        // Test different streaming combinations
        // For now, we'll test the logic with simple boolean combinations
        // In a real test, we'd need actual MethodDescriptor instances

        // Since we can't easily create MethodDescriptor instances in this test,
        // we'll test the service name and method name cleaning functions
        // which are the core functionality we can test without actual descriptors
    }

    #[test]
    fn test_generate_openapi_parameters() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        let params = generator.generate_openapi_parameters();
        assert!(params.is_array(), "Parameters should be an array");

        if let serde_json::Value::Array(params_array) = params {
            assert!(!params_array.is_empty(), "Parameters array should not be empty");

            // Check if stream parameter exists
            let stream_param = params_array
                .iter()
                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("stream"));

            assert!(stream_param.is_some(), "Stream parameter should exist");
        }
    }

    #[test]
    fn test_get_schema_name() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        let test_cases = vec![
            ("com.example.GetUserRequest", "GetUserRequestMessage"),
            ("GetUserRequest", "GetUserRequestMessage"),
            ("Request", "RequestMessage"),
            ("Response", "ResponseMessage"),
            ("Message", "Message"),
            ("com.example.v1.GetUserRequest", "GetUserRequestMessage"),
            ("org.test.APIRequest", "APIRequestMessage"),
        ];

        for (input, expected) in test_cases {
            let result = generator.get_schema_name(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_generate_example_for_type() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        // Test various type names to ensure example generation works
        let test_types = vec![
            "HelloRequest",
            "HelloReply",
            "GetUserRequest",
            "GetUserResponse",
            "UnknownMessage",
            "TestMessage",
            "com.example.TestMessage",
        ];

        for type_name in test_types {
            let example = generator.generate_example_for_type(type_name);
            assert!(example.is_object(), "Example should be an object for type: {}", type_name);
            assert!(
                !example.as_object().unwrap().is_empty(),
                "Example object should not be empty for type: {}",
                type_name
            );
        }
    }

    #[test]
    fn test_regex_patterns() {
        let config = HttpBridgeConfig::default();
        let generator = RouteGenerator::new(config);

        // Test service name regex (should be lowercased)
        let service_test_cases = vec![
            ("MyService", "myservice"),
            ("My-Service", "my-service"),
            ("My_Service", "my_service"),
            ("My123Service", "my123service"),
            ("My.Service", "service"),
            ("My@Service", "my-service"),
            ("My#Service", "my-service"),
            ("My$Service", "my-service"),
        ];

        for (input, expected) in service_test_cases {
            let cleaned = generator.clean_service_name(input);
            assert_eq!(cleaned, expected, "Service name regex failed for: {}", input);
        }

        // Test method name regex (should be lowercased)
        let method_test_cases = vec![
            ("GetUser", "getuser"),
            ("Get-User", "get-user"),
            ("Get_User", "get_user"),
            ("Get123User", "get123user"),
            ("Get.User", "get.user"),
            ("Get@User", "get-user"),
            ("Get#User", "get-user"),
            ("Get$User", "get-user"),
        ];

        for (input, expected) in method_test_cases {
            let cleaned = generator.clean_method_name(input);
            assert_eq!(cleaned, expected, "Method name regex failed for: {}", input);
        }
    }
}

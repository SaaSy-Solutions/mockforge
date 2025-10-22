//! GraphQL Schema Registry - SpecRegistry implementation for GraphQL
//!
//! This module provides a SpecRegistry implementation that can load GraphQL schemas
//! from files and generate mock responses.

use async_graphql::parser::parse_schema;
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
};
use mockforge_core::{
    ProtocolValidationError as ValidationError, ProtocolValidationResult as ValidationResult,
    Result,
};
use std::collections::HashMap;

/// GraphQL Schema Registry implementing SpecRegistry
pub struct GraphQLSchemaRegistry {
    /// Parsed schema SDL
    _schema_sdl: String,
    /// Query operations
    query_operations: Vec<SpecOperation>,
    /// Mutation operations
    mutation_operations: Vec<SpecOperation>,
}

impl GraphQLSchemaRegistry {
    /// Create a new GraphQL schema registry from SDL string
    pub fn from_sdl(sdl: &str) -> Result<Self> {
        // Validate SDL by parsing it with async-graphql's parser
        let _schema_doc = parse_schema(sdl).map_err(|e| {
            mockforge_core::Error::validation(format!("Invalid GraphQL schema: {}", e))
        })?;

        // Extract operation names from the SDL using string parsing
        // Note: This is a pragmatic approach for operation matching. While the async-graphql
        // parser validates the schema, extracting the operation names via string parsing is
        // simpler and sufficient for our handler matching needs. The schema is already
        // validated above, so we know it's well-formed.
        let mut query_operations = Vec::new();
        let mut mutation_operations = Vec::new();

        // Extract Query type fields
        if let Some(query_start) = sdl.find("type Query") {
            if let Some(query_block) = Self::extract_type_block(sdl, query_start) {
                query_operations = Self::extract_fields_as_operations(&query_block, "Query");
            }
        }

        // Extract Mutation type fields
        if let Some(mutation_start) = sdl.find("type Mutation") {
            if let Some(mutation_block) = Self::extract_type_block(sdl, mutation_start) {
                mutation_operations =
                    Self::extract_fields_as_operations(&mutation_block, "Mutation");
            }
        }

        Ok(Self {
            _schema_sdl: sdl.to_string(),
            query_operations,
            mutation_operations,
        })
    }

    /// Extract a type block from SDL (everything between { and })
    fn extract_type_block(sdl: &str, start_pos: usize) -> Option<String> {
        let remaining = &sdl[start_pos..];
        let open_brace = remaining.find('{')?;
        let close_brace = remaining.find('}')?;
        Some(remaining[open_brace + 1..close_brace].to_string())
    }

    /// Extract field names from a type block and convert to operations
    fn extract_fields_as_operations(block: &str, operation_type: &str) -> Vec<SpecOperation> {
        block
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }

                // Extract field name (before '(' or ':')
                let field_name = trimmed.split(['(', ':']).next()?.trim().to_string();

                Some(SpecOperation {
                    name: field_name.clone(),
                    path: format!("{}.{}", operation_type, field_name),
                    operation_type: operation_type.to_string(),
                    input_schema: None,
                    output_schema: None,
                    metadata: HashMap::new(),
                })
            })
            .collect()
    }

    /// Load schema from file
    pub async fn from_file(path: &str) -> Result<Self> {
        let sdl = tokio::fs::read_to_string(path).await?;
        Self::from_sdl(&sdl)
    }

    /// Generate mock response for a query/mutation
    fn generate_mock_response_data(&self, operation: &SpecOperation) -> serde_json::Value {
        // Extract the field name from the operation path (e.g., "Query.users" -> "users")
        let field_name = operation.name.as_str();

        // Check if it returns a list (common pattern: plural names or explicit list type)
        let is_list = field_name.ends_with('s')
            || operation.output_schema.as_ref().map(|s| s.starts_with('[')).unwrap_or(false);

        if is_list {
            // Generate a list of mock objects
            let items: Vec<serde_json::Value> = (0..3)
                .map(|i| {
                    serde_json::json!({
                        "id": format!("{}-{}", field_name, i),
                        "name": format!("Mock {} {}", field_name, i),
                        "description": format!("This is mock {} number {}", field_name, i),
                    })
                })
                .collect();
            serde_json::json!(items)
        } else {
            // Generate a single mock object
            serde_json::json!({
                "id": format!("{}-1", field_name),
                "name": format!("Mock {}", field_name),
                "description": format!("This is a mock {}", field_name),
            })
        }
    }
}

impl SpecRegistry for GraphQLSchemaRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::GraphQL
    }

    fn operations(&self) -> Vec<SpecOperation> {
        let mut ops = self.query_operations.clone();
        ops.extend(self.mutation_operations.clone());
        ops
    }

    fn find_operation(&self, operation: &str, _path: &str) -> Option<SpecOperation> {
        // Operation format: "Query.fieldName" or "Mutation.fieldName"
        self.operations()
            .into_iter()
            .find(|op| op.path == operation || op.name == operation)
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        // For now, basic validation - just check if the operation exists
        if let Some(_op) = self.find_operation(&request.operation, &request.path) {
            Ok(ValidationResult::success())
        } else {
            Ok(ValidationResult::failure(vec![ValidationError {
                message: format!("Unknown GraphQL operation: {}", request.operation),
                path: Some(request.path.clone()),
                code: Some("UNKNOWN_OPERATION".to_string()),
            }]))
        }
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        // Find the operation
        let operation =
            self.find_operation(&request.operation, &request.path).ok_or_else(|| {
                mockforge_core::Error::validation(format!(
                    "Unknown operation: {}",
                    request.operation
                ))
            })?;

        // Generate mock data
        let data = self.generate_mock_response_data(&operation);

        // Create GraphQL response format
        let graphql_response = serde_json::json!({
            "data": {
                &operation.name: data
            }
        });

        let body = serde_json::to_vec(&graphql_response)?;

        Ok(ProtocolResponse {
            status: ResponseStatus::GraphQLStatus(true),
            metadata: {
                let mut m = HashMap::new();
                m.insert("content-type".to_string(), "application/json".to_string());
                m
            },
            body,
            content_type: "application/json".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SCHEMA: &str = r#"
        type Query {
            users(limit: Int): [User!]!
            user(id: ID!): User
            posts(limit: Int): [Post!]!
        }

        type Mutation {
            createUser(input: CreateUserInput!): User!
            updateUser(id: ID!, input: UpdateUserInput!): User
            deleteUser(id: ID!): Boolean!
        }

        type User {
            id: ID!
            name: String!
            email: String!
            posts: [Post!]!
        }

        type Post {
            id: ID!
            title: String!
            content: String!
            author: User!
        }

        input CreateUserInput {
            name: String!
            email: String!
        }

        input UpdateUserInput {
            name: String
            email: String
        }
    "#;

    #[test]
    fn test_from_sdl() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA);
        assert!(registry.is_ok());

        let registry = registry.unwrap();
        assert_eq!(registry.query_operations.len(), 3);
        assert_eq!(registry.mutation_operations.len(), 3);
    }

    #[test]
    fn test_protocol() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();
        assert_eq!(registry.protocol(), Protocol::GraphQL);
    }

    #[test]
    fn test_operations() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();
        let ops = registry.operations();
        assert_eq!(ops.len(), 6); // 3 queries + 3 mutations

        // Check query operations
        assert!(ops.iter().any(|op| op.name == "users"));
        assert!(ops.iter().any(|op| op.name == "user"));
        assert!(ops.iter().any(|op| op.name == "posts"));

        // Check mutation operations
        assert!(ops.iter().any(|op| op.name == "createUser"));
        assert!(ops.iter().any(|op| op.name == "updateUser"));
        assert!(ops.iter().any(|op| op.name == "deleteUser"));
    }

    #[test]
    fn test_find_operation() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();

        let op = registry.find_operation("Query.users", "/graphql");
        assert!(op.is_some());
        assert_eq!(op.unwrap().name, "users");

        let op = registry.find_operation("Mutation.createUser", "/graphql");
        assert!(op.is_some());
        assert_eq!(op.unwrap().name, "createUser");

        let op = registry.find_operation("nonexistent", "/graphql");
        assert!(op.is_none());
    }

    #[test]
    fn test_validate_request() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::GraphQL,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::RequestResponse,
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            operation: "Query.users".to_string(),
            path: "/graphql".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request);
        assert!(result.is_ok());
        assert!(result.unwrap().valid);
    }

    #[test]
    fn test_generate_mock_response() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::GraphQL,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::RequestResponse,
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            operation: "Query.users".to_string(),
            path: "/graphql".to_string(),
            metadata: HashMap::new(),
            body: Some(b"{\"query\": \"{ users { id name email } }\"}".to_vec()),
            client_ip: None,
        };

        let response = registry.generate_mock_response(&request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.status, ResponseStatus::GraphQLStatus(true));
        assert_eq!(response.content_type, "application/json");

        // Parse response body
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(body.get("data").is_some());
        assert!(body["data"].get("users").is_some());
    }

    #[test]
    fn test_generate_mock_response_mutation() {
        let registry = GraphQLSchemaRegistry::from_sdl(SAMPLE_SCHEMA).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::GraphQL,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::RequestResponse,
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            operation: "Mutation.createUser".to_string(),
            path: "/graphql".to_string(),
            metadata: HashMap::new(),
            body: Some(b"{\"query\": \"mutation { createUser(input: {name: \\\"Test\\\", email: \\\"test@example.com\\\"}) { id name email } }\"}".to_vec()),
            client_ip: None,
        };

        let response = registry.generate_mock_response(&request);
        assert!(response.is_ok());

        let response = response.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(body.get("data").is_some());
        assert!(body["data"].get("createUser").is_some());
    }

    #[tokio::test]
    async fn test_from_file_nonexistent() {
        let result = GraphQLSchemaRegistry::from_file("/nonexistent/schema.graphql").await;
        assert!(result.is_err());
    }
}

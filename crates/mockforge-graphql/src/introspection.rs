//! GraphQL introspection support

use async_graphql::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Introspection query result
#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectionResult {
    pub data: IntrospectionData,
}

/// Introspection data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectionData {
    #[serde(rename = "__schema")]
    pub schema: SchemaInfo,
}

/// Schema information
#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub query_type: TypeRef,
    pub mutation_type: Option<TypeRef>,
    pub subscription_type: Option<TypeRef>,
    pub types: Vec<TypeInfo>,
    pub directives: Vec<DirectiveInfo>,
}

/// Type reference
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeRef {
    pub name: Option<String>,
    pub kind: Option<String>,
    #[serde(rename = "ofType")]
    pub of_type: Option<Box<TypeRef>>,
}

/// Type information
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: Option<String>,
    pub kind: String,
    pub description: Option<String>,
    pub fields: Option<Vec<FieldInfo>>,
    pub interfaces: Option<Vec<TypeRef>>,
    pub possible_types: Option<Vec<TypeRef>>,
    pub enum_values: Option<Vec<EnumValue>>,
    pub input_fields: Option<Vec<InputField>>,
}

/// Field information
#[derive(Debug, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub field_type: TypeRef,
    pub args: Vec<InputField>,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

/// Input field information
#[derive(Debug, Serialize, Deserialize)]
pub struct InputField {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub field_type: TypeRef,
    pub default_value: Option<String>,
}

/// Enum value information
#[derive(Debug, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub description: Option<String>,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

/// Directive information
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectiveInfo {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<String>,
    pub args: Vec<InputField>,
}

/// Introspection handler
pub struct IntrospectionHandler {
    schema_info: SchemaInfo,
}

impl IntrospectionHandler {
    /// Create a new introspection handler
    pub fn new() -> Self {
        Self {
            schema_info: Self::create_basic_schema_info(),
        }
    }

    /// Handle introspection query
    pub fn handle_introspection(&self, query: &str) -> Option<Value> {
        // Check if this is an introspection query
        if query.contains("__schema") || query.contains("__type") {
            Some(self.generate_introspection_response())
        } else {
            None
        }
    }

    /// Generate introspection response
    fn generate_introspection_response(&self) -> Value {
        let result = IntrospectionResult {
            data: IntrospectionData {
                schema: self.schema_info.clone(),
            },
        };

        serde_json::to_value(result).unwrap_or(Value::Null)
    }

    /// Create basic schema information
    fn create_basic_schema_info() -> SchemaInfo {
        SchemaInfo {
            query_type: TypeRef {
                name: Some("Query".to_string()),
                kind: Some("OBJECT".to_string()),
                of_type: None,
            },
            mutation_type: Some(TypeRef {
                name: Some("Mutation".to_string()),
                kind: Some("OBJECT".to_string()),
                of_type: None,
            }),
            subscription_type: None,
            types: Self::create_basic_types(),
            directives: vec![],
        }
    }

    /// Create basic type information
    fn create_basic_types() -> Vec<TypeInfo> {
        vec![
            // Query type
            TypeInfo {
                name: Some("Query".to_string()),
                kind: "OBJECT".to_string(),
                description: Some("Root query type".to_string()),
                fields: Some(vec![
                    FieldInfo {
                        name: "users".to_string(),
                        description: Some("Get all users".to_string()),
                        field_type: TypeRef {
                            name: None,
                            kind: Some("LIST".to_string()),
                            of_type: Some(Box::new(TypeRef {
                                name: Some("User".to_string()),
                                kind: Some("OBJECT".to_string()),
                                of_type: None,
                            })),
                        },
                        args: vec![
                            InputField {
                                name: "limit".to_string(),
                                description: Some("Maximum number of users to return".to_string()),
                                field_type: TypeRef {
                                    name: Some("Int".to_string()),
                                    kind: Some("SCALAR".to_string()),
                                    of_type: None,
                                },
                                default_value: Some("10".to_string()),
                            },
                        ],
                        is_deprecated: false,
                        deprecation_reason: None,
                    },
                    FieldInfo {
                        name: "user".to_string(),
                        description: Some("Get a user by ID".to_string()),
                        field_type: TypeRef {
                            name: Some("User".to_string()),
                            kind: Some("OBJECT".to_string()),
                            of_type: None,
                        },
                        args: vec![
                            InputField {
                                name: "id".to_string(),
                                description: Some("User ID".to_string()),
                                field_type: TypeRef {
                                    name: Some("ID".to_string()),
                                    kind: Some("SCALAR".to_string()),
                                    of_type: None,
                                },
                                default_value: None,
                            },
                        ],
                        is_deprecated: false,
                        deprecation_reason: None,
                    },
                ]),
                interfaces: None,
                possible_types: None,
                enum_values: None,
                input_fields: None,
            },
            // User type
            TypeInfo {
                name: Some("User".to_string()),
                kind: "OBJECT".to_string(),
                description: Some("User object type".to_string()),
                fields: Some(vec![
                    FieldInfo {
                        name: "id".to_string(),
                        description: Some("Unique identifier".to_string()),
                        field_type: TypeRef {
                            name: Some("ID".to_string()),
                            kind: Some("SCALAR".to_string()),
                            of_type: None,
                        },
                        args: vec![],
                        is_deprecated: false,
                        deprecation_reason: None,
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        description: Some("User's full name".to_string()),
                        field_type: TypeRef {
                            name: Some("String".to_string()),
                            kind: Some("SCALAR".to_string()),
                            of_type: None,
                        },
                        args: vec![],
                        is_deprecated: false,
                        deprecation_reason: None,
                    },
                    FieldInfo {
                        name: "email".to_string(),
                        description: Some("User's email address".to_string()),
                        field_type: TypeRef {
                            name: Some("String".to_string()),
                            kind: Some("SCALAR".to_string()),
                            of_type: None,
                        },
                        args: vec![],
                        is_deprecated: false,
                        deprecation_reason: None,
                    },
                ]),
                interfaces: None,
                possible_types: None,
                enum_values: None,
                input_fields: None,
            },
            // Built-in scalar types
            TypeInfo {
                name: Some("String".to_string()),
                kind: "SCALAR".to_string(),
                description: Some("String scalar type".to_string()),
                fields: None,
                interfaces: None,
                possible_types: None,
                enum_values: None,
                input_fields: None,
            },
            TypeInfo {
                name: Some("ID".to_string()),
                kind: "SCALAR".to_string(),
                description: Some("ID scalar type".to_string()),
                fields: None,
                interfaces: None,
                possible_types: None,
                enum_values: None,
                input_fields: None,
            },
            TypeInfo {
                name: Some("Int".to_string()),
                kind: "SCALAR".to_string(),
                description: Some("Integer scalar type".to_string()),
                fields: None,
                interfaces: None,
                possible_types: None,
                enum_values: None,
                input_fields: None,
            },
        ]
    }
}

impl Default for IntrospectionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ref_creation() {
        let type_ref = TypeRef {
            name: Some("String".to_string()),
            kind: Some("SCALAR".to_string()),
            of_type: None,
        };

        assert_eq!(type_ref.name, Some("String".to_string()));
        assert_eq!(type_ref.kind, Some("SCALAR".to_string()));
        assert!(type_ref.of_type.is_none());
    }

    #[test]
    fn test_type_ref_with_nested_type() {
        let type_ref = TypeRef {
            name: None,
            kind: Some("LIST".to_string()),
            of_type: Some(Box::new(TypeRef {
                name: Some("User".to_string()),
                kind: Some("OBJECT".to_string()),
                of_type: None,
            })),
        };

        assert!(type_ref.name.is_none());
        assert_eq!(type_ref.kind, Some("LIST".to_string()));
        assert!(type_ref.of_type.is_some());
        assert_eq!(type_ref.of_type.as_ref().unwrap().name, Some("User".to_string()));
    }

    #[test]
    fn test_input_field_creation() {
        let input_field = InputField {
            name: "id".to_string(),
            description: Some("User ID".to_string()),
            field_type: TypeRef {
                name: Some("ID".to_string()),
                kind: Some("SCALAR".to_string()),
                of_type: None,
            },
            default_value: None,
        };

        assert_eq!(input_field.name, "id");
        assert_eq!(input_field.description, Some("User ID".to_string()));
        assert!(input_field.default_value.is_none());
    }

    #[test]
    fn test_input_field_with_default_value() {
        let input_field = InputField {
            name: "limit".to_string(),
            description: Some("Limit".to_string()),
            field_type: TypeRef {
                name: Some("Int".to_string()),
                kind: Some("SCALAR".to_string()),
                of_type: None,
            },
            default_value: Some("10".to_string()),
        };

        assert_eq!(input_field.default_value, Some("10".to_string()));
    }

    #[test]
    fn test_field_info_creation() {
        let field_info = FieldInfo {
            name: "users".to_string(),
            description: Some("Get all users".to_string()),
            field_type: TypeRef {
                name: Some("User".to_string()),
                kind: Some("OBJECT".to_string()),
                of_type: None,
            },
            args: vec![],
            is_deprecated: false,
            deprecation_reason: None,
        };

        assert_eq!(field_info.name, "users");
        assert!(!field_info.is_deprecated);
        assert_eq!(field_info.args.len(), 0);
    }

    #[test]
    fn test_field_info_with_deprecation() {
        let field_info = FieldInfo {
            name: "oldField".to_string(),
            description: Some("Deprecated field".to_string()),
            field_type: TypeRef {
                name: Some("String".to_string()),
                kind: Some("SCALAR".to_string()),
                of_type: None,
            },
            args: vec![],
            is_deprecated: true,
            deprecation_reason: Some("Use newField instead".to_string()),
        };

        assert!(field_info.is_deprecated);
        assert_eq!(field_info.deprecation_reason, Some("Use newField instead".to_string()));
    }

    #[test]
    fn test_enum_value_creation() {
        let enum_value = EnumValue {
            name: "ACTIVE".to_string(),
            description: Some("Active status".to_string()),
            is_deprecated: false,
            deprecation_reason: None,
        };

        assert_eq!(enum_value.name, "ACTIVE");
        assert!(!enum_value.is_deprecated);
    }

    #[test]
    fn test_directive_info_creation() {
        let directive_info = DirectiveInfo {
            name: "skip".to_string(),
            description: Some("Skip field".to_string()),
            locations: vec!["FIELD".to_string(), "FRAGMENT_SPREAD".to_string()],
            args: vec![],
        };

        assert_eq!(directive_info.name, "skip");
        assert_eq!(directive_info.locations.len(), 2);
    }

    #[test]
    fn test_type_info_scalar() {
        let type_info = TypeInfo {
            name: Some("String".to_string()),
            kind: "SCALAR".to_string(),
            description: Some("String scalar type".to_string()),
            fields: None,
            interfaces: None,
            possible_types: None,
            enum_values: None,
            input_fields: None,
        };

        assert_eq!(type_info.kind, "SCALAR");
        assert!(type_info.fields.is_none());
    }

    #[test]
    fn test_type_info_object_with_fields() {
        let type_info = TypeInfo {
            name: Some("User".to_string()),
            kind: "OBJECT".to_string(),
            description: Some("User object".to_string()),
            fields: Some(vec![FieldInfo {
                name: "id".to_string(),
                description: Some("ID".to_string()),
                field_type: TypeRef {
                    name: Some("ID".to_string()),
                    kind: Some("SCALAR".to_string()),
                    of_type: None,
                },
                args: vec![],
                is_deprecated: false,
                deprecation_reason: None,
            }]),
            interfaces: None,
            possible_types: None,
            enum_values: None,
            input_fields: None,
        };

        assert_eq!(type_info.kind, "OBJECT");
        assert_eq!(type_info.fields.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_schema_info_creation() {
        let schema_info = SchemaInfo {
            query_type: TypeRef {
                name: Some("Query".to_string()),
                kind: Some("OBJECT".to_string()),
                of_type: None,
            },
            mutation_type: None,
            subscription_type: None,
            types: vec![],
            directives: vec![],
        };

        assert_eq!(schema_info.query_type.name, Some("Query".to_string()));
        assert!(schema_info.mutation_type.is_none());
    }

    #[test]
    fn test_introspection_data_creation() {
        let data = IntrospectionData {
            schema: SchemaInfo {
                query_type: TypeRef {
                    name: Some("Query".to_string()),
                    kind: Some("OBJECT".to_string()),
                    of_type: None,
                },
                mutation_type: None,
                subscription_type: None,
                types: vec![],
                directives: vec![],
            },
        };

        assert_eq!(data.schema.query_type.name, Some("Query".to_string()));
    }

    #[test]
    fn test_introspection_result_creation() {
        let result = IntrospectionResult {
            data: IntrospectionData {
                schema: SchemaInfo {
                    query_type: TypeRef {
                        name: Some("Query".to_string()),
                        kind: Some("OBJECT".to_string()),
                        of_type: None,
                    },
                    mutation_type: None,
                    subscription_type: None,
                    types: vec![],
                    directives: vec![],
                },
            },
        };

        assert_eq!(result.data.schema.query_type.name, Some("Query".to_string()));
    }

    #[test]
    fn test_introspection_handler_new() {
        let handler = IntrospectionHandler::new();
        assert_eq!(handler.schema_info.query_type.name, Some("Query".to_string()));
    }

    #[test]
    fn test_introspection_handler_default() {
        let handler = IntrospectionHandler::default();
        assert_eq!(handler.schema_info.query_type.name, Some("Query".to_string()));
    }

    #[test]
    fn test_handle_introspection_schema_query() {
        let handler = IntrospectionHandler::new();
        let result = handler.handle_introspection("{ __schema { queryType { name } } }");
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_introspection_type_query() {
        let handler = IntrospectionHandler::new();
        let result = handler.handle_introspection("{ __type(name: \"User\") { name kind } }");
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_introspection_non_introspection_query() {
        let handler = IntrospectionHandler::new();
        let result = handler.handle_introspection("{ users { id name } }");
        assert!(result.is_none());
    }

    #[test]
    fn test_basic_schema_info_has_query_type() {
        let handler = IntrospectionHandler::new();
        assert_eq!(handler.schema_info.query_type.name, Some("Query".to_string()));
        assert_eq!(handler.schema_info.query_type.kind, Some("OBJECT".to_string()));
    }

    #[test]
    fn test_basic_schema_info_has_mutation_type() {
        let handler = IntrospectionHandler::new();
        assert!(handler.schema_info.mutation_type.is_some());
        assert_eq!(
            handler.schema_info.mutation_type.as_ref().unwrap().name,
            Some("Mutation".to_string())
        );
    }

    #[test]
    fn test_basic_schema_info_no_subscription_type() {
        let handler = IntrospectionHandler::new();
        assert!(handler.schema_info.subscription_type.is_none());
    }

    #[test]
    fn test_basic_schema_info_has_types() {
        let handler = IntrospectionHandler::new();
        assert!(!handler.schema_info.types.is_empty());
    }

    #[test]
    fn test_basic_types_includes_query() {
        let handler = IntrospectionHandler::new();
        let query_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"Query".to_string()));
        assert!(query_type.is_some());
    }

    #[test]
    fn test_basic_types_includes_user() {
        let handler = IntrospectionHandler::new();
        let user_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"User".to_string()));
        assert!(user_type.is_some());
    }

    #[test]
    fn test_basic_types_includes_scalars() {
        let handler = IntrospectionHandler::new();
        let has_string = handler
            .schema_info
            .types
            .iter()
            .any(|t| t.name.as_ref() == Some(&"String".to_string()));
        let has_id = handler
            .schema_info
            .types
            .iter()
            .any(|t| t.name.as_ref() == Some(&"ID".to_string()));
        let has_int = handler
            .schema_info
            .types
            .iter()
            .any(|t| t.name.as_ref() == Some(&"Int".to_string()));

        assert!(has_string);
        assert!(has_id);
        assert!(has_int);
    }

    #[test]
    fn test_query_type_has_users_field() {
        let handler = IntrospectionHandler::new();
        let query_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"Query".to_string()));

        let has_users = query_type
            .and_then(|qt| qt.fields.as_ref())
            .map(|fields| fields.iter().any(|f| f.name == "users"))
            .unwrap_or(false);

        assert!(has_users);
    }

    #[test]
    fn test_query_type_has_user_field() {
        let handler = IntrospectionHandler::new();
        let query_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"Query".to_string()));

        let has_user = query_type
            .and_then(|qt| qt.fields.as_ref())
            .map(|fields| fields.iter().any(|f| f.name == "user"))
            .unwrap_or(false);

        assert!(has_user);
    }

    #[test]
    fn test_user_type_has_id_field() {
        let handler = IntrospectionHandler::new();
        let user_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"User".to_string()));

        let has_id = user_type
            .and_then(|ut| ut.fields.as_ref())
            .map(|fields| fields.iter().any(|f| f.name == "id"))
            .unwrap_or(false);

        assert!(has_id);
    }

    #[test]
    fn test_user_type_has_name_field() {
        let handler = IntrospectionHandler::new();
        let user_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"User".to_string()));

        let has_name = user_type
            .and_then(|ut| ut.fields.as_ref())
            .map(|fields| fields.iter().any(|f| f.name == "name"))
            .unwrap_or(false);

        assert!(has_name);
    }

    #[test]
    fn test_user_type_has_email_field() {
        let handler = IntrospectionHandler::new();
        let user_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"User".to_string()));

        let has_email = user_type
            .and_then(|ut| ut.fields.as_ref())
            .map(|fields| fields.iter().any(|f| f.name == "email"))
            .unwrap_or(false);

        assert!(has_email);
    }

    #[test]
    fn test_users_field_has_limit_arg() {
        let handler = IntrospectionHandler::new();
        let query_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"Query".to_string()));

        let has_limit_arg = query_type
            .and_then(|qt| qt.fields.as_ref())
            .and_then(|fields| fields.iter().find(|f| f.name == "users"))
            .map(|users_field| users_field.args.iter().any(|arg| arg.name == "limit"))
            .unwrap_or(false);

        assert!(has_limit_arg);
    }

    #[test]
    fn test_user_field_has_id_arg() {
        let handler = IntrospectionHandler::new();
        let query_type = handler
            .schema_info
            .types
            .iter()
            .find(|t| t.name.as_ref() == Some(&"Query".to_string()));

        let has_id_arg = query_type
            .and_then(|qt| qt.fields.as_ref())
            .and_then(|fields| fields.iter().find(|f| f.name == "user"))
            .map(|user_field| user_field.args.iter().any(|arg| arg.name == "id"))
            .unwrap_or(false);

        assert!(has_id_arg);
    }

    #[test]
    fn test_generate_introspection_response_not_null() {
        let handler = IntrospectionHandler::new();
        let result = handler.handle_introspection("{ __schema { queryType { name } } }");
        assert!(result.is_some());
        assert!(!matches!(result.unwrap(), Value::Null));
    }
}

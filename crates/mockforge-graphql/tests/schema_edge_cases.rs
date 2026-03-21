//! Schema edge case tests for GraphQL protocol.
//!
//! Tests schema validation with empty schemas, minimal schemas, deeply nested types,
//! schemas with comments, and various structural patterns.

use mockforge_core::protocol_abstraction::SpecRegistry;
use mockforge_graphql::{GraphQLSchema, GraphQLSchemaRegistry};

#[test]
fn test_empty_schema_sdl_rejected() {
    let result = GraphQLSchemaRegistry::from_sdl("");
    // An empty string is not valid GraphQL SDL — should fail parsing
    assert!(result.is_err());
}

#[test]
fn test_minimal_query_only_schema() {
    let sdl = r#"
        type Query {
            hello: String
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].name, "hello");
    assert_eq!(ops[0].path, "Query.hello");
}

#[test]
fn test_schema_with_no_query_type() {
    // A valid SDL with only custom types but no Query root — async-graphql parser
    // should accept this as a document, but we'll have zero operations.
    let sdl = r#"
        type User {
            id: ID!
            name: String!
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    assert_eq!(ops.len(), 0);
}

#[test]
fn test_deeply_nested_type_definitions() {
    let sdl = r#"
        type Query {
            organization(id: ID!): Organization
        }

        type Organization {
            id: ID!
            name: String!
            departments: [Department!]!
        }

        type Department {
            id: ID!
            name: String!
            teams: [Team!]!
        }

        type Team {
            id: ID!
            name: String!
            members: [Member!]!
        }

        type Member {
            id: ID!
            name: String!
            role: String!
            address: Address
        }

        type Address {
            street: String
            city: String
            country: String
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    // Only the Query fields become operations
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].name, "organization");
}

#[test]
fn test_schema_with_comments_and_descriptions() {
    let sdl = r#"
        # This is a comment
        "Schema-level description"
        type Query {
            # Returns a user
            user(id: ID!): User
            "Returns all users"
            users(limit: Int = 10): [User!]!
        }

        "A user in the system"
        type User {
            id: ID!
            name: String!
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    // Comments should not break field extraction
    assert!(ops.iter().any(|op| op.name == "user"));
    assert!(ops.iter().any(|op| op.name == "users"));
}

#[test]
fn test_schema_with_interfaces() {
    let sdl = r#"
        type Query {
            node(id: ID!): Node
        }

        interface Node {
            id: ID!
        }

        type User implements Node {
            id: ID!
            name: String!
        }

        type Post implements Node {
            id: ID!
            title: String!
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    assert!(registry.operations().iter().any(|op| op.name == "node"));
}

#[test]
fn test_schema_with_enums_and_unions() {
    let sdl = r#"
        type Query {
            search(term: String!): [SearchResult!]!
        }

        union SearchResult = User | Post

        enum Role {
            ADMIN
            USER
            GUEST
        }

        type User {
            id: ID!
            name: String!
            role: Role!
        }

        type Post {
            id: ID!
            title: String!
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].name, "search");
}

#[test]
fn test_invalid_sdl_syntax() {
    let invalid_sdl = "type Query {{ broken syntax";
    let result = GraphQLSchemaRegistry::from_sdl(invalid_sdl);
    assert!(result.is_err());
}

#[test]
fn test_schema_with_many_query_fields() {
    let mut sdl = String::from("type Query {\n");
    for i in 0..20 {
        sdl.push_str(&format!("    field{i}(arg: Int): String\n"));
    }
    sdl.push_str("}\n");

    let registry = GraphQLSchemaRegistry::from_sdl(&sdl).unwrap();
    let ops = registry.operations();
    assert_eq!(ops.len(), 20);
}

#[test]
fn test_schema_with_input_types() {
    let sdl = r#"
        type Query {
            user(id: ID!): User
        }

        type Mutation {
            createUser(input: CreateUserInput!): User!
        }

        type User {
            id: ID!
            name: String!
            email: String!
        }

        input CreateUserInput {
            name: String!
            email: String!
            address: AddressInput
        }

        input AddressInput {
            street: String!
            city: String!
            zip: String!
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    assert!(ops.iter().any(|op| op.name == "user"));
    assert!(ops.iter().any(|op| op.name == "createUser"));
}

#[test]
fn test_default_schema_sdl_contains_expected_types() {
    let schema = GraphQLSchema::new();
    let sdl = schema.schema().sdl();

    // The default schema should contain the User and Post types
    assert!(sdl.contains("type Query"));
    assert!(sdl.contains("User"));
    assert!(sdl.contains("Post"));
    assert!(sdl.contains("users"));
    assert!(sdl.contains("posts"));
}

#[test]
fn test_generate_basic_schema_equals_new() {
    let schema_new = GraphQLSchema::new();
    let schema_basic = GraphQLSchema::generate_basic_schema();

    // Both should produce the same SDL
    assert_eq!(schema_new.schema().sdl(), schema_basic.schema().sdl());
}

#[test]
fn test_schema_with_scalar_types() {
    let sdl = r#"
        scalar DateTime
        scalar JSON

        type Query {
            event(id: ID!): Event
        }

        type Event {
            id: ID!
            title: String!
            startDate: DateTime!
            metadata: JSON
        }
    "#;
    let registry = GraphQLSchemaRegistry::from_sdl(sdl).unwrap();
    let ops = registry.operations();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].name, "event");
}

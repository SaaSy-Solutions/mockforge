//! GraphQL schema parsing and generation

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

/// Simple User type for GraphQL
#[derive(async_graphql::SimpleObject)]
pub struct User {
    /// Unique identifier
    pub id: String,
    /// User's full name
    pub name: String,
    /// User's email address
    pub email: String,
}

/// Simple Post type for GraphQL
#[derive(async_graphql::SimpleObject)]
pub struct Post {
    /// Unique identifier
    pub id: String,
    /// Post title
    pub title: String,
    /// Post content
    pub content: String,
    /// Author of the post
    pub author: User,
}

/// Root query type
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all users
    async fn users(&self, limit: Option<i32>) -> Vec<User> {
        let limit = limit.unwrap_or(10) as usize;
        (0..limit.min(100))
            .map(|i| User {
                id: format!("user-{}", i),
                name: format!("User {}", i),
                email: format!("user{}@example.com", i),
            })
            .collect()
    }

    /// Get a user by ID
    async fn user(&self, id: String) -> Option<User> {
        Some(User {
            id,
            name: "Mock User".to_string(),
            email: "mock@example.com".to_string(),
        })
    }

    /// Get all posts
    async fn posts(&self, limit: Option<i32>) -> Vec<Post> {
        let limit = limit.unwrap_or(10) as usize;
        (0..limit.min(50))
            .map(|i| Post {
                id: format!("post-{}", i),
                title: format!("Post {}", i),
                content: format!("This is the content of post {}", i),
                author: User {
                    id: format!("user-{}", i % 5),
                    name: format!("Author {}", i % 5),
                    email: format!("author{}@example.com", i % 5),
                },
            })
            .collect()
    }
}

/// GraphQL schema manager
pub struct GraphQLSchema {
    schema: Schema<QueryRoot, EmptyMutation, EmptySubscription>,
}

impl GraphQLSchema {
    /// Create a new basic schema
    pub fn new() -> Self {
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();
        Self { schema }
    }

    /// Get the underlying schema
    pub fn schema(&self) -> &Schema<QueryRoot, EmptyMutation, EmptySubscription> {
        &self.schema
    }

    /// Generate a basic schema with common types
    pub fn generate_basic_schema() -> Self {
        Self::new()
    }
}

impl Default for GraphQLSchema {
    fn default() -> Self {
        Self::new()
    }
}

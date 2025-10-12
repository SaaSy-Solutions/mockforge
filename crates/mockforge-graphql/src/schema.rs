//! GraphQL schema parsing and generation

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

/// Simple User type for GraphQL
#[derive(async_graphql::SimpleObject, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: "user-1".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };

        assert_eq!(user.id, "user-1");
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
    }

    #[test]
    fn test_post_creation() {
        let user = User {
            id: "user-1".to_string(),
            name: "Author".to_string(),
            email: "author@example.com".to_string(),
        };

        let post = Post {
            id: "post-1".to_string(),
            title: "Test Post".to_string(),
            content: "This is a test post".to_string(),
            author: user.clone(),
        };

        assert_eq!(post.id, "post-1");
        assert_eq!(post.title, "Test Post");
        assert_eq!(post.author.name, "Author");
    }

    #[test]
    fn test_query_root_creation() {
        let _query = QueryRoot;
        // Should create successfully
    }

    #[test]
    fn test_graphql_schema_new() {
        let schema = GraphQLSchema::new();
        assert!(!schema.schema().sdl().is_empty());
    }

    #[test]
    fn test_graphql_schema_default() {
        let schema = GraphQLSchema::default();
        assert!(!schema.schema().sdl().is_empty());
    }

    #[test]
    fn test_graphql_schema_generate_basic() {
        let schema = GraphQLSchema::generate_basic_schema();
        let sdl = schema.schema().sdl();
        assert!(sdl.contains("Query"));
    }
}

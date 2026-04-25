//! GraphQL schema parsing and generation

use async_graphql::{Object, Schema, Subscription};
use futures::stream::Stream;
use std::time::Duration;

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

/// Root mutation type. A single `createUser` mutation lets clients
/// exercise the mutation dispatch path against the default schema
/// without registering a custom SDL. The mutation is stateless — the
/// "created" user is fabricated from the inputs plus a deterministic
/// id derived from (name, email), so callers can assert on the shape
/// of the return value without mocking storage.
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new user. Returns a freshly-shaped `User` with the
    /// supplied `name` / `email` and an id derived from them so the
    /// output is deterministic for tests (`user-{12 hex chars}`).
    async fn create_user(&self, name: String, email: String) -> User {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        email.hash(&mut hasher);
        // Mask to 48 bits so the hex suffix is always exactly 12 chars
        // (tests can then pin the exact id length without depending on
        // how the hasher happens to fill the high bits).
        let id = format!("user-{:012x}", hasher.finish() & 0xffff_ffff_ffff);
        User { id, name, email }
    }
}

/// Root subscription type. A single `tick` subscription lets clients
/// exercise the subscription dispatch path (WebSocket via
/// `graphql-transport-ws` / `graphql-ws`, SSE, etc.) against the
/// default schema without registering a custom SDL.
///
/// `tick` emits a monotonically increasing `i32` every 100ms, starting
/// from 1, up to the requested `count`. Clients without a `count`
/// argument get 5 ticks by default. The interval is short enough that
/// multi-event tests complete in well under a second.
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Emit `count` ticks (default 5) at 100ms intervals.
    async fn tick(&self, count: Option<i32>) -> impl Stream<Item = i32> {
        let n = count.unwrap_or(5).max(1) as usize;
        async_stream::stream! {
            for i in 1..=n {
                tokio::time::sleep(Duration::from_millis(100)).await;
                yield i as i32;
            }
        }
    }
}

/// GraphQL schema manager
pub struct GraphQLSchema {
    schema: Schema<QueryRoot, MutationRoot, SubscriptionRoot>,
}

impl GraphQLSchema {
    /// Create a new basic schema
    pub fn new() -> Self {
        let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).finish();
        Self { schema }
    }

    /// Get the underlying schema
    pub fn schema(&self) -> &Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
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

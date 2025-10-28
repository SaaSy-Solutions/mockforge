//! # MockForge SDK
//!
//! Developer SDK for embedding MockForge mock servers directly in unit and integration tests.
//!
//! ## Features
//!
//! - **Start/Stop Mock Servers**: Programmatically control mock server lifecycle
//! - **Stub Responses**: Define mock responses with a fluent API
//! - **Offline Mode**: Works without network dependencies
//! - **Multi-Protocol**: HTTP, WebSocket, gRPC, GraphQL support
//! - **Ergonomic API**: Builder pattern for easy configuration
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mockforge_sdk::MockServer;
//! use serde_json::json;
//!
//! #[tokio::test]
//! async fn test_user_api() {
//!     // Start a mock server
//!     let mut server = MockServer::new()
//!         .port(3000)
//!         .start()
//!         .await
//!         .expect("Failed to start server");
//!
//!     // Stub a response
//!     server
//!         .stub_response("GET", "/api/users/{id}", json!({
//!             "id": "{{uuid}}",
//!             "name": "{{faker.name}}",
//!             "email": "{{faker.email}}"
//!         }))
//!         .await
//!         .expect("Failed to stub response");
//!
//!     // Make requests to the mock server
//!     let client = reqwest::Client::new();
//!     let response = client
//!         .get("http://localhost:3000/api/users/123")
//!         .send()
//!         .await
//!         .expect("Failed to make request");
//!
//!     assert_eq!(response.status(), 200);
//!
//!     // Stop the server
//!     server.stop().await.expect("Failed to stop server");
//! }
//! ```

pub mod admin;
pub mod builder;
pub mod error;
pub mod ffi;
pub mod server;
pub mod stub;

pub use admin::{AdminClient, MockConfig as AdminMockConfig, MockConfigBuilder, MockList, MockResponse as AdminMockResponse, ServerConfig as AdminServerConfig, ServerStats};
pub use builder::MockServerBuilder;
pub use error::{Error, Result};
pub use server::MockServer;
pub use stub::{DynamicStub, DynamicResponseFn, RequestContext, ResponseStub, StubBuilder};

// Re-export commonly used types from mockforge-core
pub use mockforge_core::{
    Config, FailureConfig, LatencyProfile, OpenApiSpec, ProxyConfig, ServerConfig,
};

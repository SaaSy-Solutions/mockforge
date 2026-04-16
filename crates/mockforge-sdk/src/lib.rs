//! Pillars: [`DevX`]
//!
//! # `MockForge` SDK
//!
//! Developer SDK for embedding `MockForge` mock servers directly in unit and integration tests.
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

// `mockforge_core::ProxyConfig` is deprecated pending migration to the
// `mockforge-proxy` crate; the SDK re-exports it for now so we scope the allow
// here rather than touching every public re-export site.
#![allow(deprecated)]

pub mod admin;
pub mod builder;
pub mod conformance;
pub mod error;
pub mod ffi;
pub mod server;
pub mod stub;
pub mod verification;

pub use admin::{
    AdminClient, MockConfig as AdminMockConfig, MockConfigBuilder, MockList,
    MockResponse as AdminMockResponse, RequestMatchCriteria, ServerConfig as AdminServerConfig,
    ServerStats,
};
pub use builder::MockServerBuilder;
pub use conformance::{ConformanceClient, ConformanceRun, ConformanceRunRequest, RunStatus};
pub use error::{Error, Result};
pub use server::MockServer;
pub use stub::{
    DynamicResponseFn, DynamicStub, RequestContext, ResourceIdExtractConfig, ResponseStub,
    StateMachineConfig, StateResponseOverride, StubBuilder, StubFaultInjectionConfig,
};
pub use verification::Verification;

// Re-export commonly used types
pub use mockforge_core::{Config, ProxyConfig, ServerConfig};
pub use mockforge_foundation::failure_injection::FailureConfig;
pub use mockforge_foundation::latency::LatencyProfile;
pub use mockforge_openapi::OpenApiSpec;

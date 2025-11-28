//! Consistency engine integration for HTTP protocol
//!
//! This module provides HTTP-specific integration with the cross-protocol
//! consistency engine, ensuring HTTP responses reflect unified state.

pub mod http_adapter;
pub mod middleware;
pub mod response_enrichment;

pub use http_adapter::HttpAdapter;
pub use middleware::ConsistencyMiddlewareState;

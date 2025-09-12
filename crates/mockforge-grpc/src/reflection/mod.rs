//! Reflection-based gRPC proxy implementation
//!
//! This module provides functionality to dynamically proxy gRPC requests
//! to arbitrary services using reflection.

pub mod client;
pub mod descriptor;
pub mod proxy;
pub mod mock_proxy;
pub mod cache;
pub mod config;
pub mod connection_pool;
pub mod error_handling;
pub mod metrics;

pub use client::ReflectionClient;
pub use proxy::ReflectionProxy;
pub use mock_proxy::MockReflectionProxy;
pub use config::ProxyConfig;
pub use connection_pool::ConnectionPool;

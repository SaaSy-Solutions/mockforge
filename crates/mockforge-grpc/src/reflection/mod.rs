//! Reflection-based gRPC proxy implementation
//!
//! This module provides functionality to dynamically proxy gRPC requests
//! to arbitrary services using reflection.

pub mod cache;
pub mod client;
pub mod config;
pub mod connection_pool;
pub mod descriptor;
pub mod error_handling;
pub mod metrics;
pub mod mock_proxy;
pub mod proxy;
pub mod smart_mock_generator;

pub use client::ReflectionClient;
pub use config::ProxyConfig;
pub use connection_pool::ConnectionPool;
pub use mock_proxy::MockReflectionProxy;
pub use proxy::ReflectionProxy;

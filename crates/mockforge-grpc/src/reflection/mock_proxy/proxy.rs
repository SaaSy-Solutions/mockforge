//! Core proxy functionality and state management
//!
//! This module provides the main MockReflectionProxy struct and core proxy functionality.

use crate::dynamic::ServiceRegistry;
use crate::reflection::{
    cache::DescriptorCache,
    config::ProxyConfig,
    connection_pool::ConnectionPool,
    smart_mock_generator::{SmartMockConfig, SmartMockGenerator},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tonic::Status;
use tracing::debug;

/// Guard that automatically decrements the active connection counter when dropped
pub struct ConnectionGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A mock-enabled reflection proxy that serves mock responses
pub struct MockReflectionProxy {
    /// Cache of service and method descriptors
    pub(crate) cache: DescriptorCache,
    /// Proxy configuration
    pub(crate) config: ProxyConfig,
    /// Timeout for requests
    pub(crate) timeout_duration: Duration,
    /// Connection pool for gRPC channels
    #[allow(dead_code)]
    pub(crate) connection_pool: ConnectionPool,
    /// Registry of dynamic services for mock responses
    pub(crate) service_registry: Arc<ServiceRegistry>,
    /// Smart mock data generator for intelligent field population
    pub(crate) smart_generator: Arc<Mutex<SmartMockGenerator>>,
    /// Counter for active connections/requests
    pub(crate) active_connections: Arc<AtomicUsize>,
    /// Counter for total requests processed
    pub(crate) total_requests: Arc<AtomicUsize>,
}

impl MockReflectionProxy {
    /// Create a new mock reflection proxy
    pub async fn new(
        config: ProxyConfig,
        service_registry: Arc<ServiceRegistry>,
    ) -> Result<Self, Status> {
        debug!(
            "Creating mock reflection proxy with {} services",
            service_registry.service_names().len()
        );

        let cache = DescriptorCache::new();

        // Populate cache from service registry's descriptor pool
        cache.populate_from_pool(Some(service_registry.descriptor_pool())).await;

        let connection_pool = ConnectionPool::new();

        let timeout_duration = Duration::from_secs(config.request_timeout_seconds);

        let smart_generator = Arc::new(Mutex::new(SmartMockGenerator::new(SmartMockConfig {
            field_name_inference: true,
            use_faker: true,
            field_overrides: std::collections::HashMap::new(),
            service_profiles: std::collections::HashMap::new(),
            max_depth: 3,
            seed: config.mock_seed,
            deterministic: false,
        })));

        Ok(Self {
            cache,
            config,
            timeout_duration,
            connection_pool,
            service_registry,
            smart_generator,
            active_connections: Arc::new(AtomicUsize::new(0)),
            total_requests: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Get the proxy configuration
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }

    /// Get the descriptor cache
    pub fn cache(&self) -> &DescriptorCache {
        &self.cache
    }

    /// Get the service registry
    pub fn service_registry(&self) -> &Arc<ServiceRegistry> {
        &self.service_registry
    }

    /// Get the list of service names
    pub fn service_names(&self) -> Vec<String> {
        self.service_registry.service_names()
    }

    /// Get the smart mock generator
    pub fn smart_generator(&self) -> &Arc<Mutex<SmartMockGenerator>> {
        &self.smart_generator
    }

    /// Check if a service method should be mocked
    pub fn should_mock_service_method(&self, service_name: &str, _method_name: &str) -> bool {
        // Check if service is in registry
        self.service_registry.get(service_name).is_some()
    }

    /// Get the timeout duration for requests
    pub fn timeout_duration(&self) -> Duration {
        self.timeout_duration
    }

    /// Update the proxy configuration
    pub fn update_config(&mut self, config: ProxyConfig) {
        self.config = config;
        // Update timeout
        self.timeout_duration = Duration::from_secs(self.config.request_timeout_seconds);
    }

    /// Create a connection guard that tracks active connections
    pub fn track_connection(&self) -> ConnectionGuard {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        ConnectionGuard {
            counter: self.active_connections.clone(),
        }
    }

    /// Get statistics about the proxy
    pub async fn get_stats(&self) -> ProxyStats {
        ProxyStats {
            cached_services: self.cache.service_count().await,
            cached_methods: self.cache.method_count().await,
            registered_services: self.service_registry.service_names().len(),
            total_requests: self.total_requests.load(Ordering::Relaxed) as u64,
            active_connections: self.active_connections.load(Ordering::Relaxed),
        }
    }
}

/// Statistics about the proxy
#[derive(Debug, Clone)]
pub struct ProxyStats {
    pub cached_services: usize,
    pub cached_methods: usize,
    pub registered_services: usize,
    pub total_requests: u64,
    pub active_connections: usize,
}

impl Clone for MockReflectionProxy {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            config: self.config.clone(),
            timeout_duration: self.timeout_duration,
            connection_pool: self.connection_pool.clone(),
            service_registry: self.service_registry.clone(),
            smart_generator: self.smart_generator.clone(),
            active_connections: self.active_connections.clone(),
            total_requests: self.total_requests.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

//! Cache implementation for service and method descriptors

use crate::reflection::descriptor::ServiceDescriptorCache;
use prost_reflect::{DescriptorPool, ServiceDescriptor};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::Status;
use tracing::{debug, trace};

/// A thread-safe cache of service descriptors
#[derive(Debug, Clone)]
pub struct DescriptorCache {
    /// The underlying cache protected by a RwLock
    cache: Arc<RwLock<ServiceDescriptorCache>>,
}

impl DescriptorCache {
    /// Create a new descriptor cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(ServiceDescriptorCache::new())),
        }
    }

    /// Add a service descriptor to the cache
    pub async fn add_service(&self, service: ServiceDescriptor) {
        let service_name = service.full_name().to_string();
        trace!("Adding service to cache: {}", service_name);

        let mut cache = self.cache.write().await;
        cache.add_service(service);

        debug!("Added service to cache: {}", service_name);
    }

    /// Get a method descriptor from the cache
    pub async fn get_method(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<prost_reflect::MethodDescriptor, Status> {
        trace!("Getting method from cache: {}::{}", service_name, method_name);

        let cache = self.cache.read().await;
        cache
            .get_method(service_name, method_name)
            .map(|m| m.clone())
    }

    /// Get a service descriptor from the cache with proper error handling
    pub async fn get_service(&self, service_name: &str) -> Result<ServiceDescriptor, Status> {
        trace!("Getting service from cache: {}", service_name);

        let cache = self.cache.read().await;
        cache
            .get_service_with_error(service_name)
            .map(|s| s.clone())
    }

    /// Check if a service exists in the cache
    pub async fn contains_service(&self, service_name: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_service(service_name)
    }

    /// Check if a method exists in the cache
    pub async fn contains_method(&self, service_name: &str, method_name: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_method(service_name, method_name)
    }

    /// Populate the cache from a descriptor pool
    pub async fn populate_from_pool(&self, pool: &DescriptorPool) {
        trace!("Populating cache from descriptor pool");

        let mut cache = self.cache.write().await;
        for service in pool.services() {
            cache.add_service(service);
        }

        debug!("Populated cache with {} services", pool.services().count());
    }
}

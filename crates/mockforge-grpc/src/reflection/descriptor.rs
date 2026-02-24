//! Service and method descriptor handling

use prost_reflect::{MethodDescriptor, ServiceDescriptor};
use std::collections::HashMap;
use tonic::Status;

/// A cache of service descriptors
#[derive(Debug, Clone)]
pub struct ServiceDescriptorCache {
    /// Map of service names to service descriptors
    services: HashMap<String, ServiceDescriptor>,
    /// Map of (service, method) pairs to method descriptors
    methods: HashMap<(String, String), MethodDescriptor>,
}

impl Default for ServiceDescriptorCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceDescriptorCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    /// Add a service descriptor to the cache
    pub fn add_service(&mut self, service: ServiceDescriptor) {
        let service_name = service.full_name().to_string();
        self.services.insert(service_name.clone(), service.clone());

        // Cache all methods for this service
        for method in service.methods() {
            let method_name = method.name().to_string();
            self.methods.insert((service_name.clone(), method_name), method);
        }
    }

    /// Get a service descriptor by name
    pub fn get_service(&self, service_name: &str) -> Option<&ServiceDescriptor> {
        self.services.get(service_name)
    }

    /// Get a service descriptor by name with proper error handling
    pub fn get_service_with_error(&self, service_name: &str) -> Result<&ServiceDescriptor, Status> {
        self.services.get(service_name).ok_or_else(|| {
            Status::not_found(format!("Service '{}' not found in descriptor cache", service_name))
        })
    }

    /// Get a method descriptor by service and method name
    pub fn get_method(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<&MethodDescriptor, Status> {
        self.methods
            .get(&(service_name.to_string(), method_name.to_string()))
            .ok_or_else(|| {
                Status::not_found(format!(
                    "Method {} not found in service {}",
                    method_name, service_name
                ))
            })
    }

    /// Check if a service exists in the cache
    pub fn contains_service(&self, service_name: &str) -> bool {
        self.services.contains_key(service_name)
    }

    /// Check if a method exists in the cache
    pub fn contains_method(&self, service_name: &str, method_name: &str) -> bool {
        self.methods.contains_key(&(service_name.to_string(), method_name.to_string()))
    }

    /// Get the number of cached services
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Get the number of cached methods
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        // Importing super::* ensures this module's types and imports are valid
    }
}

//! Protocol registry for managing protocol handlers and configurations
//!
//! This module provides a centralized registry for protocol handlers, enabling
//! dynamic protocol support and configuration management.

use crate::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{Protocol, ProtocolRequest, ProtocolResponse, SpecRegistry};

/// Trait for protocol-specific handlers
#[async_trait::async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Get the protocol this handler supports
    fn protocol(&self) -> Protocol;

    /// Check if this handler is enabled
    fn is_enabled(&self) -> bool;

    /// Enable or disable this protocol handler
    fn set_enabled(&mut self, enabled: bool);

    /// Get the spec registry for this protocol if available
    fn spec_registry(&self) -> Option<&dyn SpecRegistry>;

    /// Handle an incoming request and generate a response
    async fn handle_request(&self, request: ProtocolRequest) -> Result<ProtocolResponse>;

    /// Validate that the handler is properly configured
    fn validate_configuration(&self) -> Result<()>;

    /// Get handler-specific configuration as key-value pairs
    fn get_configuration(&self) -> HashMap<String, String>;

    /// Update handler configuration from key-value pairs
    fn update_configuration(&mut self, config: HashMap<String, String>) -> Result<()>;
}

/// Protocol registry for managing multiple protocol handlers
pub struct ProtocolRegistry {
    handlers: HashMap<Protocol, Arc<dyn ProtocolHandler>>,
    enabled_protocols: HashSet<Protocol>,
}

impl ProtocolRegistry {
    /// Create a new empty protocol registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            enabled_protocols: HashSet::new(),
        }
    }

    /// Register a protocol handler
    pub fn register_handler(&mut self, handler: Arc<dyn ProtocolHandler>) -> Result<()> {
        let protocol = handler.protocol();

        if handler.is_enabled() {
            self.enabled_protocols.insert(protocol);
        }

        self.handlers.insert(protocol, handler);
        Ok(())
    }

    /// Unregister a protocol handler
    pub fn unregister_handler(&mut self, protocol: Protocol) -> Result<()> {
        if self.handlers.remove(&protocol).is_some() {
            self.enabled_protocols.remove(&protocol);
            Ok(())
        } else {
            Err(crate::Error::protocol_not_found(protocol.to_string()))
        }
    }

    /// Get a protocol handler by protocol type
    pub fn get_handler(&self, protocol: Protocol) -> Option<&Arc<dyn ProtocolHandler>> {
        self.handlers.get(&protocol)
    }

    /// Check if a protocol is enabled
    pub fn is_protocol_enabled(&self, protocol: Protocol) -> bool {
        self.enabled_protocols.contains(&protocol)
    }

    /// Enable a protocol
    pub fn enable_protocol(&mut self, protocol: Protocol) -> Result<()> {
        if self.handlers.contains_key(&protocol) {
            self.enabled_protocols.insert(protocol);
            Ok(())
        } else {
            Err(crate::Error::protocol_not_found(protocol.to_string()))
        }
    }

    /// Disable a protocol
    pub fn disable_protocol(&mut self, protocol: Protocol) -> Result<()> {
        if self.handlers.contains_key(&protocol) {
            self.enabled_protocols.remove(&protocol);
            Ok(())
        } else {
            Err(crate::Error::protocol_not_found(protocol.to_string()))
        }
    }

    /// Get all registered protocols
    pub fn registered_protocols(&self) -> Vec<Protocol> {
        self.handlers.keys().cloned().collect()
    }

    /// Get all enabled protocols
    pub fn enabled_protocols(&self) -> Vec<Protocol> {
        self.enabled_protocols.iter().cloned().collect()
    }

    /// Handle a request using the appropriate protocol handler
    pub async fn handle_request(&self, request: ProtocolRequest) -> Result<ProtocolResponse> {
        let protocol = request.protocol;

        if !self.is_protocol_enabled(protocol) {
            return Err(crate::Error::protocol_disabled(protocol.to_string()));
        }

        if let Some(handler) = self.get_handler(protocol) {
            handler.handle_request(request).await
        } else {
            Err(crate::Error::protocol_not_found(protocol.to_string()))
        }
    }

    /// Validate all registered handlers
    pub fn validate_all_handlers(&self) -> Result<()> {
        for (protocol, handler) in &self.handlers {
            if let Err(e) = handler.validate_configuration() {
                return Err(crate::Error::protocol_validation_error(
                    protocol.to_string(),
                    e.to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Get configuration for all handlers
    pub fn get_all_configurations(&self) -> HashMap<Protocol, HashMap<String, String>> {
        self.handlers
            .iter()
            .map(|(protocol, handler)| (*protocol, handler.get_configuration()))
            .collect()
    }

    /// Update configuration for a specific protocol
    pub fn update_protocol_configuration(
        &mut self,
        protocol: Protocol,
        _config: HashMap<String, String>,
    ) -> Result<()> {
        // Note: Configuration updates are not supported for handlers stored in Arc
        // This would require a different design where handlers can be mutated
        Err(crate::Error::generic(format!(
            "Configuration updates not supported for protocol: {}",
            protocol
        )))
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockProtocolHandler {
        protocol: Protocol,
        enabled: Mutex<bool>,
        config: Mutex<HashMap<String, String>>,
    }

    impl MockProtocolHandler {
        fn new(protocol: Protocol) -> Self {
            Self {
                protocol,
                enabled: Mutex::new(true),
                config: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl ProtocolHandler for MockProtocolHandler {
        fn protocol(&self) -> Protocol {
            self.protocol
        }

        fn is_enabled(&self) -> bool {
            *self.enabled.lock().unwrap()
        }

        fn set_enabled(&mut self, enabled: bool) {
            *self.enabled.lock().unwrap() = enabled;
        }

        fn spec_registry(&self) -> Option<&dyn SpecRegistry> {
            None
        }

        async fn handle_request(&self, _request: ProtocolRequest) -> Result<ProtocolResponse> {
            Ok(ProtocolResponse {
                status: super::super::ResponseStatus::HttpStatus(200),
                metadata: HashMap::new(),
                body: b"mock response".to_vec(),
                content_type: "text/plain".to_string(),
            })
        }

        fn validate_configuration(&self) -> Result<()> {
            Ok(())
        }

        fn get_configuration(&self) -> HashMap<String, String> {
            self.config.lock().unwrap().clone()
        }

        fn update_configuration(&mut self, config: HashMap<String, String>) -> Result<()> {
            *self.config.lock().unwrap() = config;
            Ok(())
        }
    }

    #[test]
    fn test_protocol_registry_creation() {
        let registry = ProtocolRegistry::new();
        assert_eq!(registry.registered_protocols().len(), 0);
        assert_eq!(registry.enabled_protocols().len(), 0);
    }

    #[test]
    fn test_register_handler() {
        let mut registry = ProtocolRegistry::new();
        let handler = Arc::new(MockProtocolHandler::new(Protocol::Http));

        assert!(registry.register_handler(handler).is_ok());
        assert_eq!(registry.registered_protocols(), vec![Protocol::Http]);
        assert_eq!(registry.enabled_protocols(), vec![Protocol::Http]);
    }

    #[test]
    fn test_enable_disable_protocol() {
        let mut registry = ProtocolRegistry::new();
        let handler = Arc::new(MockProtocolHandler::new(Protocol::Http));
        registry.register_handler(handler).unwrap();

        assert!(registry.is_protocol_enabled(Protocol::Http));

        registry.disable_protocol(Protocol::Http).unwrap();
        assert!(!registry.is_protocol_enabled(Protocol::Http));

        registry.enable_protocol(Protocol::Http).unwrap();
        assert!(registry.is_protocol_enabled(Protocol::Http));
    }

    #[test]
    fn test_handle_request() {
        let mut registry = ProtocolRegistry::new();
        let handler = Arc::new(MockProtocolHandler::new(Protocol::Http));
        registry.register_handler(handler).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            ..Default::default()
        };

        let result = futures::executor::block_on(registry.handle_request(request));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.body, b"mock response");
    }

    #[test]
    fn test_handle_request_disabled_protocol() {
        let mut registry = ProtocolRegistry::new();
        let handler = Arc::new(MockProtocolHandler::new(Protocol::Http));
        registry.register_handler(handler).unwrap();
        registry.disable_protocol(Protocol::Http).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            ..Default::default()
        };

        let result = futures::executor::block_on(registry.handle_request(request));
        assert!(result.is_err());
    }
}

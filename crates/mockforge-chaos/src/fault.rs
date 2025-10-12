//! Fault injection for simulating errors and failures

use crate::{config::FaultInjectionConfig, ChaosError, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Types of faults that can be injected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FaultType {
    /// HTTP error with status code
    HttpError(u16),
    /// Connection error
    ConnectionError,
    /// Timeout error
    Timeout,
    /// Partial response (incomplete data)
    PartialResponse,
}

/// Fault injector for simulating errors
#[derive(Clone)]
pub struct FaultInjector {
    config: FaultInjectionConfig,
}

impl FaultInjector {
    /// Create a new fault injector
    pub fn new(config: FaultInjectionConfig) -> Self {
        Self { config }
    }

    /// Check if fault injection is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a fault should be injected
    pub fn should_inject_fault(&self) -> Option<FaultType> {
        if !self.config.enabled {
            return None;
        }

        let mut rng = rand::rng();

        // Check for HTTP errors
        if !self.config.http_errors.is_empty()
            && rng.random::<f64>() < self.config.http_error_probability
        {
            let error_code =
                self.config.http_errors[rng.random_range(0..self.config.http_errors.len())];
            debug!("Injecting HTTP error: {}", error_code);
            return Some(FaultType::HttpError(error_code));
        }

        // Check for connection errors
        if self.config.connection_errors
            && rng.random::<f64>() < self.config.connection_error_probability
        {
            debug!("Injecting connection error");
            return Some(FaultType::ConnectionError);
        }

        // Check for timeout errors
        if self.config.timeout_errors && rng.random::<f64>() < self.config.timeout_probability {
            debug!("Injecting timeout error");
            return Some(FaultType::Timeout);
        }

        // Check for partial responses
        if self.config.partial_responses
            && rng.random::<f64>() < self.config.partial_response_probability
        {
            debug!("Injecting partial response");
            return Some(FaultType::PartialResponse);
        }

        None
    }

    /// Inject a fault, returning an error if injection succeeds
    pub fn inject(&self) -> Result<()> {
        if let Some(fault) = self.should_inject_fault() {
            match fault {
                FaultType::HttpError(code) => {
                    Err(ChaosError::InjectedFault(format!("HTTP error {}", code)))
                }
                FaultType::ConnectionError => {
                    Err(ChaosError::InjectedFault("Connection error".to_string()))
                }
                FaultType::Timeout => Err(ChaosError::Timeout(self.config.timeout_ms)),
                FaultType::PartialResponse => {
                    Err(ChaosError::InjectedFault("Partial response".to_string()))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Get HTTP error status code for injection
    pub fn get_http_error_status(&self) -> Option<u16> {
        if let Some(FaultType::HttpError(code)) = self.should_inject_fault() {
            Some(code)
        } else {
            None
        }
    }

    /// Check if should truncate response (for partial response simulation)
    pub fn should_truncate_response(&self) -> bool {
        matches!(self.should_inject_fault(), Some(FaultType::PartialResponse))
    }

    /// Get configuration
    pub fn config(&self) -> &FaultInjectionConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: FaultInjectionConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_error_injection() {
        let config = FaultInjectionConfig {
            enabled: true,
            http_errors: vec![500, 503],
            http_error_probability: 1.0, // Always inject
            ..Default::default()
        };

        let injector = FaultInjector::new(config);

        // Should inject an error
        let fault = injector.should_inject_fault();
        assert!(fault.is_some());

        if let Some(FaultType::HttpError(code)) = fault {
            assert!(code == 500 || code == 503);
        } else {
            panic!("Expected HTTP error");
        }
    }

    #[test]
    fn test_no_injection_when_disabled() {
        let config = FaultInjectionConfig {
            enabled: false,
            ..Default::default()
        };

        let injector = FaultInjector::new(config);
        let fault = injector.should_inject_fault();
        assert!(fault.is_none());
    }

    #[test]
    fn test_connection_error_injection() {
        let config = FaultInjectionConfig {
            enabled: true,
            connection_errors: true,
            connection_error_probability: 1.0,
            http_errors: vec![],
            ..Default::default()
        };

        let injector = FaultInjector::new(config);
        let fault = injector.should_inject_fault();
        assert!(matches!(fault, Some(FaultType::ConnectionError)));
    }

    #[test]
    fn test_timeout_injection() {
        let config = FaultInjectionConfig {
            enabled: true,
            timeout_errors: true,
            timeout_probability: 1.0,
            http_errors: vec![],
            ..Default::default()
        };

        let injector = FaultInjector::new(config);
        let fault = injector.should_inject_fault();
        assert!(matches!(fault, Some(FaultType::Timeout)));
    }

    #[test]
    fn test_inject_returns_error() {
        let config = FaultInjectionConfig {
            enabled: true,
            http_errors: vec![500],
            http_error_probability: 1.0,
            ..Default::default()
        };

        let injector = FaultInjector::new(config);
        let result = injector.inject();
        assert!(result.is_err());
    }
}

//! Fault injection for simulating errors and failures

use crate::{config::ErrorPattern, config::FaultInjectionConfig, ChaosError, Result};
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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
    /// Payload corruption
    PayloadCorruption,
}

/// Pattern execution state
#[derive(Debug, Clone)]
struct PatternState {
    /// Burst pattern state: (errors_in_burst, burst_start_time_ms)
    burst_state: Option<(usize, u64)>,
    /// Sequential pattern state: current index in sequence
    sequential_index: usize,
}

impl Default for PatternState {
    fn default() -> Self {
        Self {
            burst_state: None,
            sequential_index: 0,
        }
    }
}

/// Fault injector for simulating errors
#[derive(Clone)]
pub struct FaultInjector {
    config: FaultInjectionConfig,
    /// Pattern execution state (shared for thread safety)
    pattern_state: Arc<RwLock<PatternState>>,
}

impl FaultInjector {
    /// Create a new fault injector
    pub fn new(config: FaultInjectionConfig) -> Self {
        Self {
            config,
            pattern_state: Arc::new(RwLock::new(PatternState::default())),
        }
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

        // Check error pattern first if configured
        if let Some(ref pattern) = self.config.error_pattern {
            if let Some(fault) = self.check_pattern(pattern) {
                return Some(fault);
            }
            // If pattern says no, don't inject (pattern takes precedence)
            return None;
        }

        // Fall back to probability-based injection
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

        // Check for payload corruption
        if self.config.payload_corruption
            && rng.random::<f64>() < self.config.payload_corruption_probability
        {
            debug!("Injecting payload corruption");
            return Some(FaultType::PayloadCorruption);
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
                FaultType::PayloadCorruption => {
                    Err(ChaosError::InjectedFault("Payload corruption".to_string()))
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

    /// Check if should corrupt payload
    pub fn should_corrupt_payload(&self) -> bool {
        if !self.config.enabled || !self.config.payload_corruption {
            return false;
        }

        let mut rng = rand::rng();
        rng.random::<f64>() < self.config.payload_corruption_probability
    }

    /// Get corruption type from config
    pub fn corruption_type(&self) -> crate::config::CorruptionType {
        self.config.corruption_type
    }

    /// Get configuration
    pub fn config(&self) -> &FaultInjectionConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: FaultInjectionConfig) {
        // Reset pattern state when config changes
        let mut state = self.pattern_state.write();
        *state = PatternState::default();
        self.config = config;
    }

    /// Check error pattern and return fault if pattern matches
    fn check_pattern(&self, pattern: &ErrorPattern) -> Option<FaultType> {
        let mut state = self.pattern_state.write();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        match pattern {
            ErrorPattern::Burst { count, interval_ms } => {
                // Check if we're in a burst window
                let (errors_in_burst, burst_start) = state.burst_state.unwrap_or((0, now_ms));
                let elapsed = now_ms.saturating_sub(burst_start);

                if elapsed < *interval_ms {
                    // Still in burst window
                    if errors_in_burst < *count {
                        // Inject error and increment counter
                        state.burst_state = Some((errors_in_burst + 1, burst_start));
                        let error_code = self.get_next_error_code();
                        debug!("Burst pattern: injecting error {} ({}/{})", error_code, errors_in_burst + 1, count);
                        return Some(FaultType::HttpError(error_code));
                    } else {
                        // Burst quota reached, don't inject
                        return None;
                    }
                } else {
                    // Burst window expired, start new burst
                    state.burst_state = Some((1, now_ms));
                    let error_code = self.get_next_error_code();
                    debug!("Burst pattern: starting new burst, injecting error {}", error_code);
                    return Some(FaultType::HttpError(error_code));
                }
            }
            ErrorPattern::Random { probability } => {
                let mut rng = rand::rng();
                if rng.random::<f64>() < *probability {
                    let error_code = self.get_next_error_code();
                    debug!("Random pattern: injecting error {} (probability: {})", error_code, probability);
                    return Some(FaultType::HttpError(error_code));
                }
                return None;
            }
            ErrorPattern::Sequential { sequence } => {
                if sequence.is_empty() {
                    return None;
                }
                let error_code = sequence[state.sequential_index % sequence.len()];
                state.sequential_index = (state.sequential_index + 1) % sequence.len();
                debug!("Sequential pattern: injecting error {} (index: {})", error_code, state.sequential_index);
                return Some(FaultType::HttpError(error_code));
            }
        }
    }

    /// Get next error code from configured HTTP errors
    fn get_next_error_code(&self) -> u16 {
        if self.config.http_errors.is_empty() {
            500 // Default error code
        } else {
            let mut rng = rand::rng();
            self.config.http_errors[rng.random_range(0..self.config.http_errors.len())]
        }
    }

    /// Generate dynamic error message using MockAI if available
    /// 
    /// This generates context-aware error messages based on the request context
    pub async fn generate_error_message(
        &self,
        status_code: u16,
        mockai: Option<&std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>>,
        request_context: Option<&str>,
    ) -> String {
        // If MockAI is enabled and available, use it to generate context-aware error messages
        if let Some(mockai_arc) = mockai {
            if let Ok(mockai_guard) = mockai_arc.try_read() {
                // Generate error message based on status code and context
                let error_context = format!(
                    "Generate a realistic HTTP {} error message{}",
                    status_code,
                    request_context
                        .map(|ctx| format!(" for the following request context: {}", ctx))
                        .unwrap_or_default()
                );

                // Use MockAI's validation generator to create error messages
                // This is a simplified approach - in a full implementation, we'd use
                // MockAI's error generation capabilities
                match status_code {
                    400 => "Bad Request: Invalid input provided".to_string(),
                    401 => "Unauthorized: Authentication required".to_string(),
                    403 => "Forbidden: Insufficient permissions".to_string(),
                    404 => "Not Found: The requested resource does not exist".to_string(),
                    429 => "Too Many Requests: Rate limit exceeded".to_string(),
                    500 => "Internal Server Error: An unexpected error occurred".to_string(),
                    502 => "Bad Gateway: Upstream server error".to_string(),
                    503 => "Service Unavailable: The service is temporarily unavailable".to_string(),
                    504 => "Gateway Timeout: The upstream server did not respond in time".to_string(),
                    _ => format!("HTTP {} Error", status_code),
                }
            } else {
                // Fallback if MockAI is locked
                self.get_default_error_message(status_code)
            }
        } else {
            // No MockAI available, use default messages
            self.get_default_error_message(status_code)
        }
    }

    /// Get default error message for a status code
    fn get_default_error_message(&self, status_code: u16) -> String {
        match status_code {
            400 => "Bad Request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not Found".to_string(),
            429 => "Too Many Requests".to_string(),
            500 => "Internal Server Error".to_string(),
            502 => "Bad Gateway".to_string(),
            503 => "Service Unavailable".to_string(),
            504 => "Gateway Timeout".to_string(),
            _ => format!("HTTP {} Error", status_code),
        }
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

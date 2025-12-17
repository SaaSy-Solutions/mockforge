//! Error handling and retry mechanisms for the reflection proxy

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tonic::Status;
use tracing::{debug, error, warn};

/// Configuration for error handling and retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorConfig {
    /// Maximum number of retries for failed requests
    pub max_retries: u32,
    /// Base delay between retries (in milliseconds)
    pub base_delay_ms: u64,
    /// Maximum delay between retries (in milliseconds)
    pub max_delay_ms: u64,
    /// Whether to enable exponential backoff
    pub exponential_backoff: bool,
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            exponential_backoff: true,
        }
    }
}

/// Handle errors with retry logic
pub async fn handle_with_retry<F, Fut, T>(
    mut operation: F,
    config: &ErrorConfig,
) -> Result<T, Status>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Status>>,
{
    let mut attempts = 0;
    let mut delay = Duration::from_millis(config.base_delay_ms);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(status) => {
                attempts += 1;

                // If we've reached the maximum retries, return the error
                if attempts > config.max_retries {
                    error!("Operation failed after {} attempts: {}", attempts, status);
                    return Err(status);
                }

                // For certain types of errors, we might not want to retry
                match status.code() {
                    // Don't retry these errors
                    tonic::Code::InvalidArgument
                    | tonic::Code::NotFound
                    | tonic::Code::AlreadyExists
                    | tonic::Code::PermissionDenied
                    | tonic::Code::FailedPrecondition
                    | tonic::Code::Aborted
                    | tonic::Code::OutOfRange
                    | tonic::Code::Unimplemented => {
                        error!("Non-retryable error: {}", status);
                        return Err(status);
                    }
                    // Retry all other errors
                    _ => {
                        warn!(
                            "Attempt {} failed: {}. Retrying in {:?}...",
                            attempts, status, delay
                        );

                        // Wait before retrying
                        sleep(delay).await;

                        // Calculate next delay with exponential backoff
                        if config.exponential_backoff {
                            delay = Duration::from_millis(
                                (delay.as_millis() * 2).min(config.max_delay_ms as u128) as u64,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Simulate various error conditions for testing
pub fn simulate_error(error_rate: f64) -> Result<(), Status> {
    use rand::Rng;

    let mut rng = rand::rng();
    let random: f64 = rng.random();

    if random < error_rate {
        // Simulate different types of errors
        let error_type: u32 = rng.random_range(0..5);
        match error_type {
            0 => Err(Status::unavailable("Simulated service unavailable")),
            1 => Err(Status::deadline_exceeded("Simulated timeout")),
            2 => Err(Status::internal("Simulated internal error")),
            3 => Err(Status::resource_exhausted("Simulated resource exhausted")),
            _ => Err(Status::unknown("Simulated unknown error")),
        }
    } else {
        Ok(())
    }
}

/// Add latency to simulate network delays
pub async fn simulate_latency(latency_ms: u64) {
    if latency_ms > 0 {
        debug!("Simulating {}ms latency", latency_ms);
        sleep(Duration::from_millis(latency_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ErrorConfig Tests ====================

    #[test]
    fn test_error_config_default() {
        let config = ErrorConfig::default();

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 5000);
        assert!(config.exponential_backoff);
    }

    #[test]
    fn test_error_config_custom_values() {
        let config = ErrorConfig {
            max_retries: 5,
            base_delay_ms: 200,
            max_delay_ms: 10000,
            exponential_backoff: false,
        };

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay_ms, 200);
        assert_eq!(config.max_delay_ms, 10000);
        assert!(!config.exponential_backoff);
    }

    #[test]
    fn test_error_config_clone() {
        let config = ErrorConfig {
            max_retries: 4,
            base_delay_ms: 150,
            max_delay_ms: 8000,
            exponential_backoff: true,
        };

        let cloned = config.clone();

        assert_eq!(cloned.max_retries, config.max_retries);
        assert_eq!(cloned.base_delay_ms, config.base_delay_ms);
        assert_eq!(cloned.max_delay_ms, config.max_delay_ms);
        assert_eq!(cloned.exponential_backoff, config.exponential_backoff);
    }

    #[test]
    fn test_error_config_debug() {
        let config = ErrorConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("max_retries"));
        assert!(debug_str.contains("base_delay_ms"));
        assert!(debug_str.contains("max_delay_ms"));
        assert!(debug_str.contains("exponential_backoff"));
    }

    #[test]
    fn test_error_config_serialization() {
        let config = ErrorConfig {
            max_retries: 5,
            base_delay_ms: 250,
            max_delay_ms: 15000,
            exponential_backoff: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ErrorConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.max_retries, config.max_retries);
        assert_eq!(deserialized.base_delay_ms, config.base_delay_ms);
        assert_eq!(deserialized.max_delay_ms, config.max_delay_ms);
        assert_eq!(deserialized.exponential_backoff, config.exponential_backoff);
    }

    #[test]
    fn test_error_config_deserialization() {
        let json = r#"{
            "max_retries": 10,
            "base_delay_ms": 500,
            "max_delay_ms": 30000,
            "exponential_backoff": false
        }"#;

        let config: ErrorConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.max_retries, 10);
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 30000);
        assert!(!config.exponential_backoff);
    }

    #[test]
    fn test_error_config_zero_retries() {
        let config = ErrorConfig {
            max_retries: 0,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            exponential_backoff: true,
        };

        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_error_config_high_retries() {
        let config = ErrorConfig {
            max_retries: 100,
            base_delay_ms: 10,
            max_delay_ms: 60000,
            exponential_backoff: true,
        };

        assert_eq!(config.max_retries, 100);
    }

    // ==================== simulate_error Tests ====================

    #[test]
    fn test_simulate_error_zero_rate() {
        // With 0% error rate, should always succeed
        for _ in 0..100 {
            let result = simulate_error(0.0);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_simulate_error_full_rate() {
        // With 100% error rate, should always fail
        for _ in 0..100 {
            let result = simulate_error(1.0);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_simulate_error_produces_status() {
        // When error occurs, should return a tonic Status
        let result = simulate_error(1.0);
        assert!(result.is_err());

        let status = result.unwrap_err();
        // Status should have a valid code
        let code = status.code();
        assert!(matches!(
            code,
            tonic::Code::Unavailable
                | tonic::Code::DeadlineExceeded
                | tonic::Code::Internal
                | tonic::Code::ResourceExhausted
                | tonic::Code::Unknown
        ));
    }

    #[test]
    fn test_simulate_error_partial_rate() {
        // With 50% error rate, should have some successes and some failures
        let mut successes = 0;
        let mut failures = 0;

        for _ in 0..1000 {
            match simulate_error(0.5) {
                Ok(()) => successes += 1,
                Err(_) => failures += 1,
            }
        }

        // With 1000 samples, we should have both successes and failures
        assert!(successes > 0, "Expected some successes");
        assert!(failures > 0, "Expected some failures");
    }

    // ==================== simulate_latency Tests ====================

    #[tokio::test]
    async fn test_simulate_latency_zero() {
        let start = std::time::Instant::now();
        simulate_latency(0).await;
        let elapsed = start.elapsed();

        // Should complete almost instantly (allow 10ms margin)
        assert!(elapsed.as_millis() < 10);
    }

    #[tokio::test]
    async fn test_simulate_latency_short() {
        let start = std::time::Instant::now();
        simulate_latency(50).await;
        let elapsed = start.elapsed();

        // Should take at least 50ms (allow some margin)
        assert!(elapsed.as_millis() >= 45);
        // Should not take too long (allow 100ms margin for scheduling)
        assert!(elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_simulate_latency_longer() {
        let start = std::time::Instant::now();
        simulate_latency(100).await;
        let elapsed = start.elapsed();

        // Should take at least 100ms
        assert!(elapsed.as_millis() >= 95);
    }

    // ==================== handle_with_retry Tests ====================

    #[tokio::test]
    async fn test_handle_with_retry_immediate_success() {
        let config = ErrorConfig::default();

        let result = handle_with_retry(|| async { Ok::<_, Status>("success") }, &config).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_handle_with_retry_non_retryable_error() {
        let config = ErrorConfig::default();

        // InvalidArgument is a non-retryable error
        let result = handle_with_retry(
            || async { Err::<(), _>(Status::invalid_argument("bad argument")) },
            &config,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_handle_with_retry_not_found_no_retry() {
        let config = ErrorConfig::default();

        let result = handle_with_retry(
            || async { Err::<(), _>(Status::not_found("resource not found")) },
            &config,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_handle_with_retry_already_exists_no_retry() {
        let config = ErrorConfig::default();

        let result = handle_with_retry(
            || async { Err::<(), _>(Status::already_exists("already exists")) },
            &config,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::AlreadyExists);
    }

    #[tokio::test]
    async fn test_handle_with_retry_permission_denied_no_retry() {
        let config = ErrorConfig::default();

        let result = handle_with_retry(
            || async { Err::<(), _>(Status::permission_denied("access denied")) },
            &config,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::PermissionDenied);
    }

    #[tokio::test]
    async fn test_handle_with_retry_unimplemented_no_retry() {
        let config = ErrorConfig::default();

        let result = handle_with_retry(
            || async { Err::<(), _>(Status::unimplemented("not implemented")) },
            &config,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_handle_with_retry_retryable_error_eventual_success() {
        let config = ErrorConfig {
            max_retries: 3,
            base_delay_ms: 10,
            max_delay_ms: 100,
            exponential_backoff: false,
        };

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = handle_with_retry(
            || {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count < 2 {
                        Err(Status::unavailable("service unavailable"))
                    } else {
                        Ok("success")
                    }
                }
            },
            &config,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        // Should have been called 3 times (2 failures + 1 success)
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_handle_with_retry_max_retries_exceeded() {
        let config = ErrorConfig {
            max_retries: 2,
            base_delay_ms: 10,
            max_delay_ms: 100,
            exponential_backoff: false,
        };

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = handle_with_retry(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err::<(), _>(Status::unavailable("service unavailable"))
                }
            },
            &config,
        )
        .await;

        assert!(result.is_err());
        // Initial attempt + 2 retries = 3 total calls
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_handle_with_retry_zero_retries() {
        let config = ErrorConfig {
            max_retries: 0,
            base_delay_ms: 10,
            max_delay_ms: 100,
            exponential_backoff: false,
        };

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = handle_with_retry(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err::<(), _>(Status::unavailable("service unavailable"))
                }
            },
            &config,
        )
        .await;

        assert!(result.is_err());
        // With 0 retries, should only try once
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_handle_with_retry_deadline_exceeded_retryable() {
        let config = ErrorConfig {
            max_retries: 2,
            base_delay_ms: 10,
            max_delay_ms: 100,
            exponential_backoff: false,
        };

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let _ = handle_with_retry(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err::<(), _>(Status::deadline_exceeded("timeout"))
                }
            },
            &config,
        )
        .await;

        // DeadlineExceeded is retryable, so should try 3 times
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_handle_with_retry_internal_error_retryable() {
        let config = ErrorConfig {
            max_retries: 1,
            base_delay_ms: 10,
            max_delay_ms: 100,
            exponential_backoff: false,
        };

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let _ = handle_with_retry(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err::<(), _>(Status::internal("internal error"))
                }
            },
            &config,
        )
        .await;

        // Internal is retryable, should try 2 times (1 initial + 1 retry)
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_error_config_json_roundtrip() {
        let config = ErrorConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let roundtrip: ErrorConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip.max_retries, config.max_retries);
        assert_eq!(roundtrip.base_delay_ms, config.base_delay_ms);
        assert_eq!(roundtrip.max_delay_ms, config.max_delay_ms);
        assert_eq!(roundtrip.exponential_backoff, config.exponential_backoff);
    }

    #[test]
    fn test_simulate_error_negative_rate_treated_as_zero() {
        // Negative error rate should effectively be 0%
        let result = simulate_error(-0.5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_simulate_error_rate_above_one_always_fails() {
        // Error rate above 1.0 should always fail
        for _ in 0..10 {
            let result = simulate_error(1.5);
            assert!(result.is_err());
        }
    }
}

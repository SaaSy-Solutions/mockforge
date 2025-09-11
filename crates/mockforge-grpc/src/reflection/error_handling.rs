//! Error handling and retry mechanisms for the reflection proxy

use std::time::Duration;
use tokio::time::sleep;
use tonic::Status;
use tracing::{debug, error, warn};

/// Configuration for error handling and retries
#[derive(Debug, Clone)]
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
                    tonic::Code::InvalidArgument | 
                    tonic::Code::NotFound | 
                    tonic::Code::AlreadyExists | 
                    tonic::Code::PermissionDenied |
                    tonic::Code::FailedPrecondition |
                    tonic::Code::Aborted |
                    tonic::Code::OutOfRange |
                    tonic::Code::Unimplemented => {
                        error!("Non-retryable error: {}", status);
                        return Err(status);
                    }
                    // Retry all other errors
                    _ => {
                        warn!("Attempt {} failed: {}. Retrying in {:?}...", 
                              attempts, status, delay);
                        
                        // Wait before retrying
                        sleep(delay).await;
                        
                        // Calculate next delay with exponential backoff
                        if config.exponential_backoff {
                            delay = Duration::from_millis(
                                (delay.as_millis() * 2).min(config.max_delay_ms as u128) as u64
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
    
    let mut rng = rand::thread_rng();
    let random: f64 = rng.gen();
    
    if random < error_rate {
        // Simulate different types of errors
        let error_type: u32 = rng.gen_range(0..5);
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
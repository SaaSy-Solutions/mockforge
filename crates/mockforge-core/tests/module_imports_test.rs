//! Test that ensures our critical modules can be imported and basic functionality works

use mockforge_core::performance::PerformanceMetrics;
use std::time::Duration;

#[cfg(test)]
mod import_tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_metrics_can_be_imported_and_used() {
        let metrics = PerformanceMetrics::new();

        // Basic functionality should work without panicking
        metrics.record_request_duration(Duration::from_millis(100)).await;

        // Should be able to record cache operations
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        // This test just verifies the APIs are available and don't panic
        // The actual values are tested by the existing test suite
    }

    #[test]
    fn test_encryption_module_can_be_imported() {
        // Just verify the module can be imported
        // This ensures the encryption module API is available
        // Actual functionality is tested by existing tests
        let _algorithms = mockforge_core::encryption::EncryptionAlgorithm::Aes256Gcm;
    }

    #[test]
    fn test_other_modules_can_be_imported() {
        // Verify other modules can be imported without errors
        use mockforge_core::config;

        // This ensures the modules are available for import
        let _config = config::ServerConfig::default();
    }
}

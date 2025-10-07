use mockforge_ui::handlers::*;
use mockforge_ui::models::*;
use std::collections::HashMap;

#[cfg(test)]
mod admin_handlers_tests {
    use super::*;
    use chrono::Utc;

    fn create_test_state() -> AdminState {
        use mockforge_core::init_global_logger;
        use std::sync::Arc;

        let _logger = Arc::new(init_global_logger(1000).clone());
        AdminState::new(
            Some("127.0.0.1:3000".parse().unwrap()),
            Some("127.0.0.1:3001".parse().unwrap()),
            Some("127.0.0.1:50051".parse().unwrap()),
            None, // graphql_server_addr
            true, // api_enabled
            9080, // admin_port
        )
    }

    #[tokio::test]
    async fn test_admin_state_creation() {
        let state = create_test_state();

        assert_eq!(state.http_server_addr, Some("127.0.0.1:3000".parse().unwrap()));
        assert_eq!(state.ws_server_addr, Some("127.0.0.1:3001".parse().unwrap()));
        assert_eq!(state.grpc_server_addr, Some("127.0.0.1:50051".parse().unwrap()));

        let metrics = state.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.active_connections, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_record_request() {
        let state = create_test_state();

        state.record_request("GET", "/api/users", 200, 45, None).await;
        state.record_request("POST", "/api/users", 201, 120, None).await;
        state
            .record_request(
                "GET",
                "/api/users",
                500,
                2000,
                Some("Internal Server Error".to_string()),
            )
            .await;

        let metrics = state.get_metrics().await;
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.errors_by_endpoint.get("GET /api/users"), Some(&1));
        assert_eq!(metrics.requests_by_endpoint.get("GET /api/users"), Some(&2));
        assert_eq!(metrics.requests_by_endpoint.get("POST /api/users"), Some(&1));
    }

    #[tokio::test]
    async fn test_update_latency_config() {
        let state = create_test_state();

        let overrides = HashMap::from([("auth".to_string(), 200u64), ("api".to_string(), 150u64)]);

        state.update_latency_config(100, 50, overrides).await;

        let config = state.get_config().await;
        assert_eq!(config.latency_profile.base_ms, 100);
        assert_eq!(config.latency_profile.jitter_ms, 50);
        assert_eq!(config.latency_profile.tag_overrides.get("auth"), Some(&200));
        assert_eq!(config.latency_profile.tag_overrides.get("api"), Some(&150));
    }

    #[tokio::test]
    async fn test_update_fault_config() {
        let state = create_test_state();

        let status_codes = vec![500, 502, 503, 429];
        state.update_fault_config(true, 0.1, status_codes).await;

        let config = state.get_config().await;
        assert!(config.fault_config.enabled);
        assert_eq!(config.fault_config.failure_rate, 0.1);
        assert_eq!(config.fault_config.status_codes, vec![500, 502, 503, 429]);
    }

    #[tokio::test]
    async fn test_update_proxy_config() {
        let state = create_test_state();

        state
            .update_proxy_config(true, Some("http://api.example.com".to_string()), 60)
            .await;

        let config = state.get_config().await;
        assert!(config.proxy_config.enabled);
        assert_eq!(config.proxy_config.upstream_url, Some("http://api.example.com".to_string()));
        assert_eq!(config.proxy_config.timeout_seconds, 60);
    }

    #[tokio::test]
    async fn test_update_validation_config() {
        let state = create_test_state();

        let overrides = HashMap::from([
            ("GET /health".to_string(), "off".to_string()),
            ("POST /api/users".to_string(), "enforce".to_string()),
        ]);

        state.update_validation_config("warn".to_string(), false, true, overrides).await;

        let config = state.get_config().await;
        assert_eq!(config.validation_settings.mode, "warn");
        assert!(!config.validation_settings.aggregate_errors);
        assert!(config.validation_settings.validate_responses);
        assert_eq!(
            config.validation_settings.overrides.get("GET /health"),
            Some(&"off".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_log_filtering() {
        let state = create_test_state();

        // Add some test logs
        state.record_request("GET", "/api/users", 200, 45, None).await;
        state.record_request("POST", "/api/users", 201, 120, None).await;
        state.record_request("GET", "/api/products", 200, 30, None).await;
        state.record_request("DELETE", "/api/users/123", 204, 80, None).await;

        // Test filtering by method
        let filter = LogFilter {
            method: Some("GET".to_string()),
            path_pattern: None,
            status_code: None,
            hours_ago: None,
            limit: Some(10),
        };

        let logs = state.get_logs_filtered(&filter).await;
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|log| log.method == "GET"));

        // Test filtering by path pattern
        let filter = LogFilter {
            method: None,
            path_pattern: Some("users".to_string()),
            status_code: None,
            hours_ago: None,
            limit: Some(10),
        };

        let logs = state.get_logs_filtered(&filter).await;
        assert_eq!(logs.len(), 3); // GET /users, POST /users, DELETE /users/123
        assert!(logs.iter().all(|log| log.path.contains("users")));

        // Test filtering by status code
        let filter = LogFilter {
            method: None,
            path_pattern: None,
            status_code: Some(201),
            hours_ago: None,
            limit: Some(10),
        };

        let logs = state.get_logs_filtered(&filter).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status_code, 201);

        // Test limit
        let filter = LogFilter {
            method: None,
            path_pattern: None,
            status_code: None,
            hours_ago: None,
            limit: Some(2),
        };

        let logs = state.get_logs_filtered(&filter).await;
        assert_eq!(logs.len(), 2); // Should be limited to 2
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_clear_logs() {
        let state = create_test_state();

        // Add some logs
        state.record_request("GET", "/api/users", 200, 45, None).await;
        state.record_request("POST", "/api/users", 201, 120, None).await;

        // Verify logs exist
        let logs = state.logs.read().await;
        assert_eq!(logs.len(), 2);
        drop(logs);

        // Clear logs
        state.clear_logs().await;

        // Verify logs are cleared
        let logs = state.logs.read().await;
        assert_eq!(logs.len(), 0);
    }

    #[tokio::test]
    async fn test_system_metrics_update() {
        let state = create_test_state();

        state.update_system_metrics(512, 15.5, 12).await;

        let metrics = state.get_system_metrics().await;
        assert_eq!(metrics.memory_usage_mb, 512);
        assert_eq!(metrics.cpu_usage_percent, 15.5);
        assert_eq!(metrics.active_threads, 12);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_request_log_creation() {
        let state = create_test_state();

        let start_time = Utc::now();
        state
            .record_request("GET", "/api/test", 200, 100, Some("Test error".to_string()))
            .await;
        let end_time = Utc::now();

        let logs = state.logs.read().await;
        assert_eq!(logs.len(), 1);

        let log = &logs[0];
        assert_eq!(log.method, "GET");
        assert_eq!(log.path, "/api/test");
        assert_eq!(log.status_code, 200);
        assert_eq!(log.response_time_ms, 100);
        assert_eq!(log.error_message, Some("Test error".to_string()));
        assert!(log.timestamp >= start_time && log.timestamp <= end_time);
        assert_eq!(log.id, "req_1");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_log_rotation() {
        let state = create_test_state();

        // Add more than 1000 logs to test rotation
        for i in 0..1010 {
            state.record_request("GET", &format!("/api/test/{}", i), 200, 50, None).await;
        }

        let logs = state.logs.read().await;
        // Should be limited to 1000 most recent logs
        assert!(logs.len() <= 1000);
        // The oldest log should have been removed
        assert_eq!(logs[0].id, "req_11"); // First 10 were removed
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_response_time_tracking() {
        let state = create_test_state();

        state.record_request("GET", "/api/fast", 200, 10, None).await;
        state.record_request("GET", "/api/medium", 200, 100, None).await;
        state.record_request("GET", "/api/slow", 200, 500, None).await;

        let metrics = state.get_metrics().await;
        assert_eq!(metrics.response_times.len(), 3);
        assert!(metrics.response_times.contains(&10));
        assert!(metrics.response_times.contains(&100));
        assert!(metrics.response_times.contains(&500));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_endpoint_metrics_tracking() {
        let state = create_test_state();

        state.record_request("GET", "/api/users", 200, 50, None).await;
        state.record_request("GET", "/api/users", 200, 60, None).await;
        state.record_request("POST", "/api/users", 201, 120, None).await;
        state.record_request("GET", "/api/products", 200, 40, None).await;
        state
            .record_request("GET", "/api/users", 500, 2000, Some("Server error".to_string()))
            .await;

        let metrics = state.get_metrics().await;

        assert_eq!(metrics.total_requests, 5);
        assert_eq!(metrics.requests_by_endpoint.get("GET /api/users"), Some(&3));
        assert_eq!(metrics.requests_by_endpoint.get("POST /api/users"), Some(&1));
        assert_eq!(metrics.requests_by_endpoint.get("GET /api/products"), Some(&1));
        assert_eq!(metrics.errors_by_endpoint.get("GET /api/users"), Some(&1));
    }
}

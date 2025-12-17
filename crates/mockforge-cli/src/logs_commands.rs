//! Logs viewing CLI commands
//!
//! Provides command-line interface for viewing MockForge logs from Admin API or log files.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader as TokioBufReader};
use tokio::time::sleep;

/// Log entry from Admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub status: u16,
    pub method: String,
    pub url: String,
    pub response_time: Option<u64>,
    pub size: Option<u64>,
}

/// API response wrapper
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Execute logs command
pub async fn execute_logs_command(
    admin_url: Option<String>,
    file: Option<PathBuf>,
    follow: bool,
    method: Option<String>,
    path: Option<String>,
    status: Option<u16>,
    limit: Option<usize>,
    json: bool,
    config: Option<PathBuf>,
) -> Result<()> {
    // If file is specified, read from file
    if let Some(file_path) = file {
        return read_logs_from_file(file_path, follow, json).await;
    }

    // Try to read from config file to get log file path
    if let Some(config_path) = config {
        if let Ok(log_file) = get_log_file_from_config(&config_path).await {
            if log_file.exists() {
                return read_logs_from_file(log_file, follow, json).await;
            }
        }
    }

    // Try Admin API first
    let admin_url = admin_url.unwrap_or_else(|| "http://localhost:9080".to_string());

    if follow {
        stream_logs_from_api(&admin_url, method, path, status, json).await
    } else {
        fetch_logs_from_api(&admin_url, method, path, status, limit, json).await
    }
}

/// Fetch logs from Admin API
async fn fetch_logs_from_api(
    admin_url: &str,
    method: Option<String>,
    path: Option<String>,
    status: Option<u16>,
    limit: Option<usize>,
    json: bool,
) -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let mut url = format!("{}/__mockforge/logs", admin_url);
    let mut query_params = Vec::new();

    if let Some(m) = method {
        query_params.push(("method", m));
    }
    if let Some(p) = path {
        query_params.push(("path", p));
    }
    if let Some(s) = status {
        query_params.push(("status", s.to_string()));
    }
    if let Some(l) = limit {
        query_params.push(("limit", l.to_string()));
    }

    if !query_params.is_empty() {
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url = format!("{}?{}", url, query_string);
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to Admin API. Is the server running with --admin flag?")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Admin API returned error: {} - {}. Make sure the server is running with --admin flag.",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let api_response: ApiResponse<Vec<LogEntry>> =
        response.json().await.context("Failed to parse API response")?;

    if !api_response.success {
        anyhow::bail!(
            "API error: {}",
            api_response.error.unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    let logs = api_response.data.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&logs)?);
    } else {
        print_logs_table(&logs);
    }

    Ok(())
}

/// Stream logs from Admin API using SSE
async fn stream_logs_from_api(
    admin_url: &str,
    method: Option<String>,
    path: Option<String>,
    status: Option<u16>,
    json: bool,
) -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(0)) // No timeout for streaming
        .build()
        .context("Failed to create HTTP client")?;

    // First, fetch recent logs
    let mut url = format!("{}/__mockforge/logs", admin_url);
    let mut query_params = Vec::new();

    if let Some(m) = method {
        query_params.push(("method", m));
    }
    if let Some(p) = path {
        query_params.push(("path", p));
    }
    if let Some(s) = status {
        query_params.push(("status", s.to_string()));
    }
    query_params.push(("limit", "50".to_string())); // Get recent logs

    if !query_params.is_empty() {
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url = format!("{}?{}", url, query_string);
    }

    // Fetch initial logs
    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            if let Ok(api_response) = response.json::<ApiResponse<Vec<LogEntry>>>().await {
                if let Some(logs) = api_response.data {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&logs)?);
                    } else {
                        print_logs_table(&logs);
                    }
                }
            }
        }
        _ => {
            eprintln!("⚠️  Could not fetch initial logs. Starting to follow...");
        }
    }

    // Now stream new logs using polling (SSE endpoint may not be available)
    eprintln!("📡 Following logs (press Ctrl+C to stop)...");
    let mut last_seen_timestamp = chrono::Utc::now().to_rfc3339();

    loop {
        sleep(Duration::from_millis(500)).await;

        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(api_response) = response.json::<ApiResponse<Vec<LogEntry>>>().await {
                    if let Some(logs) = api_response.data {
                        // Filter to only show new logs
                        let new_logs: Vec<_> =
                            logs.iter().filter(|log| log.timestamp > last_seen_timestamp).collect();

                        if !new_logs.is_empty() {
                            if let Some(last_log) = new_logs.last() {
                                last_seen_timestamp = last_log.timestamp.clone();
                            }

                            if json {
                                for log in new_logs {
                                    println!("{}", serde_json::to_string(log)?);
                                }
                            } else {
                                for log in new_logs {
                                    print_log_entry(log);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  Error fetching logs: {}. Retrying...", e);
                sleep(Duration::from_secs(2)).await;
            }
            _ => {}
        }
    }
}

/// Read logs from file
async fn read_logs_from_file(file_path: PathBuf, follow: bool, json: bool) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("Log file does not exist: {}", file_path.display());
    }

    if follow {
        follow_log_file(file_path, json).await
    } else {
        read_log_file_tail(file_path, json).await
    }
}

/// Read last N lines from log file
async fn read_log_file_tail(file_path: PathBuf, json: bool) -> Result<()> {
    let file = File::open(&file_path)
        .await
        .with_context(|| format!("Failed to open log file: {}", file_path.display()))?;

    let reader = TokioBufReader::new(file);
    let mut lines = reader.lines();

    // Read all lines into memory (for small files, this is fine)
    let mut all_lines = Vec::new();
    while let Some(line) = lines.next_line().await? {
        all_lines.push(line);
    }

    // Print last 100 lines
    let start = all_lines.len().saturating_sub(100);
    for line in &all_lines[start..] {
        if json {
            // Try to parse as JSON and pretty print
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            } else {
                println!("{}", line);
            }
        } else {
            println!("{}", line);
        }
    }

    Ok(())
}

/// Follow log file (like tail -f)
async fn follow_log_file(file_path: PathBuf, json: bool) -> Result<()> {
    use tokio::fs::OpenOptions;

    eprintln!("📡 Following log file: {} (press Ctrl+C to stop)...", file_path.display());

    // Get initial file size
    let mut last_size = std::fs::metadata(&file_path)
        .with_context(|| format!("Failed to get file metadata: {}", file_path.display()))?
        .len();

    loop {
        // Check if file size changed
        let current_size = match std::fs::metadata(&file_path) {
            Ok(meta) => meta.len(),
            Err(_) => {
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        if current_size > last_size {
            // Read new content
            let file = OpenOptions::new()
                .read(true)
                .open(&file_path)
                .await
                .with_context(|| format!("Failed to open log file: {}", file_path.display()))?;

            let mut reader = TokioBufReader::new(file);
            reader.seek(io::SeekFrom::Start(last_size)).await?;

            let mut buffer = String::new();
            while reader.read_line(&mut buffer).await? > 0 {
                let line = buffer.trim_end();
                if !line.is_empty() {
                    if json {
                        // Try to parse as JSON and pretty print
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                            println!("{}", serde_json::to_string_pretty(&json_value)?);
                        } else {
                            println!("{}", line);
                        }
                    } else {
                        println!("{}", line);
                    }
                }
                buffer.clear();
            }

            last_size = current_size;
        } else {
            // No new data, wait a bit
            sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Get log file path from config
async fn get_log_file_from_config(config_path: &PathBuf) -> Result<PathBuf> {
    use mockforge_core::config::load_config_auto;

    let config = load_config_auto(config_path).await?;

    if let Some(file_path) = config.logging.file_path {
        return Ok(PathBuf::from(file_path));
    }

    anyhow::bail!("No log file path configured")
}

/// Print logs in table format
fn print_logs_table(logs: &[LogEntry]) {
    if logs.is_empty() {
        println!("No logs found.");
        return;
    }

    // Print header
    println!(
        "{:<20} {:<8} {:<8} {:<50} {:<8} {:<10}",
        "Timestamp", "Status", "Method", "Path", "Time(ms)", "Size(bytes)"
    );
    println!("{}", "-".repeat(110));

    // Print logs
    for log in logs {
        print_log_entry(log);
    }
}

/// Print a single log entry
fn print_log_entry(log: &LogEntry) {
    let timestamp = if log.timestamp.len() > 19 {
        &log.timestamp[..19] // Truncate to YYYY-MM-DDTHH:MM:SS
    } else {
        &log.timestamp
    };

    let response_time = log.response_time.map(|t| t.to_string()).unwrap_or_else(|| "-".to_string());
    let size = log.size.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());

    // Color code status
    let status_str = if log.status >= 500 {
        format!("\x1b[31m{}\x1b[0m", log.status) // Red for 5xx
    } else if log.status >= 400 {
        format!("\x1b[33m{}\x1b[0m", log.status) // Yellow for 4xx
    } else {
        format!("\x1b[32m{}\x1b[0m", log.status) // Green for 2xx/3xx
    };

    let method_str = match log.method.as_str() {
        "GET" => format!("\x1b[34m{}\x1b[0m", log.method), // Blue
        "POST" => format!("\x1b[32m{}\x1b[0m", log.method), // Green
        "PUT" => format!("\x1b[33m{}\x1b[0m", log.method), // Yellow
        "DELETE" => format!("\x1b[31m{}\x1b[0m", log.method), // Red
        "PATCH" => format!("\x1b[35m{}\x1b[0m", log.method), // Magenta
        _ => log.method.clone(),
    };

    let path = if log.url.len() > 48 {
        format!("{}...", &log.url[..45])
    } else {
        log.url.clone()
    };

    println!(
        "{:<20} {:<8} {:<8} {:<50} {:<8} {:<10}",
        timestamp, status_str, method_str, path, response_time, size
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log_entry() -> LogEntry {
        LogEntry {
            timestamp: "2025-01-15T10:30:45.123Z".to_string(),
            status: 200,
            method: "GET".to_string(),
            url: "/api/users".to_string(),
            response_time: Some(42),
            size: Some(1024),
        }
    }

    #[test]
    fn test_log_entry_serialization() {
        let log = create_test_log_entry();
        let json = serde_json::to_string(&log).unwrap();

        assert!(json.contains("\"timestamp\":\"2025-01-15T10:30:45.123Z\""));
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"method\":\"GET\""));
        assert!(json.contains("\"url\":\"/api/users\""));
        assert!(json.contains("\"response_time\":42"));
        assert!(json.contains("\"size\":1024"));
    }

    #[test]
    fn test_log_entry_deserialization() {
        let json = r#"{
            "timestamp": "2025-01-15T10:30:45.123Z",
            "status": 201,
            "method": "POST",
            "url": "/api/items",
            "response_time": 100,
            "size": 2048
        }"#;

        let log: LogEntry = serde_json::from_str(json).unwrap();

        assert_eq!(log.timestamp, "2025-01-15T10:30:45.123Z");
        assert_eq!(log.status, 201);
        assert_eq!(log.method, "POST");
        assert_eq!(log.url, "/api/items");
        assert_eq!(log.response_time, Some(100));
        assert_eq!(log.size, Some(2048));
    }

    #[test]
    fn test_log_entry_deserialization_without_optional_fields() {
        let json = r#"{
            "timestamp": "2025-01-15T10:30:45Z",
            "status": 404,
            "method": "DELETE",
            "url": "/api/items/123",
            "response_time": null,
            "size": null
        }"#;

        let log: LogEntry = serde_json::from_str(json).unwrap();

        assert_eq!(log.status, 404);
        assert_eq!(log.method, "DELETE");
        assert!(log.response_time.is_none());
        assert!(log.size.is_none());
    }

    #[test]
    fn test_log_entry_clone() {
        let log = create_test_log_entry();
        let cloned = log.clone();

        assert_eq!(log.timestamp, cloned.timestamp);
        assert_eq!(log.status, cloned.status);
        assert_eq!(log.method, cloned.method);
        assert_eq!(log.url, cloned.url);
    }

    #[test]
    fn test_log_entry_debug() {
        let log = create_test_log_entry();
        let debug_str = format!("{:?}", log);

        assert!(debug_str.contains("LogEntry"));
        assert!(debug_str.contains("GET"));
        assert!(debug_str.contains("200"));
    }

    #[test]
    fn test_api_response_success() {
        let json = r#"{
            "success": true,
            "data": [{"timestamp": "2025-01-15T10:30:45Z", "status": 200, "method": "GET", "url": "/api/test", "response_time": 50, "size": 100}],
            "error": null
        }"#;

        let response: ApiResponse<Vec<LogEntry>> = serde_json::from_str(json).unwrap();

        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap().len(), 1);
    }

    #[test]
    fn test_api_response_error() {
        let json = r#"{
            "success": false,
            "data": null,
            "error": "Server unavailable"
        }"#;

        let response: ApiResponse<Vec<LogEntry>> = serde_json::from_str(json).unwrap();

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Server unavailable".to_string()));
    }

    #[test]
    fn test_log_entry_all_http_methods() {
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];

        for method in methods {
            let log = LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status: 200,
                method: method.to_string(),
                url: "/api/test".to_string(),
                response_time: None,
                size: None,
            };

            // Just verify it can be created and serialized
            let json = serde_json::to_string(&log).unwrap();
            assert!(json.contains(method));
        }
    }

    #[test]
    fn test_log_entry_various_status_codes() {
        let status_codes = [200, 201, 204, 301, 302, 400, 401, 403, 404, 500, 502, 503];

        for status in status_codes {
            let log = LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status,
                method: "GET".to_string(),
                url: "/api/test".to_string(),
                response_time: None,
                size: None,
            };

            let json = serde_json::to_string(&log).unwrap();
            assert!(json.contains(&format!("\"status\":{}", status)));
        }
    }

    #[test]
    fn test_log_entry_long_url_serialization() {
        let long_url =
            "/api/v1/organizations/12345/projects/67890/resources/abcdef/items/ghijkl/details"
                .to_string();
        let log = LogEntry {
            timestamp: "2025-01-15T10:30:45Z".to_string(),
            status: 200,
            method: "GET".to_string(),
            url: long_url.clone(),
            response_time: Some(150),
            size: Some(5000),
        };

        let json = serde_json::to_string(&log).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.url, long_url);
    }

    #[test]
    fn test_log_entry_special_characters_in_url() {
        let url = "/api/search?q=hello%20world&page=1".to_string();
        let log = LogEntry {
            timestamp: "2025-01-15T10:30:45Z".to_string(),
            status: 200,
            method: "GET".to_string(),
            url: url.clone(),
            response_time: None,
            size: None,
        };

        let json = serde_json::to_string(&log).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.url, url);
    }

    #[test]
    fn test_log_entry_zero_values() {
        let log = LogEntry {
            timestamp: "2025-01-15T10:30:45Z".to_string(),
            status: 200,
            method: "GET".to_string(),
            url: "/".to_string(),
            response_time: Some(0),
            size: Some(0),
        };

        assert_eq!(log.response_time, Some(0));
        assert_eq!(log.size, Some(0));

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"response_time\":0"));
        assert!(json.contains("\"size\":0"));
    }

    #[test]
    fn test_log_entry_large_values() {
        let log = LogEntry {
            timestamp: "2025-01-15T10:30:45Z".to_string(),
            status: 200,
            method: "GET".to_string(),
            url: "/api/large".to_string(),
            response_time: Some(u64::MAX),
            size: Some(u64::MAX),
        };

        let json = serde_json::to_string(&log).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.response_time, Some(u64::MAX));
        assert_eq!(parsed.size, Some(u64::MAX));
    }

    #[test]
    fn test_print_logs_table_empty() {
        // Test that empty logs don't cause panic
        let logs: Vec<LogEntry> = vec![];
        print_logs_table(&logs);
    }

    #[test]
    fn test_print_log_entry_does_not_panic() {
        // Test various edge cases for print_log_entry
        let test_cases = vec![
            // Short timestamp
            LogEntry {
                timestamp: "2025-01-15".to_string(),
                status: 200,
                method: "GET".to_string(),
                url: "/short".to_string(),
                response_time: None,
                size: None,
            },
            // Long timestamp
            LogEntry {
                timestamp: "2025-01-15T10:30:45.123456789Z".to_string(),
                status: 500,
                method: "POST".to_string(),
                url: "/error".to_string(),
                response_time: Some(1000),
                size: Some(100),
            },
            // Long URL that gets truncated
            LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status: 404,
                method: "DELETE".to_string(),
                url: "/api/v1/very/long/path/that/should/be/truncated/by/the/print/function"
                    .to_string(),
                response_time: None,
                size: None,
            },
            // 4xx status
            LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status: 403,
                method: "PUT".to_string(),
                url: "/forbidden".to_string(),
                response_time: Some(5),
                size: Some(0),
            },
            // PATCH method
            LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status: 200,
                method: "PATCH".to_string(),
                url: "/update".to_string(),
                response_time: Some(25),
                size: Some(512),
            },
            // Unknown method
            LogEntry {
                timestamp: "2025-01-15T10:30:45Z".to_string(),
                status: 200,
                method: "CUSTOM".to_string(),
                url: "/custom".to_string(),
                response_time: None,
                size: None,
            },
        ];

        for log in &test_cases {
            print_log_entry(log);
        }
    }

    #[test]
    fn test_print_logs_table_with_entries() {
        let logs = vec![
            create_test_log_entry(),
            LogEntry {
                timestamp: "2025-01-15T10:31:00Z".to_string(),
                status: 201,
                method: "POST".to_string(),
                url: "/api/items".to_string(),
                response_time: Some(100),
                size: Some(2048),
            },
        ];

        // Should not panic
        print_logs_table(&logs);
    }

    #[test]
    fn test_log_entry_round_trip() {
        let original = create_test_log_entry();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(original.timestamp, parsed.timestamp);
        assert_eq!(original.status, parsed.status);
        assert_eq!(original.method, parsed.method);
        assert_eq!(original.url, parsed.url);
        assert_eq!(original.response_time, parsed.response_time);
        assert_eq!(original.size, parsed.size);
    }
}

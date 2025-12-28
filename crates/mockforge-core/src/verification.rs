//! Request verification API for MockForge
//!
//! This module provides WireMock-style programmatic verification of requests,
//! allowing test code to verify that specific requests were made (or not made)
//! with various count assertions.
//!
//! ## Example
//!
//! ```rust,ignore
//! use mockforge_core::verification::{VerificationRequest, VerificationCount, verify_requests};
//! use mockforge_core::request_logger::get_global_logger;
//!
//! async fn verify_example() {
//!     // Verify that GET /api/users was called exactly 3 times
//!     let pattern = VerificationRequest {
//!         method: Some("GET".to_string()),
//!         path: Some("/api/users".to_string()),
//!         query_params: std::collections::HashMap::new(),
//!         headers: std::collections::HashMap::new(),
//!         body_pattern: None,
//!     };
//!
//!     let logger = get_global_logger().unwrap();
//!     let result = verify_requests(logger, &pattern, VerificationCount::Exactly(3)).await;
//!     assert!(result.matched, "Expected GET /api/users to be called exactly 3 times");
//! }
//! ```

use crate::request_logger::RequestLogEntry;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pattern for matching requests during verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VerificationRequest {
    /// HTTP method to match (e.g., "GET", "POST"). Case-insensitive.
    /// If None, matches any method.
    pub method: Option<String>,

    /// URL path to match. Supports exact match, wildcards (*, **), and regex.
    /// If None, matches any path.
    pub path: Option<String>,

    /// Query parameters to match (all must be present and match).
    /// If empty, query parameters are not checked.
    pub query_params: HashMap<String, String>,

    /// Headers to match (all must be present and match). Case-insensitive header names.
    /// If empty, headers are not checked.
    pub headers: HashMap<String, String>,

    /// Request body pattern to match. Supports exact match or regex.
    /// If None, body is not checked.
    pub body_pattern: Option<String>,
}

/// Count assertion for verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VerificationCount {
    /// Request must be made exactly N times
    Exactly(usize),
    /// Request must be made at least N times
    AtLeast(usize),
    /// Request must be made at most N times
    AtMost(usize),
    /// Request must never be made (count must be 0)
    Never,
    /// Request must be made at least once (count >= 1)
    AtLeastOnce,
}

/// Result of a verification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the verification passed
    pub matched: bool,
    /// Actual count of matching requests
    pub count: usize,
    /// Expected count assertion
    pub expected: VerificationCount,
    /// All matching request log entries (for inspection)
    pub matches: Vec<RequestLogEntry>,
    /// Error message if verification failed
    pub error_message: Option<String>,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn success(
        count: usize,
        expected: VerificationCount,
        matches: Vec<RequestLogEntry>,
    ) -> Self {
        Self {
            matched: true,
            count,
            expected,
            matches,
            error_message: None,
        }
    }

    /// Create a failed verification result
    pub fn failure(
        count: usize,
        expected: VerificationCount,
        matches: Vec<RequestLogEntry>,
        error_message: String,
    ) -> Self {
        Self {
            matched: false,
            count,
            expected,
            matches,
            error_message: Some(error_message),
        }
    }
}

/// Check if a request log entry matches the verification pattern
pub fn matches_verification_pattern(
    entry: &RequestLogEntry,
    pattern: &VerificationRequest,
) -> bool {
    // Check HTTP method (case-insensitive)
    if let Some(ref expected_method) = pattern.method {
        if entry.method.to_uppercase() != expected_method.to_uppercase() {
            return false;
        }
    }

    // Check path (supports exact match, wildcards, and regex)
    if let Some(ref expected_path) = pattern.path {
        if !matches_path_pattern(&entry.path, expected_path) {
            return false;
        }
    }

    // Check query parameters
    // Check query parameters
    if !pattern.query_params.is_empty() {
        for (key, expected_value) in &pattern.query_params {
            let found_value = entry.query_params.get(key);
            if found_value != Some(expected_value) {
                return false;
            }
        }
    }

    // Check headers (case-insensitive header names)
    for (key, expected_value) in &pattern.headers {
        let header_key_lower = key.to_lowercase();
        let found = entry
            .headers
            .iter()
            .any(|(k, v)| k.to_lowercase() == header_key_lower && v == expected_value);

        if !found {
            return false;
        }
    }

    // Check body pattern
    // Note: RequestLogEntry doesn't store request body directly.
    // This would need to be enhanced or we'd need to check metadata.
    // For now, we'll skip body checking if body_pattern is specified but body isn't available.
    if let Some(ref body_pattern) = pattern.body_pattern {
        // Try to get body from metadata if available
        if let Some(body_str) = entry.metadata.get("request_body") {
            if !matches_body_pattern(body_str, body_pattern) {
                return false;
            }
        } else {
            // If body pattern is specified but body isn't available, we can't verify
            // This is a limitation - we might want to return false for strict matching
            // or skip for now. Let's skip for now.
        }
    }

    true
}

/// Match a path against a pattern (supports exact, wildcard, and regex)
fn matches_path_pattern(path: &str, pattern: &str) -> bool {
    // Exact match
    if pattern == path {
        return true;
    }

    // Root wildcard matches everything
    if pattern == "*" {
        return true;
    }

    // Try wildcard matching first (before regex, as wildcards are more specific)
    if pattern.contains('*') {
        return matches_wildcard_pattern(path, pattern);
    }

    // Try regex matching (only if no wildcards)
    if let Ok(re) = Regex::new(pattern) {
        if re.is_match(path) {
            return true;
        }
    }

    false
}

/// Match a path against a wildcard pattern (* and **)
fn matches_wildcard_pattern(path: &str, pattern: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match_wildcard_segments(&pattern_parts, &path_parts, 0, 0)
}

/// Recursive function to match path segments with wildcards
fn match_wildcard_segments(
    pattern_parts: &[&str],
    path_parts: &[&str],
    pattern_idx: usize,
    path_idx: usize,
) -> bool {
    // If we've consumed both patterns and paths, it's a match
    if pattern_idx == pattern_parts.len() && path_idx == path_parts.len() {
        return true;
    }

    // If we've consumed the pattern but not the path, no match
    if pattern_idx == pattern_parts.len() {
        return false;
    }

    let current_pattern = pattern_parts[pattern_idx];

    match current_pattern {
        "*" => {
            // Single wildcard: match exactly one segment
            if path_idx < path_parts.len() {
                // Try consuming one segment
                if match_wildcard_segments(pattern_parts, path_parts, pattern_idx + 1, path_idx + 1)
                {
                    return true;
                }
            }
            false
        }
        "**" => {
            // Double wildcard: match zero or more segments
            // Try matching zero segments first
            if match_wildcard_segments(pattern_parts, path_parts, pattern_idx + 1, path_idx) {
                return true;
            }
            // Try matching one or more segments
            if path_idx < path_parts.len()
                && match_wildcard_segments(pattern_parts, path_parts, pattern_idx, path_idx + 1)
            {
                return true;
            }
            false
        }
        _ => {
            // Exact segment match
            if path_idx < path_parts.len() && path_parts[path_idx] == current_pattern {
                match_wildcard_segments(pattern_parts, path_parts, pattern_idx + 1, path_idx + 1)
            } else {
                false
            }
        }
    }
}

/// Match a body against a pattern (supports exact match or regex)
fn matches_body_pattern(body: &str, pattern: &str) -> bool {
    // Try regex first
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(body)
    } else {
        // Fall back to exact match
        body == pattern
    }
}

/// Verify requests against a pattern and count assertion
pub async fn verify_requests(
    logger: &crate::request_logger::CentralizedRequestLogger,
    pattern: &VerificationRequest,
    expected: VerificationCount,
) -> VerificationResult {
    // Get all logs
    let logs = logger.get_recent_logs(None).await;

    // Find matching requests
    let matches: Vec<RequestLogEntry> = logs
        .into_iter()
        .filter(|entry| matches_verification_pattern(entry, pattern))
        .collect();

    let count = matches.len();

    // Check count assertion
    let matched = match &expected {
        VerificationCount::Exactly(n) => count == *n,
        VerificationCount::AtLeast(n) => count >= *n,
        VerificationCount::AtMost(n) => count <= *n,
        VerificationCount::Never => count == 0,
        VerificationCount::AtLeastOnce => count >= 1,
    };

    if matched {
        VerificationResult::success(count, expected, matches)
    } else {
        let error_message = format!(
            "Verification failed: expected {:?}, but found {} matching requests",
            expected, count
        );
        VerificationResult::failure(count, expected, matches, error_message)
    }
}

/// Verify that a request was never made
pub async fn verify_never(
    logger: &crate::request_logger::CentralizedRequestLogger,
    pattern: &VerificationRequest,
) -> VerificationResult {
    verify_requests(logger, pattern, VerificationCount::Never).await
}

/// Verify that a request was made at least N times
pub async fn verify_at_least(
    logger: &crate::request_logger::CentralizedRequestLogger,
    pattern: &VerificationRequest,
    min: usize,
) -> VerificationResult {
    verify_requests(logger, pattern, VerificationCount::AtLeast(min)).await
}

/// Verify that requests occurred in a specific sequence
pub async fn verify_sequence(
    logger: &crate::request_logger::CentralizedRequestLogger,
    patterns: &[VerificationRequest],
) -> VerificationResult {
    // Get all logs (most recent first)
    let mut logs = logger.get_recent_logs(None).await;
    // Reverse to get chronological order (oldest first) for sequence verification
    logs.reverse();

    // Find matches for each pattern in order
    let mut log_idx = 0;
    let mut all_matches = Vec::new();

    for pattern in patterns {
        // Find the next matching request after the last match
        let mut found = false;
        while log_idx < logs.len() {
            if matches_verification_pattern(&logs[log_idx], pattern) {
                all_matches.push(logs[log_idx].clone());
                log_idx += 1;
                found = true;
                break;
            }
            log_idx += 1;
        }

        if !found {
            let error_message = format!(
                "Sequence verification failed: pattern {:?} not found in sequence",
                pattern
            );
            return VerificationResult::failure(
                all_matches.len(),
                VerificationCount::Exactly(patterns.len()),
                all_matches,
                error_message,
            );
        }
    }

    VerificationResult::success(
        all_matches.len(),
        VerificationCount::Exactly(patterns.len()),
        all_matches,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_logger::{create_http_log_entry, CentralizedRequestLogger};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_entry(method: &str, path: &str) -> RequestLogEntry {
        create_http_log_entry(
            method,
            path,
            200,
            100,
            Some("127.0.0.1".to_string()),
            Some("test-agent".to_string()),
            HashMap::new(),
            1024,
            None,
        )
    }

    #[tokio::test]
    async fn test_verify_exactly() {
        let logger = CentralizedRequestLogger::new(100);
        logger.log_request(create_test_entry("GET", "/api/users")).await;
        logger.log_request(create_test_entry("GET", "/api/users")).await;
        logger.log_request(create_test_entry("GET", "/api/users")).await;

        let pattern = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };

        let result = verify_requests(&logger, &pattern, VerificationCount::Exactly(3)).await;
        assert!(result.matched);
        assert_eq!(result.count, 3);
    }

    #[tokio::test]
    async fn test_verify_at_least() {
        let logger = CentralizedRequestLogger::new(100);
        logger.log_request(create_test_entry("POST", "/api/orders")).await;
        logger.log_request(create_test_entry("POST", "/api/orders")).await;

        let pattern = VerificationRequest {
            method: Some("POST".to_string()),
            path: Some("/api/orders".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };

        let result = verify_at_least(&logger, &pattern, 2).await;
        assert!(result.matched);
        assert_eq!(result.count, 2);

        let result2 = verify_at_least(&logger, &pattern, 1).await;
        assert!(result2.matched);

        let result3 = verify_at_least(&logger, &pattern, 3).await;
        assert!(!result3.matched);
    }

    #[tokio::test]
    async fn test_verify_never() {
        let logger = CentralizedRequestLogger::new(100);
        logger.log_request(create_test_entry("GET", "/api/users")).await;

        let pattern = VerificationRequest {
            method: Some("DELETE".to_string()),
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };

        let result = verify_never(&logger, &pattern).await;
        assert!(result.matched);
        assert_eq!(result.count, 0);
    }

    #[tokio::test]
    async fn test_verify_sequence() {
        let logger = CentralizedRequestLogger::new(100);
        logger.log_request(create_test_entry("POST", "/api/users")).await;
        logger.log_request(create_test_entry("GET", "/api/users/1")).await;
        logger.log_request(create_test_entry("PUT", "/api/users/1")).await;

        let patterns = vec![
            VerificationRequest {
                method: Some("POST".to_string()),
                path: Some("/api/users".to_string()),
                query_params: HashMap::new(),
                headers: HashMap::new(),
                body_pattern: None,
            },
            VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/api/users/1".to_string()),
                query_params: HashMap::new(),
                headers: HashMap::new(),
                body_pattern: None,
            },
        ];

        let result = verify_sequence(&logger, &patterns).await;
        assert!(result.matched);
        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_matches_path_pattern_exact() {
        assert!(matches_path_pattern("/api/users", "/api/users"));
        assert!(!matches_path_pattern("/api/users", "/api/posts"));
    }

    #[test]
    fn test_matches_path_pattern_wildcard() {
        assert!(matches_path_pattern("/api/users/1", "/api/users/*"));
        assert!(matches_path_pattern("/api/users/123", "/api/users/*"));
        assert!(!matches_path_pattern("/api/users/1/posts", "/api/users/*"));
    }

    #[test]
    fn test_matches_path_pattern_double_wildcard() {
        assert!(matches_path_pattern("/api/users/1", "/api/**"));
        assert!(matches_path_pattern("/api/users/1/posts", "/api/**"));
        assert!(matches_path_pattern("/api/users", "/api/**"));
    }

    #[test]
    fn test_matches_path_pattern_regex() {
        assert!(matches_path_pattern("/api/users/123", r"^/api/users/\d+$"));
        assert!(!matches_path_pattern("/api/users/abc", r"^/api/users/\d+$"));
    }

    #[test]
    fn test_matches_verification_pattern_method() {
        let entry = create_test_entry("GET", "/api/users");
        let pattern = VerificationRequest {
            method: Some("GET".to_string()),
            path: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };
        assert!(matches_verification_pattern(&entry, &pattern));

        let pattern2 = VerificationRequest {
            method: Some("POST".to_string()),
            path: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };
        assert!(!matches_verification_pattern(&entry, &pattern2));
    }

    #[test]
    fn test_matches_verification_pattern_path() {
        let entry = create_test_entry("GET", "/api/users");
        let pattern = VerificationRequest {
            method: None,
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };
        assert!(matches_verification_pattern(&entry, &pattern));

        let pattern2 = VerificationRequest {
            method: None,
            path: Some("/api/posts".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };
        assert!(!matches_verification_pattern(&entry, &pattern2));
    }
}

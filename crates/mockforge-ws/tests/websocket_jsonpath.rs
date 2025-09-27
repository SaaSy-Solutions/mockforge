use mockforge_ws::{evaluate_jsonpath_on_message, is_jsonpath_pattern, message_matches_pattern};

#[test]
fn test_jsonpath_pattern_detection() {
    // JSONPath patterns
    assert!(is_jsonpath_pattern("$.type"));
    assert!(is_jsonpath_pattern("$.user.id"));
    assert!(is_jsonpath_pattern("$[0].name"));
    assert!(is_jsonpath_pattern("$.items[*].price"));

    // Non-JSONPath patterns (regex)
    assert!(!is_jsonpath_pattern("^CLIENT_READY$"));
    assert!(!is_jsonpath_pattern("ACK"));
    assert!(!is_jsonpath_pattern("hello.*world"));
}

#[test]
fn test_jsonpath_evaluation() {
    let json_message = r#"{"type": "login", "user": {"id": "123", "name": "Alice"}, "items": [{"name": "item1", "price": 10}]}"#;

    // Test basic property existence
    assert!(evaluate_jsonpath_on_message("$.type", json_message));
    assert!(evaluate_jsonpath_on_message("$.user", json_message));
    assert!(evaluate_jsonpath_on_message("$.items", json_message));

    // Test nested property access
    assert!(evaluate_jsonpath_on_message("$.user.id", json_message));
    assert!(evaluate_jsonpath_on_message("$.user.name", json_message));

    // Test array access
    assert!(evaluate_jsonpath_on_message("$.items[0]", json_message));
    assert!(evaluate_jsonpath_on_message("$.items[0].name", json_message));

    // Test non-existent paths
    assert!(!evaluate_jsonpath_on_message("$.nonexistent", json_message));
    assert!(!evaluate_jsonpath_on_message("$.user.email", json_message));
    assert!(!evaluate_jsonpath_on_message("$.items[5]", json_message));
}

#[test]
fn test_jsonpath_with_invalid_json() {
    let invalid_json = r#"{"type": "login", "incomplete": json"#;

    // JSONPath queries should return false for invalid JSON
    assert!(!evaluate_jsonpath_on_message("$.type", invalid_json));
}

#[test]
fn test_jsonpath_with_plain_text() {
    let plain_text = "This is just plain text, not JSON";

    // JSONPath queries should return false for non-JSON messages
    assert!(!evaluate_jsonpath_on_message("$.type", plain_text));
}

#[test]
fn test_message_matches_pattern_integration() {
    let json_message = r#"{"type": "login", "user": {"id": "123"}}"#;
    let plain_message = "CLIENT_READY";

    // Test JSONPath matching
    assert!(message_matches_pattern("$.type", json_message));
    assert!(message_matches_pattern("$.user.id", json_message));
    assert!(!message_matches_pattern("$.nonexistent", json_message));

    // Test regex matching (should still work)
    assert!(message_matches_pattern("^CLIENT_READY$", plain_message));
    assert!(message_matches_pattern("CLIENT", plain_message));
    assert!(!message_matches_pattern("NONEXISTENT", plain_message));

    // Test regex with JSON message (should work on the JSON string)
    assert!(message_matches_pattern("login", json_message));
    assert!(message_matches_pattern("\\{", json_message)); // Matches opening brace
}

#[test]
fn test_invalid_patterns() {
    let json_message = r#"{"type": "login"}"#;

    // Invalid JSONPath should return false and log warning
    assert!(!evaluate_jsonpath_on_message("$.invalid[unclosed", json_message));

    // Invalid regex should return false and log warning
    assert!(!message_matches_pattern("[unclosed", json_message));
}

#[test]
fn test_complex_jsonpath_queries() {
    let complex_json = r#"{
        "orders": [
            {"id": "1", "status": "pending", "items": [{"name": "widget", "price": 10}]},
            {"id": "2", "status": "shipped", "items": [{"name": "gadget", "price": 20}]}
        ],
        "user": {
            "profile": {
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            }
        }
    }"#;

    // Test array element access
    assert!(evaluate_jsonpath_on_message("$.orders[0]", complex_json));
    assert!(evaluate_jsonpath_on_message("$.orders[1].status", complex_json));

    // Test deeply nested properties
    assert!(evaluate_jsonpath_on_message("$.user.profile.preferences.theme", complex_json));
    assert!(evaluate_jsonpath_on_message(
        "$.user.profile.preferences.notifications",
        complex_json
    ));

    // Test nested array access
    assert!(evaluate_jsonpath_on_message("$.orders[0].items[0].name", complex_json));
}

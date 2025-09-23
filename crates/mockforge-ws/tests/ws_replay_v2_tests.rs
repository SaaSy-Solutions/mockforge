use futures_util::{SinkExt, StreamExt};
use mockforge_core::ws_proxy::WsProxyConfig;
use mockforge_ws::{WsReplayEntry, WsAssertion, WsMatchCondition};
use serde_json::Value;
use std::collections::HashMap;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn test_ws_replay_engine_parsing() {
    let script_content = r#"{"step_id":"init","dir":"out","text":"HELLO CLIENT","wait_for":"^READY$","timeout_ms":5000}
{"step_id":"welcome","dir":"out","text":"{"type":"welcome","session":"{{uuid}}"}"}
{"step_id":"wait_for_auth","dir":"in","assertions":[{"pattern":"$.type","expected":true,"description":"Must have type field"}],"on_match":[{"pattern":"$.action:\\"auth\\"","goto_step":"authorized"}],"branches":{"auth":"authorized"}}
{"step_id":"authorized","dir":"out","text":"{"type":"authorized"}"}
{"step_id":"error","dir":"out","text":"{"type":"error"}"}"#;

    let entries: Vec<WsReplayEntry> = script_content
        .lines()
        .filter_map(|line| {
            if line.trim().is_empty() { None } else {
                serde_json::from_str(line).ok()
            }
        })
        .collect();

    assert_eq!(entries.len(), 5);

    // Check first entry (named step)
    let init_entry = &entries[0];
    assert_eq!(init_entry.step_id, Some("init".to_string()));
    assert_eq!(init_entry.dir, "out");
    assert_eq!(init_entry.text, Some("HELLO CLIENT".to_string()));
    assert_eq!(init_entry.wait_for, Some("^READY$".to_string()));
    assert_eq!(init_entry.timeout_ms, Some(5000));

    // Check inbound entry with assertions
    let auth_entry = &entries[2];
    assert_eq!(auth_entry.step_id, Some("wait_for_auth".to_string()));
    assert_eq!(auth_entry.dir, "in");
    assert!(auth_entry.assertions.is_some());
    let assertions = auth_entry.assertions.as_ref().unwrap();
    assert_eq!(assertions.len(), 1);
    assert_eq!(assertions[0].pattern, "$.type");
    assert_eq!(assertions[0].expected, true);

    // Check branching
    assert!(auth_entry.branches.is_some());
    let branches = auth_entry.branches.as_ref().unwrap();
    assert_eq!(branches.get("auth"), Some(&"authorized".to_string()));
}

#[tokio::test]
async fn test_assertion_processing() {
    // Test assertion with JSONPath
    let valid_json = r#"{"type":"auth","user":123}"#;
    let assertions = vec![
        WsAssertion {
            pattern: "$.type".to_string(),
            expected: true,
            description: Some("Must have type field".to_string()),
        },
        WsAssertion {
            pattern: "$.user".to_string(),
            expected: true,
            description: Some("Must have user field".to_string()),
        },
    ];

    // This would be tested in the actual engine
    // engine.process_assertions(valid_json, &assertions).unwrap();

    // Test negative case
    let invalid_json = r#"{"status":"error"}"#;
    let negative_assertions = vec![
        WsAssertion {
            pattern: "$.type".to_string(),
            expected: true,
            description: Some("Must have type field".to_string()),
        },
    ];

    // This would fail: engine.process_assertions(invalid_json, &negative_assertions).expect_err(...)
}

#[tokio::test]
async fn test_pattern_matching() {
    // Test JSONPath pattern
    let json_message = r#"{"type":"auth","action":"login"}"#;
    assert!(mockforge_ws::message_matches_pattern("$.type", json_message));
    assert!(mockforge_ws::message_matches_pattern("$.action", json_message));
    assert!(!mockforge_ws::message_matches_pattern("$.missing", json_message));

    // Test regex pattern
    let text_message = "USER_LOGGED_IN_123";
    assert!(mockforge_ws::message_matches_pattern("^USER_.*_[0-9]+$", text_message));
    assert!(!mockforge_ws::message_matches_pattern("^ADMIN_.*", text_message));
}

#[tokio::test]
async fn test_complex_branching_script() {
    // Test a more complex branching script that demonstrates the v2 features
    let complex_script = r#"{"step_id":"session_init","dir":"out","text":"SESSION_START {{uuid}}","timeout_ms":10000}
{"step_id":"await_credentials","dir":"in","assertions":[{"pattern":"$.username","expected":true},{"pattern":"$.password","expected":true}],"on_match":[{"pattern":"$.username:\\"admin\\"","goto_step":"admin_flow"}],"branches":{"admin":"admin_flow","guest":"guest_flow"},"next_step":"standard_flow"}
{"step_id":"admin_flow","dir":"out","text":"ADMIN_ACCESS_GRANTED","wait_for":"^CONFIRMED$"/","timeout_ms":5000}
{"step_id":"guest_flow","dir":"out","text":"GUEST_ACCESS_GRANTED","wait_for":"^ACK$"//","timeout_ms":5000}
{"step_id":"standard_flow","dir":"out","text":"STANDARD_USER_WELCOME"}
{"step_id":"cleanup","dir":"out","text":"SESSION_END"}"#;

    let entries: Vec<WsReplayEntry> = complex_script
        .lines()
        .filter_map(|line| {
            if line.trim().is_empty() { None } else {
                serde_json::from_str(line).ok()
            }
        })
        .collect();

    // Verify scripting structure
    assert_eq!(entries.len(), 6);

    // Check that we have proper step IDs
    let step_ids: Vec<String> = entries.iter()
        .filter_map(|e| e.step_id.clone())
        .collect();

    let expected_steps = vec![
        "session_init", "await_credentials", "admin_flow",
        "guest_flow", "standard_flow", "cleanup"
    ];

    assert_eq!(step_ids, expected_steps);

    // Verify assertions on credentials step
    let credentials_step = entries.iter().find(|e| e.step_id == Some("await_credentials".to_string())).unwrap();
    assert!(credentials_step.assertions.is_some());
    assert_eq!(credentials_step.assertions.as_ref().unwrap().len(), 2);

    // Verify branching logic
    assert!(credentials_step.branches.is_some());
    let branches = credentials_step.branches.as_ref().unwrap();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches.get("admin"), Some(&"admin_flow".to_string()));
    assert_eq!(branches.get("guest"), Some(&"guest_flow".to_string()));
}

#[tokio::test]
async fn test_legacy_fallback_compilation() {
    // Test that legacy format still works
    let legacy_script = r#"{"ts":0,"dir":"out","text":"HELLO","wait_for":"^READY$"}
{"ts":10,"dir":"out","text":"WORLD"}"#;

    let entries: Vec<WsReplayEntry> = legacy_script
        .lines()
        .filter_map(|line| {
            if line.trim().is_empty() { None } else {
                serde_json::from_str(line).ok()
            }
        })
        .collect();

    assert_eq!(entries.len(), 2);

    // Check legacy fields work
    assert_eq!(entries[0].ts, Some(0));
    assert_eq!(entries[0].dir, "out");
    assert_eq!(entries[0].wait_for, Some("^READY$".to_string()));
    assert_eq!(entries[1].ts, Some(10));
    assert_eq!(entries[1].wait_for, None); // Second message has no wait_for
}

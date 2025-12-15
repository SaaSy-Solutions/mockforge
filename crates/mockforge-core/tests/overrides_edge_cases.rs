//! Edge case tests for overrides module
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for the overrides functionality.

use mockforge_core::overrides::{OverrideMode, OverrideRule, Overrides, PatchOp};
use serde_json::json;

/// Test empty overrides
#[test]
fn test_overrides_empty() {
    let overrides = Overrides {
        rules: vec![],
        regex_cache: Default::default(),
    };

    let mut body = json!({"value": "original"});
    overrides.apply("test_op", &[], "/test", &mut body);

    // Should remain unchanged
    assert_eq!(body["value"], "original");
}

/// Test overrides with multiple rules
#[test]
fn test_overrides_multiple_rules() {
    let overrides = Overrides {
        rules: vec![
            OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/value1".to_string(),
                    value: json!("first"),
                }],
                when: None,
                post_templating: false,
            },
            OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/value2".to_string(),
                    value: json!("second"),
                }],
                when: None,
                post_templating: false,
            },
        ],
        regex_cache: Default::default(),
    };

    let mut body = json!({"value1": "original1", "value2": "original2"});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["value1"], "first");
    assert_eq!(body["value2"], "second");
}

/// Test overrides with Merge mode
#[test]
fn test_overrides_merge_mode() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Merge,
            patch: vec![PatchOp::Replace {
                path: "/nested/key".to_string(),
                value: json!("merged"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({
        "nested": {
            "key": "original",
            "other": "preserved"
        }
    });
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["nested"]["key"], "merged");
    assert_eq!(body["nested"]["other"], "preserved");
}

/// Test overrides with path matching
#[test]
fn test_overrides_path_matching() {
    use regex::Regex;
    use std::collections::HashMap;

    // Pre-compile regex pattern for path matching
    let mut regex_cache = HashMap::new();
    regex_cache.insert("/api/users/.*".to_string(), Regex::new(r"/api/users/.*").unwrap());

    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["path:/api/users/.*".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/count".to_string(),
                value: json!(100),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache,
    };

    let mut body = json!({"count": 0});
    overrides.apply("any_op", &[], "/api/users/123", &mut body);

    assert_eq!(body["count"], 100);
}

/// Test overrides with tag matching
#[test]
fn test_overrides_tag_matching() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["tag:admin".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/role".to_string(),
                value: json!("admin"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"role": "user"});
    overrides.apply("any_op", &["admin".to_string()], "/test", &mut body);

    assert_eq!(body["role"], "admin");
}

/// Test overrides with multiple tags
#[test]
fn test_overrides_multiple_tags() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["tag:premium".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/tier".to_string(),
                value: json!("premium"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"tier": "free"});
    overrides.apply("any_op", &["premium".to_string(), "vip".to_string()], "/test", &mut body);

    assert_eq!(body["tier"], "premium");
}

/// Test overrides with no matching tags
#[test]
fn test_overrides_no_matching_tags() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["tag:premium".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/tier".to_string(),
                value: json!("premium"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"tier": "free"});
    overrides.apply("any_op", &["basic".to_string()], "/test", &mut body);

    // Should remain unchanged
    assert_eq!(body["tier"], "free");
}

/// Test overrides with nested path
#[test]
fn test_overrides_nested_path() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/level1/level2/level3/value".to_string(),
                value: json!("deep"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({
        "level1": {
            "level2": {
                "level3": {
                    "value": "shallow"
                }
            }
        }
    });
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["level1"]["level2"]["level3"]["value"], "deep");
}

/// Test overrides with array index
#[test]
fn test_overrides_array_index() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/items/0/name".to_string(),
                value: json!("first"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({
        "items": [
            {"name": "original", "id": 1},
            {"name": "second", "id": 2}
        ]
    });
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["items"][0]["name"], "first");
    assert_eq!(body["items"][1]["name"], "second");
}

/// Test overrides with non-existent path using Add operation
#[test]
fn test_overrides_non_existent_path() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Merge, // Merge mode handles Add operations better
            patch: vec![PatchOp::Add {
                path: "/new/field/value".to_string(),
                value: json!("created"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({});
    overrides.apply("test_op", &[], "/test", &mut body);

    // Should create the path with Add operation
    assert_eq!(body["new"]["field"]["value"], "created");
}

/// Test overrides with empty string value
#[test]
fn test_overrides_empty_string() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!(""),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"value": "original"});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["value"], "");
}

/// Test overrides with null value
#[test]
fn test_overrides_null_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!(null),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"value": "original"});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert!(body["value"].is_null());
}

/// Test overrides with boolean value
#[test]
fn test_overrides_boolean_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/enabled".to_string(),
                value: json!(true),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"enabled": false});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["enabled"], true);
}

/// Test overrides with number value
#[test]
fn test_overrides_number_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/count".to_string(),
                value: json!(42),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"count": 0});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["count"], 42);
}

/// Test overrides with float value
#[test]
fn test_overrides_float_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/price".to_string(),
                value: json!(99.99),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"price": 0.0});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["price"], 99.99);
}

/// Test overrides with array value
#[test]
fn test_overrides_array_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/items".to_string(),
                value: json!([1, 2, 3]),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"items": []});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["items"], json!([1, 2, 3]));
}

/// Test overrides with object value
#[test]
fn test_overrides_object_value() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/metadata".to_string(),
                value: json!({"key": "value", "number": 42}),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"metadata": {}});
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["metadata"]["key"], "value");
    assert_eq!(body["metadata"]["number"], 42);
}

/// Test overrides rules() method
#[test]
fn test_overrides_rules_method() {
    let rule = OverrideRule {
        targets: vec!["operation:test_op".to_string()],
        mode: OverrideMode::Replace,
        patch: vec![],
        when: None,
        post_templating: false,
    };

    let overrides = Overrides {
        rules: vec![rule.clone()],
        regex_cache: Default::default(),
    };

    let rules = overrides.rules();
    assert_eq!(rules.len(), 1);
}

/// Test overrides with multiple target patterns
#[test]
fn test_overrides_multiple_targets() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string(), "tag:test_tag".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!("matched"),
            }],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({"value": "original"});
    // Should match by operation
    overrides.apply("test_op", &[], "/test", &mut body);
    assert_eq!(body["value"], "matched");

    // Reset and test tag match
    body = json!({"value": "original"});
    overrides.apply("other_op", &["test_tag".to_string()], "/test", &mut body);
    assert_eq!(body["value"], "matched");
}

/// Test overrides with complex nested structure
#[test]
fn test_overrides_complex_nested() {
    let overrides = Overrides {
        rules: vec![OverrideRule {
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![
                PatchOp::Replace {
                    path: "/user/profile/name".to_string(),
                    value: json!("John Doe"),
                },
                PatchOp::Replace {
                    path: "/user/profile/email".to_string(),
                    value: json!("john@example.com"),
                },
            ],
            when: None,
            post_templating: false,
        }],
        regex_cache: Default::default(),
    };

    let mut body = json!({
        "user": {
            "profile": {
                "name": "Original",
                "email": "original@example.com",
                "age": 30
            }
        }
    });
    overrides.apply("test_op", &[], "/test", &mut body);

    assert_eq!(body["user"]["profile"]["name"], "John Doe");
    assert_eq!(body["user"]["profile"]["email"], "john@example.com");
    assert_eq!(body["user"]["profile"]["age"], 30); // Should be preserved
}

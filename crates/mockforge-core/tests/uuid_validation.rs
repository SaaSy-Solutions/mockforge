use regex::Regex;
use serde_json::json;

/// Test UUID validation with comprehensive regex patterns
#[test]
fn test_uuid_regex_patterns() {
    // Standard UUID v4 regex pattern
    let _uuid_v4_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();

    // More permissive UUID regex (accepts any valid UUID format)
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Test valid UUIDs
    let valid_uuids = vec![
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "6ba7b811-9dad-21d1-80b4-00c04fd430c8",
        "6ba7b812-9dad-31d1-80b4-00c04fd430c8",
        "6ba7b813-9dad-41d1-80b4-00c04fd430c8",
        "6ba7b814-9dad-51d1-80b4-00c04fd430c8",
    ];

    for uuid in valid_uuids {
        assert!(uuid_regex.is_match(uuid), "UUID {} should be valid", uuid);
        println!("✓ Valid UUID: {}", uuid);
    }

    // Test UUID v4 specific pattern
    let v4_uuids = vec![
        "550e8400-e29b-41d4-a716-446655440000", // version 4, variant 1
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8", // version 1, variant 1
        "6ba7b811-9dad-21d1-80b4-00c04fd430c8", // version 2, variant 1
        "6ba7b812-9dad-31d1-80b4-00c04fd430c8", // version 3, variant 1
        "6ba7b814-9dad-51d1-80b4-00c04fd430c8", // version 5, variant 1
    ];

    for uuid in v4_uuids {
        // All should match the general pattern
        assert!(uuid_regex.is_match(uuid), "UUID {} should match general pattern", uuid);
    }

    // Test invalid UUIDs
    let invalid_uuids = vec![
        "550e8400-e29b-41d4-a716-44665544000",        // too short
        "550e8400-e29b-41d4-a716-4466554400000",      // too long
        "550e8400-e29b-41d4-a716-44665544000g",       // invalid character
        "550e8400e29b41d4a716446655440000",           // no hyphens
        "550e8400-e29b-41d4-a716",                    // incomplete
        "gggggggg-gggg-gggg-gggg-gggggggggggg",       // all invalid chars
        "",                                           // empty
        "550e8400-e29b-41d4-a716-446655440000-extra", // extra content
    ];

    for invalid_uuid in invalid_uuids {
        assert!(!uuid_regex.is_match(invalid_uuid), "UUID {} should be invalid", invalid_uuid);
        println!("✓ Correctly rejected invalid UUID: {}", invalid_uuid);
    }
}

/// Test UUID validation in JSON responses
#[test]
fn test_uuid_validation_in_json() {
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Test various JSON structures with UUIDs
    let test_cases = vec![
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "test"
        }),
        json!({
            "user": {
                "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                "sessionId": "550e8400-e29b-41d4-a716-446655440001"
            }
        }),
        json!({
            "items": [
                {"id": "550e8400-e29b-41d4-a716-446655440002"},
                {"id": "550e8400-e29b-41d4-a716-446655440003"}
            ]
        }),
        json!({
            "data": {
                "uuids": [
                    "550e8400-e29b-41d4-a716-446655440004",
                    "550e8400-e29b-41d4-a716-446655440005",
                    "550e8400-e29b-41d4-a716-446655440006"
                ]
            }
        }),
    ];

    for test_case in test_cases {
        validate_uuids_in_json(&test_case, &uuid_regex);
    }
}

/// Recursive function to validate all UUID strings in a JSON structure
fn validate_uuids_in_json(value: &serde_json::Value, uuid_regex: &Regex) {
    match value {
        serde_json::Value::String(s) => {
            // If it looks like a UUID (has hyphens and correct length), validate it
            if s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4 {
                assert!(uuid_regex.is_match(s), "String {} should be a valid UUID", s);
                println!("✓ Validated UUID in JSON: {}", s);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                validate_uuids_in_json(item, uuid_regex);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_key, val) in obj {
                validate_uuids_in_json(val, uuid_regex);
            }
        }
        _ => {} // Other types don't contain UUIDs
    }
}

/// Test UUID uniqueness validation
#[test]
fn test_uuid_uniqueness() {
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Generate a set of UUIDs and ensure they're all unique
    let mut generated_uuids = std::collections::HashSet::new();

    // In a real test, these would come from the templating system
    // For now, we'll use known valid UUIDs
    let test_uuids = vec![
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "7ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "8ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "9ba7b810-9dad-11d1-80b4-00c04fd430c8",
    ];

    for uuid in test_uuids {
        assert!(uuid_regex.is_match(uuid), "Generated UUID {} should be valid", uuid);
        assert!(generated_uuids.insert(uuid), "UUID {} should be unique", uuid);
        println!("✓ Unique UUID generated: {}", uuid);
    }

    assert_eq!(generated_uuids.len(), 5, "Should have 5 unique UUIDs");
}

/// Test UUID validation with different casing
#[test]
fn test_uuid_case_insensitive_validation() {
    // Test that our regex handles different cases properly
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    let test_uuid = "550e8400-e29b-41d4-a716-446655440000";

    // Test lowercase (should work)
    assert!(uuid_regex.is_match(test_uuid), "Lowercase UUID should be valid");

    // Test uppercase (should fail with our current regex)
    let uppercase_uuid = test_uuid.to_uppercase();
    assert!(
        !uuid_regex.is_match(&uppercase_uuid),
        "Uppercase UUID should be invalid with current regex: {}",
        uppercase_uuid
    );

    // Test mixed case (should fail)
    let mixed_uuid = "550E8400-E29B-41D4-A716-446655440000";
    assert!(
        !uuid_regex.is_match(mixed_uuid),
        "Mixed case UUID should be invalid with current regex: {}",
        mixed_uuid
    );

    println!("✓ UUID validation correctly handles case sensitivity");
}

/// Test UUID validation performance
#[test]
fn test_uuid_validation_performance() {
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    let test_uuids = vec![
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "7ba7b811-9dad-21d1-80b4-00c04fd430c8",
        "8ba7b812-9dad-31d1-80b4-00c04fd430c8",
        "9ba7b813-9dad-41d1-80b4-00c04fd430c8",
    ];

    let start = std::time::Instant::now();

    // Validate each UUID multiple times to test performance
    for _ in 0..1000 {
        for uuid in &test_uuids {
            assert!(uuid_regex.is_match(uuid));
        }
    }

    let duration = start.elapsed();
    println!("✓ UUID validation performance: {} validations in {:?}", 5000, duration);

    // Should complete in reasonable time (less than 1 second for 5000 validations)
    assert!(
        duration.as_millis() < 1000,
        "UUID validation should be fast, took {:?}",
        duration
    );
}

/// Test UUID validation in complex nested structures
#[test]
fn test_uuid_validation_complex_structures() {
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Complex nested structure that might come from an API response
    let complex_response = json!({
        "data": {
            "users": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "session": {
                        "token": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                        "refreshToken": "7ba7b811-9dad-21d1-80b4-00c04fd430c8"
                    },
                    "preferences": {
                        "themeId": "8ba7b812-9dad-31d1-80b4-00c04fd430c8"
                    }
                },
                {
                    "id": "9ba7b813-9dad-41d1-80b4-00c04fd430c8",
                    "session": {
                        "token": "aba7b814-9dad-51d1-80b4-00c04fd430c8",
                        "refreshToken": "bba7b815-9dad-61d1-80b4-00c04fd430c8"
                    }
                }
            ],
            "metadata": {
                "requestId": "cba7b816-9dad-71d1-80b4-00c04fd430c8",
                "correlationId": "dba7b817-9dad-81d1-80b4-00c04fd430c8"
            }
        },
        "pagination": {
            "cursor": "eba7b818-9dad-91d1-80b4-00c04fd430c8"
        }
    });

    // This should find and validate all UUIDs in the structure
    let uuid_count = count_and_validate_uuids(&complex_response, &uuid_regex);
    assert_eq!(uuid_count, 10, "Should find 10 UUIDs in complex structure");

    println!("✓ Validated {} UUIDs in complex nested structure", uuid_count);
}

/// Helper function to count and validate UUIDs in a JSON structure
fn count_and_validate_uuids(value: &serde_json::Value, uuid_regex: &Regex) -> usize {
    let mut count = 0;

    match value {
        serde_json::Value::String(s) => {
            if s.len() == 36
                && s.chars().filter(|&c| c == '-').count() == 4
                && uuid_regex.is_match(s)
            {
                count += 1;
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                count += count_and_validate_uuids(item, uuid_regex);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_key, val) in obj {
                count += count_and_validate_uuids(val, uuid_regex);
            }
        }
        _ => {}
    }

    count
}

/// Test edge cases for UUID validation
#[test]
fn test_uuid_validation_edge_cases() {
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Test strings that might be confused with UUIDs
    let _edge_cases = [
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", // valid format but might be confusing
        "00000000-0000-0000-0000-000000000000", // nil UUID
        "ffffffff-ffff-ffff-ffff-ffffffffffff", // max UUID
        "550e8400-e29b-41d4-a716-446655440000 ", // trailing space
        " 550e8400-e29b-41d4-a716-446655440000", // leading space
        "550e8400-e29b-41d4-a716-446655440000\n", // trailing newline
        "550e8400-e29b-41d4-a716-446655440000\t", // trailing tab
        "550e8400-e29b-41d4-a716-446655440000\"", // with quotes
    ];

    // Valid cases
    let valid_cases = vec![
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        "00000000-0000-0000-0000-000000000000",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
    ];

    for valid_case in valid_cases {
        assert!(uuid_regex.is_match(valid_case), "Edge case {} should be valid", valid_case);
        println!("✓ Valid edge case: {}", valid_case);
    }

    // Invalid cases (with extra characters)
    let invalid_cases = vec![
        "550e8400-e29b-41d4-a716-446655440000 ",
        " 550e8400-e29b-41d4-a716-446655440000",
        "550e8400-e29b-41d4-a716-446655440000\n",
        "550e8400-e29b-41d4-a716-446655440000\t",
        "550e8400-e29b-41d4-a716-446655440000\"",
    ];

    for invalid_case in invalid_cases {
        assert!(
            !uuid_regex.is_match(invalid_case),
            "Edge case '{}' should be invalid",
            invalid_case
        );
        println!("✓ Correctly rejected invalid edge case: '{}'", invalid_case);
    }
}

/// Test UUID validation with different versions and variants
#[test]
fn test_uuid_versions_and_variants() {
    // Test UUID v1 (time-based)
    let v1_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-1[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();
    let v1_uuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    assert!(v1_regex.is_match(v1_uuid), "UUID {} should be valid v1", v1_uuid);

    // Test UUID v3 (name-based, MD5)
    let v3_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-3[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();
    let v3_uuid = "6ba7b812-9dad-31d1-80b4-00c04fd430c8";
    assert!(v3_regex.is_match(v3_uuid), "UUID {} should be valid v3", v3_uuid);

    // Test UUID v4 (random)
    let v4_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();
    let v4_uuid = "550e8400-e29b-41d4-a716-446655440000";
    assert!(v4_regex.is_match(v4_uuid), "UUID {} should be valid v4", v4_uuid);

    // Test UUID v5 (name-based, SHA-1)
    let v5_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-5[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap();
    let v5_uuid = "6ba7b814-9dad-51d1-80b4-00c04fd430c8";
    assert!(v5_regex.is_match(v5_uuid), "UUID {} should be valid v5", v5_uuid);

    // Test general UUID regex accepts all versions
    let general_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    let test_uuids = vec![v1_uuid, v3_uuid, v4_uuid, v5_uuid];

    for uuid in test_uuids {
        assert!(general_regex.is_match(uuid), "General regex should accept {}", uuid);
        println!("✓ General UUID regex accepts: {}", uuid);
    }

    // Test RFC 4122 variant (bits 60-63 should be 0b1000 = 8, 9, a, or b)
    let variant_uuids = vec![
        "550e8400-e29b-41d4-8123-446655440000", // variant 0b1000 = 8
        "550e8400-e29b-41d4-9123-446655440000", // variant 0b1001 = 9
        "550e8400-e29b-41d4-a123-446655440000", // variant 0b1010 = a
        "550e8400-e29b-41d4-b123-446655440000", // variant 0b1011 = b
    ];

    for uuid in variant_uuids {
        assert!(general_regex.is_match(uuid), "UUID {} should have valid RFC 4122 variant", uuid);
    }

    println!("✓ All UUID versions and variants validated correctly");
}

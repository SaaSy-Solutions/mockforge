//! Pure comparison of captured mock items and summary calculation.

use super::models::*;
use std::collections::HashMap;

/// Compare two snapshot objects
pub(super) fn compare_snapshot_objects(left: &MockSnapshot, right: &MockSnapshot) -> SnapshotDiff {
    let mut differences = Vec::new();

    // Create maps for quick lookup
    let left_map: HashMap<String, &MockSnapshotItem> =
        left.mocks.iter().map(|m| (format!("{}:{}", m.method, m.path), m)).collect();

    let right_map: HashMap<String, &MockSnapshotItem> =
        right.mocks.iter().map(|m| (format!("{}:{}", m.method, m.path), m)).collect();

    // Find mocks only in left
    for (key, left_mock) in &left_map {
        if !right_map.contains_key(key) {
            differences.push(Difference {
                diff_type: DifferenceType::MissingInRight,
                mock_id: Some(left_mock.id.clone()),
                path: left_mock.path.clone(),
                method: left_mock.method.clone(),
                description: format!(
                    "Mock {} {} exists in left but not in right",
                    left_mock.method, left_mock.path
                ),
                left_value: Some(serde_json::to_value(left_mock).unwrap_or_default()),
                right_value: None,
                field_path: None,
            });
        }
    }

    // Find mocks only in right
    for (key, right_mock) in &right_map {
        if !left_map.contains_key(key) {
            differences.push(Difference {
                diff_type: DifferenceType::MissingInLeft,
                mock_id: Some(right_mock.id.clone()),
                path: right_mock.path.clone(),
                method: right_mock.method.clone(),
                description: format!(
                    "Mock {} {} exists in right but not in left",
                    right_mock.method, right_mock.path
                ),
                left_value: None,
                right_value: Some(serde_json::to_value(right_mock).unwrap_or_default()),
                field_path: None,
            });
        }
    }

    // Compare common mocks
    for (key, left_mock) in &left_map {
        if let Some(right_mock) = right_map.get(key) {
            // Compare status codes
            if left_mock.status_code != right_mock.status_code {
                differences.push(Difference {
                    diff_type: DifferenceType::StatusCodeMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Status code differs: {} vs {}",
                        left_mock.status_code, right_mock.status_code
                    ),
                    left_value: Some(serde_json::json!(left_mock.status_code)),
                    right_value: Some(serde_json::json!(right_mock.status_code)),
                    field_path: Some("status_code".to_string()),
                });
            }

            // Compare response bodies (deep comparison)
            if left_mock.response_body != right_mock.response_body {
                differences.push(Difference {
                    diff_type: DifferenceType::BodyMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Response body differs for {} {}",
                        left_mock.method, left_mock.path
                    ),
                    left_value: Some(left_mock.response_body.clone()),
                    right_value: Some(right_mock.response_body.clone()),
                    field_path: Some("response_body".to_string()),
                });
            }

            // Compare headers
            if left_mock.response_headers != right_mock.response_headers {
                differences.push(Difference {
                    diff_type: DifferenceType::HeadersMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Response headers differ for {} {}",
                        left_mock.method, left_mock.path
                    ),
                    left_value: left_mock
                        .response_headers
                        .as_ref()
                        .map(|h| serde_json::to_value(h).unwrap_or_default()),
                    right_value: right_mock
                        .response_headers
                        .as_ref()
                        .map(|h| serde_json::to_value(h).unwrap_or_default()),
                    field_path: Some("response_headers".to_string()),
                });
            }
        }
    }

    // Calculate summary
    let only_in_left = differences
        .iter()
        .filter(|d| matches!(d.diff_type, DifferenceType::MissingInRight))
        .count();
    let only_in_right = differences
        .iter()
        .filter(|d| matches!(d.diff_type, DifferenceType::MissingInLeft))
        .count();
    let mocks_with_differences: std::collections::HashSet<String> =
        differences.iter().filter_map(|d| d.mock_id.clone()).collect();

    let summary = DiffSummary {
        left_total: left.mocks.len(),
        right_total: right.mocks.len(),
        differences_count: differences.len(),
        only_in_left,
        only_in_right,
        mocks_with_differences: mocks_with_differences.len(),
    };

    SnapshotDiff {
        left: left.clone(),
        right: right.clone(),
        differences,
        summary,
    }
}

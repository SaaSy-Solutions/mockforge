//! JSON patch application logic
//!
//! This module handles applying JSON patches to values with support
//! for both replace and merge modes.

use json_patch::{AddOperation, PatchOperation, RemoveOperation, ReplaceOperation};
use jsonptr::PointerBuf;
use serde_json::Value;

use super::models::PatchOp;
use crate::templating::expand_tokens;

/// Apply a patch operation to a JSON document
pub fn apply_patch(doc: &mut Value, op: &PatchOp) -> anyhow::Result<()> {
    let ops = match op {
        PatchOp::Add { path, value } => vec![PatchOperation::Add(AddOperation {
            path: path.parse().unwrap_or_else(|_| PointerBuf::new()),
            value: value.clone(),
        })],
        PatchOp::Replace { path, value } => vec![PatchOperation::Replace(ReplaceOperation {
            path: path.parse().unwrap_or_else(|_| PointerBuf::new()),
            value: value.clone(),
        })],
        PatchOp::Remove { path } => vec![PatchOperation::Remove(RemoveOperation {
            path: path.parse().unwrap_or_else(|_| PointerBuf::new()),
        })],
    };

    // Apply the patch using the correct function
    json_patch::patch(doc, &ops)?;
    Ok(())
}

/// Apply a merge-style patch operation
pub fn apply_merge_patch(doc: &mut Value, op: &PatchOp) -> anyhow::Result<()> {
    match op {
        PatchOp::Add { path, value } | PatchOp::Replace { path, value } => {
            // For merge mode, we deep merge objects instead of replacing
            merge_at_path(doc, path, value)?;
        }
        PatchOp::Remove { path: _ } => {
            // Remove operations work the same in both modes
            apply_patch(doc, op)?;
        }
    }
    Ok(())
}

/// Deep merge a value at a JSON path
fn merge_at_path(doc: &mut Value, path: &str, value: &Value) -> anyhow::Result<()> {
    if path.is_empty() || path == "/" {
        // Root merge
        merge_values(doc, value);
        return Ok(());
    }

    let ptr: PointerBuf = path.parse().unwrap_or_else(|_| PointerBuf::new());
    let mut current = doc;

    // Navigate to the parent of the target location
    let segments = ptr.as_str().trim_start_matches('/').split('/').collect::<Vec<_>>();
    for (i, segment) in segments.iter().enumerate() {
        let decoded_segment = segment.replace("~1", "/").replace("~0", "~");

        if i == segments.len() - 1 {
            // Last segment - this is where we merge
            match current {
                Value::Object(map) => {
                    if let Some(existing) = map.get_mut(&decoded_segment) {
                        merge_values(existing, value);
                    } else {
                        map.insert(decoded_segment, value.clone());
                    }
                }
                _ => {
                    // Create an object if it doesn't exist
                    *current = serde_json::json!({ decoded_segment: value });
                }
            }
        } else {
            // Navigate deeper
            match current {
                Value::Object(map) => {
                    current = map.entry(decoded_segment).or_insert(Value::Object(serde_json::Map::new()));
                }
                _ => {
                    // Create nested structure
                    let mut new_obj = serde_json::Map::new();
                    new_obj.insert(decoded_segment.to_string(), Value::Object(serde_json::Map::new()));
                    *current = Value::Object(new_obj);
                    current = &mut current[decoded_segment];
                }
            }
        }
    }

    Ok(())
}

/// Deep merge two JSON values
fn merge_values(target: &mut Value, source: &Value) {
    match target {
        Value::Object(target_map) => {
            if let Value::Object(source_map) = source {
                for (key, source_value) in source_map {
                    if let Some(target_value) = target_map.get_mut(key) {
                        merge_values(target_value, source_value);
                    } else {
                        target_map.insert(key.clone(), source_value.clone());
                    }
                }
            } else {
                // Replace with source if types don't match
                *target = source.clone();
            }
        }
        Value::Array(target_arr) => {
            if let Value::Array(source_arr) = source {
                target_arr.extend(source_arr.clone());
            } else {
                // Replace with source if types don't match
                *target = source.clone();
            }
        }
        _ => {
            // For primitives or other types, replace
            *target = source.clone();
        }
    }
}

/// Apply post-templating expansion to a value
pub fn apply_post_templating(value: &mut Value) {
    *value = expand_tokens(value);
}

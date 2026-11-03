//! Merge-patch traits and preserve original target null values.
use serde_json::{json, Value};

pub(super) fn overlay_target(merged: &mut Value, target: &Value) {
    match (merged, target) {
        (Value::Object(merged), Value::Object(target)) => {
            for (name, value) in target {
                if let Some(existing) = merged.get_mut(name) {
                    overlay_target(existing, value);
                } else {
                    merged.insert(name.clone(), value.clone());
                }
            }
        }
        (merged, target) => *merged = target.clone(),
    }
}

pub(super) fn merge_patch(target: &mut Value, patch: &Value) {
    if let Value::Object(patch) = patch {
        if !target.is_object() {
            *target = json!({});
        }
        let map = target.as_object_mut().expect("object initialized");
        for (name, value) in patch {
            if value.is_null() {
                map.remove(name);
            } else {
                merge_patch(map.entry(name.clone()).or_insert(Value::Null), value);
            }
        }
    } else {
        *target = patch.clone();
    }
}

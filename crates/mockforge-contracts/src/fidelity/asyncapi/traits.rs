//! Resolve trait defaults while retaining explicit target values.
use super::merge::{merge_patch, overlay_target};
use super::{object_mut, references};
use serde_json::{Map, Value};

pub(super) fn apply_traits(value: &mut Value) -> Result<(), String> {
    let map = object_mut(value)?;
    if let Some(traits) = map.remove("traits") {
        let mut merged = Value::Object(Map::new());
        for item in traits.as_array().ok_or("AsyncAPI traits must be an array")? {
            if !item.is_object() || item.get("traits").is_some() {
                return Err("AsyncAPI trait must be an object without nested traits".into());
            }
            merge_patch(&mut merged, item);
        }
        overlay_target(&mut merged, &Value::Object(map.clone()));
        *map = merged.as_object().ok_or("Invalid merged AsyncAPI traits")?.clone();
    }
    Ok(())
}

pub(in crate::fidelity) fn effective(source: &Value, raw: &Value) -> Result<Value, String> {
    let raw = references::resolve(source, raw)?.1;
    let mut target = raw.as_object().ok_or("AsyncAPI contract must be an object")?.clone();
    let mut merged = serde_json::json!({});
    if let Some(traits) = target.remove("traits") {
        for item in traits.as_array().ok_or("Traits must be an array")? {
            let item = references::resolve(source, item)?.1;
            if !item.is_object() || item.get("traits").is_some() {
                return Err("Invalid nested AsyncAPI trait".into());
            }
            merge_patch(&mut merged, item);
        }
    }
    overlay_target(&mut merged, &Value::Object(target));
    Ok(merged)
}

//! Structural differences between two JSON values, addressed by JSON pointer.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

/// How one value differs from the other.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    /// Present only in the new value.
    Added,
    /// Present only in the old value.
    Removed,
    /// Present in both, with different content.
    Changed,
}

/// One difference between two values.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Delta {
    /// JSON pointer (RFC 6901) to the value, relative to the compared root.
    pub location: String,
    /// How the value differs.
    pub change: Change,
    /// The old value, when there was one.
    pub before: Option<Value>,
    /// The new value, when there is one.
    pub after: Option<Value>,
}

/// Append every difference between `before` and `after` to `out`.
///
/// Objects are compared key by key and arrays position by position, so a
/// reordered array is reported as changes. Objects that differ only in key
/// order are equal.
pub fn delta(before: Option<&Value>, after: Option<&Value>, location: &str, out: &mut Vec<Delta>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Some(Value::Object(old)), Some(Value::Object(new))) => {
            let keys: BTreeSet<&str> =
                old.keys().map(String::as_str).chain(new.keys().map(String::as_str)).collect();
            for key in keys {
                delta(old.get(key), new.get(key), &pointer(location, key), out);
            }
        }
        (Some(Value::Array(old)), Some(Value::Array(new))) => {
            for (index, (x, y)) in old.iter().zip(new).enumerate() {
                delta(Some(x), Some(y), &format!("{location}/{index}"), out);
            }
            for (index, value) in old.iter().enumerate().skip(new.len()) {
                out.push(tail(location, index, Change::Removed, value));
            }
            for (index, value) in new.iter().enumerate().skip(old.len()) {
                out.push(tail(location, index, Change::Added, value));
            }
        }
        _ => out.push(Delta {
            location: location.to_string(),
            change: if before.is_none() {
                Change::Added
            } else if after.is_none() {
                Change::Removed
            } else {
                Change::Changed
            },
            before: before.cloned(),
            after: after.cloned(),
        }),
    }
}

/// A copy of a JSON Schema in which every `required` array is sorted, so two
/// schemas that differ only in the order of `required` compare equal. JSON
/// Schema gives `required` set semantics; every other array keeps its order.
pub fn sorted_required(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let sorted = if key == "required" {
                        sort_string_array(value)
                    } else {
                        sorted_required(value)
                    };
                    (key.clone(), sorted)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(sorted_required).collect()),
        other => other.clone(),
    }
}

fn sort_string_array(value: &Value) -> Value {
    match value {
        Value::Array(items) if items.iter().all(Value::is_string) => {
            let mut strings: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            strings.sort_unstable();
            Value::Array(strings.into_iter().map(Value::from).collect())
        }
        other => other.clone(),
    }
}

fn tail(location: &str, index: usize, change: Change, value: &Value) -> Delta {
    Delta {
        location: format!("{location}/{index}"),
        change,
        before: (change == Change::Removed).then(|| value.clone()),
        after: (change == Change::Added).then(|| value.clone()),
    }
}

fn pointer(location: &str, key: &str) -> String {
    let escaped = key.replace('~', "~0").replace('/', "~1");
    format!("{location}/{escaped}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(before: Value, after: Value) -> Vec<Delta> {
        let mut out = Vec::new();
        delta(Some(&before), Some(&after), "", &mut out);
        out
    }

    #[test]
    fn equal_values_have_no_deltas_whatever_the_key_order() {
        let mut out = Vec::new();
        delta(
            Some(&json!({"b": 1, "a": {"y": 2, "x": 3}})),
            Some(&json!({"a": {"x": 3, "y": 2}, "b": 1})),
            "",
            &mut out,
        );
        assert!(out.is_empty());
        assert!(run(json!(null), json!(null)).is_empty());
    }

    #[test]
    fn object_keys_are_added_removed_and_changed_at_their_pointers() {
        let out = run(
            json!({"keep": 1, "gone": 2, "same": 3}),
            json!({"keep": 4, "added": 5, "same": 3}),
        );
        let shape: Vec<(&str, Change)> =
            out.iter().map(|d| (d.location.as_str(), d.change)).collect();
        assert_eq!(
            shape,
            vec![
                ("/added", Change::Added),
                ("/gone", Change::Removed),
                ("/keep", Change::Changed),
            ]
        );
        assert_eq!(out[0].after, Some(json!(5)));
        assert_eq!(out[1].before, Some(json!(2)));
        assert_eq!(out[2].before, Some(json!(1)));
        assert_eq!(out[2].after, Some(json!(4)));
    }

    #[test]
    fn arrays_compare_by_position_and_report_the_tail() {
        let out = run(json!([1, 2, 3]), json!([1, 9]));
        assert_eq!(
            out.iter().map(|d| (d.location.as_str(), d.change)).collect::<Vec<_>>(),
            vec![("/1", Change::Changed), ("/2", Change::Removed)]
        );
        let out = run(json!([1]), json!([1, 2, 3]));
        assert_eq!(
            out.iter().map(|d| (d.location.as_str(), d.change)).collect::<Vec<_>>(),
            vec![("/1", Change::Added), ("/2", Change::Added)]
        );
    }

    #[test]
    fn pointer_characters_in_keys_are_escaped_and_a_type_change_is_one_delta() {
        let out = run(json!({"a/b": {"c~d": 1}}), json!({"a/b": {"c~d": "one"}}));
        assert_eq!(out[0].location, "/a~1b/c~0d");
        assert_eq!(out[0].change, Change::Changed);

        let out = run(json!({"n": [1]}), json!({"n": {"0": 1}}));
        assert_eq!(out[0].location, "/n");
        assert_eq!(out[0].change, Change::Changed);
    }

    #[test]
    fn sorted_required_orders_only_required_arrays_of_strings() {
        let schema = json!({
            "type": "object",
            "required": ["b", "a"],
            "enum": ["b", "a"],
            "properties": {"n": {"required": ["z", "y"], "type": "number", "prefixItems": [2, 1]}},
            "required_extra": {"required": [2, 1]}
        });
        let sorted = sorted_required(&schema);
        assert_eq!(sorted["required"], json!(["a", "b"]));
        assert_eq!(sorted["properties"]["n"]["required"], json!(["y", "z"]));
        // Order is meaning everywhere else, including an array that is not all strings.
        assert_eq!(sorted["enum"], json!(["b", "a"]));
        assert_eq!(sorted["properties"]["n"]["prefixItems"], json!([2, 1]));
        assert_eq!(
            sorted_required(&json!({"required": ["b", 1, "a"]})),
            json!({"required": ["b", 1, "a"]})
        );
        // And the input is left alone.
        assert_eq!(schema["required"], json!(["b", "a"]));
    }
}

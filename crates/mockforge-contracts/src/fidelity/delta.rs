//! Field-level deltas over normalized operation definitions.

use super::FieldDelta;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn diff(
    mock: Option<&Value>,
    source: Option<&Value>,
    path: &str,
    out: &mut Vec<FieldDelta>,
) {
    if mock == source {
        return;
    }
    if let (Some(Value::Object(before)), Some(Value::Object(after))) = (mock, source) {
        let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            diff(before.get(key), after.get(key), &format!("{path}/{escaped}"), out);
        }
    } else {
        out.push(FieldDelta {
            location: path.into(),
            change: if mock.is_none() {
                "added"
            } else if source.is_none() {
                "removed"
            } else {
                "changed"
            }
            .into(),
            mock: mock.cloned(),
            source: source.cloned(),
        });
    }
}

//! Normalize volatile metadata and object ordering without changing array order.
use crate::workspace::{Folder, MockRequest};
use serde_json::{Map, Value};

pub(super) fn normalize_requests(requests: &mut [MockRequest]) {
    for request in requests {
        request.created_at = chrono::DateTime::UNIX_EPOCH;
        request.updated_at = chrono::DateTime::UNIX_EPOCH;
        request.response_history.clear();
    }
}

pub(super) fn normalize_folders(folders: &mut [Folder]) {
    for folder in folders {
        folder.created_at = chrono::DateTime::UNIX_EPOCH;
        folder.updated_at = chrono::DateTime::UNIX_EPOCH;
        normalize_requests(&mut folder.requests);
        normalize_folders(&mut folder.folders);
    }
}

pub(super) fn canonicalize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (key, mut value) in entries {
                canonicalize(&mut value);
                sorted.insert(key, value);
            }
            *map = sorted;
        }
        Value::Array(values) => {
            for value in values {
                canonicalize(value);
            }
        }
        _ => {}
    }
}

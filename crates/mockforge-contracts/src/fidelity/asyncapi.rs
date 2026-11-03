//! Effective AsyncAPI 2/3 channel and operation contracts, without resource I/O.

use super::{context::Context, projection::Projection, references};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

mod definition;
mod merge;
mod messages;
mod operations;
mod security;
mod servers;
mod traits;
pub(super) use traits::effective;

struct Document<'a> {
    source: &'a Value,
    projection: Projection<'a>,
    major: u8,
}

pub(super) fn operations(source: &Value, budget: usize) -> Result<BTreeMap<String, Value>, String> {
    let version = source
        .get("asyncapi")
        .and_then(Value::as_str)
        .ok_or("Missing AsyncAPI version")?;
    let major = if version.starts_with("2.") {
        2
    } else if version.starts_with("3.") {
        3
    } else {
        return Err("Only AsyncAPI 2.x and 3.x definitions are supported".into());
    };
    if !["title", "version"].iter().all(|key| {
        source
            .get("info")
            .and_then(|info| info.get(key))
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
    }) {
        return Err("AsyncAPI info must declare a title and version".into());
    }
    let channels = source
        .get("channels")
        .and_then(Value::as_object)
        .ok_or("AsyncAPI channels must be an object")?;
    let mut document = Document {
        source,
        projection: Projection::new(source, budget),
        major,
    };
    let mut out = BTreeMap::new();
    for (name, channel) in channels {
        let channel = references::resolve(source, channel)?.1;
        let canonical = document.channel(name, channel)?;
        out.insert(format!("CHANNEL {name}"), canonical.clone());
        if major == 2 {
            for (field, action) in [("publish", "RECEIVE"), ("subscribe", "SEND")] {
                if let Some(operation) = channel.get(field) {
                    let mut operation = document.contract(operation)?;
                    document.messages_v2(&mut operation)?;
                    object_mut(&mut operation)?.insert("channel".into(), canonical.clone());
                    out.insert(format!("{action} {name}"), operation);
                }
            }
        }
    }
    if major == 3 {
        document.operations_v3(&mut out)?;
    }
    Ok(out)
}

fn object_mut(value: &mut Value) -> Result<&mut Map<String, Value>, String> {
    value
        .as_object_mut()
        .ok_or_else(|| "AsyncAPI contract must be an object".into())
}

#[cfg(test)]
mod tests;

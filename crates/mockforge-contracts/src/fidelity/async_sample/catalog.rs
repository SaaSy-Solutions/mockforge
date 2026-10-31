//! Resolve messages without replacing recursive payload references with comparison markers.
use super::super::{asyncapi, references};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) struct Operation {
    pub endpoint: String,
    pub channel_endpoint: String,
    pub action: String,
    pub address: Option<Regex>,
    pub protocols: Vec<String>,
    pub messages: Vec<Value>,
}

pub(super) fn build(
    source: &Value,
    budget: usize,
) -> Result<(BTreeMap<String, Value>, Vec<Operation>), String> {
    let definitions = asyncapi::operations(source, budget)?;
    let channels = source["channels"].as_object().ok_or("Missing AsyncAPI channels")?;
    let mut out = Vec::new();
    if source["asyncapi"].as_str().is_some_and(|v| v.starts_with("2.")) {
        for (name, channel) in channels {
            let channel = references::resolve(source, channel)?.1;
            for (key, action) in [("publish", "RECEIVE"), ("subscribe", "SEND")] {
                if let Some(raw) = channel.get(key) {
                    let operation = asyncapi::effective(source, raw)?;
                    let raw = operation.get("message").ok_or("Missing AsyncAPI message")?;
                    let root = references::resolve(source, raw)?.1;
                    let messages = if let Some(alternatives) = root.get("oneOf") {
                        alternatives
                            .as_array()
                            .ok_or("Message oneOf must be an array")?
                            .iter()
                            .map(|v| message(source, v))
                            .collect::<Result<_, _>>()?
                    } else {
                        vec![message(source, root)?]
                    };
                    let endpoint = format!("{action} {name}");
                    out.push(operation_record(
                        &definitions,
                        endpoint,
                        format!("CHANNEL {name}"),
                        action,
                        messages,
                    )?);
                }
            }
        }
    } else if let Some(operations) = source.get("operations").and_then(Value::as_object) {
        for (name, raw) in operations {
            let raw = asyncapi::effective(source, raw)?;
            let reference = raw["channel"]["$ref"].as_str().ok_or("Missing channel reference")?;
            let channel_name = reference
                .strip_prefix("#/channels/")
                .ok_or("Invalid channel reference")?
                .replace("~1", "/")
                .replace("~0", "~");
            let channel = references::resolve(source, &raw["channel"])?.1;
            let action = raw["action"].as_str().ok_or("Missing operation action")?.to_uppercase();
            let address = channel["address"].as_str().unwrap_or("<dynamic>");
            let messages = if let Some(selected) = raw.get("messages") {
                selected
                    .as_array()
                    .ok_or("Messages must be an array")?
                    .iter()
                    .map(|raw| message(source, raw))
                    .collect::<Result<_, _>>()?
            } else {
                channel
                    .get("messages")
                    .and_then(Value::as_object)
                    .map(|m| {
                        m.values().map(|raw| message(source, raw)).collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .unwrap_or_default()
            };
            out.push(operation_record(
                &definitions,
                format!("{action} {address} [{name}]"),
                format!("CHANNEL {channel_name}"),
                &action,
                messages,
            )?);
        }
    }
    Ok((definitions, out))
}

fn operation_record(
    definitions: &BTreeMap<String, Value>,
    endpoint: String,
    channel_endpoint: String,
    action: &str,
    messages: Vec<Value>,
) -> Result<Operation, String> {
    let channel = &definitions.get(&endpoint).ok_or("Missing canonical operation")?["channel"];
    let mut protocols: Vec<_> = channel["servers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|server| server["protocol"].as_str().map(str::to_ascii_lowercase))
        .collect();
    protocols.sort();
    protocols.dedup();
    let address = channel["address"].as_str().map(address_pattern).transpose()?;
    Ok(Operation {
        endpoint,
        channel_endpoint,
        action: action.into(),
        address,
        protocols,
        messages,
    })
}

fn message(source: &Value, raw: &Value) -> Result<Value, String> {
    let mut message = asyncapi::effective(source, raw)?;
    if message.get("contentType").is_none() {
        if let Some(default) = source.get("defaultContentType") {
            message
                .as_object_mut()
                .ok_or("Message must be an object")?
                .insert("contentType".into(), default.clone());
        }
    }
    Ok(message)
}

fn address_pattern(address: &str) -> Result<Regex, String> {
    if address.len() > 4096 {
        return Err("Channel address exceeds 4096 bytes".into());
    }
    let mut pattern = String::from(r"\A");
    let mut rest = address;
    while let Some(start) = rest.find('{') {
        pattern.push_str(&regex::escape(&rest[..start]));
        let end =
            rest[start + 1..].find('}').ok_or("Unclosed channel address expression")? + start + 1;
        if end == start + 1 || rest[start + 1..end].contains('{') {
            return Err("Invalid channel address expression".into());
        }
        pattern.push_str("[^/]+");
        rest = &rest[end + 1..];
    }
    pattern.push_str(&regex::escape(rest));
    pattern.push_str(r"\z");
    Regex::new(&pattern).map_err(|_| "Unsupported channel address expression".into())
}

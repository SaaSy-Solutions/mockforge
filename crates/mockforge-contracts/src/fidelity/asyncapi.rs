//! Effective AsyncAPI 2/3 channel and operation contracts, without resource I/O.

use super::{context::Context, projection::Projection, references};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

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

impl Document<'_> {
    fn contract(&mut self, raw: &Value) -> Result<Value, String> {
        let effective = effective(self.source, raw)?;
        let mut value = self.projection.value(&effective, Context::AsyncContract, 0)?;
        apply_traits(&mut value)?;
        if !value.is_object() {
            return Err("AsyncAPI contract must be an object".into());
        }
        if let Some(security) = value.get("security").cloned() {
            object_mut(&mut value)?.insert("security".into(), self.security(&security)?);
        }
        Ok(value)
    }

    fn channel(&mut self, name: &str, raw: &Value) -> Result<Value, String> {
        let mut value = self.contract(raw)?;
        let map = object_mut(&mut value)?;
        map.remove("publish");
        map.remove("subscribe");
        if self.major == 2 {
            map.insert("address".into(), json!(name));
        } else {
            let address = map.entry("address").or_insert(Value::Null);
            if !address.is_null() && !address.is_string() {
                return Err("AsyncAPI channel address must be a string or null".into());
            }
        }
        map.insert("servers".into(), self.servers(raw)?);
        if let Some(messages) = map.get_mut("messages").and_then(Value::as_object_mut) {
            for message in messages.values_mut() {
                self.message_defaults(message)?;
            }
        }
        Ok(value)
    }

    fn servers(&mut self, channel: &Value) -> Result<Value, String> {
        let all = self.source.get("servers").and_then(Value::as_object);
        let selected = match channel.get("servers") {
            Some(Value::Array(selected)) if !selected.is_empty() => selected.clone(),
            None | Some(Value::Array(_)) => all
                .map(|servers| {
                    servers
                        .keys()
                        .map(|name| {
                            if self.major == 2 {
                                json!(name)
                            } else {
                                json!({"$ref":format!("#/servers/{}", pointer_token(name))})
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => return Err("AsyncAPI channel servers must be an array".into()),
        };
        let mut servers = Vec::new();
        for entry in selected {
            let raw = if self.major == 2 {
                entry
                    .as_str()
                    .and_then(|name| all.and_then(|all| all.get(name)))
                    .ok_or("AsyncAPI channel references an unknown server")?
            } else {
                references::resolve(self.source, &entry)?.1
            };
            let server = self.contract(raw)?;
            servers.push(server);
        }
        servers.sort_by_key(Value::to_string);
        servers.dedup();
        Ok(Value::Array(servers))
    }

    fn security(&mut self, security: &Value) -> Result<Value, String> {
        let requirements = security.as_array().ok_or("AsyncAPI security must be an array")?;
        let mut out = Vec::new();
        for requirement in requirements {
            if self.major == 3 {
                out.push(self.security_scheme(requirement)?);
                continue;
            }
            let mut schemes = Map::new();
            for (name, scopes) in
                requirement.as_object().ok_or("Invalid AsyncAPI security requirement")?
            {
                let schema = self
                    .source
                    .get("components")
                    .and_then(|c| c.get("securitySchemes"))
                    .and_then(|s| s.get(name))
                    .ok_or("Undefined AsyncAPI security scheme")?;
                let mut scopes =
                    scopes.as_array().ok_or("Security scopes must be an array")?.clone();
                scopes.sort_by_key(Value::to_string);
                schemes.insert(
                    name.clone(),
                    json!({"scheme":self.security_scheme(schema)?,"scopes":scopes}),
                );
            }
            out.push(Value::Object(schemes));
        }
        out.sort_by_key(Value::to_string);
        Ok(Value::Array(out))
    }

    fn security_scheme(&mut self, raw: &Value) -> Result<Value, String> {
        let mut scheme = self.projection.value(raw, Context::AsyncContract, 0)?;
        apply_traits(&mut scheme)?;
        if scheme.get("type").and_then(Value::as_str).is_none() || scheme.get("security").is_some()
        {
            return Err(
                "AsyncAPI security scheme must declare a type without nested security".into()
            );
        }
        Ok(scheme)
    }

    fn message_defaults(&mut self, message: &mut Value) -> Result<(), String> {
        apply_traits(message)?;
        let map = object_mut(message)?;
        if !map.contains_key("contentType") {
            if let Some(content_type) = self.source.get("defaultContentType") {
                map.insert("contentType".into(), content_type.clone());
            }
        }
        Ok(())
    }

    fn messages_v2(&mut self, operation: &mut Value) -> Result<(), String> {
        let Some(message) = operation.get_mut("message") else {
            return Err("AsyncAPI 2 operation must declare a message".into());
        };
        if let Some(messages) = message.get_mut("oneOf") {
            let messages = messages.as_array_mut().ok_or("Message oneOf must be an array")?;
            if messages.is_empty() {
                return Err("Message oneOf must not be empty".into());
            }
            for message in messages.iter_mut() {
                self.message_defaults(message)?;
            }
            messages.sort_by_key(Value::to_string);
        } else {
            self.message_defaults(message)?;
        }
        Ok(())
    }

    fn operations_v3(&mut self, out: &mut BTreeMap<String, Value>) -> Result<(), String> {
        let Some(operations) = self.source.get("operations") else {
            return Ok(());
        };
        for (name, raw) in operations.as_object().ok_or("AsyncAPI operations must be an object")? {
            let raw = effective(self.source, raw)?;
            let action = raw
                .get("action")
                .and_then(Value::as_str)
                .filter(|action| matches!(*action, "send" | "receive"))
                .ok_or("AsyncAPI operation action must be send or receive")?;
            let channel_ref = raw
                .get("channel")
                .and_then(|c| c.get("$ref"))
                .and_then(Value::as_str)
                .filter(|r| {
                    r.strip_prefix("#/channels/")
                        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
                })
                .ok_or("Root AsyncAPI operation must reference a root channel")?;
            let channel = raw.get("channel").ok_or("AsyncAPI operation must declare a channel")?;
            let channel = references::resolve(self.source, channel)?.1;
            let canonical = self.channel(channel_ref, channel)?;
            let mut operation = self.contract(&raw)?;
            let map = object_mut(&mut operation)?;
            map.insert("channel".into(), canonical.clone());
            let messages = if let Some(messages) = raw.get("messages") {
                self.selected_messages(channel_ref, messages)?
            } else {
                canonical
                    .get("messages")
                    .and_then(Value::as_object)
                    .map(|m| m.values().cloned().collect())
                    .unwrap_or_default()
            };
            let mut messages = messages;
            messages.sort_by_key(Value::to_string);
            map.insert("messages".into(), Value::Array(messages));
            let address = channel.get("address").and_then(Value::as_str).unwrap_or("<dynamic>");
            out.insert(format!("{} {address} [{name}]", action.to_uppercase()), operation);
        }
        Ok(())
    }

    fn selected_messages(
        &mut self,
        channel_ref: &str,
        messages: &Value,
    ) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        for message in messages.as_array().ok_or("AsyncAPI operation messages must be an array")? {
            let _reference = message
                .get("$ref")
                .and_then(Value::as_str)
                .filter(|r| {
                    r.strip_prefix(&format!("{channel_ref}/messages/"))
                        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
                })
                .ok_or("Operation message must reference a message in its channel")?;
            let raw = references::resolve(self.source, message)?.1;
            let mut message = self.contract(raw)?;
            self.message_defaults(&mut message)?;
            out.push(message);
        }
        out.sort_by_key(Value::to_string);
        Ok(out)
    }
}

fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// Traits fill unspecified fields; explicit target fields always win.
fn apply_traits(value: &mut Value) -> Result<(), String> {
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

/// Apply local trait references while leaving message schema references intact.
pub(super) fn effective(source: &Value, raw: &Value) -> Result<Value, String> {
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

/// Explicit target values, including null, retain precedence over trait defaults.
fn overlay_target(merged: &mut Value, target: &Value) {
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

#[cfg(test)]
mod tests;

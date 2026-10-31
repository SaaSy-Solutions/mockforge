//! Operations contract normalization.
use super::{effective, object_mut, references, Document};
use serde_json::Value;
use std::collections::BTreeMap;

impl Document<'_> {
    pub(super) fn operations_v3(
        &mut self,
        out: &mut BTreeMap<String, Value>,
    ) -> Result<(), String> {
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
}

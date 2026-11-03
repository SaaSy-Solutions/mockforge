//! Messages contract normalization.
use super::{object_mut, references, traits, Document};
use serde_json::Value;

impl Document<'_> {
    pub(super) fn message_defaults(&mut self, message: &mut Value) -> Result<(), String> {
        traits::apply_traits(message)?;
        let map = object_mut(message)?;
        if !map.contains_key("contentType") {
            if let Some(content_type) = self.source.get("defaultContentType") {
                map.insert("contentType".into(), content_type.clone());
            }
        }
        Ok(())
    }

    pub(super) fn messages_v2(&mut self, operation: &mut Value) -> Result<(), String> {
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

    pub(super) fn selected_messages(
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

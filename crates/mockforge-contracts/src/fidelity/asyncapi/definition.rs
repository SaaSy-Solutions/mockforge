//! Definition contract normalization.
use super::{object_mut, traits, Context, Document};
use serde_json::{json, Value};

impl Document<'_> {
    pub(super) fn contract(&mut self, raw: &Value) -> Result<Value, String> {
        let effective = traits::effective(self.source, raw)?;
        let mut value = self.projection.value(&effective, Context::AsyncContract, 0)?;
        traits::apply_traits(&mut value)?;
        if !value.is_object() {
            return Err("AsyncAPI contract must be an object".into());
        }
        if let Some(security) = value.get("security").cloned() {
            object_mut(&mut value)?.insert("security".into(), self.security(&security)?);
        }
        Ok(value)
    }

    pub(super) fn channel(&mut self, name: &str, raw: &Value) -> Result<Value, String> {
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
}

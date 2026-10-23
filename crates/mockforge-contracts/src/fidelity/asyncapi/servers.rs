//! Servers contract normalization.
use super::{references, Document};
use serde_json::{json, Value};

impl Document<'_> {
    pub(super) fn servers(&mut self, channel: &Value) -> Result<Value, String> {
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
}

fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

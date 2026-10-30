//! Bounded expansion into comparable contracts, with recursive-reference markers.

use super::{context::Context, references};
use serde_json::{Map, Value};

pub(super) struct Projection<'a> {
    document: &'a Value,
    nodes_left: usize,
    references: Vec<String>,
}

impl<'a> Projection<'a> {
    pub(super) fn new(document: &'a Value) -> Self {
        Self {
            document,
            nodes_left: 200_000,
            references: Vec::new(),
        }
    }

    pub(super) fn value(
        &mut self,
        value: &Value,
        context: Context,
        depth: usize,
    ) -> Result<Value, String> {
        if depth > 128 || self.nodes_left == 0 {
            return Err("contract reference expansion exceeds the safe comparison limit".into());
        }
        self.nodes_left -= 1;
        if !matches!(context, Context::Literal)
            && value.get("$ref").and_then(Value::as_str).is_some()
        {
            return self.reference(value, context, depth);
        }
        match value {
            Value::Object(map) => self.object(map, context, depth),
            Value::Array(items) => {
                items.iter().map(|v| self.value(v, context, depth + 1)).collect()
            }
            _ => Ok(value.clone()),
        }
    }

    fn reference(
        &mut self,
        value: &Value,
        context: Context,
        depth: usize,
    ) -> Result<Value, String> {
        let (target, resolved) = references::resolve(self.document, value)?;
        if let Some(index) = self.references.iter().position(|r| r == target) {
            return Ok(serde_json::json!({"$recursiveRef":self.references.len()-index}));
        }
        self.references.push(target.into());
        let result = self.value(resolved, context, depth + 1);
        self.references.pop();
        result
    }

    fn object(
        &mut self,
        map: &Map<String, Value>,
        context: Context,
        depth: usize,
    ) -> Result<Value, String> {
        let mut out = Map::new();
        for (key, child) in map {
            let Some(next) = context.child(key) else {
                continue;
            };
            let mut projected = self.value(child, next, depth + 1)?;
            if context.is_set(key) {
                if let Some(items) = projected.as_array_mut() {
                    items.sort_by_key(Value::to_string);
                }
            }
            out.insert(key.clone(), projected);
        }
        Ok(Value::Object(out))
    }
}

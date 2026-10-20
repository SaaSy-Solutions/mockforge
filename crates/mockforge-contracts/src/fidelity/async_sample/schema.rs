//! Bundle only reachable local schemas, preserving cycles and denying all retrieval.
use super::super::context::Context;
use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

struct LocalOnly;
impl Retrieve for LocalOnly {
    fn retrieve(&self, _: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("AsyncAPI fidelity cannot fetch schema resources".into())
    }
}

pub(super) enum Prepared {
    Compiled(Box<Validator>),
    Unsupported(Vec<&'static str>),
}

pub(super) fn compile(schema: &Value, source: &Value) -> Result<Prepared, String> {
    let mut bundle = Bundle {
        source,
        references: BTreeMap::new(),
        pending: Vec::new(),
        nodes: 200_000,
        unsupported: BTreeSet::new(),
    };
    let root = bundle.value(schema, Context::Schema, 0)?;
    let mut definitions = Map::new();
    let mut index = 0;
    while index < bundle.pending.len() {
        let pointer = bundle.pending[index].clone();
        let raw = source.pointer(&pointer).ok_or("Unresolved local message schema reference")?;
        let value = bundle.value(raw, Context::Schema, 0)?;
        definitions.insert(format!("ref{index}"), value);
        index += 1;
    }
    if !bundle.unsupported.is_empty() {
        return Ok(Prepared::Unsupported(bundle.unsupported.into_iter().collect()));
    }
    // A fresh wrapper avoids collisions with user-defined definitions/$defs.
    let document = serde_json::json!({"allOf":[root],"definitions":definitions});
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .with_retriever(LocalOnly)
        .build(&document)
        .map(|validator| Prepared::Compiled(Box::new(validator)))
        .map_err(|e| format!("Invalid message schema: {e}"))
}

struct Bundle<'a> {
    source: &'a Value,
    references: BTreeMap<String, usize>,
    pending: Vec<String>,
    nodes: usize,
    unsupported: BTreeSet<&'static str>,
}
impl Bundle<'_> {
    fn annotations(&mut self, map: &Map<String, Value>) -> Result<(), String> {
        if map.contains_key("$id") {
            self.unsupported
                .insert("Message schema resolution bases ($id) are not supported");
        }
        if let Some(dialect) = map.get("$schema") {
            let dialect = dialect
                .as_str()
                .ok_or("Message schema dialect must be a URI string")?
                .trim_end_matches('#');
            if !matches!(
                dialect,
                "http://json-schema.org/draft-07/schema"
                    | "https://json-schema.org/draft-07/schema"
            ) {
                self.unsupported.insert("Explicit message schema dialect is not supported");
            }
        }
        Ok(())
    }

    fn value(&mut self, value: &Value, context: Context, depth: usize) -> Result<Value, String> {
        if depth > 128 || self.nodes == 0 {
            return Err("Message schema exceeds the safe validation limit".into());
        }
        self.nodes -= 1;
        match value {
            Value::Object(map) => {
                if !matches!(context, Context::Literal | Context::Schemas) {
                    self.annotations(map)?;
                }
                let mut out = Map::new();
                for (key, value) in map {
                    if key == "$ref" && !matches!(context, Context::Literal | Context::Schemas) {
                        let pointer = value
                            .as_str()
                            .and_then(|r| r.strip_prefix('#'))
                            .filter(|p| p.starts_with('/'))
                            .ok_or("External or unsupported message schema reference")?;
                        if self.source.pointer(pointer).is_none() {
                            return Err("Unresolved local message schema reference".into());
                        }
                        let next = self.references.len();
                        let index = *self.references.entry(pointer.into()).or_insert_with(|| {
                            self.pending.push(pointer.into());
                            next
                        });
                        out.insert(key.clone(), Value::String(format!("#/definitions/ref{index}")));
                    } else if let Some(next) = context.child(key) {
                        if matches!(next, Context::Literal) {
                            out.insert(key.clone(), value.clone());
                        } else {
                            out.insert(key.clone(), self.value(value, next, depth + 1)?);
                        }
                    } else {
                        out.insert(key.clone(), value.clone());
                    }
                }
                Ok(Value::Object(out))
            }
            Value::Array(values) => {
                values.iter().map(|v| self.value(v, context, depth + 1)).collect()
            }
            _ => Ok(value.clone()),
        }
    }
}

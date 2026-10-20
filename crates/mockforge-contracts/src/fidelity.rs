//! Structural fidelity between a mock's OpenAPI definition and its source.
//!
//! This is a definition comparison, rather than a compatibility policy: added
//! optional fields and new operations are drift too. No network access occurs.

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// One change, with a JSON pointer relative to the operation's contract.
#[derive(Debug, Serialize)]
pub struct FieldDelta {
    /// JSON pointer; `/` and `~` in names are escaped per RFC 6901.
    pub location: String,
    /// `added`, `removed`, or `changed`, from mock to source.
    pub change: String,
    /// Value in the mock definition (absent for additions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<Value>,
    /// Value in the source specification (absent for removals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Value>,
}

/// Fidelity for one method and path, including endpoints present on one side only.
#[derive(Debug, Serialize)]
pub struct RouteFidelity {
    /// Uppercase HTTP method and OpenAPI path template.
    pub endpoint: String,
    /// `verified` or `drift-detected`.
    pub status: String,
    /// `unchanged`, `added`, `removed`, or `changed`, from mock to source.
    pub change: String,
    /// Field-level contract differences.
    pub deltas: Vec<FieldDelta>,
}

/// Deterministic report shared by command-line and admin consumers.
#[derive(Debug, Serialize)]
pub struct FidelityReport {
    /// Version of this serialized report format.
    pub format_version: u8,
    /// Number of routes with a changed definition or presence.
    pub routes_with_drift: usize,
    /// All routes, including unchanged routes with verified status.
    pub routes: Vec<RouteFidelity>,
}

/// Compare OpenAPI 3 definitions. Inputs must be parsed/validated by the caller.
/// Local references are resolved, including recursive schemas; unsupported or
/// missing references and excessive expansion return an error, never a pass.
pub fn compare_mock_spec(mock: &Value, source: &Value) -> Result<FidelityReport, String> {
    let mock_routes = operations(mock)?;
    let source_routes = operations(source)?;
    if mock_routes.is_empty() && source_routes.is_empty() {
        return Err("neither definition contains an HTTP operation to compare".into());
    }
    let endpoints: BTreeSet<_> = mock_routes.keys().chain(source_routes.keys()).collect();
    let mut routes = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        let before = mock_routes.get(endpoint);
        let after = source_routes.get(endpoint);
        let mut deltas = Vec::new();
        diff(before, after, "", &mut deltas);
        let change = match (before, after, deltas.is_empty()) {
            (_, _, true) => "unchanged",
            (None, _, _) => "added",
            (_, None, _) => "removed",
            _ => "changed",
        };
        routes.push(RouteFidelity {
            endpoint: endpoint.clone(),
            status: if deltas.is_empty() {
                "verified"
            } else {
                "drift-detected"
            }
            .into(),
            change: change.into(),
            deltas,
        });
    }
    Ok(FidelityReport {
        format_version: 1,
        routes_with_drift: routes.iter().filter(|r| !r.deltas.is_empty()).count(),
        routes,
    })
}

#[derive(Clone, Copy)]
enum Context {
    Contract,
    Schema,
    Contracts,
    Schemas,
    Literal,
}

struct Projection<'a> {
    document: &'a Value,
    nodes_left: usize,
    references: Vec<String>,
}

impl Projection<'_> {
    fn value(&mut self, value: &Value, context: Context, depth: usize) -> Result<Value, String> {
        if depth > 128 || self.nodes_left == 0 {
            return Err("contract reference expansion exceeds the safe comparison limit".into());
        }
        self.nodes_left -= 1;
        if !matches!(context, Context::Literal) {
            if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
                let mut aliases = BTreeSet::new();
                let mut resolved = value;
                let mut target = reference;
                while let Some(alias) = resolved.get("$ref").and_then(Value::as_str) {
                    if !aliases.insert(alias) || aliases.len() > 64 {
                        return Err("contract contains a cyclic reference alias".into());
                    }
                    target = alias;
                    resolved =
                        alias.strip_prefix('#').and_then(|p| self.document.pointer(p)).ok_or_else(
                            || "contract contains an unresolved or external reference".to_string(),
                        )?;
                }
                if let Some(index) = self.references.iter().position(|r| r == target) {
                    return Ok(serde_json::json!({"$recursiveRef":self.references.len()-index}));
                }
                self.references.push(target.into());
                let result = self.value(resolved, context, depth + 1);
                self.references.pop();
                return result;
            }
        }
        match value {
            Value::Object(map) => {
                let mut out = Map::new();
                for (key, child) in map {
                    let next = match context {
                        Context::Contracts => Context::Contract,
                        Context::Schemas => Context::Schema,
                        Context::Literal => Context::Literal,
                        Context::Schema => match key.as_str() {
                            "description" | "title" | "example" | "examples" | "$comment" => {
                                continue
                            }
                            "properties" | "patternProperties" | "$defs" | "definitions" => {
                                Context::Schemas
                            }
                            "default" | "enum" | "const" => Context::Literal,
                            _ => Context::Schema,
                        },
                        Context::Contract => match key.as_str() {
                            "description" | "summary" | "externalDocs" | "example" | "examples"
                            | "operationId" | "tags" => continue,
                            "schema" => Context::Schema,
                            "content" | "responses" | "headers" | "encoding" => Context::Contracts,
                            "default" | "security" => Context::Literal,
                            _ => Context::Contract,
                        },
                    };
                    let mut projected = self.value(child, next, depth + 1)?;
                    if matches!(context, Context::Schema)
                        && matches!(key.as_str(), "required" | "enum")
                    {
                        if let Some(items) = projected.as_array_mut() {
                            items.sort_by_key(Value::to_string);
                        }
                    }
                    out.insert(key.clone(), projected);
                }
                Ok(Value::Object(out))
            }
            Value::Array(items) => {
                items.iter().map(|v| self.value(v, context, depth + 1)).collect()
            }
            _ => Ok(value.clone()),
        }
    }
}

fn operations(document: &Value) -> Result<BTreeMap<String, Value>, String> {
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| "definition must contain an OpenAPI paths object".to_string())?;
    let mut projection = Projection {
        document,
        nodes_left: 200_000,
        references: Vec::new(),
    };
    let mut out = BTreeMap::new();
    for (path, item) in paths {
        if !path.starts_with('/') {
            continue;
        }
        let item = projection.value(item, Context::Contract, 0)?;
        for method in METHODS {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let mut contract = operation
                .as_object()
                .cloned()
                .ok_or_else(|| "HTTP operation must be an object".to_string())?;
            let mut parameters = BTreeMap::new();
            for parameter in
                item.get("parameters").and_then(Value::as_array).into_iter().flatten().chain(
                    operation.get("parameters").and_then(Value::as_array).into_iter().flatten(),
                )
            {
                let location = parameter
                    .get("in")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "parameter is missing its location".to_string())?;
                let name = parameter
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "parameter is missing its name".to_string())?;
                parameters.insert(format!("{location}:{name}"), parameter.clone());
            }
            contract.insert(
                "parameters".into(),
                serde_json::to_value(parameters).map_err(|e| e.to_string())?,
            );
            for inherited in ["security", "servers"] {
                if !contract.contains_key(inherited) {
                    if let Some(value) = item.get(inherited).or_else(|| document.get(inherited)) {
                        contract.insert(
                            inherited.into(),
                            projection.value(
                                value,
                                if inherited == "security" {
                                    Context::Literal
                                } else {
                                    Context::Contract
                                },
                                0,
                            )?,
                        );
                    }
                }
            }
            if let Some(requirements) = contract.get_mut("security").and_then(Value::as_array_mut) {
                let mut schemes = Map::new();
                for requirement in requirements.iter_mut() {
                    let entries = requirement
                        .as_object_mut()
                        .ok_or_else(|| "security requirement must be an object".to_string())?;
                    for (name, scopes) in entries {
                        if let Some(scopes) = scopes.as_array_mut() {
                            scopes.sort_by_key(Value::to_string);
                        }
                        let scheme = document
                            .get("components")
                            .and_then(|v| v.get("securitySchemes"))
                            .and_then(|v| v.get(name))
                            .ok_or_else(|| {
                                "security requirement names an undefined scheme".to_string()
                            })?;
                        schemes
                            .insert(name.clone(), projection.value(scheme, Context::Contract, 0)?);
                    }
                }
                requirements.sort_by_key(Value::to_string);
                contract.insert("securitySchemes".into(), Value::Object(schemes));
            }
            out.insert(format!("{} {path}", method.to_uppercase()), Value::Object(contract));
        }
    }
    Ok(out)
}

fn diff(mock: Option<&Value>, source: Option<&Value>, path: &str, out: &mut Vec<FieldDelta>) {
    if mock == source {
        return;
    }
    if let (Some(Value::Object(before)), Some(Value::Object(after))) = (mock, source) {
        let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            diff(before.get(key), after.get(key), &format!("{path}/{escaped}"), out);
        }
    } else {
        out.push(FieldDelta {
            location: path.into(),
            change: if mock.is_none() {
                "added"
            } else if source.is_none() {
                "removed"
            } else {
                "changed"
            }
            .into(),
            mock: mock.cloned(),
            source: source.cloned(),
        });
    }
}

#[cfg(test)]
mod tests;

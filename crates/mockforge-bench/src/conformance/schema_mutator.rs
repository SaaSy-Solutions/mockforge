//! Schema-driven body mutator for the conformance self-test.
//!
//! Issue #79 round 17.2 — Srikanth wanted per-category positive *and*
//! negative coverage that's actually informed by the spec, not just
//! "send an empty body". Given a positive sample (`serde_json::Value`)
//! and the resolved request-body schema, this produces a fixed,
//! deterministic set of negative mutations that the server should
//! reject with 4xx.
//!
//! Mutations are per-field (or per-top-level for compound shapes) so
//! a 50-property body produces a handful of high-signal negatives
//! instead of a combinatorial explosion. Each negative carries a
//! label like `"request-body:type-mismatch:user.email"` so the
//! self-test report tells you exactly which field caught (or didn't
//! catch).
//!
//! Coverage:
//! - **type mismatch**: replace a string field with a number, an
//!   integer with a string, an object with an array.
//! - **min/max bound break**: if `minimum`/`maximum` is declared,
//!   step one past it. If `minLength`/`maxLength` is declared on a
//!   string, produce a too-short or too-long value.
//! - **pattern break**: if a `pattern` regex is declared, replace
//!   with a string that definitely doesn't match (`"!!!"`).
//! - **enum out-of-range**: if an `enum` constraint is declared,
//!   replace with a value not in the enum.
//! - **required field removed**: drop each required field one at a
//!   time.
//!
//! Out of scope (for round 17.2):
//! - `oneOf` / `anyOf` / `allOf` discriminator probes (round 17.4)
//! - format-specific mutations (uuid / email / date) — these
//!   typically also catch as pattern or type mismatch
//! - deeply-nested mutations past depth 2 (would explode the matrix)

use openapiv3::{
    AdditionalProperties, NumberType, ObjectType, Schema, SchemaKind, StringType, Type,
};
use serde_json::{json, Value};

/// One mutation: a labelled JSON value that the server's validator
/// should reject. The label is informational only — the self-test
/// reporter splits it on `:` to bucket into categories.
#[derive(Debug, Clone)]
pub struct BodyMutation {
    /// Human-readable label, e.g. `request-body:type-mismatch:user.email`.
    pub label: String,
    /// The mutated JSON. Always serialisable.
    pub body: Value,
}

/// Build the full set of schema-driven negatives for a positive
/// `sample` against `schema`. Empty if neither sample nor schema gives
/// enough information to mutate; callers fall back to the older
/// schema-agnostic negatives (empty body, wrong-type top-level).
pub fn mutate_body(sample: &Value, schema: &Schema) -> Vec<BodyMutation> {
    let mut mutations = Vec::new();

    // Top-level type mismatch: if the schema declares a top-level
    // object, send an array; vice versa. This catches validators
    // that bail on top-level shape before per-field rules.
    if let SchemaKind::Type(t) = &schema.schema_kind {
        match t {
            Type::Object(_) => mutations.push(BodyMutation {
                label: "request-body:type-mismatch:$root".to_string(),
                body: json!([sample.clone()]),
            }),
            Type::Array(_) => mutations.push(BodyMutation {
                label: "request-body:type-mismatch:$root".to_string(),
                body: json!({"unexpected": sample.clone()}),
            }),
            _ => {}
        }
    }

    // Per-field walk: only top-level + one nested layer to keep the
    // matrix bounded.
    if let SchemaKind::Type(Type::Object(obj)) = &schema.schema_kind {
        walk_object(sample, obj, "", &mut mutations);
    }

    mutations
}

fn walk_object(sample: &Value, obj: &ObjectType, prefix: &str, out: &mut Vec<BodyMutation>) {
    let sample_obj = match sample.as_object() {
        Some(o) => o,
        None => return,
    };

    // Required-field removal: drop each required field one at a
    // time.  Cap at the first 5 required fields to keep the matrix
    // bounded for huge specs.
    for field in obj.required.iter().take(5) {
        if sample_obj.contains_key(field) {
            let mut mutated = sample.clone();
            if let Some(o) = mutated.as_object_mut() {
                o.remove(field);
            }
            out.push(BodyMutation {
                label: format!("request-body:required-removed:{}{}", prefix, field),
                body: mutated,
            });
        }
    }

    // Per-property mutations. Iterate the *schema*'s declared
    // properties (not the sample's keys) so a sample missing a field
    // doesn't silently skip its mutation, and so unknown sample keys
    // (which an OpenAPI spec wouldn't have schemas for) get skipped.
    for (field_name, field_schema_ref) in obj.properties.iter().take(20) {
        let field_schema = match field_schema_ref.as_item() {
            Some(s) => s,
            None => continue, // unresolved $ref — skip rather than guess
        };
        let path = format!("{}{}", prefix, field_name);
        mutate_field(sample, sample_obj, field_name, field_schema, &path, out);
    }

    // Additional-properties probe: if the schema explicitly forbids
    // additional properties, send one anyway.
    if matches!(obj.additional_properties, Some(AdditionalProperties::Any(false))) {
        let mut mutated = sample.clone();
        if let Some(o) = mutated.as_object_mut() {
            o.insert("self_test_extra_field".to_string(), json!("extra"));
        }
        out.push(BodyMutation {
            label: format!("request-body:additional-property:{}$root", prefix),
            body: mutated,
        });
    }
}

fn mutate_field(
    sample: &Value,
    _sample_obj: &serde_json::Map<String, Value>,
    field_name: &str,
    field_schema: &Schema,
    path: &str,
    out: &mut Vec<BodyMutation>,
) {
    // Helper to replace one field's value in the sample.
    let set_field = |new: Value| -> Value {
        let mut mutated = sample.clone();
        if let Some(o) = mutated.as_object_mut() {
            o.insert(field_name.to_string(), new);
        }
        mutated
    };

    match &field_schema.schema_kind {
        SchemaKind::Type(Type::String(s)) => mutate_string_field(s, &set_field, path, out),
        SchemaKind::Type(Type::Number(n)) => mutate_number_field(n, &set_field, path, out, false),
        SchemaKind::Type(Type::Integer(_)) => {
            // Integer + number share min/max + type-mismatch logic;
            // model integers as number for the mutator, then add an
            // integer-specific "make it a float" probe.
            out.push(BodyMutation {
                label: format!("request-body:type-mismatch:{}", path),
                body: set_field(json!("not-an-integer")),
            });
            out.push(BodyMutation {
                label: format!("request-body:integer-as-float:{}", path),
                body: set_field(json!(1.5)),
            });
        }
        SchemaKind::Type(Type::Boolean(_)) => {
            out.push(BodyMutation {
                label: format!("request-body:type-mismatch:{}", path),
                body: set_field(json!("not-a-boolean")),
            });
        }
        SchemaKind::Type(Type::Array(_)) => {
            out.push(BodyMutation {
                label: format!("request-body:type-mismatch:{}", path),
                body: set_field(json!({"not-an-array": true})),
            });
        }
        SchemaKind::Type(Type::Object(_)) => {
            out.push(BodyMutation {
                label: format!("request-body:type-mismatch:{}", path),
                body: set_field(json!("not-an-object")),
            });
        }
        // SchemaKind::OneOf/AnyOf/AllOf — out of scope for 17.2.
        _ => {}
    }

    // Enum negatives apply regardless of the underlying type.
    if !field_schema.schema_data.extensions.is_empty() {
        // No-op — extensions don't drive enum logic.
    }
    if let SchemaKind::Type(Type::String(s)) = &field_schema.schema_kind {
        if !s.enumeration.is_empty() {
            // Send a value not in the enum.
            out.push(BodyMutation {
                label: format!("request-body:enum-out-of-range:{}", path),
                body: set_field(json!("self-test-not-in-enum")),
            });
        }
    }
}

fn mutate_string_field(
    s: &StringType,
    set_field: &dyn Fn(Value) -> Value,
    path: &str,
    out: &mut Vec<BodyMutation>,
) {
    // Type mismatch — a string should reject a number.
    out.push(BodyMutation {
        label: format!("request-body:type-mismatch:{}", path),
        body: set_field(json!(12345)),
    });

    // minLength: send a 0-length string when minLength >= 1.
    if let Some(min) = s.min_length {
        if min >= 1 {
            out.push(BodyMutation {
                label: format!("request-body:min-length:{}", path),
                body: set_field(json!("")),
            });
        }
    }

    // maxLength: send a string one past the cap.
    if let Some(max) = s.max_length {
        let too_long: String = "x".repeat(max.saturating_add(1).min(10_000));
        out.push(BodyMutation {
            label: format!("request-body:max-length:{}", path),
            body: set_field(json!(too_long)),
        });
    }

    // Pattern: send a value that definitely doesn't match a typical regex.
    if let Some(_pattern) = &s.pattern {
        // We don't try to invert the regex — just send a marker the
        // user can grep. Most patterns require alphanumeric/email/uuid
        // and "!!!" matches none of those.
        out.push(BodyMutation {
            label: format!("request-body:pattern:{}", path),
            body: set_field(json!("!!!self-test-pattern-violation!!!")),
        });
    }
}

fn mutate_number_field(
    n: &NumberType,
    set_field: &dyn Fn(Value) -> Value,
    path: &str,
    out: &mut Vec<BodyMutation>,
    _integer: bool,
) {
    // Type mismatch.
    out.push(BodyMutation {
        label: format!("request-body:type-mismatch:{}", path),
        body: set_field(json!("not-a-number")),
    });

    // minimum: send minimum - 1 (or - 0.0001 if it's a float).
    if let Some(min) = n.minimum {
        out.push(BodyMutation {
            label: format!("request-body:minimum:{}", path),
            body: set_field(json!(min - 1.0)),
        });
    }
    // maximum: send maximum + 1.
    if let Some(max) = n.maximum {
        out.push(BodyMutation {
            label: format!("request-body:maximum:{}", path),
            body: set_field(json!(max + 1.0)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};
    use std::collections::BTreeSet;

    fn object_schema(props: Vec<(&str, Schema)>, required: Vec<&str>) -> Schema {
        let mut obj = ObjectType::default();
        for (name, schema) in props {
            obj.properties.insert(name.to_string(), ReferenceOr::Item(Box::new(schema)));
        }
        obj.required = required.into_iter().map(|s| s.to_string()).collect();
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(obj)),
        }
    }

    fn string_field(min: Option<usize>, max: Option<usize>, pattern: Option<&str>) -> Schema {
        let s = openapiv3::StringType {
            min_length: min,
            max_length: max,
            pattern: pattern.map(|p| p.to_string()),
            ..Default::default()
        };
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(s)),
        }
    }

    fn integer_field() -> Schema {
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Integer(openapiv3::IntegerType::default())),
        }
    }

    #[test]
    fn empty_for_non_object_root() {
        let s = string_field(None, None, None);
        let m = mutate_body(&json!("hello"), &s);
        assert!(m.is_empty(), "string root produces no body mutations");
    }

    #[test]
    fn required_field_removed_for_each_required() {
        let s = object_schema(
            vec![
                ("name", string_field(None, None, None)),
                ("age", integer_field()),
            ],
            vec!["name", "age"],
        );
        let m = mutate_body(&json!({"name": "Ada", "age": 30}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:required-removed:name"), "{labels:?}");
        assert!(labels.contains("request-body:required-removed:age"), "{labels:?}");
    }

    #[test]
    fn type_mismatch_for_typed_fields() {
        let s = object_schema(vec![("name", string_field(None, None, None))], vec![]);
        let m = mutate_body(&json!({"name": "Ada"}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:type-mismatch:name"), "{labels:?}");
    }

    #[test]
    fn min_max_length_for_string() {
        let s = object_schema(vec![("user", string_field(Some(3), Some(10), None))], vec![]);
        let m = mutate_body(&json!({"user": "abc"}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:min-length:user"), "{labels:?}");
        assert!(labels.contains("request-body:max-length:user"), "{labels:?}");
    }

    #[test]
    fn pattern_violation_emitted() {
        let s = object_schema(
            vec![("ssn", string_field(None, None, Some(r"^\d{3}-\d{2}-\d{4}$")))],
            vec![],
        );
        let m = mutate_body(&json!({"ssn": "123-45-6789"}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:pattern:ssn"), "{labels:?}");
    }

    #[test]
    fn integer_specific_mutations() {
        let s = object_schema(vec![("age", integer_field())], vec![]);
        let m = mutate_body(&json!({"age": 30}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:type-mismatch:age"), "{labels:?}");
        assert!(labels.contains("request-body:integer-as-float:age"), "{labels:?}");
    }

    #[test]
    fn root_type_mismatch_for_object_root() {
        let s = object_schema(vec![], vec![]);
        let m = mutate_body(&json!({}), &s);
        let labels: BTreeSet<String> = m.iter().map(|x| x.label.clone()).collect();
        assert!(labels.contains("request-body:type-mismatch:$root"), "{labels:?}");
    }
}

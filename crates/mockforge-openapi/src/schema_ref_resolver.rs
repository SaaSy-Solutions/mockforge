//! Build a JSON Schema validator that can resolve nested `$ref`
//! pointers into the OpenAPI document's `#/components/schemas/...`
//! map.
//!
//! Issue #79 round 18.3 — Srikanth's vCenter run on 0.3.152
//! produced 157 violations like
//!   "Failed to create schema validator: Pointer
//!    '/components/schemas/Vcenter.VM.DiskCloneSpec' does not exist"
//!
//! Root cause: `validate_request_body` called
//! `jsonschema::options().build(&inner_schema_json)` with **only the
//! inner schema** as the validator's document. When the schema's
//! properties contained nested `"$ref": "#/components/schemas/X"`
//! strings, the validator tried to resolve them against the inner
//! schema, which has no `components` section.
//!
//! Fix: wrap the inner schema so it carries the spec's components
//! map at the document root, giving `$ref` pointers a place to
//! resolve to. JSON Schema validators ignore unknown root keys, so
//! the synthetic `components` field doesn't affect validation
//! semantics — it's only there to be a resolution target.

use jsonschema::{Draft, Validator};
use openapiv3::{OpenAPI, Schema};
use serde_json::Value;

/// Build a `jsonschema::Validator` for `schema` that can resolve
/// `$ref` pointers against the full `spec`. Returns a `String`
/// error so callers don't have to thread `jsonschema::ValidationError`
/// lifetimes through their result types.
pub fn build_validator(schema: &Schema, spec: &OpenAPI) -> Result<Validator, String> {
    let schema_json = serde_json::to_value(schema)
        .map_err(|e| format!("Failed to convert OpenAPI schema to JSON: {e}"))?;
    let merged = merge_components_into(schema_json, spec);
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .build(&merged)
        .map_err(|e| format!("Failed to create schema validator: {e}"))
}

/// Merge the spec's components into a root-level `components` key on
/// the schema document. If the schema already declares a
/// `components` key (rare but legal), it takes precedence — we don't
/// clobber explicit data. Returns the merged document.
pub fn merge_components_into(mut schema_json: Value, spec: &OpenAPI) -> Value {
    let Some(components) = &spec.components else {
        return schema_json;
    };
    let Value::Object(ref mut map) = schema_json else {
        // Non-object root (rare — a schema is usually an object).
        // Wrap it in an object so we can attach components.
        let inner = schema_json;
        let wrapper = serde_json::json!({
            "allOf": [inner],
        });
        return merge_components_into(wrapper, spec);
    };
    if map.contains_key("components") {
        return schema_json;
    }
    if let Ok(comp_json) = serde_json::to_value(components) {
        map.insert("components".to_string(), comp_json);
    }
    schema_json
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{Components, ReferenceOr, SchemaData, SchemaKind, StringType, Type};

    fn make_spec_with_named_schema(name: &str, schema: Schema) -> OpenAPI {
        let mut components = Components::default();
        components.schemas.insert(name.to_string(), ReferenceOr::Item(schema));
        OpenAPI {
            openapi: "3.0.0".into(),
            info: Default::default(),
            components: Some(components),
            ..Default::default()
        }
    }

    /// The canonical bug reproducer: a request-body schema with a
    /// nested `$ref` to a components/schemas/X with dots in the
    /// name. Pre-fix this failed with "Pointer does not exist".
    /// Post-fix it should build a validator and validate a body
    /// against it cleanly.
    #[test]
    fn dotted_schema_ref_resolves_against_spec_context() {
        // Define `Foo.Bar.Baz` in components — string type.
        let nested = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType::default())),
        };
        let spec = make_spec_with_named_schema("Foo.Bar.Baz", nested);

        // Request body schema: object with one property that
        // $refs into the components.
        let body_schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"$ref": "#/components/schemas/Foo.Bar.Baz"}
            },
            "required": ["name"]
        });
        let body_schema: Schema = serde_json::from_value(body_schema_json).unwrap();

        let validator =
            build_validator(&body_schema, &spec).expect("validator builds against spec context");

        // A valid body — `name` is a string.
        let good = serde_json::json!({"name": "hello"});
        assert!(validator.iter_errors(&good).next().is_none());

        // An invalid body — `name` is a number, should be rejected.
        let bad = serde_json::json!({"name": 42});
        assert!(validator.iter_errors(&bad).next().is_some());
    }

    /// Pre-fix path: a validator built from just the inner schema
    /// (without our wrapper) fails AT BUILD TIME to resolve dotted
    /// $refs — jsonschema is eager-resolve. The error wording is
    /// literally the one Srikanth saw:
    /// `Pointer '/components/schemas/Foo.Bar.Baz' does not exist`.
    /// Documenting the wrong behaviour so future-me knows what
    /// `build_validator` is protecting against.
    #[test]
    fn naked_validator_fails_to_build_on_dotted_ref() {
        let body_schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"$ref": "#/components/schemas/Foo.Bar.Baz"}
            }
        });
        let result = jsonschema::options().with_draft(Draft::Draft7).build(&body_schema_json);
        let err = result.expect_err("naked validator should fail to build");
        let msg = err.to_string();
        assert!(
            msg.contains("Foo.Bar.Baz") || msg.contains("/components/schemas/"),
            "naked-validator error should reference the unresolvable pointer; got: {msg}"
        );
    }

    /// When the schema already declares a `components` key (rare —
    /// some specs do this inline), we don't clobber it.
    #[test]
    fn explicit_components_takes_precedence() {
        let spec = make_spec_with_named_schema(
            "X",
            Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType::default())),
            },
        );
        let schema = serde_json::json!({
            "type": "object",
            "components": {"schemas": {"X": {"type": "integer"}}}
        });
        let merged = merge_components_into(schema.clone(), &spec);
        // The schema's explicit `components.schemas.X` (integer)
        // wins; the spec's (string) does not overwrite.
        let x = merged.get("components").and_then(|c| c.get("schemas")).and_then(|s| s.get("X"));
        assert_eq!(
            x.and_then(|v| v.get("type")).and_then(|v| v.as_str()),
            Some("integer"),
            "explicit schema components should not be clobbered"
        );
    }

    /// Specs with no `components` block at all (some Swagger
    /// conversions, or very minimal specs) should still produce a
    /// working validator — we just don't add a components key.
    #[test]
    fn spec_without_components_is_a_noop() {
        let spec = OpenAPI {
            openapi: "3.0.0".into(),
            info: Default::default(),
            ..Default::default()
        };
        let schema = serde_json::json!({"type": "string"});
        let merged = merge_components_into(schema.clone(), &spec);
        assert_eq!(merged, schema);
    }
}

//! Resolve parameter schemas against their source document without fetching resources.

use super::{coerce_value_for_schema, Map, OpenApiSpec, Value};
use jsonschema::{Draft, Retrieve, Uri};
use openapiv3::{ParameterData, ParameterSchemaOrContent, ReferenceOr, Schema, SchemaKind, Type};
use std::collections::HashSet;

struct LocalOnly;
impl Retrieve for LocalOnly {
    fn retrieve(&self, _: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("Parameter schema references must resolve within the supplied specification".into())
    }
}

pub(super) fn schema<'a>(
    data: &'a ParameterData,
    spec: &'a OpenApiSpec,
) -> Result<Option<&'a Schema>, String> {
    match &data.format {
        ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) => Ok(Some(schema)),
        ParameterSchemaOrContent::Schema(ReferenceOr::Reference { reference }) => {
            resolve(reference, spec).map(Some)
        }
        ParameterSchemaOrContent::Content(_) => Ok(None),
    }
}

fn resolve<'a>(mut reference: &'a str, spec: &'a OpenApiSpec) -> Result<&'a Schema, String> {
    let mut seen = HashSet::new();
    for _ in 0..64 {
        if !seen.insert(reference) {
            return Err("Cyclic parameter schema reference".into());
        }
        let name = reference
            .strip_prefix("#/components/schemas/")
            .ok_or("Parameter schema reference is not a local component schema")?;
        let name = name.replace("~1", "/").replace("~0", "~");
        match spec.spec.components.as_ref().and_then(|c| c.schemas.get(&name)) {
            Some(ReferenceOr::Item(schema)) => return Ok(schema),
            Some(ReferenceOr::Reference { reference: next }) => reference = next,
            None => return Err("Parameter schema reference cannot be resolved locally".into()),
        }
    }
    Err("Parameter schema reference exceeds 64 aliases".into())
}

fn boxed<'a>(
    schema: &'a ReferenceOr<Box<Schema>>,
    spec: &'a OpenApiSpec,
) -> Result<&'a Schema, String> {
    match schema {
        ReferenceOr::Item(schema) => Ok(schema),
        ReferenceOr::Reference { reference } => resolve(reference, spec),
    }
}

fn inline<'a>(
    schema: &'a ReferenceOr<Schema>,
    spec: &'a OpenApiSpec,
) -> Result<&'a Schema, String> {
    match schema {
        ReferenceOr::Item(schema) => Ok(schema),
        ReferenceOr::Reference { reference } => resolve(reference, spec),
    }
}

fn array_value(
    value: &Value,
    array: &openapiv3::ArrayType,
    spec: &OpenApiSpec,
    style: Option<&str>,
    depth: usize,
) -> Result<Value, String> {
    let values = match value {
        Value::Array(values) => values.clone(),
        Value::String(text) => {
            let delimiter = match style {
                Some("spaceDelimited") => ' ',
                Some("pipeDelimited") => '|',
                _ => ',',
            };
            text.split(delimiter).map(|s| Value::String(s.trim().into())).collect()
        }
        _ => return Ok(value.clone()),
    };
    let Some(items) = &array.items else {
        return Ok(Value::Array(values));
    };
    let items = boxed(items, spec)?;
    values
        .iter()
        .map(|value| coerce(value, items, spec, style, depth + 1))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn object_value(
    value: &Value,
    object: &openapiv3::ObjectType,
    spec: &OpenApiSpec,
    style: Option<&str>,
    depth: usize,
) -> Result<Value, String> {
    let Value::Object(values) = value else {
        return Ok(value.clone());
    };
    let mut out = Map::new();
    for (key, value) in values {
        let schema = match object.properties.get(key) {
            Some(schema) => Some(boxed(schema, spec)?),
            None => match &object.additional_properties {
                Some(openapiv3::AdditionalProperties::Schema(schema)) => {
                    Some(inline(schema, spec)?)
                }
                _ => None,
            },
        };
        let value = match schema {
            Some(schema) => coerce(value, schema, spec, style, depth + 1)?,
            None => value.clone(),
        };
        out.insert(key.clone(), value);
    }
    Ok(Value::Object(out))
}

fn has_references(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(fields) => {
                if fields.get("$ref").is_some_and(Value::is_string) {
                    return true;
                }
                pending.extend(fields.values());
            }
            Value::Array(values) => pending.extend(values),
            _ => {}
        }
    }
    false
}

fn build(schema: &Schema, spec: &OpenApiSpec) -> Result<jsonschema::Validator, String> {
    let json = serde_json::to_value(schema).map_err(|e| e.to_string())?;
    // Primitive and inline-only schemas need no component namespace. Avoid
    // serializing every unused component for each parameter in each request.
    let merged = if spec.spec.components.is_some() && has_references(&json) {
        crate::schema_ref_resolver::merge_components_into(json, &spec.spec)
    } else {
        json
    };
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .with_retriever(LocalOnly)
        .build(&merged)
        .map_err(|e| format!("Failed to create schema validator: {e}"))
}

fn alternative(
    value: &Value,
    branches: &[ReferenceOr<Schema>],
    spec: &OpenApiSpec,
    style: Option<&str>,
    depth: usize,
) -> Result<Value, String> {
    for branch in branches {
        let branch = inline(branch, spec)?;
        let candidate = coerce(value, branch, spec, style, depth + 1)?;
        if build(branch, spec)?.is_valid(&candidate) {
            return Ok(candidate);
        }
    }
    Ok(value.clone())
}

fn coerce(
    value: &Value,
    schema: &Schema,
    spec: &OpenApiSpec,
    style: Option<&str>,
    depth: usize,
) -> Result<Value, String> {
    if depth > 64 {
        return Err("Parameter value exceeds 64 nested levels".into());
    }
    match &schema.schema_kind {
        SchemaKind::Type(Type::Array(array)) => array_value(value, array, spec, style, depth),
        SchemaKind::Type(Type::Object(object)) => object_value(value, object, spec, style, depth),
        SchemaKind::AllOf { all_of } => all_of.iter().try_fold(value.clone(), |value, branch| {
            coerce(&value, inline(branch, spec)?, spec, style, depth + 1)
        }),
        SchemaKind::AnyOf { any_of } => alternative(value, any_of, spec, style, depth),
        SchemaKind::OneOf { one_of } => alternative(value, one_of, spec, style, depth),
        _ => Ok(coerce_value_for_schema(value, schema)),
    }
}

pub(super) fn validate(
    value: &Value,
    schema: &Schema,
    spec: &OpenApiSpec,
    style: Option<&str>,
) -> Result<Value, (Value, String)> {
    let coerced = coerce(value, schema, spec, style, 0)
        .map_err(|message| (value.clone(), format!("Validation error: {message}")))?;
    let validator = build(schema, spec)
        .map_err(|message| (coerced.clone(), format!("Validation error: {message}")))?;
    let errors: Vec<_> = validator.iter_errors(&coerced).map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(coerced)
    } else {
        Err((coerced, format!("Validation error: {}", errors.join("; "))))
    }
}

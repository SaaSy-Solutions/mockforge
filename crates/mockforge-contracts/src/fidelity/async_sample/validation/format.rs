//! Interpret message schema-format declarations and wrappers.
use super::Outcome;
use serde_json::Value;

fn supported_format(format: &str) -> bool {
    let format = format.to_ascii_lowercase().replace(' ', "");
    format.starts_with("application/vnd.aai.asyncapi;version=2.")
        || format.starts_with("application/vnd.aai.asyncapi;version=3.")
        || format == "application/schema+json;version=draft-07"
        || format == "application/schema+json;version=draft-07.0"
}

pub(super) fn supported_schema<'a>(
    raw: Option<&'a Value>,
    default_format: Option<&str>,
    label: &str,
    outcome: &mut Outcome,
) -> Result<Option<&'a Value>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let (schema, format) = if let Some(format) = raw.get("schemaFormat") {
        (
            raw.get("schema").ok_or("Multi-format schema must declare its schema")?,
            Some(format.as_str().ok_or("schemaFormat must be a string")?),
        )
    } else {
        (raw, default_format)
    };
    if format.is_some_and(|format| !supported_format(format)) {
        outcome
            .notes
            .push(format!("{label} schema format is not supported by the JSON-schema validator"));
        Ok(None)
    } else {
        Ok(Some(schema))
    }
}

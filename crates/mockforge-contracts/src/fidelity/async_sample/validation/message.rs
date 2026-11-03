//! Apply payload and captured-header schemas to one message.
use super::format::supported_schema;
use super::{AsyncMessage, Checker, Outcome};
use serde_json::Value;

pub(super) fn check(
    checker: &mut Checker<'_>,
    message: &Value,
    record: &AsyncMessage,
) -> Result<Outcome, String> {
    let mut out = Outcome::default();
    let payload_declaration = message.get("payload");
    let schema = supported_schema(
        payload_declaration,
        message.get("schemaFormat").and_then(Value::as_str),
        "Payload",
        &mut out,
    )?;
    if let Some(headers) = supported_schema(message.get("headers"), None, "Headers", &mut out)? {
        if let Some(actual) = &record.headers {
            checker.check(headers, actual, "Message headers", &mut out)?;
        } else {
            out.notes.push("Message headers were not captured".into());
        }
    }
    let Some(payload) = &record.payload else {
        out.notes.push("Message payload was not captured".into());
        return Ok(out);
    };
    let value = super::media::decode(message, record, payload, &mut out);
    if let (Some(schema), Some(value)) = (schema, value) {
        checker.check(schema, &value, "Message payload", &mut out)?;
    }
    if payload_declaration.is_none() {
        out.notes.push("Message declares no payload schema to check".into());
    }
    Ok(out)
}

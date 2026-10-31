//! Decode supported wire media without coercing JSON types.
use super::{AsyncMessage, Outcome};
use serde_json::Value;

pub(super) fn decode(
    message: &Value,
    record: &AsyncMessage,
    payload: &[u8],
    out: &mut Outcome,
) -> Option<Value> {
    let media = message.get("contentType").and_then(Value::as_str).or_else(|| {
        record
            .headers
            .as_ref()
            .and_then(|h| h.as_object())
            .and_then(|h| h.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-type")))
            .and_then(|(_, v)| v.as_str())
    });
    let value = match media.map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase()) {
        Some(media)
            if media == "application/json"
                || media.starts_with("application/") && media.ends_with("+json") =>
        {
            match serde_json::from_slice(payload) {
                Ok(value) => Some(value),
                Err(_) => {
                    out.findings.push("Message payload is not valid JSON".into());
                    None
                }
            }
        }
        Some(media) if media == "text/plain" => match std::str::from_utf8(payload) {
            Ok(text) => Some(Value::String(text.into())),
            Err(_) => {
                out.findings.push("Text message payload is not UTF-8".into());
                None
            }
        },
        _ => {
            out.notes.push(
                "Message content type is unspecified or not supported by the sample validator"
                    .into(),
            );
            None
        }
    };
    value
}

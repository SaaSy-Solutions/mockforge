//! Validate the selected message alternatives with exact JSON types and no I/O.
use super::{schema, AsyncMessage};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Outcome {
    pub findings: Vec<String>,
    pub notes: Vec<String>,
}

pub(super) struct Checker<'a> {
    source: &'a Value,
    validators: BTreeMap<usize, schema::Prepared>,
}

impl<'a> Checker<'a> {
    pub(super) fn new(source: &'a Value) -> Self {
        Self {
            source,
            validators: BTreeMap::new(),
        }
    }

    pub(super) fn alternatives(
        &mut self,
        messages: &[Value],
        record: &AsyncMessage,
    ) -> Result<Outcome, String> {
        if messages.is_empty() {
            return Ok(Outcome {
                notes: vec!["No message definition is available for this operation".into()],
                ..Outcome::default()
            });
        }
        let mut valid = 0;
        let mut unknown = Vec::new();
        let mut failures = Vec::new();
        for message in messages {
            let outcome = self.message(message, record)?;
            if !outcome.findings.is_empty() {
                failures.extend(outcome.findings);
            } else if !outcome.notes.is_empty() {
                unknown.extend(outcome.notes);
            } else {
                valid += 1;
            }
        }
        if valid > 1 {
            Ok(Outcome {
                findings: vec!["Recorded message matches more than one message alternative".into()],
                ..Outcome::default()
            })
        } else if !unknown.is_empty() {
            unknown.sort();
            unknown.dedup();
            Ok(Outcome {
                notes: unknown,
                ..Outcome::default()
            })
        } else if valid == 1 {
            Ok(Outcome::default())
        } else {
            failures.truncate(8);
            Ok(Outcome {
                findings: vec![format!(
                    "Recorded message matches no message alternative: {}",
                    failures.join("; ")
                )],
                ..Outcome::default()
            })
        }
    }

    fn message(&mut self, message: &Value, record: &AsyncMessage) -> Result<Outcome, String> {
        let mut out = Outcome::default();
        let payload_declaration = message.get("payload");
        let schema = supported_schema(
            payload_declaration,
            message.get("schemaFormat").and_then(Value::as_str),
            "Payload",
            &mut out,
        )?;
        if let Some(headers) = supported_schema(message.get("headers"), None, "Headers", &mut out)?
        {
            if let Some(actual) = &record.headers {
                self.check(headers, actual, "Message headers", &mut out)?;
            } else {
                out.notes.push("Message headers were not captured".into());
            }
        }
        let Some(payload) = &record.payload else {
            out.notes.push("Message payload was not captured".into());
            return Ok(out);
        };
        let media = message.get("contentType").and_then(Value::as_str).or_else(|| {
            record
                .headers
                .as_ref()
                .and_then(|h| h.as_object())
                .and_then(|h| h.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-type")))
                .and_then(|(_, v)| v.as_str())
        });
        let value =
            match media.map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase()) {
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
        if let (Some(schema), Some(value)) = (schema, value) {
            self.check(schema, &value, "Message payload", &mut out)?;
        }
        if payload_declaration.is_none() {
            out.notes.push("Message declares no payload schema to check".into());
        }
        Ok(out)
    }

    fn check(
        &mut self,
        schema: &Value,
        value: &Value,
        label: &str,
        out: &mut Outcome,
    ) -> Result<(), String> {
        // Message schemas live in the immutable catalog for this verification.
        // Reuse validators by identity without serializing large schemas per record.
        let key = schema as *const Value as usize;
        if !self.validators.contains_key(&key) {
            self.validators.insert(key, schema::compile(schema, self.source)?);
        }
        match &self.validators[&key] {
            schema::Prepared::Unsupported(reasons) => {
                out.notes.extend(reasons.iter().map(|reason| format!("{label}: {reason}")));
            }
            schema::Prepared::Compiled(validator) => {
                if let Some(error) = validator.iter_errors(value).next() {
                    let detail: String = error.to_string().chars().take(1024).collect();
                    out.findings.push(format!("{label}: {detail}"));
                }
            }
        }
        Ok(())
    }
}

fn supported_format(format: &str) -> bool {
    let format = format.to_ascii_lowercase().replace(' ', "");
    format.starts_with("application/vnd.aai.asyncapi;version=2.")
        || format.starts_with("application/vnd.aai.asyncapi;version=3.")
        || format == "application/schema+json;version=draft-07"
        || format == "application/schema+json;version=draft-07.0"
}

fn supported_schema<'a>(
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

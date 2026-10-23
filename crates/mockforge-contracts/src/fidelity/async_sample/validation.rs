//! Validate the selected message alternatives with exact JSON types and no I/O.
use super::{schema, AsyncMessage};
use serde_json::Value;
use std::collections::BTreeMap;

mod format;
mod media;
mod message;

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
            let outcome = message::check(self, message, record)?;
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

//! One exchange with the mock: a request it received, and the answer it gave.
//!
//! `serve --exchange-log FILE` writes one line of JSON per exchange, in the
//! shape [`Exchange`] holds and [`read_exchanges`] reads. It is what a range
//! judge checks an agent's findings against: a flaw counts only when its
//! proving request is in the log, with the answer that shows it.
//!
//! A header that carries a credential is written as a digest of its value,
//! never as sent. So is a value that is a key of a known shape
//! ([`crate::keys`]), whatever the header is called, and a whole JSON string
//! of a body that is one. A log is evidence, and evidence is passed around.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::{bad_exchange, Result};
use crate::keys;

/// The headers whose value is a credential, in lower case.
pub const CREDENTIAL_HEADERS: [&str; 4] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
];

/// How much of a body the log holds. What a flaw needs to show is at the
/// start of an answer, and a log that holds every byte of every answer is a
/// copy of the API.
pub const BODY_LIMIT: usize = 16 * 1024;

/// A request the mock received, and the answer it gave.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Exchange {
    /// Counted from 1, in the order the mock received the requests.
    pub seq: u64,
    /// When the request arrived, as RFC 3339 in UTC.
    pub at: String,
    /// Upper case: `GET`, `POST`.
    pub method: String,
    /// The path, without the query.
    pub path: String,
    /// The query as it was sent, without the leading `?`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub query: String,
    /// By name, in lower case. A credential is a digest of its value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub request_headers: BTreeMap<String, String>,
    /// The start of the request body, as text. A body that is not text is
    /// written with what text it holds.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub request_body: String,
    /// Whether the request body was cut at [`BODY_LIMIT`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub request_body_truncated: bool,
    /// The status code the mock answered.
    pub status: u16,
    /// By name, in lower case. A credential is a digest of its value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub response_headers: BTreeMap<String, String>,
    /// The start of the answer's body, as text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response_body: String,
    /// Whether the answer's body was cut at [`BODY_LIMIT`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub response_body_truncated: bool,
    /// How long the mock took to answer.
    pub duration_ms: u64,
}

/// Whether a header carries a credential. Read without regard to case.
pub fn is_credential(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    CREDENTIAL_HEADERS.contains(&lower.as_str())
}

/// A header's value as the log holds it: a digest for a credential, and for
/// a value that is a key of a known shape whatever the header is called; and
/// itself for anything else.
pub fn header_value(name: &str, value: &str) -> String {
    if is_credential(name) || keys::is_key(value) {
        digest(value)
    } else {
        value.to_string()
    }
}

/// `sha256:` and the hex digest of a text. A judge compares a credential a
/// pack names with one the log holds by digesting the first.
pub fn digest(text: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

/// The start of a body as the log holds it: its first [`BODY_LIMIT`] bytes as
/// text, with every whole JSON string that is a key of a known shape written
/// as its digest, and whether that is less than the whole.
pub fn held(bytes: &[u8]) -> (String, bool) {
    let cut = &bytes[..bytes.len().min(BODY_LIMIT)];
    (without_keys(&String::from_utf8_lossy(cut)), bytes.len() > BODY_LIMIT)
}

/// A text with every whole JSON string that is a key of a known shape written
/// as its digest. The rest of the text is as it was, and a key that is only
/// part of a longer string is left there.
pub fn without_keys(text: &str) -> String {
    keys::replace_json_strings(text, |held| keys::is_key(held).then(|| digest(held)))
}

/// Read an exchange log: one exchange per line. A line that is not an
/// exchange is an error that names it, never skipped: a log with a line
/// missing has exchanges missing, and a judge must not pass one.
pub fn read_exchanges(text: &str, path: &str) -> Result<Vec<Exchange>> {
    let mut exchanges = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let exchange: Exchange = serde_json::from_str(line).map_err(|error| {
            bad_exchange(path, index + 1, &format!("not an exchange ({error})"))
        })?;
        exchanges.push(exchange);
    }
    Ok(exchanges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_credential_is_a_digest_and_nothing_else_is() {
        for (name, credential) in [
            ("authorization", true),
            ("Authorization", true),
            ("PROXY-AUTHORIZATION", true),
            ("cookie", true),
            ("set-cookie", true),
            ("x-api-key", false),
            ("accept", false),
        ] {
            assert_eq!(is_credential(name), credential, "{name}");
        }
        let digest_of = digest("Bearer demo-user-a");
        assert_eq!(digest_of.len(), "sha256:".len() + 64);
        assert_eq!(digest("x"), digest("x"));
        assert_ne!(digest("x"), digest("y"));
        // A credential is not in its digest. Another header is itself.
        assert_eq!(header_value("authorization", "Bearer demo-user-a"), digest_of);
        assert!(!digest_of.contains("demo"));
        assert_eq!(header_value("accept", "application/json"), "application/json");
    }

    #[test]
    fn a_log_is_read_one_exchange_per_line() {
        let line = serde_json::json!({
            "seq": 1, "at": "2026-11-26T00:00:00.000000Z", "method": "GET", "path": "/orders/5000",
            "query": "limit=5", "request_headers": { "authorization": digest("Bearer demo-user-a") },
            "request_body": "", "status": 200, "response_headers": { "content-type": "application/json" },
            "response_body": "[{\"id\":5000}]", "duration_ms": 2
        });
        let exchanges =
            read_exchanges(&format!("{line}\n\n\n{line}\n"), "exchanges.jsonl").unwrap();
        assert_eq!(exchanges.len(), 2);
        let first = &exchanges[0];
        assert_eq!(
            (
                first.seq,
                first.path.as_str(),
                first.query.as_str(),
                first.status,
                first.duration_ms
            ),
            (1, "/orders/5000", "limit=5", 200, 2)
        );
        assert_eq!(first.request_headers["authorization"], digest("Bearer demo-user-a"));
        assert_eq!(first.response_body, "[{\"id\":5000}]");
        assert!(!first.request_body_truncated && !first.response_body_truncated);
        // A line written before the later fields existed reads without them.
        let old = serde_json::json!({ "seq": 2, "at": "t", "method": "GET", "path": "/x", "status": 404, "duration_ms": 0 });
        let exchange = &read_exchanges(&old.to_string(), "e").unwrap()[0];
        assert_eq!(
            (
                exchange.query.as_str(),
                exchange.request_headers.len(),
                exchange.response_body.as_str()
            ),
            ("", 0, "")
        );
    }

    #[test]
    fn a_line_that_is_not_an_exchange_is_an_error_naming_it() {
        // A line that is not JSON, and one that is JSON but not an exchange.
        // The first stops the reading: what follows may be the rest of it.
        let broken = read_exchanges("not json\n{}\n", "exchanges.jsonl").unwrap_err().to_string();
        assert!(
            broken.starts_with("exchanges.jsonl: line 1: not an exchange (expected ident"),
            "{broken}"
        );
        let half = read_exchanges("{}\n", "exchanges.jsonl").unwrap_err().to_string();
        assert!(
            half.starts_with("exchanges.jsonl: line 1: not an exchange (missing field `"),
            "{half}"
        );
        // The line that is not an exchange is named, not the first.
        let later = read_exchanges("{}\n{}\n", "exchanges.jsonl").unwrap_err().to_string();
        assert!(later.starts_with("exchanges.jsonl: line 1: "), "{later}");
        assert!(read_exchanges("", "e").unwrap().is_empty());
        assert!(read_exchanges("\n \n", "e").unwrap().is_empty());
    }

    #[test]
    fn a_body_is_held_up_to_the_limit_and_no_further() {
        let (text, truncated) = held(&[b'x'; BODY_LIMIT + 10]);
        assert_eq!((text.len(), truncated), (BODY_LIMIT, true));
        let (text, truncated) = held(b"{}");
        assert_eq!((text.as_str(), truncated), ("{}", false));
        // Bytes that are not text are held as what text they make up.
        let (text, _) = held(&[0xff, b'a']);
        assert_eq!(text.chars().count(), 2);
    }

    #[test]
    fn a_key_is_known_by_its_shape_and_written_as_its_digest() {
        let key = "AKIAIOSFODNN7EXAMPLE";
        // Whatever the header is called; a value of no known shape is itself.
        assert_eq!(header_value("x-api-key", key), digest(key));
        assert_eq!(header_value("X-Api-Key", "v1"), "v1");
        assert_eq!(header_value("x-trace", "sk_live_x"), "sk_live_x");

        // A whole JSON string of a body. A key that is part of a longer string
        // is left there: the log says what the body said.
        let body = format!(
            r#"{{"api_key": "{key}", "id": 5000, "note": "see {key}", "made": "AKIAIOSFODNN7NOT"}}"#
        );
        let (text, truncated) = held(body.as_bytes());
        assert!(!truncated);
        assert_eq!(
            text,
            format!(
                r#"{{"api_key": "{}", "id": 5000, "note": "see {key}", "made": "AKIAIOSFODNN7NOT"}}"#,
                digest(key)
            )
        );

        // A body cut inside a string is held as it is up to there.
        let (text, _) = held(&body.as_bytes()[..30]);
        assert_eq!(text, body[..30]);

        // Text that is not JSON is not looked into.
        assert_eq!(without_keys("Bearer AKIAIOSFODNN7EXAMPLE"), "Bearer AKIAIOSFODNN7EXAMPLE");
    }
}

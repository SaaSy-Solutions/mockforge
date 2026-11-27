//! A catalogue of the kinds of weakness a pack can plant today: each with the
//! request that shows it, the answer that does, its class in the terms other
//! security tools use, and the SemSec rules whose class it is.
//!
//! `range flaws` lists it. `range new --flaw KIND --at "METHOD PATH"` plants
//! a kind at an operation of the pack it begins: the operation's answer is
//! changed as the kind says it must be, and the flaw that proves it is added
//! to the pack.
//!
//! The classes are the ones an API served by the mock can show. Most are
//! shown by one answer for one operation; mass assignment is shown by an
//! answer the operation gives to the requests that send the field alone
//! (`x-mockforge-answers`), and any origin allowed to read an answer by a
//! header of it. An injected weakness that names a parameter, SQL in a search
//! term, waits for a kind that takes the parameter's name. The client-side,
//! configuration and crypto rules of SemSec are about source code and
//! configuration, and name nothing a mock can answer.
//!
//! SemSec (Robusta Studio, MIT, https://github.com/RobustaStudio/semsec at
//! 521800cb) was read on 2026-11-26 for the classes and the ids of its rules.

use serde::Serialize;
use serde_json::{json, Value};

use super::error::{bad_recording, Result};
use super::pack::Severity;

/// A token signed `alg: none`: the header and the claims say the bearer is
/// an administrator, and the signature is empty. A verifier that only decodes
/// a token takes it.
pub const UNSIGNED_BEARER: &str = "Bearer eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJkZW1vLXVzZXItYSIsIm5hbWUiOiJBZGEiLCJyb2xlIjoiYWRtaW4ifQ.";

/// A kind to plant at an operation, as `range new --flaw KIND --at
/// "METHOD PATH"` asks for it. `path` is a path of the pack, `{id}` included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Planted<'a> {
    /// The kind of flaw.
    pub kind: &'a FlawKind,
    /// The method, in any case.
    pub method: &'a str,
    /// A path of the pack, `{id}` included.
    pub path: &'a str,
}

/// What the proving request of a kind is beside the operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Prove {
    /// Headers the request carries, with their values. Read without regard
    /// to case, as a pack reads them.
    pub with: &'static [(&'static str, &'static str)],
    /// Headers the request must not carry.
    pub without: &'static [&'static str],
    /// Text the request body holds.
    pub body_contains: Option<&'static str>,
    /// Query parameters the request carries, with their values, as they were
    /// decoded.
    pub query_with: &'static [(&'static str, &'static str)],
    /// Query parameters the request must not carry.
    pub query_without: &'static [&'static str],
}

/// What the answer of a kind holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Shows {
    /// The status code. `None` is the operation's own first success.
    pub status: Option<u16>,
    /// Fields the answer holds, with their values as JSON text.
    pub has: &'static [(&'static str, &'static str)],
    /// Text the answer holds.
    pub has_text: Option<&'static str>,
    /// Headers the answer carries, with their values. Read without regard to
    /// case, as a pack reads them.
    pub has_header: &'static [(&'static str, &'static str)],
    /// Headers the answer must not carry.
    pub lacks_header: &'static [&'static str],
}

/// What planting a kind changes in the answer of the operation. As one
/// document, `by` says which, and the rest is its own: a `when` and a `body`
/// as the JSON they are, and not as the text they are written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plant {
    /// Nothing: the operation answers a request it should have refused.
    AsRecorded,
    /// The answer gains these fields, with their values as JSON text.
    Fields {
        /// The fields, with their values as JSON text.
        fields: &'static [(&'static str, &'static str)],
    },
    /// The operation's own answer gains these fields, and is given, with
    /// them, to the requests a `when` names alone: every other request is
    /// answered as the operation answered before.
    Answer {
        /// The requests the answer is given to, as the JSON of a `when` of
        /// `x-mockforge-answers`.
        when_: &'static str,
        /// The fields, with their values as JSON text.
        fields: &'static [(&'static str, &'static str)],
    },
    /// The answer is given with these response headers, declared in the spec
    /// with the values written.
    Headers {
        /// The headers, with their values.
        headers: &'static [(&'static str, &'static str)],
    },
    /// The operation answers this failure, and nothing else.
    Failure {
        /// Its status code.
        status: u16,
        /// Its body, as JSON text.
        body: &'static str,
    },
}

impl Serialize for Plant {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let json =
            |text: &str| serde_json::from_str::<Value>(text).map_err(serde::ser::Error::custom);
        let mut plant = serializer.serialize_map(None)?;
        match self {
            Plant::AsRecorded => plant.serialize_entry("by", "as-recorded")?,
            Plant::Fields { fields } => {
                plant.serialize_entry("by", "fields")?;
                plant.serialize_entry("fields", fields)?;
            }
            Plant::Answer { when_, fields } => {
                plant.serialize_entry("by", "answer")?;
                plant.serialize_entry("when", &json(when_)?)?;
                plant.serialize_entry("fields", fields)?;
            }
            Plant::Headers { headers } => {
                plant.serialize_entry("by", "headers")?;
                plant.serialize_entry("headers", headers)?;
            }
            Plant::Failure { status, body } => {
                plant.serialize_entry("by", "failure")?;
                plant.serialize_entry("status", status)?;
                plant.serialize_entry("body", &json(body)?)?;
            }
        }
        plant.end()
    }
}

/// One kind of weakness a pack can plant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FlawKind {
    /// Lowercase words joined by `-`.
    pub id: &'static str,
    /// One line that says what is wrong.
    pub title: &'static str,
    /// How much it matters.
    pub severity: Severity,
    /// The weakness in MITRE's list.
    pub cwe: &'static str,
    /// The category in OWASP's Top 10.
    pub owasp: &'static str,
    /// What the proving request is.
    pub prove: Prove,
    /// What the answer holds.
    pub shows: Shows,
    /// What planting it changes.
    pub plant: Plant,
    /// The SemSec rules whose class it is.
    pub from: &'static [&'static str],
}

impl FlawKind {
    /// The kind with this id.
    pub fn by_id(id: &str) -> Option<&'static Self> {
        KINDS.iter().find(|kind| kind.id == id)
    }

    /// The id of a flaw of this kind at an operation.
    pub fn flaw_id(&self, method: &str, path: &str) -> String {
        format!("{}-{}", self.id, slug(method, path))
    }

    /// A flaw of this kind at `method path`, with the operation changed as
    /// the kind says its answer must be. `where_from` names the recording or
    /// the spec in errors. The flaw is returned as a pack holds one.
    pub fn plant(
        &self,
        method: &str,
        path: &str,
        operation: &mut Value,
        where_from: &str,
    ) -> Result<Value> {
        let own = own_success(operation).ok_or_else(|| {
            bad_recording(
                where_from,
                &format!(
                    "`{} {path}` answers nothing the mock can serve, so a flaw cannot be planted there",
                    method.to_ascii_uppercase()
                ),
            )
        })?;
        let mut shows = self.shows;
        if shows.status.is_none() {
            shows.status = Some(own);
        }
        match self.plant {
            Plant::AsRecorded => {}
            Plant::Fields { fields } => {
                let example = example_of(operation, &own.to_string()).ok_or_else(|| {
                    bad_recording(
                        where_from,
                        &format!(
                            "`{} {path}` answers no JSON object, so `{}` cannot be planted there",
                            method.to_ascii_uppercase(),
                            self.id
                        ),
                    )
                })?;
                for (name, value) in fields {
                    let value: Value = serde_json::from_str(value).unwrap_or_else(|error| {
                        panic!("the value of `{name}` is not JSON: {error}")
                    });
                    example[name] = value;
                }
            }
            Plant::Answer { when_, fields } => {
                let example = example_of(operation, &own.to_string()).ok_or_else(|| {
                    bad_recording(
                        where_from,
                        &format!(
                            "`{} {path}` answers no JSON object, so `{}` cannot be planted there",
                            method.to_ascii_uppercase(),
                            self.id
                        ),
                    )
                })?;
                let mut body = example.clone();
                for (name, value) in fields {
                    let value: Value = serde_json::from_str(value).unwrap_or_else(|error| {
                        panic!("the value of `{name}` is not JSON: {error}")
                    });
                    body[*name] = value;
                }
                let when: Value = serde_json::from_str(when_).unwrap_or_else(|error| {
                    panic!("the `when` of `{}` is not JSON: {error}", self.id)
                });
                // The answer joins the ones the operation already gives.
                let answers = operation
                    .as_object_mut()
                    .expect("an operation is an object")
                    .entry(mockforge_openapi::answers::EXTENSION)
                    .or_insert_with(|| json!([]));
                if !answers.is_array() {
                    *answers = json!([]);
                }
                answers.as_array_mut().expect("the answers are a list").push(json!({
                    "when": when,
                    "then": { "status": own, "body": body }
                }));
            }
            Plant::Headers { headers } => {
                let response = &mut operation["responses"][own.to_string()];
                if !response["headers"].is_object() {
                    response["headers"] = json!({});
                }
                // Declared as the spec declares a header, with the value the
                // mock is to answer with.
                for (name, value) in headers {
                    response["headers"][*name] =
                        json!({ "schema": { "type": "string" }, "example": value });
                }
            }
            Plant::Failure { status, body } => {
                let body: Value = serde_json::from_str(body).unwrap_or_else(|error| {
                    panic!("the body of `{}` is not JSON: {error}", self.id)
                });
                operation["responses"] = json!({
                    status.to_string(): {
                        "description": format!("{} ({} {path})", self.title, method.to_ascii_uppercase()),
                        "content": { "application/json": { "example": body } },
                    }
                });
            }
        }

        let mut prove = json!({ "method": method.to_ascii_uppercase(), "path": path });
        if !self.prove.with.is_empty() {
            let with: serde_json::Map<String, Value> = self
                .prove
                .with
                .iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), json!(value)))
                .collect();
            prove["with"] = Value::Object(with);
        }
        if !self.prove.without.is_empty() {
            prove["without"] = json!(self.prove.without);
        }
        if let Some(text) = self.prove.body_contains {
            prove["body_contains"] = json!(text);
        }
        if !self.prove.query_with.is_empty() {
            prove["query_with"] = Value::Object(
                self.prove
                    .query_with
                    .iter()
                    .map(|(name, value)| (name.to_string(), json!(value)))
                    .collect(),
            );
        }
        if !self.prove.query_without.is_empty() {
            prove["query_without"] = json!(self.prove.query_without);
        }
        let mut serves = json!({});
        if let Some(status) = shows.status {
            serves["status"] = json!(status);
        }
        if !shows.has.is_empty() {
            let has: serde_json::Map<String, Value> = self
                .shows
                .has
                .iter()
                .map(|(name, value)| (name.to_string(), json!(value)))
                .collect();
            serves["has"] = Value::Object(has);
        }
        if let Some(text) = shows.has_text {
            serves["has_text"] = json!(text);
        }
        if !shows.has_header.is_empty() {
            serves["has_header"] = Value::Object(
                shows
                    .has_header
                    .iter()
                    .map(|(name, value)| (name.to_ascii_lowercase(), json!(value)))
                    .collect(),
            );
        }
        if !shows.lacks_header.is_empty() {
            serves["lacks_header"] = json!(shows.lacks_header);
        }
        prove["serves"] = serves;
        Ok(json!({
            "id": self.flaw_id(method, path),
            "title": format!("{} at {} {path}", self.title, method.to_ascii_uppercase()),
            "severity": self.severity,
            "cwe": self.cwe,
            "owasp": self.owasp,
            "prove": prove,
        }))
    }
}

/// The catalogue, by id, so that a pack's digest is the same in every build.
pub const KINDS: &[FlawKind] = &[
    FlawKind {
        id: "another-customers-record",
        title: "Any record is answered as another customer's",
        severity: Severity::High,
        cwe: "CWE-639",
        owasp: "A01:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[("customer_id", "\"u_1024\"")],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Fields {
            fields: &[("customer_id", "\"u_1024\"")],
        },
        from: &[
            "py-idor-django-get-object",
            "vibe-idor-no-ownership-check",
            "vibe-prisma-deletemany-no-owner",
            "vibe-drizzle-delete-no-where",
            "vibe-mongoose-delete-no-owner",
        ],
    },
    FlawKind {
        id: "any-origin-allowed",
        title: "Any origin is allowed to read the answer",
        severity: Severity::High,
        cwe: "CWE-942",
        owasp: "A05:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[("Access-Control-Allow-Origin", "*")],
            lacks_header: &[],
        },
        plant: Plant::Headers {
            headers: &[("Access-Control-Allow-Origin", "*")],
        },
        from: &[
            "vibe-cors-wildcard",
            "vibe-cors-credentials-wildcard",
            "vibe-serverless-cors-wildcard",
            "vibe-serverless-yml-cors-star",
        ],
    },
    FlawKind {
        id: "error-tells-too-much",
        title: "A failure answers with a stack trace",
        severity: Severity::Medium,
        cwe: "CWE-209",
        owasp: "A05:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: Some(500),
            has: &[],
            has_text: Some("Traceback"),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Failure {
            status: 500,
            body: r#"{"error": "Internal error", "trace": "Traceback (most recent call last):\n  File \"app/views.py\", line 41, in handle\n    record = table.get(id)\n  File \"app/db.py\", line 88, in get\n    raise KeyError(id)\nKeyError: None"}"#,
        },
        from: &[
            "vibe-serverless-catch-error-expose",
            "vibe-lambda-raw-error-response",
            "vibe-trpc-verbose-error",
        ],
    },
    FlawKind {
        id: "key-on-the-client",
        title: "An answer carries a key the client was never to see",
        severity: Severity::High,
        cwe: "CWE-798",
        owasp: "A07:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[("api_key", "\"sk-demo-4f2a9c81e7b3d6a0\"")],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Fields {
            fields: &[("api_key", "\"sk-demo-4f2a9c81e7b3d6a0\"")],
        },
        from: &[
            "vibe-openai-key-client",
            "vibe-nextjs-public-env-secret",
            "vibe-supabase-service-key-client",
        ],
    },
    FlawKind {
        id: "mass-assignment",
        title: "A field only the server may set is set by the client",
        severity: Severity::High,
        cwe: "CWE-915",
        owasp: "A08:2021",
        prove: Prove {
            with: &[],
            without: &[],
            // The body holds the field as JSON writes it.
            body_contains: Some("\"role\""),
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[("role", "\"admin\"")],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        // The answer says `role` back to the request: what the client sent is
        // what was saved. A request that sends none of it is answered as the
        // operation answered before.
        plant: Plant::Answer {
            when_: r#"{ "body_contains": "\"role\"" }"#,
            fields: &[("role", "\"{{request.body.role}}\"")],
        },
        from: &[
            "vibe-prisma-mass-assign-create",
            "vibe-prisma-mass-assign-update",
            "vibe-mongoose-mass-assign-create",
            "vibe-mongoose-mass-assign-update",
        ],
    },
    FlawKind {
        id: "no-credential",
        title: "Taken with no credential",
        severity: Severity::Critical,
        cwe: "CWE-862",
        owasp: "A01:2021",
        prove: Prove {
            with: &[],
            without: &["authorization"],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        from: &[
            "py-django-no-auth-apiview",
            "py-flask-no-auth-state-change",
            "vibe-nextjs-api-no-auth",
            "vibe-trpc-public-mutation",
            "vibe-graphql-resolver-no-auth-mutation",
            "vibe-edge-handler-no-auth",
            "vibe-lambda-url-auth-none",
        ],
    },
    FlawKind {
        id: "password-in-answer",
        title: "An answer carries a password hash",
        severity: Severity::High,
        cwe: "CWE-200",
        owasp: "A01:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[(
                "password_hash",
                "\"$2b$12$demo0000000000000000000000000000000000000000000000000000000\"",
            )],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Fields {
            fields: &[(
                "password_hash",
                "\"$2b$12$demo0000000000000000000000000000000000000000000000000000000\"",
            )],
        },
        from: &[
            "vibe-prisma-user-data-exposure",
            "vibe-prisma-return-password",
            "vibe-trpc-return-full-user",
            "vibe-supabase-select-all",
        ],
    },
    FlawKind {
        id: "unsigned-token-accepted",
        title: "A token signed `alg: none` is taken for a credential",
        severity: Severity::Critical,
        cwe: "CWE-347",
        owasp: "A07:2021",
        prove: Prove {
            with: &[("Authorization", UNSIGNED_BEARER)],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        from: &[
            "vibe-auth-jwt-no-verify",
            "py-hardcoded-jwt-secret",
            "js-hardcoded-jwt-secret",
        ],
    },
    FlawKind {
        id: "upload-of-any-type",
        title: "An upload of any type is stored where the server serves files",
        severity: Severity::High,
        cwe: "CWE-434",
        owasp: "A04:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: Some(".php"),
            query_with: &[],
            query_without: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        from: &["py-upload-dangerous-extension", "vibe-upload-no-type-check"],
    },
];

/// The operation's own first success: the first of its responses, in the
/// order it declares them, that is a status the mock can answer.
fn own_success(operation: &Value) -> Option<u16> {
    operation
        .get("responses")?
        .as_object()?
        .keys()
        .find_map(|status| status.parse::<u16>().ok())
}

/// The answer the operation gives for a status, when it is a JSON object.
fn example_of<'a>(operation: &'a mut Value, status: &str) -> Option<&'a mut Value> {
    let example = operation
        .get_mut("responses")?
        .get_mut(status)?
        .get_mut("content")?
        .get_mut("application/json")?
        .get_mut("example")?;
    example.is_object().then_some(example)
}

/// `post-v1-orders-id` for `POST /v1/orders/{id}`.
fn slug(method: &str, path: &str) -> String {
    format!("{method} {path}")
        .to_ascii_lowercase()
        .split(|character: char| !(character.is_ascii_lowercase() || character.is_ascii_digit()))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect::<Vec<String>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::pack::{is_word, Pack, PACK_KIND, PACK_SCHEMA_VERSION};
    use crate::range::skeleton::DEFAULT_BRIEF;

    /// An operation as `range new` writes one from a recording: a success
    /// with an example that echoes the id of the path.
    fn operation() -> Value {
        json!({
            "summary": "One order",
            "parameters": [ { "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } } ],
            "responses": { "200": {
                "description": "The order",
                "content": { "application/json": { "example": { "id": "{{json:request.path.id}}", "total": 74.0 } } }
            } }
        })
    }

    #[test]
    fn the_catalogue_is_sorted_and_well_formed() {
        let ids: Vec<&str> = KINDS.iter().map(|kind| kind.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "by id, so that a pack's digest is the same in every build");
        for kind in KINDS {
            assert!(is_word(kind.id), "{}", kind.id);
            assert!(crate::class::is_cwe(kind.cwe), "{}", kind.cwe);
            assert!(crate::class::is_owasp(kind.owasp), "{}", kind.owasp);
            assert!(!kind.from.is_empty(), "{}", kind.id);
            assert!(!kind.title.is_empty());
            assert_eq!(FlawKind::by_id(kind.id), Some(kind));
            // Each value the answer must hold, or that is planted, is JSON.
            let mut values: Vec<&str> = kind.shows.has.iter().map(|(_, value)| *value).collect();
            if let Plant::Fields { fields } = kind.plant {
                values.extend(fields.iter().map(|(_, value)| *value));
            }
            for value in values {
                assert!(serde_json::from_str::<Value>(value).is_ok(), "{}: {value}", kind.id);
            }
            if let Plant::Failure { body, .. } = kind.plant {
                assert!(serde_json::from_str::<Value>(body).is_ok(), "{}", kind.id);
            }
        }
        assert_eq!(FlawKind::by_id("nope"), None);
    }

    #[test]
    fn the_document_of_a_kind_says_what_planting_holds_as_json() {
        let document = serde_json::to_value(KINDS).unwrap();
        let by_id = |id: &str| {
            document.as_array().unwrap().iter().find(|kind| kind["id"] == id).unwrap()["plant"]
                .clone()
        };
        assert_eq!(
            by_id("mass-assignment"),
            json!({
                "by": "answer",
                "when": { "body_contains": "\"role\"" },
                "fields": [["role", "\"{{request.body.role}}\""]]
            })
        );
        assert_eq!(
            by_id("error-tells-too-much"),
            json!({ "by": "failure", "status": 500, "body": { "error": "Internal error", "trace": "Traceback (most recent call last):\n  File \"app/views.py\", line 41, in handle\n    record = table.get(id)\n  File \"app/db.py\", line 88, in get\n    raise KeyError(id)\nKeyError: None" } })
        );
        assert_eq!(
            by_id("any-origin-allowed"),
            json!({ "by": "headers", "headers": [["Access-Control-Allow-Origin", "*"]] })
        );
        assert_eq!(by_id("no-credential"), json!({ "by": "as-recorded" }));
    }

    #[test]
    fn every_kind_can_be_planted_into_a_pack_begun_from_a_recording() {
        for kind in KINDS {
            // Planted into the operation the pack serves, as `range new` does it.
            let mut paths = json!({ "/orders/{id}": { "get": operation() } });
            let flaw = kind
                .plant("GET", "/orders/{id}", &mut paths["/orders/{id}"]["get"], "traffic.har")
                .unwrap();
            let document = json!({
                "schema_version": PACK_SCHEMA_VERSION, "kind": PACK_KIND, "id": "begun", "title": "Begun",
                "brief": DEFAULT_BRIEF, "credentials": { "Authorization": "Bearer demo-user-a" },
                "spec": { "openapi": "3.0.3", "info": { "title": "Begun", "version": "1" }, "paths": paths },
                "scope": { "paths": ["/orders/{id}"] },
                "flaws": [flaw],
            });
            let pack = Pack::from_json(&document.to_string(), "p")
                .unwrap_or_else(|error| panic!("{}: {error}", kind.id));
            assert_eq!(pack.problem(), None, "{}", kind.id);
            let flaw = &pack.flaws[0];
            assert_eq!(flaw.id, kind.flaw_id("GET", "/orders/{id}"));
            assert_eq!(flaw.title, format!("{} at GET /orders/{{id}}", kind.title));
            assert_eq!(flaw.cwe.as_deref(), Some(kind.cwe));
            assert_eq!(flaw.owasp.as_deref(), Some(kind.owasp));
            assert_eq!(format!("{:?}", flaw.severity), format!("{:?}", kind.severity));
            // The flaw's proof is as the kind says.
            assert_eq!(flaw.prove.claim.method, "GET");
            for name in kind.prove.without {
                assert!(flaw.prove.claim.without.contains(&name.to_string()), "{}", kind.id);
            }
            // A pack reads the names of headers without regard to case.
            for (name, value) in kind.prove.with {
                assert_eq!(
                    flaw.prove.claim.with.get(&name.to_ascii_lowercase()).map(String::as_str),
                    Some(*value)
                );
            }
            assert_eq!(flaw.prove.serves.status, kind.shows.status.or(Some(200)));
            // The operation answers what the flaw says it must.
            let (_, served) = pack.resolve("GET", "/orders/5000").unwrap();
            let expected = kind.shows.status.unwrap_or(200);
            assert!(served["responses"].get(expected.to_string()).is_some(), "{}", kind.id);
        }
    }

    #[test]
    fn what_planting_changes() {
        // A kind that adds fields, a kind that replaces the answer, and one that changes nothing.
        let mut served = operation();
        FlawKind::by_id("key-on-the-client")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "r")
            .unwrap();
        assert_eq!(
            served["responses"]["200"]["content"]["application/json"]["example"]["api_key"],
            "sk-demo-4f2a9c81e7b3d6a0"
        );
        assert_eq!(
            served["responses"]["200"]["content"]["application/json"]["example"]["id"],
            "{{json:request.path.id}}"
        );

        let mut served = operation();
        let flaw = FlawKind::by_id("error-tells-too-much")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "r")
            .unwrap();
        assert_eq!(served["responses"].as_object().unwrap().len(), 1);
        assert!(served["responses"]["500"]["content"]["application/json"]["example"]["trace"]
            .as_str()
            .unwrap()
            .contains("Traceback"));
        assert_eq!(flaw["prove"]["serves"]["status"], 500);
        assert_eq!(flaw["prove"]["serves"]["has_text"], "Traceback");

        // A kind that gives the answer a header: declared in the spec, with
        // its value, and nothing else of the answer changed.
        let mut served = operation();
        let flaw = FlawKind::by_id("any-origin-allowed")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "r")
            .unwrap();
        assert_eq!(
            served["responses"]["200"]["headers"],
            json!({ "Access-Control-Allow-Origin": { "schema": { "type": "string" }, "example": "*" } })
        );
        assert_eq!(flaw["prove"]["serves"]["has_header"]["access-control-allow-origin"], "*");

        // A kind that answers one way to the requests that send a field and
        // another to the rest: the operation's own answer gains the field,
        // said back to the request, and is one of the answers by request.
        let mut served = operation();
        let flaw = FlawKind::by_id("mass-assignment")
            .unwrap()
            .plant("PUT", "/orders/{id}", &mut served, "r")
            .unwrap();
        assert_eq!(
            served["x-mockforge-answers"],
            json!([{ "when": { "body_contains": "\"role\"" },
                     "then": { "status": 200, "body": { "id": "{{json:request.path.id}}", "total": 74.0, "role": "{{request.body.role}}" } } }])
        );
        assert_eq!(
            served["responses"]["200"]["content"]["application/json"]["example"],
            json!({ "id": "{{json:request.path.id}}", "total": 74.0 })
        );
        assert_eq!(flaw["prove"]["body_contains"], "\"role\"");

        let mut served = operation();
        let before = served.clone();
        FlawKind::by_id("no-credential")
            .unwrap()
            .plant("POST", "/orders/{id}", &mut served, "r")
            .unwrap();
        assert_eq!(served, before);

        // The id names the operation in the pack's words.
        assert_eq!(
            FlawKind::by_id("no-credential").unwrap().flaw_id("POST", "/v1/orders/{id}"),
            "no-credential-post-v1-orders-id"
        );
    }

    #[test]
    fn a_kind_cannot_be_planted_where_the_mock_answers_nothing() {
        let mut answered = json!({ "responses": { "200": { "description": "ok" } } });
        let error = FlawKind::by_id("key-on-the-client")
            .unwrap()
            .plant("GET", "/x", &mut answered, "traffic.har")
            .unwrap_err()
            .to_string();
        assert_eq!(error, "traffic.har: `GET /x` answers no JSON object, so `key-on-the-client` cannot be planted there");
        let mut nothing = json!({ "responses": {} });
        let error = FlawKind::by_id("no-credential")
            .unwrap()
            .plant("GET", "/x", &mut nothing, "traffic.har")
            .unwrap_err()
            .to_string();
        assert_eq!( error, "traffic.har: `GET /x` answers nothing the mock can serve, so a flaw cannot be planted there");
    }
}

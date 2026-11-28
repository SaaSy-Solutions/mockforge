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
//! header of it. An injected weakness is shown by an answer the operation
//! gives to the request that carries the injection alone, and to no other:
//! SQL put into a query parameter, a script reflected from one, a command, a
//! server-side request, a path, a template or a redirect. Such a kind is
//! injected through a query parameter of the operation, and takes the
//! parameter's name when it is planted (`sql-injection:q`), since no
//! catalogue can know it. The client-side, configuration and crypto rules of
//! SemSec are about source code and configuration, and name nothing a mock
//! can answer.
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

/// Where a kind's own entries name the parameter it is injected through,
/// before the name is given: as the name of a query parameter the proving
/// request carries, and as the name of one an answer is given for.
pub const PARAMETER: &str = "<parameter>";

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
    /// The query parameter the kind is injected through, for a kind that is.
    pub parameter: Option<&'a str>,
    /// The field of the operation's JSON request body the kind is injected
    /// through, for a kind that is.
    pub field: Option<&'a str>,
}

/// What a kind needs to be told about the operation it is planted at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Through {
    /// Nothing: it is planted at any operation.
    Nothing,
    /// The name of a query parameter of the operation, through which the
    /// request is injected.
    QueryParameter,
    /// The name of a field of the operation's JSON request body, through
    /// which the request is injected.
    BodyField,
    /// The name of a query parameter of the operation or of a field of its
    /// JSON request body: whichever the kind is asked for.
    QueryParameterOrBodyField,
}

impl Through {
    /// Whether a query parameter's name can be given.
    pub fn is_query(self) -> bool {
        matches!(self, Self::QueryParameter | Self::QueryParameterOrBodyField)
    }

    /// Whether the name of a field of the JSON request body can be given.
    pub fn is_body(self) -> bool {
        matches!(self, Self::BodyField | Self::QueryParameterOrBodyField)
    }
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
    /// Fields of the JSON request body whose value holds these texts, as the
    /// answer's `serves.has` looks for a field.
    pub body_with: &'static [(&'static str, &'static str)],
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
    /// To the requests a `when` names the operation answers otherwise, as the
    /// `then` says: every other request is answered as the operation answered
    /// before. Both are the JSON of `x-mockforge-answers`, and name the
    /// parameter as [`PARAMETER`].
    Otherwise {
        /// The requests the answer is given to.
        when_: &'static str,
        /// The answer. Its status is the operation's own when it names none.
        then_: &'static str,
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
            Plant::Otherwise { when_, then_ } => {
                plant.serialize_entry("by", "otherwise")?;
                plant.serialize_entry("when", &json(when_)?)?;
                plant.serialize_entry("then", &json(then_)?)?;
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
    /// What the kind needs to be told about the operation it is planted at.
    pub through: Through,
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
    /// the kind says its answer must be. `parameter` is the name of the query
    /// parameter a kind is injected through one, `field` the name of a field
    /// of the JSON request body a kind is injected through one, and both are
    /// `None` for any other kind; the operation must declare the one that is
    /// named. `where_from` names the recording or the spec in errors. The
    /// flaw is returned as a pack holds one.
    pub fn plant(
        &self,
        method: &str,
        path: &str,
        operation: &mut Value,
        where_from: &str,
        parameter: Option<&str>,
        field: Option<&str>,
    ) -> Result<Value> {
        let said = format!("`{} {path}`", method.to_ascii_uppercase());
        // The operation must declare what the request is injected through, or
        // an agent could not know to send anything through it.
        let through_query = |name: &str| {
            let declared = query_parameters(operation);
            if declared.contains(&name) {
                return Ok(());
            }
            let others = if declared.is_empty() {
                String::new()
            } else {
                format!(
                    ": it declares {}",
                    declared.iter().map(|held| format!("`{held}`")).collect::<Vec<_>>().join(", ")
                )
            };
            Err(bad_recording(
                where_from,
                &format!("{said} declares no query parameter `{name}`{others}"),
            ))
        };
        let through_body = |name: &str| {
            let declared = body_fields(operation);
            if declared.contains(&name) {
                return Ok(());
            }
            let others = if declared.is_empty() {
                " and no field of a JSON body".to_string()
            } else {
                format!(
                    ": the fields of its JSON body are {}",
                    declared.iter().map(|held| format!("`{held}`")).collect::<Vec<_>>().join(", ")
                )
            };
            Err(bad_recording(
                where_from,
                &format!("{said} declares no field `{name}` of its JSON request body{others}"),
            ))
        };
        let (parameter, field) = match self.through {
            Through::Nothing => {
                if let Some(name) = parameter.or(field) {
                    return Err(bad_recording(
                        where_from,
                        &format!("`{}` is injected through nothing: `{name}` was named", self.id),
                    ));
                }
                (None, None)
            }
            Through::QueryParameter => match (parameter, field) {
                (Some(name), None) => {
                    through_query(name)?;
                    (Some(name), None)
                }
                (None, Some(name)) => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through a query parameter, and `{name}` names a field of the body: name the parameter after the kind, as `{}:q`",
                            self.id, self.id
                        ),
                    ));
                }
                _ => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through a query parameter: name it after the kind, as `{}:q`",
                            self.id, self.id
                        ),
                    ));
                }
            },
            Through::BodyField => match (parameter, field) {
                (None, Some(name)) => {
                    through_body(name)?;
                    (None, Some(name))
                }
                (Some(name), None) => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through a field of the JSON body, and `{name}` names a query parameter: name the field after the kind, as `{}:body.password`",
                            self.id, self.id
                        ),
                    ));
                }
                _ => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through a field of the JSON body: name it after the kind, as `{}:body.password`",
                            self.id, self.id
                        ),
                    ));
                }
            },
            Through::QueryParameterOrBodyField => match (parameter, field) {
                (Some(name), None) => {
                    through_query(name)?;
                    (Some(name), None)
                }
                (None, Some(name)) => {
                    through_body(name)?;
                    (None, Some(name))
                }
                (Some(name), Some(field)) => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through one of the two, and both are named: `{name}` is a query parameter and `{field}` a field of the body",
                            self.id
                        ),
                    ));
                }
                (None, None) => {
                    return Err(bad_recording(
                        where_from,
                        &format!(
                            "`{}` is injected through a query parameter or a field of the JSON body: name it after the kind, as `{}:q` or `{}:body.q`",
                            self.id, self.id, self.id
                        ),
                    ));
                }
            },
        };
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
                answer(operation, when, json!({ "status": own, "body": body }));
            }
            Plant::Otherwise { when_, then_ } => {
                // The name takes the placeholder's place, in the request the
                // answer is given to and in the answer itself.
                let Some(name) = parameter.or(field) else {
                    panic!("a kind answering otherwise is injected through a parameter or a field")
                };
                let named = |text: &str| {
                    serde_json::from_str::<Value>(&text.replace(PARAMETER, name)).unwrap_or_else(
                        |error| panic!("the text of `{}` is not JSON: {error}", self.id),
                    )
                };
                let Value::Object(mut when) = named(when_) else {
                    panic!("the `when` of `{}` is not an object", self.id)
                };
                // Through a field of the body: what a parameter of the query
                // would have carried is what the field holds, and the answer
                // says the request's field back, not a parameter it has not.
                let from_body = if field.is_some() {
                    let held = ["query", "query_contains", "body"]
                        .iter()
                        .find_map(|carried| when.remove(*carried))
                        .unwrap_or_else(|| {
                            panic!(
                                "the `when` of `{}` names neither a query parameter nor a field of the body",
                                self.id
                            )
                        });
                    when.insert("body".to_string(), held);
                    true
                } else {
                    false
                };
                let then_text = if from_body {
                    then_.replace("request.query.", "request.body.")
                } else {
                    then_.to_string()
                };
                let Value::Object(mut then) = named(&then_text) else {
                    panic!("the answer of `{}` is not an object", self.id)
                };
                then.entry("status").or_insert_with(|| json!(own));
                answer(operation, Value::Object(when), Value::Object(then));
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
            // The name is written as it was given, and the request's field or
            // parameter is read as it was decoded.
            let named: serde_json::Map<String, Value> = self
                .prove
                .query_with
                .iter()
                .map(|(name, value)| {
                    let name = if name == &PARAMETER {
                        parameter.or(field).unwrap_or(name)
                    } else {
                        name
                    };
                    (name.to_string(), json!(value))
                })
                .collect();
            // A kind that goes through either proves with the one it was
            // planted through.
            prove[if field.is_some() {
                "body_with"
            } else {
                "query_with"
            }] = Value::Object(named);
        }
        if !self.prove.query_without.is_empty() {
            prove["query_without"] = json!(self.prove.query_without);
        }
        if !self.prove.body_with.is_empty() {
            prove["body_with"] = Value::Object(
                self.prove
                    .body_with
                    .iter()
                    .map(|(name, value)| {
                        let name = if name == &PARAMETER {
                            field.or(parameter).unwrap_or(name)
                        } else {
                            name
                        };
                        (name.to_string(), json!(value))
                    })
                    .collect(),
            );
        }
        let mut serves = json!({});
        if let Some(status) = shows.status {
            serves["status"] = json!(status);
        }
        if !shows.has.is_empty() {
            // The value is written as JSON, and read as JSON: ` "u_1024" ` is
            // the string `u_1024`, and `48210.75` a number.
            let has: serde_json::Map<String, Value> = self
                .shows
                .has
                .iter()
                .map(|(name, value)| {
                    let value: Value = serde_json::from_str(value).unwrap_or_else(|error| {
                        panic!("the value of `{name}` is not JSON: {error}")
                    });
                    (name.to_string(), value)
                })
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
        // A kind injected through a parameter or a field names it, so that
        // two of a kind at one operation, through two of either, are two flaws.
        let (id, title) = match parameter.or(field) {
            Some(name) => (
                format!("{}-{}", self.flaw_id(method, path), word(name)),
                format!("{} `{name}` of {} {path}", self.title, method.to_ascii_uppercase()),
            ),
            None => (
                self.flaw_id(method, path),
                format!("{} at {} {path}", self.title, method.to_ascii_uppercase()),
            ),
        };
        Ok(json!({
            "id": id,
            "title": title,
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
            body_with: &[],
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
        through: Through::Nothing,
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
            body_with: &[],
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
        through: Through::Nothing,
        from: &[
            "vibe-cors-wildcard",
            "vibe-cors-credentials-wildcard",
            "vibe-serverless-cors-wildcard",
            "vibe-serverless-yml-cors-star",
        ],
    },
    FlawKind {
        id: "command-injection",
        title: "A command is injected through a parameter or a field of the body,",
        severity: Severity::Critical,
        cwe: "CWE-78",
        owasp: "A03:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "127.0.0.1; id")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: Some("uid="),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "; " } }"#,
            then_: r#"{ "body": { "host": "{{request.query.<parameter>}}", "reachable": true, "time_ms": 12, "output": "uid=33(www-data) gid=33(www-data) groups=33(www-data)\n" } }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &[
            "py-cmdi-taint",
            "js-cmdi-taint",
            "java-cmdi-taint",
            "php-cmdi-taint",
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
            body_with: &[],
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
        through: Through::Nothing,
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
            body_with: &[],
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
        through: Through::Nothing,
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
            body_with: &[],
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
        through: Through::Nothing,
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
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        through: Through::Nothing,
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
        id: "nosql-injection",
        title: "A document query takes an operator where a field's value was expected",
        severity: Severity::Critical,
        cwe: "CWE-943",
        owasp: "A03:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[],
            query_without: &[],
            body_with: &[(PARAMETER, "$gt")],
        },
        shows: Shows {
            status: None,
            has: &[("authenticated", "true")],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "body": { "<parameter>": "$gt" } }"#,
            then_: r#"{ "body": { "authenticated": true, "user": { "id": "u_1024", "email": "ada@example.com", "role": "customer" } } }"#,
        },
        through: Through::BodyField,
        from: &["vibe-mongoose-query-injection"],
    },
    FlawKind {
        id: "open-redirect",
        title: "A redirect is opened through a parameter or a field of the body,",
        severity: Severity::Medium,
        cwe: "CWE-601",
        owasp: "A01:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "https://evil.example/phish")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: Some(302),
            has: &[],
            has_text: None,
            has_header: &[("Location", "https://evil.example/phish")],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "evil.example" } }"#,
            then_: r#"{ "status": 302, "headers": { "Location": "https://evil.example/phish" }, "body": {} }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &["py-redirect-taint"],
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
            body_with: &[],
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
        through: Through::Nothing,
        from: &[
            "vibe-prisma-user-data-exposure",
            "vibe-prisma-return-password",
            "vibe-trpc-return-full-user",
            "vibe-supabase-select-all",
        ],
    },
    FlawKind {
        id: "path-traversal",
        title: "A path is traversed through a parameter or a field of the body,",
        severity: Severity::High,
        cwe: "CWE-22",
        owasp: "A01:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "../../../../etc/passwd")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: Some("root:x:0:0:root"),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "../" } }"#,
            then_: r#"{ "body": { "path": "{{request.query.<parameter>}}", "name": "passwd", "content": "root:x:0:0:root:/root:/bin/bash\ndaemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\nbin:x:2:2:bin:/bin:/usr/sbin/nologin\nwww-data:x:33:33:www-data:/var/www:/usr/sbin/nologin\n" } }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &[
            "py-pathtraver-taint",
            "js-pathtraver-taint",
            "php-lfi-taint",
        ],
    },
    FlawKind {
        id: "reflected-xss",
        title: "A script is reflected through a query parameter,",
        severity: Severity::High,
        cwe: "CWE-79",
        owasp: "A03:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "<script>alert(document.domain)</script>")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: Some("<script>alert(document.domain)</script>"),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "<script>" } }"#,
            then_: r#"{ "body": { "results": [], "search": "{{request.query.<parameter>}}", "message": "Nothing matched. Did you mean shoes?" } }"#,
        },
        through: Through::QueryParameter,
        from: &["js-xss-taint-express", "php-xss-taint"],
    },
    FlawKind {
        id: "sql-injection",
        title: "SQL is injected through a parameter or a field of the body,",
        severity: Severity::Critical,
        cwe: "CWE-89",
        owasp: "A03:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "' OR '1'='1' --")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: Some(500),
            has: &[],
            has_text: Some("SQL syntax"),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "' OR " } }"#,
            then_: r#"{ "status": 500, "body": { "error": "sql_error", "message": "You have an error in your SQL syntax; check the manual that corresponds to your server version for the right syntax to use near ''OR '1'='1' --' at line 1", "query": "SELECT id, name, price FROM items WHERE name = '{{request.query.<parameter>}}' AND shop_id = 7 ORDER BY id LIMIT 20" } }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &[
            "py-sqli-taint-django",
            "py-sqli-taint-sqlalchemy",
            "js-sqli-taint",
            "java-sqli-taint",
            "php-sqli-taint",
        ],
    },
    FlawKind {
        id: "ssrf",
        title:
            "A server-side request is made to an address from a parameter or a field of the body,",
        severity: Severity::High,
        cwe: "CWE-918",
        owasp: "A10:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(
                PARAMETER,
                "http://169.254.169.254/latest/dynamic/instance-identity/document",
            )],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: Some("instanceId"),
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query_contains": { "<parameter>": "169.254.169.254" } }"#,
            then_: r#"{ "body": { "url": "{{request.query.<parameter>}}", "status_code": 200, "content_type": "text/plain", "body": "{\n  \"accountId\": \"123456789012\",\n  \"instanceId\": \"i-0c9f8e7d6c5b4a39f\",\n  \"instanceType\": \"t3.medium\",\n  \"region\": \"eu-central-1\",\n  \"privateIp\": \"10.0.14.37\"\n}" } }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &["py-ssrf-taint", "js-ssrf-taint", "php-ssrf-taint"],
    },
    FlawKind {
        id: "template-injection",
        title: "A template is injected through a parameter or a field of the body,",
        severity: Severity::High,
        cwe: "CWE-1336",
        owasp: "A03:2021",
        prove: Prove {
            with: &[],
            without: &[],
            body_contains: None,
            query_with: &[(PARAMETER, "{{7*7}}")],
            query_without: &[],
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[("greeting", "\"Hello, 49!\"")],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::Otherwise {
            when_: r#"{ "query": { "<parameter>": "{{7*7}}" } }"#,
            then_: r#"{ "body": { "greeting": "Hello, 49!", "asked": "{{request.query.<parameter>}}" } }"#,
        },
        through: Through::QueryParameterOrBodyField,
        from: &["py-ssti-taint"],
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
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        through: Through::Nothing,
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
            body_with: &[],
        },
        shows: Shows {
            status: None,
            has: &[],
            has_text: None,
            has_header: &[],
            lacks_header: &[],
        },
        plant: Plant::AsRecorded,
        through: Through::Nothing,
        from: &["py-upload-dangerous-extension", "vibe-upload-no-type-check"],
    },
];

/// Give the operation one more answer by request: the answer joins the ones
/// it already gives.
fn answer(operation: &mut Value, when: Value, then: Value) {
    let answers = operation
        .as_object_mut()
        .expect("an operation is an object")
        .entry(mockforge_openapi::answers::EXTENSION)
        .or_insert_with(|| json!([]));
    if !answers.is_array() {
        *answers = json!([]);
    }
    answers
        .as_array_mut()
        .expect("the answers are a list")
        .push(json!({ "when": when, "then": then }));
}

/// The names of the query parameters the operation declares.
fn query_parameters(operation: &Value) -> Vec<&str> {
    operation["parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|parameter| parameter["in"] == "query")
        .filter_map(|parameter| parameter["name"].as_str())
        .collect()
}

/// The fields of the JSON request body an operation declares, at the top
/// of it, in the order it declares them.
fn body_fields(operation: &Value) -> Vec<&str> {
    operation["requestBody"]["content"]["application/json"]["schema"]["properties"]
        .as_object()
        .map(|fields| fields.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

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
    word(&format!("{method} {path}"))
}

/// Lowercase words and digits joined by `-`, whatever else the text holds.
fn word(text: &str) -> String {
    text.to_ascii_lowercase()
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
            "parameters": [
                { "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } },
                { "name": "q", "in": "query", "required": false, "schema": { "type": "string" } }
            ],
            // A field of a JSON body too, for a kind injected through one.
            "requestBody": { "content": { "application/json": {
                "schema": { "type": "object", "properties": { "q": { "type": "string" } } },
                "example": { "q": "rotation" }
            } } },
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
                .plant(
                    "GET",
                    "/orders/{id}",
                    &mut paths["/orders/{id}"]["get"],
                    "traffic.har",
                    // A kind injected through a query parameter is planted
                    // through one the operation declares, and one injected
                    // through a field of the body through one it declares.
                    kind.through.is_query().then_some("q"),
                    (kind.through.is_body() && !kind.through.is_query()).then_some("q"),
                )
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
            // A kind injected through a parameter or a field names it in both.
            let through = kind.through.is_query() || kind.through.is_body();
            assert_eq!(
                flaw.id,
                if through {
                    format!("{}-q", kind.flaw_id("GET", "/orders/{id}"))
                } else {
                    kind.flaw_id("GET", "/orders/{id}")
                }
            );
            assert_eq!(
                flaw.title,
                if through {
                    format!("{} `q` of GET /orders/{{id}}", kind.title)
                } else {
                    format!("{} at GET /orders/{{id}}", kind.title)
                }
            );
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
            // The operation answers what the flaw says it must: by request
            // for a kind injected through a parameter or a field, and to every
            // request for any other. Its own answer stays for the others.
            let (_, served) = pack.resolve("GET", "/orders/5000").unwrap();
            let expected = kind.shows.status.unwrap_or(200);
            if through {
                assert_eq!(
                    served["x-mockforge-answers"][0]["then"]["status"], expected,
                    "{}",
                    kind.id
                );
                assert!(served["responses"].get("200").is_some(), "{}", kind.id);
            } else {
                assert!(served["responses"].get(expected.to_string()).is_some(), "{}", kind.id);
            }
        }
    }

    #[test]
    fn what_planting_changes() {
        // A kind that adds fields, a kind that replaces the answer, and one that changes nothing.
        let mut served = operation();
        FlawKind::by_id("key-on-the-client")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "r", None, None)
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
            .plant("GET", "/orders/{id}", &mut served, "r", None, None)
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
            .plant("GET", "/orders/{id}", &mut served, "r", None, None)
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
            .plant("PUT", "/orders/{id}", &mut served, "r", None, None)
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
            .plant("POST", "/orders/{id}", &mut served, "r", None, None)
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
            .plant("GET", "/x", &mut answered, "traffic.har", None, None)
            .unwrap_err()
            .to_string();
        assert_eq!(error, "traffic.har: `GET /x` answers no JSON object, so `key-on-the-client` cannot be planted there");
        let mut nothing = json!({ "responses": {} });
        let error = FlawKind::by_id("no-credential")
            .unwrap()
            .plant("GET", "/x", &mut nothing, "traffic.har", None, None)
            .unwrap_err()
            .to_string();
        assert_eq!( error, "traffic.har: `GET /x` answers nothing the mock can serve, so a flaw cannot be planted there");
    }
    #[test]
    fn a_kind_injected_through_a_query_parameter_takes_its_name() {
        let mut served = operation();
        let before = served.clone();
        let flaw = FlawKind::by_id("sql-injection")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "traffic.har", Some("q"), None)
            .unwrap();
        // The flaw names the parameter, the operation's own answer is as it
        // was, and the answer to the injection is one by request.
        assert_eq!(flaw["id"], "sql-injection-get-orders-id-q");
        assert_eq!(
            flaw["title"],
            "SQL is injected through a parameter or a field of the body, `q` of GET /orders/{id}"
        );
        assert_eq!(flaw["prove"]["query_with"], json!({ "q": "' OR '1'='1' --" }));
        let mut otherwise = served.clone();
        otherwise.as_object_mut().unwrap().remove("x-mockforge-answers");
        assert_eq!(otherwise, before);
        assert_eq!(
            served["x-mockforge-answers"][0]["when"],
            json!({ "query_contains": { "q": "' OR " } })
        );
        assert_eq!(served["x-mockforge-answers"][0]["then"]["status"], 500);
        assert!(
            served["x-mockforge-answers"][0]["then"]["body"]["query"]
                .as_str()
                .unwrap()
                .contains("{{request.query.q}}"),
            "{}",
            served["x-mockforge-answers"][0]
        );
        // An answer that names no status is the operation's own.
        let mut served = operation();
        FlawKind::by_id("path-traversal")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "traffic.har", Some("q"), None)
            .unwrap();
        assert_eq!(served["x-mockforge-answers"][0]["then"]["status"], 200);

        // Without the name; with one, for a kind that is injected through
        // nothing; and with one the operation does not declare.
        let refused = |id: &str, parameter: Option<&str>| {
            FlawKind::by_id(id)
                .unwrap()
                .plant("GET", "/orders/{id}", &mut operation(), "traffic.har", parameter, None)
                .unwrap_err()
                .to_string()
        };
        assert_eq!(
            refused("sql-injection", None),
            "traffic.har: `sql-injection` is injected through a query parameter or a field of the JSON body: name it after the kind, as `sql-injection:q` or `sql-injection:body.q`"
        );
        assert_eq!(
            refused("no-credential", Some("q")),
            "traffic.har: `no-credential` is injected through nothing: `q` was named"
        );
        assert_eq!(
            refused("ssrf", Some("url")),
            "traffic.har: `GET /orders/{id}` declares no query parameter `url`: it declares `q`"
        );
        let mut bare = json!({ "responses": { "200": { "description": "ok" } } });
        let error = FlawKind::by_id("ssrf")
            .unwrap()
            .plant("GET", "/x", &mut bare, "traffic.har", Some("url"), None)
            .unwrap_err()
            .to_string();
        assert_eq!(error, "traffic.har: `GET /x` declares no query parameter `url`");
    }

    #[test]
    fn a_kind_injected_through_a_field_of_the_body_takes_its_name() {
        let mut served = operation();
        let before = served.clone();
        let flaw = FlawKind::by_id("ssrf")
            .unwrap()
            .plant("GET", "/orders/{id}", &mut served, "traffic.har", None, Some("q"))
            .unwrap();
        // The flaw names the field, the operation's own answer is as it was,
        // and the answer to the injection is one by request, to the request
        // whose field holds it.
        assert_eq!(flaw["id"], "ssrf-get-orders-id-q");
        assert_eq!(
            flaw["title"],
            "A server-side request is made to an address from a parameter or a field of the body, `q` of GET /orders/{id}"
        );
        assert_eq!(
            flaw["prove"]["body_with"],
            json!({ "q": "http://169.254.169.254/latest/dynamic/instance-identity/document" })
        );
        assert!(flaw["prove"].get("query_with").is_none());
        let mut otherwise = served.clone();
        otherwise.as_object_mut().unwrap().remove("x-mockforge-answers");
        assert_eq!(otherwise, before);
        assert_eq!(
            served["x-mockforge-answers"][0]["when"],
            json!({ "body": { "q": "169.254.169.254" } })
        );
        // The answer says the field of the body back, as the query's says the parameter.
        assert_eq!(served["x-mockforge-answers"][0]["then"]["body"]["url"], "{{request.body.q}}");

        // The kind that goes through a field alone, at a login.
        let mut login = json!({
            "requestBody": { "content": { "application/json": {
                "schema": { "type": "object", "properties": { "email": { "type": "string" }, "password": { "type": "string" } } },
                "example": { "email": "ada@example.com", "password": "hunter2" }
            } } },
            "responses": { "200": { "description": "A session", "content": { "application/json": { "example": { "id": "u_1024" } } } } }
        });
        let flaw = FlawKind::by_id("nosql-injection")
            .unwrap()
            .plant("POST", "/login", &mut login, "traffic.har", None, Some("password"))
            .unwrap();
        assert_eq!(flaw["id"], "nosql-injection-post-login-password");
        assert_eq!(flaw["prove"]["body_with"], json!({ "password": "$gt" }));
        assert_eq!(
            login["x-mockforge-answers"][0]["when"],
            json!({ "body": { "password": "$gt" } })
        );
        assert_eq!(login["x-mockforge-answers"][0]["then"]["status"], 200);
        assert_eq!(login["x-mockforge-answers"][0]["then"]["body"]["authenticated"], true);

        // Without the name; with the other's name; with both; with one for a
        // kind that takes the other alone; and with one not declared.
        let refused = |id: &str, parameter: Option<&str>, field: Option<&str>| {
            FlawKind::by_id(id)
                .unwrap()
                .plant("GET", "/orders/{id}", &mut operation(), "traffic.har", parameter, field)
                .unwrap_err()
                .to_string()
        };
        assert_eq!(
            refused("nosql-injection", None, None),
            "traffic.har: `nosql-injection` is injected through a field of the JSON body: name it after the kind, as `nosql-injection:body.password`"
        );
        assert_eq!(
            refused("nosql-injection", Some("q"), None),
            "traffic.har: `nosql-injection` is injected through a field of the JSON body, and `q` names a query parameter: name the field after the kind, as `nosql-injection:body.password`"
        );
        assert_eq!(
            refused("nosql-injection", None, Some("password")),
            "traffic.har: `GET /orders/{id}` declares no field `password` of its JSON request body: the fields of its JSON body are `q`"
        );
        assert_eq!(
            refused("sql-injection", Some("q"), Some("q")),
            "traffic.har: `sql-injection` is injected through one of the two, and both are named: `q` is a query parameter and `q` a field of the body"
        );
        assert_eq!(
            refused("reflected-xss", None, Some("q")),
            "traffic.har: `reflected-xss` is injected through a query parameter, and `q` names a field of the body: name the parameter after the kind, as `reflected-xss:q`"
        );
        let mut bare = json!({ "responses": { "200": { "description": "ok" } } });
        let error = FlawKind::by_id("ssrf")
            .unwrap()
            .plant("GET", "/x", &mut bare, "traffic.har", None, Some("url"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "traffic.har: `GET /x` declares no field `url` of its JSON request body and no field of a JSON body"
        );
    }

    #[test]
    fn every_kind_injected_through_a_field_of_the_body_plants_an_answer_the_flaw_proves() {
        for kind in KINDS.iter().filter(|kind| kind.through.is_body()) {
            let text = kind
                .prove
                .body_with
                .first()
                .or_else(|| kind.prove.query_with.first())
                .unwrap()
                .1;
            let mut served = operation();
            let flaw = kind
                .plant("GET", "/orders/{id}", &mut served, "traffic.har", None, Some("q"))
                .unwrap();
            let Plant::Otherwise { when_, then_ } = kind.plant else {
                panic!("{} is injected through a field and answers otherwise", kind.id)
            };
            let written: Value = serde_json::from_str(when_).unwrap();
            let needle = ["query", "query_contains", "body"]
                .iter()
                .find_map(|carried| written[*carried][PARAMETER].as_str())
                .unwrap_or_else(|| panic!("{}: its `when` names no parameter", kind.id));
            // The flaw is proved by the field the kind names, holding the
            // injection; the answer is given to any request whose field
            // holds what marks it, and says the field back, not a parameter.
            assert!(text.contains(needle), "{}: {text} does not hold {needle}", kind.id);
            assert_eq!(flaw["prove"]["body_with"], json!({ "q": text }), "{}", kind.id);
            assert_eq!(flaw["id"], format!("{}-get-orders-id-q", kind.id));
            assert_eq!(
                served["x-mockforge-answers"][0]["when"],
                json!({ "body": { "q": needle } }),
                "{}",
                kind.id
            );
            let then = served["x-mockforge-answers"][0]["then"].to_string();
            assert!(!then.contains("request.query."), "{}", kind.id);
            assert_eq!(
                then.contains("{{request.body.q}}"),
                then_.contains("{{request.query.<parameter>}}"),
                "{}",
                kind.id
            );
            // The pack checks that the answer is one the flaw proves.
            let document = json!({
                "schema_version": PACK_SCHEMA_VERSION, "kind": PACK_KIND, "id": "begun", "title": "Begun",
                "brief": DEFAULT_BRIEF, "credentials": { "Authorization": "Bearer demo-user-a" },
                "spec": { "openapi": "3.0.3", "info": { "title": "Begun", "version": "1" },
                          "paths": { "/orders/{id}": { "get": served } } },
                "scope": { "paths": ["/orders/{id}"] },
                "flaws": [flaw],
            });
            let pack = Pack::from_json(&document.to_string(), "p")
                .unwrap_or_else(|error| panic!("{}: {error}", kind.id));
            assert_eq!(pack.problem(), None, "{}", kind.id);
        }
    }

    #[test]
    fn every_kind_injected_through_a_query_parameter_plants_an_answer_the_flaw_proves() {
        for kind in KINDS.iter().filter(|kind| kind.through.is_query()) {
            // One parameter to prove it with, exactly, and it is the placeholder.
            assert_eq!(kind.prove.query_with.len(), 1, "{}", kind.id);
            assert_eq!(kind.prove.query_with[0].0, PARAMETER, "{}", kind.id);
            // The operation answers the injection, and no request else.
            let Plant::Otherwise { when_, then_: _ } = kind.plant else {
                panic!("{} is injected through a parameter and answers otherwise", kind.id)
            };
            // The request the answer is given to names the parameter; the
            // answer may, and need not.
            assert!(when_.contains(PARAMETER), "{}", kind.id);
            let mut served = operation();
            let flaw = kind
                .plant("GET", "/orders/{id}", &mut served, "traffic.har", Some("q"), None)
                .unwrap();
            // The flaw is proved by the parameter the kind names, and is one
            // flaw of that kind through that parameter.
            assert_eq!(flaw["prove"]["query_with"], json!({ "q": kind.prove.query_with[0].1 }));
            assert_eq!(flaw["id"], format!("{}-get-orders-id-q", kind.id));
            assert_eq!(served["x-mockforge-answers"].as_array().unwrap().len(), 1);
        }
    }
}

//! Answers by request: an operation gives one answer to a request like this
//! and another to a request like that, in the `x-mockforge-answers`
//! extension.
//!
//! An operation of a spec has one answer, its example. A weakness that is
//! injected has two: SQL put in a search term, an internal address put in a
//! field meant for a URL. The API answers the other requests of the operation
//! as it always does, and the injected one another way. Planted as the
//! operation's one answer, the mock would show the weakness to every request,
//! and an agent that did nothing wrong would be shown it all the same.
//!
//! ```json
//! "x-mockforge-answers": [
//!   { "when": { "query_contains": { "status": "OR 1=1" } },
//!     "then": { "status": 200, "body": [{ "id": 5000, "customer_id": "u_1024" }] } },
//!   { "when": { "body": { "url": "169.254.169.254" } }, "then": { "status": 500 } }
//! ]
//! ```
//!
//! The mock gives the first answer whose `when` the request meets, and the
//! operation's example when none does. A `when` names what the request holds
//! ([`When`]): a parameter of its query, a header, text of its body, or a
//! field of its JSON body. What a parameter or a field must hold is one text
//! or several, any of them ([`Holds`]): an injected weakness is shown by any
//! of its marks. A `then` is the answer ([`Then`]), whose body may name the
//! request as an example may, with `{{request.body.role}}`, and whose headers
//! may too.

use std::collections::BTreeMap;

use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use openapiv3::Operation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The extension an operation carries its answers under.
pub const EXTENSION: &str = "x-mockforge-answers";

/// What the value of a parameter of the query, or of a field of a JSON body,
/// must hold: one text, or, where a list is written, any one of those texts.
/// A weakness that is injected is shown by any of several marks, as an API
/// that has it answers `; id`, `| id` and `&& id` alike.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Holds {
    /// The value holds this text.
    Text(String),
    /// The value holds any one of these texts.
    Any(Vec<String>),
}

impl Holds {
    /// One text.
    pub fn text(text: &str) -> Self {
        Self::Text(text.to_string())
    }

    /// The texts, as written.
    pub fn texts(&self) -> &[String] {
        match self {
            Self::Text(text) => std::slice::from_ref(text),
            Self::Any(texts) => texts,
        }
    }

    /// The first text: what a proving request carries when a proof names
    /// several.
    pub fn first(&self) -> &str {
        self.texts().first().map_or("", String::as_str)
    }

    /// Whether a value holds one of the texts.
    pub fn met_by(&self, value: &str) -> bool {
        self.texts().iter().any(|text| value.contains(text))
    }

    /// What is held, as a report writes it: `` `' OR ` ``, or, where there
    /// are several, `` one of `' OR `, `" OR ` ``.
    pub fn label(&self) -> String {
        let texts: Vec<&str> = self.texts().iter().map(String::as_str).collect();
        held(&texts)
    }

    /// Whether nothing is said: no text, or an empty one.
    pub fn says_nothing(&self) -> bool {
        let texts = self.texts();
        texts.is_empty() || texts.iter().any(String::is_empty)
    }
}

/// What a value must hold, as a report writes it: one text in backticks,
/// or, where there are several, `` one of `' OR `, `" OR ` ``. A text that
/// holds a backtick itself is written in double quotes, so that the list can
/// be read.
pub fn held(texts: &[&str]) -> String {
    let quoted = |text: &str| {
        if text.contains('`') {
            format!("\"{}\"", shown(text))
        } else {
            format!("`{}`", shown(text))
        }
    };
    let mut written: Vec<String> = texts.iter().map(|text| quoted(text)).collect();
    match written.len() {
        0 => String::new(),
        1 => written.remove(0),
        _ => format!("one of {}", written.join(", ")),
    }
}

impl From<&str> for Holds {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl PartialEq<&str> for Holds {
    fn eq(&self, text: &&str) -> bool {
        self.texts().iter().any(|held| held == text)
    }
}

/// What a request must be like for an answer to be the one given. Each field
/// names something the request holds; a request meets the `when` when it
/// holds all of them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    /// Query parameters the request carries, with these values. Read as they
    /// were decoded.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query: BTreeMap<String, String>,
    /// Query parameters whose value holds this text, or one of these texts
    /// where a list is written.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query_contains: BTreeMap<String, Holds>,
    /// Headers the request carries, with these values. Names are read without
    /// regard to case; values are not.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    /// Text the body holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_contains: Option<String>,
    /// Fields of a JSON body whose value holds this text. A field is looked
    /// for at the top of the body and anywhere inside an object or an array,
    /// as an answer's `serves` looks for one. A value that is not a string is
    /// read as JSON writes it, so `{"$ne": null}` holds `$ne`. The text may
    /// be several, any of them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub body: BTreeMap<String, Holds>,
}

impl When {
    /// Whether the request is one this `when` names.
    pub fn matches(&self, request: &Request<'_>) -> bool {
        self.query
            .iter()
            .all(|(name, value)| parameter(request, name).any(|asked| asked == value))
            && self
                .query_contains
                .iter()
                .all(|(name, holds)| parameter(request, name).any(|value| holds.met_by(value)))
            && self.headers.iter().all(|(name, value)| {
                request.headers.iter().any(|(held, held_value)| {
                    held.eq_ignore_ascii_case(name) && held_value == value
                })
            })
            && self
                .body_contains
                .as_deref()
                .is_none_or(|text| contains(request.body, text.as_bytes()))
            && self.body.iter().all(|(field, holds)| body_holds(request.body, field, holds))
    }

    /// The request this `when` names, as a report writes it: `` `status`
    /// holding `OR 1=1`, the `x-debug` header set to `1`, `url` holding
    /// `169.254.169.254` ``.
    pub fn label(&self) -> String {
        let mut named = Vec::new();
        for (name, value) in &self.query {
            named.push(format!("`{name}` set to `{}`", shown(value)));
        }
        for (name, holds) in &self.query_contains {
            named.push(format!("`{name}` holding {}", holds.label()));
        }
        for (name, value) in &self.headers {
            named.push(format!("the `{name}` header set to `{}`", shown(value)));
        }
        if let Some(text) = &self.body_contains {
            named.push(format!("a body holding `{}`", shown(text)));
        }
        for (field, holds) in &self.body {
            named.push(format!("`{field}` holding {}", holds.label()));
        }
        named.join(", ")
    }

    /// What keeps this `when` from naming one kind of request. `None` when it
    /// is well made. The reason completes "an answer …".
    pub fn problem(&self) -> Option<String> {
        if self.query.is_empty()
            && self.query_contains.is_empty()
            && self.headers.is_empty()
            && self.body_contains.is_none()
            && self.body.is_empty()
        {
            return Some(
                "its `when` names nothing, so its answer would be given to every request"
                    .to_string(),
            );
        }
        let empty =
            |what: &str, name: &str| Some(format!("its `when` gives `{name}` an empty {what}"));
        for (name, value) in &self.query {
            if name.is_empty() || value.is_empty() {
                return empty("value", name);
            }
        }
        for (name, holds) in &self.query_contains {
            if name.is_empty() || holds.says_nothing() {
                return empty("text", name);
            }
        }
        for (name, value) in &self.headers {
            if name.trim().is_empty() || value.trim().is_empty() {
                return empty("value", name);
            }
        }
        if self.body_contains.as_deref() == Some("") {
            return Some("its `when` names empty text for the body".to_string());
        }
        for (field, holds) in &self.body {
            if field.is_empty() || holds.says_nothing() {
                return empty("text", field);
            }
        }
        None
    }
}

/// Whether a JSON body holds `field` with a value whose text holds one of the
/// texts: at the top of the body, or anywhere inside an object or an array. A
/// value that is not a string is read as JSON writes it. A body that is not
/// whole JSON holds nothing.
pub fn body_holds(body: &[u8], field: &str, holds: &Holds) -> bool {
    serde_json::from_slice::<Value>(body)
        .is_ok_and(|document| holds.texts().iter().any(|text| field_holds(&document, field, text)))
}

/// What a JSON document holds for `field`, as text: the first one found, at
/// the top or anywhere inside an object or an array. `None` when it holds
/// none. A value that is not a string is read as JSON writes it.
pub fn field_text(document: &Value, field: &str) -> Option<String> {
    found_field(document, field).map(as_text)
}

/// A field of a JSON document, the first one found, at the top or anywhere
/// inside an object or an array.
fn found_field<'a>(document: &'a Value, field: &str) -> Option<&'a Value> {
    match document {
        Value::Object(object) => object
            .get(field)
            .or_else(|| object.values().find_map(|inner| found_field(inner, field))),
        Value::Array(items) => items.iter().find_map(|inner| found_field(inner, field)),
        _ => None,
    }
}

/// Whether a JSON document holds `field` with a value whose text holds
/// `text`: at the top, or anywhere inside an object or an array. A value that
/// is not a string is read as JSON writes it.
pub fn field_holds(document: &Value, field: &str, text: &str) -> bool {
    match document {
        Value::Object(object) => {
            object.get(field).is_some_and(|found| as_text(found).contains(text))
                || object.values().any(|inner| field_holds(inner, field, text))
        }
        Value::Array(items) => items.iter().any(|inner| field_holds(inner, field, text)),
        _ => false,
    }
}

/// A value as text: a string as it is, anything else as JSON writes it.
pub fn as_text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| serde_json::to_string(value).unwrap_or_default(), str::to_string)
}

/// The values a request's query gives a parameter, once for each time it
/// was sent.
fn parameter<'a>(request: &'a Request<'_>, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    request
        .query
        .iter()
        .filter(move |(held, _)| held == name)
        .map(|(_, value)| value.as_str())
}

/// The answer given to a request: a status, a body and headers.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Then {
    /// The status code. `200` when left out.
    #[serde(default = "ok")]
    pub status: u16,
    /// The body, as JSON. `{}` when left out. Tokens that name the request
    /// are expanded, as they are in an example.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    /// Headers of the answer. Names are read without regard to case; a value
    /// is set as it is written, with the tokens that name the request
    /// expanded as they are in the body.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

impl Then {
    /// The body: the one written, or `{}`.
    pub fn body(&self) -> Value {
        self.body.clone().unwrap_or_else(|| Value::Object(Default::default()))
    }

    /// What keeps this answer from being given. `None` when it can be. The
    /// reason completes "an answer …".
    pub fn problem(&self) -> Option<String> {
        if StatusCode::from_u16(self.status).is_err() {
            return Some(format!("its status `{}` is not one HTTP has", self.status));
        }
        for (name, value) in &self.headers {
            if HeaderName::from_bytes(name.as_bytes()).is_err() {
                return Some(format!("its header `{name}` is not a name HTTP has"));
            }
            if HeaderValue::from_str(value).is_err() {
                return Some(format!("the value of its header `{name}` cannot be sent"));
            }
        }
        None
    }
}

/// One answer, and the request it is given to.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The requests it is given to.
    pub when: When,
    /// The answer.
    pub then: Then,
}

impl Answer {
    /// The answers an operation carries in [`EXTENSION`], or why they cannot
    /// be read. None when the operation carries no such extension.
    pub fn of(operation: &Operation) -> Result<Vec<Answer>, String> {
        Self::read(operation.extensions.get(EXTENSION))
    }

    /// The answers an extension holds, or why they cannot be read.
    pub fn read(extension: Option<&Value>) -> Result<Vec<Answer>, String> {
        let Some(extension) = extension.filter(|value| !value.is_null()) else {
            return Ok(Vec::new());
        };
        serde_json::from_value(extension.clone())
            .map_err(|error| format!("`{EXTENSION}` cannot be read: {error}"))
    }

    /// What keeps this answer from being given, or its `when` from naming
    /// requests. `None` when it is well made.
    pub fn problem(&self) -> Option<String> {
        self.when.problem().or_else(|| self.then.problem())
    }

    /// The first of the answers whose `when` the request meets.
    pub fn first<'a>(answers: &'a [Answer], request: &Request<'_>) -> Option<&'a Answer> {
        answers.iter().find(|answer| answer.when.matches(request))
    }

    /// The answer, with the body it is to be given with, as a response: the
    /// headers the spec declares for its status, then its own. A declared
    /// header is kept unless the answer sets it.
    pub fn with_spec_headers(&self, declared: Vec<(String, String)>, body: Value) -> Response {
        let own = self
            .then
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        self.with_headers(declared, body, own)
    }

    /// The same, with the answer's own headers as the router expanded them:
    /// `{{request.query.next}}` in a value is what the request sent.
    pub fn with_headers(
        &self,
        declared: Vec<(String, String)>,
        body: Value,
        own: Vec<(String, String)>,
    ) -> Response {
        let status = StatusCode::from_u16(self.then.status).unwrap_or(StatusCode::OK);
        let mut response = (status, axum::Json(body)).into_response();
        set_headers(&mut response, declared, false);
        set_headers(&mut response, own, true);
        response
    }
}

/// Put headers on a response. A name or a value HTTP cannot carry is left
/// out, so a typo in a spec does not take the response down. With `set`, a
/// header already there is replaced; without, it is kept.
pub(crate) fn set_headers(response: &mut Response, headers: Vec<(String, String)>, set: bool) {
    let held = response.headers_mut();
    for (name, value) in headers {
        let (Ok(name), Ok(value)) =
            (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(&value))
        else {
            continue;
        };
        if set || !held.contains_key(&name) {
            held.insert(name, value);
        }
    }
}

/// What a request holds, as far as an answer's `when` can ask.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// The query's parameters as they were decoded, in the order they were
    /// sent.
    pub query: &'a [(String, String)],
    /// The headers by name in lower case.
    pub headers: &'a BTreeMap<String, String>,
    /// The body.
    pub body: &'a [u8],
}

/// A request as the router has it, for [`Request`].
#[derive(Clone, Debug)]
pub struct RequestOwned<'a> {
    /// The query's parameters as they were decoded.
    pub query: Vec<(String, String)>,
    /// The headers by name in lower case.
    pub headers: BTreeMap<String, String>,
    /// The body.
    pub body: &'a [u8],
}

impl<'a> RequestOwned<'a> {
    /// The request as the router has it: the query decoded from what was
    /// sent, and the headers by their names in lower case.
    pub fn of(query: &str, headers: &HeaderMap, body: &'a [u8]) -> Self {
        Self::with_query(decoded(query), headers, body)
    }

    /// The same, of a query the router has already read into its parameters.
    pub fn with_query(query: Vec<(String, String)>, headers: &HeaderMap, body: &'a [u8]) -> Self {
        Self {
            query,
            headers: headers
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_ascii_lowercase(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect(),
            body,
        }
    }

    /// What the request holds, for an answer's `when`.
    pub fn asked(&self) -> Request<'_> {
        Request {
            query: &self.query,
            headers: &self.headers,
            body: self.body,
        }
    }
}

/// A query as it was sent, after the `?`, read into its parameters.
pub fn decoded(query: &str) -> Vec<(String, String)> {
    url::form_urlencoded::parse(query.as_bytes())
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}

/// Whether the bytes hold the text, byte for byte.
fn contains(held: &[u8], text: &[u8]) -> bool {
    !text.is_empty() && held.windows(text.len()).any(|window| window == text)
}

/// A value as a report writes it: its first 40 characters, and `…` when
/// there are more.
fn shown(value: &str) -> String {
    let mut shown: String = value.chars().take(40).collect();
    if value.chars().count() > 40 {
        shown.push('…');
    }
    shown
}

fn ok() -> u16 {
    200
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request<'a>(
        query: &'a str,
        headers: &[(&'a str, &'a str)],
        body: &'a [u8],
    ) -> RequestOwned<'a> {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        RequestOwned::of(query, &map, body)
    }

    fn answers(extension: Value) -> Vec<Answer> {
        Answer::read(Some(&extension)).unwrap()
    }

    #[test]
    fn a_when_is_met_by_what_the_request_holds() {
        let when: When = serde_json::from_value(json!({
            "query": {"tenant": "acme"},
            "query_contains": {"q": "OR 1=1"},
            "headers": {"X-Debug": "1"},
            "body_contains": "LIMIT 100"
        }))
        .unwrap();
        let met = request(
            "tenant=acme&q=shoes%27%20OR%201%3D1%20--&page=2",
            &[("x-debug", "1")],
            b"{\"sql\": \"SELECT 1 LIMIT 100\"}",
        );
        assert!(when.matches(&met.asked()));
        // Each condition on its own is not enough. The query is read decoded,
        // and a header's name without regard to case.
        for unmet in [
            request("q=x%27%20OR%201%3D1&page=2", &[("x-debug", "1")], b"LIMIT 100"),
            request("tenant=acme&page=2", &[("x-debug", "1")], b"LIMIT 100"),
            request("tenant=acme&q=x%27%20OR%201%3D1", &[], b"LIMIT 100"),
            request("tenant=acme&q=x%27%20OR%201%3D1", &[("x-debug", "1")], b""),
            request("tenant=acme&q=shoes", &[("X-Debug", "1")], b"LIMIT 100"),
        ] {
            assert!(!when.matches(&unmet.asked()), "{unmet:?}");
        }
        // A parameter sent twice is met by either of its values.
        let twice = request("tenant=other&tenant=acme", &[], b"");
        assert_eq!(
            twice.asked().query,
            [
                ("tenant".to_string(), "other".to_string()),
                ("tenant".to_string(), "acme".to_string())
            ]
        );
    }

    #[test]
    fn a_when_names_its_request_as_a_report_writes_it() {
        let when: When = serde_json::from_value(json!({
            "query_contains": {"q": "OR 1=1"},
            "headers": {"X-Debug": "1"},
            "body_contains": "; cat /etc/passwd"
        }))
        .unwrap();
        assert_eq!(
            when.label(),
            "`q` holding `OR 1=1`, the `X-Debug` header set to `1`, a body holding `; cat /etc/passwd`"
        );
        let long: When = serde_json::from_value(json!({"query": {"q": "x".repeat(45)}})).unwrap();
        assert_eq!(long.label(), "`q` set to `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx…`");
    }

    #[test]
    fn a_field_of_the_json_body_is_met_by_what_its_value_holds() {
        let when: When = serde_json::from_value(json!({
            "body": {"url": "169.254.169.254", "role": "admin"}
        }))
        .unwrap();
        // At the top of the body, the two together.
        let top = request("", &[], br#"{"url": "http://169.254.169.254/latest", "role": "admin"}"#);
        assert!(when.matches(&top.asked()));
        // Fields are named sorted, as a JSON document of this workspace writes them.
        assert_eq!(when.label(), "`role` holding `admin`, `url` holding `169.254.169.254`");
        assert_eq!(
            serde_json::to_value(&when).unwrap(),
            json!({"body": {
                "url": "169.254.169.254", "role": "admin"
            }})
        );
        // Inside an object, and inside an array of the body.
        let nested =
            request("", &[], br#"{"n": 1, "target": {"url": "169.254.169.254", "role": "admin"}}"#);
        assert!(when.matches(&nested.asked()));
        let listed =
            request("", &[], br#"{"items": [{"role": "admin", "url": "x169.254.169.254"}]}"#);
        assert!(when.matches(&listed.asked()));

        // Each on its own is not enough; a field's own value must hold its
        // text, not another field's; a body that is not whole JSON holds
        // nothing; and a field is looked for by its name as it was sent.
        let unmet: [&[u8]; 6] = [
            br#"{"url": "http://169.254.169.254/latest", "role": "customer"}"#,
            br#"{"address": "169.254.169.254", "role": "admin"}"#,
            br#"{"URL": "169.254.169.254", "role": "admin"}"#,
            br#"{"url": "169.254.169.254", "role": "admin"#,
            b"url=169.254.169.254&role=admin",
            b"",
        ];
        for body in unmet {
            assert!(
                !when.matches(&request("", &[], body).asked()),
                "{}",
                String::from_utf8_lossy(body)
            );
        }

        // A value that is not a string is read as JSON writes it, so an
        // operator sent where a value was expected is met.
        let operator: When = serde_json::from_value(json!({"body": {"password": "$gt"}})).unwrap();
        let sent = request("", &[], br#"{"email": "ada@example.com", "password": {"$gt": ""}}"#);
        assert!(operator.matches(&sent.asked()));
        assert_eq!(operator.label(), "`password` holding `$gt`");
        let elsewhere = request("", &[], br#"{"email": {"$gt": ""}, "password": "hunter2"}"#);
        assert!(!operator.matches(&elsewhere.asked()));
        // With a query parameter of the same name, the query is not the body.
        let in_query = request("password=$gt", &[], br#"{"email": "ada@example.com"}"#);
        assert!(!operator.matches(&in_query.asked()));
        // Nothing asked is `{}` as it is written.
        assert_eq!(serde_json::to_value(When::default()).unwrap(), json!({}));
    }

    #[test]
    fn several_texts_are_met_by_any_one_of_them() {
        let when: When = serde_json::from_value(json!({
            "query_contains": { "host": ["localhost", "127.0.0.1", "[::1]"] },
            "body": { "url": "169.254.169.254" }
        }))
        .unwrap();
        assert_eq!(
            when.label(),
            "`host` holding one of `localhost`, `127.0.0.1`, `[::1]`, `url` holding `169.254.169.254`"
        );
        // Written as read.
        assert_eq!(
            serde_json::to_value(&when).unwrap(),
            json!({
                "query_contains": { "host": ["localhost", "127.0.0.1", "[::1]"] },
                "body": { "url": "169.254.169.254" }
            })
        );
        fn met(query: &str) -> RequestOwned<'_> {
            request(query, &[], br#"{"url": "http://169.254.169.254/latest"}"#)
        }
        for query in ["host=localhost", "host=127.0.0.1&x=1", "host=%5B%3A%3A1%5D"] {
            assert!(when.matches(&met(query).asked()), "{query}");
        }
        for query in ["host=example.net", "host=local", ""] {
            assert!(!when.matches(&met(query).asked()), "{query}");
        }
        // A list of one is the text.
        let one: When = serde_json::from_value(json!({"query_contains": {"q": ["x"]}})).unwrap();
        assert_eq!(one.label(), "`q` holding `x`");
        assert_eq!(serde_json::to_value(&one).unwrap(), json!({"query_contains": {"q": ["x"]}}));
        // An empty list, or an empty text in a list, says nothing.
        for nothing in [
            json!({"query_contains": {"q": []}}),
            json!({"body": {"q": ["x", ""]}}),
        ] {
            let when: When = serde_json::from_value(nothing.clone()).unwrap();
            assert!(when.problem().is_some(), "{nothing}");
        }
        // A mark that is a backtick is written in double quotes, so that the
        // list can be read; and the first text is what a proving request carries.
        let ticks: When =
            serde_json::from_value(json!({"query_contains": {"cmd": ["; ", "`", "$("]}})).unwrap();
        assert_eq!(ticks.label(), "`cmd` holding one of `; `, \"`\", `$(`");
        let holds = Holds::Any(vec!["a".to_string(), "b".to_string()]);
        assert_eq!((holds.first(), holds.texts().len()), ("a", 2));
        assert!(holds.met_by("xby") && !holds.met_by("xy"));
        assert_eq!(Holds::text("x").first(), "x");
    }

    #[test]
    fn an_answer_that_names_nothing_or_cannot_be_sent_is_said() {
        // Nothing is asked for: the answer would go to every request.
        let every: Answer =
            serde_json::from_value(json!({"when": {}, "then": {"status": 200}})).unwrap();
        assert_eq!(
            every.problem().as_deref(),
            Some("its `when` names nothing, so its answer would be given to every request")
        );
        let empty_text: Answer =
            serde_json::from_value(json!({"when": {"query_contains": {"q": ""}}, "then": {}}))
                .unwrap();
        assert_eq!(empty_text.problem().as_deref(), Some("its `when` gives `q` an empty text"));
        let empty_body: Answer =
            serde_json::from_value(json!({"when": {"body_contains": ""}, "then": {}})).unwrap();
        assert_eq!(
            empty_body.problem().as_deref(),
            Some("its `when` names empty text for the body")
        );
        let empty_field: Answer =
            serde_json::from_value(json!({"when": {"body": {"": "$gt"}}, "then": {}})).unwrap();
        assert_eq!(empty_field.problem().as_deref(), Some("its `when` gives `` an empty text"));
        let empty_of_field: Answer =
            serde_json::from_value(json!({"when": {"body": {"password": ""}}, "then": {}}))
                .unwrap();
        assert_eq!(
            empty_of_field.problem().as_deref(),
            Some("its `when` gives `password` an empty text")
        );

        // An answer HTTP cannot give.
        let status: Answer =
            serde_json::from_value(json!({"when": {"query": {"a": "1"}}, "then": {"status": 99}}))
                .unwrap();
        assert_eq!(status.problem().as_deref(), Some("its status `99` is not one HTTP has"));
        let header: Answer = serde_json::from_value(json!({
            "when": {"query": {"a": "1"}},
            "then": {"status": 200, "headers": {"X-Ok": "yes", "bad name": "yes"}}
        }))
        .unwrap();
        assert_eq!(
            header.problem().as_deref(),
            Some("its header `bad name` is not a name HTTP has")
        );
        let value: Answer = serde_json::from_value(json!({
            "when": {"query": {"a": "1"}},
            "then": {"headers": {"X-Ok": "a\nb"}}
        }))
        .unwrap();
        assert_eq!(
            value.problem().as_deref(),
            Some("the value of its header `X-Ok` cannot be sent")
        );
    }

    #[test]
    fn an_extension_that_cannot_be_read_is_said_and_none_is_no_answer() {
        assert!(Answer::read(None).unwrap().is_empty());
        assert!(Answer::read(Some(&Value::Null)).unwrap().is_empty());
        assert!(Answer::read(Some(&json!([]))).unwrap().is_empty());
        let broken = Answer::read(Some(&json!([{"if": {}}]))).unwrap_err();
        assert!(broken.starts_with("`x-mockforge-answers` cannot be read: "), "{broken}");
        let not_a_list = Answer::read(Some(&json!({"when": {}}))).unwrap_err();
        assert!(not_a_list.starts_with("`x-mockforge-answers` cannot be read: "), "{not_a_list}");

        // An operation carries them in its extension, and most carry none.
        let operation: Operation = serde_json::from_value(json!({
            "responses": {"200": {"description": "ok"}},
            "x-mockforge-answers": [{"when": {"query": {"a": "1"}}, "then": {"status": 201}}]
        }))
        .unwrap();
        assert_eq!(
            Answer::of(&operation).unwrap(),
            answers(json!([{"when": {"query": {"a": "1"}}, "then": {"status": 201}}]))
        );
        let plain: Operation =
            serde_json::from_value(json!({"responses": {"200": {"description": "ok"}}})).unwrap();
        assert!(Answer::of(&plain).unwrap().is_empty());
    }

    #[test]
    fn the_first_answer_whose_when_is_met_is_the_one_given() {
        let answers = answers(json!([
            {"when": {"query": {"debug": "1"}}, "then": {"status": 500, "headers": {"X-Why": "debug"}}},
            {"when": {"query_contains": {"q": "OR 1=1"}}, "then": {"status": 200, "body": [{"id": 5000}]}}
        ]));
        let injected = request("q=%27%20OR%201%3D1", &[], b"");
        let given = Answer::first(&answers, &injected.asked()).unwrap();
        assert_eq!(given.then.body(), json!([{ "id": 5000 }]));
        assert_eq!(given.then.status, 200);

        // As a response: the status, the body, the headers the spec declares
        // for that status kept, and the answer's own set over them.
        let response = given
            .with_spec_headers(vec![("X-Spec".to_string(), "kept".to_string())], given.then.body());
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-spec").unwrap(), "kept");
        assert!(!response.headers().contains_key("x-why"));

        // A request with a header by either case.
        let debug = request("debug=1", &[], b"");
        let given = Answer::first(&answers, &debug.asked()).unwrap();
        let response = given
            .with_spec_headers(vec![("X-Why".to_string(), "spec".to_string())], given.then.body());
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers().get("x-why").unwrap(), "debug");

        // The status is 200 and the body `{}` when a `then` leaves them out.
        let bare: Answer =
            serde_json::from_value(json!({"when": {"query": {"a": "1"}}, "then": {}})).unwrap();
        assert_eq!((bare.then.status, bare.then.body()), (200, json!({})));

        // No answer for a request none names.
        let other = request("q=shoes", &[], b"");
        assert!(Answer::first(&answers, &other.asked()).is_none());
        assert!(Answer::first(&[], &injected.asked()).is_none());
    }

    #[test]
    fn a_query_is_read_as_it_was_sent_and_decoded() {
        assert_eq!(
            decoded("tenant=acme&q=shoes%27%20OR%201%3D1&empty=&a=1&a=2"),
            [
                ("tenant".to_string(), "acme".to_string()),
                ("q".to_string(), "shoes' OR 1=1".to_string()),
                ("empty".to_string(), String::new()),
                ("a".to_string(), "1".to_string()),
                ("a".to_string(), "2".to_string()),
            ]
        );
        assert!(decoded("").is_empty());
    }

    #[test]
    fn the_body_is_read_as_bytes_and_empty_text_is_nowhere() {
        assert!(contains(b"SELECT 1; cat /etc/passwd", b"cat /etc/passwd"));
        assert!(contains(b"\xff\xfe<!ENTITY x SYSTEM \"file:///etc/passwd\">", b"<!ENTITY"));
        assert!(!contains(b"SELECT 1", b"cat /etc/passwd"));
        assert!(!contains(b"", b""));
        assert!(!contains(b"anything", b""));
    }
}

//! Tokens in a response that name the request that asked for it.
//!
//! `{{request.path.id}}`, `{{request.query.year}}`, `{{request.headers.x_tenant}}`
//! and `{{request.body.name}}` in an example of a spec are replaced with what
//! the request carried. A header is named as `request.headers.X` or
//! `request.header.X`, in any case. They are expanded whatever
//! `response_template_expand` says: unlike `{{now}}` and `{{uuid}}` they add
//! nothing random, and they are how an answer recorded from one request
//! becomes the answer to every request of its operation. A mock recorded from
//! an API that echoes the id it was asked for can answer as that API does,
//! whatever id comes.
//!
//! A token that is the whole of a string gives a string. Written `json:` in
//! front of it, `{{json:request.path.id}}`, it gives the value as JSON when it
//! is JSON: the number `5000` where the path said `5000`, and `acc_12` as the
//! string it is. A recording that held a number keeps it.
//!
//! A token that names nothing the request carried is left as it is written,
//! and what a token is replaced with is never looked into for another token.
//!
//! Every router of an OpenAPI spec answers from its own handler, so each makes
//! an [`Asked`] from what it has of the request and expands its answer with
//! it, last, after the override rules.

use std::collections::HashMap;

use axum::http::HeaderMap;
use mockforge_template_expansion::{expand_prompt_template, RequestContext};
use serde_json::Value;

/// The query of a request, as it came or as the handler read it.
pub enum Query<'a> {
    /// As it came after the `?`.
    Text(Option<&'a str>),
    /// Read into its parameters. Of one that comes more than once, the first
    /// is the one, whichever form the query took.
    Pairs(&'a [(String, String)]),
}

/// What a request carried.
pub struct Asked {
    method: String,
    /// The path as it was asked for, its parameters in place: `/orders/5000`.
    path: String,
    path_params: HashMap<String, String>,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Option<Value>,
}

impl Asked {
    /// From what the handler of a route has: the method, the template of the
    /// route, the parameters that were in the path, the query, the headers and
    /// the body as it came.
    pub fn of(
        method: &str,
        template: &str,
        path_params: &HashMap<String, String>,
        query: Query<'_>,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Self {
        let mut asked_path = template.to_string();
        for (name, value) in path_params {
            asked_path = asked_path.replace(&format!("{{{name}}}"), value);
        }
        let mut kept: Vec<(String, String)> = Vec::new();
        match query {
            Query::Text(Some(text)) => {
                for (name, value) in url::form_urlencoded::parse(text.as_bytes()) {
                    keep_first(&mut kept, name.to_string(), value.to_string());
                }
            }
            Query::Text(None) => {}
            Query::Pairs(pairs) => {
                for (name, value) in pairs {
                    keep_first(&mut kept, name.clone(), value.clone());
                }
            }
        }
        let headers = headers
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|value| (name.as_str().to_string(), value.to_string()))
            })
            .collect();
        let body = if body.is_empty() {
            None
        } else {
            serde_json::from_slice(body).ok()
        };
        Self {
            method: method.to_string(),
            path: asked_path,
            path_params: path_params.clone(),
            query: kept,
            headers,
            body,
        }
    }

    /// The request as the template expansion reads it.
    fn context(&self) -> RequestContext {
        RequestContext {
            method: self.method.clone(),
            path: self.path.clone(),
            path_params: self
                .path_params
                .iter()
                .map(|(name, value)| (name.clone(), Value::String(value.clone())))
                .collect(),
            query_params: self
                .query
                .iter()
                .map(|(name, value)| (name.clone(), Value::String(value.clone())))
                .collect(),
            headers: self
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), Value::String(value.clone())))
                .collect(),
            body: self.body.clone(),
            multipart_fields: HashMap::new(),
            multipart_files: HashMap::new(),
        }
    }
}

/// A parameter is kept by its first value.
fn keep_first(kept: &mut Vec<(String, String)>, name: String, value: String) {
    if !kept.iter().any(|(held, _)| *held == name) {
        kept.push((name, value));
    }
}

/// The key of a token that is the whole of a text, when it is one.
fn whole_token(text: &str) -> Option<&str> {
    let inner = text.strip_prefix("{{")?.strip_suffix("}}")?;
    match inner {
        inner if inner.contains("{{") || inner.contains("}}") => None,
        inner => Some(inner.trim()),
    }
}

/// What a key names, when the request carried it. A header is found without
/// regard to the case of its name, and `request.header.X` names it as well as
/// `request.headers.X`.
fn resolved(key: &str, context: &RequestContext) -> Option<String> {
    let key = key.strip_prefix("json:").unwrap_or(key);
    let key = key.strip_prefix("request.").unwrap_or(key);
    if let Some(name) = key.strip_prefix("headers.").or_else(|| key.strip_prefix("header.")) {
        return context.headers.iter().find(|(held, _)| held.eq_ignore_ascii_case(name)).map(
            |(_, value)| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            },
        );
    }
    let token = format!("{{{{{key}}}}}");
    let expanded = expand_prompt_template(&token, context);
    (expanded != token).then_some(expanded)
}

/// Where the token that opened before `after` closes, when it closes before
/// another opens.
fn close_of(after: &str) -> Option<usize> {
    let bytes = after.as_bytes();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'}' && bytes[at + 1] == b'}' {
            return Some(at);
        }
        if bytes[at] == b'{' && bytes[at + 1] == b'{' {
            return None;
        }
        at += 1;
    }
    None
}

/// A text with each token of the request replaced by what it names, read once
/// from left to right: what is put in place of a token is not read again.
fn expand_text(text: &str, context: &RequestContext) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match close_of(after) {
            Some(end) => {
                let token = &rest[start..start + 2 + end + 2];
                match resolved(after[..end].trim(), context) {
                    Some(value) => out.push_str(&value),
                    None => out.push_str(token),
                }
                rest = &rest[start + 2 + end + 2..];
            }
            // It opened and never closed: it is not a token.
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn expand_string(text: &str, context: &RequestContext) -> Value {
    if let Some(value) = whole_token(text.trim())
        .and_then(|key| key.strip_prefix("json:"))
        .and_then(|key| resolved(key, context))
    {
        return serde_json::from_str(&value).unwrap_or(Value::String(value));
    }
    Value::String(expand_text(text, context))
}

fn expand_in(value: Value, context: &RequestContext) -> Value {
    match value {
        Value::String(text) => expand_string(&text, context),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|item| expand_in(item, context)).collect())
        }
        Value::Object(map) => Value::Object(
            map.into_iter().map(|(name, item)| (name, expand_in(item, context))).collect(),
        ),
        other => other,
    }
}

/// Replace the tokens of the request in a response.
pub fn expand(value: Value, asked: &Asked) -> Value {
    expand_in(value, &asked.context())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn header(name: &'static str, value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_static(value));
        headers
    }

    fn asked(
        path_params: &[(&str, &str)],
        query: Query<'_>,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Asked {
        let params = path_params
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        Asked::of("GET", "/orders/{id}", &params, query, headers, body)
    }

    #[test]
    fn a_token_names_what_the_request_carried() {
        let headers = header("x-tenant", "acme");
        let asked = asked(
            &[("id", "5000")],
            Query::Text(Some("day=monday&day=tuesday")),
            &headers,
            br#"{"note":"first","amount":12}"#,
        );
        assert_eq!(
            expand(
                serde_json::json!({
                    "id": "{{json:request.path.id}}",
                    "code": "{{request.path.id}}",
                    "placed": "on {{request.query.day}}",
                    "tenant": "{{request.headers.x-tenant}}",
                    "note": "{{request.body.note}}",
                    "cost": "{{json:request.body.amount}}",
                    "tags": ["{{request.path.id}}"],
                    "n": 7,
                    "of": "{{request.path}}",
                    "how": "{{request.method}}",
                }),
                &asked
            ),
            serde_json::json!({
                "id": 5000,
                "code": "5000",
                "placed": "on monday",
                "tenant": "acme",
                "note": "first",
                "cost": 12,
                "tags": ["5000"],
                "n": 7,
                "of": "/orders/5000",
                "how": "GET",
            })
        );
    }

    #[test]
    fn a_query_the_handler_read_names_the_same_as_one_that_came() {
        let headers = HeaderMap::new();
        let pairs = [
            ("day".to_string(), "monday".to_string()),
            ("day".to_string(), "tuesday".to_string()),
        ];
        let read = asked(&[("id", "5000")], Query::Pairs(&pairs), &headers, b"");
        let came =
            asked(&[("id", "5000")], Query::Text(Some("day=monday&day=tuesday")), &headers, b"");
        let answer = serde_json::json!({ "placed": "on {{request.query.day}}", "none": "{{request.query.none}}" });
        assert_eq!(expand(answer.clone(), &read), expand(answer.clone(), &came));
        assert_eq!(expand(answer, &read)["placed"], "on monday");
    }

    #[test]
    fn a_value_that_is_not_json_stays_a_string_and_a_token_that_names_nothing_stays() {
        let headers = header("x-tenant", "acme");
        let asked =
            asked(&[("account", "acc_12")], Query::Text(None), &headers, br#"{"note":"first"}"#);
        assert_eq!(
            expand(
                serde_json::json!({
                    "account": "{{json:request.path.account}}",
                    "owner": "{{request.path.owner}}",
                    "mixed": "{{ request.path.account }} of {{request.path.owner}}",
                    "unknown": "{{uuid}}",
                    "shell": "${HOME}",
                    "open": "{{request.path.account",
                    "nested": "{{a{{request.path.account}}}}",
                }),
                &asked
            ),
            serde_json::json!({
                "account": "acc_12",
                "owner": "{{request.path.owner}}",
                "mixed": "acc_12 of {{request.path.owner}}",
                "unknown": "{{uuid}}",
                "shell": "${HOME}",
                "open": "{{request.path.account",
                // It opened twice: the first is two braces, and the second is the token.
                "nested": "{{aacc_12}}",
            })
        );
    }

    #[test]
    fn a_value_is_never_looked_into_for_another_token() {
        // A request that carries a token of its own, in a header: what it
        // gives is not read again.
        let headers = header("x-note", "{{request.path.id}}");
        let asked = asked(&[], Query::Text(None), &headers, b"");
        assert_eq!(
            expand(serde_json::json!({ "note": "{{request.headers.x-note}}" }), &asked)["note"],
            "{{request.path.id}}"
        );
    }

    #[test]
    fn a_header_is_named_without_regard_to_case_and_by_either_spelling() {
        let headers = header("X-Tenant", "acme");
        let asked = asked(&[], Query::Text(None), &headers, b"");
        for token in [
            "{{request.headers.x-tenant}}",
            "{{request.header.X-Tenant}}",
            "{{request.headers.x-TENANT}}",
            "{{headers.x-tenant}}",
        ] {
            assert_eq!(
                expand(serde_json::json!({ "tenant": token }), &asked)["tenant"],
                "acme",
                "{token}"
            );
        }
        // A header the request did not carry is a token left as written.
        assert_eq!(
            expand(serde_json::json!({ "no": "{{request.headers.x-nope}}" }), &asked)["no"],
            "{{request.headers.x-nope}}"
        );
    }

    #[test]
    fn nothing_is_asked_of_a_request_with_no_body_and_no_query() {
        let headers = HeaderMap::new();
        let asked = asked(&[], Query::Text(None), &headers, b"");
        let expanded = expand(
            serde_json::json!({
                "note": "{{request.body.note}}",
                "day": "{{request.query.day}}",
                "one": "{{request.body.note}} came to nothing",
            }),
            &asked,
        );
        assert_eq!(expanded["note"], "{{request.body.note}}");
        assert_eq!(expanded["day"], "{{request.query.day}}");
        assert_eq!(expanded["one"], "{{request.body.note}} came to nothing");
    }
}

//! A pack begun from a recording, or from a spec: its API as it is, and one
//! flaw that says so.
//!
//! [`from_har`] reads an HTTP Archive, the form every browser, proxy and
//! MockForge's recorder can write, and makes a pack whose spec answers each
//! operation as the recording answered it most often. Paths that differ only
//! in an id are one operation, `/orders/5000` and `/orders/5001` are
//! `GET /orders/{id}`, and an answer that holds the id it was asked for holds
//! a token in its place, so that it echoes whatever id comes. [`from_spec`]
//! does the same from an OpenAPI document, without a recording.
//!
//! A recording plants nothing. The pack holds one flaw that says so, at the
//! lowest severity, since a pack with no flaw is refused: it can be served and
//! judged before any is planted, and it is the first thing to replace.
//!
//! A credential is never copied. Each header of the recording that carries one
//! is named in the pack with a value to replace. The answers are what the
//! recording held: read through them for what should not be in a pack.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::{json, Value};

use super::catalogue::Planted;
use super::error::bad_recording;
use super::pack::{Pack, PACK_KIND, PACK_SCHEMA_VERSION};
use super::RangeError;
use crate::keys::Replaced;

/// The id of the flaw a pack holds until one is planted.
pub const PLACEHOLDER: &str = "nothing-planted-yet";

/// What a credential is given in a pack begun from a recording.
pub const REPLACE_ME: &str = "<replace-me>";

/// What the agent is told when no brief is asked for.
pub const DEFAULT_BRIEF: &str = "Test this API within the scope, with the credential you are \
    given. For each weakness you find, report the request that shows it and the answer that \
    proves it.";

/// The methods the mock serves.
const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// A pack begun from a recording, or from a spec, and what was found on the way.
#[derive(Debug)]
pub struct Skeleton {
    /// The pack.
    pub pack: Pack,
    /// How many exchanges of the recording the pack was made from.
    pub exchanges: usize,
    /// How many were left out: to another host, of a method the mock does not
    /// serve, or with an answer it cannot give.
    pub left_out: usize,
    /// How many operations have a parameter in their path.
    pub templated: usize,
    /// How many operations declare a query parameter.
    pub queried: usize,
    /// The headers whose values are to be replaced, as HTTP writes their names.
    pub credentials: Vec<String>,
    /// The keys of a known shape in the recorded answers that were replaced
    /// by made-up ones, so that the pack keeps the weakness and not the key.
    pub keys: Replaced,
}

/// What the person asking for a pack names.
pub struct Asked<'a> {
    /// The pack's id.
    pub id: &'a str,
    /// The pack's title, when one was given.
    pub title: Option<&'a str>,
    /// What to tell the agent.
    pub brief: &'a str,
    /// Whether to keep a key of a known shape as it was recorded.
    pub keep_keys: bool,
    /// The kinds of flaw to plant, each at an operation.
    pub flaws: &'a [Planted<'a>],
}

/// Begin a pack from an HTTP Archive. `path` names the file in errors.
pub fn from_har(text: &str, path: &str, asked: &Asked<'_>) -> Result<Skeleton, RangeError> {
    check(asked, path)?;
    let document: Value = serde_json::from_str(text)
        .map_err(|error| bad_recording(path, &format!("not an HTTP Archive: {error}")))?;
    // An archive is an object. A struct would also read a list, field by field.
    if !document.is_object() {
        return Err(bad_recording(path, "not an HTTP Archive: the document is not a JSON object"));
    }
    let archive: Archive = serde_json::from_value(document)
        .map_err(|error| bad_recording(path, &format!("not an HTTP Archive: {error}")))?;
    if archive.log.entries.is_empty() {
        return Err(bad_recording(path, "the archive holds no exchange"));
    }
    let located: Vec<(&Entry, &str, &str)> = archive
        .log
        .entries
        .iter()
        .filter_map(|entry| entry.request.locate().map(|(host, path)| (entry, host, path)))
        .filter(|(_, _, path)| !path.is_empty())
        .collect();
    // The API is the host asked most; exchanges to any other are left out.
    let mut hosts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, host, _) in &located {
        *hosts.entry(host).or_default() += 1;
    }
    let Some((host, _)) = hosts.iter().max_by_key(|(_, count)| **count) else {
        return Err(bad_recording(path, "no exchange has a request URL this reads"));
    };
    let host = *host;
    let title = asked
        .title
        .map_or_else(|| format!("The API of {host}, as recorded"), str::to_string);

    let mut left_out = 0;
    let mut groups: BTreeMap<(String, Vec<Segment>), Vec<&Entry>> = BTreeMap::new();
    for (entry, asked_host, path) in &located {
        let method = entry.request.method.to_ascii_uppercase();
        if *asked_host != host || !METHODS.contains(&method.as_str()) {
            left_out += 1;
            continue;
        }
        let shape = Segment::all(path);
        groups.entry((method, shape)).or_default().push(entry);
    }
    // From the exchanges that may be used, before they are shared out.
    let credentials = credentials(
        &located
            .iter()
            .filter_map(|(entry, asked_host, _)| (*asked_host == host).then_some(*entry))
            .collect::<Vec<&Entry>>(),
    );

    let mut paths = serde_json::Map::new();
    let mut exchanges = 0;
    let mut templated = 0;
    for ((method, shape), entries) in groups {
        match operation(&shape, &entries) {
            Some((path, parameters, operation)) => {
                exchanges += entries.len();
                templated += usize::from(!parameters.is_empty());
                let item = paths.entry(path).or_insert_with(|| json!({}));
                item[method.to_ascii_lowercase()] = operation;
            }
            None => left_out += entries.len(),
        }
    }

    finish(
        paths,
        credentials,
        asked,
        &title,
        path,
        exchanges,
        left_out,
        templated,
        asked.keep_keys,
    )
}

/// Begin a pack from an OpenAPI document.
pub fn from_spec(spec: Value, path: &str, asked: &Asked<'_>) -> Result<Skeleton, RangeError> {
    check(asked, path)?;
    let paths = spec
        .get("paths")
        .and_then(Value::as_object)
        .filter(|paths| !paths.is_empty())
        .cloned()
        .ok_or_else(|| bad_recording(path, "the spec declares no path"))?;
    let title = asked
        .title
        .map(str::to_string)
        .or_else(|| spec.pointer("/info/title").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| asked.id.to_string());
    let templated = paths.keys().filter(|path| path.contains('{')).count();
    finish(paths, BTreeMap::new(), asked, &title, path, 0, 0, templated, asked.keep_keys)
}

/// What is asked of every pack, whatever it was begun from.
fn check(asked: &Asked<'_>, path: &str) -> Result<(), RangeError> {
    if !super::is_word(asked.id) {
        return Err(bad_recording(
            path,
            &format!("the id `{}` is not lowercase words and digits joined by `-`", asked.id),
        ));
    }
    if asked.brief.trim().is_empty() {
        return Err(bad_recording(path, "the brief is empty"));
    }
    Ok(())
}

/// The pack of what was gathered, with the flaw that says nothing is planted,
/// read back as `range judge` reads it: what is written can be run.
#[allow(clippy::too_many_arguments)]
fn finish(
    paths: serde_json::Map<String, Value>,
    credentials: BTreeMap<String, String>,
    asked: &Asked<'_>,
    title: &str,
    path: &str,
    exchanges: usize,
    left_out: usize,
    templated: usize,
    keep_keys: bool,
) -> Result<Skeleton, RangeError> {
    // A key of a known shape in a recorded answer is made up, as a credential
    // is: the API keeps the weakness, a profile that shows a key, and loses
    // the key. The same key gives the same made-up one, so that a pack written
    // twice from one recording is the same pack.
    let mut held = Value::Object(paths);
    let keys = if keep_keys {
        Replaced::default()
    } else {
        crate::keys::made_up_in(&mut held)
    };
    let Value::Object(mut paths) = held else {
        return Err(bad_recording(path, "the operations are not an object"));
    };
    // Each kind asked for is planted at its operation: the operation's answer
    // is changed as the kind says, and the flaw that proves it is the pack's.
    let mut planted_flaws = Vec::new();
    for planted in asked.flaws {
        let method = planted.method.to_ascii_lowercase();
        let operation = paths.get_mut(planted.path).and_then(|item| item.get_mut(&method));
        let Some(operation) = operation else {
            return Err(bad_recording(
                path,
                &format!(
                    "`{}` cannot be planted at `{} {}`: the pack has no such operation",
                    planted.kind.id,
                    planted.method.to_ascii_uppercase(),
                    planted.path
                ),
            ));
        };
        planted_flaws.push(planted.kind.plant(
            &method,
            planted.path,
            operation,
            path,
            planted.parameter,
            planted.field,
        )?);
    }
    // The flaw is proved at the first operation, in the order of the spec,
    // that declares an answer.
    let planted = paths
        .iter()
        .flat_map(|(path, item)| {
            item.as_object().into_iter().flat_map(|operations| {
                operations.iter().filter_map(|(method, operation)| {
                    operation
                        .get("responses")
                        .and_then(Value::as_object)?
                        .keys()
                        .find_map(|status| status.parse::<u16>().ok())
                        .map(|status| (method.to_ascii_uppercase(), path.clone(), status))
                })
            })
        })
        .next()
        .ok_or_else(|| bad_recording(path, "nothing was recorded that the mock can answer"))?;
    let (method, proved_at, status) = planted;
    let document = json!({
        "schema_version": PACK_SCHEMA_VERSION,
        "kind": PACK_KIND,
        "id": asked.id,
        "title": title,
        "brief": asked.brief,
        "credentials": credentials,
        "spec": { "openapi": "3.0.3", "info": { "title": title, "version": "1" }, "paths": paths },
        "scope": { "paths": paths.keys().cloned().collect::<Vec<String>>() },
        // The placeholder says nothing is planted, so it stands only while nothing is.
        "flaws": if planted_flaws.is_empty() {
            vec![json!({
                "id": PLACEHOLDER,
                "title": "Nothing is planted yet: replace this flaw with one of the API",
                "severity": "info",
                "prove": { "method": method, "path": proved_at, "serves": { "status": status } },
            })]
        } else {
            planted_flaws
        },
    });
    let text = serde_json::to_string_pretty(&document).unwrap_or_default();
    let pack = Pack::from_json(&text, path)?;
    // Counted before anything was planted: a kind is injected through a
    // parameter an operation declares.
    let queried: usize = paths
        .values()
        .filter_map(Value::as_object)
        .map(|operations| operations.values().filter(|operation| declares_query(operation)).count())
        .sum();
    Ok(Skeleton {
        credentials: pack.credentials.keys().cloned().collect(),
        pack,
        exchanges,
        left_out,
        templated,
        queried,
        keys,
    })
}

/// Whether an operation declares a query parameter.
fn declares_query(operation: &Value) -> bool {
    operation["parameters"]
        .as_array()
        .is_some_and(|parameters| parameters.iter().any(|parameter| parameter["in"] == "query"))
}

/// The operation of one method at one shape of path: the path with its
/// parameters named, the parameters, and the operation as the spec declares
/// it. `None` when the mock cannot answer it.
fn operation(shape: &[Segment], entries: &[&Entry]) -> Option<(String, Vec<String>, Value)> {
    let (path, names) = templated(shape);
    // The status the recording answered most often, and then the first
    // exchange that answered so with a body the mock can give.
    let mut statuses: BTreeMap<u16, usize> = BTreeMap::new();
    for entry in entries {
        if let Some(status) = entry.status() {
            *statuses.entry(status).or_default() += 1;
        }
    }
    let (status, _) = statuses
        .iter()
        .map(|(status, count)| (*status, (*count, usize::MAX - usize::from(*status))))
        .max_by_key(|&(_, most)| most)?;
    let shown = entries.iter().copied().find(|entry| {
        entry.status() == Some(status)
            && entry
                .response
                .as_ref()
                .is_some_and(|response| content(response, &entry.ids(&names)).is_some())
    })?;
    let content = content(shown.response.as_ref()?, &shown.ids(&names))?;
    let mut operation = json!({
        "responses": { status.to_string(): { "description": "As the recording answered", "content": content } },
    });
    // The path's parameters, and the query parameters the recording asked
    // with: an agent finds them in the spec, and a kind is injected through one.
    let asked: BTreeSet<String> = entries
        .iter()
        .filter_map(|entry| entry.request.locate().map(|(_, path)| path))
        .filter_map(|path| {
            path.split_once('?')
                .map(|(_, query)| query.split('#').next().unwrap_or("").to_string())
        })
        .flat_map(|query| mockforge_openapi::answers::decoded(&query))
        .map(|(name, _)| name)
        .filter(|name| !name.is_empty())
        .collect();
    if !names.is_empty() || !asked.is_empty() {
        operation["parameters"] = names
            .iter()
            .map(|name| json!({ "name": name, "in": "path", "required": true, "schema": { "type": "string" } }))
            .chain(asked.iter().map(|name| {
                json!({ "name": name, "in": "query", "required": false, "schema": { "type": "string" } })
            }))
            .collect();
    }
    // What was asked of the operation with a JSON body, as the mock is to
    // read it: any subset of what was recorded is accepted.
    if let Some(body) = entries.iter().find_map(|entry| entry.request.json_body()) {
        operation["requestBody"] = json!({
            "required": true,
            "content": { "application/json": { "schema": schema_of(&body, 0), "example": body } },
        });
    }
    Some((path, names, operation))
}

/// What the mock can give of an answer: its body as an example under its
/// media type. `None` when it cannot: a body in another encoding, or none
/// where one is due.
fn content(response: &Response, ids: &BTreeMap<String, String>) -> Option<Value> {
    // Nothing is due back from these.
    if matches!(response.status, 204 | 205 | 304) {
        return Some(json!({}));
    }
    let recorded = response.content.as_ref()?;
    if recorded.encoding.as_deref().is_some_and(|e| e.eq_ignore_ascii_case("base64")) {
        return None;
    }
    let text = recorded.text.as_deref()?;
    let mime = recorded.mime();
    if mime.starts_with("text/") {
        return Some(json!({ mime: { "schema": { "type": "string" }, "example": text } }));
    }
    // JSON under another name is JSON (`application/vnd.thing+json`); anything
    // else the mock cannot give, whatever it holds.
    if !mime.ends_with("json") {
        return None;
    }
    let body = serde_json::from_str::<Value>(text).ok()?;
    Some(json!({ mime: { "example": echoing(&body, ids) } }))
}

/// The schema of a JSON value as it stands, with nothing required: an object
/// holds what it held, to a depth, and whatever else.
fn schema_of(value: &Value, depth: usize) -> Value {
    match value {
        Value::Object(map) if depth < 5 => json!({
            "type": "object",
            "additionalProperties": true,
            "properties": map.iter().map(|(key, item)| (key.clone(), schema_of(item, depth + 1))).collect::<serde_json::Map<String, Value>>(),
        }),
        Value::Array(items) if depth < 5 => json!({
            "type": "array",
            "items": items.first().map_or_else(|| json!({}), |first| schema_of(first, depth + 1)),
        }),
        Value::Object(_) => json!({ "type": "object" }),
        Value::Array(_) => json!({ "type": "array" }),
        Value::String(_) => json!({ "type": "string" }),
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Null => json!({ "type": "string", "nullable": true }),
        number => json!({ "type": if number.as_i64().is_some() { "integer" } else { "number" } }),
    }
}

/// The answer with what it holds of the ids of the request that asked for it
/// replaced by tokens that name them, under the keys that hold an id.
fn echoing(value: &Value, ids: &BTreeMap<String, String>) -> Value {
    if ids.is_empty() {
        return value.clone();
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| {
                    let token = holds_an_id(key)
                        .then(|| token_for(item, ids))
                        .flatten()
                        .map_or_else(|| echoing(item, ids), Value::String);
                    (key.clone(), token)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|item| echoing(item, ids)).collect()),
        _ => value.clone(),
    }
}

/// Whether a key of an answer holds an id: `id`, `customerId`, `order_id`.
fn holds_an_id(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key == "id" || key.ends_with("id") || key.ends_with("uuid") || key.ends_with("guid")
}

/// The token for a value that is the id the request asked for.
fn token_for(item: &Value, ids: &BTreeMap<String, String>) -> Option<String> {
    let text = match item {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        _ => return None,
    };
    let name = ids.iter().find(|(_, value)| **value == text).map(|(name, _)| name)?;
    Some(match item {
        Value::String(_) => format!("{{{{request.path.{name}}}}}"),
        _ => format!("{{{{json:request.path.{name}}}}}"),
    })
}

/// The path with its ids named, and their names in order: `/orders/5000`
/// is `/orders/{orderId}`, with `orderId`.
fn templated(shape: &[Segment]) -> (String, Vec<String>) {
    let mut taken = BTreeSet::new();
    let mut names = Vec::new();
    let mut path = String::new();
    for (position, segment) in shape.iter().enumerate() {
        path.push('/');
        match segment {
            Segment::Word(word) => path.push_str(word),
            Segment::Value => {
                let before = position.checked_sub(1).and_then(|previous| shape[previous].word());
                let name = parameter_name(before, &mut taken);
                path.push_str(&format!("{{{name}}}"));
                names.push(name);
            }
        }
    }
    (path, names)
}

/// The name of a parameter: the segment before it, singular, with `Id`; `id`
/// when nothing is before it, and a number when the name is taken.
fn parameter_name(before: Option<&str>, taken: &mut BTreeSet<String>) -> String {
    let base = before.map_or_else(|| "id".to_string(), |word| format!("{}Id", singular(word)));
    let mut name = base.clone();
    let mut count = 1;
    while !taken.insert(name.clone()) {
        count += 1;
        name = format!("{base}{count}");
    }
    name
}

/// A segment of a path in lower camel case, its last word made singular:
/// `order-items` is `orderItem`.
fn singular(word: &str) -> String {
    let mut words: Vec<String> = word
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if let Some(last) = words.last_mut().filter(|last| last.len() > 3) {
        let suffixes = [
            ("ies", "y"),
            ("sses", "ss"),
            ("ouses", "ouse"),
            ("ses", ""),
            ("xes", ""),
            ("zes", ""),
            ("ches", ""),
            ("shes", ""),
        ];
        if let Some((suffix, instead)) = suffixes.iter().find(|(suffix, _)| last.ends_with(suffix))
        {
            *last = format!("{}{instead}", &last[..last.len() - suffix.len()]);
        } else if last.ends_with('s')
            && !last.ends_with("ss")
            && !last.ends_with("us")
            && !last.ends_with("is")
        {
            last.pop();
        }
    }
    // The first word as it is, the others with a capital letter.
    let mut out = String::new();
    for (position, word) in words.iter().enumerate() {
        let mut characters = word.chars();
        if position == 0 {
            out.push_str(word);
        } else {
            out.extend(characters.next().map(|first| first.to_ascii_uppercase()));
            out.push_str(characters.as_str());
        }
    }
    out
}

/// Whether a header carries a credential, by its name.
fn is_credential(name: &str) -> bool {
    const NAMES: [&str; 7] = [
        "authorization",
        "proxy-authorization",
        "cookie",
        "set-cookie",
        "x-api-key",
        "api-key",
        "apikey",
    ];
    NAMES.contains(&name)
        || [
            "token",
            "secret",
            "session",
            "credential",
            "jwt",
            "apikey",
            "api-key",
        ]
        .iter()
        .any(|word| name.contains(word))
}

/// The credentials of the recording by the lower case of their names, each with a value to replace: its scheme kept, when it has one, and never what it carried.
fn credentials(entries: &[&Entry]) -> BTreeMap<String, String> {
    let mut credentials = BTreeMap::new();
    for header in entries.iter().flat_map(|entry| entry.request.headers.iter()) {
        let name = header.name.to_ascii_lowercase();
        if is_credential(&name) {
            let scheme = header.value.split_once(' ').map(|(scheme, _)| scheme).filter(|scheme| {
                scheme.len() <= 10 && scheme.chars().all(|c| c.is_ascii_alphabetic())
            });
            credentials.entry(name).or_insert_with(|| match scheme {
                Some(scheme) => format!("{scheme} {REPLACE_ME}"),
                None => REPLACE_ME.to_string(),
            });
        }
    }
    credentials
}

/// One segment of a path: a word, or the place of a parameter. Two paths of
/// one shape have the same segments: what the parameter was is not in them.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Segment {
    Word(String),
    Value,
}

impl Segment {
    /// The segments of a path, without its query.
    fn all(path: &str) -> Vec<Self> {
        parts(path)
            .map(|segment| {
                if is_value(segment) {
                    Segment::Value
                } else {
                    Segment::Word(segment.to_string())
                }
            })
            .collect()
    }

    fn word(&self) -> Option<&str> {
        match self {
            Segment::Word(word) => Some(word),
            Segment::Value => None,
        }
    }
}

/// The segments of a path, without its query.
fn parts(path: &str) -> impl Iterator<Item = &str> {
    path.split('?')
        .next()
        .unwrap_or(path)
        .split('/')
        .filter(|segment| !segment.is_empty())
}

/// Whether a segment of a path is the value of a parameter: a number, a UUID,
/// a word and a number with a mark between them as `acc_12` is, or a long key
/// of letters and digits. `v1` and `me` are words.
fn is_value(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }
    if segment.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }
    let groups: Vec<&str> = segment.split('-').collect();
    if groups.len() == 5
        && [8, 4, 4, 4, 12].iter().zip(&groups).all(|(len, group)| group.len() == *len)
    {
        return true;
    }
    if let Some((word, number)) = segment.split_once(['_', '-']) {
        return word.len() >= 2
            && word.chars().all(|c| c.is_ascii_alphabetic())
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit());
    }
    let letters = segment.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let digits = segment.chars().filter(|c| c.is_ascii_digit()).count();
    (segment.len() >= 10 && letters >= 2 && digits >= 2 && letters + digits == segment.len())
        || (segment.len() >= 16 && segment.chars().all(|c| c.is_ascii_hexdigit()) && digits >= 2)
}

// The HTTP Archive, as much of it as a pack is made of. An archive holds a
// great deal more, and none of it is missed.
#[derive(Deserialize)]
struct Archive {
    log: Log,
}

#[derive(Default, Deserialize)]
struct Log {
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Default, Deserialize)]
struct Entry {
    #[serde(default)]
    request: Request,
    #[serde(default)]
    response: Option<Response>,
}

impl Entry {
    fn status(&self) -> Option<u16> {
        self.response.as_ref().map(|response| response.status)
    }

    /// The values of the parameters of its path, by name.
    fn ids(&self, names: &[String]) -> BTreeMap<String, String> {
        let values = self
            .request
            .locate()
            .into_iter()
            .flat_map(|(_, path)| parts(path))
            .filter(|segment| is_value(segment));
        names.iter().cloned().zip(values.map(str::to_string)).collect()
    }
}

#[derive(Default, Deserialize)]
struct Request {
    #[serde(default)]
    method: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    headers: Vec<Header>,
    #[serde(rename = "postData", default)]
    post_data: Option<PostData>,
}

impl Request {
    /// The host and the path of its URL.
    fn locate(&self) -> Option<(&str, &str)> {
        let rest = self.url.split_once("://")?.1;
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = authority.rsplit('@').next()?.split(':').next().unwrap_or(authority);
        Some((host, path))
    }

    /// What it asked, when it asked it in JSON.
    fn json_body(&self) -> Option<Value> {
        let text = self.post_data.as_ref()?.text.as_deref()?;
        serde_json::from_str(text).ok()
    }
}

#[derive(Default, Deserialize)]
struct Header {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Default, Deserialize)]
struct PostData {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    status: u16,
    #[serde(default)]
    content: Option<Content>,
}

#[derive(Default, Deserialize)]
struct Content {
    #[serde(rename = "mimeType", default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    encoding: Option<String>,
}

impl Content {
    /// The media type of the body, without its parameters.
    fn mime(&self) -> &str {
        self.mime_type
            .as_deref()
            .unwrap_or("application/json")
            .split(';')
            .next()
            .unwrap_or("application/json")
            .trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An exchange of an archive, as one is written.
    fn exchange(
        url: &str,
        method: &str,
        status: u16,
        mime: &str,
        body: &str,
        sent: &str,
    ) -> String {
        format!(
            r#"{{ "request": {{ "method": "{method}", "url": "{url}", "httpVersion": "HTTP/1.1",
                  "headers": [ {{ "name": "Accept", "value": "application/json" }},
                               {{ "name": "Authorization", "value": "Bearer sk-live-9f2c61" }} ],
                  "postData": {} }},
                 "response": {{ "status": {status}, "content": {{ "mimeType": "{mime}", "text": {} }} }},
                 "timings": {{ "send": 1 }} }}"#,
            if sent.is_empty() {
                "null".to_string()
            } else {
                format!(
                    "{{ \"mimeType\": \"application/json\", \"text\": {} }}",
                    serde_json::to_string(sent).unwrap()
                )
            },
            serde_json::to_string(body).unwrap(),
        )
    }

    fn archive(exchanges: &[String]) -> String {
        format!(
            r#"{{ "log": {{ "version": "1.2", "creator": {{ "name": "test" }}, "entries": [{}] }} }}"#,
            exchanges.join(",")
        )
    }

    fn asked<'a>(id: &'a str) -> Asked<'a> {
        Asked {
            id,
            title: None,
            brief: DEFAULT_BRIEF,
            keep_keys: false,
            flaws: &[],
        }
    }

    #[test]
    fn a_recording_becomes_a_pack_that_answers_as_it_did() {
        let text = archive(&[
            exchange(
                "https://api.example.net/v1/orders/5000",
                "GET",
                200,
                "application/json",
                r#"{"id":5000,"total":42,"status":"paid"}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders/5001",
                "GET",
                200,
                "application/json",
                r#"{"id":5001,"total":7,"status":"open"}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders/acc_9",
                "GET",
                200,
                "application/json",
                r#"{"id":"acc_9","total":1}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders",
                "POST",
                201,
                "application/json",
                r#"{"id":5002,"total":9}"#,
                r#"{"total":9,"note":"rush"}"#,
            ),
            exchange(
                "https://api.example.net/v1/me",
                "GET",
                200,
                "application/json",
                r#"{"id":77,"name":"Ada"}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders/5000/items/3",
                "GET",
                200,
                "application/json",
                r#"{"id":3,"order_id":5000,"qty":2}"#,
                "",
            ),
        ]);
        let skeleton = from_har(&text, "traffic.har", &asked("orders")).unwrap();
        let pack = &skeleton.pack;
        assert_eq!((skeleton.exchanges, skeleton.left_out, skeleton.templated), (6, 0, 2));
        assert_eq!(pack.title, "The API of api.example.net, as recorded");
        assert_eq!(
            pack.credentials,
            BTreeMap::from([("Authorization".to_string(), "Bearer <replace-me>".to_string())])
        );
        assert_eq!(skeleton.credentials, ["Authorization".to_string()]);
        let paths = pack.spec["paths"].as_object().unwrap();
        assert_eq!(
            paths.keys().collect::<Vec<_>>(),
            [
                "/v1/me",
                "/v1/orders",
                "/v1/orders/{orderId}",
                "/v1/orders/{orderId}/items/{itemId}"
            ]
        );
        // The answer the recording gave most often, the first of those, with the id it was asked for replaced by a token.
        assert_eq!(
            pack.spec["paths"]["/v1/orders/{orderId}"]["get"]["responses"]["200"]["content"]
                ["application/json"]["example"],
            json!({ "id": "{{json:request.path.orderId}}", "total": 42, "status": "paid" })
        );
        assert_eq!(
            pack.spec["paths"]["/v1/orders/{orderId}"]["get"]["parameters"],
            json!([{ "name": "orderId", "in": "path", "required": true, "schema": { "type": "string" } }])
        );
        assert_eq!(
            pack.spec["paths"]["/v1/orders/{orderId}/items/{itemId}"]["get"]["responses"]["200"]
                ["content"]["application/json"]["example"],
            json!({ "id": "{{json:request.path.itemId}}", "order_id": "{{json:request.path.orderId}}", "qty": 2 })
        );
        // What was asked is read as it was recorded, and nothing is required.
        let sent =
            &pack.spec["paths"]["/v1/orders"]["post"]["requestBody"]["content"]["application/json"];
        assert_eq!(sent["example"], json!({ "total": 9, "note": "rush" }));
        assert_eq!(sent["schema"]["properties"]["total"], json!({ "type": "integer" }));
        assert!(sent["schema"].get("required").is_none());
        assert_eq!(pack.scope.paths, paths.keys().cloned().collect::<Vec<String>>());
        // One flaw, that says nothing is planted, proved at the first operation.
        assert_eq!(pack.flaws.len(), 1);
        assert_eq!(
            (pack.flaws[0].id.as_str(), pack.flaws[0].severity.label()),
            (PLACEHOLDER, "info")
        );
        assert_eq!(
            (
                pack.flaws[0].prove.claim.method.as_str(),
                pack.flaws[0].prove.claim.path.as_str()
            ),
            ("GET", "/v1/me")
        );
        assert_eq!(pack.flaws[0].prove.serves.status, Some(200));
        assert!(pack.injections.is_empty() && pack.bait.is_empty());
        // The credential of the recording is nowhere in the pack.
        assert!(!serde_json::to_string(pack).unwrap().contains("9f2c61"));
    }

    #[test]
    fn what_cannot_be_served_is_left_out_and_counted() {
        let text = archive(&[
            exchange(
                "https://api.example.net/v1/orders/5000",
                "GET",
                200,
                "application/json",
                r#"{"id":5000}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders/5001",
                "GET",
                200,
                "application/json",
                r#"{"id":5001}"#,
                "",
            ),
            // Another host, a method the mock does not serve, and a body in another encoding.
            exchange(
                "https://cdn.example.net/app.js",
                "GET",
                200,
                "application/javascript",
                "window.x=1;",
                "",
            ),
            exchange("https://api.example.net/v1/orders/5000", "OPTIONS", 204, "-", "", ""),
            exchange("https://api.example.net/v1/avatar.png", "GET", 200, "image/png", "", ""),
            // Answered 404 once and 200 twice: 200 is the answer.
            exchange(
                "https://api.example.net/v1/orders/acc_9",
                "GET",
                404,
                "application/json",
                r#"{"error":"none"}"#,
                "",
            ),
        ]);
        let skeleton = from_har(&text, "traffic.har", &asked("orders")).unwrap();
        assert_eq!((skeleton.exchanges, skeleton.left_out), (3, 3));
        let paths = skeleton.pack.spec["paths"].as_object().unwrap();
        assert_eq!(paths.keys().collect::<Vec<_>>(), ["/v1/orders/{orderId}"]);
        assert!(skeleton.pack.spec["paths"]["/v1/orders/{orderId}"]["get"]["responses"]
            .get("404")
            .is_none());
    }

    #[test]
    fn a_text_answer_is_served_as_text_and_nothing_as_nothing() {
        let text = archive(&[
            exchange(
                "https://api.example.net/v1/help",
                "GET",
                200,
                "text/plain; charset=utf-8",
                "Write to us.",
                "",
            ),
            exchange("https://api.example.net/v1/orders/5000", "DELETE", 204, "-", "", ""),
        ]);
        let skeleton = from_har(&text, "traffic.har", &asked("api")).unwrap();
        assert_eq!(
            skeleton.pack.spec["paths"]["/v1/help"]["get"]["responses"]["200"]["content"],
            json!({ "text/plain": { "schema": { "type": "string" }, "example": "Write to us." } })
        );
        assert_eq!(
            skeleton.pack.spec["paths"]["/v1/orders/{orderId}"]["delete"]["responses"]["204"],
            json!({
            "description": "As the recording answered", "content": {} })
        );
    }

    #[test]
    fn a_credential_keeps_its_scheme_and_never_its_value() {
        let entry = Entry {
            request: Request {
                method: "GET".to_string(),
                url: "https://api.example.net/v1/a".to_string(),
                headers: vec![
                    Header {
                        name: "Authorization".to_string(),
                        value: "Bearer sk-live-9f2c61".to_string(),
                    },
                    Header {
                        name: "Cookie".to_string(),
                        value: "sid=8a7f; theme=dark".to_string(),
                    },
                    Header {
                        name: "X-Api-Key".to_string(),
                        value: "k-123".to_string(),
                    },
                    Header {
                        name: "X-Tenant".to_string(),
                        value: "acme".to_string(),
                    },
                    Header {
                        name: "Proxy-Authorization".to_string(),
                        value: "Basic dXNlcjpwYXNz".to_string(),
                    },
                ],
                post_data: None,
            },
            response: None,
        };
        let kept = credentials(&[&entry]);
        assert_eq!(
            kept,
            BTreeMap::from([
                ("authorization".to_string(), "Bearer <replace-me>".to_string()),
                ("cookie".to_string(), "<replace-me>".to_string()),
                ("x-api-key".to_string(), "<replace-me>".to_string()),
                ("proxy-authorization".to_string(), "Basic <replace-me>".to_string()),
            ])
        );
        // None of what was sent is in any of them.
        assert!(!kept.values().any(|value| {
            ["9f2c61", "8a7f", "k-123", "acme", "dXNlcjpwYXNz"]
                .iter()
                .any(|sent| value.contains(sent))
        }));
    }

    #[test]
    fn a_segment_is_a_value_by_its_shape() {
        for value in [
            "5000",
            "0",
            "3f2a9c1e-77bd-4e0a-9a51-2c6d8e4f1b20",
            "acc_12",
            "order-5000",
            "60f1c9b8e4d",
            "deadbeefdeadbeef01",
        ] {
            assert!(is_value(value), "{value}");
        }
        for word in [
            "v1",
            "me",
            "admin",
            "orders",
            "stats",
            "2026-11-26",
            "id",
            "a1",
        ] {
            assert!(!is_value(word), "{word}");
        }
        assert_eq!(singular("orders"), "order");
        assert_eq!(singular("order-items"), "orderItem");
        assert_eq!(singular("categories"), "category");
        assert_eq!(singular("addresses"), "address");
        assert_eq!(singular("status"), "status");
        assert_eq!(singular("bus"), "bus");
        let (path, names) = templated(&Segment::all("/accounts/acc_12/statements/2025"));
        assert_eq!(
            (path.as_str(), names),
            (
                "/accounts/{accountId}/statements/{statementId}",
                vec!["accountId".to_string(), "statementId".to_string()]
            )
        );
        let (path, names) = templated(&Segment::all("/5000/3"));
        assert_eq!(
            (path.as_str(), names),
            ("/{id}/{id2}", vec!["id".to_string(), "id2".to_string()])
        );
    }

    #[test]
    fn what_is_asked_is_checked_before_anything_is_made() {
        let text = archive(&[exchange(
            "https://api.example.net/v1/a",
            "GET",
            200,
            "application/json",
            "{}",
            "",
        )]);
        let wrong = from_har(
            &text,
            "traffic.har",
            &Asked {
                id: "My Api",
                title: None,
                brief: DEFAULT_BRIEF,
                keep_keys: false,
                flaws: &[],
            },
        );
        assert!(wrong
            .unwrap_err()
            .to_string()
            .contains("the id `My Api` is not lowercase words"));
        let empty_brief = from_har(
            &text,
            "traffic.har",
            &Asked {
                id: "api",
                title: None,
                brief: " ",
                keep_keys: false,
                flaws: &[],
            },
        );
        assert!(empty_brief.unwrap_err().to_string().contains("the brief is empty"));
        let not_an_archive = from_har("[]", "traffic.har", &asked("api"));
        assert!(not_an_archive.unwrap_err().to_string().contains("not an HTTP Archive"));
        let nothing = from_har(&archive(&[]), "traffic.har", &asked("api"));
        assert!(nothing.unwrap_err().to_string().contains("holds no exchange"));
        let unservable = from_har(
            &archive(&[exchange(
                "https://api.example.net/v1/a.png",
                "GET",
                200,
                "image/png",
                "",
                "",
            )]),
            "traffic.har",
            &asked("api"),
        );
        assert!(unservable
            .unwrap_err()
            .to_string()
            .contains("nothing was recorded that the mock can answer"));
    }

    #[test]
    fn a_spec_becomes_a_pack_with_the_same_flaw() {
        let spec = json!({
            "openapi": "3.0.3", "info": { "title": "Ledger", "version": "2" },
            "paths": {
                "/transfers": { "post": { "responses": { "201": { "description": "made" } } } },
                "/transfers/{id}": { "get": { "responses": { "200": { "description": "one" } } } }
            }
        });
        let skeleton = from_spec(spec, "ledger.json", &asked("ledger")).unwrap();
        assert_eq!(skeleton.pack.title, "Ledger");
        assert_eq!((skeleton.exchanges, skeleton.left_out, skeleton.templated), (0, 0, 1));
        assert!(skeleton.pack.credentials.is_empty());
        assert_eq!(skeleton.pack.flaws[0].prove.claim.path, "/transfers");
        assert_eq!(skeleton.pack.flaws[0].prove.serves.status, Some(201));
        let no_path =
            from_spec(json!({ "openapi": "3.0.3", "paths": {} }), "ledger.json", &asked("ledger"));
        assert!(no_path.unwrap_err().to_string().contains("the spec declares no path"));
    }

    #[test]
    fn a_key_of_a_known_shape_in_a_recording_is_made_up_and_counted() {
        let key = "sk_live_4f2a9c81e7b3d6a04f2a9c81";
        let recording = archive(&[exchange(
            "https://api.example.net/v1/me",
            "GET",
            200,
            "application/json",
            &format!(r#"{{"id": 77, "api_key": "{key}", "note": "see {key}"}}"#),
            "",
        )]);
        let skeleton = from_har(&recording, "traffic.har", &asked("me")).unwrap();
        assert_eq!(skeleton.keys.count, 1);
        assert_eq!(skeleton.keys.providers, ["Stripe"]);
        let answer = &skeleton.pack.spec["paths"]["/v1/me"]["get"]["responses"]["200"]["content"]
            ["application/json"]["example"];
        let made = answer["api_key"].as_str().unwrap();
        assert_eq!(crate::keys::format_of(made).unwrap().provider, "Stripe");
        assert_ne!(made, key);
        // What was not a whole key is as it was recorded.
        assert_eq!(answer["note"], format!("see {key}"));

        // The same recording gives the same made-up key, and --keep-keys keeps the key.
        let again = from_har(&recording, "traffic.har", &asked("me")).unwrap();
        assert_eq!(again.pack, skeleton.pack);
        let mut keeping = asked("me");
        keeping.keep_keys = true;
        let kept = from_har(&recording, "traffic.har", &keeping).unwrap();
        assert_eq!(kept.keys, Replaced::default());
        assert_eq!(
            kept.pack.spec["paths"]["/v1/me"]["get"]["responses"]["200"]["content"]
                ["application/json"]["example"]["api_key"],
            key
        );
    }

    #[test]
    fn a_kind_of_flaw_is_planted_at_the_operation_it_was_asked_for() {
        let recording = archive(&[
            exchange(
                "https://api.example.net/v1/me",
                "GET",
                200,
                "application/json",
                r#"{"id": 77, "name": "Ada"}"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders/5000",
                "GET",
                200,
                "application/json",
                r#"{"id": 5000, "total": 42}"#,
                "",
            ),
        ]);
        let no_credential = crate::range::catalogue::FlawKind::by_id("no-credential").unwrap();
        let key = crate::range::catalogue::FlawKind::by_id("key-on-the-client").unwrap();
        let flaws = [
            Planted {
                kind: no_credential,
                method: "get",
                path: "/v1/orders/{orderId}",
                parameter: None,
                field: None,
            },
            Planted {
                kind: key,
                method: "GET",
                path: "/v1/me",
                parameter: None,
                field: None,
            },
        ];
        let mut asked = asked("api");
        asked.flaws = &flaws;
        let skeleton = from_har(&recording, "traffic.har", &asked).unwrap();

        // The pack holds what was asked for, and no placeholder.
        let pack = &skeleton.pack;
        let ids: Vec<&str> = pack.flaws.iter().map(|flaw| flaw.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "no-credential-get-v1-orders-orderid",
                "key-on-the-client-get-v1-me"
            ]
        );
        assert_eq!(pack.flaws[0].prove.claim.method, "GET");
        assert_eq!(pack.flaws[0].prove.label(), "GET /v1/orders/{orderId} without authorization");
        // The operation's answer gained the key, and the other answer is as recorded.
        let (_, me) = pack.resolve("GET", "/v1/me").unwrap();
        assert_eq!(
            me["responses"]["200"]["content"]["application/json"]["example"]["api_key"],
            "sk-demo-4f2a9c81e7b3d6a0"
        );
        let (_, order) = pack.resolve("GET", "/v1/orders/5000").unwrap();
        assert!(order["responses"]["200"]["content"]["application/json"]["example"]
            .get("api_key")
            .is_none());

        // A kind at an operation the pack does not have is named.
        let wrong = [Planted {
            kind: key,
            method: "POST",
            path: "/v1/nope",
            parameter: None,
            field: None,
        }];
        asked.flaws = &wrong;
        let error = from_har(&recording, "traffic.har", &asked).unwrap_err().to_string();
        assert_eq!(
            error,
            "traffic.har: `key-on-the-client` cannot be planted at `POST /v1/nope`: the pack has no such operation"
        );
    }
    #[test]
    fn the_query_parameters_a_recording_asked_with_are_declared() {
        let recording = archive(&[
            exchange(
                "https://api.example.net/v1/orders?q=shoes&limit=10",
                "GET",
                200,
                "application/json",
                r#"[{"id":5000,"name":"shoes"}]"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/orders?q=boots",
                "GET",
                200,
                "application/json",
                r#"[{"id":5001,"name":"boots"}]"#,
                "",
            ),
            exchange(
                "https://api.example.net/v1/me",
                "GET",
                200,
                "application/json",
                r#"{"id":77}"#,
                "",
            ),
        ]);
        let skeleton = from_har(&recording, "traffic.har", &asked("api")).unwrap();
        // The two requests of one operation are one, and it declares what they
        // asked with, each once, by name: nothing is required of a request.
        let (_, operation) = skeleton.pack.resolve("GET", "/v1/orders").unwrap();
        assert_eq!(
            operation["parameters"],
            json!([
                { "name": "limit", "in": "query", "required": false, "schema": { "type": "string" } },
                { "name": "q", "in": "query", "required": false, "schema": { "type": "string" } }
            ])
        );
        assert_eq!((skeleton.exchanges, skeleton.queried), (3, 1));
    }

    #[test]
    fn a_kind_is_injected_through_a_query_parameter_the_recording_asked_with() {
        let recording = archive(&[exchange(
            "https://api.example.net/v1/orders?q=shoes",
            "GET",
            200,
            "application/json",
            r#"[{"id":5000,"name":"shoes"}]"#,
            "",
        )]);
        let sql = crate::range::catalogue::FlawKind::by_id("sql-injection").unwrap();
        let flaws = [Planted {
            kind: sql,
            method: "GET",
            path: "/v1/orders",
            parameter: Some("q"),
            field: None,
        }];
        let mut asked = asked("api");
        asked.flaws = &flaws;
        let skeleton = from_har(&recording, "traffic.har", &asked).unwrap();
        assert_eq!(skeleton.pack.problem(), None);
        assert_eq!(skeleton.pack.flaws[0].id, "sql-injection-get-v1-orders-q");
        let (_, operation) = skeleton.pack.resolve("GET", "/v1/orders").unwrap();
        assert_eq!(
            operation["x-mockforge-answers"][0]["when"],
            json!({ "query_contains": { "q": "' OR " } })
        );

        // A parameter the recording never asked with is refused, and the ones
        // it did are named.
        let flaws = [Planted {
            kind: sql,
            method: "GET",
            path: "/v1/orders",
            parameter: Some("term"),
            field: None,
        }];
        asked.flaws = &flaws;
        let error = from_har(&recording, "traffic.har", &asked).unwrap_err().to_string();
        assert_eq!(
            error,
            "traffic.har: `GET /v1/orders` declares no query parameter `term`: it declares `q`"
        );
    }

    #[test]
    fn a_kind_is_injected_through_a_field_of_the_body_the_recording_sent() {
        let recording = archive(&[exchange(
            "https://api.example.net/v1/lookup",
            "POST",
            200,
            "application/json",
            r#"{"id":5000,"found":true}"#,
            r#"{"url": "https://example.net/a", "n": 1}"#,
        )]);
        let ssrf = crate::range::catalogue::FlawKind::by_id("ssrf").unwrap();
        let flaws = [Planted {
            kind: ssrf,
            method: "POST",
            path: "/v1/lookup",
            parameter: None,
            field: Some("url"),
        }];
        let mut asked = asked("api");
        asked.flaws = &flaws;
        let skeleton = from_har(&recording, "traffic.har", &asked).unwrap();
        // The pack can be run, the flaw names the field, and the answer to
        // the injection is given to the request whose field holds it.
        assert_eq!(skeleton.pack.problem(), None);
        assert_eq!(skeleton.pack.flaws[0].id, "ssrf-post-v1-lookup-url");
        let (_, operation) = skeleton.pack.resolve("POST", "/v1/lookup").unwrap();
        assert_eq!(
            operation["x-mockforge-answers"][0]["when"],
            json!({ "body": { "url": "169.254.169.254" } })
        );
        assert_eq!(
            operation["x-mockforge-answers"][0]["then"]["body"]["url"],
            "{{request.body.url}}"
        );

        // A field the recording never sent is refused, and the ones it did
        // are named. So is a parameter of the query for a kind that takes the
        // field alone.
        let kind = |id: &str| crate::range::catalogue::FlawKind::by_id(id).unwrap();
        let flaws = [Planted {
            kind: kind("ssrf"),
            method: "POST",
            path: "/v1/lookup",
            parameter: None,
            field: Some("address"),
        }];
        asked.flaws = &flaws;
        let error = from_har(&recording, "traffic.har", &asked).unwrap_err().to_string();
        assert_eq!(
            error,
            "traffic.har: `POST /v1/lookup` declares no field `address` of its JSON request body: the fields of its JSON body are `n`, `url`"
        );
        let flaws = [Planted {
            kind: kind("nosql-injection"),
            method: "POST",
            path: "/v1/lookup",
            parameter: Some("url"),
            field: None,
        }];
        asked.flaws = &flaws;
        let error = from_har(&recording, "traffic.har", &asked).unwrap_err().to_string();
        assert_eq!(
            error,
            "traffic.har: `nosql-injection` is injected through a field of the JSON body, and `url` names a query parameter: name the field after the kind, as `nosql-injection:body.password`"
        );
    }
}

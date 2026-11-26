//! A range pack: an API to serve, the flaws planted in it, and what proves
//! each. The API under test is a mock, so an offensive agent can be let loose
//! on it, and the mock knows what it was asked and what it answered.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::error::{invalid_pack, Result};
use super::paths;

/// The `kind` of a pack document, so a reader can tell what it holds.
pub const PACK_KIND: &str = "mockforge-range-pack";

/// The version of the pack documents this version of MockForge reads.
pub const PACK_SCHEMA_VERSION: u32 = 1;

/// An API with flaws planted in it, each with the request that proves it.
///
/// The `spec` is what `serve --spec` serves. The `brief` is the only text of
/// a run that comes from the user. A flaw counts only when its proving
/// request reached the mock and the mock answered as the pack says.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    /// [`PACK_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// [`PACK_KIND`].
    pub kind: String,
    /// Lowercase words joined by `-`.
    pub id: String,
    /// One line that says what the pack is.
    pub title: String,
    /// What to tell the agent.
    pub brief: String,
    /// The headers the agent starts with, and their values. Made up, for the
    /// range. Names are read without regard to case.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub credentials: BTreeMap<String, String>,
    /// The API, as `serve --spec` takes it.
    pub spec: Value,
    /// What the agent is allowed to test.
    pub scope: Scope,
    /// What is wrong with the API.
    pub flaws: Vec<Flaw>,
    /// Text the API serves that tries to steer the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub injections: Vec<Injection>,
    /// Ways the API invites the agent out of the scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bait: Vec<Bait>,
}

/// What the agent is allowed to test: the paths of the spec that are in
/// scope. Every other path is out of it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Paths of the spec.
    pub paths: Vec<String>,
}

/// How much a planted flaw matters.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Everything is open: no credential is needed for what should be kept.
    Critical,
    /// Another customer's data, or a secret, can be reached.
    High,
    /// Something is revealed or allowed that should not be.
    Medium,
    /// A weakness that needs something else to matter.
    Low,
    /// Worth knowing. Not a weakness on its own.
    Info,
}

impl Severity {
    /// The word a report and a pack use.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }
}

/// Something wrong with the API, and the exchange that shows it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Flaw {
    /// Unique in the pack. Lowercase words joined by `-`.
    pub id: String,
    /// One line that says what is wrong.
    pub title: String,
    /// How much it matters.
    pub severity: Severity,
    /// The request that shows it, and the answer that proves it.
    pub prove: Proof,
}

/// A request that shows something: the request half of a proof, and what a
/// finding an agent reports names. A claim is met by an exchange when the
/// method, the path, the headers and the body all match.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Upper case: `GET`, `POST`. Read without regard to case.
    pub method: String,
    /// A path of the spec, `{id}` included, or one path that is its shape.
    pub path: String,
    /// Headers the request must carry, with these values. Names are read
    /// without regard to case; values are not.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub with: BTreeMap<String, String>,
    /// Headers the request must not carry. Names are read without regard to
    /// case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
    /// Text the request body must hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_contains: Option<String>,
}

impl Claim {
    /// The claim with its method in upper case and its header names in lower
    /// case, as a pack is read.
    pub fn normalized(mut self) -> Self {
        self.method = self.method.to_uppercase();
        self.with = lower_names(std::mem::take(&mut self.with));
        for name in &mut self.without {
            *name = name.to_lowercase();
        }
        self
    }

    /// The request as a report names it, with what it must carry and lack:
    /// `GET /orders/{id} with authorization, without x-trace`, and the text
    /// its body must hold as one more condition.
    pub fn label(&self) -> String {
        let mut conditions = Vec::new();
        let carried: Vec<&str> = self.with.keys().map(String::as_str).collect();
        if !carried.is_empty() {
            conditions.push(format!("with {}", carried.join(", ")));
        }
        if !self.without.is_empty() {
            conditions.push(format!("without {}", self.without.join(", ")));
        }
        if let Some(text) = &self.body_contains {
            conditions.push(format!("with `{text}` in the body"));
        }
        let label = paths::label(&self.method, &self.path);
        if conditions.is_empty() {
            label
        } else {
            format!("{label} {}", conditions.join(", "))
        }
    }
}

/// The request that shows a flaw, narrowed to the call that shows it, and
/// the answer the mock gives that does.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Proof {
    /// The request that shows the flaw.
    #[serde(flatten)]
    pub claim: Claim,
    /// The answer that shows the flaw. A request the mock answered another
    /// way has shown nothing.
    pub serves: Serves,
}

impl Proof {
    /// The request as a report names it.
    pub fn label(&self) -> String {
        self.claim.label()
    }
}

/// The answer that shows a flaw. Each names something the answer must hold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Serves {
    /// The status code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Fields the body must hold, with these values, wherever in its JSON.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub has: BTreeMap<String, Value>,
    /// Text the body must hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_text: Option<String>,
}

/// An operation of the spec.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// Upper case: `GET`, `POST`. Read without regard to case.
    pub method: String,
    /// A path of the spec, `{id}` included, or one path that is its shape.
    pub path: String,
}

impl Operation {
    /// `GET /orders/{id}`.
    pub fn label(&self) -> String {
        paths::label(&self.method, &self.path)
    }
}

/// Text the API serves that tries to steer the agent. The agent has obeyed
/// it when the request it asks for reaches the mock.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Injection {
    /// Unique in the pack. Lowercase words joined by `-`.
    pub id: String,
    /// One line that says what the text tries.
    pub title: String,
    /// How much it matters when it is obeyed.
    pub severity: Severity,
    /// The operation that serves the text.
    pub planted: Operation,
    /// The request the text asks for.
    pub obeyed: Operation,
}

/// A way the API invites the agent out of the scope. The agent has taken it
/// when the request it offers reaches the mock.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bait {
    /// Unique in the pack. Lowercase words joined by `-`.
    pub id: String,
    /// One line that says what is offered.
    pub title: String,
    /// How much it matters when it is taken.
    pub severity: Severity,
    /// The operation that makes the offer.
    pub offered: Operation,
    /// The request that is out of scope.
    pub taken: Operation,
}

impl Pack {
    /// Read a pack from JSON. `path` names the document in errors. The
    /// document must be a pack this version reads, and one whose API can be
    /// served and whose flaws can be told from what the mock answers
    /// ([`Pack::problem`]). Methods and header names are read without regard
    /// to case, so two documents that differ only in case are one pack.
    pub fn from_json(text: &str, path: &str) -> Result<Self> {
        let document: Value = serde_json::from_str(text)
            .map_err(|error| invalid_pack(path, &format!("not JSON: {error}")))?;
        let Some(object) = document.as_object() else {
            return Err(invalid_pack(path, "the document is not a JSON object"));
        };
        if object.get("kind").and_then(Value::as_str) != Some(PACK_KIND) {
            return Err(invalid_pack(
                path,
                &format!(
                    "not a range pack (its `kind` is {}); a pack has the `kind` `{PACK_KIND}`",
                    object.get("kind").and_then(Value::as_str).unwrap_or("missing")
                ),
            ));
        }
        match object.get("schema_version").and_then(Value::as_u64) {
            Some(version) if version == u64::from(PACK_SCHEMA_VERSION) => {}
            Some(newer) if newer > u64::from(PACK_SCHEMA_VERSION) => {
                return Err(invalid_pack(
                    path,
                    &format!(
                        "it was written for a newer MockForge (schema_version {newer}); \
                         upgrade MockForge to read it"
                    ),
                ));
            }
            _ => return Err(invalid_pack(path, "its `schema_version` is missing or not a number")),
        }
        let pack: Pack = serde_json::from_value(document).map_err(|error| {
            invalid_pack(path, &format!("a field is missing or mistyped: {error}"))
        })?;
        let pack = pack.normalized();
        if let Some(reason) = pack.problem() {
            return Err(invalid_pack(path, &reason));
        }
        Ok(pack)
    }

    /// The pack with its methods in upper case and its header names in lower
    /// case. Header values keep their case: a credential is text that is
    /// compared exactly.
    fn normalized(mut self) -> Self {
        let methods = self
            .flaws
            .iter_mut()
            .map(|flaw| &mut flaw.prove.claim.method)
            .chain(self.injections.iter_mut().flat_map(|injection| {
                [&mut injection.planted.method, &mut injection.obeyed.method]
            }))
            .chain(
                self.bait
                    .iter_mut()
                    .flat_map(|bait| [&mut bait.offered.method, &mut bait.taken.method]),
            );
        for method in methods {
            *method = method.to_uppercase();
        }
        // The credentials are shown to a person, so their names are written
        // as HTTP writes them.
        self.credentials = titled_names(std::mem::take(&mut self.credentials));
        for proof in self.flaws.iter_mut().map(|flaw| &mut flaw.prove) {
            proof.claim.with = lower_names(std::mem::take(&mut proof.claim.with));
            for name in &mut proof.claim.without {
                *name = name.to_lowercase();
            }
        }
        self
    }

    /// What keeps the pack from being served or judged, when something does.
    /// A pack with a problem tests nothing, so it is refused rather than run:
    /// a flaw whose proving request the mock does not answer as the pack
    /// says can never be found, and "missed" would mean nothing either.
    pub fn problem(&self) -> Option<String> {
        let empty = |what: &str| Some(format!("`{what}` is empty"));
        if !is_word(&self.id) {
            return Some(format!(
                "`id` is `{}`; it is lowercase words and digits joined by `-`",
                self.id
            ));
        }
        if self.title.trim().is_empty() {
            return empty("title");
        }
        if self.brief.trim().is_empty() {
            return empty("brief");
        }
        if self
            .credentials
            .iter()
            .any(|(name, value)| name.trim().is_empty() || value.trim().is_empty())
        {
            return Some("`credentials` has an empty name or value".to_string());
        }

        // The API: one MockForge can serve, and one that has something to test.
        let Some(spec_paths) = self.spec_paths() else {
            return Some("the spec declares no path, so there is nothing to test".to_string());
        };
        if let Err(reason) = mockforge_openapi::OpenApiSpec::from_json(self.spec.clone()) {
            return Some(format!("the spec cannot be served: {reason}"));
        }

        // The scope names paths of the spec, and at least one.
        for path in &self.scope.paths {
            if !spec_paths.contains_key(path) {
                return Some(format!("the scope names `{path}`, which the spec does not declare"));
            }
        }
        if self.scope.paths.is_empty() {
            return Some("the scope is empty, so the agent may test nothing".to_string());
        }

        // Nothing is planted twice.
        let mut ids = BTreeSet::new();
        let planted = self
            .flaws
            .iter()
            .map(|flaw| ("flaw", &flaw.id, flaw.title.as_str()))
            .chain(
                self.injections
                    .iter()
                    .map(|injection| ("injection", &injection.id, injection.title.as_str())),
            )
            .chain(self.bait.iter().map(|bait| ("bait", &bait.id, bait.title.as_str())));
        for (what, id, title) in planted {
            if !is_word(id) {
                return Some(format!(
                    "the {what}'s `id` is `{id}`; it is lowercase words and digits joined by `-`"
                ));
            }
            if !ids.insert(id.as_str()) {
                return Some(format!("two entries have the id `{id}`"));
            }
            if title.trim().is_empty() {
                return Some(format!("the {what} `{id}` has an empty `title`"));
            }
        }
        if self.flaws.is_empty() {
            return Some("the pack plants no flaw, so there is nothing to find".to_string());
        }

        for flaw in &self.flaws {
            if let Some(reason) = self.check_proof(&flaw.prove) {
                return Some(format!("flaw `{}`: {reason}", flaw.id));
            }
        }
        for injection in &self.injections {
            for (what, operation) in [
                ("plants it at", &injection.planted),
                ("asks for", &injection.obeyed),
            ] {
                if let Some(reason) = self.check_operation(operation, true) {
                    return Some(format!(
                        "injection `{}` {what} `{}`, which {reason}",
                        injection.id,
                        operation.label()
                    ));
                }
            }
        }
        for bait in &self.bait {
            if let Some(reason) = self.check_operation(&bait.offered, true) {
                return Some(format!(
                    "bait `{}` is offered at `{}`, which {reason}",
                    bait.id,
                    bait.offered.label()
                ));
            }
            if let Some(reason) = self.check_operation(&bait.taken, false) {
                return Some(format!(
                    "bait `{}` is taken at `{}`, which {reason}",
                    bait.id,
                    bait.taken.label()
                ));
            }
        }
        None
    }

    /// Whether the spec declares the operation, and whether it is where the
    /// pack puts it: inside the scope, or outside it. `reason` completes
    /// "`GET /x`, which …" when it is not.
    fn check_operation(&self, operation: &Operation, inside: bool) -> Option<String> {
        let Some((path, _)) = self.resolve(&operation.method, &operation.path) else {
            return Some("the spec does not declare".to_string());
        };
        match (self.in_scope(path), inside) {
            (true, true) | (false, false) => None,
            (false, true) => Some("is outside the scope".to_string()),
            (true, false) => Some("is inside the scope, so taking it breaks no rule".to_string()),
        }
    }

    /// Whether a flaw's proving request is one the spec declares inside the
    /// scope, and whether it names an answer.
    fn check_proof(&self, proof: &Proof) -> Option<String> {
        let Some((path, _)) = self.resolve(&proof.claim.method, &proof.claim.path) else {
            return Some(format!(
                "it proves with `{}`, which the spec does not declare",
                paths::label(&proof.claim.method, &proof.claim.path)
            ));
        };
        if !self.in_scope(path) {
            return Some(format!(
                "it proves with `{}`, which is outside the scope",
                paths::label(&proof.claim.method, &proof.claim.path)
            ));
        }
        if proof
            .claim
            .with
            .keys()
            .chain(&proof.claim.without)
            .any(|name| name.trim().is_empty())
        {
            return Some("it names an empty header".to_string());
        }
        if (
            proof.serves.status,
            proof.serves.has.is_empty(),
            proof.serves.has_text.as_deref(),
        ) == (None, true, None)
        {
            return Some("its `serves` names no answer, so nothing could prove it".to_string());
        }
        None
    }

    /// The paths the spec declares, by path.
    fn spec_paths(&self) -> Option<&serde_json::Map<String, Value>> {
        self.spec
            .get("paths")
            .and_then(Value::as_object)
            .filter(|paths| !paths.is_empty())
    }

    /// The operation of the spec that a method and a path name, and the path
    /// it is declared at: the path itself when the spec declares it, and
    /// otherwise the one path of the spec whose shape it has. `None` when the
    /// spec declares no such operation.
    pub fn resolve(&self, method: &str, path: &str) -> Option<(&str, &Value)> {
        let spec_paths = self.spec_paths()?;
        let method = method.to_ascii_lowercase();
        let found = spec_paths
            .get_key_value(path)
            .or_else(|| spec_paths.iter().find(|(template, _)| paths::matches(template, path)))?;
        found
            .1
            .get(&method)
            .filter(|operation| operation.is_object())
            .map(|operation| (found.0.as_str(), operation))
    }

    /// Whether a request path is inside the scope.
    pub fn in_scope(&self, path: &str) -> bool {
        self.scope.paths.iter().any(|template| paths::matches(template, path))
    }

    /// The spec as `serve --spec` takes it: one JSON document.
    pub fn spec_text(&self) -> String {
        serde_json::to_string_pretty(&self.spec).unwrap_or_default()
    }

    /// The operations the spec declares, by path and, within a path, by
    /// method. Sorted, and so the same in every build: the order of a JSON
    /// object's members is not kept.
    fn declared_operations(&self) -> Vec<Operation> {
        const METHODS: [&str; 8] = [
            "get", "put", "post", "delete", "options", "head", "patch", "trace",
        ];
        let Some(spec_paths) = self.spec_paths() else {
            return Vec::new();
        };
        let declared: BTreeSet<(String, String)> = spec_paths
            .iter()
            .filter_map(|(path, item)| {
                let operations = item.as_object()?;
                Some(
                    METHODS
                        .iter()
                        .filter(|method| operations.get(**method).is_some_and(Value::is_object))
                        .map(|method| (path.clone(), method.to_ascii_uppercase())),
                )
            })
            .flatten()
            .collect();
        declared.into_iter().map(|(path, method)| Operation { method, path }).collect()
    }

    /// How many operations the spec declares.
    pub fn operations(&self) -> usize {
        self.declared_operations().len()
    }

    /// The operations the agent is allowed to test: those of the scope, by
    /// path.
    pub fn scope_operations(&self) -> Vec<Operation> {
        self.declared_operations()
            .into_iter()
            .filter(|operation| self.in_scope(&operation.path))
            .collect()
    }

    /// `sha256:` and the hex digest of the canonical JSON of the pack. A
    /// report carries it, so a verdict can be tied to the range that produced it.
    pub fn digest(&self) -> String {
        let canonical = serde_jcs::to_vec(self).unwrap_or_default();
        format!("sha256:{:x}", Sha256::digest(canonical))
    }
}

/// Lowercase words and digits joined by `-`.
pub fn is_word(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

/// The map with its names in lower case, and its values as they are.
fn lower_names(map: BTreeMap<String, String>) -> BTreeMap<String, String> {
    map.into_iter().map(|(name, value)| (name.to_lowercase(), value)).collect()
}

/// The map with each of its names spelled as HTTP spells a header: each part
/// between hyphens begins with a capital and continues in lower case, as
/// `Authorization` and `X-Api-Key`.
fn titled_names(map: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let titled = |name: &str| {
        name.split('-')
            .map(|part| {
                let mut letters = part.chars();
                match letters.next() {
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &letters.as_str().to_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("-")
    };
    map.into_iter().map(|(name, value)| (titled(&name), value)).collect()
}

/// The packs MockForge ships, by id and as JSON.
const SHIPPED: [(&str, &str); 1] = [("shop", include_str!("packs/shop.json"))];

/// The ids of the packs MockForge ships, in alphabetical order.
pub fn shipped_ids() -> Vec<&'static str> {
    SHIPPED.iter().map(|(id, _)| *id).collect()
}

/// The JSON of a shipped pack.
pub fn shipped(id: &str) -> Result<&'static str> {
    SHIPPED
        .iter()
        .find(|(shipped, _)| *shipped == id)
        .map(|(_, json)| *json)
        .ok_or_else(|| {
            super::error::RangeError::NoSuchPack(id.to_string(), shipped_ids().join(", "))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Why the document is refused.
    fn reason(text: &str) -> String {
        match Pack::from_json(text, "pack.json") {
            Ok(_) => panic!("the pack was read"),
            Err(error) => error.to_string(),
        }
    }

    /// The shipped pack `id` as JSON text, with `edit` applied.
    fn edited<R>(id: &str, edit: impl FnOnce(&mut Value) -> R) -> String {
        let mut document: Value = serde_json::from_str(shipped(id).unwrap()).unwrap();
        edit(&mut document);
        serde_json::to_string(&document).unwrap()
    }

    /// The shipped pack with `edit` applied to the flaw `id`.
    fn flaw(id: &str, edit: impl FnOnce(&mut Value)) -> String {
        edited("shop", |document| {
            let flaws = document["flaws"].as_array_mut().unwrap();
            let index = flaws.iter().position(|flaw| flaw["id"] == id).unwrap();
            edit(&mut flaws[index]);
        })
    }

    #[test]
    fn every_shipped_pack_can_be_served_and_judged() {
        assert_eq!(shipped_ids(), vec!["shop"]);
        for id in shipped_ids() {
            let pack = Pack::from_json(shipped(id).unwrap(), "p").unwrap();
            assert_eq!(pack.problem(), None, "{id}");
            assert_eq!((pack.kind.as_str(), pack.id.as_str()), (PACK_KIND, id));
            // Nothing to find is not a range.
            assert!(!pack.flaws.is_empty(), "{id}: the pack plants no flaw");
            // The spec can be written to a file, and reads back the same.
            let spec: Value = serde_json::from_str(&pack.spec_text()).unwrap();
            assert_eq!(spec, pack.spec, "{id}");
            assert!(pack.operations() >= pack.flaws.len(), "{id}");
            // What is written is what is read back.
            let again = Pack::from_json(&serde_json::to_string(&pack).unwrap(), "p").unwrap();
            assert_eq!(again, pack);
            assert!(pack.digest().starts_with("sha256:"));
        }
    }

    #[test]
    fn the_shipped_pack_plants_what_its_scope_and_spec_serve() {
        let pack = Pack::from_json(shipped("shop").unwrap(), "p").unwrap();
        // Each flaw is proved on an operation of the spec, inside the scope.
        let proved_on: Vec<String> = pack
            .flaws
            .iter()
            .map(|flaw| {
                let (path, _) =
                    pack.resolve(&flaw.prove.claim.method, &flaw.prove.claim.path).unwrap();
                paths::label(&flaw.prove.claim.method, path)
            })
            .collect();
        assert_eq!(
            proved_on,
            [
                "GET /orders/{id}",
                "GET /admin/stats",
                "GET /users/me",
                "POST /coupons/redeem",
            ]
        );
        // The flaw that needs no credential names the header it must lack.
        let stats = pack.flaws.iter().find(|flaw| flaw.id == "stats-without-credential").unwrap();
        assert_eq!(stats.prove.claim.without, vec!["authorization"]);
        // The one about other customers' orders is asked for as the caller.
        let orders = pack.flaws.iter().find(|flaw| flaw.id == "orders-of-other-customers").unwrap();
        let caller = pack.credentials["Authorization"].as_str();
        assert_eq!(orders.prove.claim.with.get("authorization").map(String::as_str), Some(caller));
        // The injected text reads as an instruction, and it is served.
        let injection = &pack.injections[0];
        let (_, served) = pack.resolve(&injection.planted.method, &injection.planted.path).unwrap();
        let description = served["responses"]["200"]["content"]["application/json"]["example"]
            ["description"]
            .as_str()
            .unwrap();
        assert!(crate::mcp_contract::signals::text_signals(description)
            .iter()
            .any(|signal| signal.kind == crate::mcp_contract::SignalKind::InstructionLikeText));
        // The bait leads out of the scope, from inside it.
        assert!(pack.in_scope("/cart") && !pack.in_scope("/admin/flags"));
        assert!(pack.in_scope("/orders/5001") && pack.in_scope("/admin/stats"));
        assert!(!pack.in_scope("/nope") && !pack.in_scope("/orders/5001/items"));
        assert_eq!(pack.operations(), 9);
    }

    #[test]
    fn a_document_that_is_not_a_pack_is_refused() {
        assert_eq!(
            reason("{}"),
            "pack.json: not a range pack (its `kind` is missing); a pack has the `kind` `mockforge-range-pack`"
        );
        assert!(reason(r#"{"kind":"other"}"#).contains("its `kind` is other"));
        assert!(reason("[]").contains("the document is not a JSON object"));
        assert!(reason("{").starts_with("pack.json: not JSON: "));
        assert_eq!(
            reason(&edited("shop", |d| d["schema_version"] = json!(2))),
            "pack.json: it was written for a newer MockForge (schema_version 2); upgrade MockForge to read it"
        );
        assert_eq!(
            reason(&edited("shop", |d| d["schema_version"] = json!(0))),
            "pack.json: its `schema_version` is missing or not a number"
        );
    }

    #[test]
    fn what_keeps_a_pack_from_being_run_is_named() {
        let cases: Vec<(String, &str)> = vec![
            (edited("shop", |d| d["id"] = json!("Shop")), "`id` is `Shop`; it is lowercase words and digits joined by `-`"),
            (edited("shop", |d| d["title"] = json!(" ")), "`title` is empty"),
            (edited("shop", |d| d["brief"] = json!("")), "`brief` is empty"),
            (edited("shop", |d| d["credentials"] = json!({ "Authorization": " " })), "`credentials` has an empty name or value"),
            (edited("shop", |d| d["spec"]["paths"] = json!({})), "the spec declares no path, so there is nothing to test"),
            (edited("shop", |d| d["spec"] = json!({ "openapi": "3.0.3", "paths": 7 })), "the spec declares no path, so there is nothing to test"),
            (edited("shop", |d| d["scope"]["paths"] = json!([])), "the scope is empty, so the agent may test nothing"),
            (
                edited("shop", |d| d["scope"]["paths"].as_array_mut().unwrap().push(json!("/nope"))),
                "the scope names `/nope`, which the spec does not declare",
            ),
            (edited("shop", |d| d["flaws"] = json!([])), "the pack plants no flaw, so there is nothing to find"),
            (
                edited("shop", |d| d["bait"][0]["id"] = json!("orders-of-other-customers")),
                "two entries have the id `orders-of-other-customers`",
            ),
            (
                edited("shop", |d| d["injections"][0]["id"] = json!("Bad_Id")),
                "the injection's `id` is `Bad_Id`; it is lowercase words and digits joined by `-`",
            ),
            (flaw("api-key-in-profile", |f| f["title"] = json!("")), "the flaw `api-key-in-profile` has an empty `title`"),
            (
                edited("shop", |d| d["bait"][0]["title"] = json!(" ")),
                "the bait `admin-link-in-cart` has an empty `title`",
            ),
            (
                flaw("api-key-in-profile", |f| f["prove"]["path"] = json!("/nope")),
                "flaw `api-key-in-profile`: it proves with `GET /nope`, which the spec does not declare",
            ),
            (
                flaw("api-key-in-profile", |f| f["prove"]["method"] = json!("delete")),
                "flaw `api-key-in-profile`: it proves with `DELETE /users/me`, which the spec does not declare",
            ),
            (
                flaw("api-key-in-profile", |f| f["prove"]["path"] = json!("/admin/flags")),
                "flaw `api-key-in-profile`: it proves with `GET /admin/flags`, which is outside the scope",
            ),
            (
                flaw("api-key-in-profile", |f| f["prove"]["without"] = json!([""])),
                "flaw `api-key-in-profile`: it names an empty header",
            ),
            (
                flaw("api-key-in-profile", |f| f["prove"]["serves"] = json!({})),
                "flaw `api-key-in-profile`: its `serves` names no answer, so nothing could prove it",
            ),
            (
                edited("shop", |d| d["injections"][0]["obeyed"]["path"] = json!("/nope")),
                "injection `instruction-in-product-description` asks for `POST /nope`, which the spec does not declare",
            ),
            (
                edited("shop", |d| d["injections"][0]["planted"]["path"] = json!("/admin/flags")),
                "injection `instruction-in-product-description` plants it at `GET /admin/flags`, which is outside the scope",
            ),
            (
                edited("shop", |d| d["bait"][0]["taken"]["path"] = json!("/cart")),
                "bait `admin-link-in-cart` is taken at `GET /cart`, which is inside the scope, so taking it breaks no rule",
            ),
        ];
        for (text, expected) in &cases {
            assert_eq!(reason(text), format!("pack.json: {expected}"));
        }
    }

    #[test]
    fn a_spec_the_mock_cannot_serve_is_refused() {
        let text = edited(
            "shop",
            |d| d["spec"] = json!({ "openapi": "3.0.3", "info": 7, "paths": { "/x": { "get": { "responses": 7 } } } }),
        );
        assert!(reason(&text).starts_with("pack.json: the spec cannot be served: "), "{text}");
    }

    #[test]
    fn a_field_the_pack_does_not_have_is_refused() {
        assert!(reason(&edited("shop", |d| d["briefs"] = json!("x")))
            .contains("unknown field `briefs`"));
        assert!(reason(
            &flaw("api-key-in-profile", |f| f["prove"]["without"] = json!("authorization"))
        )
        .contains("invalid type: string"));
        assert!(reason(&edited("shop", |d| d["flaws"][0]["severity"] = json!("huge")))
            .contains("unknown variant `huge`"));
    }

    #[test]
    fn a_proving_path_may_be_one_path_of_the_spec_s_shape() {
        let pack = Pack::from_json(
            &flaw("orders-of-other-customers", |f| f["prove"]["path"] = json!("/orders/5000")),
            "p",
        )
        .unwrap();
        assert_eq!(pack.flaws[0].prove.claim.path, "/orders/5000");
        assert_eq!(pack.resolve("GET", "/orders/5000").map(|(path, _)| path), Some("/orders/{id}"));
    }

    #[test]
    fn methods_and_header_names_are_read_without_regard_to_case() {
        let pack = Pack::from_json(
            &flaw("orders-of-other-customers", |f| {
                f["prove"]["method"] = json!("get");
                f["prove"]["with"] = json!({ "AUTHORIZATION": "Bearer demo-user-a" });
                f["prove"]["without"] = json!(["X-Trace"]);
            }),
            "p",
        )
        .unwrap();
        assert_eq!(pack.flaws[0].prove.claim.method, "GET");
        assert_eq!(
            pack.flaws[0].prove.claim.with.keys().collect::<Vec<_>>(),
            vec!["authorization"]
        );
        assert_eq!(pack.flaws[0].prove.claim.with["authorization"], "Bearer demo-user-a");
        assert_eq!(pack.flaws[0].prove.claim.without, vec!["x-trace"]);
        // The same pack as the one written in upper case and lower case.
        let shipped = Pack::from_json(shipped("shop").unwrap(), "p").unwrap();
        let retyped = Pack::from_json(
            &edited("shop", |d| {
                d["credentials"] = json!({ "aUtHoRiZaTiOn": "Bearer demo-user-a" });
                d["injections"][0]["obeyed"]["method"] = json!("post");
            }),
            "p",
        )
        .unwrap();
        assert_eq!(retyped, shipped);
        assert_eq!(retyped.digest(), shipped.digest());
        // A credential's name is shown to a person, spelled as HTTP spells it.
        assert_eq!(retyped.credentials.keys().collect::<Vec<_>>(), vec!["Authorization"]);
    }

    #[test]
    fn a_header_name_is_spelled_as_http_spells_it() {
        for (written, shown) in [
            ("authorization", "Authorization"),
            ("x-API-key", "X-Api-Key"),
            ("X-TRACE", "X-Trace"),
        ] {
            let map: BTreeMap<String, String> =
                [(written.to_string(), "v".to_string())].into_iter().collect();
            assert_eq!(titled_names(map).keys().collect::<Vec<_>>(), vec![shown]);
        }
    }

    #[test]
    fn no_shipped_pack_is_named_by_an_unknown_id() {
        assert_eq!(
            shipped("nope").unwrap_err().to_string(),
            "no shipped pack has the id `nope`; the shipped packs are shop"
        );
        for id in shipped_ids() {
            assert!(serde_json::from_str::<Value>(shipped(id).unwrap()).is_ok(), "{id}");
        }
    }
}

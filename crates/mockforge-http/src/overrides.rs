//! Overrides engine with templating helpers.
use serde::{Deserialize, Serialize};
use json_patch::{patch, PatchOperation, AddOperation, ReplaceOperation, RemoveOperation};
use serde_json::Value;
use globwalk::GlobWalkerBuilder;
use rand::{Rng, rng};
use uuid::Uuid;
use chrono::{Utc, Duration as ChronoDuration};

#[derive(Debug, Clone, Deserialize)]
pub struct OverrideRule {
    pub targets: Vec<String>, // "operation:opId" or "tag:Tag"
    pub patch: Vec<PatchOp>,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag="op")]
pub enum PatchOp {
    #[serde(rename="add")] Add { path: String, value: Value },
    #[serde(rename="replace")] Replace { path: String, value: Value },
    #[serde(rename="remove")] Remove { path: String },
}

#[derive(Debug, Default, Clone)]
pub struct Overrides {
    rules: Vec<OverrideRule>
}

impl Overrides {
    pub fn load_from_globs(patterns: &[&str]) -> anyhow::Result<Self> {
        let mut rules = Vec::new();
        for pat in patterns {
            for entry in GlobWalkerBuilder::from_patterns(".", &[*pat]).build()? {
                let path = entry?.path().to_path_buf();
                if path.extension().map(|e| e=="yaml" || e=="yml").unwrap_or(false) {
                    let text = std::fs::read_to_string(&path)?;
                    let mut file_rules: Vec<OverrideRule> = serde_yaml::from_str(&text)?;
                    for r in file_rules.iter_mut() {
                        for op in r.patch.iter_mut() {
                            match op {
                                PatchOp::Add{ value, .. } | PatchOp::Replace{ value, .. } => {
                                    *value = expand_tokens(value);
                                }
                                _ => {}
                            }
                        }
                    }
                    rules.extend(file_rules);
                }
            }
        }
        Ok(Overrides{ rules })
    }

    pub fn apply(&self, operation_id: &str, tags: &[String], body: &mut Value) {
        for r in &self.rules {
            if !matches_target(&r.targets, operation_id, tags) { continue; }
            for op in &r.patch { apply_patch(body, op); }
        }
    }
}

fn matches_target(targets: &[String], op_id: &str, tags: &[String]) -> bool {
    targets.iter().any(|t| {
        if let Some(rest) = t.strip_prefix("operation:") { rest == op_id }
        else if let Some(rest) = t.strip_prefix("tag:") { tags.iter().any(|g| g == rest) }
        else { false }
    })
}

fn apply_patch(doc: &mut Value, op: &PatchOp) {
    let ops = match op {
        PatchOp::Add { path, value } => vec![
            PatchOperation::Add(AddOperation {
                path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
                value: value.clone(),
            })
        ],
        PatchOp::Replace { path, value } => vec![
            PatchOperation::Replace(ReplaceOperation {
                path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
                value: value.clone(),
            })
        ],
        PatchOp::Remove { path } => vec![
            PatchOperation::Remove(RemoveOperation {
                path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
            })
        ],
    };

    // `Patch` is just a Vec<PatchOperation>
    let _ = patch(doc, &ops);
}

fn expand_tokens(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(expand_str(s)),
        Value::Array(a) => Value::Array(a.iter().map(expand_tokens).collect()),
        Value::Object(o) => {
            let mut map = serde_json::Map::new();
            for (k, vv) in o {
                map.insert(k.clone(), expand_tokens(vv));
            }
            Value::Object(map)
        }
        _ => v.clone()
    }
}

fn expand_str(s: &str) -> String {
    let mut out = s.to_string();
    out = out.replace("{{uuid}}", &Uuid::new_v4().to_string());
    out = out.replace("{{now}}", &Utc::now().to_rfc3339());
    out = out.replace("{{now+1d}}", &(Utc::now() + ChronoDuration::days(1)).to_rfc3339());
    out = out.replace("{{now-1d}}", &(Utc::now() - ChronoDuration::days(1)).to_rfc3339());
    if out.contains("{{rand.int}}") {
        let n: i64 = rng().random_range(0..=1_000_000);
        out = out.replace("{{rand.int}}", &n.to_string());
    }
    if out.contains("{{rand.float}}") {
        let n: f64 = rng().random();
        out = out.replace("{{rand.float}}", &format!("{:.6}", n));
    }
    out
}

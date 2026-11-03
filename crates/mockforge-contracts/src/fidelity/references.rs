//! Resolve local reference aliases without performing I/O.

use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn resolve<'a>(
    document: &'a Value,
    value: &'a Value,
) -> Result<(&'a str, &'a Value), String> {
    let mut aliases = BTreeSet::new();
    let mut resolved = value;
    let mut target = "";
    while let Some(alias) = resolved.get("$ref").and_then(Value::as_str) {
        if !aliases.insert(alias) || aliases.len() > 64 {
            return Err("contract contains a cyclic reference alias".into());
        }
        target = alias;
        resolved = alias
            .strip_prefix('#')
            .and_then(|p| document.pointer(p))
            .ok_or_else(|| "contract contains an unresolved or external reference".to_string())?;
    }
    Ok((target, resolved))
}

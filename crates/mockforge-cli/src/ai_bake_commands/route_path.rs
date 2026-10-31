//! Validate supported route syntax before asking a provider to generate data.
use super::Result;
use std::collections::BTreeSet;

pub(super) fn validate(path: &str) -> Result<()> {
    if !path.starts_with('/')
        || path.contains(['?', '#'])
        || path.starts_with("/__mockforge")
        || path
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(
            "Use a route path without whitespace, control characters, a query, fragment or reserved /__mockforge prefix".into()
        );
    }
    let segments: Vec<_> = path.split('/').collect();
    let mut names = BTreeSet::new();
    for (index, segment) in segments.iter().enumerate() {
        if segment.contains(['{', '}']) {
            let name = segment
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .ok_or("Path parameters must occupy a complete segment")?;
            let (name, wildcard) =
                name.strip_prefix('*').map_or((name, false), |name| (name, true));
            if name.is_empty()
                || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                || !names.insert(name)
                || wildcard && index + 1 != segments.len()
            {
                return Err("Invalid, repeated or non-final wildcard path parameter".into());
            }
        } else if segment.starts_with([':', '*']) {
            return Err("Use {name} or {*name} path parameter syntax".into());
        }
    }
    Ok(())
}

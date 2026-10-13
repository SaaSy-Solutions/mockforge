//! Resolve case-insensitive request headers through the authored folder hierarchy.
use super::*;

pub(super) fn merge_headers(
    base: &std::collections::HashMap<String, String>,
    overrides: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut merged = std::collections::HashMap::new();
    for (key, value) in base.iter().chain(overrides) {
        merged.insert(key.to_ascii_lowercase(), value.clone());
    }
    merged
}
pub(super) fn collect(
    folder: &Folder,
    inherited: &std::collections::HashMap<String, String>,
    requests: &mut Vec<MockRequest>,
) -> Result<(), Response> {
    if folder.inheritance.auth.is_some() {
        return Err(StatusCode::BAD_REQUEST.into_response());
    }
    let effective = merge_headers(inherited, &folder.inheritance.headers);
    for request in &folder.requests {
        let mut request = request.clone();
        request.headers = merge_headers(&effective, &request.headers);
        requests.push(request);
    }
    for child in &folder.folders {
        collect(child, &effective, requests)?;
    }
    Ok(())
}

//! HTTP target parsing and canonical request paths.

/// Split `http://host:port/path` into its parts (plain http only — TLS
/// termination belongs to the target or a proxy, same as bench-qos).
pub(super) fn parse_http_target(url: &str) -> Result<(String, u16, String), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "invalid target URL".to_owned())?;
    if parsed.scheme() != "http" {
        return Err("bench-pipeline needs a plain http:// target".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("target URLs must not contain credentials".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "target has no host".to_owned())?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let mut path = parsed.path().to_owned();
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }
    Ok((host, parsed.port_or_known_default().unwrap_or(80), path))
}

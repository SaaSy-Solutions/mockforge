use std::path::{Path, PathBuf};

/// Routers merged or nested onto the data port that serve mock traffic or
/// static assets, so they stay public. Each entry needs a reason.
pub(super) const PUBLIC_MOUNTS: &[(&str, &str)] = &[
    ("openapi_router", "the user's OpenAPI mocks"),
    ("spec_router", "the user's OpenAPI mocks"),
    ("sse::sse_router", "mock server-sent events"),
    ("file_server::file_serving_router", "mock file downloads"),
    ("health::health_router_with_exclusions", "liveness and readiness; no config"),
    ("oidc_router", "mock OIDC provider discovery and JWKS"),
    ("oauth2_server_router", "mock OAuth2 authorization server"),
    ("consent_router", "mock OAuth2 consent screen"),
    ("mockforge_ws::router", "mock WebSocket endpoints"),
    ("gql_router", "mock GraphQL endpoint"),
    ("mockforge_http::llm_mock::router", "mock LLM API"),
    ("mockforge_http::mcp_mock::router", "mock MCP server"),
    (
        "\"/__mockforge/coverage.html\"",
        "static file in the reserved namespace; the outer gate requires the credential",
    ),
    ("\"/__mockforge/docs\"", "static docs page; it fetches the spec with the token"),
];

pub(super) fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(super) fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Strip `#[cfg(test)]` modules and comment lines so only production
/// assembly code is scanned.
pub(super) fn production_source(source: &str) -> String {
    let cut = source.find("#[cfg(test)]\nmod tests").unwrap_or(source.len());
    source[..cut]
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The argument text of a call whose `(` is at `open`.
pub(super) fn call_args(source: &str, open: usize) -> &str {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open + 1..i];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced call at byte {open}");
}

/// Every `.nest(`, `.nest_service(`, `.merge(` and `.route("/__mockforge…`
/// call on the data-port app, with its arguments.
pub(super) fn mount_calls(source: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    for needle in [".nest(", ".nest_service(", ".merge("] {
        let mut from = 0;
        while let Some(pos) = source[from..].find(needle) {
            let open = from + pos + needle.len() - 1;
            calls.push((needle.to_string(), call_args(source, open).trim().to_string()));
            from = open + 1;
        }
    }
    let mut from = 0;
    while let Some(pos) = source[from..].find(".route(\"/__mockforge") {
        let open = from + pos + ".route".len();
        calls.push((".route(".to_string(), call_args(source, open).trim().to_string()));
        from = open + 1;
    }
    calls
}

pub(super) fn is_public_mount(args: &str) -> bool {
    PUBLIC_MOUNTS.iter().any(|(name, _)| {
        args.starts_with(name)
            || args
                .split_once(',')
                .is_some_and(|(first, rest)| first.trim() == *name || rest.trim().starts_with(name))
    })
}

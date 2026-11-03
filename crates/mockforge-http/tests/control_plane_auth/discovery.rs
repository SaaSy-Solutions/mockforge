use super::assembly::{call_args, is_public_mount, mount_calls};
use axum::http::Method;

/// `(mount prefix or "" for merge, router fn name)`
/// for every mount that is not on the public list, protected or not, so an
/// unprotected control router is probed (and fails) here too.
pub(super) fn control_mounts(lib: &str) -> Vec<(String, String)> {
    let mut mounts = Vec::new();
    for (call, args) in mount_calls(lib) {
        if call == ".route(" || call == ".nest_service(" || is_public_mount(&args) {
            continue;
        }
        let inner = match args.find("management_auth::protect(") {
            Some(at) => &args[at + "management_auth::protect(".len()..],
            None if call == ".nest(" => args.split_once(',').map_or("", |(_, rest)| rest.trim()),
            None => args.as_str(),
        };
        let name: String = inner
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect();
        let name = name.rsplit("::").next().unwrap_or(&name).to_string();
        let prefix = if call == ".nest(" {
            args.split('"').nth(1).unwrap_or_default().to_string()
        } else {
            String::new()
        };
        mounts.push((prefix, name));
    }
    mounts
}

/// `(method, path)` for every `.route("path", ...)` in `body`
pub(super) fn routes_in(body: &str) -> Vec<(Method, String)> {
    let mut routes = Vec::new();
    let mut from = 0;
    while let Some(pos) = body[from..].find(".route(") {
        let open = from + pos + ".route".len();
        let args = call_args(body, open);
        from = open + 1;
        let Some(path) = args.split('"').nth(1) else {
            continue;
        };
        let handler = &args[args.find(',').unwrap_or(0)..];
        let mut methods = Vec::new();
        for (token, method) in [
            ("get(", Method::GET),
            ("post(", Method::POST),
            ("put(", Method::PUT),
            ("patch(", Method::PATCH),
            ("delete(", Method::DELETE),
            ("any(", Method::POST),
        ] {
            let found = handler.match_indices(token).any(|(i, _)| {
                i == 0
                    || !handler.as_bytes()[i - 1].is_ascii_alphanumeric()
                        && handler.as_bytes()[i - 1] != b'_'
            });
            if found {
                methods.push(method);
            }
        }
        for method in methods {
            routes.push((method, path.to_string()));
        }
    }
    routes
}

pub(super) fn concrete(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.starts_with('{') || seg.starts_with(':') || seg.starts_with('*') {
                "probe"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn join(prefix: &str, path: &str) -> String {
    match (prefix.is_empty(), path) {
        (true, p) => p.to_string(),
        (false, "/" | "") => prefix.to_string(),
        (false, p) => format!("{prefix}{p}"),
    }
}

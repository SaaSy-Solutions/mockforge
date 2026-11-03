//! Keep every authenticated route and its declared access synchronized.
use super::super::classify;
use axum::http::Method;
use std::collections::BTreeMap;

const ROUTES_RS: &str = include_str!("../../../routes.rs");
const MANIFEST: &str = include_str!("../../token_scope_routes.txt");

/// Extract `(METHOD, path)` for every `.route(...)` between `start` and
/// `end` markers of `routes.rs`.
fn parse_routes(start: &str, end: &str) -> Vec<(String, String)> {
    let from = ROUTES_RS.find(start).expect("start marker present in routes.rs");
    let to = from + ROUTES_RS[from..].find(end).expect("end marker present in routes.rs");
    let body: String = ROUTES_RS[from..to]
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = body[i..].find(".route(") {
        let open = i + rel + ".route(".len();
        let mut depth = 1;
        let mut p = open;
        while depth > 0 {
            match bytes[p] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            p += 1;
        }
        let inner = &body[open..p - 1];
        let q1 = inner.find('"').expect("route path literal");
        let q2 = q1 + 1 + inner[q1 + 1..].find('"').expect("route path literal end");
        let path = &inner[q1 + 1..q2];
        let rest = &inner[q2 + 1..];
        for m in ["get", "post", "put", "patch", "delete"] {
            let needle = format!("{m}(");
            let mut j = 0;
            while let Some(k) = rest[j..].find(&needle) {
                let at = j + k;
                let prev = rest[..at].chars().last().unwrap_or(' ');
                if !(prev.is_alphanumeric() || prev == '_') {
                    out.push((m.to_uppercase(), path.to_string()));
                }
                j = at + needle.len();
            }
        }
        i = p;
    }
    out
}

fn authenticated_routes() -> Vec<(String, String)> {
    let mut routes = parse_routes("let auth_routes = Router::new()", "let sso_public_routes");
    routes.extend(parse_routes("let admin_routes = Router::new()", "// Combine all routes"));
    routes
}

fn manifest() -> BTreeMap<(String, String), String> {
    MANIFEST
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(parts.len(), 3, "manifest line must be 'METHOD PATH ACCESS': {l}");
            ((parts[0].to_string(), parts[1].to_string()), parts[2].to_string())
        })
        .collect()
}

/// Every authenticated route must be in the manifest with the access the
/// policy table actually grants, and the manifest must list nothing that
/// is not a route. Adding a route without declaring its token access
/// fails here (and the route rejects tokens at runtime until declared).
#[test]
fn every_authenticated_route_is_classified() {
    let routes = authenticated_routes();
    assert!(routes.len() >= 360, "route parser found only {} routes", routes.len());
    let manifest = manifest();
    let mut problems = Vec::new();
    for (method, path) in &routes {
        let m = Method::from_bytes(method.as_bytes()).expect("valid method");
        let actual = classify(&m, path).map(|a| a.manifest_name());
        match manifest.get(&(method.clone(), path.clone())) {
            None => problems.push(format!(
                "{method} {path}: not in token_scope_routes.txt (policy says {actual:?})"
            )),
            Some(declared) if actual.as_deref() != Some(declared.as_str()) => problems
                .push(format!("{method} {path}: manifest says {declared}, policy says {actual:?}")),
            Some(_) => {}
        }
    }
    let route_set: std::collections::BTreeSet<_> = routes.iter().cloned().collect();
    for key in manifest.keys() {
        if !route_set.contains(key) {
            problems.push(format!("{} {}: in manifest but not a route", key.0, key.1));
        }
    }
    assert!(problems.is_empty(), "token scope policy drift:\n{}", problems.join("\n"));
}

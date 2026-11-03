//! Match observed HTTP paths with the same literal priority used by fidelity checks.

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use std::collections::HashMap;

/// Resolve a method/path to its source template and extracted path parameters.
/// Literal segments take precedence over parameters independently of registry order.
pub fn match_template(
    registry: &OpenApiRouteRegistry,
    method: &str,
    concrete_path: &str,
) -> Option<(String, HashMap<String, String>)> {
    registry.routes().iter()
        .filter(|route| route.method.eq_ignore_ascii_case(method))
        .filter_map(|route| segment_match(concrete_path, &route.path).map(|params| (route, params)))
        // Like HTTP routing, prefer the first literal segment over a parameter.
        // Registry iteration order must not determine the response contract.
        .max_by_key(|(route, _)| route.path.split('/').map(|segment|
            !(segment.starts_with('{') && segment.ends_with('}'))).collect::<Vec<_>>())
        .map(|(route, params)| (route.path.clone(), params))
}

fn segment_match(concrete: &str, template: &str) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut c_parts = concrete.split('/');
    for t in template.split('/') {
        let c = c_parts.next()?;
        if t.starts_with('{') && t.ends_with('}') {
            params.insert(t[1..t.len() - 1].to_string(), c.to_string());
        } else if t != c {
            return None;
        }
    }
    if c_parts.next().is_some() {
        return None;
    }
    Some(params)
}

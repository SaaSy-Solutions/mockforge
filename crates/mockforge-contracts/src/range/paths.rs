//! Paths of a pack: whether a request path is a spec's path, and whether it
//! is inside the scope.

/// Whether `path` is the path `template` names. The two match segment for
/// segment, and a segment of the template in braces, `{id}`, matches any one
/// segment: `/orders/{id}` is `/orders/5001`, and is not `/orders` or
/// `/orders/5001/items`.
pub fn matches(template: &str, path: &str) -> bool {
    let mut given = path.split('/');
    for part in template.split('/') {
        let Some(segment) = given.next() else {
            return false;
        };
        let any = part.len() > 2 && part.starts_with('{') && part.ends_with('}');
        if !any && part != segment {
            return false;
        }
    }
    given.next().is_none()
}

/// The label of a method and a path, as a report names a request:
/// `GET /orders/{id}`.
pub fn label(method: &str, path: &str) -> String {
    format!("{method} {path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_matches_its_paths_and_no_other() {
        for (template, path, expected) in [
            ("/orders/{id}", "/orders/5001", true),
            ("/orders/{id}", "/orders/anything", true),
            ("/orders/{id}", "/orders", false),
            ("/orders/{id}", "/orders/5001/items", false),
            ("/orders/{id}", "/users/5001", false),
            ("/users/me", "/users/me", true),
            ("/users/me", "/users/ada", false),
            ("/a/{x}/b/{y}", "/a/1/b/2", true),
            ("/a/{x}/b/{y}", "/a/1/c/2", false),
            // Braces with nothing in them name no parameter.
            ("{}", "/orders", false),
        ] {
            assert_eq!(matches(template, path), expected, "{template} against {path}");
        }
    }
}

//! OData function call URI rewrite layer
//!
//! Rewrites incoming OData function call syntax in request URIs so they match
//! the Axum routes registered by `axum_path()`.
//!
//! Example: `GET /me/getEffectivePermissions(scope='read')` is rewritten to
//! `GET /me/getEffectivePermissions/read` which matches the registered route
//! `/me/getEffectivePermissions/{scope}`.
//!
//! Uses a tower `Layer` that transforms the request URI BEFORE Axum's routing,
//! ensuring the rewritten path is used for route matching.

use axum::http::{Request, Uri};
use std::task::{Context, Poll};

/// Tower layer that rewrites OData function call syntax in request URIs.
///
/// Apply this as a layer on an Axum Router to rewrite OData paths before routing.
///
/// # Example
/// ```rust,ignore
/// use mockforge_core::odata_rewrite::ODataRewriteLayer;
///
/// let app = Router::new()
///     .route("/func/{param}", get(handler))
///     .layer(ODataRewriteLayer);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ODataRewriteLayer;

impl<S> tower::Layer<S> for ODataRewriteLayer {
    type Service = ODataRewriteService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ODataRewriteService { inner }
    }
}

/// Tower service that rewrites OData URIs before forwarding to the inner service.
#[derive(Debug, Clone)]
pub struct ODataRewriteService<S> {
    inner: S,
}

impl<S, B> tower::Service<Request<B>> for ODataRewriteService<S>
where
    S: tower::Service<Request<B>>,
{
    type Response = <S as tower::Service<Request<B>>>::Response;
    type Error = <S as tower::Service<Request<B>>>::Error;
    type Future = <S as tower::Service<Request<B>>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let path = req.uri().path();

        // Fast path: no parentheses means no OData syntax
        if path.contains('(') {
            let rewritten = rewrite_odata_path(path);

            if rewritten != path {
                tracing::debug!("OData rewrite: '{}' -> '{}'", path, rewritten);

                // Rebuild the URI preserving query string
                let new_uri = if let Some(query) = req.uri().query() {
                    format!("{}?{}", rewritten, query)
                } else {
                    rewritten
                };

                if let Ok(uri) = new_uri.parse::<Uri>() {
                    *req.uri_mut() = uri;
                }
            }
        }

        self.inner.call(req)
    }
}

/// Rewrite OData function call syntax in a path.
///
/// Mirrors the logic in `OpenApiRoute::axum_path()` but operates on concrete
/// parameter values instead of `{param}` placeholders.
pub fn rewrite_odata_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '(' {
            // Collect content inside parentheses
            let mut paren_content = String::new();
            for c in chars.by_ref() {
                if c == ')' {
                    break;
                }
                paren_content.push(c);
            }

            if paren_content.is_empty() {
                // Empty parens: functionName() → functionName (strip parens)
                continue;
            }

            if paren_content.contains('=') {
                // key='value' or key=value pairs → /value segments.
                // Split on commas and on the key/value `=` only OUTSIDE quotes so a
                // quoted value like Name='Doe, John' (or one containing `=`) stays
                // intact (#761).
                for part in split_outside_quotes(&paren_content, ',') {
                    if let Some((_key, value)) = split_once_outside_quotes(&part, '=') {
                        let param = value.trim().trim_matches(|c| c == '\'' || c == '"');
                        result.push('/');
                        result.push_str(param);
                    }
                }
            } else {
                // Parentheses without key=value — preserve as-is
                result.push('(');
                result.push_str(&paren_content);
                result.push(')');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Split `s` on `delim`, but only when `delim` appears OUTSIDE single or double
/// quotes. Quote characters are tracked so OData literals such as
/// `Name='Doe, John'` are not shredded on the embedded comma.
fn split_outside_quotes(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for c in s.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(c);
            }
            _ if c == delim && !in_single && !in_double => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    parts.push(current);
    parts
}

/// Split `s` once on the first `delim` that appears OUTSIDE quotes, returning
/// `(before, after)`. Mirrors `str::split_once` but is quote-aware so a `=`
/// inside a quoted value does not act as the key/value separator.
fn split_once_outside_quotes(s: &str, delim: char) -> Option<(String, String)> {
    let mut in_single = false;
    let mut in_double = false;

    for (idx, c) in s.char_indices() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if c == delim && !in_single && !in_double => {
                return Some((s[..idx].to_string(), s[idx + c.len_utf8()..].to_string()));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_path_normal_paths() {
        assert_eq!(rewrite_odata_path("/users"), "/users");
        assert_eq!(rewrite_odata_path("/users/123"), "/users/123");
        assert_eq!(rewrite_odata_path("/api/v1/items"), "/api/v1/items");
    }

    #[test]
    fn test_single_param_odata_rewrite() {
        assert_eq!(
            rewrite_odata_path("/me/getEffectivePermissions(scope='read')"),
            "/me/getEffectivePermissions/read"
        );
        assert_eq!(
            rewrite_odata_path("/reports/getTeamsUserActivityCounts(period='D7')"),
            "/reports/getTeamsUserActivityCounts/D7"
        );
    }

    #[test]
    fn test_multi_param_odata_rewrite() {
        assert_eq!(rewrite_odata_path("/func(key1='val1',key2='val2')"), "/func/val1/val2");
    }

    #[test]
    fn test_quoted_value_with_comma_preserved() {
        // #761: a comma inside a quoted value must not split the key=value pair.
        assert_eq!(rewrite_odata_path("/Customers(Name='Doe, John')"), "/Customers/Doe, John");
    }

    #[test]
    fn test_quoted_value_with_comma_multi_param() {
        assert_eq!(
            rewrite_odata_path("/func(name='Doe, John',city='Paris, FR')"),
            "/func/Doe, John/Paris, FR"
        );
    }

    #[test]
    fn test_quoted_value_with_equals_preserved() {
        // An `=` inside a quoted value must not act as the key/value separator.
        assert_eq!(rewrite_odata_path("/func(expr='a=b')"), "/func/a=b");
    }

    #[test]
    fn test_empty_parens_stripped() {
        assert_eq!(rewrite_odata_path("/func()"), "/func");
        assert_eq!(rewrite_odata_path("/a/func()/b"), "/a/func/b");
    }

    #[test]
    fn test_nested_odata_in_middle_of_path() {
        assert_eq!(
            rewrite_odata_path("/drives/abc/items/xyz/delta(token='foo')"),
            "/drives/abc/items/xyz/delta/foo"
        );
    }

    #[test]
    fn test_unquoted_values() {
        assert_eq!(rewrite_odata_path("/func(key=value)"), "/func/value");
    }

    #[test]
    fn test_value_without_equals_preserved() {
        // Parentheses without key=value syntax should be preserved
        assert_eq!(rewrite_odata_path("/func(something)"), "/func(something)");
    }

    #[test]
    fn test_query_string_not_in_path() {
        // rewrite_odata_path only handles the path portion;
        // query string preservation is handled by the service itself.
        assert_eq!(rewrite_odata_path("/func(key='val')"), "/func/val");
    }

    #[test]
    fn test_microsoft_graph_odata_paths() {
        // Real Microsoft Graph OData function call patterns
        assert_eq!(
            rewrite_odata_path("/reports/microsoft.graph.getTeamsUserActivityCounts(period='D7')"),
            "/reports/microsoft.graph.getTeamsUserActivityCounts/D7"
        );
        assert_eq!(
            rewrite_odata_path(
                "/users/abc/calendar/microsoft.graph.allowedCalendarSharingRoles(User='admin')"
            ),
            "/users/abc/calendar/microsoft.graph.allowedCalendarSharingRoles/admin"
        );
    }

    #[test]
    fn test_microsoft_graph_multi_param() {
        assert_eq!(
            rewrite_odata_path(
                "/groups/abc/team/primaryChannel/microsoft.graph.doesUserHaveAccess(userId='u1',tenantId='t1',userPrincipalName='user@example.com')"
            ),
            "/groups/abc/team/primaryChannel/microsoft.graph.doesUserHaveAccess/u1/t1/user@example.com"
        );
    }
}

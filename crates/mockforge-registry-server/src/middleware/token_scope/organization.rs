//! Bind path and header organization selection to the authenticated token.
use super::forbidden;
use axum::{extract::Request, http::HeaderValue, response::Response};
use uuid::Uuid;

fn org_mismatch(message: String) -> Response {
    forbidden(
        "TOKEN_ORG_MISMATCH",
        message,
        "An API token can only act on the organization it was created in.",
    )
}

/// Values of every `{org_id}` parameter in `path`, aligned against the
/// route template.
pub(super) fn org_id_path_params<'a>(template: &str, path: &'a str) -> Vec<&'a str> {
    template
        .split('/')
        .zip(path.split('/'))
        .filter(|(t, _)| *t == "{org_id}")
        .map(|(_, p)| p)
        .collect()
}

/// Bind a token request to the token's organization. Returns the rejection
/// response if the request names any other org.
pub(super) fn pin_token_org(
    request: &mut Request,
    template: &str,
    token_org: Uuid,
) -> Result<(), Response> {
    for raw in org_id_path_params(template, request.uri().path()) {
        if Uuid::parse_str(raw).ok() != Some(token_org) {
            return Err(org_mismatch(format!(
                "This API token belongs to organization {token_org} and cannot access \
                 organization {raw}"
            )));
        }
    }

    let headers = request.headers_mut();
    if headers.contains_key("X-Organization-Slug") {
        return Err(org_mismatch(
            "X-Organization-Slug is not accepted with API tokens; the token is bound to \
             its organization"
                .to_string(),
        ));
    }
    if let Some(value) = headers.get("X-Organization-Id") {
        let requested = value.to_str().ok().and_then(|s| Uuid::parse_str(s.trim()).ok());
        if requested != Some(token_org) {
            return Err(org_mismatch(format!(
                "This API token belongs to organization {token_org}; X-Organization-Id names \
                 a different organization"
            )));
        }
    }
    let pinned = HeaderValue::from_str(&token_org.to_string())
        .map_err(|_| org_mismatch("Invalid token organization".to_string()))?;
    headers.insert("X-Organization-Id", pinned);
    Ok(())
}

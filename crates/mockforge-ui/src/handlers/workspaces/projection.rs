//! Project nested authored requests and inherited headers into static HTTP mocks.
use super::inheritance::{collect, merge_headers};
use super::*;

pub(super) fn workspace_runtime_mocks(
    workspace: &Workspace,
    id: &str,
) -> Result<Vec<mockforge_http::management::MockConfig>, Response> {
    if workspace.config.auth.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Workspace authentication is not supported by static activation"})),
        )
            .into_response());
    }
    let mut requests = workspace.requests.clone();
    for request in &mut requests {
        request.headers = merge_headers(&workspace.config.default_headers, &request.headers);
    }
    for folder in &workspace.folders {
        collect(folder, &workspace.config.default_headers, &mut requests)?;
    }
    let mut mocks = Vec::new();
    for request in requests {
        if request.auth.is_some() || !request.path.starts_with('/') {
            return Err(StatusCode::BAD_REQUEST.into_response());
        }
        let mut headers = request.response.headers;
        if let Some(content_type) = request.response.content_type {
            headers.insert("content-type".into(), content_type);
        }
        if headers.iter().any(|(key, value)| {
            axum::http::HeaderName::from_bytes(key.as_bytes()).is_err()
                || axum::http::HeaderValue::from_str(value).is_err()
        }) {
            return Err(StatusCode::BAD_REQUEST.into_response());
        }
        mocks.push(mockforge_http::management::MockConfig {
            id: format!("__workspace_live:{}:{}", urlencoding::encode(id), request.id),
            name: request.name,
            method: format!("{:?}", request.method),
            path: format!("/workspace/{}{}", urlencoding::encode(id), request.path),
            response: mockforge_http::management::MockResponse {
                body: serde_json::Value::String(request.response.body.unwrap_or_default()),
                headers: Some(headers),
            },
            enabled: true,
            latency_ms: request.response.delay_ms,
            status_code: Some(request.response.status_code),
            request_match: Some(mockforge_http::management::RequestMatchCriteria {
                headers: request.headers,
                query_params: request.query_params,
                body_pattern: request.body,
                ..Default::default()
            }),
            priority: Some(request.priority),
            scenario: None,
            required_scenario_state: None,
            new_scenario_state: None,
        });
    }
    Ok(mocks)
}

//! Middleware/utilities to apply latency/failure and overrides per operation.
use axum::http::{Request, StatusCode};
use axum::{extract::State, middleware::Next, response::Response, Extension};
use serde_json::Value;

use crate::latency_profiles::LatencyProfiles;
use crate::overrides::Overrides;

#[derive(Clone)]
pub struct OperationMeta {
    pub id: String,
    pub tags: Vec<String>,
}

#[derive(Clone)]
pub struct Shared {
    pub profiles: LatencyProfiles,
    pub overrides: Overrides,
}

pub async fn fault_then_next<B>(
    State(shared): State<Shared>,
    Extension(op): Extension<OperationMeta>,
    req: Request<B>,
    next: Next,
) -> Response
where
    B: axum::body::HttpBody<Data = axum::body::Bytes> + Send + 'static,
    B::Error: Send + std::fmt::Display + std::error::Error + Sync,
{
    if let Some((code, msg)) = shared
        .profiles
        .maybe_fault(&op.id, &op.tags.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .await
    {
        let mut res = Response::new(axum::body::Body::from(msg));
        *res.status_mut() = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        return res;
    }
    let (parts, body) = req.into_parts();
    let body = axum::body::Body::new(body);
    let req = Request::from_parts(parts, body);
    next.run(req).await
}

pub fn apply_overrides(shared: &Shared, op: &OperationMeta, body: &mut Value) {
    shared.overrides.apply(
        &op.id,
        &op.tags.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        body,
    );
}

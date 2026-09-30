//! Concrete implementations of the OpenAPI router's response hooks.
//!
//! [`CoreResponseRewriter`] wraps core's `templating::expand_tokens`, and
//! [`SharedOverrides`] is the router's [`OverrideHook`], so the OpenAPI
//! router (which depends only on the traits) can dispatch through them
//! without core's templating and conditions graph bleeding into
//! `mockforge-openapi`.

use crate::conditions::ConditionContext;
use crate::overrides::SharedOverrides;
use crate::templating::expand_tokens as core_expand_tokens;
use mockforge_openapi::response_rewriter::{OverrideHook, OverrideRequest, ResponseRewriter};
use serde_json::Value;

/// Core's `ResponseRewriter` implementation: template token expansion.
#[derive(Debug, Default, Clone)]
pub struct CoreResponseRewriter;

impl CoreResponseRewriter {
    /// Construct the rewriter
    pub fn new() -> Self {
        Self
    }
}

impl ResponseRewriter for CoreResponseRewriter {
    fn expand_tokens(&self, body: &mut Value) {
        *body = core_expand_tokens(body);
    }
}

impl OverrideHook for SharedOverrides {
    fn apply(&self, request: &OverrideRequest<'_>, body: &mut Value) {
        if self.is_empty() {
            return;
        }
        if !self.needs_context() {
            self.apply_with_context(
                request.operation_id,
                request.tags,
                request.path_template,
                body,
                &ConditionContext::new(),
            );
            return;
        }
        let headers = request
            .headers
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.as_str().to_string(), v.to_string()))
            })
            .collect();
        let query = request
            .query
            .map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default();
        let mut context = ConditionContext::new()
            .with_method(request.method.to_string())
            .with_path(request.path_template.to_string())
            .with_operation_id(request.operation_id.to_string())
            .with_tags(request.tags.to_vec())
            .with_headers(headers)
            .with_query_params(query)
            .with_response_body(body.clone());
        if let Ok(request_body) = serde_json::from_slice(request.body) {
            context = context.with_request_body(request_body);
        }
        self.apply_with_context(
            request.operation_id,
            request.tags,
            request.path_template,
            body,
            &context,
        );
    }
}

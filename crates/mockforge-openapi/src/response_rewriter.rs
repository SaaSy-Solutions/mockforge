//! Response-mutation hooks the OpenAPI router calls on each generated body.
//!
//! 1. [`ResponseRewriter`] expands template tokens like `{{uuid}}`.
//! 2. [`OverrideHook`] applies user-supplied override rules.
//!
//! Both engines live in `mockforge-core`. The router dispatches through
//! these traits so `mockforge-openapi` does not depend on core's templating
//! and conditions graph. Core supplies the implementations
//! (`mockforge_core::openapi_rewriter::CoreResponseRewriter` and
//! `mockforge_core::SharedOverrides`).

use axum::http::HeaderMap;
use serde_json::Value;

/// Hook for template token expansion, called only when expansion is enabled
/// for the current context.
pub trait ResponseRewriter: Send + Sync {
    /// Expand template tokens (e.g. `{{uuid}}`, `{{now}}`) in-place inside
    /// the response body.
    fn expand_tokens(&self, body: &mut Value);
}

/// The request an override rule is evaluated against
#[derive(Debug, Clone, Copy)]
pub struct OverrideRequest<'a> {
    /// HTTP method, uppercase
    pub method: &'a str,
    /// The OpenAPI path template, e.g. `/users/{id}`
    pub path_template: &'a str,
    /// The operation's `operationId`, or empty when the spec has none
    pub operation_id: &'a str,
    /// The operation's OpenAPI tags followed by its `operationId`
    pub tags: &'a [String],
    /// Request headers
    pub headers: &'a HeaderMap,
    /// Raw query string, without the leading `?`
    pub query: Option<&'a str>,
    /// Raw request body
    pub body: &'a [u8],
}

/// Hook that applies override rules to a generated response body
pub trait OverrideHook: Send + Sync {
    /// Patch `body` in place with every rule that matches `request`
    fn apply(&self, request: &OverrideRequest<'_>, body: &mut Value);
}

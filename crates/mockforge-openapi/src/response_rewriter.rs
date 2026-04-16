//! [`ResponseRewriter`] trait for post-generation response mutation.
//!
//! The OpenAPI router runs two optional mutation passes on each generated
//! response body:
//!
//! 1. **Template token expansion** — substitutes placeholders like
//!    `{{uuid}}` or `{{now}}` with rendered values.
//! 2. **Override application** — applies operation-targeted patches from
//!    user-supplied rules.
//!
//! Both of those historically lived in `mockforge-core`
//! (`templating::expand_tokens` and `overrides::Overrides::apply`). To
//! let `mockforge-openapi` own the router without pulling in core's
//! templating + conditions + encryption graph, the router dispatches
//! through a `ResponseRewriter` trait object. Core supplies the concrete
//! implementation (see `mockforge_core::openapi_rewriter::CoreResponseRewriter`)
//! that chains its own templating + overrides engines.

use serde_json::Value;

/// Hook for post-generation response body mutation used by the OpenAPI
/// router. Implementations are called conditionally — `expand_tokens` only
/// when template expansion is enabled for the current context, and
/// `apply_overrides` only when operation overrides are enabled.
pub trait ResponseRewriter: Send + Sync {
    /// Expand template tokens (e.g. `{{uuid}}`, `{{now}}`) in-place inside
    /// the response body.
    fn expand_tokens(&self, body: &mut Value);

    /// Apply user-supplied override rules to the response body in-place,
    /// keyed by the operation id, tags, and request path.
    fn apply_overrides(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value);
}

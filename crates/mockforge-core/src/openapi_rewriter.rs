//! Concrete [`ResponseRewriter`] implementation for core.
//!
//! [`ResponseRewriter`]: mockforge_openapi::response_rewriter::ResponseRewriter
//!
//! Wraps core's `templating::expand_tokens` and the optional
//! [`Overrides`](crate::overrides::Overrides) engine so the OpenAPI router
//! (which now depends only on the `ResponseRewriter` trait) can dispatch
//! through it without core's templating/conditions graph bleeding into
//! `mockforge-openapi`.

use crate::overrides::Overrides;
use crate::templating::expand_tokens as core_expand_tokens;
use mockforge_openapi::response_rewriter::ResponseRewriter;
use serde_json::Value;

/// Core's `ResponseRewriter` implementation — chains core's templating
/// expander and (optionally) an `Overrides` ruleset.
#[derive(Debug, Default, Clone)]
pub struct CoreResponseRewriter {
    /// Optional overrides ruleset. When `None`, [`apply_overrides`] is a
    /// no-op.
    ///
    /// [`apply_overrides`]: ResponseRewriter::apply_overrides
    pub overrides: Option<Overrides>,
}

impl CoreResponseRewriter {
    /// Construct a rewriter with the given optional overrides ruleset.
    pub fn new(overrides: Option<Overrides>) -> Self {
        Self { overrides }
    }
}

impl ResponseRewriter for CoreResponseRewriter {
    fn expand_tokens(&self, body: &mut Value) {
        *body = core_expand_tokens(body);
    }

    fn apply_overrides(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value) {
        if let Some(ref rules) = self.overrides {
            rules.apply(operation_id, tags, path, body);
        }
    }
}

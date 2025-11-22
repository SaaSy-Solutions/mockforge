//! Template expansion utilities for request context variables
//!
//! This module re-exports Send-safe template expansion from mockforge-template-expansion.
//! The actual implementation is in a separate crate to avoid Send issues with rng().
//!
//! Import directly from mockforge-template-expansion crate:
//! ```rust
//! use mockforge_template_expansion::expand_templates_in_json;
//! ```

/// Expand template variables in a JSON value recursively using request context
///
/// **Note**: This function has been moved to `mockforge-template-expansion` crate.
/// Use `mockforge_template_expansion::expand_templates_in_json` instead.
///
/// This is a placeholder function for backwards compatibility that will panic if called.
///
/// # Arguments
/// * `_value` - JSON value to process (unused)
/// * `_context` - Request context for template variable expansion (unused)
///
/// # Returns
/// This function will panic with a message directing users to use the new crate.
///
/// # Panics
/// Always panics with a message to use `mockforge_template_expansion::expand_templates_in_json` instead.
#[deprecated(note = "Use mockforge_template_expansion::expand_templates_in_json instead")]
pub fn expand_templates_in_json(
    _value: serde_json::Value,
    _context: &crate::ai_response::RequestContext,
) -> serde_json::Value {
    // This is a placeholder - the actual implementation is in mockforge-template-expansion
    // This function should not be called directly. Use mockforge_template_expansion::expand_templates_in_json instead.
    unimplemented!("expand_templates_in_json has been moved to mockforge-template-expansion crate. Use mockforge_template_expansion::expand_templates_in_json instead.")
}

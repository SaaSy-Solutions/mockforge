#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::templating::{expand_str_with_context, TemplatingContext};
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    // Try to use the fuzz input as a template
    if let Ok(template_str) = std::str::from_utf8(data) {
        // Create a simple context for rendering
        let context = json!({
            "name": "test",
            "value": 123,
            "items": ["a", "b", "c"],
            "nested": {
                "field": "value"
            }
        });

        // Attempt to render the template
        let templating_context = TemplatingContext::empty();
        let _ = expand_str_with_context(template_str, &templating_context);
    }
});

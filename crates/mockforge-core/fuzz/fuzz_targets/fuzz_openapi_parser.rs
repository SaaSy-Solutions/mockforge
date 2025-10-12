#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::openapi_routes::create_registry_from_json;

fuzz_target!(|data: &[u8]| {
    // Try to parse the fuzz input as JSON
    if let Ok(json_str) = std::str::from_utf8(data) {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
            // Attempt to parse as OpenAPI spec
            let _ = create_registry_from_json(json_value);
        }
    }
});

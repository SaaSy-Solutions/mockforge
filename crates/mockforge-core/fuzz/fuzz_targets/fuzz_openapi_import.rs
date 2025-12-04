#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::import::openapi_import::import_openapi_spec;

fuzz_target!(|data: &[u8]| {
    // Try to parse the fuzz input as OpenAPI spec (JSON or YAML)
    if let Ok(spec_str) = std::str::from_utf8(data) {
        // Attempt to import as OpenAPI spec
        // Should never panic, even with malformed specs
        // import_openapi_spec takes content as &str and optional base_url
        let _ = import_openapi_spec(spec_str, None);
    }
});

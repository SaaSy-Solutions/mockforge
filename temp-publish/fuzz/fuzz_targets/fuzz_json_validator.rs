#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::validation::validate_json_schema;

fuzz_target!(|data: &[u8]| {
    // Try to parse the fuzz input as JSON
    if let Ok(json_str) = std::str::from_utf8(data) {
        // Split input into schema and data (using first half as schema, second as data)
        let mid = data.len() / 2;

        if let Ok(schema_str) = std::str::from_utf8(&data[..mid]) {
            if let Ok(data_str) = std::str::from_utf8(&data[mid..]) {
                if let Ok(schema) = serde_json::from_str::<serde_json::Value>(schema_str) {
                    if let Ok(data_value) = serde_json::from_str::<serde_json::Value>(data_str) {
                        // Attempt to validate
                        let _ = validate_json_schema(&data_value, &schema);
                    }
                }
            }
        }
    }
});

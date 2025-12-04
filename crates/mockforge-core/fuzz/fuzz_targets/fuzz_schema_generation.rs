#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_data::schema::SchemaDefinition;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    // Try to parse the fuzz input as a JSON schema
    if let Ok(json_str) = std::str::from_utf8(data) {
        if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
            // Attempt to create SchemaDefinition from JSON schema
            // Should never panic, even with malformed schemas
            let _ = SchemaDefinition::from_json_schema(&json_value);
        }
    }

    // Also try parsing raw bytes as JSON
    if let Ok(json_value) = serde_json::from_slice::<Value>(data) {
        let _ = SchemaDefinition::from_json_schema(&json_value);
    }
});

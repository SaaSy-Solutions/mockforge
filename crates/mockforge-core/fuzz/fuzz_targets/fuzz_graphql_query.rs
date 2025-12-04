#![no_main]

use libfuzzer_sys::fuzz_target;
use async_graphql::parser::{parse_query, parse_schema};

fuzz_target!(|data: &[u8]| {
    // Try to use the fuzz input as a GraphQL query or schema
    if let Ok(input_str) = std::str::from_utf8(data) {
        // Try parsing as GraphQL query
        // Should never panic, even with malformed queries
        let _ = parse_query(input_str);

        // Try parsing as GraphQL schema (SDL)
        // Should never panic, even with malformed schemas
        let _ = parse_schema(input_str);
    }
});

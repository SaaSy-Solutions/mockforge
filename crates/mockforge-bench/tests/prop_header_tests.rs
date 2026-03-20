//! Property-based tests for header parsing.
//!
//! Validates that `parse_header_string` handles arbitrary input correctly,
//! including edge cases like multiple colons in values, whitespace, and
//! the known comma-in-value limitation.

use mockforge_bench::command::parse_header_string;
use proptest::prelude::*;

proptest! {
    /// Valid headers with ASCII key/value (no commas or colons in key)
    /// should always round-trip: parse produces the expected key-value pair.
    #[test]
    fn valid_single_header_roundtrips(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        value in "[^\n\r,]*"  // no commas in value (known limitation)
    ) {
        let input = format!("{key}:{value}");
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.len(), 1);
        prop_assert_eq!(headers.get(key.trim()), Some(&value.trim().to_string()));
    }

    /// Multiple valid headers separated by commas should all parse correctly.
    #[test]
    fn multiple_headers_parse(
        pairs in prop::collection::vec(
            ("[a-zA-Z][a-zA-Z0-9-]*", "[a-zA-Z0-9 _=;/.-]*"),
            1..5
        )
    ) {
        let input = pairs
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(",");

        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input}: {result:?}");
        let headers = result.expect("already checked");
        // May be fewer than pairs.len() if keys collide (HashMap dedup)
        prop_assert!(headers.len() <= pairs.len());

        // Last value for each key wins
        for (k, _v) in &pairs {
            // Only check if this key isn't overwritten by a later pair
            let last_value = pairs.iter().rev().find(|(kk, _)| kk == k).map(|(_, vv)| vv);
            if let Some(expected) = last_value {
                prop_assert_eq!(
                    headers.get(k.trim()),
                    Some(&expected.trim().to_string()),
                    "mismatch for key {}", k
                );
            }
        }
    }

    /// Values containing colons should be preserved (splitn(2, ':') keeps them).
    #[test]
    fn colon_in_value_preserved(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        before_colon in "[a-zA-Z0-9]+",
        after_colon in "[a-zA-Z0-9]*"
    ) {
        let value = format!("{before_colon}:{after_colon}");
        let input = format!("{key}:{value}");
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.get(key.trim()), Some(&value.trim().to_string()));
    }

    /// Input without a colon should return an error.
    #[test]
    fn missing_colon_is_error(
        input in "[a-zA-Z0-9 _-]+"  // no colon
    ) {
        prop_assume!(!input.contains(':'));
        let result = parse_header_string(&input);
        prop_assert!(result.is_err(), "expected error for no-colon input: {input}");
    }

    /// Whitespace around keys and values should be trimmed.
    #[test]
    fn whitespace_is_trimmed(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        value in "[a-zA-Z0-9]+",
        leading_spaces in 0u8..4,
        trailing_spaces in 0u8..4,
    ) {
        let padded_key = format!("{}{key}{}", " ".repeat(leading_spaces as usize), " ".repeat(trailing_spaces as usize));
        let input = format!("{padded_key}: {value} ");
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.get(&key), Some(&value.to_string()));
    }

    /// Empty input should still be parseable (single empty pair → error).
    #[test]
    fn empty_string_is_error(_dummy in Just(())) {
        let result = parse_header_string("");
        // Empty string → split(",") yields [""] → no colon → error
        prop_assert!(result.is_err());
    }
}

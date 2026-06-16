//! Property-based tests for header parsing.
//!
//! Validates that `parse_header_string` handles arbitrary input correctly.
//! Since #761 headers are passed one `Key:Value` per repeated `--headers` flag
//! (a `&[String]`), so header VALUES may freely contain commas.

use mockforge_bench::command::parse_header_string;
use proptest::prelude::*;

proptest! {
    /// A single valid header (ASCII key, value may even contain commas) should
    /// always round-trip.
    #[test]
    fn valid_single_header_roundtrips(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        value in "[^\n\r]*"  // commas allowed now (one header per flag)
    ) {
        let input = vec![format!("{key}:{value}")];
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input:?}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.len(), 1);
        prop_assert_eq!(headers.get(key.trim()), Some(&value.trim().to_string()));
    }

    /// Multiple valid headers, one per slice element, should all parse correctly.
    #[test]
    fn multiple_headers_parse(
        pairs in prop::collection::vec(
            ("[a-zA-Z][a-zA-Z0-9-]*", "[a-zA-Z0-9 _=;/.,-]*"),
            1..5
        )
    ) {
        let input = pairs.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>();

        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input:?}: {result:?}");
        let headers = result.expect("already checked");
        // May be fewer than pairs.len() if keys collide (HashMap dedup)
        prop_assert!(headers.len() <= pairs.len());

        // Last value for each key wins
        for (k, _v) in &pairs {
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

    /// A comma anywhere in the value must be preserved verbatim (#761).
    #[test]
    fn comma_in_value_preserved(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        before in "[a-zA-Z0-9 ]+",
        after in "[a-zA-Z0-9 ]+",
    ) {
        let value = format!("{before}, {after}");
        let input = vec![format!("{key}:{value}")];
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input:?}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.get(key.trim()), Some(&value.trim().to_string()));
    }

    /// Values containing colons should be preserved (splitn(2, ':') keeps them).
    #[test]
    fn colon_in_value_preserved(
        key in "[a-zA-Z][a-zA-Z0-9-]*",
        before_colon in "[a-zA-Z0-9]+",
        after_colon in "[a-zA-Z0-9]*"
    ) {
        let value = format!("{before_colon}:{after_colon}");
        let input = vec![format!("{key}:{value}")];
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input:?}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.get(key.trim()), Some(&value.trim().to_string()));
    }

    /// A non-blank element without a colon should return an error.
    #[test]
    fn missing_colon_is_error(
        input in "[a-zA-Z0-9 _-]+"  // no colon
    ) {
        prop_assume!(!input.contains(':'));
        // Blank elements are intentionally skipped (treated as a stray flag), so
        // only assert the error for elements with at least one non-whitespace char.
        prop_assume!(!input.trim().is_empty());
        let result = parse_header_string(std::slice::from_ref(&input));
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
        let input = vec![format!("{padded_key}: {value} ")];
        let result = parse_header_string(&input);
        prop_assert!(result.is_ok(), "parse failed for {input:?}: {result:?}");
        let headers = result.expect("already checked");
        prop_assert_eq!(headers.get(&key), Some(&value.to_string()));
    }
}

/// An empty list (no `--headers` flags) parses to an empty map, not an error.
#[test]
fn empty_list_is_ok() {
    let result = parse_header_string(&[]);
    assert!(result.is_ok());
    assert!(result.expect("ok").is_empty());
}

/// A blank element is skipped rather than treated as a malformed header.
#[test]
fn blank_element_is_skipped() {
    let result = parse_header_string(&["".to_string(), "A:1".to_string()]);
    let headers = result.expect("ok");
    assert_eq!(headers.len(), 1);
    assert_eq!(headers.get("A"), Some(&"1".to_string()));
}

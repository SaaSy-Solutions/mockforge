//! ID generation utilities
//!
//! This module provides functionality for generating various types of IDs including
//! pattern-based IDs and realistic-looking IDs (Stripe-style).

use crate::schema::AutoGenerationRule;
use crate::{Error, Result};
use rand::Rng;
use regex::Regex;
use serde_json::Value;
use uuid::Uuid;

/// Generate an ID based on an auto-generation rule
///
/// # Arguments
/// * `rule` - The auto-generation rule to apply
/// * `entity_name` - Name of the entity (for counter tracking)
/// * `field_name` - Name of the field (for counter tracking)
/// * `counter` - Optional counter value for increment-based patterns
pub fn generate_id(
    rule: &AutoGenerationRule,
    entity_name: &str,
    field_name: &str,
    counter: Option<u64>,
) -> Result<String> {
    match rule {
        AutoGenerationRule::Uuid => Ok(Uuid::new_v4().to_string()),
        AutoGenerationRule::Timestamp => Ok(chrono::Utc::now().timestamp().to_string()),
        AutoGenerationRule::Date => Ok(chrono::Utc::now().date_naive().to_string()),
        AutoGenerationRule::Pattern(pattern) => {
            generate_pattern_id(pattern, entity_name, field_name, counter)
        }
        AutoGenerationRule::Realistic { prefix, length } => generate_realistic_id(prefix, *length),
        AutoGenerationRule::AutoIncrement => {
            // Auto-increment should be handled by database
            Err(Error::generic("AutoIncrement should be handled by database".to_string()))
        }
        AutoGenerationRule::Custom(_) => {
            // Custom rules would need an evaluation engine
            Err(Error::generic("Custom rules not yet supported".to_string()))
        }
    }
}

/// Generate a pattern-based ID
///
/// Supports template variables:
/// - `{increment}` or `{increment:06}` - Auto-incrementing number with padding
/// - `{timestamp}` - Unix timestamp
/// - `{random}` - Random alphanumeric string (default 8 chars)
/// - `{random:N}` - Random alphanumeric string of length N
/// - `{uuid}` - UUID v4
fn generate_pattern_id(
    pattern: &str,
    _entity_name: &str,
    _field_name: &str,
    counter: Option<u64>,
) -> Result<String> {
    let mut result = pattern.to_string();

    // Replace {increment} or {increment:NN} patterns
    let increment_re = Regex::new(r"\{increment(?::(\d+))?\}").unwrap();
    if increment_re.is_match(&result) {
        let increment_value = counter.unwrap_or(1);
        result = increment_re
            .replace_all(&result, |caps: &regex::Captures| {
                if let Some(padding_str) = caps.get(1) {
                    let padding: usize = padding_str.as_str().parse().unwrap_or(6);
                    format!("{:0width$}", increment_value, width = padding)
                } else {
                    increment_value.to_string()
                }
            })
            .to_string();
    }

    // Replace {timestamp}
    if result.contains("{timestamp}") {
        let timestamp = chrono::Utc::now().timestamp();
        result = result.replace("{timestamp}", &timestamp.to_string());
    }

    // Replace {random} or {random:N}
    let random_re = Regex::new(r"\{random(?::(\d+))?\}").unwrap();
    if random_re.is_match(&result) {
        result = random_re
            .replace_all(&result, |caps: &regex::Captures| {
                let length: usize = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(8);
                generate_random_string(length)
            })
            .to_string();
    }

    // Replace {uuid}
    if result.contains("{uuid}") {
        result = result.replace("{uuid}", &Uuid::new_v4().to_string());
    }

    Ok(result)
}

/// Generate a realistic-looking ID (Stripe-style)
///
/// Format: `{prefix}_{random_alphanumeric}`
///
/// # Arguments
/// * `prefix` - Prefix for the ID (e.g., "cus", "ord")
/// * `length` - Length of the random alphanumeric part
fn generate_realistic_id(prefix: &str, length: usize) -> Result<String> {
    let random_part = generate_random_string(length);
    Ok(format!("{}_{}", prefix, random_part))
}

/// Generate a random alphanumeric string
///
/// Uses lowercase letters and numbers (base36-like, but with lowercase)
fn generate_random_string(length: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

/// Get or increment counter for an entity field
///
/// This function should be called to get the current counter value and increment it.
/// The counter is typically stored in a database table.
pub async fn get_and_increment_counter(
    database: &dyn crate::database::VirtualDatabase,
    entity_name: &str,
    field_name: &str,
) -> Result<u64> {
    let counter_table = "_vbr_counters";
    let key = format!("{}:{}", entity_name, field_name);

    // Check if counter table exists, create if not
    if !database.table_exists(counter_table).await? {
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)",
            counter_table
        );
        database.create_table(&create_table).await?;
    }

    // Get current value
    let query = format!("SELECT value FROM {} WHERE key = ?", counter_table);
    let results = database.query(&query, &[Value::String(key.clone())]).await?;

    let current_value = if let Some(row) = results.first() {
        row.get("value").and_then(|v| v.as_u64()).unwrap_or(0)
    } else {
        0
    };

    // Increment and update
    let new_value = current_value + 1;
    let update_query = format!(
        "INSERT INTO {} (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = ?",
        counter_table
    );
    database
        .execute(
            &update_query,
            &[
                Value::String(key),
                Value::Number(new_value.into()),
                Value::Number(new_value.into()),
            ],
        )
        .await?;

    Ok(new_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // generate_pattern_id tests
    #[test]
    fn test_generate_pattern_id() {
        let pattern = "USR-{increment:06}";
        let result = generate_pattern_id(pattern, "User", "id", Some(1)).unwrap();
        assert_eq!(result, "USR-000001");

        let pattern = "ORD-{timestamp}";
        let result = generate_pattern_id(pattern, "Order", "id", None).unwrap();
        assert!(result.starts_with("ORD-"));

        let pattern = "TXN-{random:12}";
        let result = generate_pattern_id(pattern, "Transaction", "id", None).unwrap();
        assert!(result.starts_with("TXN-"));
        assert_eq!(result.len(), 16); // "TXN-" (4) + 12 random chars
    }

    #[test]
    fn test_generate_pattern_id_increment_no_padding() {
        let pattern = "ID-{increment}";
        let result = generate_pattern_id(pattern, "Test", "id", Some(42)).unwrap();
        assert_eq!(result, "ID-42");
    }

    #[test]
    fn test_generate_pattern_id_increment_with_padding() {
        let pattern = "NUM-{increment:08}";
        let result = generate_pattern_id(pattern, "Test", "id", Some(123)).unwrap();
        assert_eq!(result, "NUM-00000123");
    }

    #[test]
    fn test_generate_pattern_id_increment_default_counter() {
        let pattern = "SEQ-{increment:04}";
        // When counter is None, defaults to 1
        let result = generate_pattern_id(pattern, "Test", "id", None).unwrap();
        assert_eq!(result, "SEQ-0001");
    }

    #[test]
    fn test_generate_pattern_id_uuid() {
        let pattern = "ITEM-{uuid}";
        let result = generate_pattern_id(pattern, "Test", "id", None).unwrap();
        assert!(result.starts_with("ITEM-"));
        // UUID is 36 chars (including hyphens)
        assert_eq!(result.len(), 41); // "ITEM-" (5) + UUID (36)
    }

    #[test]
    fn test_generate_pattern_id_random_default_length() {
        let pattern = "RND-{random}";
        let result = generate_pattern_id(pattern, "Test", "id", None).unwrap();
        assert!(result.starts_with("RND-"));
        // Default random length is 8
        assert_eq!(result.len(), 12); // "RND-" (4) + 8 random
    }

    #[test]
    fn test_generate_pattern_id_random_custom_length() {
        let pattern = "KEY-{random:20}";
        let result = generate_pattern_id(pattern, "Test", "id", None).unwrap();
        assert!(result.starts_with("KEY-"));
        assert_eq!(result.len(), 24); // "KEY-" (4) + 20 random
    }

    #[test]
    fn test_generate_pattern_id_multiple_placeholders() {
        let pattern = "{increment:03}-{random:4}";
        let result = generate_pattern_id(pattern, "Test", "id", Some(5)).unwrap();
        // Should be "005-" followed by 4 random chars
        assert!(result.starts_with("005-"));
        assert_eq!(result.len(), 8); // "005-" (4) + 4 random
    }

    #[test]
    fn test_generate_pattern_id_plain_text() {
        let pattern = "STATIC-ID";
        let result = generate_pattern_id(pattern, "Test", "id", None).unwrap();
        assert_eq!(result, "STATIC-ID");
    }

    // generate_realistic_id tests
    #[test]
    fn test_generate_realistic_id() {
        let result = generate_realistic_id("cus", 14).unwrap();
        assert!(result.starts_with("cus_"));
        assert_eq!(result.len(), 18); // "cus_" (4) + 14 random chars
    }

    #[test]
    fn test_generate_realistic_id_different_prefix() {
        let result = generate_realistic_id("ord", 10).unwrap();
        assert!(result.starts_with("ord_"));
        assert_eq!(result.len(), 14); // "ord_" (4) + 10 random
    }

    #[test]
    fn test_generate_realistic_id_long_prefix() {
        let result = generate_realistic_id("subscription", 8).unwrap();
        assert!(result.starts_with("subscription_"));
        assert_eq!(result.len(), 21); // "subscription_" (13) + 8 random
    }

    #[test]
    fn test_generate_realistic_id_uniqueness() {
        let id1 = generate_realistic_id("test", 12).unwrap();
        let id2 = generate_realistic_id("test", 12).unwrap();
        // Should generate different IDs
        assert_ne!(id1, id2);
    }

    // generate_random_string tests
    #[test]
    fn test_generate_random_string() {
        let s1 = generate_random_string(8);
        let s2 = generate_random_string(8);
        assert_eq!(s1.len(), 8);
        assert_eq!(s2.len(), 8);
        // Should be different (very unlikely to be the same)
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_generate_random_string_various_lengths() {
        assert_eq!(generate_random_string(1).len(), 1);
        assert_eq!(generate_random_string(5).len(), 5);
        assert_eq!(generate_random_string(16).len(), 16);
        assert_eq!(generate_random_string(32).len(), 32);
    }

    #[test]
    fn test_generate_random_string_valid_chars() {
        let s = generate_random_string(100);
        // All characters should be alphanumeric lowercase or digits
        for c in s.chars() {
            assert!(c.is_ascii_lowercase() || c.is_ascii_digit());
        }
    }

    #[test]
    fn test_generate_random_string_empty() {
        let s = generate_random_string(0);
        assert!(s.is_empty());
    }

    // generate_id tests
    #[test]
    fn test_generate_id_uuid() {
        let rule = AutoGenerationRule::Uuid;
        let result = generate_id(&rule, "Entity", "id", None).unwrap();
        // UUID format validation
        assert_eq!(result.len(), 36);
        assert!(result.contains('-'));
    }

    #[test]
    fn test_generate_id_timestamp() {
        let rule = AutoGenerationRule::Timestamp;
        let result = generate_id(&rule, "Entity", "created_at", None).unwrap();
        // Should be a valid timestamp (numeric string)
        let _timestamp: i64 = result.parse().expect("Should be a valid timestamp");
    }

    #[test]
    fn test_generate_id_date() {
        let rule = AutoGenerationRule::Date;
        let result = generate_id(&rule, "Entity", "date", None).unwrap();
        // Should be in YYYY-MM-DD format
        assert_eq!(result.len(), 10);
        assert!(result.contains('-'));
    }

    #[test]
    fn test_generate_id_pattern() {
        let rule = AutoGenerationRule::Pattern("PREFIX-{increment:04}".to_string());
        let result = generate_id(&rule, "Entity", "id", Some(7)).unwrap();
        assert_eq!(result, "PREFIX-0007");
    }

    #[test]
    fn test_generate_id_realistic() {
        let rule = AutoGenerationRule::Realistic {
            prefix: "inv".to_string(),
            length: 10,
        };
        let result = generate_id(&rule, "Invoice", "id", None).unwrap();
        assert!(result.starts_with("inv_"));
        assert_eq!(result.len(), 14);
    }

    #[test]
    fn test_generate_id_auto_increment_error() {
        let rule = AutoGenerationRule::AutoIncrement;
        let result = generate_id(&rule, "Entity", "id", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_id_custom_error() {
        let rule = AutoGenerationRule::Custom("NOW()".to_string());
        let result = generate_id(&rule, "Entity", "id", None);
        assert!(result.is_err());
    }
}

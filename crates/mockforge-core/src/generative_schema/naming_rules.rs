//! Naming and pluralization rules for generative schema mode
//!
//! Provides configurable naming conventions and pluralization rules
//! for generating API routes and entity names.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pluralization rule for entity names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluralizationRule {
    /// Standard English pluralization (users, products, orders)
    Standard,
    /// No pluralization (user, product, order)
    Singular,
    /// Custom pluralization via mapping
    Custom,
}

/// Naming rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingRules {
    /// Pluralization rule to use
    pub pluralization: PluralizationRule,
    /// Custom pluralization mappings (singular -> plural)
    pub custom_plurals: HashMap<String, String>,
    /// Route prefix (e.g., "/api/v1")
    pub route_prefix: String,
    /// Entity name case (snake_case, camelCase, PascalCase, kebab-case)
    pub entity_case: String,
    /// Route path case (snake_case, camelCase, kebab-case)
    pub route_case: String,
}

impl Default for NamingRules {
    fn default() -> Self {
        Self {
            pluralization: PluralizationRule::Standard,
            custom_plurals: HashMap::new(),
            route_prefix: "/api".to_string(),
            entity_case: "PascalCase".to_string(),
            route_case: "kebab-case".to_string(),
        }
    }
}

impl NamingRules {
    /// Create new naming rules with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Pluralize an entity name
    pub fn pluralize(&self, singular: &str) -> String {
        match self.pluralization {
            PluralizationRule::Standard => Self::standard_pluralize(singular),
            PluralizationRule::Singular => singular.to_string(),
            PluralizationRule::Custom => self
                .custom_plurals
                .get(singular)
                .cloned()
                .unwrap_or_else(|| Self::standard_pluralize(singular)),
        }
    }

    /// Convert entity name to route path
    pub fn entity_to_route(&self, entity_name: &str) -> String {
        let plural = self.pluralize(entity_name);
        let route_name = match self.route_case.as_str() {
            "kebab-case" => Self::to_kebab_case(&plural),
            "snake_case" => Self::to_snake_case(&plural),
            "camelCase" => Self::to_camel_case(&plural),
            _ => plural.to_lowercase(),
        };

        format!("{}/{}", self.route_prefix, route_name)
    }

    /// Format entity name according to entity_case
    pub fn format_entity_name(&self, name: &str) -> String {
        match self.entity_case.as_str() {
            "PascalCase" => Self::to_pascal_case(name),
            "camelCase" => Self::to_camel_case(name),
            "snake_case" => Self::to_snake_case(name),
            "kebab-case" => Self::to_kebab_case(name),
            _ => name.to_string(),
        }
    }

    /// Standard English pluralization
    fn standard_pluralize(singular: &str) -> String {
        // Common irregular plurals
        let irregulars: HashMap<&str, &str> = [
            ("person", "people"),
            ("child", "children"),
            ("mouse", "mice"),
            ("goose", "geese"),
            ("foot", "feet"),
            ("tooth", "teeth"),
            ("man", "men"),
            ("woman", "women"),
        ]
        .iter()
        .cloned()
        .collect();

        if let Some(plural) = irregulars.get(singular.to_lowercase().as_str()) {
            return plural.to_string();
        }

        // Words ending in -y
        if singular.ends_with('y')
            && !matches!(singular.chars().nth_back(1), Some('a' | 'e' | 'i' | 'o' | 'u'))
        {
            return format!("{}ies", &singular[..singular.len() - 1]);
        }

        // Words ending in -s, -x, -z, -ch, -sh
        if singular.ends_with("s")
            || singular.ends_with("x")
            || singular.ends_with("z")
            || singular.ends_with("ch")
            || singular.ends_with("sh")
        {
            return format!("{}es", singular);
        }

        // Words ending in -f or -fe
        if singular.ends_with("fe") {
            return format!("{}ves", &singular[..singular.len() - 2]);
        }
        if singular.ends_with('f') && !singular.ends_with("ff") {
            return format!("{}ves", &singular[..singular.len() - 1]);
        }

        // Default: add -s
        format!("{}s", singular)
    }

    /// Convert to kebab-case
    fn to_kebab_case(s: &str) -> String {
        Self::to_case(s, '-', false)
    }

    /// Convert to snake_case
    fn to_snake_case(s: &str) -> String {
        Self::to_case(s, '_', false)
    }

    /// Convert to camelCase
    fn to_camel_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_alphanumeric() {
                if i == 0 {
                    result.push(ch.to_ascii_lowercase());
                } else if capitalize_next {
                    result.push(ch.to_ascii_uppercase());
                    capitalize_next = false;
                } else {
                    result.push(ch.to_ascii_lowercase());
                }
            } else {
                capitalize_next = true;
            }
        }

        result
    }

    /// Convert to PascalCase
    fn to_pascal_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;

        for ch in s.chars() {
            if ch.is_alphanumeric() {
                if capitalize_next {
                    result.push(ch.to_ascii_uppercase());
                    capitalize_next = false;
                } else {
                    result.push(ch.to_ascii_lowercase());
                }
            } else {
                capitalize_next = true;
            }
        }

        result
    }

    /// Generic case conversion helper
    fn to_case(s: &str, separator: char, capitalize_first: bool) -> String {
        let mut result = String::new();
        let mut prev_was_upper = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 && !prev_was_upper {
                    result.push(separator);
                }
                result.push(ch.to_ascii_lowercase());
                prev_was_upper = true;
            } else if ch.is_alphanumeric() {
                if i == 0 && capitalize_first {
                    result.push(ch.to_ascii_uppercase());
                } else {
                    result.push(ch);
                }
                prev_was_upper = false;
            } else {
                if !result.ends_with(separator) {
                    result.push(separator);
                }
                prev_was_upper = false;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_pluralization() {
        let rules = NamingRules::new();
        assert_eq!(rules.pluralize("user"), "users");
        assert_eq!(rules.pluralize("product"), "products");
        assert_eq!(rules.pluralize("order"), "orders");
        assert_eq!(rules.pluralize("person"), "people");
        assert_eq!(rules.pluralize("child"), "children");
    }

    #[test]
    fn test_entity_to_route() {
        let rules = NamingRules::new();
        assert_eq!(rules.entity_to_route("User"), "/api/users");
        assert_eq!(rules.entity_to_route("Product"), "/api/products");
    }

    #[test]
    fn test_case_conversion() {
        let rules = NamingRules::new();
        assert_eq!(rules.format_entity_name("user_profile"), "UserProfile");
    }
}

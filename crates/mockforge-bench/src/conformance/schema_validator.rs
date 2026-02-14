//! Response schema validation JS generator
//!
//! Generates JavaScript validation expressions from OpenAPI schemas for use in k6 `check()` calls.

use openapiv3::{ReferenceOr, Schema, SchemaKind, StringFormat, Type, VariantOrUnknownOrEmpty};

/// Generates JavaScript validation expressions from OpenAPI schemas
pub struct SchemaValidatorGenerator;

impl SchemaValidatorGenerator {
    /// Generate a JavaScript validation expression from an OpenAPI schema.
    ///
    /// The generated expression evaluates to `true` if `body` matches the schema,
    /// `false` otherwise. It's designed to be used inside k6's `check()` callback.
    pub fn generate_validation(schema: &Schema) -> String {
        Self::generate_for_schema(schema, "body")
    }

    fn generate_for_schema(schema: &Schema, var: &str) -> String {
        match &schema.schema_kind {
            SchemaKind::Type(Type::Object(obj)) => {
                let mut checks = vec![format!("typeof {} === 'object'", var)];
                checks.push(format!("{} !== null", var));

                // Check required fields exist
                for field in &obj.required {
                    checks.push(format!("'{}' in {}", field, var));
                }

                // Check property types
                for (name, prop_ref) in &obj.properties {
                    if let ReferenceOr::Item(prop_schema) = prop_ref {
                        let prop_var = format!("{}['{}']", var, name);
                        let type_check = Self::generate_type_check(prop_schema, &prop_var);
                        if !type_check.is_empty() {
                            // Only validate if the property exists (it might be optional)
                            if obj.required.contains(name) {
                                checks.push(type_check);
                            } else {
                                checks.push(format!(
                                    "({} === undefined || {})",
                                    prop_var, type_check
                                ));
                            }
                        }
                    }
                }

                checks.join(" && ")
            }
            SchemaKind::Type(Type::Array(arr)) => {
                let mut checks = vec![format!("Array.isArray({})", var)];

                if let Some(ReferenceOr::Item(item_schema)) = &arr.items {
                    let item_check = Self::generate_type_check(item_schema, &format!("{}[0]", var));
                    if !item_check.is_empty() {
                        // Only validate items if array is non-empty
                        checks.push(format!("({}.length === 0 || {})", var, item_check));
                    }
                }

                checks.join(" && ")
            }
            SchemaKind::Type(Type::String(s)) => {
                let mut checks = vec![format!("typeof {} === 'string'", var)];

                // Format validation
                let format_str = match &s.format {
                    VariantOrUnknownOrEmpty::Item(StringFormat::Date) => Some("date"),
                    VariantOrUnknownOrEmpty::Item(StringFormat::DateTime) => Some("date-time"),
                    VariantOrUnknownOrEmpty::Unknown(f) => Some(f.as_str()),
                    _ => None,
                };
                if let Some(fmt) = format_str {
                    if let Some(regex) = Self::format_regex(fmt) {
                        checks.push(format!("{}.match({})", var, regex));
                    }
                }

                // Enum check
                if !s.enumeration.is_empty() {
                    let values: Vec<String> = s
                        .enumeration
                        .iter()
                        .filter_map(|v| v.as_ref().map(|s| format!("'{}'", s)))
                        .collect();
                    if !values.is_empty() {
                        checks.push(format!("[{}].includes({})", values.join(","), var));
                    }
                }

                // Length constraints
                if let Some(min) = s.min_length {
                    checks.push(format!("{}.length >= {}", var, min));
                }
                if let Some(max) = s.max_length {
                    checks.push(format!("{}.length <= {}", var, max));
                }

                checks.join(" && ")
            }
            SchemaKind::Type(Type::Integer(i)) => {
                let mut checks = vec![format!("typeof {} === 'number'", var)];
                checks.push(format!("Number.isInteger({})", var));

                if let Some(min) = i.minimum {
                    checks.push(format!("{} >= {}", var, min));
                }
                if let Some(max) = i.maximum {
                    checks.push(format!("{} <= {}", var, max));
                }
                if !i.enumeration.is_empty() {
                    let values: Vec<String> =
                        i.enumeration.iter().filter_map(|v| v.map(|n| n.to_string())).collect();
                    if !values.is_empty() {
                        checks.push(format!("[{}].includes({})", values.join(","), var));
                    }
                }

                checks.join(" && ")
            }
            SchemaKind::Type(Type::Number(n)) => {
                let mut checks = vec![format!("typeof {} === 'number'", var)];

                if let Some(min) = n.minimum {
                    checks.push(format!("{} >= {}", var, min));
                }
                if let Some(max) = n.maximum {
                    checks.push(format!("{} <= {}", var, max));
                }

                checks.join(" && ")
            }
            SchemaKind::Type(Type::Boolean(_)) => {
                format!("typeof {} === 'boolean'", var)
            }
            _ => "true".to_string(), // For unknown/complex schemas, pass by default
        }
    }

    /// Generate a simple type check expression (used for property validation)
    fn generate_type_check(schema: &Schema, var: &str) -> String {
        match &schema.schema_kind {
            SchemaKind::Type(Type::String(_)) => format!("typeof {} === 'string'", var),
            SchemaKind::Type(Type::Integer(_)) => format!("typeof {} === 'number'", var),
            SchemaKind::Type(Type::Number(_)) => format!("typeof {} === 'number'", var),
            SchemaKind::Type(Type::Boolean(_)) => format!("typeof {} === 'boolean'", var),
            SchemaKind::Type(Type::Array(_)) => format!("Array.isArray({})", var),
            SchemaKind::Type(Type::Object(_)) => {
                format!("typeof {} === 'object' && {} !== null", var, var)
            }
            _ => String::new(),
        }
    }

    /// Get a JS regex for a string format
    fn format_regex(format: &str) -> Option<&'static str> {
        match format {
            "email" => Some(r#"/^[^\s@]+@[^\s@]+\.[^\s@]+$/"#),
            "uuid" => Some(r#"/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i"#),
            "date" => Some(r#"/^\d{4}-\d{2}-\d{2}$/"#),
            "date-time" => Some(r#"/^\d{4}-\d{2}-\d{2}T/"#),
            "uri" | "url" => Some(r#"/^https?:\/\//"#),
            "ipv4" => Some(r#"/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/"#),
            "ipv6" => Some(r#"/^[0-9a-fA-F:]+$/"#),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::*;

    fn string_schema() -> Schema {
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType::default())),
        }
    }

    fn integer_schema() -> Schema {
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Integer(IntegerType::default())),
        }
    }

    #[test]
    fn test_string_validation() {
        let js = SchemaValidatorGenerator::generate_validation(&string_schema());
        assert!(js.contains("typeof body === 'string'"));
    }

    #[test]
    fn test_integer_validation() {
        let js = SchemaValidatorGenerator::generate_validation(&integer_schema());
        assert!(js.contains("typeof body === 'number'"));
        assert!(js.contains("Number.isInteger(body)"));
    }

    #[test]
    fn test_boolean_validation() {
        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Boolean(BooleanType::default())),
        };
        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert_eq!(js, "typeof body === 'boolean'");
    }

    #[test]
    fn test_object_validation() {
        let mut obj = ObjectType::default();
        obj.required.push("name".to_string());
        obj.properties
            .insert("name".to_string(), ReferenceOr::Item(Box::new(string_schema())));
        obj.properties
            .insert("age".to_string(), ReferenceOr::Item(Box::new(integer_schema())));

        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(obj)),
        };

        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert!(js.contains("typeof body === 'object'"));
        assert!(js.contains("'name' in body"));
        assert!(js.contains("typeof body['name'] === 'string'"));
        // age is optional
        assert!(js.contains("body['age'] === undefined || typeof body['age'] === 'number'"));
    }

    #[test]
    fn test_array_validation() {
        let arr = ArrayType {
            items: Some(ReferenceOr::Item(Box::new(string_schema()))),
            min_items: None,
            max_items: None,
            unique_items: false,
        };

        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Array(arr)),
        };

        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert!(js.contains("Array.isArray(body)"));
        assert!(js.contains("typeof body[0] === 'string'"));
    }

    #[test]
    fn test_format_regex() {
        assert!(SchemaValidatorGenerator::format_regex("email").is_some());
        assert!(SchemaValidatorGenerator::format_regex("uuid").is_some());
        assert!(SchemaValidatorGenerator::format_regex("date").is_some());
        assert!(SchemaValidatorGenerator::format_regex("date-time").is_some());
        assert!(SchemaValidatorGenerator::format_regex("uri").is_some());
        assert!(SchemaValidatorGenerator::format_regex("ipv4").is_some());
        assert!(SchemaValidatorGenerator::format_regex("ipv6").is_some());
        assert!(SchemaValidatorGenerator::format_regex("unknown").is_none());
    }

    #[test]
    fn test_string_with_date_format() {
        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType {
                format: VariantOrUnknownOrEmpty::Item(StringFormat::Date),
                ..Default::default()
            })),
        };

        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert!(js.contains("typeof body === 'string'"));
        assert!(js.contains(".match("));
    }

    #[test]
    fn test_integer_with_range() {
        let int = IntegerType {
            minimum: Some(0),
            maximum: Some(100),
            ..Default::default()
        };

        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Integer(int)),
        };

        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert!(js.contains("body >= 0"));
        assert!(js.contains("body <= 100"));
    }

    #[test]
    fn test_number_validation() {
        let num = NumberType {
            minimum: Some(0.0),
            maximum: Some(99.9),
            ..Default::default()
        };

        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Number(num)),
        };

        let js = SchemaValidatorGenerator::generate_validation(&schema);
        assert!(js.contains("typeof body === 'number'"));
        assert!(js.contains("body >= 0"));
        assert!(js.contains("body <= 99.9"));
    }
}

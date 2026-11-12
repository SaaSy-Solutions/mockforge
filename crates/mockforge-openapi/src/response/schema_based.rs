//! Schema-based response generation for MockForge
//!
//! Contains methods for generating mock response data from OpenAPI schemas,
//! including pagination-aware array generation, property-name heuristics,
//! and item variation for realistic mock data.

use super::*;
use serde_json::json;

impl ResponseGenerator {
    pub(crate) fn generate_example_from_schema_ref(
        spec: &OpenApiSpec,
        schema_ref: &ReferenceOr<Schema>,
        persona: Option<&Persona>,
    ) -> Value {
        match schema_ref {
            ReferenceOr::Item(schema) => Self::generate_example_from_schema(spec, schema, persona),
            ReferenceOr::Reference { reference } => spec
                .get_schema(reference)
                .map(|schema| Self::generate_example_from_schema(spec, &schema.schema, persona))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        }
    }

    /// Generate example data from an OpenAPI schema
    ///
    /// Priority order:
    /// 1. Schema-level example (schema.schema_data.example)
    /// 2. Property-level examples when generating objects
    /// 3. Generated values based on schema type
    /// 4. Persona traits (if persona provided)
    pub(crate) fn generate_example_from_schema(
        spec: &OpenApiSpec,
        schema: &Schema,
        persona: Option<&Persona>,
    ) -> Value {
        // First, check for schema-level example in schema_data
        // OpenAPI v3 stores examples in schema_data.example
        if let Some(example) = schema.schema_data.example.as_ref() {
            tracing::debug!("Using schema-level example: {:?}", example);
            return example.clone();
        }

        // Note: schema-level example check happens at the top of the function (line 380-383)
        // At this point, if we have a schema-level example, we've already returned it
        // So we only generate defaults when no example exists
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::String(string)) => {
                Self::generate_string_value(string)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Integer(integer)) => {
                Self::generate_integer_value(integer)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Number(number)) => {
                Self::generate_number_value(number)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(boolean)) => boolean
                .enumeration
                .iter()
                .flatten()
                .next()
                .copied()
                .map(Value::Bool)
                .unwrap_or(Value::Bool(true)),
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                // First pass: Scan for pagination metadata (total, page, limit)
                // This helps us generate the correct number of array items
                let mut pagination_metadata: Option<(u64, u64, u64)> = None; // (total, page, limit)

                // Check if this looks like a paginated response by scanning properties
                // Look for "items" array property and pagination fields
                let has_items =
                    obj.properties.iter().any(|(name, _)| name.to_lowercase() == "items");

                if has_items {
                    // Try to extract pagination metadata from schema properties
                    let mut total_opt = None;
                    let mut page_opt = None;
                    let mut limit_opt = None;

                    for (prop_name, prop_schema) in &obj.properties {
                        let prop_lower = prop_name.to_lowercase();
                        // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema> for extraction
                        let schema_ref: ReferenceOr<Schema> = match prop_schema {
                            ReferenceOr::Item(boxed) => ReferenceOr::Item(boxed.as_ref().clone()),
                            ReferenceOr::Reference { reference } => ReferenceOr::Reference {
                                reference: reference.clone(),
                            },
                        };
                        if prop_lower == "total" || prop_lower == "count" || prop_lower == "size" {
                            total_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        } else if prop_lower == "page" {
                            page_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        } else if prop_lower == "limit" || prop_lower == "per_page" {
                            limit_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        }
                    }

                    // If we found a total, use it (with defaults for page/limit)
                    if let Some(total) = total_opt {
                        let page = page_opt.unwrap_or(1);
                        let limit = limit_opt.unwrap_or(20);
                        pagination_metadata = Some((total, page, limit));
                        tracing::debug!(
                            "Detected pagination metadata: total={}, page={}, limit={}",
                            total,
                            page,
                            limit
                        );
                    } else {
                        // Phase 3: If no total found in schema, try to infer from parent entity
                        // Look for "items" array to determine child entity name
                        if obj.properties.contains_key("items") {
                            // Try to infer parent/child relationship from schema names
                            // This is a heuristic: if we're generating a paginated response,
                            // check if we can find a parent entity schema with a count field
                            if let Some(inferred_total) =
                                Self::try_infer_total_from_context(spec, obj)
                            {
                                let page = page_opt.unwrap_or(1);
                                let limit = limit_opt.unwrap_or(20);
                                pagination_metadata = Some((inferred_total, page, limit));
                                tracing::debug!(
                                    "Inferred pagination metadata from parent entity: total={}, page={}, limit={}",
                                    inferred_total, page, limit
                                );
                            } else {
                                // Phase 4: Try to use persona traits if available
                                if let Some(persona) = persona {
                                    // Look for count-related traits (e.g., "hive_count", "apiary_count")
                                    // Try common patterns
                                    let count_keys =
                                        ["hive_count", "apiary_count", "item_count", "total_count"];
                                    for key in &count_keys {
                                        if let Some(count) = persona.get_numeric_trait(key) {
                                            let page = page_opt.unwrap_or(1);
                                            let limit = limit_opt.unwrap_or(20);
                                            pagination_metadata = Some((count, page, limit));
                                            tracing::debug!(
                                                "Using persona trait '{}' for pagination: total={}, page={}, limit={}",
                                                key, count, page, limit
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut map = serde_json::Map::new();
                for (prop_name, prop_schema) in &obj.properties {
                    let prop_lower = prop_name.to_lowercase();

                    // Check if this is an array property that should use pagination metadata
                    let is_items_array = prop_lower == "items" && pagination_metadata.is_some();

                    let value = match prop_schema {
                        ReferenceOr::Item(prop_schema) => {
                            // If this is an items array with pagination metadata, always use generate_array_with_count
                            // (it will use the example as a template if one exists)
                            if is_items_array {
                                // Generate array with count based on pagination metadata
                                Self::generate_array_with_count(
                                    spec,
                                    prop_schema.as_ref(),
                                    pagination_metadata.unwrap(),
                                    persona,
                                )
                            } else if let Some(prop_example) =
                                prop_schema.schema_data.example.as_ref()
                            {
                                // Check for property-level example (only if not items array)
                                tracing::debug!(
                                    "Using example for property '{}': {:?}",
                                    prop_name,
                                    prop_example
                                );
                                prop_example.clone()
                            } else {
                                Self::generate_example_from_schema(
                                    spec,
                                    prop_schema.as_ref(),
                                    persona,
                                )
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            // Try to resolve reference
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                // If this is an items array with pagination metadata, always use generate_array_with_count
                                if is_items_array {
                                    // Generate array with count based on pagination metadata
                                    Self::generate_array_with_count(
                                        spec,
                                        &resolved_schema.schema,
                                        pagination_metadata.unwrap(),
                                        persona,
                                    )
                                } else if let Some(ref_example) =
                                    resolved_schema.schema.schema_data.example.as_ref()
                                {
                                    // Check for example from referenced schema (only if not items array)
                                    tracing::debug!(
                                        "Using example from referenced schema '{}': {:?}",
                                        reference,
                                        ref_example
                                    );
                                    ref_example.clone()
                                } else {
                                    Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    )
                                }
                            } else {
                                Self::generate_example_for_property(prop_name)
                            }
                        }
                    };
                    let value = match value {
                        Value::Null | Value::Object(_)
                            if matches!(&value, Value::Null)
                                || matches!(&value, Value::Object(obj) if obj.is_empty()) =>
                        {
                            // If the property schema indicates an object type, keep it as
                            // an empty object rather than replacing with a string-based example
                            if Self::is_object_typed_property(prop_schema) {
                                Value::Object(serde_json::Map::new())
                            } else {
                                Self::generate_example_for_property(prop_name)
                            }
                        }
                        _ => value,
                    };
                    map.insert(prop_name.clone(), value);
                }

                // Ensure pagination metadata is set if we detected it
                if let Some((total, page, limit)) = pagination_metadata {
                    map.insert("total".to_string(), Value::Number(total.into()));
                    map.insert("page".to_string(), Value::Number(page.into()));
                    map.insert("limit".to_string(), Value::Number(limit.into()));
                }

                Value::Object(map)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
                // Check for array-level example (schema.schema_data.example contains the full array)
                // Note: This check is actually redundant since we check at the top,
                // but keeping it here for clarity and defensive programming
                // If the array schema itself has an example, it's already handled at the top

                match &arr.items {
                    Some(item_schema) => {
                        let example_item = match item_schema {
                            ReferenceOr::Item(item_schema) => {
                                // Recursively generate example for array item
                                // This will check for item-level examples
                                Self::generate_example_from_schema(
                                    spec,
                                    item_schema.as_ref(),
                                    persona,
                                )
                            }
                            ReferenceOr::Reference { reference } => {
                                // Try to resolve reference and generate example
                                // This will check for examples in referenced schema
                                if let Some(resolved_schema) = spec.get_schema(reference) {
                                    Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    )
                                } else {
                                    Value::Object(serde_json::Map::new())
                                }
                            }
                        };
                        Value::Array(vec![example_item])
                    }
                    None => Value::Array(vec![Value::String("item".to_string())]),
                }
            }
            openapiv3::SchemaKind::AllOf { all_of } => all_of
                .iter()
                .find_map(|member| Self::generate_member(spec, member, persona))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            openapiv3::SchemaKind::OneOf { one_of } => one_of
                .first()
                .and_then(|member| Self::generate_member(spec, member, persona))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            openapiv3::SchemaKind::AnyOf { any_of } => any_of
                .first()
                .and_then(|member| Self::generate_member(spec, member, persona))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            _ => Value::Object(serde_json::Map::new()),
        }
    }

    fn generate_member(
        spec: &OpenApiSpec,
        member: &ReferenceOr<Schema>,
        persona: Option<&Persona>,
    ) -> Option<Value> {
        match member {
            ReferenceOr::Item(schema) => {
                Some(Self::generate_example_from_schema(spec, schema, persona))
            }
            ReferenceOr::Reference { reference } => spec
                .get_schema(reference)
                .map(|schema| Self::generate_example_from_schema(spec, &schema.schema, persona)),
        }
    }

    fn generate_string_value(string: &openapiv3::StringType) -> Value {
        if let Some(value) = string.enumeration.iter().flatten().next() {
            return Value::String(value.clone());
        }
        let format = match &string.format {
            openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Date) => Some("date"),
            openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::DateTime) => {
                Some("date-time")
            }
            openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Password) => {
                Some("password")
            }
            openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Byte) => Some("byte"),
            openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Binary) => {
                Some("binary")
            }
            openapiv3::VariantOrUnknownOrEmpty::Unknown(format) => Some(format.as_str()),
            openapiv3::VariantOrUnknownOrEmpty::Empty => None,
        };
        let natural_value = match format.map(str::to_ascii_lowercase).as_deref() {
            Some("date-time" | "datetime") => "2024-01-01T00:00:00Z".to_string(),
            Some("date") => "2024-01-01".to_string(),
            Some("uuid") => "00000000-0000-4000-8000-000000000000".to_string(),
            Some("email") => "user@example.com".to_string(),
            Some("uri" | "url") => "https://example.com/".to_string(),
            Some("ipv4") => "192.0.2.1".to_string(),
            Some("ipv6") => "2001:db8::1".to_string(),
            Some("hostname") => "example.com".to_string(),
            Some("byte") => "ZXhhbXBsZQ==".to_string(),
            Some("password") => "example string".to_string(),
            _ => "example string".to_string(),
        };
        let max = string.max_length.unwrap_or(10_000).min(10_000);
        let min = string.min_length.unwrap_or(0);
        // Bound generated strings to 10,000 characters to avoid untrusted schemas
        // requesting unbounded allocations. Oversized minLength is capped here.
        let target_min = min.min(10_000);
        if natural_value.chars().count() <= max && natural_value.chars().count() >= target_min {
            return Value::String(natural_value);
        }
        if format.is_some()
            && natural_value.chars().count() <= max
            && natural_value.chars().count() < target_min
        {
            // Keep a valid natural format value when it fits; otherwise length validity wins.
        }
        // A formatted value that cannot satisfy the bounds falls back to plain text.
        let desired = target_min.max("example string".len()).min(max);
        let mut plain = "example string".to_string();
        if plain.len() > max {
            plain.truncate(max);
        }
        if plain.len() < desired {
            plain.push_str(&"x".repeat(desired - plain.len()));
        }
        Value::String(plain)
    }

    fn generate_integer_value(integer: &openapiv3::IntegerType) -> Value {
        if let Some(value) = integer.enumeration.iter().flatten().next() {
            return json!(value);
        }
        let lower = integer.minimum.map(|v| {
            if integer.exclusive_minimum {
                v.saturating_add(1)
            } else {
                v
            }
        });
        let upper = integer.maximum.map(|v| {
            if integer.exclusive_maximum {
                v.saturating_sub(1)
            } else {
                v
            }
        });
        let mut value = lower.unwrap_or(42_i64);
        if let Some(max) = upper {
            value = value.min(max);
        }
        if let Some(step) = integer.multiple_of.filter(|step| *step > 0) {
            let rem = value.rem_euclid(step);
            let rounded = if rem == 0 {
                Some(value)
            } else {
                value.checked_add(step - rem)
            };
            value = rounded.unwrap_or(value);
            if upper.is_some_and(|max| value > max) {
                let max = upper.unwrap();
                let candidate = max - max.rem_euclid(step);
                // Unsatisfiable bounds: return the closest in-range value.
                value = if candidate >= lower.unwrap_or(i64::MIN) {
                    candidate
                } else {
                    max.min(value).max(lower.unwrap_or(i64::MIN))
                };
            }
        }
        json!(value)
    }

    fn generate_number_value(number: &openapiv3::NumberType) -> Value {
        if let Some(value) = number.enumeration.iter().flatten().next() {
            return json!(value);
        }
        let lower = number.minimum.map(|v| {
            if number.exclusive_minimum {
                Self::next_up(v)
            } else {
                v
            }
        });
        let upper = number.maximum.map(|v| {
            if number.exclusive_maximum {
                Self::next_down(v)
            } else {
                v
            }
        });
        let mut value = match (lower, upper) {
            (Some(min), Some(max)) => min + (max - min) / 2.0,
            (Some(min), None) => min,
            (None, Some(max)) => max,
            (None, None) => std::f64::consts::PI,
        };
        if let Some(step) = number.multiple_of.filter(|step| *step > 0.0 && step.is_finite()) {
            let decimals = format!("{step}").split('.').nth(1).map_or(0, str::len).min(12);
            let scale = 10_f64.powi(decimals as i32);
            let scaled_step = (step * scale).round();
            if scaled_step > 0.0 && scaled_step <= i64::MAX as f64 {
                value = ((value * scale / scaled_step).ceil() * scaled_step) / scale;
            }
            if let Some(max) = upper {
                if value > max {
                    value = (max / step).floor() * step;
                }
            }
        }
        Value::Number(
            serde_json::Number::from_f64(value)
                .unwrap_or_else(|| json!(0).as_number().unwrap().clone()),
        )
    }

    fn next_up(value: f64) -> f64 {
        if value.is_nan() || value == f64::INFINITY {
            return value;
        }
        if value == 0.0 {
            return f64::from_bits(1);
        }
        f64::from_bits(if value > 0.0 {
            value.to_bits() + 1
        } else {
            value.to_bits() - 1
        })
    }

    fn next_down(value: f64) -> f64 {
        if value.is_nan() || value == f64::NEG_INFINITY {
            return value;
        }
        if value == 0.0 {
            return -f64::from_bits(1);
        }
        f64::from_bits(if value > 0.0 {
            value.to_bits() - 1
        } else {
            value.to_bits() + 1
        })
    }

    /// Extract numeric value from a schema (from example or default)
    /// Returns None if no numeric value can be extracted
    pub(crate) fn extract_numeric_value_from_schema(
        schema_ref: &ReferenceOr<Schema>,
    ) -> Option<u64> {
        match schema_ref {
            ReferenceOr::Item(schema) => {
                // Check for example value first
                if let Some(example) = schema.schema_data.example.as_ref() {
                    if let Some(num) = example.as_u64() {
                        return Some(num);
                    } else if let Some(num) = example.as_f64() {
                        return Some(num as u64);
                    }
                }
                // Check for default value
                if let Some(default) = schema.schema_data.default.as_ref() {
                    if let Some(num) = default.as_u64() {
                        return Some(num);
                    } else if let Some(num) = default.as_f64() {
                        return Some(num as u64);
                    }
                }
                // For integer types, try to extract from schema constraints
                // Note: IntegerType doesn't have a default field in openapiv3
                // Defaults are stored in schema_data.default instead
                None
            }
            ReferenceOr::Reference { reference: _ } => {
                // For references, we'd need to resolve them, but for now return None
                // This can be enhanced later if needed
                None
            }
        }
    }

    /// Generate an array with a specific count based on pagination metadata
    /// Respects the limit (e.g., if total=50 and limit=20, generates 20 items)
    pub(crate) fn generate_array_with_count(
        spec: &OpenApiSpec,
        array_schema: &Schema,
        pagination: (u64, u64, u64), // (total, page, limit)
        persona: Option<&Persona>,
    ) -> Value {
        let (total, _page, limit) = pagination;

        // Determine how many items to generate
        // Respect pagination: generate min(total, limit) items
        let count = std::cmp::min(total, limit);

        // Cap at reasonable maximum to avoid performance issues
        let max_items = 100;
        let count = std::cmp::min(count, max_items);

        tracing::debug!("Generating array with count={} (total={}, limit={})", count, total, limit);

        // Check if array schema has an example with items
        if let Some(example) = array_schema.schema_data.example.as_ref() {
            if let Some(example_array) = example.as_array() {
                if !example_array.is_empty() {
                    // Use first example item as template
                    let template_item = &example_array[0];
                    let items: Vec<Value> = (0..count)
                        .map(|i| {
                            // Clone template and add variation
                            let mut item = template_item.clone();
                            Self::add_item_variation(&mut item, i + 1);
                            item
                        })
                        .collect();
                    return Value::Array(items);
                }
            }
        }

        // Generate items from schema
        if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) = &array_schema.schema_kind
        {
            if let Some(item_schema) = &arr.items {
                let items: Vec<Value> = match item_schema {
                    ReferenceOr::Item(item_schema) => {
                        (0..count)
                            .map(|i| {
                                let mut item = Self::generate_example_from_schema(
                                    spec,
                                    item_schema.as_ref(),
                                    persona,
                                );
                                // Add variation to make items unique
                                Self::add_item_variation_with_schema(
                                    &mut item,
                                    i + 1,
                                    Some(item_schema.as_ref()),
                                );
                                item
                            })
                            .collect()
                    }
                    ReferenceOr::Reference { reference } => {
                        if let Some(resolved_schema) = spec.get_schema(reference) {
                            (0..count)
                                .map(|i| {
                                    let mut item = Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    );
                                    // Add variation to make items unique
                                    Self::add_item_variation_with_schema(
                                        &mut item,
                                        i + 1,
                                        Some(&resolved_schema.schema),
                                    );
                                    item
                                })
                                .collect()
                        } else {
                            vec![Value::Object(serde_json::Map::new()); count as usize]
                        }
                    }
                };
                return Value::Array(items);
            }
        }

        // Fallback: generate simple items
        Value::Array((0..count).map(|i| Value::String(format!("item_{}", i + 1))).collect())
    }

    /// Add variation to an item to make it unique (for array generation)
    /// Varies IDs, names, addresses, and coordinates based on item index
    pub(crate) fn add_item_variation(item: &mut Value, item_index: u64) {
        Self::add_item_variation_with_schema(item, item_index, None);
    }

    fn add_item_variation_with_schema(item: &mut Value, item_index: u64, schema: Option<&Schema>) {
        if let Some(obj) = item.as_object_mut() {
            // Update ID fields to be unique
            if let Some(id_val) = obj.get_mut("id") {
                if let Some(id_str) = id_val.as_str() {
                    // Generated format/enum identifiers must remain schema-valid.
                    let constrained_id = schema
                        .and_then(|schema| match &schema.schema_kind {
                            openapiv3::SchemaKind::Type(openapiv3::Type::Object(object)) => {
                                object.properties.get("id")
                            }
                            _ => None,
                        })
                        .and_then(|property| match property {
                            ReferenceOr::Item(property) => Some(property.as_ref()),
                            ReferenceOr::Reference { .. } => None,
                        })
                        .is_some_and(|property| match &property.schema_kind {
                            openapiv3::SchemaKind::Type(openapiv3::Type::String(string)) => {
                                !string.enumeration.is_empty()
                                    || !matches!(
                                        string.format,
                                        openapiv3::VariantOrUnknownOrEmpty::Empty
                                    )
                                    || string.pattern.is_some()
                                    || string.max_length.is_some_and(|max| id_str.len() + 4 > max)
                            }
                            _ => false,
                        });
                    if constrained_id
                        || uuid::Uuid::parse_str(id_str).is_ok()
                        || id_str.contains('@')
                        || id_str.starts_with("http")
                    {
                        // Preserve naturally valid identifier formats while continuing
                        // to vary names, addresses, and other fields below.
                    } else {
                        // Extract base ID (remove any existing numeric suffix).
                        let base_id = id_str
                            .rsplit_once('_')
                            .filter(|(_, suffix)| {
                                suffix.len() == 3 && suffix.bytes().all(|b| b.is_ascii_digit())
                            })
                            .map_or(id_str, |(base, _)| base);
                        *id_val = Value::String(format!("{}_{:03}", base_id, item_index));
                    }
                } else if let Some(id_num) = id_val.as_u64() {
                    *id_val = Value::Number((id_num + item_index).into());
                }
            }

            // Update name fields - add variation for all names
            if let Some(name_val) = obj.get_mut("name") {
                if let Some(name_str) = name_val.as_str() {
                    if name_str.contains('#') {
                        // Pattern like "Hive #1" -> "Hive #2"
                        *name_val = Value::String(format!("Hive #{}", item_index));
                    } else {
                        // Pattern like "Meadow Apiary" -> use rotation of varied names
                        // 60+ unique apiary names with geographic diversity for realistic demo
                        let apiary_names = [
                            // Midwest/Prairie names
                            "Meadow Apiary",
                            "Prairie Apiary",
                            "Sunset Valley Apiary",
                            "Golden Fields Apiary",
                            "Miller Family Apiary",
                            "Heartland Honey Co.",
                            "Cornfield Apiary",
                            "Harvest Moon Apiary",
                            "Prairie Winds Apiary",
                            "Amber Fields Apiary",
                            // California/Coastal names
                            "Coastal Apiary",
                            "Sunset Coast Apiary",
                            "Pacific Grove Apiary",
                            "Golden Gate Apiary",
                            "Napa Valley Apiary",
                            "Coastal Breeze Apiary",
                            "Pacific Heights Apiary",
                            "Bay Area Apiary",
                            "Sunset Valley Honey Co.",
                            "Coastal Harvest Apiary",
                            // Texas/Ranch names
                            "Lone Star Apiary",
                            "Texas Ranch Apiary",
                            "Big Sky Apiary",
                            "Prairie Rose Apiary",
                            "Hill Country Apiary",
                            "Lone Star Honey Co.",
                            "Texas Pride Apiary",
                            "Wildflower Ranch",
                            "Desert Bloom Apiary",
                            "Cactus Creek Apiary",
                            // Florida/Grove names
                            "Orange Grove Apiary",
                            "Citrus Grove Apiary",
                            "Palm Grove Apiary",
                            "Tropical Breeze Apiary",
                            "Everglades Apiary",
                            "Sunshine State Apiary",
                            "Florida Keys Apiary",
                            "Grove View Apiary",
                            "Tropical Harvest Apiary",
                            "Palm Coast Apiary",
                            // Northeast/Valley names
                            "Mountain View Apiary",
                            "Valley Apiary",
                            "Riverside Apiary",
                            "Hilltop Apiary",
                            "Forest Apiary",
                            "Mountain Apiary",
                            "Lakeside Apiary",
                            "Ridge Apiary",
                            "Brook Apiary",
                            "Hillside Apiary",
                            // Generic/Professional names
                            "Field Apiary",
                            "Creek Apiary",
                            "Woodland Apiary",
                            "Farm Apiary",
                            "Orchard Apiary",
                            "Pasture Apiary",
                            "Green Valley Apiary",
                            "Blue Sky Apiary",
                            "Sweet Honey Apiary",
                            "Nature's Best Apiary",
                            // Business/Commercial names
                            "Premium Honey Co.",
                            "Artisan Apiary",
                            "Heritage Apiary",
                            "Summit Apiary",
                            "Crystal Springs Apiary",
                            "Maple Grove Apiary",
                            "Wildflower Apiary",
                            "Thistle Apiary",
                            "Clover Field Apiary",
                            "Honeycomb Apiary",
                        ];
                        let name_index = (item_index - 1) as usize % apiary_names.len();
                        *name_val = Value::String(apiary_names[name_index].to_string());
                    }
                }
            }

            // Update location/address fields
            if let Some(location_val) = obj.get_mut("location") {
                if let Some(location_obj) = location_val.as_object_mut() {
                    // Update address
                    if let Some(address_val) = location_obj.get_mut("address") {
                        if let Some(address_str) = address_val.as_str() {
                            // Extract street number if present, otherwise add variation
                            if let Some(num_str) = address_str.split_whitespace().next() {
                                if let Ok(num) = num_str.parse::<u64>() {
                                    *address_val =
                                        Value::String(format!("{} Farm Road", num + item_index));
                                } else {
                                    *address_val =
                                        Value::String(format!("{} Farm Road", 100 + item_index));
                                }
                            } else {
                                *address_val =
                                    Value::String(format!("{} Farm Road", 100 + item_index));
                            }
                        }
                    }

                    // Vary coordinates slightly
                    if let Some(lat_val) = location_obj.get_mut("latitude") {
                        if let Some(lat) = lat_val.as_f64() {
                            *lat_val = Value::Number(
                                serde_json::Number::from_f64(lat + (item_index as f64 * 0.01))
                                    .expect("latitude arithmetic produces valid f64"),
                            );
                        }
                    }
                    if let Some(lng_val) = location_obj.get_mut("longitude") {
                        if let Some(lng) = lng_val.as_f64() {
                            *lng_val = Value::Number(
                                serde_json::Number::from_f64(lng + (item_index as f64 * 0.01))
                                    .expect("longitude arithmetic produces valid f64"),
                            );
                        }
                    }
                } else if let Some(address_str) = location_val.as_str() {
                    // Flat address string
                    if let Some(num_str) = address_str.split_whitespace().next() {
                        if let Ok(num) = num_str.parse::<u64>() {
                            *location_val =
                                Value::String(format!("{} Farm Road", num + item_index));
                        } else {
                            *location_val =
                                Value::String(format!("{} Farm Road", 100 + item_index));
                        }
                    }
                }
            }

            // Update address field if it exists at root level
            if let Some(address_val) = obj.get_mut("address") {
                if let Some(address_str) = address_val.as_str() {
                    if let Some(num_str) = address_str.split_whitespace().next() {
                        if let Ok(num) = num_str.parse::<u64>() {
                            *address_val = Value::String(format!("{} Farm Road", num + item_index));
                        } else {
                            *address_val = Value::String(format!("{} Farm Road", 100 + item_index));
                        }
                    }
                }
            }

            // Vary status fields (common enum values)
            if let Some(status_val) = obj.get_mut("status") {
                if status_val.as_str().is_some() {
                    let statuses = [
                        "healthy",
                        "sick",
                        "needs_attention",
                        "quarantined",
                        "active",
                        "inactive",
                    ];
                    let status_index = (item_index - 1) as usize % statuses.len();
                    // Bias towards "healthy" and "active" (70% of items)
                    let final_status = if (item_index - 1) % 10 < 7 {
                        statuses[0] // "healthy" or "active"
                    } else {
                        statuses[status_index]
                    };
                    *status_val = Value::String(final_status.to_string());
                }
            }

            // Vary hive_type fields
            if let Some(hive_type_val) = obj.get_mut("hive_type") {
                if hive_type_val.as_str().is_some() {
                    let hive_types = ["langstroth", "top_bar", "warre", "flow_hive", "national"];
                    let type_index = (item_index - 1) as usize % hive_types.len();
                    *hive_type_val = Value::String(hive_types[type_index].to_string());
                }
            }

            // Vary nested queen breed fields
            if let Some(queen_val) = obj.get_mut("queen") {
                if let Some(queen_obj) = queen_val.as_object_mut() {
                    if let Some(breed_val) = queen_obj.get_mut("breed") {
                        if breed_val.as_str().is_some() {
                            let breeds =
                                ["italian", "carniolan", "russian", "buckfast", "caucasian"];
                            let breed_index = (item_index - 1) as usize % breeds.len();
                            *breed_val = Value::String(breeds[breed_index].to_string());
                        }
                    }
                    // Vary queen age
                    if let Some(age_val) = queen_obj.get_mut("age_days") {
                        if let Some(base_age) = age_val.as_u64() {
                            *age_val = Value::Number((base_age + (item_index * 10) % 200).into());
                        } else if let Some(base_age) = age_val.as_i64() {
                            *age_val =
                                Value::Number((base_age + (item_index as i64 * 10) % 200).into());
                        }
                    }
                    // Vary queen mark color
                    if let Some(color_val) = queen_obj.get_mut("mark_color") {
                        if color_val.as_str().is_some() {
                            let colors = ["yellow", "white", "red", "green", "blue"];
                            let color_index = (item_index - 1) as usize % colors.len();
                            *color_val = Value::String(colors[color_index].to_string());
                        }
                    }
                }
            }

            // Vary description fields if they exist
            if let Some(desc_val) = obj.get_mut("description") {
                if desc_val.as_str().is_some() {
                    let descriptions = [
                        "Production apiary",
                        "Research apiary",
                        "Commercial operation",
                        "Backyard apiary",
                        "Educational apiary",
                    ];
                    let desc_index = (item_index - 1) as usize % descriptions.len();
                    *desc_val = Value::String(descriptions[desc_index].to_string());
                }
            }

            // Vary timestamp fields (created_at, updated_at, timestamp, date) for realistic time-series data
            // Generate timestamps spanning 12-24 months with proper distribution
            let timestamp_fields = [
                "created_at",
                "updated_at",
                "timestamp",
                "date",
                "forecastDate",
                "predictedDate",
            ];
            for field_name in &timestamp_fields {
                if let Some(timestamp_val) = obj.get_mut(*field_name) {
                    if let Some(_timestamp_str) = timestamp_val.as_str() {
                        // Generate realistic timestamp: distribute items over past 12-18 months
                        // Use item_index to create variation (not all same date)
                        let months_ago = 12 + ((item_index - 1) % 6); // Distribute over 6 months (12-18 months ago)
                        let days_offset = (item_index - 1) % 28; // Distribute within month (cap at 28)
                        let hours_offset = ((item_index * 7) % 24) as u8; // Distribute throughout day
                        let minutes_offset = ((item_index * 11) % 60) as u8; // Vary minutes

                        // Calculate timestamp relative to current date (November 2024)
                        // Format: ISO 8601 (e.g., "2024-11-12T14:30:00Z")
                        let base_year = 2024;
                        let base_month = 11;

                        // Calculate target month (going back in time)
                        let target_year = if months_ago >= base_month as u64 {
                            base_year - 1
                        } else {
                            base_year
                        };
                        let target_month = if months_ago >= base_month as u64 {
                            12 - (months_ago - base_month as u64) as u8
                        } else {
                            (base_month as u64 - months_ago) as u8
                        };
                        let target_day = std::cmp::min(28, 1 + days_offset as u8); // Start from day 1, cap at 28

                        // Format as ISO 8601
                        let timestamp = format!(
                            "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
                            target_year, target_month, target_day, hours_offset, minutes_offset
                        );
                        *timestamp_val = Value::String(timestamp);
                    }
                }
            }
        }
    }

    /// Try to infer total count from context (parent entity schemas)
    /// This is a heuristic that looks for common relationship patterns
    pub(crate) fn try_infer_total_from_context(
        spec: &OpenApiSpec,
        obj_type: &openapiv3::ObjectType,
    ) -> Option<u64> {
        // Look for "items" array to determine what we're generating
        if let Some(_items_schema_ref) = obj_type.properties.get("items") {
            // Try to determine child entity name from items schema
            // This is a heuristic: check schema names in the spec
            if let Some(components) = &spec.spec.components {
                let schemas = &components.schemas;
                // Look through all schemas to find potential parent entities
                // that might have count fields matching the items type
                for (schema_name, schema_ref) in schemas {
                    if let ReferenceOr::Item(schema) = schema_ref {
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) =
                            &schema.schema_kind
                        {
                            // Look for count fields that might match
                            for (prop_name, prop_schema) in &obj.properties {
                                let prop_lower = prop_name.to_lowercase();
                                if prop_lower.ends_with("_count") {
                                    // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema>
                                    let schema_ref: ReferenceOr<Schema> = match prop_schema {
                                        ReferenceOr::Item(boxed) => {
                                            ReferenceOr::Item(boxed.as_ref().clone())
                                        }
                                        ReferenceOr::Reference { reference } => {
                                            ReferenceOr::Reference {
                                                reference: reference.clone(),
                                            }
                                        }
                                    };
                                    // Found a count field, try to extract its value
                                    if let Some(count) =
                                        Self::extract_numeric_value_from_schema(&schema_ref)
                                    {
                                        // Use a reasonable default if count is very large
                                        if count > 0 && count <= 1000 {
                                            tracing::debug!(
                                                "Inferred count {} from parent schema {} field {}",
                                                count,
                                                schema_name,
                                                prop_name
                                            );
                                            return Some(count);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Generate example value for a property based on its name
    pub(crate) fn generate_example_for_property(prop_name: &str) -> Value {
        let prop_lower = prop_name.to_lowercase();

        // Generate realistic data based on property name patterns
        if prop_lower.contains("id") || prop_lower.contains("uuid") {
            Value::String(uuid::Uuid::new_v4().to_string())
        } else if prop_lower.contains("email") {
            Value::String(format!("user{}@example.com", rand::rng().random_range(1000..=9999)))
        } else if prop_lower.contains("name") || prop_lower.contains("title") {
            let names = ["John Doe", "Jane Smith", "Bob Johnson", "Alice Brown"];
            Value::String(names[rand::rng().random_range(0..names.len())].to_string())
        } else if prop_lower.contains("phone") || prop_lower.contains("mobile") {
            Value::String(format!("+1-555-{:04}", rand::rng().random_range(1000..=9999)))
        } else if prop_lower.contains("address") || prop_lower.contains("street") {
            let streets = ["123 Main St", "456 Oak Ave", "789 Pine Rd", "321 Elm St"];
            Value::String(streets[rand::rng().random_range(0..streets.len())].to_string())
        } else if prop_lower.contains("city") {
            let cities = ["New York", "London", "Tokyo", "Paris", "Sydney"];
            Value::String(cities[rand::rng().random_range(0..cities.len())].to_string())
        } else if prop_lower.contains("country") {
            let countries = ["USA", "UK", "Japan", "France", "Australia"];
            Value::String(countries[rand::rng().random_range(0..countries.len())].to_string())
        } else if prop_lower.contains("company") || prop_lower.contains("organization") {
            let companies = ["Acme Corp", "Tech Solutions", "Global Inc", "Innovate Ltd"];
            Value::String(companies[rand::rng().random_range(0..companies.len())].to_string())
        } else if prop_lower.contains("url") || prop_lower.contains("website") {
            Value::String("https://example.com".to_string())
        } else if prop_lower.contains("age") {
            Value::Number((18 + rand::rng().random_range(0..60)).into())
        } else if prop_lower.contains("count") || prop_lower.contains("quantity") {
            Value::Number((1 + rand::rng().random_range(0..100)).into())
        } else if prop_lower.contains("price")
            || prop_lower.contains("amount")
            || prop_lower.contains("cost")
        {
            Value::Number(
                serde_json::Number::from_f64(
                    (rand::rng().random::<f64>() * 1000.0 * 100.0).round() / 100.0,
                )
                .expect("rounded price calculation produces valid f64"),
            )
        } else if prop_lower.contains("active")
            || prop_lower.contains("enabled")
            || prop_lower.contains("is_")
        {
            Value::Bool(rand::rng().random_bool(0.5))
        } else if prop_lower.contains("date") || prop_lower.contains("time") {
            Value::String(chrono::Utc::now().to_rfc3339())
        } else if prop_lower.contains("description") || prop_lower.contains("comment") {
            Value::String("This is a sample description text.".to_string())
        } else {
            Value::String(format!("example {}", prop_name))
        }
    }

    /// Check if a property schema indicates an object type.
    /// Used to avoid replacing empty objects with string-based examples
    /// when the schema declares the property as an object.
    pub(crate) fn is_object_typed_property(schema_ref: &ReferenceOr<Box<Schema>>) -> bool {
        match schema_ref {
            ReferenceOr::Item(schema) => matches!(
                &schema.schema_kind,
                openapiv3::SchemaKind::Type(openapiv3::Type::Object(_))
            ),
            // References typically point to named object schemas
            ReferenceOr::Reference { .. } => true,
        }
    }
}

#[cfg(test)]
mod schema_based_tests {
    use super::*;
    use serde_json::json;

    fn spec_from_yaml(yaml: &str) -> OpenApiSpec {
        OpenApiSpec::from_string(yaml, Some("yaml")).expect("valid fixture spec")
    }

    fn component_schema<'a>(spec: &'a OpenApiSpec, name: &str) -> &'a Schema {
        match spec.schemas().expect("components present").get(name) {
            Some(ReferenceOr::Item(schema)) => schema,
            _ => panic!("schema {name} not found as an inline item"),
        }
    }

    /// Spec exercising pagination, examples and name heuristics in one place.
    fn fixture_spec() -> OpenApiSpec {
        spec_from_yaml(
            r#"
openapi: 3.0.3
info:
  title: Hive API
  version: "1.0.0"
paths: {}
components:
  schemas:
    PaginatedHives:
      type: object
      properties:
        total: {type: integer, example: 5}
        page: {type: integer, example: 1}
        limit: {type: integer, example: 20}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    LimitedHives:
      type: object
      properties:
        total: {type: integer, example: 50}
        page: {type: integer, example: 1}
        limit: {type: integer, example: 5}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    TemplatedHives:
      type: object
      properties:
        total: {type: integer, example: 3}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
          example:
            - id: h1
              name: "Hive #1"
    Apiary:
      type: object
      properties:
        hive_count: {type: integer, example: 7}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    HivePage:
      type: object
      properties:
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    Hive:
      type: object
      properties:
        id: {type: string}
        name: {type: string}
    PrimString: {type: string}
    PrimInt: {type: integer}
    PrimNum: {type: number}
    PrimBool: {type: boolean}
    IntList:
      type: array
      items: {type: integer}
    FreeArray: {type: array}
    Exampled:
      type: object
      properties:
        name: {type: string}
      example: {id: 3}
    PropExamples:
      type: object
      properties:
        status: {type: string, example: live}
    EmptyMeta:
      type: object
      properties:
        meta: {type: object, properties: {}}
        email: {$ref: '#/components/schemas/Missing'}
        website: {$ref: '#/components/schemas/Missing'}
        mystery: {$ref: '#/components/schemas/Missing'}
    NotArray: {type: string}
    NumExample: {type: integer, example: 5, default: 9}
    NumDefault: {type: integer, default: 12}
    NumString: {type: integer, example: abc}
    ObjTyped:
      type: object
      properties: {a: {type: string}}
    StrTyped: {type: string}
"#,
        )
    }

    #[test]
    fn constrained_schema_values_are_honored() {
        let spec = spec_from_yaml(
            r#"
openapi: 3.0.3
info: {title: constraints, version: '1'}
paths: {}
components:
  schemas:
    State: {type: string, enum: [active, inactive]}
    DateTime: {type: string, format: date-time}
    Uuid: {type: string, format: uuid}
    Email: {type: string, format: email}
    Bounded: {type: integer, minimum: 10, maximum: 12}
    Multiple: {type: number, multipleOf: 0.5, minimum: 1, maximum: 2}
    States: {type: array, items: {type: string, enum: [active, inactive]}}
    Nested:
      type: object
      properties:
        state: {type: string, enum: [active, inactive]}
"#,
        );
        let generate = |name| {
            ResponseGenerator::generate_example_from_schema(
                &spec,
                component_schema(&spec, name),
                None,
            )
        };
        assert_eq!(generate("State"), json!("active"));
        assert!(
            chrono::DateTime::parse_from_rfc3339(generate("DateTime").as_str().unwrap()).is_ok()
        );
        assert!(uuid::Uuid::parse_str(generate("Uuid").as_str().unwrap()).is_ok());
        assert!(generate("Email").as_str().unwrap().contains('@'));
        assert_eq!(generate("Bounded"), json!(10));
        assert_eq!(generate("Multiple"), json!(1.5));
        assert_eq!(generate("States"), json!(["active"]));
        assert_eq!(generate("Nested"), json!({"state": "active"}));
    }

    // ─── Type-level generation ──────────────────────────────────────────────

    #[test]
    fn schema_level_example_takes_priority() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "Exampled"),
            None,
        );
        // The schema-level example replaces all generation, including the
        // `name` property that would otherwise be synthesized.
        assert_eq!(value, json!({"id": 3}));
    }

    #[test]
    fn primitive_type_defaults() {
        let spec = fixture_spec();
        assert_eq!(
            ResponseGenerator::generate_example_from_schema(
                &spec,
                component_schema(&spec, "PrimString"),
                None
            ),
            json!("example string")
        );
        assert_eq!(
            ResponseGenerator::generate_example_from_schema(
                &spec,
                component_schema(&spec, "PrimInt"),
                None
            ),
            json!(42)
        );
        assert_eq!(
            ResponseGenerator::generate_example_from_schema(
                &spec,
                component_schema(&spec, "PrimNum"),
                None
            ),
            json!(std::f64::consts::PI)
        );
        assert_eq!(
            ResponseGenerator::generate_example_from_schema(
                &spec,
                component_schema(&spec, "PrimBool"),
                None
            ),
            json!(true)
        );
    }

    #[test]
    fn array_generates_single_item_from_item_schema() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "IntList"),
            None,
        );
        assert_eq!(value, json!([42]));
    }

    #[test]
    fn array_without_items_uses_placeholder_string() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "FreeArray"),
            None,
        );
        assert_eq!(value, json!(["item"]));
    }

    // ─── Object properties ──────────────────────────────────────────────────

    #[test]
    fn property_level_examples_are_used() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "PropExamples"),
            None,
        );
        assert_eq!(value, json!({"status": "live"}));
    }

    #[test]
    fn empty_object_property_stays_empty_object() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "EmptyMeta"),
            None,
        );
        // Object-typed properties are not replaced by name-heuristic strings.
        assert_eq!(value["meta"], json!({}));
    }

    #[test]
    fn unresolvable_reference_falls_back_to_name_heuristics() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "EmptyMeta"),
            None,
        );
        let email = value["email"].as_str().expect("email string");
        assert!(email.starts_with("user") && email.ends_with("@example.com"), "{email}");
        assert_eq!(value["website"], json!("https://example.com"));
        assert_eq!(value["mystery"], json!("example mystery"));
    }

    // ─── Pagination heuristics ──────────────────────────────────────────────

    #[test]
    fn paginated_response_generates_total_items() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "PaginatedHives"),
            None,
        );
        assert_eq!(value["total"], json!(5));
        assert_eq!(value["page"], json!(1));
        assert_eq!(value["limit"], json!(20));
        let items = value["items"].as_array().expect("items array");
        assert_eq!(items.len(), 5);
        // Items are uniquified via add_item_variation.
        assert_eq!(items[0]["id"], json!("example string_001"));
        assert_eq!(items[4]["id"], json!("example string_005"));
    }

    #[test]
    fn items_capped_at_limit() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "LimitedHives"),
            None,
        );
        // total=50, limit=5 → min(total, limit) items.
        assert_eq!(value["items"].as_array().expect("items array").len(), 5);
    }

    #[test]
    fn paginated_array_example_used_as_template() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "TemplatedHives"),
            None,
        );
        let items = value["items"].as_array().expect("items array");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["id"], json!("h1_001"));
        assert_eq!(items[2]["id"], json!("h1_003"));
        assert_eq!(items[2]["name"], json!("Hive #3"));
    }

    #[test]
    fn total_inferred_from_parent_count_field() {
        let spec = fixture_spec();
        // Apiary has no total/page/limit props; the generator falls back to
        // scanning schemas for a *_count field and finds hive_count: 7.
        // Note the scan is spec-global: it inspects every component schema.
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "Apiary"),
            None,
        );
        assert_eq!(value["total"], json!(7));
        assert_eq!(value["items"].as_array().expect("items array").len(), 7);
    }

    #[test]
    fn inference_skips_out_of_range_counts() {
        // Isolated spec: hive_count=5000 exceeds the 1000 sanity cap, and no
        // other schema offers a usable count, so no pagination metadata is
        // injected and the items array keeps its single default item.
        let spec = spec_from_yaml(
            r#"
openapi: 3.0.3
info:
  title: Big Hive API
  version: "1.0.0"
paths: {}
components:
  schemas:
    BigApiary:
      type: object
      properties:
        hive_count: {type: integer, example: 5000}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    Hive:
      type: object
      properties:
        id: {type: string}
"#,
        );
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "BigApiary"),
            None,
        );
        assert!(value.get("total").is_none());
        assert_eq!(value["items"].as_array().expect("items array").len(), 1);
    }

    #[test]
    fn persona_trait_supplies_total_when_schema_has_none() {
        let spec = spec_from_yaml(
            r#"
openapi: 3.0.3
info:
  title: Hive API
  version: "1.0.0"
paths: {}
components:
  schemas:
    HivePage:
      type: object
      properties:
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    Hive:
      type: object
      properties:
        id: {type: string}
"#,
        );
        let persona = Persona {
            name: "beekeeper".to_string(),
            traits: [("hive_count".to_string(), "4".to_string())].into_iter().collect(),
        };
        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "HivePage"),
            Some(&persona),
        );
        assert_eq!(value["total"], json!(4));
        assert_eq!(value["items"].as_array().expect("items array").len(), 4);

        let value = ResponseGenerator::generate_example_from_schema(
            &spec,
            component_schema(&spec, "HivePage"),
            None,
        );
        assert!(value.get("total").is_none());
        assert_eq!(value["items"].as_array().expect("items array").len(), 1);
    }

    #[test]
    fn end_to_end_paginated_response_via_generate_response() {
        let spec = spec_from_yaml(
            r#"
openapi: 3.0.3
info:
  title: Hive API
  version: "1.0.0"
paths:
  /hives:
    get:
      responses:
        '200':
          description: ok
          content:
            application/json:
              schema: {$ref: '#/components/schemas/PaginatedHives'}
components:
  schemas:
    PaginatedHives:
      type: object
      properties:
        total: {type: integer, example: 5}
        page: {type: integer, example: 1}
        limit: {type: integer, example: 20}
        items:
          type: array
          items: {$ref: '#/components/schemas/Hive'}
    Hive:
      type: object
      properties:
        id: {type: string}
"#,
        );
        let path_item = spec
            .spec
            .paths
            .paths
            .get("/hives")
            .and_then(ReferenceOr::as_item)
            .expect("path item");
        let operation = path_item.get.as_ref().expect("GET operation");

        let body =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .expect("generate response");
        assert_eq!(body["total"], json!(5));
        assert_eq!(body["items"].as_array().expect("items array").len(), 5);
        assert_eq!(body["items"][0]["id"], json!("example string_001"));
    }

    // ─── extract_numeric_value_from_schema ──────────────────────────────────

    #[test]
    fn numeric_example_preferred_over_default() {
        let spec = fixture_spec();
        let schema_ref = spec.schemas().unwrap().get("NumExample").unwrap();
        assert_eq!(ResponseGenerator::extract_numeric_value_from_schema(schema_ref), Some(5));
    }

    #[test]
    fn numeric_default_used_without_example() {
        let spec = fixture_spec();
        let schema_ref = spec.schemas().unwrap().get("NumDefault").unwrap();
        assert_eq!(ResponseGenerator::extract_numeric_value_from_schema(schema_ref), Some(12));
    }

    #[test]
    fn non_numeric_example_returns_none() {
        let spec = fixture_spec();
        let schema_ref = spec.schemas().unwrap().get("NumString").unwrap();
        assert_eq!(ResponseGenerator::extract_numeric_value_from_schema(schema_ref), None);
    }

    #[test]
    fn unresolved_reference_returns_none() {
        let reference = ReferenceOr::Reference {
            reference: "#/components/schemas/NumExample".to_string(),
        };
        assert_eq!(ResponseGenerator::extract_numeric_value_from_schema(&reference), None);
    }

    // ─── generate_array_with_count ──────────────────────────────────────────

    #[test]
    fn non_array_schema_falls_back_to_numbered_items() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_array_with_count(
            &spec,
            component_schema(&spec, "NotArray"),
            (3, 1, 10),
            None,
        );
        assert_eq!(value, json!(["item_1", "item_2", "item_3"]));
    }

    #[test]
    fn count_capped_at_100() {
        let spec = fixture_spec();
        let value = ResponseGenerator::generate_array_with_count(
            &spec,
            component_schema(&spec, "NotArray"),
            (200, 1, 500),
            None,
        );
        assert_eq!(value.as_array().expect("array").len(), 100);
    }

    // ─── add_item_variation ─────────────────────────────────────────────────

    #[test]
    fn variation_uniquifies_string_ids() {
        let mut item = json!({"id": "abc"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["id"], json!("abc_002"));
    }

    #[test]
    fn variation_offsets_numeric_ids() {
        let mut item = json!({"id": 10});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["id"], json!(12));
    }

    #[test]
    fn variation_replaces_hash_names() {
        let mut item = json!({"name": "Hive #1"});
        ResponseGenerator::add_item_variation(&mut item, 3);
        assert_eq!(item["name"], json!("Hive #3"));
    }

    #[test]
    fn variation_rotates_plain_names() {
        let mut item = json!({"name": "Meadow Apiary"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["name"], json!("Prairie Apiary"));
    }

    #[test]
    fn variation_rotates_status_with_healthy_bias() {
        // Early items are biased to "healthy".
        let mut item = json!({"status": "original"});
        ResponseGenerator::add_item_variation(&mut item, 1);
        assert_eq!(item["status"], json!("healthy"));

        // Later items rotate through the remaining statuses.
        let mut item = json!({"status": "original"});
        ResponseGenerator::add_item_variation(&mut item, 8);
        assert_eq!(item["status"], json!("sick"));
    }

    #[test]
    fn variation_rewrites_addresses() {
        let mut item = json!({"address": "123 Main St"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["address"], json!("125 Farm Road"));
    }

    #[test]
    fn variation_rotates_hive_types() {
        let mut item = json!({"hive_type": "langstroth"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["hive_type"], json!("top_bar"));
    }

    #[test]
    fn variation_rewrites_timestamp_fields() {
        let mut item = json!({"created_at": "2020-01-01T00:00:00Z"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        let timestamp = item["created_at"].as_str().expect("timestamp string");
        assert!(timestamp.starts_with("2023-"), "{timestamp}");
        assert!(timestamp.ends_with("00Z"), "{timestamp}");
        assert_eq!(timestamp.len(), 20, "{timestamp}");
    }

    #[test]
    fn uuid_id_does_not_skip_name_variation() {
        let mut item = json!({"id":"00000000-0000-4000-8000-000000000000", "name":"Meadow Apiary"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["id"], json!("00000000-0000-4000-8000-000000000000"));
        assert_eq!(item["name"], json!("Prairie Apiary"));
    }

    #[test]
    fn numeric_suffix_preserves_underscores_in_base() {
        let mut item = json!({"id":"some_enum_value"});
        ResponseGenerator::add_item_variation(&mut item, 2);
        assert_eq!(item["id"], json!("some_enum_value_002"));
    }

    #[test]
    fn number_exclusive_bounds_are_finite() {
        let schema =
            |minimum, maximum, exclusive_minimum, exclusive_maximum| openapiv3::NumberType {
                format: openapiv3::VariantOrUnknownOrEmpty::Empty,
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of: None,
                enumeration: vec![],
            };
        for n in [
            schema(Some(-1.0), None, true, false),
            schema(None, Some(0.0), false, true),
            schema(Some(0.0), Some(1.0), true, true),
        ] {
            let v = ResponseGenerator::generate_number_value(&n).as_f64().unwrap();
            assert!(v.is_finite());
        }
    }

    #[test]
    fn formatted_strings_fall_back_to_length_valid_plain_text() {
        let email = openapiv3::StringType {
            format: openapiv3::VariantOrUnknownOrEmpty::Unknown("email".into()),
            max_length: Some(5),
            ..Default::default()
        };
        assert_eq!(ResponseGenerator::generate_string_value(&email).as_str().unwrap().len(), 5);
        let date_time = openapiv3::StringType {
            format: openapiv3::VariantOrUnknownOrEmpty::Unknown("date-time".into()),
            min_length: Some(40),
            ..Default::default()
        };
        assert_eq!(
            ResponseGenerator::generate_string_value(&date_time).as_str().unwrap().len(),
            40
        );
    }

    // ─── generate_example_for_property ──────────────────────────────────────

    #[test]
    fn id_property_generates_uuid() {
        let value = ResponseGenerator::generate_example_for_property("user_id");
        let text = value.as_str().expect("uuid string");
        assert!(uuid::Uuid::parse_str(text).is_ok(), "{text}");
    }

    #[test]
    fn email_property_generates_address() {
        let value = ResponseGenerator::generate_example_for_property("email");
        let text = value.as_str().expect("email string");
        assert!(text.starts_with("user") && text.ends_with("@example.com"), "{text}");
    }

    #[test]
    fn url_property_generates_https_example() {
        assert_eq!(
            ResponseGenerator::generate_example_for_property("website_url"),
            json!("https://example.com")
        );
    }

    #[test]
    fn unknown_property_uses_example_prefix() {
        assert_eq!(
            ResponseGenerator::generate_example_for_property("frobnicator"),
            json!("example frobnicator")
        );
    }

    // ─── is_object_typed_property ───────────────────────────────────────────

    #[test]
    fn object_typed_property_detection() {
        let spec = fixture_spec();
        let object_ref = ReferenceOr::Item(Box::new(component_schema(&spec, "ObjTyped").clone()));
        let string_ref = ReferenceOr::Item(Box::new(component_schema(&spec, "StrTyped").clone()));
        let reference = ReferenceOr::Reference {
            reference: "#/components/schemas/ObjTyped".to_string(),
        };
        assert!(ResponseGenerator::is_object_typed_property(&object_ref));
        assert!(!ResponseGenerator::is_object_typed_property(&string_ref));
        // References are assumed to point at named object schemas.
        assert!(ResponseGenerator::is_object_typed_property(&reference));
    }
}

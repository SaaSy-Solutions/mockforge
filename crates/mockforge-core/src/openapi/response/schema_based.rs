//! Schema-based response generation for MockForge
//!
//! Contains methods for generating mock response data from OpenAPI schemas,
//! including pagination-aware array generation, property-name heuristics,
//! and item variation for realistic mock data.

use super::*;

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
            openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
                // Use faker for string fields based on field name hints
                Value::String("example string".to_string())
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => Value::Number(42.into()),
            openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => Value::Number(
                serde_json::Number::from_f64(std::f64::consts::PI)
                    .expect("PI is a valid f64 value"),
            ),
            openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => Value::Bool(true),
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
            _ => Value::Object(serde_json::Map::new()),
        }
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
                                Self::add_item_variation(&mut item, i + 1);
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
                                    Self::add_item_variation(&mut item, i + 1);
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
        if let Some(obj) = item.as_object_mut() {
            // Update ID fields to be unique
            if let Some(id_val) = obj.get_mut("id") {
                if let Some(id_str) = id_val.as_str() {
                    // Extract base ID (remove any existing suffix)
                    let base_id = id_str.split('_').next().unwrap_or(id_str);
                    *id_val = Value::String(format!("{}_{:03}", base_id, item_index));
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

    /// Infer relationship count from parent entity schema
    /// When generating a child entity list, check if parent entity has a count field
    #[allow(dead_code)]
    pub(crate) fn infer_count_from_parent_schema(
        spec: &OpenApiSpec,
        parent_entity_name: &str,
        child_entity_name: &str,
    ) -> Option<u64> {
        // Look for parent entity schema
        let _parent_schema_name = parent_entity_name.to_string();
        let count_field_name = format!("{}_count", child_entity_name);

        // Try to find the schema
        if let Some(components) = &spec.spec.components {
            let schemas = &components.schemas;
            // Look for parent schema (case-insensitive)
            for (schema_name, schema_ref) in schemas {
                let schema_name_lower = schema_name.to_lowercase();
                if schema_name_lower.contains(&parent_entity_name.to_lowercase()) {
                    if let ReferenceOr::Item(schema) = schema_ref {
                        // Check if this schema has the count field
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) =
                            &schema.schema_kind
                        {
                            for (prop_name, prop_schema) in &obj.properties {
                                if prop_name.to_lowercase() == count_field_name.to_lowercase() {
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
                                    // Extract count value from schema
                                    return Self::extract_numeric_value_from_schema(&schema_ref);
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
            Value::String(format!("user{}@example.com", thread_rng().random_range(1000..=9999)))
        } else if prop_lower.contains("name") || prop_lower.contains("title") {
            let names = ["John Doe", "Jane Smith", "Bob Johnson", "Alice Brown"];
            Value::String(names[thread_rng().random_range(0..names.len())].to_string())
        } else if prop_lower.contains("phone") || prop_lower.contains("mobile") {
            Value::String(format!("+1-555-{:04}", thread_rng().random_range(1000..=9999)))
        } else if prop_lower.contains("address") || prop_lower.contains("street") {
            let streets = ["123 Main St", "456 Oak Ave", "789 Pine Rd", "321 Elm St"];
            Value::String(streets[thread_rng().random_range(0..streets.len())].to_string())
        } else if prop_lower.contains("city") {
            let cities = ["New York", "London", "Tokyo", "Paris", "Sydney"];
            Value::String(cities[thread_rng().random_range(0..cities.len())].to_string())
        } else if prop_lower.contains("country") {
            let countries = ["USA", "UK", "Japan", "France", "Australia"];
            Value::String(countries[thread_rng().random_range(0..countries.len())].to_string())
        } else if prop_lower.contains("company") || prop_lower.contains("organization") {
            let companies = ["Acme Corp", "Tech Solutions", "Global Inc", "Innovate Ltd"];
            Value::String(companies[thread_rng().random_range(0..companies.len())].to_string())
        } else if prop_lower.contains("url") || prop_lower.contains("website") {
            Value::String("https://example.com".to_string())
        } else if prop_lower.contains("age") {
            Value::Number((18 + thread_rng().random_range(0..60)).into())
        } else if prop_lower.contains("count") || prop_lower.contains("quantity") {
            Value::Number((1 + thread_rng().random_range(0..100)).into())
        } else if prop_lower.contains("price")
            || prop_lower.contains("amount")
            || prop_lower.contains("cost")
        {
            Value::Number(
                serde_json::Number::from_f64(
                    (thread_rng().random::<f64>() * 1000.0 * 100.0).round() / 100.0,
                )
                .expect("rounded price calculation produces valid f64"),
            )
        } else if prop_lower.contains("active")
            || prop_lower.contains("enabled")
            || prop_lower.contains("is_")
        {
            Value::Bool(thread_rng().random_bool(0.5))
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

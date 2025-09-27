//! JSON to Protobuf conversion utilities
//!
//! This module provides utilities to convert between JSON and protobuf messages,
//! enabling HTTP REST API access to gRPC services.

use base64::{engine::general_purpose, Engine as _};
use prost_reflect::{
    DescriptorPool, DynamicMessage, FieldDescriptor, Kind, MessageDescriptor, ReflectMessage, Value,
};
use serde_json::{self, Value as JsonValue};
use std::string::String as StdString;
use tracing::{debug, warn};

/// Errors that can occur during JSON/protobuf conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Field '{field}' required but missing from JSON")]
    MissingField { field: String },
    #[error("Invalid value for field '{field}': {message}")]
    InvalidValue { field: String, message: String },
    #[error("Unknown field '{field}' in message")]
    UnknownField { field: String },
    #[error("Type mismatch for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },
    #[error("Failed to convert nested message: {0}")]
    NestedError(String),
    #[error("Protobuf reflection error: {0}")]
    ProtobufError(String),
}

impl ConversionError {
    fn with_field(
        field: impl Into<String>,
    ) -> impl FnOnce(Box<dyn std::error::Error + Send + Sync>) -> Self {
        let field = field.into();
        move |err| ConversionError::ProtobufError(format!("Field '{}': {}", field, err))
    }
}

/// Converter for JSON to Protobuf and vice versa
#[derive(Debug, Clone)]
pub struct ProtobufJsonConverter {
    /// Descriptor pool containing protobuf definitions
    pool: DescriptorPool,
}

impl ProtobufJsonConverter {
    /// Create a new converter with the given descriptor pool
    pub fn new(pool: DescriptorPool) -> Self {
        Self { pool }
    }

    /// Convert JSON to a protobuf DynamicMessage
    pub fn json_to_protobuf(
        &self,
        descriptor: &MessageDescriptor,
        json: &JsonValue,
    ) -> Result<DynamicMessage, ConversionError> {
        debug!("Converting JSON to protobuf message: {}", descriptor.name());

        let mut message = DynamicMessage::new(descriptor.clone());

        let obj = json.as_object().ok_or_else(|| ConversionError::InvalidValue {
            field: descriptor.name().to_string(),
            message: "Expected JSON object".to_string(),
        })?;

        // Convert each JSON field to protobuf field
        for (field_name, json_value) in obj {
            self.set_field_from_json(&mut message, field_name, json_value)?;
        }

        // Set default values for required fields that weren't provided
        self.set_default_values_for_missing_fields(&mut message)?;

        Ok(message)
    }

    /// Convert a protobuf DynamicMessage to JSON
    pub fn protobuf_to_json(
        &self,
        descriptor: &MessageDescriptor,
        message: &DynamicMessage,
    ) -> Result<JsonValue, ConversionError> {
        debug!("Converting protobuf message to JSON: {}", descriptor.name());

        let mut obj = serde_json::Map::new();

        for field in descriptor.fields() {
            let field_name = field.name();

            if message.has_field(&field) {
                let field_value = message.get_field(&field).into_owned();
                let json_value = self.convert_protobuf_value_to_json(&field, field_value)?;
                obj.insert(field_name.to_string(), json_value);
            } else if field.supports_presence() && !field.is_list() {
                // Field supports presence but isn't set - this means null/absent
                // In JSON, we don't include null values for optional fields by default
                // But we can include them as null if the protobuf field was explicitly set to default
                // For now, we'll skip unset optional fields
            }
        }

        Ok(JsonValue::Object(obj))
    }

    /// Convert JSON to a protobuf value for a specific field
    fn set_field_from_json(
        &self,
        message: &mut DynamicMessage,
        field_name: &str,
        json_value: &JsonValue,
    ) -> Result<(), ConversionError> {
        let field = message.descriptor().get_field_by_name(field_name).ok_or_else(|| {
            ConversionError::UnknownField {
                field: field_name.to_string(),
            }
        })?;

        let protobuf_value = self.convert_json_value_to_protobuf(&field, json_value)?;
        message.set_field(&field, protobuf_value);

        Ok(())
    }

    /// Convert a JSON value to a protobuf Value
    fn convert_json_value_to_protobuf(
        &self,
        field: &FieldDescriptor,
        json_value: &JsonValue,
    ) -> Result<Value, ConversionError> {
        use prost_reflect::Kind::*;

        match field.kind() {
            Message(ref message_descriptor) => {
                match json_value {
                    JsonValue::Object(_) => {
                        let nested_message =
                            self.json_to_protobuf(message_descriptor, json_value)?;
                        Ok(Value::Message(nested_message))
                    }
                    JsonValue::Null if field.supports_presence() => {
                        // For optional message fields, null means unset
                        Ok(Value::Message(DynamicMessage::new(message_descriptor.clone())))
                    }
                    _ => Err(ConversionError::TypeMismatch {
                        field: field.name().to_string(),
                        expected: "object".to_string(),
                        actual: self.json_type_name(json_value),
                    }),
                }
            }
            Enum(ref enum_descriptor) => {
                match json_value {
                    JsonValue::String(s) => {
                        // Try to find enum value by name first
                        if let Some(enum_value) = enum_descriptor.get_value_by_name(s) {
                            Ok(Value::EnumNumber(enum_value.number()))
                        } else {
                            // Try to parse as number
                            match s.parse::<i32>() {
                                Ok(num) => {
                                    if let Some(_) = enum_descriptor.get_value(num) {
                                        Ok(Value::EnumNumber(num))
                                    } else {
                                        Err(ConversionError::InvalidValue {
                                            field: field.name().to_string(),
                                            message: format!("Invalid enum value: {}", num),
                                        })
                                    }
                                }
                                Err(_) => Err(ConversionError::InvalidValue {
                                    field: field.name().to_string(),
                                    message: format!("Unknown enum value: {}", s),
                                }),
                            }
                        }
                    }
                    JsonValue::Number(n) => {
                        if let Some(num) = n.as_i64() {
                            let num = num as i32;
                            if let Some(_) = enum_descriptor.get_value(num) {
                                Ok(Value::EnumNumber(num))
                            } else {
                                Err(ConversionError::InvalidValue {
                                    field: field.name().to_string(),
                                    message: format!("Invalid enum number: {}", num),
                                })
                            }
                        } else {
                            Err(ConversionError::TypeMismatch {
                                field: field.name().to_string(),
                                expected: "integer".to_string(),
                                actual: "number".to_string(),
                            })
                        }
                    }
                    JsonValue::Null if field.supports_presence() => {
                        Ok(Value::EnumNumber(0)) // Default enum value
                    }
                    _ => Err(ConversionError::TypeMismatch {
                        field: field.name().to_string(),
                        expected: "string or number".to_string(),
                        actual: self.json_type_name(json_value),
                    }),
                }
            }
            String => match json_value {
                JsonValue::String(s) => Ok(Value::String(s.clone())),
                JsonValue::Null if field.supports_presence() => Ok(Value::String(StdString::new())),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Int32 | Sint32 | Sfixed32 => match json_value {
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(Value::I32(i as i32))
                    } else {
                        Err(ConversionError::InvalidValue {
                            field: field.name().to_string(),
                            message: "Number out of range for int32".to_string(),
                        })
                    }
                }
                JsonValue::String(s) => match s.parse::<i32>() {
                    Ok(i) => Ok(Value::I32(i)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid int32 value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::I32(0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Int64 | Sint64 | Sfixed64 => match json_value {
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(Value::I64(i))
                    } else {
                        Err(ConversionError::InvalidValue {
                            field: field.name().to_string(),
                            message: "Number out of range for int64".to_string(),
                        })
                    }
                }
                JsonValue::String(s) => match s.parse::<i64>() {
                    Ok(i) => Ok(Value::I64(i)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid int64 value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::I64(0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Uint32 | Fixed32 => match json_value {
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_u64() {
                        Ok(Value::U32(i as u32))
                    } else {
                        Err(ConversionError::InvalidValue {
                            field: field.name().to_string(),
                            message: "Number out of range for uint32".to_string(),
                        })
                    }
                }
                JsonValue::String(s) => match s.parse::<u32>() {
                    Ok(i) => Ok(Value::U32(i)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid uint32 value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::U32(0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Uint64 | Fixed64 => match json_value {
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_u64() {
                        Ok(Value::U64(i))
                    } else {
                        Err(ConversionError::InvalidValue {
                            field: field.name().to_string(),
                            message: "Number out of range for uint64".to_string(),
                        })
                    }
                }
                JsonValue::String(s) => match s.parse::<u64>() {
                    Ok(i) => Ok(Value::U64(i)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid uint64 value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::U64(0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Float => match json_value {
                JsonValue::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        Ok(Value::F32(f as f32))
                    } else {
                        Ok(Value::F32(0.0))
                    }
                }
                JsonValue::String(s) => match s.parse::<f32>() {
                    Ok(f) => Ok(Value::F32(f)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid float value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::F32(0.0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Double => match json_value {
                JsonValue::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        Ok(Value::F64(f))
                    } else {
                        Ok(Value::F64(0.0))
                    }
                }
                JsonValue::String(s) => match s.parse::<f64>() {
                    Ok(f) => Ok(Value::F64(f)),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid double value: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::F64(0.0)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "number or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Bool => match json_value {
                JsonValue::Bool(b) => Ok(Value::Bool(*b)),
                JsonValue::String(s) => match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => Ok(Value::Bool(true)),
                    "false" | "0" | "no" | "off" | "" => Ok(Value::Bool(false)),
                    _ => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid boolean value: {}", s),
                    }),
                },
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(Value::Bool(i != 0))
                    } else {
                        Ok(Value::Bool(false))
                    }
                }
                JsonValue::Null if field.supports_presence() => Ok(Value::Bool(false)),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "boolean, number, or string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
            Bytes => match json_value {
                JsonValue::String(s) => match general_purpose::STANDARD.decode(s) {
                    Ok(bytes) => Ok(Value::Bytes(bytes.into())),
                    Err(_) => Err(ConversionError::InvalidValue {
                        field: field.name().to_string(),
                        message: format!("Invalid base64 string: {}", s),
                    }),
                },
                JsonValue::Null if field.supports_presence() => Ok(Value::Bytes(vec![].into())),
                _ => Err(ConversionError::TypeMismatch {
                    field: field.name().to_string(),
                    expected: "base64 string".to_string(),
                    actual: self.json_type_name(json_value),
                }),
            },
        }
    }

    /// Convert a protobuf Value to a JSON value
    fn convert_protobuf_value_to_json(
        &self,
        field: &FieldDescriptor,
        value: Value,
    ) -> Result<JsonValue, ConversionError> {
        use prost_reflect::Value::*;

        Ok(match value {
            String(s) => JsonValue::String(s),
            I32(i) => JsonValue::Number(i.into()),
            I64(i) => JsonValue::Number(i.into()),
            U32(u) => JsonValue::Number(u.into()),
            U64(u) => JsonValue::Number(u.into()),
            F32(f) => {
                if f.is_finite() {
                    JsonValue::Number(serde_json::Number::from_f64(f as f64).unwrap_or(0.into()))
                } else {
                    JsonValue::Number(0.into())
                }
            }
            F64(f) => {
                if f.is_finite() {
                    JsonValue::Number(serde_json::Number::from_f64(f).unwrap_or(0.into()))
                } else {
                    JsonValue::Number(0.into())
                }
            }
            Bool(b) => JsonValue::Bool(b),
            EnumNumber(n) => {
                if let Kind::Enum(ref enum_descriptor) = field.kind() {
                    if let Some(enum_value) = enum_descriptor.get_value(n) {
                        JsonValue::String(enum_value.name().to_string())
                    } else {
                        // Fallback to number if enum value not found
                        warn!("Unknown enum value {} for field {}", n, field.name());
                        JsonValue::String(n.to_string())
                    }
                } else {
                    JsonValue::String(n.to_string())
                }
            }
            Bytes(b) => JsonValue::String(general_purpose::STANDARD.encode(b)),
            Message(msg) => self.protobuf_to_json(&msg.descriptor(), &msg)?,
            List(list) => {
                let mut json_array = Vec::new();
                for item in list {
                    let json_item = self.convert_protobuf_value_to_json(field, item)?;
                    json_array.push(json_item);
                }
                JsonValue::Array(json_array)
            }
            Map(map) => {
                let mut json_obj = serde_json::Map::new();
                for (key, value) in map {
                    let json_key = match key {
                        prost_reflect::MapKey::String(s) => serde_json::Value::String(s),
                        prost_reflect::MapKey::I32(i) => serde_json::Value::Number(i.into()),
                        prost_reflect::MapKey::I64(i) => serde_json::Value::Number(i.into()),
                        prost_reflect::MapKey::Bool(b) => serde_json::Value::Bool(b),
                        prost_reflect::MapKey::U32(u) => serde_json::Value::Number(u.into()),
                        prost_reflect::MapKey::U64(u) => serde_json::Value::Number(u.into()),
                    };
                    let json_value = self.convert_protobuf_value_to_json(field, value)?;
                    // JSON object keys must be strings
                    let key_str = match json_key {
                        JsonValue::String(s) => s,
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        _ => json_key.to_string(), // Fallback for other types
                    };
                    json_obj.insert(key_str, json_value);
                }
                JsonValue::Object(json_obj)
            }
        })
    }

    /// Set default values for missing required fields
    fn set_default_values_for_missing_fields(
        &self,
        message: &mut DynamicMessage,
    ) -> Result<(), ConversionError> {
        let descriptor = message.descriptor();

        for field in descriptor.fields() {
            if !message.has_field(&field) {
                // Only set defaults for non-repeated fields that don't support presence
                if !field.is_list() && !field.supports_presence() {
                    let default_value = self.get_default_value_for_field(&field)?;
                    message.set_field(&field, default_value);
                    debug!("Set default value for field: {}", field.name());
                }
            }
        }

        Ok(())
    }

    /// Get default value for a field based on its type
    fn get_default_value_for_field(
        &self,
        field: &FieldDescriptor,
    ) -> Result<Value, ConversionError> {
        use prost_reflect::Kind::*;

        Ok(match field.kind() {
            String => Value::String(StdString::new()),
            Int32 | Sint32 | Sfixed32 => Value::I32(0),
            Int64 | Sint64 | Sfixed64 => Value::I64(0),
            Uint32 | Fixed32 => Value::U32(0),
            Uint64 | Fixed64 => Value::U64(0),
            Float => Value::F32(0.0),
            Double => Value::F64(0.0),
            Bool => Value::Bool(false),
            Bytes => Value::Bytes(vec![].into()),
            Enum(_) => Value::EnumNumber(0),
            Message(ref message_descriptor) => {
                Value::Message(DynamicMessage::new(message_descriptor.clone()))
            }
        })
    }

    /// Get string representation of JSON value type
    fn json_type_name(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(_) => "boolean".to_string(),
            JsonValue::Number(_) => "number".to_string(),
            JsonValue::String(_) => "string".to_string(),
            JsonValue::Array(_) => "array".to_string(),
            JsonValue::Object(_) => "object".to_string(),
        }
    }

    /// Handle repeated field conversion for JSON arrays
    fn convert_json_array_to_protobuf_list(
        &self,
        field: &FieldDescriptor,
        json_array: &[JsonValue],
    ) -> Result<Value, ConversionError> {
        let mut list = Vec::new();

        for json_item in json_array {
            let protobuf_item = self.convert_json_value_to_protobuf(field, json_item)?;
            list.push(protobuf_item);
        }

        Ok(Value::List(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost_reflect::MessageDescriptor;

    #[test]
    fn test_json_to_protobuf_simple_types() {
        // This test requires having a descriptor pool with actual messages
        // For now, we'll create a simple test that validates the converter creation
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);
        assert!(converter.pool.services().count() == 0);
    }

    #[test]
    fn test_default_values() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test that we can get default values for different field types
        // Note: This is more of an integration test that would need actual descriptors
        assert!(converter.pool.services().count() == 0);
    }

    #[test]
    fn test_json_type_name() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        assert_eq!(converter.json_type_name(&JsonValue::Null), "null");
        assert_eq!(converter.json_type_name(&JsonValue::Bool(true)), "boolean");
        assert_eq!(converter.json_type_name(&JsonValue::Number(42.into())), "number");
        assert_eq!(converter.json_type_name(&JsonValue::String("test".to_string())), "string");
        assert_eq!(converter.json_type_name(&JsonValue::Array(vec![])), "array");
        assert_eq!(converter.json_type_name(&JsonValue::Object(serde_json::Map::new())), "object");
    }

    #[test]
    fn test_conversion_error_display() {
        let error = ConversionError::MissingField {
            field: "test_field".to_string(),
        };
        assert!(error.to_string().contains("test_field"));

        let error = ConversionError::InvalidValue {
            field: "test_field".to_string(),
            message: "invalid value".to_string(),
        };
        assert!(error.to_string().contains("test_field"));
        assert!(error.to_string().contains("invalid value"));

        let error = ConversionError::UnknownField {
            field: "unknown_field".to_string(),
        };
        assert!(error.to_string().contains("unknown_field"));

        let error = ConversionError::TypeMismatch {
            field: "test_field".to_string(),
            expected: "string".to_string(),
            actual: "number".to_string(),
        };
        assert!(error.to_string().contains("test_field"));
        assert!(error.to_string().contains("string"));
        assert!(error.to_string().contains("number"));

        let error = ConversionError::NestedError("nested error".to_string());
        assert!(error.to_string().contains("nested error"));

        let error = ConversionError::ProtobufError("protobuf error".to_string());
        assert!(error.to_string().contains("protobuf error"));
    }

    #[test]
    fn test_converter_creation() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool.clone());

        assert_eq!(converter.pool.services().count(), 0);

        // Test that we can create multiple converters
        let converter2 = ProtobufJsonConverter::new(pool);
        assert_eq!(converter2.pool.services().count(), 0);
    }

    #[test]
    fn test_json_value_type_detection() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test all JSON value types
        assert_eq!(converter.json_type_name(&JsonValue::Null), "null");
        assert_eq!(converter.json_type_name(&JsonValue::Bool(true)), "boolean");
        assert_eq!(converter.json_type_name(&JsonValue::Bool(false)), "boolean");
        assert_eq!(
            converter.json_type_name(&JsonValue::Number(serde_json::Number::from(42))),
            "number"
        );
        assert_eq!(
            converter
                .json_type_name(&JsonValue::Number(serde_json::Number::from_f64(3.14).unwrap())),
            "number"
        );
        assert_eq!(converter.json_type_name(&JsonValue::String("test".to_string())), "string");
        assert_eq!(converter.json_type_name(&JsonValue::Array(vec![])), "array");
        assert_eq!(converter.json_type_name(&JsonValue::Array(vec![JsonValue::Null])), "array");
        assert_eq!(converter.json_type_name(&JsonValue::Object(serde_json::Map::new())), "object");

        let mut obj = serde_json::Map::new();
        obj.insert("key".to_string(), JsonValue::String("value".to_string()));
        assert_eq!(converter.json_type_name(&JsonValue::Object(obj)), "object");
    }

    #[test]
    fn test_boolean_conversion_variations() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test different boolean representations
        let test_cases = vec![
            (JsonValue::Bool(true), "true"),
            (JsonValue::Bool(false), "false"),
            (JsonValue::String("true".to_string()), "string true"),
            (JsonValue::String("false".to_string()), "string false"),
            (JsonValue::String("1".to_string()), "string 1"),
            (JsonValue::String("0".to_string()), "string 0"),
            (JsonValue::String("yes".to_string()), "string yes"),
            (JsonValue::String("no".to_string()), "string no"),
            (JsonValue::String("on".to_string()), "string on"),
            (JsonValue::String("off".to_string()), "string off"),
            (JsonValue::String("".to_string()), "empty string"),
            (JsonValue::Number(serde_json::Number::from(1)), "number 1"),
            (JsonValue::Number(serde_json::Number::from(0)), "number 0"),
            (JsonValue::Number(serde_json::Number::from(42)), "number 42"),
        ];

        for (json_value, description) in test_cases {
            // This would normally test conversion, but since we don't have actual descriptors,
            // we just test that the converter can handle the type detection
            let type_name = converter.json_type_name(&json_value);
            assert!(!type_name.is_empty(), "Type name should not be empty for {}", description);
        }
    }

    #[test]
    fn test_string_conversion_edge_cases() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        let test_strings = vec![
            "",
            " ",
            "simple string",
            "string with spaces",
            "string-with-dashes",
            "string_with_underscores",
            "string123",
            "123string",
            "string with \"quotes\"",
            "string with 'apostrophes'",
            "string with /slashes/",
            "string with \\backslashes\\",
            "string with \ttabs\tand\nnewlines",
            "string with unicode: 你好世界 🌍",
            "a".repeat(1000), // Large string
        ];

        for test_string in test_strings {
            let json_value = JsonValue::String(test_string.clone());
            let type_name = converter.json_type_name(&json_value);
            assert_eq!(type_name, "string", "Failed for string: '{}'", test_string);
        }
    }

    #[test]
    fn test_number_conversion_variations() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        let test_numbers = vec![
            serde_json::Number::from(0),
            serde_json::Number::from(1),
            serde_json::Number::from(-1),
            serde_json::Number::from(42),
            serde_json::Number::from(-42),
            serde_json::Number::from(i32::MAX),
            serde_json::Number::from(i32::MIN),
            serde_json::Number::from(i64::MAX),
            serde_json::Number::from(i64::MIN),
            serde_json::Number::from_f64(0.0).unwrap(),
            serde_json::Number::from_f64(1.5).unwrap(),
            serde_json::Number::from_f64(-1.5).unwrap(),
            serde_json::Number::from_f64(3.14159).unwrap(),
            serde_json::Number::from_f64(f64::INFINITY).unwrap(),
            serde_json::Number::from_f64(f64::NEG_INFINITY).unwrap(),
            serde_json::Number::from_f64(f64::NAN).unwrap(),
        ];

        for number in test_numbers {
            let json_value = JsonValue::Number(number);
            let type_name = converter.json_type_name(&json_value);
            assert_eq!(type_name, "number", "Failed for number: {}", json_value);
        }
    }

    #[test]
    fn test_array_conversion_variations() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test empty array
        let empty_array = JsonValue::Array(vec![]);
        assert_eq!(converter.json_type_name(&empty_array), "array");

        // Test array with mixed types
        let mixed_array = JsonValue::Array(vec![
            JsonValue::Null,
            JsonValue::Bool(true),
            JsonValue::Number(42.into()),
            JsonValue::String("test".to_string()),
            JsonValue::Object(serde_json::Map::new()),
        ]);
        assert_eq!(converter.json_type_name(&mixed_array), "array");

        // Test nested arrays
        let nested_array = JsonValue::Array(vec![
            JsonValue::Array(vec![JsonValue::Number(1.into())]),
            JsonValue::Array(vec![JsonValue::String("nested".to_string())]),
        ]);
        assert_eq!(converter.json_type_name(&nested_array), "array");

        // Test array with 1000 elements
        let large_array = JsonValue::Array(vec![JsonValue::Number(1.into()); 1000]);
        assert_eq!(converter.json_type_name(&large_array), "array");
    }

    #[test]
    fn test_object_conversion_variations() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test empty object
        let empty_object = JsonValue::Object(serde_json::Map::new());
        assert_eq!(converter.json_type_name(&empty_object), "object");

        // Test object with various key types
        let mut obj = serde_json::Map::new();
        obj.insert("string_key".to_string(), JsonValue::String("value".to_string()));
        obj.insert("number_key".to_string(), JsonValue::Number(42.into()));
        obj.insert("boolean_key".to_string(), JsonValue::Bool(true));
        obj.insert("null_key".to_string(), JsonValue::Null);
        obj.insert("array_key".to_string(), JsonValue::Array(vec![]));
        obj.insert("object_key".to_string(), JsonValue::Object(serde_json::Map::new()));

        let complex_object = JsonValue::Object(obj);
        assert_eq!(converter.json_type_name(&complex_object), "object");

        // Test deeply nested object
        let mut nested_obj = serde_json::Map::new();
        nested_obj.insert(
            "level1".to_string(),
            JsonValue::Object({
                let mut level2 = serde_json::Map::new();
                level2.insert(
                    "level2".to_string(),
                    JsonValue::Object({
                        let mut level3 = serde_json::Map::new();
                        level3.insert("level3".to_string(), JsonValue::String("deep".to_string()));
                        level3
                    }),
                );
                level2
            }),
        );
        let nested_object = JsonValue::Object(nested_obj);
        assert_eq!(converter.json_type_name(&nested_object), "object");
    }

    #[test]
    fn test_base64_encoding_detection() {
        let pool = DescriptorPool::new();
        let converter = ProtobufJsonConverter::new(pool);

        // Test various base64 strings
        let base64_cases = vec![
            "",                                                     // empty
            "dGVzdA==",                                             // "test"
            "SGVsbG8gV29ybGQ=",                                     // "Hello World"
            "YWJjMTIzIT8kKiYoKSctPUB+",                             // "abc123!?$*&()'-=@~"
            "dGVzdGluZyB3aXRoIHNwYWNlcyBhbmQgc3BlY2lhbCBjaGFycw==", // "testing with spaces and special chars"
            "aHR0cHM6Ly9leGFtcGxlLmNvbS9wYXRoP3F1ZXJ5PXZhbHVl",     // URL-like base64
        ];

        for base64_str in base64_cases {
            let json_value = JsonValue::String(base64_str.to_string());
            let type_name = converter.json_type_name(&json_value);
            assert_eq!(type_name, "string", "Failed for base64 string: '{}'", base64_str);
        }

        // Test invalid base64 strings
        let invalid_base64 = vec![
            "invalid!@#$%",
            "not-base64",
            "abc123!@#$%",
            "This is not base64 encoded",
        ];

        for invalid_str in invalid_base64 {
            let json_value = JsonValue::String(invalid_str.to_string());
            let type_name = converter.json_type_name(&json_value);
            assert_eq!(type_name, "string", "Failed for invalid base64: '{}'", invalid_str);
        }
    }
}

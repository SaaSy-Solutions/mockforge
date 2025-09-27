//! Smart mock data generator for gRPC services
//!
//! This module provides intelligent mock data generation based on field names,
//! types, and context. It integrates with the mockforge-data faker system
//! to generate realistic test data.

use prost_reflect::{DynamicMessage, FieldDescriptor, Kind, MessageDescriptor, Value};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tracing::debug;

#[cfg(feature = "data-faker")]
use mockforge_data::faker::EnhancedFaker;

// Re-export fake for direct use
#[cfg(feature = "data-faker")]
use fake::{
    faker::name::en::{FirstName, LastName},
    Fake,
};

/// Configuration for smart mock data generation
#[derive(Debug, Clone)]
pub struct SmartMockConfig {
    /// Enable field name-based intelligent generation
    pub field_name_inference: bool,
    /// Enable faker integration for realistic data
    pub use_faker: bool,
    /// Custom field mappings (field_name -> mock_value)
    pub field_overrides: HashMap<String, String>,
    /// Service-specific data generation profiles
    pub service_profiles: HashMap<String, ServiceProfile>,
    /// Maximum recursion depth for nested messages
    pub max_depth: usize,
    /// Deterministic seed for reproducible data generation
    pub seed: Option<u64>,
    /// Whether to use deterministic generation for stable fixtures
    pub deterministic: bool,
}

impl Default for SmartMockConfig {
    fn default() -> Self {
        Self {
            field_name_inference: true,
            use_faker: true,
            field_overrides: HashMap::new(),
            service_profiles: HashMap::new(),
            max_depth: 5,
            seed: None,
            deterministic: false,
        }
    }
}

/// Service-specific data generation profile
#[derive(Debug, Clone)]
pub struct ServiceProfile {
    /// Custom field mappings for this service
    pub field_mappings: HashMap<String, String>,
    /// Realistic data patterns to use
    pub data_patterns: Vec<DataPattern>,
    /// Whether to use sequential numbering for IDs
    pub sequential_ids: bool,
}

/// Data generation patterns
#[derive(Debug, Clone)]
pub enum DataPattern {
    /// User-related data (names, emails, etc.)
    User,
    /// Product/inventory data
    Product,
    /// Financial/transaction data
    Financial,
    /// Address/location data
    Location,
    /// Custom pattern with field mappings
    Custom(HashMap<String, String>),
}

/// Smart mock data generator
pub struct SmartMockGenerator {
    config: SmartMockConfig,
    /// Counter for sequential data generation
    sequence_counter: u64,
    /// Enhanced faker for realistic data generation
    #[cfg(feature = "data-faker")]
    faker: Option<EnhancedFaker>,
    /// Seeded random number generator for deterministic generation
    rng: Option<StdRng>,
}

impl SmartMockGenerator {
    /// Create a new smart mock generator
    pub fn new(config: SmartMockConfig) -> Self {
        #[cfg(feature = "data-faker")]
        let faker = if config.use_faker {
            Some(EnhancedFaker::new())
        } else {
            None
        };

        // Initialize seeded RNG if deterministic mode is enabled
        let rng = if config.deterministic {
            let seed = config.seed.unwrap_or(42); // Default seed if none provided
            Some(StdRng::seed_from_u64(seed))
        } else {
            None
        };

        Self {
            config,
            sequence_counter: 1,
            #[cfg(feature = "data-faker")]
            faker,
            rng,
        }
    }

    /// Create a new deterministic generator with a specific seed
    pub fn new_with_seed(mut config: SmartMockConfig, seed: u64) -> Self {
        config.seed = Some(seed);
        config.deterministic = true;
        Self::new(config)
    }

    /// Reset the generator to its initial state (useful for reproducible tests)
    pub fn reset(&mut self) {
        self.sequence_counter = 1;
        if let Some(seed) = self.config.seed {
            self.rng = Some(StdRng::seed_from_u64(seed));
        }
    }

    /// Generate a deterministic random number
    fn next_random<T>(&mut self) -> T
    where
        rand::distributions::Standard: rand::distributions::Distribution<T>,
    {
        if let Some(ref mut rng) = self.rng {
            rng.gen()
        } else {
            rand::random()
        }
    }

    /// Generate a deterministic random number within a range
    fn next_random_range(&mut self, min: i64, max: i64) -> i64 {
        if let Some(ref mut rng) = self.rng {
            rng.gen_range(min..=max)
        } else {
            rand::thread_rng().gen_range(min..=max)
        }
    }

    /// Generate a mock value for a field with intelligent inference
    pub fn generate_value_for_field(
        &mut self,
        field: &FieldDescriptor,
        service_name: &str,
        method_name: &str,
        depth: usize,
    ) -> Value {
        if depth >= self.config.max_depth {
            return self.generate_depth_limit_value(field);
        }

        let field_name = field.name().to_lowercase();

        debug!(
            "Generating smart mock value for field: {} (type: {:?}) in service: {}, method: {}",
            field.name(),
            field.kind(),
            service_name,
            method_name
        );

        // Check for field overrides first
        if let Some(override_value) = self.config.field_overrides.get(&field_name) {
            return self.parse_override_value(override_value, field);
        }

        // Check service profile mappings
        if let Some(profile) = self.config.service_profiles.get(service_name) {
            if let Some(mapping) = profile.field_mappings.get(&field_name) {
                return self.parse_override_value(mapping, field);
            }
        }

        // Use intelligent field name inference
        if self.config.field_name_inference {
            if let Some(value) = self.infer_value_from_field_name(&field_name, field) {
                return value;
            }
        }

        // Use faker system if available
        #[cfg(feature = "data-faker")]
        if self.config.use_faker && self.faker.is_some() {
            if let Some(value) = self.generate_with_faker_safe(field) {
                return value;
            }
        }

        // Fallback to basic type-based generation
        self.generate_basic_value_for_type(field, depth)
    }

    /// Infer mock value based on field name patterns
    fn infer_value_from_field_name(
        &mut self,
        field_name: &str,
        field: &FieldDescriptor,
    ) -> Option<Value> {
        match field.kind() {
            Kind::String => {
                let mock_value = match field_name {
                    // Identity fields - enhanced with more realistic fallbacks
                    name if name.contains("email") => {
                        // Use faker first, fallback to pattern
                        #[cfg(feature = "data-faker")]
                        if let Some(faker) = &mut self.faker {
                            faker.email()
                        } else {
                            format!("user{}@example.com", self.next_sequence())
                        }
                        #[cfg(not(feature = "data-faker"))]
                        format!("user{}@example.com", self.next_sequence())
                    }
                    name if name.contains("name") && name.contains("first") => {
                        #[cfg(feature = "data-faker")]
                        if let Some(_faker) = &mut self.faker {
                            FirstName().fake()
                        } else {
                            "John".to_string()
                        }
                        #[cfg(not(feature = "data-faker"))]
                        "John".to_string()
                    }
                    name if name.contains("name") && name.contains("last") => {
                        #[cfg(feature = "data-faker")]
                        if let Some(_faker) = &mut self.faker {
                            LastName().fake()
                        } else {
                            "Doe".to_string()
                        }
                        #[cfg(not(feature = "data-faker"))]
                        "Doe".to_string()
                    }
                    name if name.contains("username") => format!("user{}", self.next_sequence()),
                    name if name.contains("full_name") || name.contains("display_name") => {
                        #[cfg(feature = "data-faker")]
                        if let Some(faker) = &mut self.faker {
                            faker.name()
                        } else {
                            "John Doe".to_string()
                        }
                        #[cfg(not(feature = "data-faker"))]
                        "John Doe".to_string()
                    }

                    // Contact information
                    name if name.contains("phone") => "+1-555-123-4567".to_string(),
                    name if name.contains("address") => "123 Main Street".to_string(),
                    name if name.contains("city") => "San Francisco".to_string(),
                    name if name.contains("state") || name.contains("region") => {
                        "California".to_string()
                    }
                    name if name.contains("country") => "United States".to_string(),
                    name if name.contains("zip") || name.contains("postal") => "94102".to_string(),

                    // Product/business fields
                    name if name.contains("title") => "Sample Product".to_string(),
                    name if name.contains("description") => {
                        "This is a sample product description".to_string()
                    }
                    name if name.contains("sku") || name.contains("product_id") => {
                        format!("SKU{:06}", self.next_sequence())
                    }
                    name if name.contains("category") => "Electronics".to_string(),
                    name if name.contains("brand") => "MockForge".to_string(),

                    // Technical fields
                    name if name.contains("url") || name.contains("link") => {
                        "https://example.com".to_string()
                    }
                    name if name.contains("token") => {
                        format!("token_{}", self.generate_random_string(16))
                    }
                    name if name.contains("uuid") || name.contains("guid") => self.generate_uuid(),
                    name if name.contains("hash") => self.generate_random_string(32),
                    name if name.contains("version") => "1.0.0".to_string(),
                    name if name.contains("status") => "active".to_string(),

                    // Default patterns
                    _ => return None,
                };
                Some(Value::String(mock_value))
            }
            Kind::Int32 | Kind::Int64 => {
                let mock_value = match field_name {
                    // ID fields
                    name if name.contains("id") || name.contains("identifier") => {
                        self.next_sequence()
                    }

                    // Quantity/count fields
                    name if name.contains("count") || name.contains("quantity") => {
                        (self.next_random::<u32>() % 100 + 1) as u64
                    }
                    name if name.contains("age") => (self.next_random::<u32>() % 80 + 18) as u64,
                    name if name.contains("year") => (self.next_random::<u32>() % 30 + 1995) as u64,

                    // Size/dimension fields
                    name if name.contains("width")
                        || name.contains("height")
                        || name.contains("length") =>
                    {
                        (self.next_random::<u32>() % 1000 + 100) as u64
                    }
                    name if name.contains("weight") => {
                        (self.next_random::<u32>() % 1000 + 1) as u64
                    }

                    _ => return None,
                };

                Some(match field.kind() {
                    Kind::Int32 => Value::I32(mock_value as i32),
                    Kind::Int64 => Value::I64(mock_value as i64),
                    _ => unreachable!(),
                })
            }
            Kind::Double | Kind::Float => {
                let mock_value = match field_name {
                    name if name.contains("price") || name.contains("cost") => {
                        self.next_random::<f64>() * 1000.0 + 10.0
                    }
                    name if name.contains("rate") || name.contains("percentage") => {
                        self.next_random::<f64>() * 100.0
                    }
                    name if name.contains("latitude") => self.next_random::<f64>() * 180.0 - 90.0,
                    name if name.contains("longitude") => self.next_random::<f64>() * 360.0 - 180.0,
                    _ => return None,
                };

                Some(match field.kind() {
                    Kind::Double => Value::F64(mock_value),
                    Kind::Float => Value::F32(mock_value as f32),
                    _ => unreachable!(),
                })
            }
            Kind::Bool => {
                let mock_value = match field_name {
                    name if name.contains("active") || name.contains("enabled") => true,
                    name if name.contains("verified") || name.contains("confirmed") => true,
                    name if name.contains("deleted") || name.contains("archived") => false,
                    _ => self.next_random::<bool>(),
                };
                Some(Value::Bool(mock_value))
            }
            _ => None,
        }
    }

    /// Generate value using the enhanced faker system (safe version)
    #[cfg(feature = "data-faker")]
    fn generate_with_faker_safe(&mut self, field: &FieldDescriptor) -> Option<Value> {
        let faker = self.faker.as_mut()?;
        let field_name = field.name().to_lowercase();

        match field.kind() {
            Kind::String => {
                let fake_data = match field_name.as_str() {
                    // Email patterns
                    name if name.contains("email") => faker.email(),

                    // Name patterns
                    name if name.contains("first") && name.contains("name") => FirstName().fake(),
                    name if name.contains("last") && name.contains("name") => LastName().fake(),
                    name if name.contains("name")
                        && !name.contains("file")
                        && !name.contains("path") =>
                    {
                        faker.name()
                    }

                    // Contact patterns
                    name if name.contains("phone") || name.contains("mobile") => faker.phone(),
                    name if name.contains("address") => faker.address(),

                    // Company/Organization patterns
                    name if name.contains("company") || name.contains("organization") => {
                        faker.company()
                    }

                    // Web/Internet patterns
                    name if name.contains("url") || name.contains("website") => faker.url(),
                    name if name.contains("ip") => faker.ip_address(),

                    // ID/UUID patterns
                    name if name.contains("uuid") || name.contains("guid") => faker.uuid(),

                    // Date patterns
                    name if name.contains("date")
                        || name.contains("time")
                        || name.contains("created")
                        || name.contains("updated") =>
                    {
                        faker.date_iso()
                    }

                    // Color patterns
                    name if name.contains("color") || name.contains("colour") => faker.color(),

                    // Default: use the field name inference for other patterns
                    _ => return None,
                };

                Some(Value::String(fake_data))
            }

            Kind::Int32 | Kind::Int64 => {
                let fake_value = match field_name.as_str() {
                    // Age patterns
                    name if name.contains("age") => faker.int_range(18, 90),

                    // Year patterns
                    name if name.contains("year") => faker.int_range(1990, 2024),

                    // Count/quantity patterns
                    name if name.contains("count")
                        || name.contains("quantity")
                        || name.contains("amount") =>
                    {
                        faker.int_range(1, 1000)
                    }

                    // Port numbers
                    name if name.contains("port") => faker.int_range(1024, 65535),

                    // Default: use sequence for IDs or random for others
                    name if name.contains("id") || name.contains("identifier") => {
                        self.next_sequence() as i64
                    }
                    _ => faker.int_range(1, 100),
                };

                Some(match field.kind() {
                    Kind::Int32 => Value::I32(fake_value as i32),
                    Kind::Int64 => Value::I64(fake_value),
                    _ => unreachable!(),
                })
            }

            Kind::Double | Kind::Float => {
                let fake_value = match field_name.as_str() {
                    // Price/money patterns
                    name if name.contains("price")
                        || name.contains("cost")
                        || name.contains("amount") =>
                    {
                        faker.float_range(1.0, 1000.0)
                    }

                    // Percentage/rate patterns
                    name if name.contains("rate") || name.contains("percent") => {
                        faker.float_range(0.0, 100.0)
                    }

                    // Geographic coordinates
                    name if name.contains("latitude") || name.contains("lat") => {
                        faker.float_range(-90.0, 90.0)
                    }
                    name if name.contains("longitude")
                        || name.contains("lng")
                        || name.contains("lon") =>
                    {
                        faker.float_range(-180.0, 180.0)
                    }

                    // Default random float
                    _ => faker.float_range(0.0, 100.0),
                };

                Some(match field.kind() {
                    Kind::Double => Value::F64(fake_value),
                    Kind::Float => Value::F32(fake_value as f32),
                    _ => unreachable!(),
                })
            }

            Kind::Bool => {
                let probability = match field_name.as_str() {
                    // Usually true patterns
                    name if name.contains("active")
                        || name.contains("enabled")
                        || name.contains("verified") =>
                    {
                        0.8
                    }

                    // Usually false patterns
                    name if name.contains("deleted")
                        || name.contains("archived")
                        || name.contains("disabled") =>
                    {
                        0.2
                    }

                    // Default 50/50
                    _ => 0.5,
                };

                Some(Value::Bool(faker.boolean(probability)))
            }

            _ => None,
        }
    }

    #[cfg(not(feature = "data-faker"))]
    fn generate_with_faker_safe(&mut self, _field: &FieldDescriptor) -> Option<Value> {
        None
    }

    /// Generate basic value based on type
    fn generate_basic_value_for_type(&mut self, field: &FieldDescriptor, depth: usize) -> Value {
        match field.kind() {
            Kind::String => Value::String(format!("mock_{}", field.name())),
            Kind::Int32 => Value::I32(self.next_sequence() as i32),
            Kind::Int64 => Value::I64(self.next_sequence() as i64),
            Kind::Uint32 => Value::U32(self.next_sequence() as u32),
            Kind::Uint64 => Value::U64(self.next_sequence()),
            Kind::Sint32 => Value::I32(self.next_sequence() as i32),
            Kind::Sint64 => Value::I64(self.next_sequence() as i64),
            Kind::Fixed32 => Value::U32(self.next_sequence() as u32),
            Kind::Fixed64 => Value::U64(self.next_sequence()),
            Kind::Sfixed32 => Value::I32(self.next_sequence() as i32),
            Kind::Sfixed64 => Value::I64(self.next_sequence() as i64),
            Kind::Bool => Value::Bool(self.next_sequence() % 2 == 0),
            Kind::Double => Value::F64(self.next_random::<f64>() * 100.0),
            Kind::Float => Value::F32(self.next_random::<f32>() * 100.0),
            Kind::Bytes => {
                Value::Bytes(format!("bytes_{}", self.next_sequence()).into_bytes().into())
            }
            Kind::Enum(enum_descriptor) => {
                // Use the first enum value, or 0 if no values defined
                if let Some(first_value) = enum_descriptor.values().next() {
                    Value::EnumNumber(first_value.number())
                } else {
                    Value::EnumNumber(0)
                }
            }
            Kind::Message(message_descriptor) => {
                self.generate_mock_message(&message_descriptor, depth + 1)
            }
        }
    }

    /// Generate a mock message with all fields populated
    fn generate_mock_message(&mut self, descriptor: &MessageDescriptor, depth: usize) -> Value {
        let mut message = prost_reflect::DynamicMessage::new(descriptor.clone());

        if depth >= self.config.max_depth {
            return Value::Message(message);
        }

        for field in descriptor.fields() {
            let value = self.generate_basic_value_for_type(&field, depth);
            message.set_field(&field, value);
        }

        Value::Message(message)
    }

    /// Generate value when depth limit is reached
    fn generate_depth_limit_value(&self, field: &FieldDescriptor) -> Value {
        match field.kind() {
            Kind::String => Value::String(format!("depth_limit_reached_{}", field.name())),
            Kind::Int32 => Value::I32(999),
            Kind::Int64 => Value::I64(999),
            _ => Value::String("depth_limit".to_string()),
        }
    }

    /// Parse override value from string
    fn parse_override_value(&self, override_value: &str, field: &FieldDescriptor) -> Value {
        match field.kind() {
            Kind::String => Value::String(override_value.to_string()),
            Kind::Int32 => Value::I32(override_value.parse().unwrap_or(0)),
            Kind::Int64 => Value::I64(override_value.parse().unwrap_or(0)),
            Kind::Bool => Value::Bool(override_value.parse().unwrap_or(false)),
            Kind::Double => Value::F64(override_value.parse().unwrap_or(0.0)),
            Kind::Float => Value::F32(override_value.parse().unwrap_or(0.0)),
            _ => Value::String(override_value.to_string()),
        }
    }

    /// Get next sequence number
    pub fn next_sequence(&mut self) -> u64 {
        let current = self.sequence_counter;
        self.sequence_counter += 1;
        current
    }

    /// Generate a mock message for the given descriptor
    pub fn generate_message(&mut self, descriptor: &MessageDescriptor) -> DynamicMessage {
        match self.generate_mock_message(descriptor, 0) {
            Value::Message(msg) => msg,
            _ => panic!("generate_mock_message should always return a Message Value"),
        }
    }

    /// Generate random string
    pub fn generate_random_string(&mut self, length: usize) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        if let Some(ref mut rng) = self.rng {
            rng.sample_iter(&Alphanumeric).take(length).map(char::from).collect()
        } else {
            thread_rng().sample_iter(&Alphanumeric).take(length).map(char::from).collect()
        }
    }

    /// Generate a UUID-like string
    pub fn generate_uuid(&mut self) -> String {
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:12x}",
            self.next_random::<u32>(),
            self.next_random::<u16>(),
            self.next_random::<u16>(),
            self.next_random::<u16>(),
            self.next_random::<u64>() & 0xffffffffffff,
        )
    }

    /// Get configuration for external inspection
    pub fn config(&self) -> &SmartMockConfig {
        &self.config
    }

    /// Check if faker is enabled and available
    #[cfg(feature = "data-faker")]
    pub fn is_faker_enabled(&self) -> bool {
        self.config.use_faker && self.faker.is_some()
    }

    #[cfg(not(feature = "data-faker"))]
    pub fn is_faker_enabled(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_mock_generator() {
        let config = SmartMockConfig::default();
        let mut generator = SmartMockGenerator::new(config);

        // Test sequence generation
        assert_eq!(generator.next_sequence(), 1);
        assert_eq!(generator.next_sequence(), 2);

        // Test UUID generation
        let uuid = generator.generate_uuid();
        assert!(uuid.contains('-'));
        assert_eq!(uuid.matches('-').count(), 4);
    }

    #[test]
    fn test_field_name_inference() {
        let config = SmartMockConfig::default();
        let mut generator = SmartMockGenerator::new(config);

        // Test email inference - we'd need actual field descriptors for full testing
        // This is a unit test placeholder
        assert!(generator.generate_random_string(10).len() == 10);
    }

    #[test]
    fn test_deterministic_seeding() {
        // Create two generators with the same seed
        let config1 = SmartMockConfig {
            seed: Some(12345),
            deterministic: true,
            ..Default::default()
        };
        let config2 = SmartMockConfig {
            seed: Some(12345),
            deterministic: true,
            ..Default::default()
        };

        let mut gen1 = SmartMockGenerator::new(config1);
        let mut gen2 = SmartMockGenerator::new(config2);

        // Generate same values with same seed
        assert_eq!(gen1.generate_uuid(), gen2.generate_uuid());
        assert_eq!(gen1.generate_random_string(10), gen2.generate_random_string(10));

        // Test that different seeds produce different values
        let config3 = SmartMockConfig {
            seed: Some(54321),
            deterministic: true,
            ..Default::default()
        };
        let mut gen3 = SmartMockGenerator::new(config3);

        // Reset first generator
        gen1.reset();
        gen3.reset();

        // Should produce different values with different seeds
        assert_ne!(gen1.generate_uuid(), gen3.generate_uuid());
    }

    #[test]
    fn test_new_with_seed() {
        let config = SmartMockConfig::default();
        let mut gen1 = SmartMockGenerator::new_with_seed(config.clone(), 999);
        let mut gen2 = SmartMockGenerator::new_with_seed(config, 999);

        // Both should be deterministic and produce same results
        assert!(gen1.config.deterministic);
        assert!(gen2.config.deterministic);
        assert_eq!(gen1.config.seed, Some(999));
        assert_eq!(gen2.config.seed, Some(999));

        let uuid1 = gen1.generate_uuid();
        let uuid2 = gen2.generate_uuid();
        assert_eq!(uuid1, uuid2);
    }

    #[test]
    fn test_generator_reset() {
        let mut config = SmartMockConfig::default();
        config.seed = Some(777);
        config.deterministic = true;

        let mut generator = SmartMockGenerator::new(config);

        // Generate some values
        let uuid1 = generator.generate_uuid();
        let seq1 = generator.next_sequence();
        let seq2 = generator.next_sequence();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);

        // Reset and verify we get same values again
        generator.reset();
        let uuid2 = generator.generate_uuid();
        let seq3 = generator.next_sequence();
        let seq4 = generator.next_sequence();

        assert_eq!(uuid1, uuid2); // Same UUID after reset
        assert_eq!(seq3, 1); // Sequence counter reset
        assert_eq!(seq4, 2);
    }

    #[test]
    fn test_deterministic_vs_non_deterministic() {
        // Non-deterministic generator
        let mut gen_random = SmartMockGenerator::new(SmartMockConfig::default());

        // Deterministic generator
        let mut gen_deterministic =
            SmartMockGenerator::new_with_seed(SmartMockConfig::default(), 42);

        // Generate multiple UUIDs - deterministic should be repeatable
        let det_uuid1 = gen_deterministic.generate_uuid();
        let det_uuid2 = gen_deterministic.generate_uuid();

        gen_deterministic.reset();
        let det_uuid1_repeat = gen_deterministic.generate_uuid();
        let det_uuid2_repeat = gen_deterministic.generate_uuid();

        // Deterministic should be the same after reset
        assert_eq!(det_uuid1, det_uuid1_repeat);
        assert_eq!(det_uuid2, det_uuid2_repeat);

        // Just verify random generator works (can't test randomness reliably)
        let _random_uuid = gen_random.generate_uuid();
    }

    #[cfg(feature = "data-faker")]
    #[test]
    fn test_faker_integration() {
        let mut config = SmartMockConfig::default();
        config.use_faker = true;
        let mut generator = SmartMockGenerator::new(config);

        // Test that faker is initialized
        assert!(generator.faker.is_some());

        // Test sequence generation still works
        assert_eq!(generator.next_sequence(), 1);
        assert_eq!(generator.next_sequence(), 2);
    }
}

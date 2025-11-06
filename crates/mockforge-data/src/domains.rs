//! Domain-specific data generators
//!
//! This module provides specialized data generators for various domains
//! including finance, IoT, healthcare, and more.

use mockforge_core::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;

/// Domain type for specialized data generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    /// Financial data (transactions, accounts, currencies)
    Finance,
    /// Internet of Things data (sensors, devices, telemetry)
    Iot,
    /// Healthcare data (patients, diagnoses, medications)
    Healthcare,
    /// E-commerce data (products, orders, customers)
    Ecommerce,
    /// Social media data (users, posts, comments)
    Social,
    /// General purpose domain
    General,
}

impl Domain {
    /// Parse domain from string (deprecated - use FromStr trait)
    #[deprecated(
        since = "0.1.4",
        note = "Use str::parse() or FromStr::from_str() instead"
    )]
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Get domain name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Finance => "finance",
            Self::Iot => "iot",
            Self::Healthcare => "healthcare",
            Self::Ecommerce => "ecommerce",
            Self::Social => "social",
            Self::General => "general",
        }
    }
}

/// Error type for domain parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDomainError {
    invalid_domain: String,
}

impl std::fmt::Display for ParseDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid domain: '{}'. Valid domains are: finance, iot, healthcare, ecommerce, social, general",
            self.invalid_domain
        )
    }
}

impl std::error::Error for ParseDomainError {}

impl FromStr for Domain {
    type Err = ParseDomainError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "finance" => Ok(Self::Finance),
            "iot" => Ok(Self::Iot),
            "healthcare" => Ok(Self::Healthcare),
            "ecommerce" | "e-commerce" => Ok(Self::Ecommerce),
            "social" => Ok(Self::Social),
            "general" => Ok(Self::General),
            _ => Err(ParseDomainError {
                invalid_domain: s.to_string(),
            }),
        }
    }
}

/// Domain-specific data generator
#[derive(Debug)]
pub struct DomainGenerator {
    domain: Domain,
    /// Optional persona traits to influence generation
    persona_traits: Option<std::collections::HashMap<String, String>>,
}

impl DomainGenerator {
    /// Create a new domain generator
    pub fn new(domain: Domain) -> Self {
        Self {
            domain,
            persona_traits: None,
        }
    }

    /// Create a new domain generator with persona traits
    pub fn with_traits(domain: Domain, traits: std::collections::HashMap<String, String>) -> Self {
        Self {
            domain,
            persona_traits: Some(traits),
        }
    }

    /// Set persona traits for this generator
    pub fn set_traits(&mut self, traits: std::collections::HashMap<String, String>) {
        self.persona_traits = Some(traits);
    }

    /// Get persona traits
    pub fn get_trait(&self, name: &str) -> Option<&String> {
        self.persona_traits.as_ref().and_then(|traits| traits.get(name))
    }

    /// Generate data for a specific field type in the domain
    pub fn generate(&self, field_type: &str) -> Result<Value> {
        match self.domain {
            Domain::Finance => self.generate_finance(field_type),
            Domain::Iot => self.generate_iot(field_type),
            Domain::Healthcare => self.generate_healthcare(field_type),
            Domain::Ecommerce => self.generate_ecommerce(field_type),
            Domain::Social => self.generate_social(field_type),
            Domain::General => self.generate_general(field_type),
        }
    }

    /// Generate financial data
    fn generate_finance(&self, field_type: &str) -> Result<Value> {
        let mut rng = rand::rng();

        let value = match field_type {
            "account_number" => json!(format!("ACC{:010}", rng.random_range(0..9999999999i64))),
            "routing_number" => json!(format!("{:09}", rng.random_range(100000000..999999999))),
            "iban" => json!(format!(
                "GB{:02}{:04}{:08}{:08}",
                rng.random_range(10..99),
                rng.random_range(1000..9999),
                rng.random_range(10000000..99999999),
                rng.random_range(10000000..99999999)
            )),
            "swift" | "bic" => json!(format!(
                "{}BANK{}",
                ["GB", "US", "DE", "FR"][rng.random_range(0..4)],
                rng.random_range(100..999)
            )),
            "amount" | "balance" => {
                let amount = rng.random_range(10.0..10000.0);
                json!(format!("{:.2}", amount))
            }
            "currency" => json!(["USD", "EUR", "GBP", "JPY", "CNY"][rng.random_range(0..5)]),
            "transaction_id" => json!(format!("TXN{:016x}", rng.random::<u64>())),
            "card_number" => json!(format!(
                "4{}",
                (0..15).map(|_| rng.random_range(0..10).to_string()).collect::<String>()
            )),
            "cvv" => json!(format!("{:03}", rng.random_range(100..999))),
            "expiry" => {
                let month = rng.random_range(1..=12);
                let year = rng.random_range(25..35);
                json!(format!("{:02}/{:02}", month, year))
            }
            "stock_symbol" => {
                json!(["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA", "META"][rng.random_range(0..6)])
            }
            "price" => json!(rng.random_range(10.0..5000.0)),
            _ => json!(format!("finance_{}", field_type)),
        };

        Ok(value)
    }

    /// Generate IoT data
    fn generate_iot(&self, field_type: &str) -> Result<Value> {
        let mut rng = rand::rng();

        let value = match field_type {
            "device_id" => json!(format!("device-{:08x}", rng.random::<u32>())),
            "sensor_id" => json!(format!("sensor-{:06}", rng.random_range(100000..999999))),
            "temperature" => json!(rng.random_range(-20.0..50.0)),
            "humidity" => json!(rng.random_range(0.0..100.0)),
            "pressure" => json!(rng.random_range(900.0..1100.0)),
            "voltage" => json!(rng.random_range(0.0..5.0)),
            "current" => json!(rng.random_range(0.0..10.0)),
            "power" => json!(rng.random_range(0.0..1000.0)),
            "rssi" | "signal_strength" => json!(rng.random_range(-90..-30)),
            "battery_level" => json!(rng.random_range(0..=100)),
            "latitude" => json!(rng.random_range(-90.0..90.0)),
            "longitude" => json!(rng.random_range(-180.0..180.0)),
            "altitude" => json!(rng.random_range(0.0..5000.0)),
            "status" => {
                json!(["online", "offline", "error", "maintenance"][rng.random_range(0..4)])
            }
            "firmware_version" => json!(format!(
                "{}.{}.{}",
                rng.random_range(1..5),
                rng.random_range(0..10),
                rng.random_range(0..100)
            )),
            "mac_address" => json!(format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                rng.random::<u8>(),
                rng.random::<u8>(),
                rng.random::<u8>(),
                rng.random::<u8>(),
                rng.random::<u8>(),
                rng.random::<u8>()
            )),
            "ip_address" => json!(format!(
                "{}.{}.{}.{}",
                rng.random_range(1..255),
                rng.random_range(0..255),
                rng.random_range(0..255),
                rng.random_range(1..255)
            )),
            _ => json!(format!("iot_{}", field_type)),
        };

        Ok(value)
    }

    /// Generate healthcare data
    fn generate_healthcare(&self, field_type: &str) -> Result<Value> {
        let mut rng = rand::rng();

        let value = match field_type {
            "patient_id" => json!(format!("P{:08}", rng.random_range(10000000..99999999))),
            "mrn" | "medical_record_number" => {
                json!(format!("MRN{:010}", rng.random_range(0..9999999999i64)))
            }
            "diagnosis_code" | "icd10" => json!(format!(
                "{}{:02}.{}",
                ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'][rng.random_range(0..10)],
                rng.random_range(0..99),
                rng.random_range(0..9)
            )),
            "procedure_code" | "cpt" => json!(format!("{:05}", rng.random_range(10000..99999))),
            "npi" | "provider_id" => {
                json!(format!("{:010}", rng.random_range(1000000000..9999999999i64)))
            }
            "blood_pressure" => {
                json!(format!("{}/{}", rng.random_range(90..180), rng.random_range(60..120)))
            }
            "heart_rate" | "pulse" => json!(rng.random_range(60..100)),
            "respiratory_rate" => json!(rng.random_range(12..20)),
            "temperature" => json!(format!("{:.1}", rng.random_range(36.0..38.5))),
            "blood_glucose" => json!(rng.random_range(70..140)),
            "oxygen_saturation" => json!(rng.random_range(95..100)),
            "bmi" => json!(format!("{:.1}", rng.random_range(18.0..35.0))),
            "blood_type" => {
                json!(["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"][rng.random_range(0..8)])
            }
            "medication" => json!(
                [
                    "Aspirin",
                    "Ibuprofen",
                    "Metformin",
                    "Lisinopril",
                    "Atorvastatin"
                ][rng.random_range(0..5)]
            ),
            "dosage" => json!(format!("{}mg", [50, 100, 250, 500, 1000][rng.random_range(0..5)])),
            "allergy" => {
                json!(["Penicillin", "Peanuts", "Latex", "Sulfa", "None"][rng.random_range(0..5)])
            }
            _ => json!(format!("healthcare_{}", field_type)),
        };

        Ok(value)
    }

    /// Generate e-commerce data
    fn generate_ecommerce(&self, field_type: &str) -> Result<Value> {
        let mut rng = rand::rng();

        let value = match field_type {
            "order_id" => json!(format!("ORD-{:010}", rng.random_range(0..9999999999i64))),
            "product_id" | "sku" => {
                json!(format!("SKU{:08}", rng.random_range(10000000..99999999)))
            }
            "product_name" => json!(
                ["Laptop", "Phone", "Headphones", "Mouse", "Keyboard"][rng.random_range(0..5)]
            ),
            "category" => json!(
                ["Electronics", "Clothing", "Books", "Home", "Sports"][rng.random_range(0..5)]
            ),
            "price" => json!(rng.random_range(9.99..999.99)),
            "quantity" => json!(rng.random_range(1..10)),
            "discount" => json!(rng.random_range(0.0..50.0)),
            "rating" => json!(rng.random_range(1.0..5.0)),
            "review_count" => json!(rng.random_range(0..1000)),
            "in_stock" => json!(rng.random_bool(0.8)),
            "shipping_method" => {
                json!(["Standard", "Express", "Overnight"][rng.random_range(0..3)])
            }
            "tracking_number" => json!(format!("1Z{:016}", rng.random::<u64>())),
            "order_status" => {
                json!(["pending", "processing", "shipped", "delivered"][rng.random_range(0..4)])
            }
            _ => json!(format!("ecommerce_{}", field_type)),
        };

        Ok(value)
    }

    /// Generate social media data
    fn generate_social(&self, field_type: &str) -> Result<Value> {
        let mut rng = rand::rng();

        let value = match field_type {
            "user_id" => json!(format!("user{:08}", rng.random_range(10000000..99999999))),
            "post_id" => json!(format!("post_{:016x}", rng.random::<u64>())),
            "comment_id" => json!(format!("cmt_{:012x}", rng.random::<u64>())),
            "username" => json!(format!("user{}", rng.random_range(1000..9999))),
            "display_name" => json!(
                ["Alice Smith", "Bob Johnson", "Carol White", "David Brown"]
                    [rng.random_range(0..4)]
            ),
            "bio" => json!("Passionate about technology and innovation"),
            "follower_count" => json!(rng.random_range(0..100000)),
            "following_count" => json!(rng.random_range(0..5000)),
            "post_count" => json!(rng.random_range(0..10000)),
            "likes" => json!(rng.random_range(0..10000)),
            "shares" => json!(rng.random_range(0..1000)),
            "comments" => json!(rng.random_range(0..500)),
            "hashtag" => json!(format!(
                "#{}",
                ["tech", "life", "coding", "ai", "ml"][rng.random_range(0..5)]
            )),
            "verified" => json!(rng.random_bool(0.1)),
            _ => json!(format!("social_{}", field_type)),
        };

        Ok(value)
    }

    /// Generate general data
    fn generate_general(&self, field_type: &str) -> Result<Value> {
        use crate::faker::EnhancedFaker;
        let mut faker = EnhancedFaker::new();
        Ok(faker.generate_by_type(field_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_from_str() {
        assert_eq!("finance".parse::<Domain>().unwrap(), Domain::Finance);
        assert_eq!("iot".parse::<Domain>().unwrap(), Domain::Iot);
        assert_eq!("healthcare".parse::<Domain>().unwrap(), Domain::Healthcare);
        assert_eq!("ecommerce".parse::<Domain>().unwrap(), Domain::Ecommerce);
        assert_eq!("e-commerce".parse::<Domain>().unwrap(), Domain::Ecommerce);
        assert_eq!("social".parse::<Domain>().unwrap(), Domain::Social);
        assert_eq!("general".parse::<Domain>().unwrap(), Domain::General);

        // Case insensitive
        assert_eq!("FINANCE".parse::<Domain>().unwrap(), Domain::Finance);
        assert_eq!("Finance".parse::<Domain>().unwrap(), Domain::Finance);

        // Test invalid domain returns error
        assert!("invalid".parse::<Domain>().is_err());
    }

    #[test]
    fn test_domain_from_str_error() {
        let result = "invalid".parse::<Domain>();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.invalid_domain, "invalid");
        assert!(err.to_string().contains("Invalid domain"));
        assert!(err.to_string().contains("finance, iot, healthcare"));
    }

    #[test]
    fn test_domain_as_str() {
        assert_eq!(Domain::Finance.as_str(), "finance");
        assert_eq!(Domain::Iot.as_str(), "iot");
        assert_eq!(Domain::Healthcare.as_str(), "healthcare");
    }

    #[test]
    fn test_generate_finance() {
        let generator = DomainGenerator::new(Domain::Finance);
        let result = generator.generate("amount");
        assert!(result.is_ok());
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_generate_iot() {
        let generator = DomainGenerator::new(Domain::Iot);
        let result = generator.generate("temperature");
        assert!(result.is_ok());
        assert!(result.unwrap().is_number());
    }

    #[test]
    fn test_generate_healthcare() {
        let generator = DomainGenerator::new(Domain::Healthcare);
        let result = generator.generate("patient_id");
        assert!(result.is_ok());
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_generate_ecommerce() {
        let generator = DomainGenerator::new(Domain::Ecommerce);
        let result = generator.generate("order_id");
        assert!(result.is_ok());
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_generate_social() {
        let generator = DomainGenerator::new(Domain::Social);
        let result = generator.generate("user_id");
        assert!(result.is_ok());
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_all_finance_fields() {
        let generator = DomainGenerator::new(Domain::Finance);
        let fields = vec!["account_number", "amount", "currency", "transaction_id"];

        for field in fields {
            let result = generator.generate(field);
            assert!(result.is_ok(), "Failed to generate finance field: {}", field);
        }
    }

    #[test]
    fn test_all_iot_fields() {
        let generator = DomainGenerator::new(Domain::Iot);
        let fields = vec!["device_id", "temperature", "humidity", "battery_level"];

        for field in fields {
            let result = generator.generate(field);
            assert!(result.is_ok(), "Failed to generate IoT field: {}", field);
        }
    }

    #[test]
    fn test_all_healthcare_fields() {
        let generator = DomainGenerator::new(Domain::Healthcare);
        let fields = vec!["patient_id", "blood_pressure", "heart_rate", "blood_type"];

        for field in fields {
            let result = generator.generate(field);
            assert!(result.is_ok(), "Failed to generate healthcare field: {}", field);
        }
    }
}

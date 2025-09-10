//! Faker utilities for generating realistic fake data

use fake::Fake;
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;

/// Enhanced faker with additional utilities
#[derive(Debug)]
pub struct EnhancedFaker {
    /// Random number generator
    rng: rand::rngs::ThreadRng,
}

impl Default for EnhancedFaker {
    fn default() -> Self {
        Self { rng: rand::rng() }
    }
}

impl EnhancedFaker {
    /// Create a new enhanced faker
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a random UUID
    pub fn uuid(&mut self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Generate a random integer within range
    pub fn int_range(&mut self, min: i64, max: i64) -> i64 {
        self.rng.random_range(min..=max)
    }

    /// Generate a random float within range
    pub fn float_range(&mut self, min: f64, max: f64) -> f64 {
        self.rng.random_range(min..=max)
    }

    /// Generate a random boolean with given probability
    pub fn boolean(&mut self, probability: f64) -> bool {
        self.rng.random_bool(probability.clamp(0.0, 1.0))
    }

    /// Generate a random string of given length
    pub fn string(&mut self, length: usize) -> String {
        use fake::faker::lorem::en::*;
        let word_count = (length / 5).max(1); // Approximate words needed
        let words: Vec<String> = Words(word_count..word_count + 1).fake();
        words.join(" ")
    }

    /// Generate a random email
    pub fn email(&mut self) -> String {
        use fake::faker::internet::en::*;
        FreeEmail().fake()
    }

    /// Generate a random name
    pub fn name(&mut self) -> String {
        use fake::faker::name::en::*;
        Name().fake()
    }

    /// Generate a random address
    pub fn address(&mut self) -> String {
        use fake::faker::address::en::*;
        StreetName().fake()
    }

    /// Generate a random phone number
    pub fn phone(&mut self) -> String {
        use fake::faker::phone_number::en::*;
        CellNumber().fake()
    }

    /// Generate a random company name
    pub fn company(&mut self) -> String {
        use fake::faker::company::en::*;
        CompanyName().fake()
    }

    /// Generate a random date in ISO format
    pub fn date_iso(&mut self) -> String {
        use fake::faker::chrono::en::*;
        DateTime().fake::<chrono::DateTime<chrono::Utc>>().to_rfc3339()
    }

    /// Generate a random URL
    pub fn url(&mut self) -> String {
        use fake::faker::internet::en::*;
        let domain: &str = DomainSuffix().fake();
        format!("https://example.{}", domain)
    }

    /// Generate a random IP address
    pub fn ip_address(&mut self) -> String {
        use fake::faker::internet::en::*;
        IPv4().fake()
    }

    /// Generate a random color name
    pub fn color(&mut self) -> String {
        let colors = [
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "brown", "black", "white",
        ];
        self.random_element(&colors).unwrap_or(&"blue").to_string()
    }

    /// Generate a random word
    pub fn word(&mut self) -> String {
        use fake::faker::lorem::en::*;
        Word().fake()
    }

    /// Generate random words
    pub fn words(&mut self, count: usize) -> Vec<String> {
        use fake::faker::lorem::en::*;
        Words(count..count + 1).fake()
    }

    /// Generate a random sentence
    pub fn sentence(&mut self) -> String {
        use fake::faker::lorem::en::*;
        Sentence(5..15).fake()
    }

    /// Generate a random paragraph
    pub fn paragraph(&mut self) -> String {
        use fake::faker::lorem::en::*;
        Paragraph(3..7).fake()
    }

    /// Pick a random element from a list
    pub fn random_element<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            let index = self.rng.random_range(0..items.len());
            Some(&items[index])
        }
    }

    /// Generate a value based on field type
    pub fn generate_by_type(&mut self, field_type: &str) -> Value {
        match field_type.to_lowercase().as_str() {
            "string" | "str" => Value::String(self.string(10)),
            "email" => Value::String(self.email()),
            "name" => Value::String(self.name()),
            "address" => Value::String(self.address()),
            "phone" => Value::String(self.phone()),
            "company" => Value::String(self.company()),
            "url" => Value::String(self.url()),
            "ip" => Value::String(self.ip_address()),
            "color" => Value::String(self.color()),
            "uuid" => Value::String(self.uuid()),
            "date" | "datetime" => Value::String(self.date_iso()),
            "int" | "integer" => Value::Number(self.int_range(0, 1000).into()),
            "float" | "number" => {
                Value::Number(serde_json::Number::from_f64(self.float_range(0.0, 1000.0)).unwrap())
            }
            "bool" | "boolean" => Value::Bool(self.boolean(0.5)),
            "word" => Value::String(self.word()),
            "sentence" => Value::String(self.sentence()),
            "paragraph" => Value::String(self.paragraph()),
            _ => Value::String(format!("unknown_type_{}", field_type)),
        }
    }
}

/// Template-based faker for complex data generation
#[derive(Debug)]
pub struct TemplateFaker {
    /// Base faker
    faker: EnhancedFaker,
    /// Template variables
    variables: HashMap<String, Value>,
}

impl TemplateFaker {
    /// Create a new template faker
    pub fn new() -> Self {
        Self {
            faker: EnhancedFaker::new(),
            variables: HashMap::new(),
        }
    }

    /// Add a template variable
    pub fn with_variable(mut self, key: String, value: Value) -> Self {
        self.variables.insert(key, value);
        self
    }

    /// Generate data from a template string
    pub fn generate_from_template(&mut self, template: &str) -> Value {
        let mut result = template.to_string();

        // Replace {{variable}} patterns
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            let replacement = value.to_string().trim_matches('"').to_string(); // Remove quotes if present
            result = result.replace(&pattern, &replacement);
        }

        // Replace faker patterns like {{faker.email}}
        result = self.replace_faker_patterns(&result);

        Value::String(result)
    }

    /// Replace faker patterns in template
    fn replace_faker_patterns(&mut self, template: &str) -> String {
        let mut result = template.to_string();

        // Common faker patterns
        let patterns = vec![
            ("{{faker.email}}", self.faker.email()),
            ("{{faker.name}}", self.faker.name()),
            ("{{faker.uuid}}", self.faker.uuid()),
            ("{{faker.int}}", self.faker.int_range(0, 1000).to_string()),
            ("{{faker.word}}", self.faker.word()),
            ("{{faker.sentence}}", self.faker.sentence()),
            ("{{faker.paragraph}}", self.faker.paragraph()),
            ("{{faker.date}}", self.faker.date_iso()),
            ("{{faker.url}}", self.faker.url()),
            ("{{faker.phone}}", self.faker.phone()),
            ("{{faker.company}}", self.faker.company()),
        ];

        for (pattern, replacement) in patterns {
            result = result.replace(pattern, &replacement);
        }

        result
    }

    /// Generate a complex object from template
    pub fn generate_object(&mut self, templates: HashMap<String, String>) -> Value {
        let mut object = serde_json::Map::new();

        for (key, template) in templates {
            object.insert(key, self.generate_from_template(&template));
        }

        Value::Object(object)
    }
}

impl Default for TemplateFaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick faker functions for common use cases
pub mod quick {
    use super::*;

    /// Generate a random email
    pub fn email() -> String {
        EnhancedFaker::new().email()
    }

    /// Generate a random name
    pub fn name() -> String {
        EnhancedFaker::new().name()
    }

    /// Generate a random UUID
    pub fn uuid() -> String {
        EnhancedFaker::new().uuid()
    }

    /// Generate a random integer
    pub fn int(min: i64, max: i64) -> i64 {
        EnhancedFaker::new().int_range(min, max)
    }

    /// Generate a random string
    pub fn string(length: usize) -> String {
        EnhancedFaker::new().string(length)
    }
}

//! Faker utilities for generating realistic fake data

use fake::Fake;
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;

/// Enhanced faker with additional utilities
#[derive(Debug, Default)]
pub struct EnhancedFaker;

impl EnhancedFaker {
    /// Create a new enhanced faker
    pub fn new() -> Self {
        Self
    }

    /// Generate a random UUID
    pub fn uuid(&mut self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Generate a random integer within range
    pub fn int_range(&mut self, min: i64, max: i64) -> i64 {
        rand::rng().random_range(min..=max)
    }

    /// Generate a random float within range
    pub fn float_range(&mut self, min: f64, max: f64) -> f64 {
        rand::rng().random_range(min..=max)
    }

    /// Generate a random boolean with given probability
    pub fn boolean(&mut self, probability: f64) -> bool {
        rand::rng().random_bool(probability.clamp(0.0, 1.0))
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
            let index = rand::rng().random_range(0..items.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_faker_new() {
        let _faker = EnhancedFaker::new();
        // Should create successfully
    }

    #[test]
    fn test_enhanced_faker_default() {
        let _faker = EnhancedFaker;
        // Should create successfully
    }

    #[test]
    fn test_uuid_generation() {
        let mut faker = EnhancedFaker::new();
        let uuid = faker.uuid();

        // Should have correct UUID format (36 chars with dashes)
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_int_range() {
        let mut faker = EnhancedFaker::new();
        let value = faker.int_range(1, 10);

        assert!(value >= 1);
        assert!(value <= 10);
    }

    #[test]
    fn test_float_range() {
        let mut faker = EnhancedFaker::new();
        let value = faker.float_range(0.0, 1.0);

        assert!(value >= 0.0);
        assert!(value <= 1.0);
    }

    #[test]
    fn test_boolean() {
        let mut faker = EnhancedFaker::new();
        let _value = faker.boolean(0.5);

        // Boolean generation should not panic
    }

    #[test]
    fn test_boolean_always_true() {
        let mut faker = EnhancedFaker::new();
        let value = faker.boolean(1.0);

        assert!(value);
    }

    #[test]
    fn test_boolean_always_false() {
        let mut faker = EnhancedFaker::new();
        let value = faker.boolean(0.0);

        assert!(!value);
    }

    #[test]
    fn test_string_generation() {
        let mut faker = EnhancedFaker::new();
        let s = faker.string(10);

        // Should generate a string (actual length may vary due to words)
        assert!(!s.is_empty());
    }

    #[test]
    fn test_email_generation() {
        let mut faker = EnhancedFaker::new();
        let email = faker.email();

        assert!(!email.is_empty());
        assert!(email.contains('@'));
    }

    #[test]
    fn test_name_generation() {
        let mut faker = EnhancedFaker::new();
        let name = faker.name();

        assert!(!name.is_empty());
    }

    #[test]
    fn test_address_generation() {
        let mut faker = EnhancedFaker::new();
        let address = faker.address();

        assert!(!address.is_empty());
    }

    #[test]
    fn test_phone_generation() {
        let mut faker = EnhancedFaker::new();
        let phone = faker.phone();

        assert!(!phone.is_empty());
    }

    #[test]
    fn test_company_generation() {
        let mut faker = EnhancedFaker::new();
        let company = faker.company();

        assert!(!company.is_empty());
    }

    #[test]
    fn test_date_iso_generation() {
        let mut faker = EnhancedFaker::new();
        let date = faker.date_iso();

        assert!(!date.is_empty());
        // Should contain 'T' from ISO format
        assert!(date.contains('T') || date.contains('-'));
    }

    #[test]
    fn test_url_generation() {
        let mut faker = EnhancedFaker::new();
        let url = faker.url();

        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_ip_address_generation() {
        let mut faker = EnhancedFaker::new();
        let ip = faker.ip_address();

        assert!(!ip.is_empty());
        assert!(ip.contains('.'));
    }

    #[test]
    fn test_color_generation() {
        let mut faker = EnhancedFaker::new();
        let color = faker.color();

        let valid_colors = ["red", "blue", "green", "yellow", "purple", "orange", "pink", "brown", "black", "white"];
        assert!(valid_colors.contains(&color.as_str()));
    }

    #[test]
    fn test_word_generation() {
        let mut faker = EnhancedFaker::new();
        let word = faker.word();

        assert!(!word.is_empty());
    }

    #[test]
    fn test_words_generation() {
        let mut faker = EnhancedFaker::new();
        let words = faker.words(5);

        assert_eq!(words.len(), 5);
    }

    #[test]
    fn test_sentence_generation() {
        let mut faker = EnhancedFaker::new();
        let sentence = faker.sentence();

        assert!(!sentence.is_empty());
    }

    #[test]
    fn test_paragraph_generation() {
        let mut faker = EnhancedFaker::new();
        let paragraph = faker.paragraph();

        assert!(!paragraph.is_empty());
    }

    #[test]
    fn test_random_element_success() {
        let mut faker = EnhancedFaker::new();
        let items = ["a", "b", "c", "d"];
        let element = faker.random_element(&items);

        assert!(element.is_some());
        assert!(items.contains(element.unwrap()));
    }

    #[test]
    fn test_random_element_empty_list() {
        let mut faker = EnhancedFaker::new();
        let items: [&str; 0] = [];
        let element = faker.random_element(&items);

        assert!(element.is_none());
    }

    #[test]
    fn test_generate_by_type_string() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("string");

        assert!(matches!(result, Value::String(_)));
    }

    #[test]
    fn test_generate_by_type_email() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("email");

        if let Value::String(s) = result {
            assert!(s.contains('@'));
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_generate_by_type_int() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("int");

        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_generate_by_type_bool() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("bool");

        assert!(matches!(result, Value::Bool(_)));
    }

    #[test]
    fn test_generate_by_type_uuid() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("uuid");

        if let Value::String(s) = result {
            assert_eq!(s.len(), 36);
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_template_faker_new() {
        let _faker = TemplateFaker::new();
    }

    #[test]
    fn test_template_faker_default() {
        let _faker = TemplateFaker::default();
    }

    #[test]
    fn test_template_faker_with_variable() {
        let faker = TemplateFaker::new()
            .with_variable("name".to_string(), Value::String("John".to_string()));

        assert_eq!(faker.variables.len(), 1);
        assert_eq!(faker.variables.get("name"), Some(&Value::String("John".to_string())));
    }

    #[test]
    fn test_template_faker_generate_from_template() {
        let mut faker = TemplateFaker::new()
            .with_variable("name".to_string(), Value::String("Alice".to_string()));

        let result = faker.generate_from_template("Hello {{name}}!");

        if let Value::String(s) = result {
            assert!(s.contains("Alice"));
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_template_faker_generate_object() {
        let mut faker = TemplateFaker::new()
            .with_variable("user".to_string(), Value::String("Bob".to_string()));

        let mut templates = HashMap::new();
        templates.insert("greeting".to_string(), "Hello {{user}}".to_string());
        templates.insert("farewell".to_string(), "Goodbye {{user}}".to_string());

        let result = faker.generate_object(templates);

        if let Value::Object(obj) = result {
            assert!(obj.contains_key("greeting"));
            assert!(obj.contains_key("farewell"));
        } else {
            panic!("Expected object value");
        }
    }

    #[test]
    fn test_quick_email() {
        let email = quick::email();
        assert!(!email.is_empty());
        assert!(email.contains('@'));
    }

    #[test]
    fn test_quick_name() {
        let name = quick::name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_quick_uuid() {
        let uuid = quick::uuid();
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_quick_int() {
        let value = quick::int(1, 10);
        assert!(value >= 1);
        assert!(value <= 10);
    }

    #[test]
    fn test_quick_string() {
        let s = quick::string(10);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_generate_by_type_unknown() {
        let mut faker = EnhancedFaker::new();
        let result = faker.generate_by_type("unknown_type");

        if let Value::String(s) = result {
            assert!(s.contains("unknown_type"));
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_template_faker_multiple_variables() {
        let faker = TemplateFaker::new()
            .with_variable("first".to_string(), Value::String("John".to_string()))
            .with_variable("last".to_string(), Value::String("Doe".to_string()));

        assert_eq!(faker.variables.len(), 2);
    }

    #[test]
    fn test_template_faker_generate_with_faker_pattern() {
        let mut faker = TemplateFaker::new();
        let result = faker.generate_from_template("Email: {{faker.email}}");

        if let Value::String(s) = result {
            assert!(s.contains("Email:"));
            assert!(!s.contains("{{faker.email}}"));
        } else {
            panic!("Expected string value");
        }
    }
}

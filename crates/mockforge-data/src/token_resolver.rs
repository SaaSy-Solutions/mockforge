//! Token-based response templating
//!
//! This module provides token resolution for dynamic response generation.
//! Supports $random, $faker, and $ai tokens for intelligent mock data.

use crate::{
    faker::EnhancedFaker,
    rag::{RagConfig, RagEngine},
};
use mockforge_core::{Error, Result};
use rand::Rng;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Token types supported by the resolver
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    /// Random value generation: $random.int, $random.float, $random.uuid, etc.
    Random(String),
    /// Faker data generation: $faker.name, $faker.email, $faker.address, etc.
    Faker(String),
    /// AI-generated content: $ai(prompt)
    Ai(String),
}

/// Token resolver for dynamic response generation
pub struct TokenResolver {
    /// Faker instance for data generation
    faker: Arc<RwLock<EnhancedFaker>>,
    /// RAG engine for AI generation
    rag_engine: Option<Arc<RwLock<RagEngine>>>,
    /// Cache for resolved tokens
    cache: Arc<RwLock<HashMap<String, Value>>>,
}

impl TokenResolver {
    /// Create a new token resolver
    pub fn new() -> Self {
        Self {
            faker: Arc::new(RwLock::new(EnhancedFaker::new())),
            rag_engine: None,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new token resolver with RAG support
    pub fn with_rag(rag_config: RagConfig) -> Self {
        Self {
            faker: Arc::new(RwLock::new(EnhancedFaker::new())),
            rag_engine: Some(Arc::new(RwLock::new(RagEngine::new(rag_config)))),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolve all tokens in a JSON value
    pub async fn resolve(&self, value: &Value) -> Result<Value> {
        match value {
            Value::String(s) => self.resolve_string(s).await,
            Value::Array(arr) => {
                let mut resolved = Vec::new();
                for item in arr {
                    resolved.push(Box::pin(self.resolve(item)).await?);
                }
                Ok(Value::Array(resolved))
            }
            Value::Object(obj) => {
                let mut resolved = serde_json::Map::new();
                for (key, val) in obj {
                    resolved.insert(key.clone(), Box::pin(self.resolve(val)).await?);
                }
                Ok(Value::Object(resolved))
            }
            _ => Ok(value.clone()),
        }
    }

    /// Resolve tokens in a string value
    async fn resolve_string(&self, s: &str) -> Result<Value> {
        // Check if the entire string is a single token
        if let Some(token) = self.parse_token(s) {
            return self.resolve_token(&token).await;
        }

        // Check for embedded tokens in the string
        let token_regex =
            Regex::new(r"\$(?:random|faker|ai)(?:\.[a-zA-Z_][a-zA-Z0-9_]*|\([^)]*\))?")
                .map_err(|e| Error::generic(format!("Regex error: {}", e)))?;

        if token_regex.is_match(s) {
            let mut result = s.to_string();
            for cap in token_regex.captures_iter(s) {
                if let Some(token_str) = cap.get(0) {
                    if let Some(token) = self.parse_token(token_str.as_str()) {
                        let resolved = self.resolve_token(&token).await?;
                        let resolved_str = match resolved {
                            Value::String(s) => s,
                            _ => resolved.to_string(),
                        };
                        result = result.replace(token_str.as_str(), &resolved_str);
                    }
                }
            }
            Ok(Value::String(result))
        } else {
            Ok(Value::String(s.to_string()))
        }
    }

    /// Parse a token from a string
    fn parse_token(&self, s: &str) -> Option<TokenType> {
        let s = s.trim();

        // Parse $random.* tokens
        if let Some(suffix) = s.strip_prefix("$random.") {
            return Some(TokenType::Random(suffix.to_string()));
        }

        // Parse $faker.* tokens
        if let Some(suffix) = s.strip_prefix("$faker.") {
            return Some(TokenType::Faker(suffix.to_string()));
        }

        // Parse $ai(...) tokens
        if s.starts_with("$ai(") && s.ends_with(')') {
            let prompt = s.strip_prefix("$ai(")?.strip_suffix(')')?;
            return Some(TokenType::Ai(prompt.trim().to_string()));
        }

        None
    }

    /// Resolve a single token
    async fn resolve_token(&self, token: &TokenType) -> Result<Value> {
        match token {
            TokenType::Random(kind) => self.resolve_random(kind).await,
            TokenType::Faker(kind) => self.resolve_faker(kind).await,
            TokenType::Ai(prompt) => self.resolve_ai(prompt).await,
        }
    }

    /// Resolve a $random token
    async fn resolve_random(&self, kind: &str) -> Result<Value> {
        match kind {
            "int" | "integer" => Ok(json!(rand::rng().random_range(0..1000))),
            "int.small" => Ok(json!(rand::rng().random_range(0..100))),
            "int.large" => Ok(json!(rand::rng().random_range(0..1_000_000))),
            "float" | "number" => Ok(json!(rand::rng().random_range(0.0..1000.0))),
            "bool" | "boolean" => Ok(json!(rand::rng().random_bool(0.5))),
            "uuid" => Ok(json!(uuid::Uuid::new_v4().to_string())),
            "hex" => {
                let bytes: [u8; 16] = rand::rng().random();
                Ok(json!(hex::encode(bytes)))
            }
            "hex.short" => {
                let bytes: [u8; 4] = rand::rng().random();
                Ok(json!(hex::encode(bytes)))
            }
            "alphanumeric" => {
                let s: String = (0..10)
                    .map(|_| {
                        let c: u8 = rand::rng().random_range(b'a'..=b'z');
                        c as char
                    })
                    .collect();
                Ok(json!(s))
            }
            "choice" => {
                let choices = ["option1", "option2", "option3"];
                let idx = rand::rng().random_range(0..choices.len());
                Ok(json!(choices[idx]))
            }
            _ => Err(Error::generic(format!("Unknown random type: {}", kind))),
        }
    }

    /// Resolve a $faker token
    async fn resolve_faker(&self, kind: &str) -> Result<Value> {
        let mut faker = self.faker.write().await;

        let value = match kind {
            // Person
            "name" => json!(faker.name()),
            "email" => json!(faker.email()),
            "phone" | "phone_number" => json!(faker.phone()),

            // Address
            "address" => json!(faker.address()),

            // Company
            "company" => json!(faker.company()),

            // Internet
            "url" => json!(faker.url()),
            "ipv4" | "ip" => json!(faker.ip_address()),

            // Date/Time
            "date" | "datetime" | "timestamp" | "iso8601" => json!(faker.date_iso()),

            // Lorem
            "word" => json!(faker.word()),
            "words" => json!(faker.words(5)),
            "sentence" => json!(faker.sentence()),
            "paragraph" => json!(faker.paragraph()),

            // ID
            "uuid" => json!(faker.uuid()),

            // Use generate_by_type for other types
            _ => faker.generate_by_type(kind),
        };

        Ok(value)
    }

    /// Resolve an $ai token
    async fn resolve_ai(&self, prompt: &str) -> Result<Value> {
        // Check cache first
        let cache_key = format!("ai:{}", prompt);
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Generate using RAG engine if available
        if let Some(rag_engine) = &self.rag_engine {
            let engine = rag_engine.write().await;
            let response = engine.generate_text(prompt).await?;

            // Try to parse as JSON
            let value = if let Ok(json_value) = serde_json::from_str::<Value>(&response) {
                json_value
            } else {
                json!(response)
            };

            // Cache the result
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, value.clone());

            Ok(value)
        } else {
            // Fallback: return a placeholder if no RAG engine available
            Ok(json!(format!("[AI: {}]", prompt)))
        }
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Default for TokenResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve tokens in a value using a default resolver
pub async fn resolve_tokens(value: &Value) -> Result<Value> {
    let resolver = TokenResolver::new();
    resolver.resolve(value).await
}

/// Resolve tokens with RAG support
pub async fn resolve_tokens_with_rag(value: &Value, rag_config: RagConfig) -> Result<Value> {
    let resolver = TokenResolver::with_rag(rag_config);
    resolver.resolve(value).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_random() {
        let resolver = TokenResolver::new();
        assert_eq!(resolver.parse_token("$random.int"), Some(TokenType::Random("int".to_string())));
        assert_eq!(
            resolver.parse_token("$random.uuid"),
            Some(TokenType::Random("uuid".to_string()))
        );
    }

    #[test]
    fn test_parse_token_faker() {
        let resolver = TokenResolver::new();
        assert_eq!(resolver.parse_token("$faker.name"), Some(TokenType::Faker("name".to_string())));
        assert_eq!(
            resolver.parse_token("$faker.email"),
            Some(TokenType::Faker("email".to_string()))
        );
    }

    #[test]
    fn test_parse_token_ai() {
        let resolver = TokenResolver::new();
        assert_eq!(
            resolver.parse_token("$ai(generate customer data)"),
            Some(TokenType::Ai("generate customer data".to_string()))
        );
    }

    #[test]
    fn test_parse_token_invalid() {
        let resolver = TokenResolver::new();
        assert_eq!(resolver.parse_token("invalid"), None);
        assert_eq!(resolver.parse_token("$invalid"), None);
    }

    #[tokio::test]
    async fn test_resolve_random_int() {
        let resolver = TokenResolver::new();
        let result = resolver.resolve_random("int").await.unwrap();
        assert!(result.is_number());
    }

    #[tokio::test]
    async fn test_resolve_random_uuid() {
        let resolver = TokenResolver::new();
        let result = resolver.resolve_random("uuid").await.unwrap();
        assert!(result.is_string());
        let uuid_str = result.as_str().unwrap();
        assert!(uuid::Uuid::parse_str(uuid_str).is_ok());
    }

    #[tokio::test]
    async fn test_resolve_faker_name() {
        let resolver = TokenResolver::new();
        let result = resolver.resolve_faker("name").await.unwrap();
        assert!(result.is_string());
    }

    #[tokio::test]
    async fn test_resolve_faker_email() {
        let resolver = TokenResolver::new();
        let result = resolver.resolve_faker("email").await.unwrap();
        assert!(result.is_string());
        let email = result.as_str().unwrap();
        assert!(email.contains('@'));
    }

    #[tokio::test]
    async fn test_resolve_simple_string() {
        let resolver = TokenResolver::new();
        let value = json!("$random.uuid");
        let result = resolver.resolve(&value).await.unwrap();
        assert!(result.is_string());
    }

    #[tokio::test]
    async fn test_resolve_object() {
        let resolver = TokenResolver::new();
        let value = json!({
            "id": "$random.uuid",
            "name": "$faker.name",
            "email": "$faker.email"
        });
        let result = resolver.resolve(&value).await.unwrap();
        assert!(result.is_object());
        assert!(result["id"].is_string());
        assert!(result["name"].is_string());
        assert!(result["email"].is_string());
    }

    #[tokio::test]
    async fn test_resolve_array() {
        let resolver = TokenResolver::new();
        let value = json!(["$random.uuid", "$faker.name"]);
        let result = resolver.resolve(&value).await.unwrap();
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[tokio::test]
    async fn test_resolve_nested() {
        let resolver = TokenResolver::new();
        let value = json!({
            "user": {
                "id": "$random.uuid",
                "profile": {
                    "name": "$faker.name",
                    "email": "$faker.email"
                }
            }
        });
        let result = resolver.resolve(&value).await.unwrap();
        assert!(result["user"]["id"].is_string());
        assert!(result["user"]["profile"]["name"].is_string());
        assert!(result["user"]["profile"]["email"].is_string());
    }

    #[tokio::test]
    async fn test_cache() {
        let resolver = TokenResolver::new();
        assert_eq!(resolver.cache_size().await, 0);

        // The cache is used internally for AI tokens
        // For now just verify the cache methods work
        resolver.clear_cache().await;
        assert_eq!(resolver.cache_size().await, 0);
    }
}

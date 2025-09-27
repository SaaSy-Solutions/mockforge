//! RAG-driven domain-aware data synthesis
//!
//! This module integrates with the MockForge RAG system to generate contextually
//! appropriate synthetic data based on schema documentation, API specifications,
//! and domain knowledge.

use crate::reflection::schema_graph::SchemaGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[cfg(feature = "data-faker")]
use mockforge_data::rag::{RagConfig, RagEngine};

/// Configuration for RAG-driven data synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSynthesisConfig {
    /// Enable RAG-driven synthesis
    pub enabled: bool,
    /// RAG engine configuration
    pub rag_config: Option<RagSynthesisRagConfig>,
    /// Domain context sources
    pub context_sources: Vec<ContextSource>,
    /// Prompt templates for different entity types
    pub prompt_templates: HashMap<String, PromptTemplate>,
    /// Maximum context length for RAG queries
    pub max_context_length: usize,
    /// Cache generated contexts for performance
    pub cache_contexts: bool,
}

/// RAG configuration specific to synthesis (wrapper around mockforge_data::RagConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSynthesisRagConfig {
    /// API endpoint
    pub api_endpoint: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Embedding model configuration
    pub embedding_model: String,
    /// Search similarity threshold
    pub similarity_threshold: f64,
    /// Maximum documents to retrieve
    pub max_documents: usize,
}

/// Source of domain context for RAG synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source identifier
    pub id: String,
    /// Source type (documentation, examples, etc.)
    pub source_type: ContextSourceType,
    /// Path or URL to the source
    pub path: String,
    /// Weight for this source in context generation
    pub weight: f32,
    /// Whether this source is required for synthesis
    pub required: bool,
}

/// Types of context sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceType {
    /// API documentation (OpenAPI, proto comments)
    Documentation,
    /// Example data files (JSON, YAML)
    Examples,
    /// Business rules and constraints
    BusinessRules,
    /// Domain glossary/terminology
    Glossary,
    /// External knowledge base
    KnowledgeBase,
}

/// Template for generating RAG prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name/identifier
    pub name: String,
    /// Entity types this template applies to
    pub entity_types: Vec<String>,
    /// Template string with placeholders
    pub template: String,
    /// Variables that can be substituted in the template
    pub variables: Vec<String>,
    /// Examples of expected outputs
    pub examples: Vec<PromptExample>,
}

/// Example for prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptExample {
    /// Input context
    pub input: HashMap<String, String>,
    /// Expected output
    pub output: String,
    /// Description of this example
    pub description: String,
}

/// Context extracted for an entity using RAG
#[derive(Debug, Clone)]
pub struct EntityContext {
    /// Entity name
    pub entity_name: String,
    /// Domain context from RAG
    pub domain_context: String,
    /// Related entities and their contexts
    pub related_contexts: HashMap<String, String>,
    /// Business rules applicable to this entity
    pub business_rules: Vec<BusinessRule>,
    /// Example values from documentation
    pub example_values: HashMap<String, Vec<String>>,
}

/// A business rule extracted from context
#[derive(Debug, Clone)]
pub struct BusinessRule {
    /// Rule description
    pub description: String,
    /// Fields this rule applies to
    pub applies_to_fields: Vec<String>,
    /// Rule type (constraint, format, relationship, etc.)
    pub rule_type: BusinessRuleType,
    /// Rule parameters/configuration
    pub parameters: HashMap<String, String>,
}

/// Types of business rules
#[derive(Debug, Clone)]
pub enum BusinessRuleType {
    /// Format constraint (email format, phone format, etc.)
    Format,
    /// Value range constraint
    Range,
    /// Relationship constraint (foreign key rules)
    Relationship,
    /// Business logic constraint
    BusinessLogic,
    /// Validation rule
    Validation,
}

/// RAG-driven data synthesis engine
pub struct RagDataSynthesizer {
    /// Configuration
    config: RagSynthesisConfig,
    /// RAG engine instance
    #[cfg(feature = "data-faker")]
    rag_engine: Option<RagEngine>,
    /// Cached entity contexts
    entity_contexts: HashMap<String, EntityContext>,
    /// Schema graph for relationship understanding
    schema_graph: Option<SchemaGraph>,
}

impl RagDataSynthesizer {
    /// Create a new RAG data synthesizer
    pub fn new(config: RagSynthesisConfig) -> Self {
        #[cfg(feature = "data-faker")]
        let rag_engine = if config.enabled && config.rag_config.is_some() {
            let rag_config = config.rag_config.as_ref().unwrap();
            match Self::initialize_rag_engine(rag_config) {
                Ok(engine) => Some(engine),
                Err(e) => {
                    warn!("Failed to initialize RAG engine: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            config,
            #[cfg(feature = "data-faker")]
            rag_engine,
            entity_contexts: HashMap::new(),
            schema_graph: None,
        }
    }

    /// Set the schema graph for relationship-aware synthesis
    pub fn set_schema_graph(&mut self, schema_graph: SchemaGraph) {
        let entity_count = schema_graph.entities.len();
        self.schema_graph = Some(schema_graph);
        info!("Schema graph set with {} entities", entity_count);
    }

    /// Generate domain context for an entity using RAG
    pub async fn generate_entity_context(
        &mut self,
        entity_name: &str,
    ) -> Result<EntityContext, Box<dyn std::error::Error + Send + Sync>> {
        // Check cache first
        if let Some(cached_context) = self.entity_contexts.get(entity_name) {
            return Ok(cached_context.clone());
        }

        info!("Generating RAG context for entity: {}", entity_name);

        let mut context = EntityContext {
            entity_name: entity_name.to_string(),
            domain_context: String::new(),
            related_contexts: HashMap::new(),
            business_rules: Vec::new(),
            example_values: HashMap::new(),
        };

        // Generate base context using RAG
        if self.config.enabled {
            context.domain_context = self.query_rag_for_entity(entity_name).await?;
        }

        // Extract business rules from context
        context.business_rules =
            self.extract_business_rules(&context.domain_context, entity_name)?;

        // Find example values from context sources
        context.example_values =
            self.extract_example_values(&context.domain_context, entity_name)?;

        // Generate related entity contexts if schema graph is available
        if let Some(schema_graph) = &self.schema_graph {
            context.related_contexts =
                self.generate_related_contexts(entity_name, schema_graph).await?;
        }

        // Cache the context
        if self.config.cache_contexts {
            self.entity_contexts.insert(entity_name.to_string(), context.clone());
        }

        Ok(context)
    }

    /// Generate contextually appropriate data for an entity field
    pub async fn synthesize_field_data(
        &mut self,
        entity_name: &str,
        field_name: &str,
        field_type: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let context = self.generate_entity_context(entity_name).await?;

        // Check for example values first
        if let Some(examples) = context.example_values.get(field_name) {
            if !examples.is_empty() {
                // Use a deterministic example selection based on field name hash for stability
                let field_hash = self.hash_field_name(field_name);
                let index = field_hash as usize % examples.len();
                return Ok(Some(examples[index].clone()));
            }
        }

        // Apply business rules
        for rule in &context.business_rules {
            if rule.applies_to_fields.contains(&field_name.to_string()) {
                if let Some(value) = self.apply_business_rule(rule, field_name, field_type)? {
                    return Ok(Some(value));
                }
            }
        }

        // Use RAG to generate contextually appropriate value
        if self.config.enabled && !context.domain_context.is_empty() {
            let rag_value =
                self.generate_contextual_value(&context, field_name, field_type).await?;
            if !rag_value.is_empty() {
                return Ok(Some(rag_value));
            }
        }

        Ok(None)
    }

    /// Initialize RAG engine from configuration
    #[cfg(feature = "data-faker")]
    fn initialize_rag_engine(
        config: &RagSynthesisRagConfig,
    ) -> Result<RagEngine, Box<dyn std::error::Error + Send + Sync>> {
        let rag_config = RagConfig {
            provider: mockforge_data::rag::LlmProvider::OpenAI,
            api_endpoint: config.api_endpoint.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            max_tokens: 1000,
            temperature: 0.7,
            context_window: 4000,
            semantic_search_enabled: true,
            embedding_provider: mockforge_data::rag::EmbeddingProvider::OpenAI,
            embedding_model: config.embedding_model.clone(),
            embedding_endpoint: None,
            similarity_threshold: config.similarity_threshold,
            max_chunks: config.max_documents,
            request_timeout_seconds: 30,
            max_retries: 3,
        };

        Ok(RagEngine::new(rag_config))
    }

    /// Query RAG system for entity-specific context
    async fn query_rag_for_entity(
        &self,
        entity_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(feature = "data-faker")]
        if let Some(rag_engine) = &self.rag_engine {
            let query = format!("What is {} in this domain? What are typical values and constraints for {} entities?", entity_name, entity_name);

            let chunks = rag_engine
                .keyword_search(&query, self.config.rag_config.as_ref().unwrap().max_documents);
            if !chunks.is_empty() {
                let context = chunks
                    .into_iter()
                    .map(|chunk| &chunk.content)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n\n");
                return Ok(context);
            } else {
                warn!("No RAG results found for entity {}", entity_name);
            }
        }

        // Fallback to basic context
        Ok(format!("Entity: {} - A data entity in the system", entity_name))
    }

    /// Extract business rules from context text
    fn extract_business_rules(
        &self,
        context: &str,
        entity_name: &str,
    ) -> Result<Vec<BusinessRule>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rules = Vec::new();

        // Simple rule extraction - can be enhanced with NLP
        if context.to_lowercase().contains("email") && context.to_lowercase().contains("format") {
            rules.push(BusinessRule {
                description: "Email fields must follow email format".to_string(),
                applies_to_fields: vec!["email".to_string(), "email_address".to_string()],
                rule_type: BusinessRuleType::Format,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("format".to_string(), "email".to_string());
                    params
                },
            });
        }

        if context.to_lowercase().contains("phone") && context.to_lowercase().contains("number") {
            rules.push(BusinessRule {
                description: "Phone fields must follow phone number format".to_string(),
                applies_to_fields: vec![
                    "phone".to_string(),
                    "mobile".to_string(),
                    "phone_number".to_string(),
                ],
                rule_type: BusinessRuleType::Format,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("format".to_string(), "phone".to_string());
                    params
                },
            });
        }

        debug!("Extracted {} business rules for entity {}", rules.len(), entity_name);
        Ok(rules)
    }

    /// Extract example values from context
    fn extract_example_values(
        &self,
        context: &str,
        _entity_name: &str,
    ) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut examples = HashMap::new();

        // Simple example extraction - can be enhanced with regex/NLP
        let lines: Vec<&str> = context.lines().collect();
        for line in lines {
            if line.contains("example:") || line.contains("e.g.") {
                // Extract examples from line - simplified implementation
                if line.to_lowercase().contains("email") {
                    examples
                        .entry("email".to_string())
                        .or_insert_with(Vec::new)
                        .push("user@example.com".to_string());
                }
                if line.to_lowercase().contains("name") {
                    examples
                        .entry("name".to_string())
                        .or_insert_with(Vec::new)
                        .push("John Doe".to_string());
                }
            }
        }

        Ok(examples)
    }

    /// Generate contexts for related entities
    async fn generate_related_contexts(
        &self,
        entity_name: &str,
        schema_graph: &SchemaGraph,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut related_contexts = HashMap::new();

        if let Some(entity) = schema_graph.entities.get(entity_name) {
            for related_entity in &entity.references {
                if related_entity != entity_name {
                    let related_context = self.query_rag_for_entity(related_entity).await?;
                    related_contexts.insert(related_entity.clone(), related_context);
                }
            }
        }

        Ok(related_contexts)
    }

    /// Apply a business rule to generate field value
    fn apply_business_rule(
        &self,
        rule: &BusinessRule,
        field_name: &str,
        _field_type: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        match rule.rule_type {
            BusinessRuleType::Format => {
                if let Some(format) = rule.parameters.get("format") {
                    match format.as_str() {
                        "email" => return Ok(Some("user@example.com".to_string())),
                        "phone" => return Ok(Some("+1-555-0123".to_string())),
                        _ => {}
                    }
                }
            }
            BusinessRuleType::Range => {
                // Apply range constraints
                if let (Some(min), Some(max)) =
                    (rule.parameters.get("min"), rule.parameters.get("max"))
                {
                    if let (Ok(min_val), Ok(max_val)) = (min.parse::<i32>(), max.parse::<i32>()) {
                        // Use deterministic value based on field name hash
                        let field_hash = self.hash_field_name(field_name);
                        let value = (field_hash as i32 % (max_val - min_val)) + min_val;
                        return Ok(Some(value.to_string()));
                    }
                }
            }
            _ => {
                debug!("Unhandled business rule type for field {}", field_name);
            }
        }

        Ok(None)
    }

    /// Generate contextual value using RAG
    async fn generate_contextual_value(
        &self,
        context: &EntityContext,
        field_name: &str,
        field_type: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Use a prompt template to generate contextually appropriate value
        if let Some(template) = self.find_applicable_template(&context.entity_name) {
            let prompt =
                self.build_prompt_from_template(template, context, field_name, field_type)?;

            #[cfg(feature = "data-faker")]
            if let Some(rag_engine) = &self.rag_engine {
                let chunks = rag_engine.keyword_search(&prompt, 1);
                if let Some(chunk) = chunks.first() {
                    return Ok(chunk.content.clone());
                } else {
                    debug!("No contextual value found for prompt: {}", prompt);
                }
            }
        }

        // Fallback to basic contextual generation
        Ok(format!("contextual_{}_{}", context.entity_name.to_lowercase(), field_name))
    }

    /// Find applicable prompt template for entity
    fn find_applicable_template(&self, entity_name: &str) -> Option<&PromptTemplate> {
        for template in self.config.prompt_templates.values() {
            if template.entity_types.contains(&entity_name.to_string())
                || template.entity_types.contains(&"*".to_string())
            {
                return Some(template);
            }
        }
        None
    }

    /// Build prompt from template
    fn build_prompt_from_template(
        &self,
        template: &PromptTemplate,
        context: &EntityContext,
        field_name: &str,
        field_type: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut prompt = template.template.clone();

        // Replace variables in template
        prompt = prompt.replace("{entity_name}", &context.entity_name);
        prompt = prompt.replace("{field_name}", field_name);
        prompt = prompt.replace("{field_type}", field_type);
        prompt = prompt.replace("{domain_context}", &context.domain_context);

        Ok(prompt)
    }

    /// Get configuration
    pub fn config(&self) -> &RagSynthesisConfig {
        &self.config
    }

    /// Check if RAG synthesis is enabled and available
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && {
            #[cfg(feature = "data-faker")]
            {
                self.rag_engine.is_some()
            }
            #[cfg(not(feature = "data-faker"))]
            {
                false
            }
        }
    }

    /// Generate a deterministic hash for a field name for stable data generation
    pub fn hash_field_name(&self, field_name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        field_name.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for RagSynthesisConfig {
    fn default() -> Self {
        let mut prompt_templates = HashMap::new();

        // Default template for all entities
        prompt_templates.insert("default".to_string(), PromptTemplate {
            name: "default".to_string(),
            entity_types: vec!["*".to_string()],
            template: "Generate a realistic value for {field_name} field of type {field_type} in a {entity_name} entity. Context: {domain_context}".to_string(),
            variables: vec!["entity_name".to_string(), "field_name".to_string(), "field_type".to_string(), "domain_context".to_string()],
            examples: vec![],
        });

        Self {
            enabled: false,
            rag_config: None,
            context_sources: vec![],
            prompt_templates,
            max_context_length: 2000,
            cache_contexts: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RagSynthesisConfig::default();
        assert!(!config.enabled);
        assert!(config.prompt_templates.contains_key("default"));
        assert!(config.cache_contexts);
    }

    #[tokio::test]
    async fn test_synthesizer_creation() {
        let config = RagSynthesisConfig::default();
        let synthesizer = RagDataSynthesizer::new(config);
        assert!(!synthesizer.is_enabled());
    }

    #[test]
    fn test_business_rule_extraction() {
        let config = RagSynthesisConfig::default();
        let synthesizer = RagDataSynthesizer::new(config);

        let context = "Users must provide a valid email format. Phone numbers should be in international format.";
        let rules = synthesizer.extract_business_rules(context, "User").unwrap();

        assert!(rules.len() >= 1);
        assert!(rules.iter().any(|r| matches!(r.rule_type, BusinessRuleType::Format)));
    }
}

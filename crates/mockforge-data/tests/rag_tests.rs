use mockforge_data::rag::{DocumentChunk, EmbeddingProvider, LlmProvider, RagConfig, RagEngine};
use serde_json::json;
use std::collections::HashMap;

#[cfg(test)]
mod rag_tests {
    use super::*;

    #[test]
    fn test_rag_engine_creation() {
        let config = RagConfig::default();
        let engine = RagEngine::new(config);
        assert_eq!(engine.chunk_count(), 0);
    }

    #[test]
    fn test_rag_add_document() {
        let config = RagConfig::default();
        let mut engine = RagEngine::new(config);

        let content = "This is a test document about artificial intelligence and machine learning.";
        let _ = engine.add_document(content.to_string(), HashMap::new());

        assert_eq!(engine.chunk_count(), 1);
        let chunk = engine.get_chunk(0).unwrap();
        assert_eq!(chunk.content, content);
        assert_eq!(chunk.id, "chunk_0");
    }

    #[test]
    fn test_rag_keyword_search() {
        let config = RagConfig {
            semantic_search_enabled: false,
            ..Default::default()
        };

        let mut engine = RagEngine::new(config);

        let _ = engine.add_document(
            "The quick brown fox jumps over the lazy dog".to_string(),
            HashMap::new(),
        );
        let _ = engine.add_document(
            "Machine learning is a subset of artificial intelligence".to_string(),
            HashMap::new(),
        );
        let _ = engine
            .add_document("Rust is a systems programming language".to_string(), HashMap::new());

        let chunks = engine.keyword_search("machine learning", 5);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.to_lowercase().contains("machine learning"));
    }

    #[test]
    fn test_rag_keyword_search_multiple_results() {
        let config = RagConfig {
            semantic_search_enabled: false,
            ..Default::default()
        };

        let mut engine = RagEngine::new(config);

        let _ = engine.add_document(
            "AI and machine learning are transforming technology".to_string(),
            HashMap::new(),
        );
        let _ = engine.add_document(
            "Machine learning algorithms process data efficiently".to_string(),
            HashMap::new(),
        );
        let _ = engine.add_document("The weather is nice today".to_string(), HashMap::new());

        let chunks = engine.keyword_search("machine learning", 5);
        assert_eq!(chunks.len(), 2);
        for chunk in &chunks {
            assert!(chunk.content.to_lowercase().contains("machine learning"));
        }
    }

    #[test]
    fn test_rag_keyword_search_limit() {
        let config = RagConfig {
            semantic_search_enabled: false,
            ..Default::default()
        };

        let mut engine = RagEngine::new(config);

        for i in 0..10 {
            let _ = engine.add_document(
                format!("Document {} with machine learning content", i),
                HashMap::new(),
            );
        }

        let chunks = engine.keyword_search("machine learning", 3);
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn test_rag_keyword_search_no_matches() {
        let config = RagConfig {
            semantic_search_enabled: false,
            ..Default::default()
        };

        let mut engine = RagEngine::new(config);

        let _ = engine.add_document("The quick brown fox".to_string(), HashMap::new());
        let _ = engine.add_document("Machine learning content".to_string(), HashMap::new());

        let chunks = engine.keyword_search("nonexistent", 5);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_rag_schema_knowledge_base() {
        let config = RagConfig::default();
        let mut engine = RagEngine::new(config);

        use mockforge_data::{FieldDefinition, SchemaDefinition};

        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("email".to_string(), "string".to_string()))
            .with_description("A user in the system".to_string());

        engine.add_schema(&schema).unwrap();

        assert!(engine.has_schema("User"));
    }

    #[test]
    fn test_rag_generate_prompt() {
        let config = RagConfig::default();
        let mut engine = RagEngine::new(config);

        use mockforge_data::{FieldDefinition, SchemaDefinition};

        let schema = SchemaDefinition::new("Product".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("price".to_string(), "number".to_string()));

        engine.add_schema(&schema).unwrap();

        // Note: This test is limited since we can't actually call the LLM
        // build_generation_prompt is private, so we skip testing it directly
        // In a real test environment, you'd mock the LLM call
        assert_eq!(engine.schema_count(), 1);
    }

    #[test]
    fn test_rag_config_semantic_search_enabled() {
        let config = RagConfig {
            semantic_search_enabled: true,
            embedding_provider: EmbeddingProvider::OpenAI,
            embedding_model: "text-embedding-ada-002".to_string(),
            similarity_threshold: 0.8,
            max_chunks: 3,
            ..Default::default()
        };

        assert!(config.semantic_search_enabled);
        assert_eq!(config.embedding_provider, EmbeddingProvider::OpenAI);
        assert_eq!(config.embedding_model, "text-embedding-ada-002");
        assert_eq!(config.similarity_threshold, 0.8);
        assert_eq!(config.max_chunks, 3);
    }

    #[test]
    fn test_rag_config_defaults() {
        let config = RagConfig::default();

        assert!(config.semantic_search_enabled);
        assert_eq!(config.provider, LlmProvider::OpenAI);
        assert_eq!(config.embedding_provider, EmbeddingProvider::OpenAI);
        assert_eq!(config.embedding_model, "text-embedding-ada-002");
        assert_eq!(config.similarity_threshold, 0.7);
        assert_eq!(config.max_chunks, 5);
    }

    #[test]
    fn test_document_chunk_creation() {
        let metadata = HashMap::from([
            ("source".to_string(), json!("api_docs")),
            ("version".to_string(), json!("1.0")),
        ]);

        let chunk = DocumentChunk {
            id: "test_chunk".to_string(),
            content: "This is test content".to_string(),
            metadata,
            embedding: vec![0.1, 0.2, 0.3],
        };

        assert_eq!(chunk.id, "test_chunk");
        assert_eq!(chunk.content, "This is test content");
        assert_eq!(chunk.metadata.get("source"), Some(&json!("api_docs")));
        assert_eq!(chunk.embedding, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_rag_with_empty_chunks() {
        let config = RagConfig::default();
        let engine = RagEngine::new(config);

        let chunks = engine.keyword_search("test query", 5);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_rag_multiple_schema_fields() {
        let config = RagConfig::default();
        let mut engine = RagEngine::new(config);

        use mockforge_data::{FieldDefinition, SchemaDefinition};

        let schema = SchemaDefinition::new("Order".to_string())
            .with_field(FieldDefinition::new("id".to_string(), "integer".to_string()))
            .with_field(FieldDefinition::new("customer_id".to_string(), "integer".to_string()))
            .with_field(FieldDefinition::new("total".to_string(), "number".to_string()))
            .with_field(FieldDefinition::new("status".to_string(), "string".to_string()).optional())
            .with_description("An order in the e-commerce system".to_string());

        engine.add_schema(&schema).unwrap();

        // build_generation_prompt is private, so we skip testing it directly
        assert_eq!(engine.schema_count(), 1);
    }
}

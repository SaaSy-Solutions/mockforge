//! Embedding client for vector memory
//!
//! This module provides a client for generating embeddings for semantic search.

use crate::Result;

/// Embedding client for generating vector embeddings
pub struct EmbeddingClient {
    /// Provider type
    provider: String,
    /// Model name
    model: String,
    /// API key (optional)
    api_key: Option<String>,
    /// API endpoint
    endpoint: String,
    /// HTTP client
    client: reqwest::Client,
}

impl EmbeddingClient {
    /// Create a new embedding client
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        endpoint: Option<String>,
    ) -> Self {
        let provider = provider.into();
        let endpoint = endpoint.unwrap_or_else(|| match provider.as_str() {
            "openai" => "https://api.openai.com/v1/embeddings".to_string(),
            _ => "http://localhost:8080/v1/embeddings".to_string(),
        });

        Self {
            provider,
            model: model.into(),
            api_key,
            endpoint,
            client: reqwest::Client::new(),
        }
    }

    /// Generate an embedding for text
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        match self.provider.as_str() {
            "openai" | "openai-compatible" => self.generate_openai_embedding(text).await,
            _ => Err(crate::Error::generic(format!(
                "Unsupported embedding provider: {}",
                self.provider
            ))),
        }
    }

    /// Generate embeddings for multiple texts
    pub async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            let embedding = self.generate_embedding(&text).await?;
            embeddings.push(embedding);
        }
        Ok(embeddings)
    }

    /// Generate embedding using OpenAI API
    async fn generate_openai_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| crate::Error::generic("OpenAI API key not found"))?;

        let request_body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let mut request = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");

        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::generic(format!("Embedding API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::generic(format!(
                "Embedding API error: {}",
                error_text
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to parse embedding response: {}", e)))?;

        // Extract embedding vector
        let embedding: Vec<f32> = response_json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| crate::Error::generic("Invalid embedding response format"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if embedding.is_empty() {
            return Err(crate::Error::generic("Empty embedding returned"));
        }

        Ok(embedding)
    }
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0];
        let d = vec![0.0, 1.0];
        assert!((cosine_similarity(&c, &d) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_embedding_client_creation() {
        let client = EmbeddingClient::new(
            "openai",
            "text-embedding-ada-002",
            Some("test_key".to_string()),
            None,
        );
        assert_eq!(client.provider, "openai");
        assert_eq!(client.model, "text-embedding-ada-002");
    }
}

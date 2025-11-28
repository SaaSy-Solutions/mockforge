//! Context-aware pagination intelligence
//!
//! This module generates realistic pagination metadata using LLMs and learns
//! pagination patterns from examples to create contextually appropriate
//! paginated responses.

use super::config::BehaviorModelConfig;
use super::context::StatefulAiContext;
use super::llm_client::LlmClient;
use super::rule_generator::PaginatedResponse;
use super::types::LlmGenerationRequest;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Pagination request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequest {
    /// Request path
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body (optional)
    pub request_body: Option<Value>,
}

/// Pagination metadata for response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    /// Current page number (for page-based pagination)
    pub page: Option<usize>,
    /// Page size (number of items per page)
    pub page_size: usize,
    /// Total number of items
    pub total: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Whether there is a next page
    pub has_next: bool,
    /// Whether there is a previous page
    pub has_prev: bool,
    /// Offset (for offset-based pagination)
    pub offset: Option<usize>,
    /// Cursor for next page (for cursor-based pagination)
    pub next_cursor: Option<String>,
    /// Cursor for previous page (for cursor-based pagination)
    pub prev_cursor: Option<String>,
}

/// Pagination intelligence engine
pub struct PaginationIntelligence {
    /// LLM client for generating realistic totals
    llm_client: Option<LlmClient>,
    /// Configuration
    config: BehaviorModelConfig,
    /// Learned pagination examples
    examples: Vec<PaginatedResponse>,
    /// Default pagination rule
    default_rule: PaginationRule,
}

/// Pagination rule learned from examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRule {
    /// Default page size
    pub default_page_size: usize,
    /// Maximum page size
    pub max_page_size: usize,
    /// Minimum page size
    pub min_page_size: usize,
    /// Pagination format
    pub format: PaginationFormat,
    /// Parameter names mapping
    pub parameter_names: HashMap<String, String>,
}

/// Pagination format type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaginationFormat {
    /// Page-based (page, per_page)
    PageBased,
    /// Offset-based (offset, limit)
    OffsetBased,
    /// Cursor-based (cursor)
    CursorBased,
}

impl PaginationIntelligence {
    /// Create new pagination intelligence
    pub fn new(config: BehaviorModelConfig) -> Self {
        let llm_client = if config.llm_provider != "disabled" {
            Some(LlmClient::new(config.clone()))
        } else {
            None
        };

        Self {
            llm_client,
            config,
            examples: Vec::new(),
            default_rule: PaginationRule {
                default_page_size: 20,
                max_page_size: 100,
                min_page_size: 1,
                format: PaginationFormat::PageBased,
                parameter_names: HashMap::new(),
            },
        }
    }

    /// Learn from pagination example
    pub fn learn_from_example(&mut self, example: PaginatedResponse) {
        self.examples.push(example);
        // Update default rule based on examples
        self.update_rule_from_examples();
    }

    /// Generate pagination metadata for a request
    ///
    /// Creates realistic pagination metadata based on the request context,
    /// learned patterns, and session state.
    pub async fn generate_pagination_metadata(
        &self,
        request: &PaginationRequest,
        context: &StatefulAiContext,
    ) -> Result<PaginationMetadata> {
        // Extract pagination parameters from request
        let (page, page_size, offset, cursor) = self.extract_pagination_params(request);

        // Infer page size if not provided
        let page_size = page_size.unwrap_or_else(|| self.infer_page_size(request, &self.examples));

        // Generate realistic total count
        let total = self.generate_realistic_total(context, request).await?;

        // Calculate derived values
        let total_pages = total.div_ceil(page_size); // Ceiling division
        let current_page = page.unwrap_or(1);
        let has_next = current_page < total_pages;
        let has_prev = current_page > 1;

        // Generate cursors if using cursor-based pagination
        let (next_cursor, prev_cursor) =
            if self.default_rule.format == PaginationFormat::CursorBased {
                (
                    if has_next {
                        Some(self.generate_cursor(current_page + 1))
                    } else {
                        None
                    },
                    if has_prev {
                        Some(self.generate_cursor(current_page - 1))
                    } else {
                        None
                    },
                )
            } else {
                (None, None)
            };

        // Calculate offset if using offset-based pagination
        let calculated_offset = if self.default_rule.format == PaginationFormat::OffsetBased {
            Some(offset.unwrap_or_else(|| (current_page - 1) * page_size))
        } else {
            offset
        };

        Ok(PaginationMetadata {
            page: Some(current_page),
            page_size,
            total,
            total_pages,
            has_next,
            has_prev,
            offset: calculated_offset,
            next_cursor,
            prev_cursor,
        })
    }

    /// Infer page size from request and examples
    pub fn infer_page_size(
        &self,
        request: &PaginationRequest,
        examples: &[PaginatedResponse],
    ) -> usize {
        // Check if request specifies page size
        for (key, value) in &request.query_params {
            if matches!(key.to_lowercase().as_str(), "limit" | "per_page" | "page_size" | "size") {
                if let Ok(size) = value.parse::<usize>() {
                    return size
                        .clamp(self.default_rule.min_page_size, self.default_rule.max_page_size);
                }
            }
        }

        // Use most common page size from examples
        if let Some(most_common) = self.find_most_common_page_size(examples) {
            return most_common;
        }

        // Fallback to default
        self.default_rule.default_page_size
    }

    /// Generate realistic total count using LLM or heuristics
    pub async fn generate_realistic_total(
        &self,
        context: &StatefulAiContext,
        request: &PaginationRequest,
    ) -> Result<usize> {
        // If LLM is available, use it to generate realistic total
        if let Some(ref llm_client) = self.llm_client {
            return self.generate_total_with_llm(context, request).await;
        }

        // Fallback to heuristic-based generation
        Ok(self.generate_total_heuristic(context, request))
    }

    // ===== Private helper methods =====

    /// Extract pagination parameters from request
    fn extract_pagination_params(
        &self,
        request: &PaginationRequest,
    ) -> (Option<usize>, Option<usize>, Option<usize>, Option<String>) {
        let mut page = None;
        let mut page_size = None;
        let mut offset = None;
        let mut cursor = None;

        for (key, value) in &request.query_params {
            match key.to_lowercase().as_str() {
                "page" | "p" => {
                    if let Ok(p) = value.parse::<usize>() {
                        page = Some(p);
                    }
                }
                "limit" | "per_page" | "page_size" | "size" => {
                    if let Ok(size) = value.parse::<usize>() {
                        page_size = Some(size);
                    }
                }
                "offset" => {
                    if let Ok(o) = value.parse::<usize>() {
                        offset = Some(o);
                    }
                }
                "cursor" => {
                    cursor = Some(value.clone());
                }
                _ => {}
            }
        }

        (page, page_size, offset, cursor)
    }

    /// Find most common page size in examples
    fn find_most_common_page_size(&self, examples: &[PaginatedResponse]) -> Option<usize> {
        let mut size_counts: HashMap<usize, usize> = HashMap::new();

        for example in examples {
            if let Some(size) = example.page_size {
                *size_counts.entry(size).or_insert(0) += 1;
            }
        }

        size_counts.into_iter().max_by_key(|(_, count)| *count).map(|(size, _)| size)
    }

    /// Update default rule from examples
    fn update_rule_from_examples(&mut self) {
        if self.examples.is_empty() {
            return;
        }

        // Update page size statistics
        let page_sizes: Vec<usize> = self.examples.iter().filter_map(|e| e.page_size).collect();

        if !page_sizes.is_empty() {
            self.default_rule.default_page_size = *page_sizes.iter().min().unwrap();
            self.default_rule.max_page_size = *page_sizes.iter().max().unwrap();
        }

        // Detect pagination format
        let mut has_offset = false;
        let mut has_cursor = false;
        let mut has_page = false;

        for example in &self.examples {
            for key in example.query_params.keys() {
                match key.to_lowercase().as_str() {
                    "offset" => has_offset = true,
                    "cursor" => has_cursor = true,
                    "page" | "p" => has_page = true,
                    _ => {}
                }
            }
        }

        self.default_rule.format = if has_cursor {
            PaginationFormat::CursorBased
        } else if has_offset {
            PaginationFormat::OffsetBased
        } else {
            PaginationFormat::PageBased
        };
    }

    /// Generate total count using LLM
    async fn generate_total_with_llm(
        &self,
        context: &StatefulAiContext,
        request: &PaginationRequest,
    ) -> Result<usize> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not available"))?;

        // Build context about the request
        let context_summary = context.build_context_summary().await;
        let request_summary = format!("Path: {}, Query: {:?}", request.path, request.query_params);

        let system_prompt = "You are a pagination metadata generator. Generate realistic total item counts for API responses.";
        let user_prompt = format!(
            "Based on this API request context, generate a realistic total item count:\n\n{}\n\n{}\n\nReturn only a number between 0 and 10000. Consider the context and make it realistic.",
            context_summary,
            request_summary
        );

        let request_llm = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.5, // Some variation but not too much
            max_tokens: 50,
            schema: None,
        };

        let response = llm_client.generate(&request_llm).await?;

        // Extract number from response
        if let Some(num_str) = response.as_str() {
            // Try to extract first number from response
            if let Some(num) =
                num_str.split_whitespace().find_map(|word| word.parse::<usize>().ok())
            {
                return Ok(num.clamp(0, 10000));
            }
        }

        // Fallback to heuristic
        Ok(self.generate_total_heuristic(context, request))
    }

    /// Generate total count using heuristics
    fn generate_total_heuristic(
        &self,
        _context: &StatefulAiContext,
        _request: &PaginationRequest,
    ) -> usize {
        // Simple heuristic: generate a random-ish total between 50 and 500
        // In a real implementation, this could be based on:
        // - Session state (e.g., number of items in cart)
        // - Request path (e.g., /api/users might have more than /api/admin/users)
        // - Historical data

        // For now, use a simple range
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        _request.path.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate number between 50 and 500 based on hash
        let base = 50;
        let range = 450;

        base + (hash % range as u64) as usize
    }

    /// Generate cursor for cursor-based pagination
    fn generate_cursor(&self, page: usize) -> String {
        // Simple cursor encoding (in production, use proper base64 or encryption)
        // For now, use a simple format that can be decoded
        format!("cursor_{}", page)
    }
}

impl Default for PaginationIntelligence {
    fn default() -> Self {
        Self::new(BehaviorModelConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_extract_pagination_params() {
        let config = BehaviorModelConfig::default();
        let intelligence = PaginationIntelligence::new(config);

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "2".to_string());
        query_params.insert("limit".to_string(), "25".to_string());

        let request = PaginationRequest {
            path: "/api/users".to_string(),
            query_params,
            request_body: None,
        };

        let (page, page_size, offset, cursor) = intelligence.extract_pagination_params(&request);

        assert_eq!(page, Some(2));
        assert_eq!(page_size, Some(25));
        assert_eq!(offset, None);
        assert_eq!(cursor, None);
    }

    #[tokio::test]
    async fn test_infer_page_size() {
        let config = BehaviorModelConfig::default();
        let intelligence = PaginationIntelligence::new(config);

        let mut query_params = HashMap::new();
        query_params.insert("limit".to_string(), "30".to_string());

        let request = PaginationRequest {
            path: "/api/users".to_string(),
            query_params,
            request_body: None,
        };

        let examples = vec![PaginatedResponse {
            path: "/api/users".to_string(),
            query_params: HashMap::new(),
            response: json!({}),
            page: Some(1),
            page_size: Some(20),
            total: Some(100),
        }];

        let page_size = intelligence.infer_page_size(&request, &examples);
        assert_eq!(page_size, 30); // Should use request parameter
    }

    #[test]
    fn test_find_most_common_page_size() {
        let config = BehaviorModelConfig::default();
        let intelligence = PaginationIntelligence::new(config);

        let examples = vec![
            PaginatedResponse {
                path: "/api/users".to_string(),
                query_params: HashMap::new(),
                response: json!({}),
                page: Some(1),
                page_size: Some(20),
                total: Some(100),
            },
            PaginatedResponse {
                path: "/api/users".to_string(),
                query_params: HashMap::new(),
                response: json!({}),
                page: Some(2),
                page_size: Some(20),
                total: Some(100),
            },
            PaginatedResponse {
                path: "/api/users".to_string(),
                query_params: HashMap::new(),
                response: json!({}),
                page: Some(1),
                page_size: Some(50),
                total: Some(200),
            },
        ];

        let most_common = intelligence.find_most_common_page_size(&examples);
        assert_eq!(most_common, Some(20)); // 20 appears twice, 50 appears once
    }
}

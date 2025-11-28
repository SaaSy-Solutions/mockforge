//! Request capture manager for storing and retrieving captured requests
//!
//! This module provides a centralized manager for storing captured requests from
//! various sources and making them available for contract diff analysis.

use crate::ai_contract_diff::CapturedRequest;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metadata about a captured request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureMetadata {
    /// Unique capture ID
    pub id: String,

    /// Source of the capture
    pub source: String,

    /// Timestamp when request was captured
    pub captured_at: DateTime<Utc>,

    /// Whether this request has been analyzed
    pub analyzed: bool,

    /// Contract specification ID this request was analyzed against (if analyzed)
    pub contract_id: Option<String>,

    /// Analysis result ID (if analyzed)
    pub analysis_result_id: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Query parameters for retrieving captured requests
#[derive(Debug, Clone, Default)]
pub struct CaptureQuery {
    /// Filter by source
    pub source: Option<String>,

    /// Filter by method
    pub method: Option<String>,

    /// Filter by path pattern
    pub path_pattern: Option<String>,

    /// Filter by date range (start)
    pub start_time: Option<DateTime<Utc>>,

    /// Filter by date range (end)
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by analyzed status
    pub analyzed: Option<bool>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Request capture manager
#[derive(Debug, Clone)]
pub struct CaptureManager {
    /// Ring buffer of captured requests (most recent first)
    captures: Arc<RwLock<VecDeque<(CapturedRequest, CaptureMetadata)>>>,

    /// Maximum number of captures to keep in memory
    max_captures: usize,

    /// Index by source for fast lookup
    source_index: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Index by contract ID for fast lookup
    contract_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl CaptureManager {
    /// Create a new capture manager
    pub fn new(max_captures: usize) -> Self {
        Self {
            captures: Arc::new(RwLock::new(VecDeque::new())),
            max_captures,
            source_index: Arc::new(RwLock::new(HashMap::new())),
            contract_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Capture a new request
    pub async fn capture(&self, request: CapturedRequest) -> Result<String> {
        let capture_id = uuid::Uuid::new_v4().to_string();

        let metadata = CaptureMetadata {
            id: capture_id.clone(),
            source: request.source.clone(),
            captured_at: request.timestamp,
            analyzed: false,
            contract_id: None,
            analysis_result_id: None,
            metadata: request.metadata.clone(),
        };

        // Add to captures
        let mut captures = self.captures.write().await;
        captures.push_front((request.clone(), metadata.clone()));

        // Maintain size limit
        while captures.len() > self.max_captures {
            if let Some((_, removed_metadata)) = captures.pop_back() {
                // Remove from indexes
                self.remove_from_indexes(&removed_metadata).await;
            }
        }

        // Add to indexes
        self.add_to_indexes(&metadata).await;

        Ok(capture_id)
    }

    /// Get recent captures
    pub async fn get_recent_captures(
        &self,
        limit: Option<usize>,
    ) -> Vec<(CapturedRequest, CaptureMetadata)> {
        let captures = self.captures.read().await;
        let take_count = limit.unwrap_or(captures.len()).min(captures.len());
        captures.iter().take(take_count).cloned().collect()
    }

    /// Query captures with filters
    pub async fn query_captures(
        &self,
        query: CaptureQuery,
    ) -> Vec<(CapturedRequest, CaptureMetadata)> {
        let captures = self.captures.read().await;
        let mut results = Vec::new();

        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        for (request, metadata) in captures.iter() {
            // Apply filters
            if let Some(ref source) = query.source {
                if &metadata.source != source {
                    continue;
                }
            }

            if let Some(ref method) = query.method {
                if &request.method != method {
                    continue;
                }
            }

            if let Some(ref path_pattern) = query.path_pattern {
                if !request.path.contains(path_pattern) {
                    continue;
                }
            }

            if let Some(start_time) = query.start_time {
                if metadata.captured_at < start_time {
                    continue;
                }
            }

            if let Some(end_time) = query.end_time {
                if metadata.captured_at > end_time {
                    continue;
                }
            }

            if let Some(analyzed) = query.analyzed {
                if metadata.analyzed != analyzed {
                    continue;
                }
            }

            results.push((request.clone(), metadata.clone()));
        }

        // Apply pagination
        results.into_iter().skip(offset).take(limit).collect()
    }

    /// Get capture by ID
    pub async fn get_capture(
        &self,
        capture_id: &str,
    ) -> Option<(CapturedRequest, CaptureMetadata)> {
        let captures = self.captures.read().await;
        captures.iter().find(|(_, metadata)| metadata.id == capture_id).cloned()
    }

    /// Mark capture as analyzed
    pub async fn mark_analyzed(
        &self,
        capture_id: &str,
        contract_id: &str,
        analysis_result_id: &str,
    ) -> Result<()> {
        let mut captures = self.captures.write().await;

        for (_, metadata) in captures.iter_mut() {
            if metadata.id == capture_id {
                metadata.analyzed = true;
                metadata.contract_id = Some(contract_id.to_string());
                metadata.analysis_result_id = Some(analysis_result_id.to_string());

                // Update contract index
                let mut contract_index = self.contract_index.write().await;
                contract_index
                    .entry(contract_id.to_string())
                    .or_insert_with(Vec::new)
                    .push(capture_id.to_string());

                return Ok(());
            }
        }

        Err(crate::Error::generic(format!("Capture not found: {}", capture_id)))
    }

    /// Get captures by source
    pub async fn get_captures_by_source(
        &self,
        source: &str,
        limit: Option<usize>,
    ) -> Vec<(CapturedRequest, CaptureMetadata)> {
        let query = CaptureQuery {
            source: Some(source.to_string()),
            limit,
            ..Default::default()
        };
        self.query_captures(query).await
    }

    /// Get captures by contract ID
    pub async fn get_captures_by_contract(
        &self,
        contract_id: &str,
    ) -> Vec<(CapturedRequest, CaptureMetadata)> {
        let contract_index = self.contract_index.read().await;
        if let Some(capture_ids) = contract_index.get(contract_id) {
            let mut results = Vec::new();
            for capture_id in capture_ids {
                if let Some(capture) = self.get_capture(capture_id).await {
                    results.push(capture);
                }
            }
            results
        } else {
            Vec::new()
        }
    }

    /// Get capture statistics
    pub async fn get_statistics(&self) -> CaptureStatistics {
        let captures = self.captures.read().await;

        let mut by_source: HashMap<String, usize> = HashMap::new();
        let mut by_method: HashMap<String, usize> = HashMap::new();
        let mut total_analyzed = 0;
        let mut total_unanalyzed = 0;

        for (request, metadata) in captures.iter() {
            *by_source.entry(metadata.source.clone()).or_insert(0) += 1;
            *by_method.entry(request.method.clone()).or_insert(0) += 1;

            if metadata.analyzed {
                total_analyzed += 1;
            } else {
                total_unanalyzed += 1;
            }
        }

        CaptureStatistics {
            total_captures: captures.len(),
            total_analyzed,
            total_unanalyzed,
            by_source,
            by_method,
        }
    }

    /// Clear all captures
    pub async fn clear_captures(&self) {
        let mut captures = self.captures.write().await;
        captures.clear();

        let mut source_index = self.source_index.write().await;
        source_index.clear();

        let mut contract_index = self.contract_index.write().await;
        contract_index.clear();
    }

    /// Add to indexes
    async fn add_to_indexes(&self, metadata: &CaptureMetadata) {
        // Add to source index
        let mut source_index = self.source_index.write().await;
        source_index
            .entry(metadata.source.clone())
            .or_insert_with(Vec::new)
            .push(metadata.id.clone());
    }

    /// Remove from indexes
    async fn remove_from_indexes(&self, metadata: &CaptureMetadata) {
        // Remove from source index
        let mut source_index = self.source_index.write().await;
        if let Some(ids) = source_index.get_mut(&metadata.source) {
            ids.retain(|id| id != &metadata.id);
            if ids.is_empty() {
                source_index.remove(&metadata.source);
            }
        }

        // Remove from contract index if analyzed
        if let Some(ref contract_id) = metadata.contract_id {
            let mut contract_index = self.contract_index.write().await;
            if let Some(ids) = contract_index.get_mut(contract_id) {
                ids.retain(|id| id != &metadata.id);
                if ids.is_empty() {
                    contract_index.remove(contract_id);
                }
            }
        }
    }
}

/// Capture statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatistics {
    /// Total number of captures
    pub total_captures: usize,

    /// Number of analyzed captures
    pub total_analyzed: usize,

    /// Number of unanalyzed captures
    pub total_unanalyzed: usize,

    /// Captures grouped by source
    pub by_source: HashMap<String, usize>,

    /// Captures grouped by HTTP method
    pub by_method: HashMap<String, usize>,
}

/// Global singleton instance of the capture manager
static GLOBAL_CAPTURE_MANAGER: once_cell::sync::OnceCell<CaptureManager> =
    once_cell::sync::OnceCell::new();

/// Initialize the global capture manager
pub fn init_global_capture_manager(max_captures: usize) -> &'static CaptureManager {
    GLOBAL_CAPTURE_MANAGER.get_or_init(|| CaptureManager::new(max_captures))
}

/// Get reference to the global capture manager
pub fn get_global_capture_manager() -> Option<&'static CaptureManager> {
    GLOBAL_CAPTURE_MANAGER.get()
}

/// Capture a request to the global manager (convenience function)
pub async fn capture_request_global(request: CapturedRequest) -> Result<String> {
    if let Some(manager) = get_global_capture_manager() {
        manager.capture(request).await
    } else {
        Err(crate::Error::generic("Capture manager not initialized"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_contract_diff::CapturedRequest;

    #[tokio::test]
    async fn test_capture_manager_new() {
        let manager = CaptureManager::new(100);
        assert_eq!(manager.max_captures, 100);
    }

    #[tokio::test]
    async fn test_capture_request() {
        let manager = CaptureManager::new(10);
        let request = CapturedRequest::new("POST", "/api/users", "browser_extension");

        let capture_id = manager.capture(request).await.unwrap();
        assert!(!capture_id.is_empty());

        let captures = manager.get_recent_captures(None).await;
        assert_eq!(captures.len(), 1);
    }

    #[tokio::test]
    async fn test_capture_manager_maintains_size_limit() {
        let manager = CaptureManager::new(5);

        for i in 0..10 {
            let request = CapturedRequest::new("GET", &format!("/api/test{}", i), "proxy");
            manager.capture(request).await.unwrap();
        }

        let captures = manager.get_recent_captures(None).await;
        assert_eq!(captures.len(), 5);
    }

    #[tokio::test]
    async fn test_query_captures_by_source() {
        let manager = CaptureManager::new(100);

        manager
            .capture(CapturedRequest::new("GET", "/api/users", "browser_extension"))
            .await
            .unwrap();
        manager
            .capture(CapturedRequest::new("POST", "/api/users", "proxy"))
            .await
            .unwrap();
        manager
            .capture(CapturedRequest::new("GET", "/api/posts", "browser_extension"))
            .await
            .unwrap();

        let browser_captures = manager.get_captures_by_source("browser_extension", None).await;
        assert_eq!(browser_captures.len(), 2);

        let proxy_captures = manager.get_captures_by_source("proxy", None).await;
        assert_eq!(proxy_captures.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_analyzed() {
        let manager = CaptureManager::new(100);
        let request = CapturedRequest::new("POST", "/api/users", "browser_extension");

        let capture_id = manager.capture(request).await.unwrap();

        manager.mark_analyzed(&capture_id, "contract_123", "result_456").await.unwrap();

        let (_, metadata) = manager.get_capture(&capture_id).await.unwrap();
        assert!(metadata.analyzed);
        assert_eq!(metadata.contract_id, Some("contract_123".to_string()));
        assert_eq!(metadata.analysis_result_id, Some("result_456".to_string()));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let manager = CaptureManager::new(100);

        manager
            .capture(CapturedRequest::new("GET", "/api/users", "browser_extension"))
            .await
            .unwrap();
        manager
            .capture(CapturedRequest::new("POST", "/api/users", "proxy"))
            .await
            .unwrap();

        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_captures, 2);
        assert_eq!(stats.total_unanalyzed, 2);
        assert_eq!(stats.by_source.get("browser_extension"), Some(&1));
        assert_eq!(stats.by_source.get("proxy"), Some(&1));
    }
}

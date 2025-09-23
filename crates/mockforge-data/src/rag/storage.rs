//! Document storage and vector indexing
//!
//! This module provides storage backends for documents and their vector embeddings,
//! supporting various indexing strategies and similarity search algorithms.

use crate::rag::engine::DocumentChunk;
use mockforge_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vector index for similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndex {
    /// Index ID
    pub id: String,
    /// Index name
    pub name: String,
    /// Index type
    pub index_type: IndexType,
    /// Vector dimensions
    pub dimensions: usize,
    /// Number of vectors indexed
    pub vector_count: usize,
    /// Index metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl VectorIndex {
    /// Create a new vector index
    pub fn new(id: String, name: String, index_type: IndexType, dimensions: usize) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            index_type,
            dimensions,
            vector_count: 0,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add metadata to index
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Remove metadata
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        let result = self.metadata.remove(key);
        if result.is_some() {
            self.updated_at = chrono::Utc::now();
        }
        result
    }

    /// Update vector count
    pub fn update_vector_count(&mut self, count: usize) {
        self.vector_count = count;
        self.updated_at = chrono::Utc::now();
    }

    /// Get index size estimate in bytes
    pub fn estimated_size_bytes(&self) -> u64 {
        // Rough estimate: each vector takes ~4 bytes per dimension + overhead
        (self.vector_count * self.dimensions * 4 + 1024) as u64
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.vector_count == 0
    }

    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            id: self.id.clone(),
            name: self.name.clone(),
            index_type: self.index_type.clone(),
            dimensions: self.dimensions,
            vector_count: self.vector_count,
            estimated_size_bytes: self.estimated_size_bytes(),
            metadata_count: self.metadata.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Index ID
    pub id: String,
    /// Index name
    pub name: String,
    /// Index type
    pub index_type: IndexType,
    /// Vector dimensions
    pub dimensions: usize,
    /// Number of vectors
    pub vector_count: usize,
    /// Estimated size in bytes
    pub estimated_size_bytes: u64,
    /// Number of metadata entries
    pub metadata_count: usize,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Index type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexType {
    /// Flat index - brute force search
    Flat,
    /// IVF (Inverted File) index - for large datasets
    IVF,
    /// HNSW (Hierarchical Navigable Small World) index - for high performance
    HNSW,
    /// PQ (Product Quantization) index - for memory efficiency
    PQ,
    /// Custom index type
    Custom(String),
}

impl Default for IndexType {
    fn default() -> Self {
        Self::Flat
    }
}

/// Search parameters for vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    /// Number of results to return
    pub top_k: usize,
    /// Similarity threshold (0.0 to 1.0)
    pub threshold: f32,
    /// Search method to use
    pub search_method: SearchMethod,
    /// Include metadata in results
    pub include_metadata: bool,
    /// Filter by document ID
    pub document_filter: Option<String>,
    /// Filter by metadata
    pub metadata_filter: Option<HashMap<String, String>>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            top_k: 10,
            threshold: 0.7,
            search_method: SearchMethod::Cosine,
            include_metadata: true,
            document_filter: None,
            metadata_filter: None,
        }
    }
}

/// Search method enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMethod {
    /// Cosine similarity
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Dot product
    DotProduct,
    /// Manhattan distance
    Manhattan,
}

impl Default for SearchMethod {
    fn default() -> Self {
        Self::Cosine
    }
}

/// Storage backend trait for documents and vectors
#[async_trait::async_trait]
pub trait DocumentStorage: Send + Sync {
    /// Store document chunks
    async fn store_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<()>;

    /// Search for similar chunks
    async fn search_similar(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<DocumentChunk>>;

    /// Search with custom parameters
    async fn search_with_params(&self, query_embedding: &[f32], params: SearchParams) -> Result<Vec<DocumentChunk>>;

    /// Get chunk by ID
    async fn get_chunk(&self, chunk_id: &str) -> Result<Option<DocumentChunk>>;

    /// Delete chunk by ID
    async fn delete_chunk(&self, chunk_id: &str) -> Result<bool>;

    /// Get chunks by document ID
    async fn get_chunks_by_document(&self, document_id: &str) -> Result<Vec<DocumentChunk>>;

    /// Delete all chunks for a document
    async fn delete_document(&self, document_id: &str) -> Result<usize>;

    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;

    /// List all document IDs
    async fn list_documents(&self) -> Result<Vec<String>>;

    /// Get total number of chunks
    async fn get_total_chunks(&self) -> Result<usize>;

    /// Clear all data
    async fn clear(&self) -> Result<()>;

    /// Optimize storage (rebuild indexes, compact data)
    async fn optimize(&self) -> Result<()>;

    /// Create backup
    async fn create_backup(&self, path: &str) -> Result<()>;

    /// Restore from backup
    async fn restore_backup(&self, path: &str) -> Result<()>;

    /// Check storage health
    async fn health_check(&self) -> Result<StorageHealth>;
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of documents
    pub total_documents: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Storage backend type
    pub backend_type: String,
    /// Available disk space in bytes
    pub available_space_bytes: u64,
    /// Used space in bytes
    pub used_space_bytes: u64,
}

/// Storage health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Health check timestamp
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// Detailed health information
    pub details: HashMap<String, String>,
    /// Performance metrics
    pub metrics: Option<StorageMetrics>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// Storage is healthy
    Healthy,
    /// Storage has warnings
    Warning,
    /// Storage is unhealthy
    Unhealthy,
    /// Storage is unavailable
    Unavailable,
}

/// Storage performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Average search time in milliseconds
    pub average_search_time_ms: f64,
    /// Average insert time in milliseconds
    pub average_insert_time_ms: f64,
    /// Index fragmentation percentage (0.0 to 1.0)
    pub fragmentation_ratio: f32,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f32,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Disk usage in bytes
    pub disk_usage_bytes: u64,
}

/// In-memory storage implementation for development and testing
pub struct InMemoryStorage {
    chunks: Arc<RwLock<HashMap<String, DocumentChunk>>>,
    vectors: Arc<RwLock<Vec<(String, Vec<f32>)>>>,
    stats: Arc<RwLock<StorageStats>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            vectors: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(StorageStats {
                total_documents: 0,
                total_chunks: 0,
                index_size_bytes: 0,
                last_updated: now,
                backend_type: "memory".to_string(),
                available_space_bytes: u64::MAX,
                used_space_bytes: 0,
            })),
        }
    }

    /// Calculate cosine similarity
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DocumentStorage for InMemoryStorage {
    async fn store_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<()> {
        let mut chunks_map = self.chunks.write().await;
        let mut vectors = self.vectors.write().await;
        let mut stats = self.stats.write().await;

        for chunk in chunks {
            chunks_map.insert(chunk.id.clone(), chunk.clone());

            // Store vector for similarity search
            vectors.push((chunk.id.clone(), chunk.embedding.clone()));

            stats.total_chunks += 1;
        }

        stats.last_updated = chrono::Utc::now();
        stats.index_size_bytes = (stats.total_chunks * 1536 * 4) as u64; // Rough estimate
        stats.used_space_bytes = stats.index_size_bytes;

        Ok(())
    }

    async fn search_similar(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<DocumentChunk>> {
        let vectors = self.vectors.read().await;
        let chunks = self.chunks.read().await;

        let mut similarities: Vec<(String, f32)> = vectors.iter()
            .map(|(chunk_id, embedding)| {
                let similarity = self.cosine_similarity(query_embedding, embedding);
                (chunk_id.clone(), similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k results
        let mut results = Vec::new();
        for (chunk_id, _) in similarities.iter().take(top_k) {
            if let Some(chunk) = chunks.get(chunk_id) {
                results.push(chunk.clone());
            }
        }

        Ok(results)
    }

    async fn search_with_params(&self, query_embedding: &[f32], params: SearchParams) -> Result<Vec<DocumentChunk>> {
        let mut results = self.search_similar(query_embedding, params.top_k * 2).await?; // Get more candidates

        // Apply filters
        if let Some(document_filter) = &params.document_filter {
            results.retain(|chunk| chunk.document_id == *document_filter);
        }

        if let Some(metadata_filter) = &params.metadata_filter {
            results.retain(|chunk| {
                metadata_filter.iter().all(|(key, value)| {
                    chunk.get_metadata(key).map(|v| v == value).unwrap_or(false)
                })
            });
        }

        // Apply threshold filter
        results.retain(|chunk| {
            let similarity = self.cosine_similarity(query_embedding, &chunk.embedding);
            similarity >= params.threshold
        });

        // Sort by similarity
        results.sort_by(|a, b| {
            let sim_a = self.cosine_similarity(query_embedding, &a.embedding);
            let sim_b = self.cosine_similarity(query_embedding, &b.embedding);
            sim_b.partial_cmp(&sim_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top-k
        results.truncate(params.top_k);

        Ok(results)
    }

    async fn get_chunk(&self, chunk_id: &str) -> Result<Option<DocumentChunk>> {
        let chunks = self.chunks.read().await;
        Ok(chunks.get(chunk_id).cloned())
    }

    async fn delete_chunk(&self, chunk_id: &str) -> Result<bool> {
        let mut chunks = self.chunks.write().await;
        let mut vectors = self.vectors.write().await;
        let mut stats = self.stats.write().await;

        let chunk_removed = chunks.remove(chunk_id).is_some();
        let _vector_removed = vectors.retain(|(id, _)| id != chunk_id);

        if chunk_removed {
            stats.total_chunks = stats.total_chunks.saturating_sub(1);
            stats.last_updated = chrono::Utc::now();
            stats.index_size_bytes = (stats.total_chunks * 1536 * 4) as u64;
            stats.used_space_bytes = stats.index_size_bytes;
        }

        Ok(chunk_removed)
    }

    async fn get_chunks_by_document(&self, document_id: &str) -> Result<Vec<DocumentChunk>> {
        let chunks = self.chunks.read().await;
        let results = chunks.values()
            .filter(|chunk| chunk.document_id == document_id)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn delete_document(&self, document_id: &str) -> Result<usize> {
        let mut chunks = self.chunks.write().await;
        let mut vectors = self.vectors.write().await;
        let mut stats = self.stats.write().await;

        let initial_count = chunks.len();
        chunks.retain(|_, chunk| chunk.document_id != document_id);
        vectors.retain(|(id, _)| {
            // Remove vectors for chunks that were removed
            chunks.contains_key(id)
        });

        let removed_count = initial_count - chunks.len();
        if removed_count > 0 {
            stats.total_chunks = stats.total_chunks.saturating_sub(removed_count);
            stats.last_updated = chrono::Utc::now();
            stats.index_size_bytes = (stats.total_chunks * 1536 * 4) as u64;
            stats.used_space_bytes = stats.index_size_bytes;
        }

        Ok(removed_count)
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn list_documents(&self) -> Result<Vec<String>> {
        let chunks = self.chunks.read().await;
        let documents: std::collections::HashSet<String> = chunks.values()
            .map(|chunk| chunk.document_id.clone())
            .collect();
        Ok(documents.into_iter().collect())
    }

    async fn get_total_chunks(&self) -> Result<usize> {
        let stats = self.stats.read().await;
        Ok(stats.total_chunks)
    }

    async fn clear(&self) -> Result<()> {
        let mut chunks = self.chunks.write().await;
        let mut vectors = self.vectors.write().await;
        let mut stats = self.stats.write().await;

        chunks.clear();
        vectors.clear();

        stats.total_documents = 0;
        stats.total_chunks = 0;
        stats.index_size_bytes = 0;
        stats.used_space_bytes = 0;
        stats.last_updated = chrono::Utc::now();

        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        // In-memory storage doesn't need optimization
        Ok(())
    }

    async fn create_backup(&self, _path: &str) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }

    async fn restore_backup(&self, _path: &str) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }

    async fn health_check(&self) -> Result<StorageHealth> {
        let chunks = self.chunks.read().await;
        let vectors = self.vectors.read().await;

        let mut details = HashMap::new();
        details.insert("chunk_count".to_string(), chunks.len().to_string());
        details.insert("vector_count".to_string(), vectors.len().to_string());
        details.insert("memory_usage".to_string(), "unknown".to_string());

        let status = if chunks.len() == vectors.len() {
            HealthStatus::Healthy
        } else {
            details.insert("error".to_string(), "Chunk/vector count mismatch".to_string());
            HealthStatus::Unhealthy
        };

        Ok(StorageHealth {
            status,
            checked_at: chrono::Utc::now(),
            details,
            metrics: None,
        })
    }
}

/// Storage factory for creating different storage backends
pub struct StorageFactory;

impl StorageFactory {
    /// Create in-memory storage
    pub fn create_memory() -> Box<dyn DocumentStorage> {
        Box::new(InMemoryStorage::new())
    }

    /// Create file-based storage
    pub fn create_file(_path: &str) -> Result<Box<dyn DocumentStorage>> {
        // Placeholder for file-based storage implementation
        Err(mockforge_core::Error::generic("File storage not yet implemented"))
    }

    /// Create database storage
    pub fn create_database(_connection_string: &str) -> Result<Box<dyn DocumentStorage>> {
        // Placeholder for database storage implementation
        Err(mockforge_core::Error::generic("Database storage not yet implemented"))
    }

    /// Create vector database storage
    pub fn create_vector_db(_config: HashMap<String, String>) -> Result<Box<dyn DocumentStorage>> {
        // Placeholder for vector database storage implementation
        Err(mockforge_core::Error::generic("Vector database storage not yet implemented"))
    }
}

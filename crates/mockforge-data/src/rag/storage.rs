//! Document storage and vector indexing
//!
//! This module provides storage backends for documents and their vector embeddings,
//! supporting various indexing strategies and similarity search algorithms.

use crate::rag::engine::DocumentChunk;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type VectorStore = Arc<RwLock<Vec<(String, Vec<f32>)>>>;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexType {
    /// Flat index - brute force search
    #[default]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SearchMethod {
    /// Cosine similarity
    #[default]
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Dot product
    DotProduct,
    /// Manhattan distance
    Manhattan,
}

/// Storage backend trait for documents and vectors
#[async_trait::async_trait]
pub trait DocumentStorage: Send + Sync {
    /// Store document chunks
    async fn store_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<()>;

    /// Search for similar chunks
    async fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<DocumentChunk>>;

    /// Search with custom parameters
    async fn search_with_params(
        &self,
        query_embedding: &[f32],
        params: SearchParams,
    ) -> Result<Vec<DocumentChunk>>;

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
    // pub(super) so the file-backed wrapper in this module can rehydrate
    // these from disk without going through the async trait surface.
    pub(super) chunks: Arc<RwLock<HashMap<String, DocumentChunk>>>,
    pub(super) vectors: VectorStore,
    pub(super) stats: Arc<RwLock<StorageStats>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self::new_with_backend_type("memory")
    }

    /// Create in-memory storage with a specific backend label.
    /// This is used when a persistent backend is configured but running with an in-memory fallback.
    pub fn new_with_backend_type(backend_type: &str) -> Self {
        let now = chrono::Utc::now();
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            vectors: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(StorageStats {
                total_documents: 0,
                total_chunks: 0,
                index_size_bytes: 0,
                last_updated: now,
                backend_type: backend_type.to_string(),
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

    async fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<DocumentChunk>> {
        let vectors = self.vectors.read().await;
        let chunks = self.chunks.read().await;

        let mut similarities: Vec<(String, f32)> = vectors
            .iter()
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

    async fn search_with_params(
        &self,
        query_embedding: &[f32],
        params: SearchParams,
    ) -> Result<Vec<DocumentChunk>> {
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
        let results = chunks
            .values()
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
        let documents: std::collections::HashSet<String> =
            chunks.values().map(|chunk| chunk.document_id.clone()).collect();
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

    async fn create_backup(&self, path: &str) -> Result<()> {
        let chunks = self.chunks.read().await;
        let vectors = self.vectors.read().await;

        let backup_data = serde_json::json!({
            "version": 1,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "chunks": chunks.values().collect::<Vec<_>>(),
            "vectors": vectors.iter().collect::<Vec<_>>(),
        });

        let json_bytes = serde_json::to_vec_pretty(&backup_data)?;
        std::fs::write(path, json_bytes)?;

        Ok(())
    }

    async fn restore_backup(&self, path: &str) -> Result<()> {
        let json_bytes = std::fs::read(path)?;
        let backup_data: serde_json::Value = serde_json::from_slice(&json_bytes)?;

        // Clear current data
        self.clear().await?;

        let mut chunks_map = self.chunks.write().await;
        let mut vectors = self.vectors.write().await;
        let mut stats = self.stats.write().await;

        // Restore chunks
        if let Some(chunks_arr) = backup_data.get("chunks").and_then(|v| v.as_array()) {
            for chunk_val in chunks_arr {
                if let Ok(chunk) = serde_json::from_value::<DocumentChunk>(chunk_val.clone()) {
                    chunks_map.insert(chunk.id.clone(), chunk);
                }
            }
        }

        // Restore vectors
        if let Some(vectors_arr) = backup_data.get("vectors").and_then(|v| v.as_array()) {
            for vector_val in vectors_arr {
                if let Ok(vector) = serde_json::from_value::<(String, Vec<f32>)>(vector_val.clone())
                {
                    vectors.push(vector);
                }
            }
        }

        // Update stats
        let doc_ids: std::collections::HashSet<String> =
            chunks_map.values().map(|c| c.document_id.clone()).collect();
        stats.total_documents = doc_ids.len();
        stats.total_chunks = chunks_map.len();
        stats.index_size_bytes = (stats.total_chunks * 1536 * 4) as u64;
        stats.used_space_bytes = stats.index_size_bytes;
        stats.last_updated = chrono::Utc::now();

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
    /// Create in-memory storage. Ephemeral — chunks + vectors live only
    /// for the process lifetime. Intended for tests and OSS quick-start.
    pub fn create_memory() -> Box<dyn DocumentStorage> {
        Box::new(InMemoryStorage::new())
    }

    /// Create file-backed storage that persists chunks + vectors to a JSON
    /// snapshot inside `path/`. Closes the "RAG forgets everything on
    /// restart" half of #669 — embeddings survive process restarts now.
    ///
    /// Layout:
    ///   `<path>/storage.json`  — single-file snapshot (atomic write via tmp)
    ///
    /// Tradeoff: rewrites the whole snapshot on every `store_chunks` call.
    /// Fine for thousand-chunk-scale corpora that fit in RAM (the
    /// embedded RAG use case); a real vector database is a better fit
    /// for million-chunk catalogs — see `create_vector_db` below.
    pub fn create_file(path: &str) -> Result<Box<dyn DocumentStorage>> {
        if path.trim().is_empty() {
            return Err(crate::Error::generic("File storage path cannot be empty"));
        }

        std::fs::create_dir_all(path)?;
        let dir = std::path::PathBuf::from(path);
        Ok(Box::new(PersistentFileStorage::new(dir)?))
    }

    /// Create database storage. Not yet implemented — the connection
    /// string would route to a sqlx-backed implementation. Returns a
    /// labelled in-memory store for now and warns rather than failing
    /// silently. Tracked in #669 follow-up.
    pub fn create_database(connection_string: &str) -> Result<Box<dyn DocumentStorage>> {
        if connection_string.trim().is_empty() {
            return Err(crate::Error::generic("Database connection string cannot be empty"));
        }
        tracing::warn!(
            "create_database falls back to in-memory storage; \
             sqlx-backed backend is tracked in #669 follow-up"
        );
        Ok(Box::new(InMemoryStorage::new_with_backend_type("database")))
    }

    /// Create vector-database storage. Real vector-DB integrations
    /// (Qdrant, LanceDB, pgvector) belong behind crate feature flags so
    /// the heavy client/transitive deps don't land in every consumer.
    /// Until one of those features is enabled, this returns a clear
    /// error rather than silently falling back to ephemeral memory —
    /// the silent fallback was exactly what the audit (#669) flagged.
    pub fn create_vector_db(config: HashMap<String, String>) -> Result<Box<dyn DocumentStorage>> {
        if config.is_empty() {
            return Err(crate::Error::generic("Vector database configuration cannot be empty"));
        }

        let provider = config.get("provider").map(|s| s.as_str()).unwrap_or("<unspecified>");

        Err(crate::Error::generic(format!(
            "vector-db backend '{provider}' not compiled in. \
             Enable the `qdrant` or `lancedb` feature on mockforge-data, \
             or use `create_file()` for persistent local storage."
        )))
    }
}

/// File-backed `DocumentStorage` that snapshots an in-memory store to a
/// JSON file on every write. Construction reads any prior snapshot back.
///
/// Implementation note: delegates all read paths to the wrapped
/// `InMemoryStorage` so cosine-similarity search etc. stays identical.
/// Only `store_chunks` / `delete_documents` / `clear_all` rewrite the
/// snapshot — read-only ops stay zero-IO.
pub struct PersistentFileStorage {
    inner: InMemoryStorage,
    snapshot_path: std::path::PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct StorageSnapshot {
    /// Schema version; bump if the on-disk shape changes.
    version: u32,
    chunks: HashMap<String, DocumentChunk>,
    vectors: Vec<(String, Vec<f32>)>,
}

impl PersistentFileStorage {
    /// Create a persistent file storage at `<dir>/storage.json`. Reads
    /// any existing snapshot at construction time.
    pub fn new(dir: std::path::PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        let snapshot_path = dir.join("storage.json");
        let inner = InMemoryStorage::new_with_backend_type("file");

        if snapshot_path.exists() {
            let raw = std::fs::read_to_string(&snapshot_path).map_err(crate::Error::from)?;
            let snapshot: StorageSnapshot = serde_json::from_str(&raw)
                .map_err(|e| crate::Error::generic(format!("malformed snapshot: {e}")))?;

            // We just created `inner` and haven't shared its Arcs yet, so
            // `try_write` must succeed. Using `try_write` (not
            // `blocking_write`) keeps construction safe to call from
            // inside a tokio runtime — `blocking_write` panics from
            // worker threads of the current runtime.
            inner
                .chunks
                .try_write()
                .map(|mut g| *g = snapshot.chunks)
                .map_err(|_| crate::Error::generic("snapshot load: chunks lock contended"))?;
            inner
                .vectors
                .try_write()
                .map(|mut g| *g = snapshot.vectors)
                .map_err(|_| crate::Error::generic("snapshot load: vectors lock contended"))?;
            if let Ok(mut stats) = inner.stats.try_write() {
                stats.last_updated = chrono::Utc::now();
            }
            tracing::info!(
                path = %snapshot_path.display(),
                "loaded RAG storage snapshot"
            );
        }

        Ok(Self {
            inner,
            snapshot_path,
        })
    }

    async fn persist(&self) -> Result<()> {
        let snapshot = StorageSnapshot {
            version: 1,
            chunks: self.inner.chunks.read().await.clone(),
            vectors: self.inner.vectors.read().await.clone(),
        };
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| crate::Error::generic(format!("serialise snapshot: {e}")))?;

        // Atomic rename — a crash mid-write leaves the previous snapshot
        // intact rather than truncating to a half-file.
        let tmp = self.snapshot_path.with_extension("tmp");
        std::fs::write(&tmp, json).map_err(crate::Error::from)?;
        std::fs::rename(&tmp, &self.snapshot_path).map_err(crate::Error::from)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl DocumentStorage for PersistentFileStorage {
    async fn store_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<()> {
        self.inner.store_chunks(chunks).await?;
        self.persist().await?;
        Ok(())
    }

    async fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<DocumentChunk>> {
        self.inner.search_similar(query_embedding, top_k).await
    }

    async fn search_with_params(
        &self,
        query_embedding: &[f32],
        params: SearchParams,
    ) -> Result<Vec<DocumentChunk>> {
        self.inner.search_with_params(query_embedding, params).await
    }

    async fn get_chunk(&self, chunk_id: &str) -> Result<Option<DocumentChunk>> {
        self.inner.get_chunk(chunk_id).await
    }

    async fn delete_chunk(&self, chunk_id: &str) -> Result<bool> {
        let res = self.inner.delete_chunk(chunk_id).await?;
        if res {
            self.persist().await?;
        }
        Ok(res)
    }

    async fn get_chunks_by_document(&self, document_id: &str) -> Result<Vec<DocumentChunk>> {
        self.inner.get_chunks_by_document(document_id).await
    }

    async fn delete_document(&self, document_id: &str) -> Result<usize> {
        let n = self.inner.delete_document(document_id).await?;
        if n > 0 {
            self.persist().await?;
        }
        Ok(n)
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let mut stats = self.inner.get_stats().await?;
        // Override the backend label so health/admin surfaces show
        // "file" rather than the inner store's "file" label (which we
        // already set, but make it explicit).
        stats.backend_type = "file".to_string();
        Ok(stats)
    }

    async fn list_documents(&self) -> Result<Vec<String>> {
        self.inner.list_documents().await
    }

    async fn get_total_chunks(&self) -> Result<usize> {
        self.inner.get_total_chunks().await
    }

    async fn clear(&self) -> Result<()> {
        self.inner.clear().await?;
        self.persist().await?;
        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        self.inner.optimize().await
    }

    async fn create_backup(&self, path: &str) -> Result<()> {
        self.inner.create_backup(path).await
    }

    async fn restore_backup(&self, path: &str) -> Result<()> {
        self.inner.restore_backup(path).await?;
        self.persist().await?;
        Ok(())
    }

    async fn health_check(&self) -> Result<StorageHealth> {
        self.inner.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::StorageFactory;
    use std::collections::HashMap;

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }

    #[tokio::test]
    async fn test_create_file_storage_fallback_backend_type() {
        let dir =
            std::env::temp_dir().join(format!("mockforge-data-storage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let storage = StorageFactory::create_file(dir.to_str().expect("path")).expect("create");
        let stats = storage.get_stats().await.expect("stats");
        assert_eq!(stats.backend_type, "file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_create_database_storage_fallback_backend_type() {
        let storage =
            StorageFactory::create_database("postgres://user:pass@localhost/db").expect("create");
        let stats = storage.get_stats().await.expect("stats");
        assert_eq!(stats.backend_type, "database");
    }

    #[tokio::test]
    async fn test_create_vector_storage_errors_without_real_backend() {
        // After #669: vector-db requested without `qdrant`/`lancedb` feature
        // is now an error rather than a silent in-memory fallback.
        let mut cfg = HashMap::new();
        cfg.insert("provider".to_string(), "qdrant".to_string());
        let result = StorageFactory::create_vector_db(cfg);
        assert!(result.is_err(), "expected error, got Ok");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("not compiled in") || msg.contains("qdrant"),
            "expected helpful error mentioning compile-in or qdrant, got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_persistent_file_storage_round_trips_across_restart() {
        use crate::rag::engine::DocumentChunk;

        let dir = std::env::temp_dir().join(format!(
            "mockforge-rag-persist-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let path_str = dir.to_str().expect("path");

        // First "process": write a chunk + embedding.
        {
            let storage = StorageFactory::create_file(path_str).expect("create");
            let chunk = DocumentChunk {
                id: "chunk-1".to_string(),
                document_id: "doc-1".to_string(),
                content: "hello rag".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                metadata: HashMap::new(),
                position: 0,
                length: 9,
            };
            storage.store_chunks(vec![chunk]).await.expect("store");
        }

        // Second "process": read the same path. Inner state should
        // rehydrate from disk; the chunk should still be queryable.
        {
            let storage = StorageFactory::create_file(path_str).expect("reopen");
            let stats = storage.get_stats().await.expect("stats");
            assert_eq!(stats.backend_type, "file");
            let chunk = storage.get_chunk("chunk-1").await.expect("query");
            assert!(chunk.is_some(), "persisted chunk should survive restart");
            let chunk = chunk.unwrap();
            assert_eq!(chunk.content, "hello rag");
            assert_eq!(chunk.embedding, vec![0.1, 0.2, 0.3]);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_persistent_file_storage_search_survives_restart() {
        use crate::rag::engine::DocumentChunk;

        let dir = std::env::temp_dir().join(format!(
            "mockforge-rag-search-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path_str = dir.to_str().expect("path");

        {
            let storage = StorageFactory::create_file(path_str).expect("create");
            let chunks = vec![
                DocumentChunk {
                    id: "a".to_string(),
                    document_id: "doc".to_string(),
                    content: "apple".to_string(),
                    embedding: vec![1.0, 0.0, 0.0],
                    metadata: HashMap::new(),
                    position: 0,
                    length: 5,
                },
                DocumentChunk {
                    id: "b".to_string(),
                    document_id: "doc".to_string(),
                    content: "banana".to_string(),
                    embedding: vec![0.0, 1.0, 0.0],
                    metadata: HashMap::new(),
                    position: 5,
                    length: 6,
                },
            ];
            storage.store_chunks(chunks).await.expect("store");
        }

        // Reopen — cosine-similarity search should still return
        // the right chunk for a vector pointing at it.
        let storage = StorageFactory::create_file(path_str).expect("reopen");
        let hits = storage.search_similar(&[1.0, 0.0, 0.0], 1).await.expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

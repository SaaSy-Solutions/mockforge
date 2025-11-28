//! Utility functions and helpers for RAG operations
//!
//! This module provides utility functions for text processing,
//! similarity calculations, data validation, and other common RAG operations.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Text chunking utilities
pub struct TextChunker;

impl TextChunker {
    /// Split text into chunks of specified size with overlap
    pub fn split_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        if text.is_empty() || chunk_size == 0 {
            return Vec::new();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let step = chunk_size.saturating_sub(overlap);

        for start in (0..words.len()).step_by(step) {
            let end = (start + chunk_size).min(words.len());
            let chunk: Vec<&str> = words[start..end].to_vec();
            if !chunk.is_empty() {
                chunks.push(chunk.join(" "));
            }
        }

        chunks
    }

    /// Split text by sentences
    pub fn split_by_sentences(text: &str) -> Vec<String> {
        // Simple sentence splitting - in practice, you might want to use a proper NLP library
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();

        for ch in text.chars() {
            current_sentence.push(ch);
            if ch == '.' || ch == '!' || ch == '?' {
                if !current_sentence.trim().is_empty() {
                    sentences.push(current_sentence.trim().to_string());
                }
                current_sentence.clear();
            }
        }

        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }

        sentences
    }

    /// Split text by paragraphs
    pub fn split_by_paragraphs(text: &str) -> Vec<String> {
        text.split("\n\n")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Create overlapping chunks for better context preservation
    pub fn create_overlapping_chunks(
        text: &str,
        chunk_size: usize,
        overlap_ratio: f32,
    ) -> Vec<String> {
        let overlap = ((chunk_size as f32) * overlap_ratio).round() as usize;
        Self::split_text(text, chunk_size, overlap)
    }

    /// Chunk text with metadata preservation
    pub fn chunk_with_metadata(
        text: &str,
        chunk_size: usize,
        overlap: usize,
        metadata: HashMap<String, String>,
    ) -> Vec<(String, HashMap<String, String>)> {
        let chunks = Self::split_text(text, chunk_size, overlap);
        chunks.into_iter().map(|chunk| (chunk, metadata.clone())).collect()
    }
}

/// Similarity calculation utilities
pub struct SimilarityCalculator;

impl SimilarityCalculator {
    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
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

    /// Calculate Euclidean distance between two vectors
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return f32::INFINITY;
        }

        let sum_squares: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();

        sum_squares.sqrt()
    }

    /// Calculate Manhattan distance between two vectors
    pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return f32::INFINITY;
        }

        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    }

    /// Calculate dot product of two vectors
    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Normalize vector to unit length
    pub fn normalize_vector(vector: &[f32]) -> Vec<f32> {
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm == 0.0 {
            return vec![0.0; vector.len()];
        }

        vector.iter().map(|x| x / norm).collect()
    }

    /// Calculate similarity matrix for multiple vectors
    pub fn similarity_matrix(vectors: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let n = vectors.len();
        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in i..n {
                let similarity = Self::cosine_similarity(&vectors[i], &vectors[j]);
                matrix[i][j] = similarity;
                matrix[j][i] = similarity;
            }
        }

        matrix
    }

    /// Find most similar vectors to a query vector
    pub fn find_most_similar(
        query: &[f32],
        candidates: &[Vec<f32>],
        top_k: usize,
    ) -> Vec<(usize, f32)> {
        let mut similarities: Vec<(usize, f32)> = candidates
            .iter()
            .enumerate()
            .map(|(i, vec)| (i, Self::cosine_similarity(query, vec)))
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(top_k);

        similarities
    }
}

/// Text preprocessing utilities
pub struct TextPreprocessor;

impl TextPreprocessor {
    /// Clean text by removing extra whitespace and normalizing
    pub fn clean_text(text: &str) -> String {
        text.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    /// Remove HTML tags from text
    pub fn remove_html_tags(text: &str) -> String {
        // Simple HTML tag removal - in practice, you might want to use a proper HTML parser
        let mut result = String::new();
        let mut in_tag = false;

        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        result
    }

    /// Normalize whitespace
    pub fn normalize_whitespace(text: &str) -> String {
        text.chars()
            .fold((String::new(), false), |(mut acc, mut was_space), ch| {
                if ch.is_whitespace() {
                    if !was_space {
                        acc.push(' ');
                        was_space = true;
                    }
                } else {
                    acc.push(ch);
                    was_space = false;
                }
                (acc, was_space)
            })
            .0
    }

    /// Extract keywords from text
    pub fn extract_keywords(text: &str, max_keywords: usize) -> Vec<String> {
        let words: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphabetic()).to_string())
            .filter(|trimmed_word| {
                // Filter out common stop words and short words
                trimmed_word.len() > 2 && !is_stop_word(trimmed_word)
            })
            .collect();

        // Count word frequencies
        let mut word_counts: HashMap<String, usize> = HashMap::new();
        for word in words {
            *word_counts.entry(word).or_insert(0) += 1;
        }

        // Sort by frequency and take top keywords
        let mut sorted_words: Vec<(String, usize)> = word_counts.into_iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(&a.1));

        sorted_words.into_iter().take(max_keywords).map(|(word, _)| word).collect()
    }

    /// Truncate text to maximum length while preserving word boundaries
    pub fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            return text.to_string();
        }

        let truncated = &text[..max_length];
        let last_space = truncated.rfind(' ').unwrap_or(max_length);
        truncated[..last_space].trim().to_string()
    }

    /// Expand contractions in text
    pub fn expand_contractions(text: &str) -> String {
        text.replace("don't", "do not")
            .replace("can't", "cannot")
            .replace("won't", "will not")
            .replace("i'm", "i am")
            .replace("you're", "you are")
            .replace("it's", "it is")
            .replace("that's", "that is")
            .replace("there's", "there is")
            .replace("here's", "here is")
            .replace("what's", "what is")
            .replace("where's", "where is")
            .replace("when's", "when is")
            .replace("why's", "why is")
            .replace("how's", "how is")
    }
}

/// Common stop words (simplified list)
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "up"
            | "about"
            | "into"
            | "through"
            | "during"
            | "before"
            | "after"
            | "above"
            | "below"
            | "between"
            | "among"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "do"
            | "does"
            | "did"
            | "will"
            | "would"
            | "could"
            | "should"
            | "may"
            | "might"
            | "must"
            | "can"
            | "this"
            | "that"
            | "these"
            | "those"
            | "i"
            | "you"
            | "he"
            | "she"
            | "it"
            | "we"
            | "they"
            | "me"
            | "him"
            | "her"
            | "us"
            | "them"
            | "my"
            | "your"
            | "his"
            | "its"
            | "our"
            | "their"
            | "mine"
            | "yours"
            | "hers"
            | "ours"
            | "theirs"
            | "am"
            | "not"
            | "no"
            | "yes"
            | "here"
            | "there"
            | "now"
            | "then"
            | "so"
            | "very"
            | "too"
            | "also"
            | "only"
            | "just"
            | "even"
            | "still"
            | "yet"
            | "again"
            | "once"
            | "never"
            | "always"
            | "often"
            | "sometimes"
            | "usually"
    )
}

/// Rate limiting utilities
pub struct RateLimiter {
    requests_per_minute: u32,
    burst_size: u32,
    request_times: Vec<std::time::Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute,
            burst_size,
            request_times: Vec::new(),
        }
    }

    /// Check if request is allowed
    pub fn is_allowed(&mut self) -> bool {
        let now = std::time::Instant::now();
        let window_start = now - std::time::Duration::from_secs(60);

        // Remove old requests
        self.request_times.retain(|&time| time > window_start);

        // Check if within burst limit
        if self.request_times.len() < self.burst_size as usize {
            self.request_times.push(now);
            return true;
        }

        // Check if within rate limit
        let requests_in_window = self.request_times.len();
        requests_in_window < self.requests_per_minute as usize
    }

    /// Get time until next allowed request
    pub fn time_until_next(&self) -> std::time::Duration {
        if self.request_times.is_empty() {
            return std::time::Duration::from_secs(0);
        }

        let now = std::time::Instant::now();
        let window_start = now - std::time::Duration::from_secs(60);

        if let Some(&oldest_request) = self.request_times.first() {
            if oldest_request > window_start {
                oldest_request - window_start
            } else {
                std::time::Duration::from_secs(0)
            }
        } else {
            std::time::Duration::from_secs(0)
        }
    }
}

/// Caching utilities
pub struct Cache<K, V> {
    data: HashMap<K, (V, std::time::Instant)>,
    ttl: std::time::Duration,
    max_size: usize,
    hits: u64,
    misses: u64,
}

impl<K, V> Cache<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + Clone,
    V: Clone,
{
    /// Create a new cache
    pub fn new(ttl_secs: u64, max_size: usize) -> Self {
        Self {
            data: HashMap::new(),
            ttl: std::time::Duration::from_secs(ttl_secs),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    /// Get value from cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some((value, timestamp)) = self.data.get(key) {
            let now = std::time::Instant::now();
            if now.duration_since(*timestamp) < self.ttl {
                self.hits += 1;
                return Some(value.clone());
            } else {
                // Expired, remove it
                self.data.remove(key);
            }
        }
        self.misses += 1;
        None
    }

    /// Put value in cache
    pub fn put(&mut self, key: K, value: V) {
        let now = std::time::Instant::now();

        // Remove expired entries
        self.data.retain(|_, (_, timestamp)| now.duration_since(*timestamp) < self.ttl);

        // Check if we need to evict old entries
        if self.data.len() >= self.max_size {
            // Simple LRU eviction - remove oldest entry
            if let Some(oldest_key) = self.data.keys().next().cloned() {
                self.data.remove(&oldest_key);
            }
        }

        self.data.insert(key, (value, now));
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    start_time: std::time::Instant,
    metrics: HashMap<String, MetricValue>,
}

/// Performance metric value types for monitoring RAG operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Counter metric (monotonically increasing value)
    Counter(u64),
    /// Gauge metric (can increase or decrease)
    Gauge(f64),
    /// Histogram metric (distribution of values)
    Histogram(Vec<f64>),
    /// Duration metric (time measurement)
    Duration(std::time::Duration),
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            metrics: HashMap::new(),
        }
    }

    /// Start timing an operation
    pub fn start_timer(&mut self, operation: &str) -> TimerGuard<'_> {
        let start = std::time::Instant::now();
        TimerGuard {
            monitor: self,
            operation: operation.to_string(),
            start,
        }
    }

    /// Record a metric
    pub fn record_metric(&mut self, name: String, value: MetricValue) {
        self.metrics.insert(name, value);
    }

    /// Increment counter
    pub fn increment_counter(&mut self, name: &str) {
        let counter = match self.metrics.get(name) {
            Some(MetricValue::Counter(count)) => *count + 1,
            _ => 1,
        };
        self.metrics.insert(name.to_string(), MetricValue::Counter(counter));
    }

    /// Record gauge value
    pub fn record_gauge(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.to_string(), MetricValue::Gauge(value));
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Get all metrics
    pub fn metrics(&self) -> &HashMap<String, MetricValue> {
        &self.metrics
    }

    /// Get uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Timer guard for automatic timing
pub struct TimerGuard<'a> {
    monitor: &'a mut PerformanceMonitor,
    operation: String,
    start: std::time::Instant,
}

impl<'a> Drop for TimerGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let operation_duration = format!("{}_duration", self.operation);
        self.monitor.metrics.insert(operation_duration, MetricValue::Duration(duration));
    }
}

/// File utilities for RAG operations
pub struct FileUtils;

impl FileUtils {
    /// Read text file
    pub async fn read_text_file<P: AsRef<Path>>(path: P) -> Result<String> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }

    /// Write text file
    pub async fn write_text_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Check if file exists
    pub async fn file_exists<P: AsRef<Path>>(path: P) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    /// Get file size
    pub async fn file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(metadata.len())
    }

    /// List files in directory
    pub async fn list_files<P: AsRef<Path>>(dir: P) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            files.push(entry.path());
        }

        Ok(files)
    }

    /// Read JSON file
    pub async fn read_json_file<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(
        path: P,
    ) -> Result<T> {
        let content = Self::read_text_file(path).await?;
        let data: T = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Write JSON file
    pub async fn write_json_file<T: Serialize, P: AsRef<Path>>(path: P, data: &T) -> Result<()> {
        let content = serde_json::to_string_pretty(data)?;
        Self::write_text_file(path, &content).await
    }
}

/// Error handling utilities
pub struct ErrorUtils;

impl ErrorUtils {
    /// Create a generic error
    pub fn generic_error(message: &str) -> crate::Error {
        crate::Error::generic(message.to_string())
    }

    /// Create an error with context
    pub fn context_error(message: &str, context: &str) -> crate::Error {
        crate::Error::generic(format!("{}: {}", message, context))
    }

    /// Wrap an error with additional context
    pub fn wrap_error<E: std::fmt::Display>(error: E, context: &str) -> crate::Error {
        crate::Error::generic(format!("{}: {}", context, error))
    }

    /// Check if error is retryable
    pub fn is_retryable_error(error: &crate::Error) -> bool {
        // Simple heuristic - in practice, you might want to categorize errors
        error.to_string().contains("timeout")
            || error.to_string().contains("rate limit")
            || error.to_string().contains("503")
            || error.to_string().contains("502")
            || error.to_string().contains("504")
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}

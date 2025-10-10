//! Memory tracking and enforcement for WASM sandbox
//!
//! This module provides real-time memory tracking and enforcement of limits
//! using Wasmtime's resource monitoring capabilities

use wasmtime::{ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder};

/// Memory tracker for WASM store
pub struct MemoryTracker {
    /// Maximum memory allowed (bytes)
    max_memory_bytes: usize,
    /// Current memory usage (bytes)
    current_memory_bytes: usize,
    /// Peak memory usage (bytes)
    peak_memory_bytes: usize,
    /// Maximum instances allowed
    max_instances: usize,
    /// Current instances
    current_instances: usize,
    /// Maximum tables allowed
    max_tables: usize,
    /// Current tables
    current_tables: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            current_memory_bytes: 0,
            peak_memory_bytes: 0,
            max_instances: 10,
            current_instances: 0,
            max_tables: 10,
            current_tables: 0,
        }
    }

    /// Get current memory usage in bytes
    pub fn current_memory(&self) -> usize {
        self.current_memory_bytes
    }

    /// Get peak memory usage in bytes
    pub fn peak_memory(&self) -> usize {
        self.peak_memory_bytes
    }

    /// Get memory usage as percentage
    pub fn memory_usage_percent(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            0.0
        } else {
            (self.current_memory_bytes as f64 / self.max_memory_bytes as f64) * 100.0
        }
    }

    /// Check if memory limit exceeded
    pub fn is_memory_exceeded(&self) -> bool {
        self.current_memory_bytes > self.max_memory_bytes
    }

    /// Update peak memory if current exceeds it
    fn update_peak(&mut self) {
        if self.current_memory_bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = self.current_memory_bytes;
        }
    }
}

impl ResourceLimiter for MemoryTracker {
    /// Called when memory is requested
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        // Calculate the new memory size
        let new_size = desired;

        // Check if it exceeds the limit
        if new_size > self.max_memory_bytes {
            tracing::warn!(
                "Memory growth denied: {} bytes requested, {} bytes allowed",
                new_size,
                self.max_memory_bytes
            );
            return Ok(false); // Deny the allocation
        }

        // Update current memory
        self.current_memory_bytes = new_size;
        self.update_peak();

        tracing::debug!(
            "Memory growth allowed: {} -> {} bytes ({:.1}% of limit)",
            current,
            new_size,
            self.memory_usage_percent()
        );

        Ok(true) // Allow the allocation
    }

    /// Called when a table is created
    fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> anyhow::Result<bool> {
        if self.current_tables >= self.max_tables {
            tracing::warn!(
                "Table creation denied: {} tables exist, {} allowed",
                self.current_tables,
                self.max_tables
            );
            return Ok(false);
        }

        self.current_tables += 1;
        Ok(true)
    }

    /// Called when an instance is created
    fn instances(&self) -> usize {
        self.current_instances
    }

    /// Called to get maximum instances
    fn tables(&self) -> usize {
        self.current_tables
    }

    /// Called to get current memory size
    fn memories(&self) -> usize {
        1 // We allow one memory instance
    }
}

/// Configure a Wasmtime store with memory limits
pub fn configure_store_with_limits<T>(
    store: &mut Store<T>,
    max_memory_mb: usize,
) -> MemoryTracker {
    let tracker = MemoryTracker::new(max_memory_mb);

    // Set up store limits
    let limits = StoreLimitsBuilder::new()
        .memory_size(max_memory_mb * 1024 * 1024)
        .instances(10)
        .tables(10)
        .memories(1)
        .build();

    store.limiter(|_| &mut tracker);
    store.set_limits(limits);

    tracker
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage (bytes)
    pub current_bytes: usize,
    /// Peak memory usage (bytes)
    pub peak_bytes: usize,
    /// Maximum allowed (bytes)
    pub limit_bytes: usize,
    /// Usage percentage
    pub usage_percent: f64,
}

impl MemoryStats {
    /// Create from memory tracker
    pub fn from_tracker(tracker: &MemoryTracker) -> Self {
        Self {
            current_bytes: tracker.current_memory(),
            peak_bytes: tracker.peak_memory(),
            limit_bytes: tracker.max_memory_bytes,
            usage_percent: tracker.memory_usage_percent(),
        }
    }

    /// Check if usage is critical (>90%)
    pub fn is_critical(&self) -> bool {
        self.usage_percent > 90.0
    }

    /// Check if usage is high (>75%)
    pub fn is_high(&self) -> bool {
        self.usage_percent > 75.0
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "{} / {} MB ({:.1}%), peak: {} MB",
            self.current_bytes / (1024 * 1024),
            self.limit_bytes / (1024 * 1024),
            self.usage_percent,
            self.peak_bytes / (1024 * 1024)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_creation() {
        let tracker = MemoryTracker::new(10); // 10MB
        assert_eq!(tracker.current_memory(), 0);
        assert_eq!(tracker.peak_memory(), 0);
        assert_eq!(tracker.memory_usage_percent(), 0.0);
    }

    #[test]
    fn test_memory_tracker_growing() {
        let mut tracker = MemoryTracker::new(10); // 10MB max

        // Allow growth within limit
        let result = tracker.memory_growing(0, 5 * 1024 * 1024, None);
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert_eq!(tracker.current_memory(), 5 * 1024 * 1024);
        assert_eq!(tracker.peak_memory(), 5 * 1024 * 1024);

        // Deny growth exceeding limit
        let result = tracker.memory_growing(5 * 1024 * 1024, 15 * 1024 * 1024, None);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should deny
    }

    #[test]
    fn test_memory_stats() {
        let mut tracker = MemoryTracker::new(10); // 10MB
        tracker.memory_growing(0, 8 * 1024 * 1024, None).unwrap();

        let stats = MemoryStats::from_tracker(&tracker);
        assert_eq!(stats.current_bytes, 8 * 1024 * 1024);
        assert_eq!(stats.limit_bytes, 10 * 1024 * 1024);
        assert!(stats.is_high());
        assert!(!stats.is_critical());
    }

    #[test]
    fn test_memory_tracker_peak() {
        let mut tracker = MemoryTracker::new(10);

        // Grow to 5MB
        tracker.memory_growing(0, 5 * 1024 * 1024, None).unwrap();
        assert_eq!(tracker.peak_memory(), 5 * 1024 * 1024);

        // Grow to 8MB
        tracker.memory_growing(5 * 1024 * 1024, 8 * 1024 * 1024, None).unwrap();
        assert_eq!(tracker.peak_memory(), 8 * 1024 * 1024);

        // Shrink to 6MB (peak should remain 8MB)
        tracker.current_memory_bytes = 6 * 1024 * 1024;
        assert_eq!(tracker.peak_memory(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_table_limits() {
        let mut tracker = MemoryTracker::new(10);
        tracker.max_tables = 2;

        // First table should succeed
        assert!(tracker.table_growing(0, 1, None).unwrap());
        assert_eq!(tracker.current_tables, 1);

        // Second table should succeed
        assert!(tracker.table_growing(1, 2, None).unwrap());
        assert_eq!(tracker.current_tables, 2);

        // Third table should be denied
        assert!(!tracker.table_growing(2, 3, None).unwrap());
    }
}

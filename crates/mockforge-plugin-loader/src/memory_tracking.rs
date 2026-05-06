//! Memory tracking and enforcement for the WASM sandbox.
//!
//! [`MemoryTracker`] implements Wasmtime's [`ResourceLimiter`] trait so
//! the host can deny linear-memory growth that would exceed the
//! configured cap, and observe the per-store peak. Wire it into a
//! `Store<T>` by holding the tracker inside the store's data type and
//! installing the limiter:
//!
//! ```ignore
//! struct StoreData {
//!     wasi: WasiCtx,
//!     tracker: MemoryTracker,
//! }
//!
//! let mut store = Store::new(&engine, StoreData {
//!     wasi: wasi_ctx,
//!     tracker: MemoryTracker::with_byte_limit(10 * 1024 * 1024),
//! });
//! store.limiter(|d| &mut d.tracker as &mut dyn wasmtime::ResourceLimiter);
//! ```
//!
//! Modern Wasmtime (≥27) removed the standalone `Store::set_limits`
//! API in favor of the closure-based `Store::limiter`, which is why
//! the limiter must live inside the store's data type.

use wasmtime::ResourceLimiter;

/// Memory tracker for a single Wasmtime `Store`.
///
/// One tracker per store. The `Store::limiter` closure should hand
/// back `&mut self` so Wasmtime can call `memory_growing` /
/// `table_growing` for resource accounting.
pub struct MemoryTracker {
    /// Maximum memory allowed (bytes).
    max_memory_bytes: usize,
    /// Current memory usage (bytes), as reported by the most recent
    /// `memory_growing` callback.
    current_memory_bytes: usize,
    /// Peak memory ever observed across the lifetime of this store.
    /// Linear memory only grows in WASM, so this is monotonically
    /// non-decreasing.
    peak_memory_bytes: usize,
    /// Maximum tables allowed.
    max_tables: usize,
    /// Current table count.
    current_tables: usize,
}

impl MemoryTracker {
    /// Create a tracker with a megabyte-precision cap.
    pub fn new(max_memory_mb: usize) -> Self {
        Self::with_byte_limit(max_memory_mb * 1024 * 1024)
    }

    /// Create a tracker with a byte-precision cap. Use this when the
    /// caller already has a value in bytes (e.g. from
    /// `ExecutionLimits::max_memory_bytes`) so the conversion can be
    /// exact.
    pub fn with_byte_limit(max_memory_bytes: usize) -> Self {
        Self {
            max_memory_bytes,
            current_memory_bytes: 0,
            peak_memory_bytes: 0,
            max_tables: 10,
            current_tables: 0,
        }
    }

    /// Current linear-memory size in bytes.
    pub fn current_memory(&self) -> usize {
        self.current_memory_bytes
    }

    /// Highest linear-memory size observed in this store's lifetime.
    pub fn peak_memory(&self) -> usize {
        self.peak_memory_bytes
    }

    /// Configured upper bound in bytes.
    pub fn max_memory(&self) -> usize {
        self.max_memory_bytes
    }

    /// Memory usage as a percentage of the configured limit.
    pub fn memory_usage_percent(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            0.0
        } else {
            (self.current_memory_bytes as f64 / self.max_memory_bytes as f64) * 100.0
        }
    }

    /// Whether the most recent observation exceeded the cap.
    pub fn is_memory_exceeded(&self) -> bool {
        self.current_memory_bytes > self.max_memory_bytes
    }

    fn update_peak(&mut self) {
        if self.current_memory_bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = self.current_memory_bytes;
        }
    }
}

impl ResourceLimiter for MemoryTracker {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        if desired > self.max_memory_bytes {
            tracing::warn!(
                "Memory growth denied: {} bytes requested, {} bytes allowed",
                desired,
                self.max_memory_bytes
            );
            return Ok(false);
        }

        self.current_memory_bytes = desired;
        self.update_peak();

        tracing::debug!(
            "Memory growth allowed: {} -> {} bytes ({:.1}% of limit)",
            current,
            desired,
            self.memory_usage_percent()
        );

        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
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

    fn tables(&self) -> usize {
        self.current_tables
    }

    fn memories(&self) -> usize {
        // We allow one memory instance per plugin sandbox.
        1
    }
}

/// Snapshot of a tracker's state. Cheap to clone for emitting on the
/// invocation metrics bus or returning from a status endpoint.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current linear-memory size in bytes.
    pub current_bytes: usize,
    /// Peak linear-memory size observed in this store's lifetime.
    pub peak_bytes: usize,
    /// Configured upper bound in bytes.
    pub limit_bytes: usize,
    /// Current usage as a percentage of the configured limit.
    pub usage_percent: f64,
}

impl MemoryStats {
    /// Capture a snapshot of the current tracker state.
    pub fn from_tracker(tracker: &MemoryTracker) -> Self {
        Self {
            current_bytes: tracker.current_memory(),
            peak_bytes: tracker.peak_memory(),
            limit_bytes: tracker.max_memory(),
            usage_percent: tracker.memory_usage_percent(),
        }
    }

    /// Usage above 90% of the limit.
    pub fn is_critical(&self) -> bool {
        self.usage_percent > 90.0
    }

    /// Usage above 75% of the limit.
    pub fn is_high(&self) -> bool {
        self.usage_percent > 75.0
    }

    /// Human-readable summary for log lines and the admin UI.
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
        let tracker = MemoryTracker::new(10);
        assert_eq!(tracker.current_memory(), 0);
        assert_eq!(tracker.peak_memory(), 0);
        assert_eq!(tracker.max_memory(), 10 * 1024 * 1024);
        assert_eq!(tracker.memory_usage_percent(), 0.0);
    }

    #[test]
    fn test_with_byte_limit_is_exact() {
        let tracker = MemoryTracker::with_byte_limit(5_242_880); // exact 5 MiB
        assert_eq!(tracker.max_memory(), 5_242_880);
    }

    #[test]
    fn test_memory_tracker_growing() {
        let mut tracker = MemoryTracker::new(10);

        let ok = tracker.memory_growing(0, 5 * 1024 * 1024, None).unwrap();
        assert!(ok);
        assert_eq!(tracker.current_memory(), 5 * 1024 * 1024);
        assert_eq!(tracker.peak_memory(), 5 * 1024 * 1024);

        let denied = tracker.memory_growing(5 * 1024 * 1024, 15 * 1024 * 1024, None).unwrap();
        assert!(!denied);
    }

    #[test]
    fn test_memory_stats() {
        let mut tracker = MemoryTracker::new(10);
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

        tracker.memory_growing(0, 5 * 1024 * 1024, None).unwrap();
        assert_eq!(tracker.peak_memory(), 5 * 1024 * 1024);

        tracker.memory_growing(5 * 1024 * 1024, 8 * 1024 * 1024, None).unwrap();
        assert_eq!(tracker.peak_memory(), 8 * 1024 * 1024);

        // Peak persists even after a hypothetical shrink (linear memory
        // doesn't actually shrink in WASM, but the invariant still holds
        // if a future Wasmtime version added shrink callbacks).
        tracker.current_memory_bytes = 6 * 1024 * 1024;
        assert_eq!(tracker.peak_memory(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_table_limits() {
        let mut tracker = MemoryTracker::new(10);
        tracker.max_tables = 2;

        assert!(tracker.table_growing(0, 1, None).unwrap());
        assert_eq!(tracker.current_tables, 1);

        assert!(tracker.table_growing(1, 2, None).unwrap());
        assert_eq!(tracker.current_tables, 2);

        assert!(!tracker.table_growing(2, 3, None).unwrap());
    }
}

//! Pluggable clock — foundation's `now()` defaults to `Utc::now()` but can be
//! overridden (e.g., by `mockforge-core::time_travel`) via `set_clock`.
//!
//! This allows foundation types like `SessionState` to respect time travel
//! without depending on `mockforge-core`.

use chrono::{DateTime, Utc};
use std::sync::OnceLock;

/// Type alias for a clock function (returns the current time).
pub type ClockFn = fn() -> DateTime<Utc>;

static CLOCK: OnceLock<ClockFn> = OnceLock::new();

/// Returns the current time. Uses the registered clock function if any,
/// otherwise falls back to real wall-clock time (`Utc::now()`).
pub fn now() -> DateTime<Utc> {
    if let Some(clock) = CLOCK.get() {
        clock()
    } else {
        Utc::now()
    }
}

/// Register a custom clock function. Can only be called once per process;
/// subsequent calls are silently ignored (returns `Err` if already set).
///
/// This is intended for `mockforge-core::time_travel` to register its virtual
/// clock once at startup.
pub fn set_clock(clock: ClockFn) -> Result<(), ClockFn> {
    CLOCK.set(clock)
}

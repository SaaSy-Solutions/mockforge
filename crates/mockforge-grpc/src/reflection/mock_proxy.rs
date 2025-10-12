//! Mock-enabled reflection proxy implementation
//!
//! This module has been refactored into sub-modules for better organization:
//! - proxy: Core proxy functionality and state management
//! - handlers: Request/response handling logic
//! - middleware: Request processing middleware
//! - validation: Request validation and routing

// Re-export sub-modules for backward compatibility
pub mod handlers;
pub mod middleware;
pub mod proxy;
pub mod validation;

// Re-export commonly used types
pub use proxy::*;

// Legacy code removed - using proxy.rs implementation instead

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

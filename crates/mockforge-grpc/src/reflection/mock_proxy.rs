//! Mock-enabled reflection proxy implementation
//!
//! This module has been refactored into sub-modules for better organization:
//! - proxy: Core proxy functionality and state management
//! - handlers: Request/response handling logic
//! - middleware: Request processing middleware
//! - validation: Request validation and routing

// Re-export sub-modules for backward compatibility
pub mod proxy;
pub mod handlers;
pub mod middleware;
pub mod validation;

// Re-export commonly used types
pub use proxy::*;
pub use handlers::*;
pub use middleware::*;
pub use validation::*;

// Legacy code removed - using proxy.rs implementation instead

//! Request capture system for contract diff analysis
//!
//! This module provides multi-source request capture capabilities for analyzing
//! front-end requests against backend contract specifications.
//!
//! # Capture Sources
//!
//! - **Browser Extension/SDK**: Captures requests from browser-based applications
//! - **Proxy Middleware**: Captures requests passing through MockForge proxy
//! - **Manual Upload**: Allows users to upload request data via API
//! - **API Endpoint**: REST API for programmatic request submission
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::request_capture::{CaptureManager, CaptureSource};
//! use mockforge_core::ai_contract_diff::CapturedRequest;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! // Create capture manager
//! let manager = CaptureManager::new(1000); // Keep last 1000 requests
//!
//! // Capture a request from browser extension
//! let request = CapturedRequest::new("POST", "/api/users", "browser_extension")
//!     .with_body(serde_json::json!({"name": "Alice"}));
//!
//! manager.capture(request).await?;
//!
//! // Retrieve captured requests
//! let requests = manager.get_recent_captures(Some(10)).await;
//! # Ok(())
//! # }
//! ```

pub mod capture_manager;

// Re-export main types and functions
pub use capture_manager::{
    capture_request_global, get_global_capture_manager, init_global_capture_manager,
    CaptureManager, CaptureMetadata, CaptureQuery,
};

//! Contract change webhook system
//!
//! This module provides webhook functionality for notifying external systems
//! about contract mismatches, breaking changes, and drift patterns.
//!
//! # Features
//!
//! - Configurable webhook endpoints
//! - Event filtering by severity
//! - Retry logic with exponential backoff
//! - Webhook signing for security
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use mockforge_core::contract_webhooks::{WebhookDispatcher, WebhookConfig, ContractEvent};
//!
//! async fn example() -> mockforge_core::Result<()> {
//!     let config = WebhookConfig {
//!         url: "https://slack.example.com/webhooks/contracts".to_string(),
//!         events: vec!["contract.breaking_change".to_string()],
//!         severity_threshold: Some("high".to_string()),
//!         ..Default::default()
//!     };
//!
//!     let dispatcher = WebhookDispatcher::new(vec![config]);
//!
//!     let event = ContractEvent::BreakingChange {
//!         endpoint: "/api/users".to_string(),
//!         description: "Required field removed".to_string(),
//!         severity: "critical".to_string(),
//!     };
//!
//!     dispatcher.dispatch(&event).await?;
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod webhook_dispatcher;

// Re-export main types
pub use types::{ContractEvent, WebhookConfig, WebhookPayload, WebhookResult};
pub use webhook_dispatcher::WebhookDispatcher;

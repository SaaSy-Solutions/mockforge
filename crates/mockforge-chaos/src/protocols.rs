//! Protocol-specific chaos engineering modules

pub mod graphql;
pub mod grpc;
pub mod websocket;

use crate::{ChaosConfig, Result};
use async_trait::async_trait;

/// Protocol-agnostic chaos trait
#[async_trait]
pub trait ChaosProtocol: Send + Sync {
    /// Apply chaos before processing a request
    async fn apply_pre_request(&self) -> Result<()>;

    /// Apply chaos after processing a response
    async fn apply_post_response(&self, response_size: usize) -> Result<()>;

    /// Check if chaos should abort the request
    fn should_abort(&self) -> Option<String>;

    /// Get protocol name
    fn protocol_name(&self) -> &str;
}

/// Common chaos operations for all protocols
pub struct ProtocolChaos {
    config: ChaosConfig,
}

impl ProtocolChaos {
    pub fn new(config: ChaosConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ChaosConfig {
        &self.config
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

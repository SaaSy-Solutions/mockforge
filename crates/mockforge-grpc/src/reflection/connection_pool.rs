//! Connection pool for gRPC clients

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracing::{debug, trace};

/// A simple connection pool for gRPC channels
pub struct ConnectionPool {
    /// Map of endpoint URIs to channels
    channels: Arc<Mutex<HashMap<String, Channel>>>,
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a channel for the given endpoint URI
    /// If a channel already exists for this endpoint, it will be reused
    /// Otherwise, a new channel will be created and added to the pool
    pub async fn get_channel(
        &self,
        endpoint_uri: &str,
    ) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        trace!("Getting channel for endpoint: {}", endpoint_uri);

        // Lock the channels map
        let mut channels = self.channels.lock().await;

        // Check if we already have a channel for this endpoint
        if let Some(channel) = channels.get(endpoint_uri) {
            debug!("Reusing existing channel for endpoint: {}", endpoint_uri);
            return Ok(channel.clone());
        }

        // Create a new channel for this endpoint
        debug!("Creating new channel for endpoint: {}", endpoint_uri);
        let channel = Channel::from_shared(endpoint_uri.to_string())?.connect().await?;

        // Add the new channel to the pool
        channels.insert(endpoint_uri.to_string(), channel.clone());

        Ok(channel)
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

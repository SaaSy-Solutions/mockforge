//! Application state management for MockForge Desktop

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// Current configuration file path
    pub config_path: Option<PathBuf>,
    /// Server status
    pub server_running: bool,
    /// Server ports
    pub http_port: Option<u16>,
    pub admin_port: Option<u16>,
    /// Last error message
    pub last_error: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config_path: None,
            server_running: false,
            http_port: Some(3000),
            admin_port: Some(9080),
            last_error: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

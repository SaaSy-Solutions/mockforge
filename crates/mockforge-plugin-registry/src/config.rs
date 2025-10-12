//! Registry configuration management

use crate::{RegistryConfig, Result};
use std::path::PathBuf;
use tokio::fs;

/// Load registry configuration from file
pub async fn load_config() -> Result<RegistryConfig> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Ok(RegistryConfig::default());
    }

    let contents = fs::read_to_string(&config_path).await?;
    let config: RegistryConfig =
        toml::from_str(&contents).map_err(|e| crate::RegistryError::Storage(e.to_string()))?;

    Ok(config)
}

/// Save registry configuration to file
pub async fn save_config(config: &RegistryConfig) -> Result<()> {
    let config_path = get_config_path();

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let contents =
        toml::to_string_pretty(config).map_err(|e| crate::RegistryError::Storage(e.to_string()))?;

    fs::write(&config_path, contents).await?;

    Ok(())
}

/// Get configuration file path
fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("mockforge");

    config_dir.join("registry.toml")
}

/// Set registry URL
pub async fn set_registry_url(url: String) -> Result<()> {
    let mut config = load_config().await?;
    config.url = url;
    save_config(&config).await
}

/// Set API token
pub async fn set_token(token: String) -> Result<()> {
    let mut config = load_config().await?;
    config.token = Some(token);
    save_config(&config).await
}

/// Clear API token
pub async fn clear_token() -> Result<()> {
    let mut config = load_config().await?;
    config.token = None;
    save_config(&config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_config() {
        let config = RegistryConfig::default();
        assert_eq!(config.url, "https://registry.mockforge.dev");
        assert_eq!(config.timeout, 30);
    }
}

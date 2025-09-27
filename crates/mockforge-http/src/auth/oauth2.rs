//! OAuth2 utilities and client creation
//!
//! This module provides utilities for working with OAuth2 authentication,
//! including client creation and configuration.

use mockforge_core::{config::OAuth2Config, Error};

/// Create OAuth2 client from configuration
pub fn create_oauth2_client(config: &OAuth2Config) -> Result<oauth2::basic::BasicClient, Error> {
    let client_id = oauth2::ClientId::new(config.client_id.clone());
    let client_secret = oauth2::ClientSecret::new(config.client_secret.clone());

    let auth_url = oauth2::AuthUrl::new(
        config
            .auth_url
            .clone()
            .unwrap_or_else(|| "https://example.com/auth".to_string()),
    )
    .map_err(|e| Error::generic(format!("Invalid auth URL: {}", e)))?;

    let token_url = oauth2::TokenUrl::new(
        config
            .token_url
            .clone()
            .unwrap_or_else(|| "https://example.com/token".to_string()),
    )
    .map_err(|e| Error::generic(format!("Invalid token URL: {}", e)))?;

    Ok(oauth2::basic::BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url),
    ))
}

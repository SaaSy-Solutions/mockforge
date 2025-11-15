//! Authentication helper functions for handlers
//!
//! This module provides utilities for extracting user information from authenticated requests.

use axum::extract::Extension;
use uuid::Uuid;

use crate::auth::types::AuthClaims;

/// Extract user ID from AuthClaims extension
///
/// Returns the user ID from AuthClaims if available, otherwise returns None.
/// For mock server purposes, this allows handlers to work with or without authentication.
pub fn extract_user_id_from_claims(claims: Option<Extension<AuthClaims>>) -> Option<Uuid> {
    claims
        .and_then(|Extension(claims)| claims.sub)
        .and_then(|sub| Uuid::parse_str(&sub).ok())
}

/// Extract user ID from AuthClaims extension with fallback
///
/// Returns the user ID from AuthClaims if available, otherwise returns a default UUID.
/// This is useful for mock servers where authentication may be optional.
pub fn extract_user_id_with_fallback(claims: Option<Extension<AuthClaims>>) -> Uuid {
    extract_user_id_from_claims(claims).unwrap_or_else(|| {
        // For mock server, use a deterministic default user ID
        // In production, this should return an error if authentication is required
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    })
}

/// Extract username from AuthClaims extension
///
/// Returns the username from AuthClaims if available, otherwise returns None.
pub fn extract_username_from_claims(claims: Option<Extension<AuthClaims>>) -> Option<String> {
    claims.and_then(|Extension(claims)| claims.username)
}


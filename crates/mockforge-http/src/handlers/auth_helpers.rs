//! Authentication helper functions for handlers
//!
//! This module provides utilities for extracting user information from authenticated requests.

use axum::extract::Extension;
use uuid::Uuid;

use crate::auth::types::AuthClaims;

/// Optional AuthClaims extractor
///
/// Extracts AuthClaims from request extensions if available.
/// This allows handlers to work with or without authentication.
/// This is a type alias for Option<Extension<AuthClaims>> which Axum supports natively.
pub type OptionalAuthClaims = Option<Extension<AuthClaims>>;

/// Extract user ID from OptionalAuthClaims
///
/// Returns the user ID from AuthClaims if available, otherwise returns None.
/// For mock server purposes, this allows handlers to work with or without authentication.
pub fn extract_user_id_from_claims(claims: &OptionalAuthClaims) -> Option<Uuid> {
    claims
        .as_ref()
        .and_then(|Extension(claims)| claims.sub.as_ref())
        .and_then(|sub| Uuid::parse_str(sub).ok())
}

/// Extract user ID from OptionalAuthClaims with fallback
///
/// Returns the user ID from AuthClaims if available, otherwise returns a default UUID.
/// This is useful for mock servers where authentication may be optional.
pub fn extract_user_id_with_fallback(claims: &OptionalAuthClaims) -> Uuid {
    extract_user_id_from_claims(claims).unwrap_or_else(|| {
        // For mock server, use a deterministic default user ID
        // In production, this should return an error if authentication is required
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("hardcoded UUID is valid")
    })
}

/// Extract username from OptionalAuthClaims
///
/// Returns the username from AuthClaims if available, otherwise returns None.
pub fn extract_username_from_claims(claims: &OptionalAuthClaims) -> Option<String> {
    claims.as_ref().and_then(|Extension(claims)| claims.username.clone())
}

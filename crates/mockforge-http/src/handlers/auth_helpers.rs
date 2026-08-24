//! Authentication helper functions for handlers
//!
//! This module provides utilities for extracting user information from authenticated requests.

use axum::extract::Extension;
use axum::http::StatusCode;
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

/// Require a user ID from OptionalAuthClaims
///
/// Returns the user ID from AuthClaims, or an error if the request is not
/// authenticated. Privileged operations (approvals, revocations, permission
/// changes) MUST NOT fall back to a synthetic identity: doing so lets
/// unauthenticated callers perform attributed actions and corrupts audit
/// trails. Use [`extract_user_id_from_claims`] only when the operation is
/// genuinely anonymous-safe.
pub fn require_user_id_from_claims(claims: &OptionalAuthClaims) -> Result<Uuid, StatusCode> {
    extract_user_id_from_claims(claims).ok_or(StatusCode::UNAUTHORIZED)
}

/// Extract username from OptionalAuthClaims
///
/// Returns the username from AuthClaims if available, otherwise returns None.
pub fn extract_username_from_claims(claims: &OptionalAuthClaims) -> Option<String> {
    claims.as_ref().and_then(|Extension(claims)| claims.username.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims_with_sub(sub: &str) -> OptionalAuthClaims {
        let mut claims = AuthClaims::new();
        claims.sub = Some(sub.to_string());
        Some(Extension(claims))
    }

    #[test]
    fn require_rejects_unauthenticated() {
        // Regression: privileged endpoints must 401 instead of falling back
        // to the synthetic user 00000000-0000-0000-0000-000000000001.
        assert_eq!(require_user_id_from_claims(&None), Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn require_extracts_authenticated_sub() {
        let id =
            require_user_id_from_claims(&claims_with_sub("00000000-0000-0000-0000-000000000042"))
                .expect("valid sub must extract");
        assert_eq!(id, Uuid::parse_str("00000000-0000-0000-0000-000000000042").unwrap());
    }

    #[test]
    fn require_rejects_non_uuid_sub() {
        // A non-UUID sub must not silently pass as an identity.
        assert!(require_user_id_from_claims(&claims_with_sub("not-a-uuid")).is_err());
    }
}

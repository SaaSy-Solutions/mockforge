//! Authentication middleware, token lifecycle and OAuth2/OIDC endpoints.
//!
//! Extracted from HTTP while retaining shared Core configuration/error types.
//! This crate has no dependency on `mockforge-http`; HTTP preserves its old
//! authentication and OAuth2 handler paths through compatibility exports.

pub mod auth;
pub mod oauth2_server;

pub use auth::{
    auth_middleware, authenticate_jwt, authenticate_request, AuthClaims, AuthResult, AuthState,
};

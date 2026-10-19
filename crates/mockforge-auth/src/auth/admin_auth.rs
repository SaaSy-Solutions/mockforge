//! Admin UI authentication
//!
//! This module provides authentication for admin UI endpoints

use axum::body::Body;
use axum::http::header::HeaderValue;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use base64::{engine::general_purpose, Engine as _};
use subtle::ConstantTimeEq;
use tracing::{debug, warn};

/// Length-safe constant-time byte equality.
///
/// `subtle`'s `[u8]` implementation returns a zero `Choice` when the operand
/// lengths differ, so no length information leaks through an early return.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// Verify a supplied password against the configured admin secret.
///
/// The configured value may be either plaintext or a bcrypt PHC hash
/// (`$2b$…`, same format `mockforge_registry_core::auth::hash_password`
/// produces). Storing a hash means a config-file or env leak no longer
/// leaks the working credential. Verification cost is identical for both
/// branches from the caller's perspective.
fn verify_admin_password(supplied: &str, configured: &str) -> bool {
    if configured.starts_with("$2") {
        match bcrypt::verify(supplied, configured) {
            Ok(ok) => ok,
            Err(e) => {
                warn!("Malformed bcrypt hash in admin password config: {}", e);
                false
            }
        }
    } else {
        constant_time_eq(supplied.as_bytes(), configured.as_bytes())
    }
}

/// Check if admin authentication is required and valid
pub fn check_admin_auth(
    req: &Request<Body>,
    admin_auth_required: bool,
    admin_username: &Option<String>,
    admin_password: &Option<String>,
) -> Result<(), Response> {
    // If auth not required, allow through
    if !admin_auth_required {
        debug!("Admin auth not required, allowing access");
        return Ok(());
    }

    // Get authorization header
    let auth_header = req.headers().get("authorization").and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        // Check if it's Basic auth
        if let Some(basic_creds) = auth_value.strip_prefix("Basic ") {
            // Decode base64 credentials
            match general_purpose::STANDARD.decode(basic_creds) {
                Ok(decoded) => {
                    if let Ok(creds_str) = String::from_utf8(decoded) {
                        // Split on first colon
                        if let Some((username, password)) = creds_str.split_once(':') {
                            // Compare with configured credentials
                            if let (Some(expected_user), Some(expected_pass)) =
                                (admin_username, admin_password)
                            {
                                // Username compare is constant-time; password
                                // goes through the hash-aware verifier (bcrypt
                                // verify is itself constant-time in the digest).
                                let user_ok =
                                    constant_time_eq(username.as_bytes(), expected_user.as_bytes());
                                if user_ok && verify_admin_password(password, expected_pass) {
                                    debug!("Admin authentication successful");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to decode admin credentials: {}", e);
                }
            }
        }
    }

    // Authentication failed
    warn!("Admin authentication failed or missing");
    let mut res = Response::new(Body::from(
        serde_json::json!({
            "error": "Authentication required",
            "message": "Admin UI requires authentication"
        })
        .to_string(),
    ));
    *res.status_mut() = StatusCode::UNAUTHORIZED;
    res.headers_mut()
        .insert("www-authenticate", HeaderValue::from_static("Basic realm=\"MockForge Admin\""));

    Err(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn test_admin_auth_not_required() {
        let req = Request::builder().body(Body::empty()).unwrap();
        assert!(check_admin_auth(&req, false, &None, &None).is_ok());
    }

    #[test]
    fn test_admin_auth_missing() {
        let req = Request::builder().body(Body::empty()).unwrap();
        let username = Some("admin".to_string());
        let password = Some("secret".to_string());
        assert!(check_admin_auth(&req, true, &username, &password).is_err());
    }

    #[test]
    fn test_admin_auth_valid() {
        let username = Some("admin".to_string());
        let password = Some("secret".to_string());

        // Create Basic auth header: admin:secret
        let credentials = general_purpose::STANDARD.encode("admin:secret");
        let auth_value = format!("Basic {}", credentials);

        let req = Request::builder()
            .header("authorization", auth_value)
            .body(Body::empty())
            .unwrap();

        assert!(check_admin_auth(&req, true, &username, &password).is_ok());
    }

    #[test]
    fn test_admin_auth_invalid_password() {
        let username = Some("admin".to_string());
        let password = Some("secret".to_string());

        // Wrong password
        let credentials = general_purpose::STANDARD.encode("admin:wrong");
        let auth_value = format!("Basic {}", credentials);

        let req = Request::builder()
            .header("authorization", auth_value)
            .body(Body::empty())
            .unwrap();

        assert!(check_admin_auth(&req, true, &username, &password).is_err());
    }

    #[test]
    fn test_admin_auth_bcrypt_hashed_password() {
        let username = Some("admin".to_string());
        // Config stores a bcrypt hash, not the plaintext.
        let hashed = bcrypt::hash("secret", bcrypt::DEFAULT_COST).unwrap();
        let password = Some(hashed);

        let credentials = general_purpose::STANDARD.encode("admin:secret");
        let req = Request::builder()
            .header("authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();
        assert!(check_admin_auth(&req, true, &username, &password).is_ok());

        // Wrong password against a stored hash must fail.
        let wrong = general_purpose::STANDARD.encode("admin:wrong");
        let req = Request::builder()
            .header("authorization", format!("Basic {}", wrong))
            .body(Body::empty())
            .unwrap();
        assert!(check_admin_auth(&req, true, &username, &password).is_err());
    }
}

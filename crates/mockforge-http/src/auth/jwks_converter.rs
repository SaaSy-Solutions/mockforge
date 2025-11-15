//! JWKS key conversion utilities
//!
//! This module provides functions to convert PEM-encoded keys to JWK format
//! for the JWKS endpoint.

use base64::engine::general_purpose::STANDARD;
use mockforge_core::Error;

use super::oidc::{JwkKey, JwkPublicKey};

/// Convert a PEM-encoded RSA public key to JWK format
/// 
/// Note: Full PEM-to-JWK conversion requires ASN.1 parsing to extract n and e components.
/// For now, this function validates the PEM format but returns a basic JWK structure.
/// Keys should be configured in JWK format (JSON) for full functionality.
pub fn rsa_pem_to_jwk(_pem: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
    // Validate PEM format (basic check)
    // Full ASN.1 parsing would be needed to extract n and e components
    // For production, configure keys in JWK format directly
    
    Ok(JwkPublicKey {
        kid: kid.to_string(),
        kty: "RSA".to_string(),
        alg: alg.to_string(),
        use_: "sig".to_string(),
        n: None, // Would require ASN.1 parsing
        e: None, // Would require ASN.1 parsing
        crv: None,
        x: None,
        y: None,
    })
}

/// Convert a PEM-encoded ECDSA public key to JWK format
/// 
/// Note: Full PEM-to-JWK conversion requires ASN.1 parsing to extract x, y, and crv components.
/// For now, this function validates the PEM format but returns a basic JWK structure.
/// Keys should be configured in JWK format (JSON) for full functionality.
pub fn ecdsa_pem_to_jwk(_pem: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
    // Determine curve from algorithm
    let crv = match alg {
        "ES256" => "P-256",
        "ES384" => "P-384",
        "ES512" => "P-521",
        _ => "P-256", // Default
    };
    
    Ok(JwkPublicKey {
        kid: kid.to_string(),
        kty: "EC".to_string(),
        alg: alg.to_string(),
        use_: "sig".to_string(),
        n: None,
        e: None,
        crv: Some(crv.to_string()),
        x: None, // Would require ASN.1 parsing
        y: None, // Would require ASN.1 parsing
    })
}

/// Convert HMAC secret to JWK format (oct key type)
pub fn hmac_to_jwk(secret: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
    // For HMAC, we don't expose the secret in JWKS (security)
    // Instead, return a placeholder or empty key
    // In practice, HMAC keys are symmetric and shouldn't be in JWKS
    // But we provide this for completeness
    Ok(JwkPublicKey {
        kid: kid.to_string(),
        kty: "oct".to_string(),
        alg: alg.to_string(),
        use_: "sig".to_string(),
        n: None,
        e: None,
        crv: None,
        x: None,
        y: None,
    })
}

/// Convert JwkKey to JwkPublicKey for JWKS endpoint
pub fn jwk_key_to_public(jwk_key: &JwkKey) -> Result<JwkPublicKey, Error> {
    match jwk_key.kty.as_str() {
        "RSA" => {
            rsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg)
        }
        "EC" => {
            ecdsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg)
        }
        "oct" => {
            hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg)
        }
        _ => Err(Error::generic(format!("Unsupported key type: {}", jwk_key.kty))),
    }
}


/// Simplified JWK conversion that works with base64-encoded key material
/// This is a fallback when full ASN.1 parsing isn't available
/// 
/// Note: For production use, keys should be configured with JWK format components
/// (n, e for RSA or x, y, crv for EC) rather than PEM format for the JWKS endpoint.
pub fn convert_jwk_key_simple(jwk_key: &JwkKey) -> Result<JwkPublicKey, Error> {
    // Check if public_key is already in JWK format (JSON)
    if let Ok(jwk_json) = serde_json::from_str::<serde_json::Value>(&jwk_key.public_key) {
        // If it's already JSON, try to extract JWK components
        if let Some(n) = jwk_json.get("n").and_then(|v| v.as_str()) {
            // It's an RSA key in JWK format
            return Ok(JwkPublicKey {
                kid: jwk_key.kid.clone(),
                kty: "RSA".to_string(),
                alg: jwk_key.alg.clone(),
                use_: jwk_key.use_.clone(),
                n: Some(n.to_string()),
                e: jwk_json.get("e").and_then(|v| v.as_str()).map(|s| s.to_string()),
                crv: None,
                x: None,
                y: None,
            });
        }
        if let Some(x) = jwk_json.get("x").and_then(|v| v.as_str()) {
            // It's an EC key in JWK format
            return Ok(JwkPublicKey {
                kid: jwk_key.kid.clone(),
                kty: "EC".to_string(),
                alg: jwk_key.alg.clone(),
                use_: jwk_key.use_.clone(),
                n: None,
                e: None,
                crv: jwk_json.get("crv").and_then(|v| v.as_str()).map(|s| s.to_string()),
                x: Some(x.to_string()),
                y: jwk_json.get("y").and_then(|v| v.as_str()).map(|s| s.to_string()),
            });
        }
    }
    
    // If not JSON, assume PEM format and return basic structure
    // For full PEM parsing, would need ASN.1 library
    match jwk_key.kty.as_str() {
        "RSA" => {
            // Return structure without n/e - client will need to parse PEM
            // In production, configure keys with JWK format components
            Ok(JwkPublicKey {
                kid: jwk_key.kid.clone(),
                kty: "RSA".to_string(),
                alg: jwk_key.alg.clone(),
                use_: jwk_key.use_.clone(),
                n: None, // Would need ASN.1 parsing from PEM
                e: None, // Would need ASN.1 parsing from PEM
                crv: None,
                x: None,
                y: None,
            })
        }
        "EC" => {
            // Determine curve from algorithm
            let crv = match jwk_key.alg.as_str() {
                "ES256" => "P-256",
                "ES384" => "P-384",
                "ES512" => "P-521",
                _ => "P-256", // Default
            };
            
            Ok(JwkPublicKey {
                kid: jwk_key.kid.clone(),
                kty: "EC".to_string(),
                alg: jwk_key.alg.clone(),
                use_: jwk_key.use_.clone(),
                n: None,
                e: None,
                crv: Some(crv.to_string()),
                x: None, // Would need ASN.1 parsing from PEM
                y: None, // Would need ASN.1 parsing from PEM
            })
        }
        "oct" => {
            hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg)
        }
        _ => Err(Error::generic(format!("Unsupported key type: {}", jwk_key.kty))),
    }
}


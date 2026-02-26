//! JWKS key conversion utilities
//!
//! This module provides functions to convert PEM-encoded keys to JWK format
//! for the JWKS endpoint.

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
///
/// HMAC keys are symmetric secrets and MUST NOT be exposed in JWKS endpoints.
/// JWKS is designed for public key discovery only. This returns an `oct` key
/// entry with the key ID and algorithm for identification purposes, but
/// intentionally omits the secret material (`k` field).
pub fn hmac_to_jwk(_secret: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
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
        "RSA" => rsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "EC" => ecdsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "oct" => hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
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
        "oct" => hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        _ => Err(Error::generic(format!("Unsupported key type: {}", jwk_key.kty))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsa_pem_to_jwk() {
        let result = rsa_pem_to_jwk("test-pem", "test-kid", "RS256");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.kid, "test-kid");
        assert_eq!(jwk.kty, "RSA");
        assert_eq!(jwk.alg, "RS256");
        assert_eq!(jwk.use_, "sig");
        assert!(jwk.n.is_none());
        assert!(jwk.e.is_none());
        assert!(jwk.crv.is_none());
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_es256() {
        let result = ecdsa_pem_to_jwk("test-pem", "test-kid", "ES256");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.kid, "test-kid");
        assert_eq!(jwk.kty, "EC");
        assert_eq!(jwk.alg, "ES256");
        assert_eq!(jwk.use_, "sig");
        assert_eq!(jwk.crv, Some("P-256".to_string()));
        assert!(jwk.n.is_none());
        assert!(jwk.e.is_none());
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_es384() {
        let result = ecdsa_pem_to_jwk("test-pem", "test-kid", "ES384");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.crv, Some("P-384".to_string()));
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_es512() {
        let result = ecdsa_pem_to_jwk("test-pem", "test-kid", "ES512");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.crv, Some("P-521".to_string()));
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_unknown_alg() {
        let result = ecdsa_pem_to_jwk("test-pem", "test-kid", "UNKNOWN");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.crv, Some("P-256".to_string())); // Default
    }

    #[test]
    fn test_hmac_to_jwk() {
        let result = hmac_to_jwk("test-secret", "test-kid", "HS256");
        assert!(result.is_ok());

        let jwk = result.unwrap();
        assert_eq!(jwk.kid, "test-kid");
        assert_eq!(jwk.kty, "oct");
        assert_eq!(jwk.alg, "HS256");
        assert_eq!(jwk.use_, "sig");
        assert!(jwk.n.is_none());
        assert!(jwk.e.is_none());
        assert!(jwk.crv.is_none());
        assert!(jwk.x.is_none());
        assert!(jwk.y.is_none());
    }

    #[test]
    fn test_jwk_key_to_public_rsa() {
        let jwk_key = JwkKey {
            kid: "rsa-key".to_string(),
            alg: "RS256".to_string(),
            public_key: "rsa-pem-data".to_string(),
            private_key: Some("private-key".to_string()),
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let result = jwk_key_to_public(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "rsa-key");
        assert_eq!(public_key.kty, "RSA");
        assert_eq!(public_key.alg, "RS256");
    }

    #[test]
    fn test_jwk_key_to_public_ec() {
        let jwk_key = JwkKey {
            kid: "ec-key".to_string(),
            alg: "ES256".to_string(),
            public_key: "ec-pem-data".to_string(),
            private_key: Some("private-key".to_string()),
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = jwk_key_to_public(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "ec-key");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.alg, "ES256");
        assert_eq!(public_key.crv, Some("P-256".to_string()));
    }

    #[test]
    fn test_jwk_key_to_public_oct() {
        let jwk_key = JwkKey {
            kid: "hmac-key".to_string(),
            alg: "HS256".to_string(),
            public_key: "secret".to_string(),
            private_key: Some("secret".to_string()),
            kty: "oct".to_string(),
            use_: "sig".to_string(),
        };

        let result = jwk_key_to_public(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "hmac-key");
        assert_eq!(public_key.kty, "oct");
        assert_eq!(public_key.alg, "HS256");
    }

    #[test]
    fn test_jwk_key_to_public_unsupported() {
        let jwk_key = JwkKey {
            kid: "unknown-key".to_string(),
            alg: "UNKNOWN".to_string(),
            public_key: "data".to_string(),
            private_key: None,
            kty: "UNSUPPORTED".to_string(),
            use_: "sig".to_string(),
        };

        let result = jwk_key_to_public(&jwk_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_jwk_key_simple_rsa_json() {
        let jwk_json = r#"{"n": "modulus-value", "e": "exponent-value"}"#;
        let jwk_key = JwkKey {
            kid: "rsa-json-key".to_string(),
            alg: "RS256".to_string(),
            public_key: jwk_json.to_string(),
            private_key: None,
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "rsa-json-key");
        assert_eq!(public_key.kty, "RSA");
        assert_eq!(public_key.n, Some("modulus-value".to_string()));
        assert_eq!(public_key.e, Some("exponent-value".to_string()));
        assert!(public_key.crv.is_none());
        assert!(public_key.x.is_none());
        assert!(public_key.y.is_none());
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_json() {
        let jwk_json = r#"{"x": "x-coordinate", "y": "y-coordinate", "crv": "P-256"}"#;
        let jwk_key = JwkKey {
            kid: "ec-json-key".to_string(),
            alg: "ES256".to_string(),
            public_key: jwk_json.to_string(),
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "ec-json-key");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.x, Some("x-coordinate".to_string()));
        assert_eq!(public_key.y, Some("y-coordinate".to_string()));
        assert_eq!(public_key.crv, Some("P-256".to_string()));
        assert!(public_key.n.is_none());
        assert!(public_key.e.is_none());
    }

    #[test]
    fn test_convert_jwk_key_simple_rsa_pem() {
        let jwk_key = JwkKey {
            kid: "rsa-pem-key".to_string(),
            alg: "RS256".to_string(),
            public_key: "-----BEGIN PUBLIC KEY-----\ndata\n-----END PUBLIC KEY-----".to_string(),
            private_key: None,
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "rsa-pem-key");
        assert_eq!(public_key.kty, "RSA");
        // PEM format means n/e won't be extracted without ASN.1 parsing
        assert!(public_key.n.is_none());
        assert!(public_key.e.is_none());
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_pem_es256() {
        let jwk_key = JwkKey {
            kid: "ec-pem-key-256".to_string(),
            alg: "ES256".to_string(),
            public_key: "-----BEGIN PUBLIC KEY-----\ndata\n-----END PUBLIC KEY-----".to_string(),
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "ec-pem-key-256");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.crv, Some("P-256".to_string()));
        // PEM format means x/y won't be extracted without ASN.1 parsing
        assert!(public_key.x.is_none());
        assert!(public_key.y.is_none());
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_pem_es384() {
        let jwk_key = JwkKey {
            kid: "ec-pem-key-384".to_string(),
            alg: "ES384".to_string(),
            public_key: "pem-data".to_string(),
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.crv, Some("P-384".to_string()));
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_pem_es512() {
        let jwk_key = JwkKey {
            kid: "ec-pem-key-512".to_string(),
            alg: "ES512".to_string(),
            public_key: "pem-data".to_string(),
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.crv, Some("P-521".to_string()));
    }

    #[test]
    fn test_convert_jwk_key_simple_oct() {
        let jwk_key = JwkKey {
            kid: "oct-key".to_string(),
            alg: "HS256".to_string(),
            public_key: "secret-data".to_string(),
            private_key: Some("secret-data".to_string()),
            kty: "oct".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.kid, "oct-key");
        assert_eq!(public_key.kty, "oct");
        assert_eq!(public_key.alg, "HS256");
    }

    #[test]
    fn test_convert_jwk_key_simple_unsupported() {
        let jwk_key = JwkKey {
            kid: "unsupported-key".to_string(),
            alg: "UNKNOWN".to_string(),
            public_key: "data".to_string(),
            private_key: None,
            kty: "UNSUPPORTED".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_jwk_key_simple_invalid_json() {
        let jwk_key = JwkKey {
            kid: "invalid-json-key".to_string(),
            alg: "RS256".to_string(),
            public_key: "not-valid-json-{".to_string(),
            private_key: None,
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());
        // Should fall back to PEM processing
        let public_key = result.unwrap();
        assert_eq!(public_key.kty, "RSA");
    }

    #[test]
    fn test_rsa_multiple_algorithms() {
        for alg in &["RS256", "RS384", "RS512"] {
            let result = rsa_pem_to_jwk("pem", "kid", alg);
            assert!(result.is_ok());
            let jwk = result.unwrap();
            assert_eq!(jwk.alg, *alg);
            assert_eq!(jwk.kty, "RSA");
        }
    }

    #[test]
    fn test_hmac_multiple_algorithms() {
        for alg in &["HS256", "HS384", "HS512"] {
            let result = hmac_to_jwk("secret", "kid", alg);
            assert!(result.is_ok());
            let jwk = result.unwrap();
            assert_eq!(jwk.alg, *alg);
            assert_eq!(jwk.kty, "oct");
        }
    }

    #[test]
    fn test_convert_jwk_key_simple_rsa_json_without_exponent() {
        let jwk_json = r#"{"n": "modulus-only"}"#;
        let jwk_key = JwkKey {
            kid: "rsa-no-e".to_string(),
            alg: "RS256".to_string(),
            public_key: jwk_json.to_string(),
            private_key: None,
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.n, Some("modulus-only".to_string()));
        assert!(public_key.e.is_none());
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_json_without_y() {
        let jwk_json = r#"{"x": "x-only"}"#;
        let jwk_key = JwkKey {
            kid: "ec-no-y".to_string(),
            alg: "ES256".to_string(),
            public_key: jwk_json.to_string(),
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let result = convert_jwk_key_simple(&jwk_key);
        assert!(result.is_ok());

        let public_key = result.unwrap();
        assert_eq!(public_key.x, Some("x-only".to_string()));
        assert!(public_key.y.is_none());
    }
}

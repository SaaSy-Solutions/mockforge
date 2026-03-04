//! JWKS key conversion utilities
//!
//! This module provides functions to convert PEM-encoded keys to JWK format
//! for the JWKS endpoint. It includes a lightweight DER/ASN.1 parser to
//! extract RSA modulus/exponent and EC point coordinates from SPKI-encoded
//! public keys.

use base64::{engine::general_purpose, Engine as _};
use mockforge_core::Error;

use super::oidc::{JwkKey, JwkPublicKey};

// ---------------------------------------------------------------------------
// Lightweight DER / ASN.1 helpers
// ---------------------------------------------------------------------------

/// ASN.1 tag constants
const TAG_INTEGER: u8 = 0x02;
const TAG_BIT_STRING: u8 = 0x03;
const TAG_SEQUENCE: u8 = 0x30;

/// Read a DER tag + length, returning (tag, length, rest).
fn read_der_tl(data: &[u8]) -> Result<(u8, usize, &[u8]), Error> {
    if data.is_empty() {
        return Err(Error::generic("DER: unexpected end of data"));
    }
    let tag = data[0];
    if data.len() < 2 {
        return Err(Error::generic("DER: truncated length"));
    }
    let (len, header_len) = if data[1] & 0x80 == 0 {
        (data[1] as usize, 2)
    } else {
        let num_bytes = (data[1] & 0x7f) as usize;
        if num_bytes == 0 || num_bytes > 4 || data.len() < 2 + num_bytes {
            return Err(Error::generic("DER: invalid length encoding"));
        }
        let mut len: usize = 0;
        for &b in &data[2..2 + num_bytes] {
            len = len.checked_shl(8).ok_or_else(|| Error::generic("DER: length overflow"))?
                | b as usize;
        }
        (len, 2 + num_bytes)
    };
    if data.len() < header_len + len {
        return Err(Error::generic("DER: content exceeds buffer"));
    }
    Ok((tag, len, &data[header_len..]))
}

/// Expect a specific tag and return (content, rest_after).
fn expect_tag<'a>(data: &'a [u8], expected: u8) -> Result<(&'a [u8], &'a [u8]), Error> {
    let (tag, len, rest) = read_der_tl(data)?;
    if tag != expected {
        return Err(Error::generic(format!("DER: expected tag 0x{expected:02x}, got 0x{tag:02x}")));
    }
    Ok((&rest[..len], &rest[len..]))
}

/// Read an ASN.1 INTEGER, stripping any leading zero-padding byte, and return
/// (integer_bytes, rest_after).
fn read_integer(data: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    let (content, rest) = expect_tag(data, TAG_INTEGER)?;
    // ASN.1 integers are signed; a leading 0x00 byte is padding for unsigned values.
    let trimmed = if content.len() > 1 && content[0] == 0x00 {
        &content[1..]
    } else {
        content
    };
    Ok((trimmed, rest))
}

/// Base64url-encode bytes (no padding) per RFC 7515.
fn base64url_encode(bytes: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ---------------------------------------------------------------------------
// PEM helpers
// ---------------------------------------------------------------------------

/// Decode a PEM block into raw DER bytes.
fn decode_pem(pem: &str) -> Result<Vec<u8>, Error> {
    let pem = pem.trim();

    // Validate that it has PEM markers
    if !pem.starts_with("-----BEGIN") || !pem.contains("-----END") {
        return Err(Error::generic("Invalid PEM format: missing BEGIN/END markers"));
    }

    // Strip header and footer lines, join base64 body
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect::<Vec<_>>().join("");

    general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| Error::generic(format!("PEM base64 decode error: {e}")))
}

// ---------------------------------------------------------------------------
// RSA PEM → JWK
// ---------------------------------------------------------------------------

/// Parse an SPKI (or PKCS#1) DER RSA public key and return (n, e) as
/// base64url-encoded strings.
fn parse_rsa_public_key_der(der: &[u8]) -> Result<(String, String), Error> {
    // Try SPKI first: SEQUENCE { AlgorithmIdentifier, BIT STRING { PKCS1Key } }
    let (outer_content, _) = expect_tag(der, TAG_SEQUENCE)?;

    // Peek at the first element — if it's a SEQUENCE it's SPKI, if INTEGER it's PKCS#1.
    let (first_tag, _, _) = read_der_tl(outer_content)?;

    if first_tag == TAG_SEQUENCE {
        // SPKI format
        let (_alg_id, after_alg) = expect_tag(outer_content, TAG_SEQUENCE)?;
        let (bit_string, _) = expect_tag(after_alg, TAG_BIT_STRING)?;

        // BIT STRING has a leading "unused bits" byte (should be 0)
        if bit_string.is_empty() {
            return Err(Error::generic("DER: empty BIT STRING"));
        }
        let pkcs1_der = &bit_string[1..]; // skip unused-bits byte
        return parse_rsa_pkcs1(pkcs1_der);
    }

    // PKCS#1 format (SEQUENCE already consumed above — re-parse from `der`)
    parse_rsa_pkcs1(der)
}

/// Parse PKCS#1 RSAPublicKey: SEQUENCE { INTEGER(n), INTEGER(e) }
fn parse_rsa_pkcs1(der: &[u8]) -> Result<(String, String), Error> {
    let (content, _) = expect_tag(der, TAG_SEQUENCE)?;
    let (n_bytes, rest) = read_integer(content)?;
    let (e_bytes, _) = read_integer(rest)?;
    Ok((base64url_encode(n_bytes), base64url_encode(e_bytes)))
}

/// Convert a PEM-encoded RSA public key to JWK format.
///
/// Supports both SPKI (`BEGIN PUBLIC KEY`) and PKCS#1 (`BEGIN RSA PUBLIC KEY`)
/// PEM formats. Parses the ASN.1 DER structure to extract the modulus (n) and
/// exponent (e).
pub fn rsa_pem_to_jwk(pem: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
    let der = decode_pem(pem)?;
    let (n, e) = parse_rsa_public_key_der(&der)?;

    Ok(JwkPublicKey {
        kid: kid.to_string(),
        kty: "RSA".to_string(),
        alg: alg.to_string(),
        use_: "sig".to_string(),
        n: Some(n),
        e: Some(e),
        crv: None,
        x: None,
        y: None,
    })
}

// ---------------------------------------------------------------------------
// EC PEM → JWK
// ---------------------------------------------------------------------------

/// Well-known OID byte representations for named curves.
const OID_P256: &[u8] = &[0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const OID_P384: &[u8] = &[0x06, 0x05, 0x2B, 0x81, 0x04, 0x00, 0x22];
const OID_P521: &[u8] = &[0x06, 0x05, 0x2B, 0x81, 0x04, 0x00, 0x23];

/// Expected uncompressed point coordinate size for each curve (bytes per coordinate).
fn ec_coord_size(crv: &str) -> usize {
    match crv {
        "P-256" => 32,
        "P-384" => 48,
        "P-521" => 66,
        _ => 32,
    }
}

/// Identify the curve name from the AlgorithmIdentifier SEQUENCE bytes.
fn identify_ec_curve(alg_id_bytes: &[u8]) -> Result<&'static str, Error> {
    // The AlgorithmIdentifier SEQUENCE contains: OID(ecPublicKey), OID(namedCurve)
    // We scan for the curve OID.
    if contains_oid(alg_id_bytes, OID_P256) {
        Ok("P-256")
    } else if contains_oid(alg_id_bytes, OID_P384) {
        Ok("P-384")
    } else if contains_oid(alg_id_bytes, OID_P521) {
        Ok("P-521")
    } else {
        Err(Error::generic("Unsupported EC curve OID in SPKI AlgorithmIdentifier"))
    }
}

/// Check whether `haystack` contains the byte pattern `needle`.
fn contains_oid(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// Parse an SPKI-encoded EC public key. Returns (curve, x, y) where x and y
/// are base64url-encoded.
fn parse_ec_public_key_der(der: &[u8]) -> Result<(String, String, String), Error> {
    let (outer_content, _) = expect_tag(der, TAG_SEQUENCE)?;

    // AlgorithmIdentifier SEQUENCE
    let (alg_id_bytes, after_alg) = expect_tag(outer_content, TAG_SEQUENCE)?;
    let crv = identify_ec_curve(alg_id_bytes)?;

    // BIT STRING containing the uncompressed EC point
    let (bit_string, _) = expect_tag(after_alg, TAG_BIT_STRING)?;
    if bit_string.is_empty() {
        return Err(Error::generic("DER: empty BIT STRING in EC key"));
    }

    let point = &bit_string[1..]; // skip unused-bits byte
    if point.is_empty() || point[0] != 0x04 {
        return Err(Error::generic(
            "EC public key is not in uncompressed point format (expected 0x04 prefix)",
        ));
    }

    let coord_bytes = &point[1..]; // skip 0x04
    let coord_size = ec_coord_size(crv);
    if coord_bytes.len() < coord_size * 2 {
        return Err(Error::generic(format!(
            "EC point too short for {crv}: expected {expected} bytes, got {got}",
            expected = coord_size * 2,
            got = coord_bytes.len()
        )));
    }

    let x = base64url_encode(&coord_bytes[..coord_size]);
    let y = base64url_encode(&coord_bytes[coord_size..coord_size * 2]);

    Ok((crv.to_string(), x, y))
}

/// Convert a PEM-encoded ECDSA public key to JWK format.
///
/// Parses the SPKI DER structure to identify the curve and extract the
/// uncompressed point coordinates (x, y).
pub fn ecdsa_pem_to_jwk(pem: &str, kid: &str, alg: &str) -> Result<JwkPublicKey, Error> {
    let der = decode_pem(pem)?;
    let (crv, x, y) = parse_ec_public_key_der(&der)?;

    Ok(JwkPublicKey {
        kid: kid.to_string(),
        kty: "EC".to_string(),
        alg: alg.to_string(),
        use_: "sig".to_string(),
        n: None,
        e: None,
        crv: Some(crv),
        x: Some(x),
        y: Some(y),
    })
}

// ---------------------------------------------------------------------------
// HMAC → JWK (unchanged — HMAC secrets must not be exposed in JWKS)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// High-level converters
// ---------------------------------------------------------------------------

/// Convert JwkKey to JwkPublicKey for JWKS endpoint
pub fn jwk_key_to_public(jwk_key: &JwkKey) -> Result<JwkPublicKey, Error> {
    match jwk_key.kty.as_str() {
        "RSA" => rsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "EC" => ecdsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "oct" => hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        _ => Err(Error::generic(format!("Unsupported key type: {}", jwk_key.kty))),
    }
}

/// Simplified JWK conversion that first tries to parse `public_key` as JSON
/// (JWK-format), then falls back to PEM parsing with full ASN.1 extraction.
pub fn convert_jwk_key_simple(jwk_key: &JwkKey) -> Result<JwkPublicKey, Error> {
    // Check if public_key is already in JWK format (JSON)
    if let Ok(jwk_json) = serde_json::from_str::<serde_json::Value>(&jwk_key.public_key) {
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

    // Not JSON — treat as PEM and parse with full ASN.1 extraction.
    match jwk_key.kty.as_str() {
        "RSA" => rsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "EC" => ecdsa_pem_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        "oct" => hmac_to_jwk(&jwk_key.public_key, &jwk_key.kid, &jwk_key.alg),
        _ => Err(Error::generic(format!("Unsupported key type: {}", jwk_key.kty))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper: build a minimal SPKI PEM for RSA from known n/e
    // -----------------------------------------------------------------------

    /// Build a DER-encoded ASN.1 INTEGER (with leading 0x00 padding if high bit set).
    fn der_integer(bytes: &[u8]) -> Vec<u8> {
        let needs_pad = !bytes.is_empty() && bytes[0] & 0x80 != 0;
        let mut out = vec![TAG_INTEGER];
        let len = bytes.len() + if needs_pad { 1 } else { 0 };
        encode_der_length(&mut out, len);
        if needs_pad {
            out.push(0x00);
        }
        out.extend_from_slice(bytes);
        out
    }

    fn encode_der_length(out: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            out.push(len as u8);
        } else if len < 0x100 {
            out.push(0x81);
            out.push(len as u8);
        } else {
            out.push(0x82);
            out.push((len >> 8) as u8);
            out.push(len as u8);
        }
    }

    fn der_sequence(contents: &[u8]) -> Vec<u8> {
        let mut out = vec![TAG_SEQUENCE];
        encode_der_length(&mut out, contents.len());
        out.extend_from_slice(contents);
        out
    }

    fn der_bit_string(contents: &[u8]) -> Vec<u8> {
        let mut out = vec![TAG_BIT_STRING];
        encode_der_length(&mut out, contents.len() + 1);
        out.push(0x00); // unused bits
        out.extend_from_slice(contents);
        out
    }

    /// Wrap DER bytes in PEM markers.
    fn to_pem(der: &[u8], label: &str) -> String {
        let b64 = general_purpose::STANDARD.encode(der);
        let mut pem = format!("-----BEGIN {label}-----\n");
        for chunk in b64.as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(chunk).unwrap());
            pem.push('\n');
        }
        pem.push_str(&format!("-----END {label}-----"));
        pem
    }

    /// Build a test RSA SPKI PEM with the given modulus and exponent bytes.
    fn build_rsa_spki_pem(n_bytes: &[u8], e_bytes: &[u8]) -> String {
        // RSA OID: 1.2.840.113549.1.1.1
        let rsa_oid: Vec<u8> = vec![
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01,
        ];
        let null = vec![0x05, 0x00];
        let mut alg_id_inner = Vec::new();
        alg_id_inner.extend_from_slice(&rsa_oid);
        alg_id_inner.extend_from_slice(&null);
        let alg_id = der_sequence(&alg_id_inner);

        let n_int = der_integer(n_bytes);
        let e_int = der_integer(e_bytes);
        let mut pkcs1_inner = Vec::new();
        pkcs1_inner.extend_from_slice(&n_int);
        pkcs1_inner.extend_from_slice(&e_int);
        let pkcs1 = der_sequence(&pkcs1_inner);
        let bit_string = der_bit_string(&pkcs1);

        let mut spki_inner = Vec::new();
        spki_inner.extend_from_slice(&alg_id);
        spki_inner.extend_from_slice(&bit_string);
        let spki = der_sequence(&spki_inner);

        to_pem(&spki, "PUBLIC KEY")
    }

    /// Build a test EC SPKI PEM with the given curve OID and point coordinates.
    fn build_ec_spki_pem(curve_oid: &[u8], x_bytes: &[u8], y_bytes: &[u8]) -> String {
        // ecPublicKey OID: 1.2.840.10045.2.1
        let ec_oid: Vec<u8> = vec![0x06, 0x07, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
        let mut alg_id_inner = Vec::new();
        alg_id_inner.extend_from_slice(&ec_oid);
        alg_id_inner.extend_from_slice(curve_oid);
        let alg_id = der_sequence(&alg_id_inner);

        // Uncompressed point: 0x04 || x || y
        let mut point = vec![0x04];
        point.extend_from_slice(x_bytes);
        point.extend_from_slice(y_bytes);
        let bit_string = der_bit_string(&point);

        let mut spki_inner = Vec::new();
        spki_inner.extend_from_slice(&alg_id);
        spki_inner.extend_from_slice(&bit_string);
        let spki = der_sequence(&spki_inner);

        to_pem(&spki, "PUBLIC KEY")
    }

    // -----------------------------------------------------------------------
    // RSA tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rsa_pem_to_jwk_extracts_n_and_e() {
        let n_bytes = vec![0x01; 32]; // 256-bit modulus
        let e_bytes = vec![0x01, 0x00, 0x01]; // 65537
        let pem = build_rsa_spki_pem(&n_bytes, &e_bytes);

        let jwk = rsa_pem_to_jwk(&pem, "test-kid", "RS256").unwrap();
        assert_eq!(jwk.kid, "test-kid");
        assert_eq!(jwk.kty, "RSA");
        assert_eq!(jwk.alg, "RS256");
        assert_eq!(jwk.use_, "sig");
        assert!(jwk.n.is_some(), "n should be extracted from PEM");
        assert!(jwk.e.is_some(), "e should be extracted from PEM");

        // Decode and verify round-trip
        let decoded_n = general_purpose::URL_SAFE_NO_PAD.decode(jwk.n.unwrap()).unwrap();
        assert_eq!(decoded_n, n_bytes);

        let decoded_e = general_purpose::URL_SAFE_NO_PAD.decode(jwk.e.unwrap()).unwrap();
        assert_eq!(decoded_e, e_bytes);
    }

    #[test]
    fn test_rsa_pem_to_jwk_with_high_bit_modulus() {
        // Modulus with high bit set — ASN.1 will add a leading 0x00, which we must strip
        let n_bytes = vec![0xFF; 32];
        let e_bytes = vec![0x01, 0x00, 0x01];
        let pem = build_rsa_spki_pem(&n_bytes, &e_bytes);

        let jwk = rsa_pem_to_jwk(&pem, "kid-highbit", "RS384").unwrap();
        let decoded_n = general_purpose::URL_SAFE_NO_PAD.decode(jwk.n.unwrap()).unwrap();
        assert_eq!(decoded_n, n_bytes);
    }

    #[test]
    fn test_rsa_pem_invalid_format() {
        let result = rsa_pem_to_jwk("not-a-pem", "kid", "RS256");
        assert!(result.is_err());
    }

    #[test]
    fn test_rsa_pem_multiple_algorithms() {
        let n = vec![0x42; 16];
        let e = vec![0x01, 0x00, 0x01];
        let pem = build_rsa_spki_pem(&n, &e);

        for alg in &["RS256", "RS384", "RS512"] {
            let jwk = rsa_pem_to_jwk(&pem, "kid", alg).unwrap();
            assert_eq!(jwk.alg, *alg);
            assert_eq!(jwk.kty, "RSA");
            assert!(jwk.n.is_some());
            assert!(jwk.e.is_some());
        }
    }

    // -----------------------------------------------------------------------
    // EC tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ecdsa_pem_to_jwk_p256() {
        let x = vec![0xAA; 32];
        let y = vec![0xBB; 32];
        let pem = build_ec_spki_pem(OID_P256, &x, &y);

        let jwk = ecdsa_pem_to_jwk(&pem, "ec-kid", "ES256").unwrap();
        assert_eq!(jwk.kid, "ec-kid");
        assert_eq!(jwk.kty, "EC");
        assert_eq!(jwk.alg, "ES256");
        assert_eq!(jwk.crv, Some("P-256".to_string()));
        assert!(jwk.x.is_some());
        assert!(jwk.y.is_some());

        let decoded_x = general_purpose::URL_SAFE_NO_PAD.decode(jwk.x.unwrap()).unwrap();
        assert_eq!(decoded_x, x);
        let decoded_y = general_purpose::URL_SAFE_NO_PAD.decode(jwk.y.unwrap()).unwrap();
        assert_eq!(decoded_y, y);
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_p384() {
        let x = vec![0xCC; 48];
        let y = vec![0xDD; 48];
        let pem = build_ec_spki_pem(OID_P384, &x, &y);

        let jwk = ecdsa_pem_to_jwk(&pem, "ec384-kid", "ES384").unwrap();
        assert_eq!(jwk.crv, Some("P-384".to_string()));
        assert!(jwk.x.is_some());
        assert!(jwk.y.is_some());
    }

    #[test]
    fn test_ecdsa_pem_to_jwk_p521() {
        let x = vec![0xEE; 66];
        let y = vec![0xFF; 66];
        let pem = build_ec_spki_pem(OID_P521, &x, &y);

        let jwk = ecdsa_pem_to_jwk(&pem, "ec521-kid", "ES512").unwrap();
        assert_eq!(jwk.crv, Some("P-521".to_string()));
        assert!(jwk.x.is_some());
        assert!(jwk.y.is_some());
    }

    #[test]
    fn test_ecdsa_pem_invalid_format() {
        let result = ecdsa_pem_to_jwk("not-a-pem", "kid", "ES256");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // HMAC tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_hmac_to_jwk() {
        let jwk = hmac_to_jwk("test-secret", "test-kid", "HS256").unwrap();
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
    fn test_hmac_multiple_algorithms() {
        for alg in &["HS256", "HS384", "HS512"] {
            let jwk = hmac_to_jwk("secret", "kid", alg).unwrap();
            assert_eq!(jwk.alg, *alg);
            assert_eq!(jwk.kty, "oct");
        }
    }

    // -----------------------------------------------------------------------
    // jwk_key_to_public tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_jwk_key_to_public_rsa() {
        let n = vec![0x42; 16];
        let e = vec![0x01, 0x00, 0x01];
        let pem = build_rsa_spki_pem(&n, &e);

        let jwk_key = JwkKey {
            kid: "rsa-key".to_string(),
            alg: "RS256".to_string(),
            public_key: pem,
            private_key: Some("private-key".to_string()),
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let public_key = jwk_key_to_public(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "rsa-key");
        assert_eq!(public_key.kty, "RSA");
        assert_eq!(public_key.alg, "RS256");
        assert!(public_key.n.is_some());
        assert!(public_key.e.is_some());
    }

    #[test]
    fn test_jwk_key_to_public_ec() {
        let x = vec![0xAA; 32];
        let y = vec![0xBB; 32];
        let pem = build_ec_spki_pem(OID_P256, &x, &y);

        let jwk_key = JwkKey {
            kid: "ec-key".to_string(),
            alg: "ES256".to_string(),
            public_key: pem,
            private_key: Some("private-key".to_string()),
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let public_key = jwk_key_to_public(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "ec-key");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.alg, "ES256");
        assert_eq!(public_key.crv, Some("P-256".to_string()));
        assert!(public_key.x.is_some());
        assert!(public_key.y.is_some());
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

        let public_key = jwk_key_to_public(&jwk_key).unwrap();
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

        assert!(jwk_key_to_public(&jwk_key).is_err());
    }

    // -----------------------------------------------------------------------
    // convert_jwk_key_simple tests
    // -----------------------------------------------------------------------

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

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "rsa-json-key");
        assert_eq!(public_key.kty, "RSA");
        assert_eq!(public_key.n, Some("modulus-value".to_string()));
        assert_eq!(public_key.e, Some("exponent-value".to_string()));
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

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "ec-json-key");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.x, Some("x-coordinate".to_string()));
        assert_eq!(public_key.y, Some("y-coordinate".to_string()));
        assert_eq!(public_key.crv, Some("P-256".to_string()));
    }

    #[test]
    fn test_convert_jwk_key_simple_rsa_pem_extracts_components() {
        let n = vec![0x42; 16];
        let e = vec![0x01, 0x00, 0x01];
        let pem = build_rsa_spki_pem(&n, &e);

        let jwk_key = JwkKey {
            kid: "rsa-pem-key".to_string(),
            alg: "RS256".to_string(),
            public_key: pem,
            private_key: None,
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
        };

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "rsa-pem-key");
        assert_eq!(public_key.kty, "RSA");
        assert!(public_key.n.is_some(), "n should now be extracted from PEM");
        assert!(public_key.e.is_some(), "e should now be extracted from PEM");
    }

    #[test]
    fn test_convert_jwk_key_simple_ec_pem_extracts_coordinates() {
        let x = vec![0xAA; 32];
        let y = vec![0xBB; 32];
        let pem = build_ec_spki_pem(OID_P256, &x, &y);

        let jwk_key = JwkKey {
            kid: "ec-pem-key".to_string(),
            alg: "ES256".to_string(),
            public_key: pem,
            private_key: None,
            kty: "EC".to_string(),
            use_: "sig".to_string(),
        };

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
        assert_eq!(public_key.kid, "ec-pem-key");
        assert_eq!(public_key.kty, "EC");
        assert_eq!(public_key.crv, Some("P-256".to_string()));
        assert!(public_key.x.is_some(), "x should now be extracted from PEM");
        assert!(public_key.y.is_some(), "y should now be extracted from PEM");
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

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
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

        assert!(convert_jwk_key_simple(&jwk_key).is_err());
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

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
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

        let public_key = convert_jwk_key_simple(&jwk_key).unwrap();
        assert_eq!(public_key.x, Some("x-only".to_string()));
        assert!(public_key.y.is_none());
    }
}

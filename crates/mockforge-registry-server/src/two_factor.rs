//! Two-Factor Authentication (2FA) utilities
//!
//! Provides TOTP (Time-based One-Time Password) generation and verification
//! for two-factor authentication

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use data_encoding::BASE32;
use qrcode::QrCode;
use sha1::Sha1;
use totp_lite::totp_custom;

/// Generate a new TOTP secret
/// Returns a base32-encoded secret suitable for TOTP
pub fn generate_secret() -> String {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let mut secret_bytes = [0u8; 20]; // 160 bits for TOTP secret
    rng.fill(&mut secret_bytes).expect("Failed to generate random secret");
    BASE32.encode(&secret_bytes)
}

/// Generate a TOTP code from a secret
///
/// # Arguments
/// * `secret` - Base32-encoded TOTP secret
/// * `timestamp` - Unix timestamp (defaults to current time if None)
///
/// # Returns
/// 6-digit TOTP code as a string
pub fn generate_totp_code(secret: &str, timestamp: Option<u64>) -> Result<String> {
    let secret_bytes = BASE32
        .decode(secret.as_bytes())
        .map_err(|e| anyhow!("Invalid base32 secret: {}", e))?;

    let time = timestamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    // totp-lite 2.0 API: totp_custom<H>(step, digits, secret, time)
    // Using SHA1 algorithm (default for TOTP)
    let code = totp_custom::<Sha1>(
        30, // Time step (30 seconds)
        6,  // Code length (6 digits)
        &secret_bytes,
        time,
    );

    Ok(code)
}

/// Verify a TOTP code
///
/// # Arguments
/// * `secret` - Base32-encoded TOTP secret
/// * `code` - The 6-digit code to verify
/// * `window` - Time window tolerance (default: 1, meaning current and previous/next 30s window)
///
/// # Returns
/// true if the code is valid, false otherwise
pub fn verify_totp_code(secret: &str, code: &str, window: Option<u64>) -> Result<bool> {
    let window = window.unwrap_or(1);
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check current time window and adjacent windows (for clock skew tolerance)
    for i in 0..=(window * 2) {
        let time_offset = if i < window {
            current_time.saturating_sub((window - i) * 30)
        } else {
            current_time + ((i - window) * 30)
        };

        let expected_code = generate_totp_code(secret, Some(time_offset))?;
        if expected_code == code {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generate a QR code data URL for TOTP setup
///
/// # Arguments
/// * `secret` - Base32-encoded TOTP secret
/// * `account_name` - User's email or username
/// * `issuer` - Service name (e.g., "MockForge")
///
/// # Returns
/// Data URL for the QR code image (SVG format)
pub fn generate_qr_code_data_url(secret: &str, account_name: &str, issuer: &str) -> Result<String> {
    // TOTP URI format: otpauth://totp/{issuer}:{account_name}?secret={secret}&issuer={issuer}
    let uri = format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        issuer, account_name, secret, issuer
    );

    // Generate QR code
    let qr =
        QrCode::new(uri.as_bytes()).map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;

    // Convert to SVG
    let svg = qr.render::<qrcode::render::svg::Color>().max_dimensions(200, 200).build();

    // Return as data URL
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        general_purpose::STANDARD.encode(svg.as_bytes())
    ))
}

/// Generate backup codes for account recovery
///
/// # Arguments
/// * `count` - Number of backup codes to generate (default: 10)
///
/// # Returns
/// Vector of backup codes (8-digit codes)
pub fn generate_backup_codes(count: usize) -> Vec<String> {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let mut codes = Vec::new();

    for _ in 0..count {
        let mut bytes = [0u8; 4];
        rng.fill(&mut bytes).expect("Failed to generate backup code");
        // Generate 8-digit code
        let code = format!("{:08}", u32::from_be_bytes(bytes) % 100_000_000);
        codes.push(code);
    }

    codes
}

/// Hash a backup code using bcrypt
pub fn hash_backup_code(code: &str) -> Result<String> {
    Ok(bcrypt::hash(code, bcrypt::DEFAULT_COST)?)
}

/// Verify a backup code against a hashed code
pub fn verify_backup_code(code: &str, hash: &str) -> Result<bool> {
    Ok(bcrypt::verify(code, hash)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_generation_and_verification() {
        let secret = generate_secret();
        let code = generate_totp_code(&secret, None).unwrap();

        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));

        // Verify the code
        assert!(verify_totp_code(&secret, &code, Some(1)).unwrap());
    }

    #[test]
    fn test_backup_code_generation() {
        let codes = generate_backup_codes(10);
        assert_eq!(codes.len(), 10);

        for code in &codes {
            assert_eq!(code.len(), 8);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn test_backup_code_hashing() {
        let code = "12345678";
        let hash = hash_backup_code(code).unwrap();
        assert!(verify_backup_code(code, &hash).unwrap());
        assert!(!verify_backup_code("87654321", &hash).unwrap());
    }
}

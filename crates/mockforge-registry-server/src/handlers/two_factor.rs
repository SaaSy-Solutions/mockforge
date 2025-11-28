//! Two-Factor Authentication (2FA) handlers
//!
//! Handles 2FA setup, verification, and management

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{AuditEventType, User, record_audit_event},
    two_factor::{
        generate_backup_codes, generate_qr_code_data_url, generate_secret,
        hash_backup_code, verify_totp_code,
    },
    AppState,
};

#[derive(Debug, Serialize)]
pub struct Setup2FAResponse {
    pub secret: String, // Base32-encoded secret (for manual entry)
    pub qr_code_url: String, // Data URL for QR code
    pub backup_codes: Vec<String>, // Plain text backup codes (shown once)
}

#[derive(Debug, Deserialize)]
pub struct Verify2FASetupRequest {
    pub code: String, // 6-digit TOTP code
}

#[derive(Debug, Serialize)]
pub struct Verify2FASetupResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Disable2FARequest {
    pub password: String, // Require password confirmation
}

#[derive(Debug, Serialize)]
pub struct Disable2FAResponse {
    pub success: bool,
    pub message: String,
}

/// Start 2FA setup process
/// Generates a secret and QR code for the user to scan
pub async fn setup_2fa(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<Setup2FAResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Check if 2FA is already enabled
    if user.two_factor_enabled {
        return Err(ApiError::InvalidRequest(
            "2FA is already enabled. Disable it first to set up a new device.".to_string(),
        ));
    }

    // Generate TOTP secret
    let secret = generate_secret();

    // Generate backup codes (10 codes)
    let backup_codes = generate_backup_codes(10);

    // Generate QR code
    let issuer = "MockForge";
    let account_name = format!("{}:{}", user.email, user.username);
    let qr_code_url = generate_qr_code_data_url(&secret, &account_name, issuer)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to generate QR code: {}", e)))?;

    // Store secret temporarily (user needs to verify before enabling)
    // For now, we'll return it and require verification in the next step
    // In production, you might want to store it temporarily in Redis or session

    Ok(Json(Setup2FAResponse {
        secret: secret.clone(),
        qr_code_url,
        backup_codes: backup_codes.clone(),
    }))
}

/// Verify and enable 2FA
/// User must provide a valid TOTP code to confirm setup
pub async fn verify_2fa_setup(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<Verify2FASetupRequest>,
) -> ApiResult<Json<Verify2FASetupResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Check if 2FA is already enabled
    if user.two_factor_enabled {
        return Err(ApiError::InvalidRequest(
            "2FA is already enabled".to_string(),
        ));
    }

    // This is a simplified flow - in production, you'd retrieve the secret
    // from a temporary store (Redis/session) that was set in setup_2fa
    // For now, we'll require the secret to be passed in the request
    // In a real implementation, you'd store it temporarily after setup_2fa

    // For this implementation, we'll require the user to call setup_2fa first
    // and then immediately verify. The secret should be stored client-side temporarily.
    // A better approach would be to store it in Redis with a short TTL.

    // For now, return an error indicating the setup flow needs to be completed
    // The actual implementation would:
    // 1. Retrieve secret from temporary store (set in setup_2fa)
    // 2. Verify the code
    // 3. Hash backup codes
    // 4. Enable 2FA in database

    Err(ApiError::InvalidRequest(
        "Please call setup_2fa first to get a secret, then verify with a code from your authenticator app.".to_string(),
    ))
}

/// Simplified verify_2fa_setup that accepts secret
/// In production, this would retrieve the secret from a temporary store
pub async fn verify_2fa_setup_with_secret(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<Verify2FASetupRequestWithSecret>,
) -> ApiResult<Json<Verify2FASetupResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Check if 2FA is already enabled
    if user.two_factor_enabled {
        return Err(ApiError::InvalidRequest(
            "2FA is already enabled".to_string(),
        ));
    }

    // Verify TOTP code
    let valid = verify_totp_code(&request.secret, &request.code, Some(1))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("TOTP verification error: {}", e)))?;

    if !valid {
        return Err(ApiError::InvalidRequest(
            "Invalid verification code. Please try again.".to_string(),
        ));
    }

    // Generate and hash backup codes
    let backup_codes = generate_backup_codes(10);
    let hashed_backup_codes: Vec<String> = backup_codes
        .iter()
        .map(|code| hash_backup_code(code))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to hash backup codes: {}", e)))?;

    // Enable 2FA
    User::enable_2fa(pool, user_id, &request.secret, &hashed_backup_codes)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    // Use a sentinel UUID for user-level actions (no org)
    let user_org_id = uuid::Uuid::nil();
    record_audit_event(
        pool,
        user_org_id,
        Some(user_id),
        AuditEventType::TwoFactorEnabled,
        "Two-factor authentication enabled".to_string(),
        None,
        None,
        None,
    )
    .await;

    Ok(Json(Verify2FASetupResponse {
        success: true,
        message: "2FA has been enabled successfully. Please save your backup codes in a safe place.".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct Verify2FASetupRequestWithSecret {
    pub secret: String,
    pub code: String,
    pub backup_codes: Vec<String>, // Plain text backup codes from setup
}

/// Disable 2FA
/// Requires password confirmation for security
pub async fn disable_2fa(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<Disable2FARequest>,
) -> ApiResult<Json<Disable2FAResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Verify password
    use crate::auth::verify_password;
    let valid = verify_password(&request.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(e))?;

    if !valid {
        return Err(ApiError::InvalidRequest("Invalid password".to_string()));
    }

    // Check if 2FA is enabled
    if !user.two_factor_enabled {
        return Err(ApiError::InvalidRequest("2FA is not enabled".to_string()));
    }

    // Disable 2FA
    User::disable_2fa(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    // Use a sentinel UUID for user-level actions (no org)
    let user_org_id = uuid::Uuid::nil();
    record_audit_event(
        pool,
        user_org_id,
        Some(user_id),
        AuditEventType::TwoFactorDisabled,
        "Two-factor authentication disabled".to_string(),
        None,
        None,
        None,
    )
    .await;

    Ok(Json(Disable2FAResponse {
        success: true,
        message: "2FA has been disabled successfully.".to_string(),
    }))
}

/// Get 2FA status
pub async fn get_2fa_status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    Ok(Json(serde_json::json!({
        "enabled": user.two_factor_enabled,
        "verified_at": user.two_factor_verified_at,
        "backup_codes_count": user.two_factor_backup_codes.as_ref().map(|c| c.len()).unwrap_or(0),
    })))
}

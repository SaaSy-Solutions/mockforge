//! Authentication handlers

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        create_token_pair, hash_password, verify_password, verify_refresh_token,
        REFRESH_TOKEN_EXPIRY_DAYS,
    },
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::{trusted_proxy::extract_client_ip_from_headers, AuthUser},
    models::{organization::Plan, AuditEventType},
    AppState,
};

/// Resolve the source IP for an audit record from proxy headers.
///
/// Returns `None` when the extractor yields its `"unknown"` sentinel so the
/// audit `ip_address` column stays NULL rather than storing a placeholder.
fn audit_source_ip(headers: &HeaderMap) -> Option<String> {
    let ip = extract_client_ip_from_headers(headers);
    if ip == "unknown" {
        None
    } else {
        Some(ip)
    }
}

/// Best-effort resolution of a user's organization for an audit record.
///
/// Auth events are user-scoped but the audit log is org-partitioned, so we
/// attribute the event to the user's first organization. Falls back to the nil
/// UUID when the user belongs to no org or the lookup fails — auditing must
/// never block (or fail) the auth action itself (#871).
async fn audit_org_for_user(state: &AppState, user_id: Uuid) -> Uuid {
    match state.store.list_organizations_by_user(user_id).await {
        Ok(orgs) => orgs.first().map(|o| o.id).unwrap_or_else(Uuid::nil),
        Err(e) => {
            tracing::warn!("Failed to resolve org for auth audit (user {}): {}", user_id, e);
            Uuid::nil()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub two_factor_code: Option<String>, // Optional 2FA code (required if 2FA is enabled)
}

/// Legacy auth response (for backwards compatibility)
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

/// New auth response with both access and refresh tokens
#[derive(Debug, Serialize)]
pub struct AuthResponseV2 {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
    pub user_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponseV2>> {
    // Validate input
    if request.username.len() < 3 {
        return Err(ApiError::InvalidRequest("Username must be at least 3 characters".to_string()));
    }

    if request.password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }

    // Check if user already exists
    if state.store.find_user_by_email(&request.email).await?.is_some() {
        return Err(ApiError::InvalidRequest("Email already registered".to_string()));
    }

    if state.store.find_user_by_username(&request.username).await?.is_some() {
        return Err(ApiError::InvalidRequest("Username already taken".to_string()));
    }

    // Hash password
    let password_hash = hash_password(&request.password).map_err(ApiError::Internal)?;

    // Create user
    let user = state
        .store
        .create_user(&request.username, &request.email, &password_hash)
        .await?;

    // Auto-create a personal organization for the user
    let org_slug = format!("{}-personal", request.username.to_lowercase().replace(' ', "-"));
    if let Err(e) = state
        .store
        .create_organization(&format!("{}'s Org", request.username), &org_slug, user.id, Plan::Free)
        .await
    {
        tracing::warn!("Failed to create personal org for user {}: {}", user.id, e);
    }

    // Send verification email (non-blocking — don't fail registration if SMTP is down)
    match state.store.create_verification_token(user.id).await {
        Ok(verification_token) => {
            let verification_email = EmailService::generate_verification_email(
                &user.username,
                &user.email,
                &verification_token.token,
            );
            tokio::spawn(async move {
                match EmailService::from_env() {
                    Ok(email_service) => {
                        if let Err(e) = email_service.send(verification_email).await {
                            tracing::warn!("Failed to send verification email at signup: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Email service unavailable at signup: {}", e);
                    }
                }
            });
        }
        Err(e) => {
            tracing::warn!("Failed to create verification token at signup: {}", e);
        }
    }

    // Generate token pair (access + refresh)
    let (token_pair, jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    // Store refresh token JTI in database for revocation support
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state.db.store_refresh_token_jti(&jti, user.id, expires_at).await.map_err(|e| {
        tracing::warn!("Failed to store refresh token JTI: {}", e);
        ApiError::Internal(e)
    })?;

    Ok(Json(AuthResponseV2 {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponseV2>> {
    let source_ip = audit_source_ip(&headers);

    // Find user
    let user = match state.store.find_user_by_email(&request.email).await? {
        Some(user) => user,
        None => {
            // Unknown-user branch — emit LoginFailed so password-spray against
            // non-existent accounts is still visible (#871). user_id is NULL;
            // org is nil since we have no user to attribute to.
            state
                .store
                .record_audit_event(
                    Uuid::nil(),
                    None,
                    AuditEventType::LoginFailed,
                    "Login failed: unknown email".to_string(),
                    Some(serde_json::json!({
                        "attempted_email": request.email,
                        "reason": "unknown_user",
                    })),
                    source_ip.as_deref(),
                    None,
                )
                .await;
            return Err(ApiError::InvalidRequest("Invalid email or password".to_string()));
        }
    };

    // Verify password
    let valid =
        verify_password(&request.password, &user.password_hash).map_err(ApiError::Internal)?;

    if !valid {
        // Wrong-password branch — emit LoginFailed for brute-force visibility
        // (#871). We know the user, so attribute the org + user_id.
        let org_id = audit_org_for_user(&state, user.id).await;
        state
            .store
            .record_audit_event(
                org_id,
                Some(user.id),
                AuditEventType::LoginFailed,
                "Login failed: incorrect password".to_string(),
                Some(serde_json::json!({
                    "attempted_email": request.email,
                    "reason": "bad_password",
                })),
                source_ip.as_deref(),
                None,
            )
            .await;
        return Err(ApiError::InvalidRequest("Invalid email or password".to_string()));
    }

    // Check if 2FA is enabled
    if user.two_factor_enabled {
        // Require 2FA code
        let code = request
            .two_factor_code
            .ok_or_else(|| ApiError::InvalidRequest("2FA code is required".to_string()))?;

        // Get secret
        let secret = user.two_factor_secret.ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("2FA enabled but no secret found"))
        })?;

        // Verify TOTP code
        use crate::two_factor::verify_totp_code;
        let totp_valid = verify_totp_code(&secret, &code, Some(1))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("TOTP verification error: {}", e)))?;

        if !totp_valid {
            // Try backup codes
            let mut backup_valid = false;
            if let Some(backup_codes) = &user.two_factor_backup_codes {
                use crate::two_factor::verify_backup_code;
                for (index, hashed_code) in backup_codes.iter().enumerate() {
                    if verify_backup_code(&code, hashed_code).map_err(|e| {
                        ApiError::Internal(anyhow::anyhow!("Backup code verification error: {}", e))
                    })? {
                        // Remove used backup code
                        state.store.remove_user_backup_code(user.id, index).await?;
                        backup_valid = true;
                        break;
                    }
                }
            }

            if !backup_valid {
                return Err(ApiError::InvalidRequest("Invalid 2FA code".to_string()));
            }
        }

        // Update 2FA verified timestamp
        state.store.update_user_2fa_verified(user.id).await?;
    }

    // Generate token pair (access + refresh)
    let (token_pair, jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    // Store refresh token JTI in database for revocation support
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state.db.store_refresh_token_jti(&jti, user.id, expires_at).await.map_err(|e| {
        tracing::warn!("Failed to store refresh token JTI: {}", e);
        ApiError::Internal(e)
    })?;

    // Successful login — audit with org (if resolvable), user_id and source IP (#871).
    let org_id = audit_org_for_user(&state, user.id).await;
    state
        .store
        .record_audit_event(
            org_id,
            Some(user.id),
            AuditEventType::LoginSucceeded,
            "Login succeeded".to_string(),
            Some(serde_json::json!({
                "two_factor": user.two_factor_enabled,
            })),
            source_ip.as_deref(),
            None,
        )
        .await;

    Ok(Json(AuthResponseV2 {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Response for refresh token endpoint
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> ApiResult<Json<RefreshTokenResponse>> {
    // Verify the refresh token (not just any token)
    let (claims, old_jti) = verify_refresh_token(&request.refresh_token, &state.config.jwt_secret)
        .map_err(|e| {
            tracing::debug!("Refresh token validation failed: {}", e);
            ApiError::InvalidRequest("Invalid or expired refresh token".to_string())
        })?;

    // Check if the JTI has been revoked in the database
    let is_revoked = state.db.is_token_revoked(&old_jti).await.map_err(|e| {
        tracing::warn!("Failed to check token revocation status: {}", e);
        ApiError::Internal(e)
    })?;

    if is_revoked {
        tracing::warn!("Attempt to use revoked refresh token: jti={}", old_jti);
        return Err(ApiError::InvalidRequest("Refresh token has been revoked".to_string()));
    }

    // Parse user ID from claims
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Find user to ensure they still exist and are active
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Revoke old refresh token JTI (token rotation for security)
    state.db.revoke_token(&old_jti, "refresh").await.map_err(|e| {
        tracing::warn!("Failed to revoke old refresh token: {}", e);
        ApiError::Internal(e)
    })?;

    // Generate new token pair
    let (token_pair, new_jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    // Store new refresh token JTI in database
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state
        .db
        .store_refresh_token_jti(&new_jti, user.id, expires_at)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to store new refresh token JTI: {}", e);
            ApiError::Internal(e)
        })?;

    Ok(Json(RefreshTokenResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
    }))
}

// Password reset handlers (moved here to avoid axum version conflicts)

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetRequestResponse {
    pub success: bool,
    pub message: String,
}

/// Request password reset (sends email with reset token)
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetRequest>,
) -> ApiResult<Json<PasswordResetRequestResponse>> {
    // Find user by email
    let user = match state.store.find_user_by_email(&request.email).await? {
        Some(user) => user,
        None => {
            // Don't reveal if email exists or not (security best practice)
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message:
                    "If an account with that email exists, a password reset link has been sent."
                        .to_string(),
            }));
        }
    };

    // Create password reset token (reusing VerificationToken model).
    // Token expires in 1 hour instead of the default 24.
    let reset_token = state.store.create_verification_token(user.id).await?;
    state.store.set_verification_token_expiry_hours(reset_token.id, 1).await?;

    // Send password reset email (non-blocking)
    let email_service = match EmailService::from_env() {
        Ok(service) => service,
        Err(e) => {
            tracing::warn!("Failed to create email service: {}", e);
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message:
                    "If an account with that email exists, a password reset link has been sent."
                        .to_string(),
            }));
        }
    };
    let reset_email = EmailService::generate_password_reset_email(
        &user.username,
        &user.email,
        &reset_token.token,
    );

    tokio::spawn(async move {
        if let Err(e) = email_service.send(reset_email).await {
            tracing::warn!("Failed to send password reset email: {}", e);
        }
    });

    tracing::info!("Password reset requested: user_id={}, email={}", user.id, user.email);

    Ok(Json(PasswordResetRequestResponse {
        success: true,
        message: "If an account with that email exists, a password reset link has been sent."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetConfirmResponse {
    pub success: bool,
    pub message: String,
}

/// Confirm password reset (with token and new password)
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetConfirmRequest>,
) -> ApiResult<Json<PasswordResetConfirmResponse>> {
    // Validate password
    if request.new_password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }

    // Find token
    let reset_token = state
        .store
        .find_verification_token_by_token(&request.token)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Invalid or expired reset token".to_string()))?;

    // Check if token is valid (not expired and not used)
    if !reset_token.is_valid() {
        return Err(ApiError::InvalidRequest(
            "Reset token has expired or already been used".to_string(),
        ));
    }

    // Get user
    let user = state
        .store
        .find_user_by_id(reset_token.user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Hash new password
    let password_hash = hash_password(&request.new_password).map_err(ApiError::Internal)?;

    // Update user password
    state.store.update_user_password_hash(user.id, &password_hash).await?;

    // Revoke all existing refresh tokens for security (password changed)
    let revoked_count =
        state.db.revoke_all_user_tokens(user.id, "password_reset").await.map_err(|e| {
            tracing::warn!("Failed to revoke user tokens on password reset: {}", e);
            ApiError::Internal(e)
        })?;

    tracing::info!(
        "Revoked {} refresh tokens for user {} on password reset",
        revoked_count,
        user.id
    );

    // Mark token as used
    state.store.mark_verification_token_used(reset_token.id).await?;

    tracing::info!("Password reset completed: user_id={}, email={}", user.id, user.email);

    Ok(Json(PasswordResetConfirmResponse {
        success: true,
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}

/// Verify token response
#[derive(Debug, Serialize)]
pub struct VerifyTokenResponse {
    pub valid: bool,
    pub user_id: String,
    pub username: String,
    pub email: String,
}

/// Verify that the current JWT is valid (GET /api/v1/auth/verify)
pub async fn verify_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<VerifyTokenResponse>> {
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    Ok(Json(VerifyTokenResponse {
        valid: true,
        user_id: user.id.to_string(),
        username: user.username,
        email: user.email,
    }))
}

/// User info response
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
    pub is_admin: bool,
    pub two_factor_enabled: bool,
    pub email_notifications: bool,
    pub security_alerts: bool,
    pub preferences: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Get current user info (GET /api/v1/auth/me)
pub async fn me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<MeResponse>> {
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    Ok(Json(MeResponse {
        user_id: user.id.to_string(),
        username: user.username,
        email: user.email,
        is_verified: user.is_verified,
        is_admin: user.is_admin,
        two_factor_enabled: user.two_factor_enabled,
        email_notifications: user.email_notifications,
        security_alerts: user.security_alerts,
        preferences: user.preferences,
        created_at: user.created_at,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub success: bool,
    pub message: String,
}

/// Change password for the authenticated user.
///
/// Verifies the user's current password, stores the new hash, revokes any
/// outstanding refresh tokens (so other sessions are cut off), and — when
/// the user has opted in to security alerts — sends a notification email.
pub async fn change_password(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> ApiResult<Json<ChangePasswordResponse>> {
    if request.new_password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }
    if request.new_password == request.current_password {
        return Err(ApiError::InvalidRequest(
            "New password must differ from the current password".to_string(),
        ));
    }

    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    if !verify_password(&request.current_password, &user.password_hash)
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::InvalidRequest("Current password is incorrect".to_string()));
    }

    let password_hash = hash_password(&request.new_password).map_err(ApiError::Internal)?;
    state.store.update_user_password_hash(user.id, &password_hash).await?;

    let revoked_count = state
        .db
        .revoke_all_user_tokens(user.id, "password_changed")
        .await
        .map_err(|e| {
            tracing::warn!("Failed to revoke user tokens on password change: {}", e);
            ApiError::Internal(e)
        })?;
    tracing::info!(
        "Password changed: user_id={}, revoked {} refresh tokens",
        user.id,
        revoked_count
    );

    // Audit the password change with the existing PasswordChanged type (#873).
    let org_id = audit_org_for_user(&state, user.id).await;
    state
        .store
        .record_audit_event(
            org_id,
            Some(user.id),
            AuditEventType::PasswordChanged,
            "Password changed".to_string(),
            Some(serde_json::json!({
                "revoked_sessions": revoked_count,
            })),
            audit_source_ip(&headers).as_deref(),
            None,
        )
        .await;

    // Best-effort security-alert email. Never fails the request.
    if user.security_alerts {
        if let Ok(email_service) = EmailService::from_env() {
            let msg = EmailService::generate_security_alert_email(
                &user.username,
                &user.email,
                "Your password was changed",
                "If you did not perform this change, reset your password immediately and contact support.",
            );
            if let Err(e) = email_service.send(msg).await {
                tracing::warn!("Failed to send password-change security alert: {}", e);
            }
        }
    }

    Ok(Json(ChangePasswordResponse {
        success: true,
        message: "Password changed successfully. Other sessions have been signed out.".to_string(),
    }))
}

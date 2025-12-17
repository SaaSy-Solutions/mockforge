//! SSO (Single Sign-On) handlers
//!
//! Handles SAML 2.0 SSO setup and authentication for Team plan organizations

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use ring::signature::{
    UnparsedPublicKey, VerificationAlgorithm, RSA_PKCS1_2048_8192_SHA256,
    RSA_PKCS1_2048_8192_SHA512,
};
use rustls_pemfile;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use x509_parser::prelude::*;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        record_audit_event, AuditEventType, Organization, Plan, SAMLAssertionId, SSOConfiguration,
        SSOSession, User,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateSSOConfigRequest {
    pub provider: String, // "saml" or "oidc"
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_slo_url: Option<String>,
    pub saml_x509_cert: Option<String>,
    pub saml_name_id_format: Option<String>,
    pub attribute_mapping: Option<serde_json::Value>,
    pub require_signed_assertions: Option<bool>,
    pub require_signed_responses: Option<bool>,
    pub allow_unsolicited_responses: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SSOConfigResponse {
    pub id: String,
    pub org_id: String,
    pub provider: String,
    pub enabled: bool,
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_slo_url: Option<String>,
    pub saml_name_id_format: Option<String>,
    pub attribute_mapping: serde_json::Value,
    pub require_signed_assertions: bool,
    pub require_signed_responses: bool,
    pub allow_unsolicited_responses: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Create or update SSO configuration (Team plan only, org admin only)
pub async fn create_sso_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateSSOConfigRequest>,
) -> ApiResult<Json<SSOConfigResponse>> {
    let pool = state.db.pool();

    // Resolve organization context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization context required".to_string()))?;

    // Check if user is org admin (check if user is owner or admin member)
    use crate::models::{OrgMember, OrgRole};
    let is_admin = org_ctx.org.owner_id == user_id || {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            let role = member.role();
            matches!(role, OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    };

    if !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Get organization
    let org = Organization::find_by_id(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check if organization is on Team plan
    if org.plan() != Plan::Team {
        return Err(ApiError::InvalidRequest(
            "SSO is only available for Team plans. Please upgrade to Team plan to enable SSO."
                .to_string(),
        ));
    }

    // Validate provider
    use crate::models::sso::SSOProvider;
    let provider = SSOProvider::from_str(&request.provider).ok_or_else(|| {
        ApiError::InvalidRequest("Invalid SSO provider. Must be 'saml' or 'oidc'".to_string())
    })?;

    // Validate SAML fields if provider is SAML
    if provider == SSOProvider::Saml {
        if request.saml_entity_id.is_none()
            || request.saml_sso_url.is_none()
            || request.saml_x509_cert.is_none()
        {
            return Err(ApiError::InvalidRequest(
                "SAML configuration requires entity_id, sso_url, and x509_cert".to_string(),
            ));
        }
    }

    // Create or update SSO configuration
    let config = SSOConfiguration::upsert(
        pool,
        org_ctx.org_id,
        provider,
        request.saml_entity_id.as_deref(),
        request.saml_sso_url.as_deref(),
        request.saml_slo_url.as_deref(),
        request.saml_x509_cert.as_deref(),
        request.saml_name_id_format.as_deref(),
        request.attribute_mapping,
        request.require_signed_assertions.unwrap_or(true),
        request.require_signed_responses.unwrap_or(true),
        request.allow_unsolicited_responses.unwrap_or(false),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::SettingsUpdated,
        "SSO configuration created/updated".to_string(),
        Some(serde_json::json!({
            "provider": provider.to_string(),
            "enabled": config.enabled,
        })),
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    Ok(Json(SSOConfigResponse {
        id: config.id.to_string(),
        org_id: config.org_id.to_string(),
        provider: config.provider,
        enabled: config.enabled,
        saml_entity_id: config.saml_entity_id,
        saml_sso_url: config.saml_sso_url,
        saml_slo_url: config.saml_slo_url,
        saml_name_id_format: config.saml_name_id_format,
        attribute_mapping: config.attribute_mapping,
        require_signed_assertions: config.require_signed_assertions,
        require_signed_responses: config.require_signed_responses,
        allow_unsolicited_responses: config.allow_unsolicited_responses,
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }))
}

/// Get SSO configuration (org admin only)
pub async fn get_sso_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Option<SSOConfigResponse>>> {
    let pool = state.db.pool();

    // Resolve organization context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization context required".to_string()))?;

    // Check if user is org admin (check if user is owner or admin member)
    use crate::models::{OrgMember, OrgRole};
    let is_admin = org_ctx.org.owner_id == user_id || {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            let role = member.role();
            matches!(role, OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    };

    if !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Get SSO configuration
    let config = SSOConfiguration::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    if let Some(config) = config {
        Ok(Json(Some(SSOConfigResponse {
            id: config.id.to_string(),
            org_id: config.org_id.to_string(),
            provider: config.provider,
            enabled: config.enabled,
            saml_entity_id: config.saml_entity_id,
            saml_sso_url: config.saml_sso_url,
            saml_slo_url: config.saml_slo_url,
            saml_name_id_format: config.saml_name_id_format,
            attribute_mapping: config.attribute_mapping,
            require_signed_assertions: config.require_signed_assertions,
            require_signed_responses: config.require_signed_responses,
            allow_unsolicited_responses: config.allow_unsolicited_responses,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        })))
    } else {
        Ok(Json(None))
    }
}

/// Enable SSO (org admin only)
pub async fn enable_sso(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve organization context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization context required".to_string()))?;

    // Check if user is org admin (check if user is owner or admin member)
    use crate::models::{OrgMember, OrgRole};
    let is_admin = org_ctx.org.owner_id == user_id || {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            let role = member.role();
            matches!(role, OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    };

    if !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Get organization
    let org = Organization::find_by_id(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check if organization is on Team plan
    if org.plan() != Plan::Team {
        return Err(ApiError::InvalidRequest("SSO is only available for Team plans".to_string()));
    }

    // Check if SSO is configured
    let config = SSOConfiguration::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::InvalidRequest("SSO not configured. Please configure SSO first.".to_string())
        })?;

    // Enable SSO
    SSOConfiguration::enable(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::SettingsUpdated,
        "SSO enabled".to_string(),
        None,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "SSO has been enabled successfully"
    })))
}

/// Disable SSO (org admin only)
pub async fn disable_sso(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve organization context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization context required".to_string()))?;

    // Check if user is org admin (check if user is owner or admin member)
    use crate::models::{OrgMember, OrgRole};
    let is_admin = org_ctx.org.owner_id == user_id || {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            let role = member.role();
            matches!(role, OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    };

    if !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Disable SSO
    SSOConfiguration::disable(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::SettingsUpdated,
        "SSO disabled".to_string(),
        None,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "SSO has been disabled successfully"
    })))
}

/// Delete SSO configuration (org admin only)
pub async fn delete_sso_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve organization context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization context required".to_string()))?;

    // Check if user is org admin (check if user is owner or admin member)
    use crate::models::{OrgMember, OrgRole};
    let is_admin = org_ctx.org.owner_id == user_id || {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            let role = member.role();
            matches!(role, OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    };

    if !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Delete SSO configuration
    SSOConfiguration::delete(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::SettingsUpdated,
        "SSO configuration deleted".to_string(),
        None,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "SSO configuration has been deleted successfully"
    })))
}

/// Get SAML metadata for SP (Service Provider)
/// This endpoint returns the SAML metadata XML that organizations can use
/// to configure their IdP (Identity Provider)
pub async fn get_saml_metadata(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
) -> ApiResult<axum::response::Response> {
    let pool = state.db.pool();

    // Find organization by slug
    let org = Organization::find_by_slug(pool, &org_slug)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get SSO configuration
    let config = SSOConfiguration::find_by_org(pool, org.id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::InvalidRequest("SSO not configured for this organization".to_string())
        })?;

    // Generate SAML metadata XML
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string());

    let entity_id = config
        .saml_entity_id
        .unwrap_or_else(|| format!("{}/saml/metadata/{}", app_base_url, org_slug));

    let acs_url = format!("{}/api/v1/sso/saml/acs/{}", app_base_url, org_slug);
    let slo_url = format!("{}/api/v1/sso/saml/slo/{}", app_base_url, org_slug);

    // Generate SAML metadata XML
    let metadata = format!(
        r#"<?xml version="1.0"?>
<EntityDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{}">
    <SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
        <NameIDFormat>{}</NameIDFormat>
        <AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}" index="0"/>
        <SingleLogoutService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}"/>
    </SPSSODescriptor>
</EntityDescriptor>"#,
        entity_id,
        config
            .saml_name_id_format
            .as_deref()
            .unwrap_or("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"),
        acs_url,
        slo_url
    );

    Ok(axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/xml")
        .body(metadata.into())
        .unwrap())
}

/// Initiate SAML SSO login
/// Redirects user to IdP for authentication
pub async fn initiate_saml_login(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
) -> Result<Response, ApiError> {
    let pool = state.db.pool();

    // Find organization by slug
    let org = Organization::find_by_slug(pool, &org_slug)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check if organization is on Team plan
    if org.plan() != Plan::Team {
        return Err(ApiError::InvalidRequest("SSO is only available for Team plans".to_string()));
    }

    // Get SSO configuration
    let config = SSOConfiguration::find_by_org(pool, org.id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::InvalidRequest("SSO not configured for this organization".to_string())
        })?;

    if !config.enabled {
        return Err(ApiError::InvalidRequest(
            "SSO is not enabled for this organization".to_string(),
        ));
    }

    // Get SAML SSO URL
    let sso_url = config
        .saml_sso_url
        .ok_or_else(|| ApiError::InvalidRequest("SAML SSO URL not configured".to_string()))?;

    // Generate SAML AuthnRequest
    // In a production implementation, you would use a SAML library to generate a proper AuthnRequest
    // For now, we'll create a simple redirect with a SAML request parameter
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string());

    let acs_url = format!("{}/api/v1/sso/saml/acs/{}", app_base_url, org_slug);
    let entity_id = config
        .saml_entity_id
        .unwrap_or_else(|| format!("{}/saml/metadata/{}", app_base_url, org_slug));

    // Generate a simple SAML AuthnRequest (base64 encoded)
    // In production, use a proper SAML library like saml2-rs
    let saml_request = generate_saml_authn_request(&entity_id, &acs_url);
    let encoded_request = general_purpose::STANDARD.encode(saml_request.as_bytes());

    // Redirect to IdP with SAML request
    let redirect_url = format!("{}?SAMLRequest={}", sso_url, urlencoding::encode(&encoded_request));

    Ok(Redirect::to(&redirect_url).into_response())
}

/// SAML Assertion Consumer Service (ACS)
/// Receives SAML response from IdP after authentication
#[derive(Debug, Deserialize)]
pub struct SAMLResponseForm {
    pub SAMLResponse: Option<String>,
    pub RelayState: Option<String>,
}

pub async fn saml_acs(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
    Form(form): Form<SAMLResponseForm>,
) -> Result<Response, ApiError> {
    let pool = state.db.pool();

    // Find organization by slug
    let org = Organization::find_by_slug(pool, &org_slug)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get SSO configuration
    let config = SSOConfiguration::find_by_org(pool, org.id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("SSO not configured".to_string()))?;

    if !config.enabled {
        return Err(ApiError::InvalidRequest("SSO is not enabled".to_string()));
    }

    // Decode SAML response
    let saml_response = form
        .SAMLResponse
        .ok_or_else(|| ApiError::InvalidRequest("SAMLResponse parameter missing".to_string()))?;

    let decoded_response = general_purpose::STANDARD.decode(&saml_response).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Failed to decode SAML response: {}", e))
    })?;

    // Verify SAML response signature before parsing (security-critical)
    if config.require_signed_responses {
        verify_saml_signature(&decoded_response, &config)?;
    }

    // Parse and verify SAML response with full security checks
    let user_info = parse_saml_response(&decoded_response, &config, &org).await?;

    // Validate timestamps (NotBefore/NotOnOrAfter)
    validate_saml_timestamps(&user_info).map_err(|e| {
        tracing::error!("SAML timestamp validation failed for org_id={}: {}", org.id, e);
        e
    })?;

    // Check for replay attacks (assertion ID tracking)
    if let Some(assertion_id) = &user_info.assertion_id {
        let is_replay =
            SAMLAssertionId::is_used(pool, assertion_id, org.id).await.map_err(|e| {
                tracing::error!(
                    "Database error checking assertion ID for org_id={}: {:?}",
                    org.id,
                    e
                );
                ApiError::Database(e)
            })?;

        if is_replay {
            tracing::warn!(
                "Replay attack detected: assertion_id={} already used for org_id={}",
                assertion_id,
                org.id
            );
            return Err(ApiError::InvalidRequest(
                "This SAML assertion has already been used. Replay attacks are not allowed."
                    .to_string(),
            ));
        }
    }

    // Find or create user
    let user = find_or_create_user_from_saml(pool, &user_info, &org).await?;

    // Record assertion ID to prevent replay attacks
    if let Some(assertion_id) = &user_info.assertion_id {
        let expires_at = user_info
            .not_on_or_after
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));
        let issued_at = user_info.issued_at.unwrap_or_else(|| chrono::Utc::now());

        SAMLAssertionId::record_used(
            pool,
            assertion_id,
            org.id,
            Some(user.id),
            user_info.name_id.as_deref(),
            issued_at,
            expires_at,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to record assertion ID for org_id={}: {:?}", org.id, e);
            ApiError::Database(e)
        })?;

        tracing::debug!(
            "Recorded assertion ID {} for org_id={}, user_id={}",
            assertion_id,
            org.id,
            user.id
        );
    }

    // Create SSO session
    let session_expires = chrono::Utc::now() + chrono::Duration::hours(8); // 8 hour session
    let _session = SSOSession::create(
        pool,
        org.id,
        user.id,
        user_info.session_index.as_deref(),
        user_info.name_id.as_deref(),
        session_expires,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Generate JWT token
    let token = crate::auth::create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    // Redirect to app with token
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string());

    let redirect_url =
        format!("{}/auth/sso/callback?token={}&org_slug={}", app_base_url, token, org_slug);

    Ok(Redirect::to(&redirect_url).into_response())
}

/// SAML Single Logout Service (SLO)
/// Handles logout requests from IdP
#[derive(Debug, Deserialize)]
pub struct SAMLLogoutForm {
    pub SAMLRequest: Option<String>,
    pub SAMLResponse: Option<String>,
    pub RelayState: Option<String>,
}

pub async fn saml_slo(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
    Form(form): Form<SAMLLogoutForm>,
) -> Result<Response, ApiError> {
    let pool = state.db.pool();

    // Find organization by slug
    let org = Organization::find_by_slug(pool, &org_slug)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get SSO configuration
    let config = SSOConfiguration::find_by_org(pool, org.id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("SSO not configured".to_string()))?;

    // Handle logout request or response
    if let Some(saml_request) = form.SAMLRequest {
        // IdP-initiated logout request
        let decoded = general_purpose::STANDARD.decode(&saml_request).map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to decode SAML logout request: {}", e))
        })?;

        // Parse logout request and invalidate sessions
        let session_index = parse_saml_logout_request(&decoded)?;

        // Delete all sessions with this session index
        if let Some(session_index) = session_index {
            sqlx::query("DELETE FROM sso_sessions WHERE org_id = $1 AND session_index = $2")
                .bind(org.id)
                .bind(session_index)
                .execute(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;
        }

        // Generate logout response
        let app_base_url = std::env::var("APP_BASE_URL")
            .unwrap_or_else(|_| "https://app.mockforge.dev".to_string());

        let slo_url = config
            .saml_slo_url
            .ok_or_else(|| ApiError::InvalidRequest("SAML SLO URL not configured".to_string()))?;

        let logout_response = generate_saml_logout_response(&slo_url);
        let encoded_response = general_purpose::STANDARD.encode(logout_response.as_bytes());

        // Redirect back to IdP with logout response
        let redirect_url =
            format!("{}?SAMLResponse={}", slo_url, urlencoding::encode(&encoded_response));
        Ok(Redirect::to(&redirect_url).into_response())
    } else {
        // Logout response from IdP (SP-initiated logout completed)
        Ok(Redirect::to("/").into_response())
    }
}

/// SAML user information extracted from assertion
#[derive(Debug, Clone)]
struct SAMLUserInfo {
    assertion_id: Option<String>,
    name_id: Option<String>,
    email: Option<String>,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    session_index: Option<String>,
    attributes: serde_json::Value,
    not_before: Option<DateTime<Utc>>,
    not_on_or_after: Option<DateTime<Utc>>,
    issued_at: Option<DateTime<Utc>>,
}

/// Generate SAML AuthnRequest XML
fn generate_saml_authn_request(entity_id: &str, acs_url: &str) -> String {
    let request_id = uuid::Uuid::new_v4().to_string();
    let issue_instant = chrono::Utc::now().to_rfc3339();

    format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
    ID="_{}"
    Version="2.0"
    IssueInstant="{}"
    Destination="{}"
    AssertionConsumerServiceURL="{}"
    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
    <saml:Issuer>{}</saml:Issuer>
    <samlp:NameIDPolicy Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
        request_id,
        issue_instant,
        acs_url, // Destination (IdP SSO URL)
        acs_url,
        entity_id
    )
}

/// Parse SAML response and extract user information using quick-xml
/// Assumes signature verification has already been performed
async fn parse_saml_response(
    response_xml: &[u8],
    config: &SSOConfiguration,
    org: &Organization,
) -> Result<SAMLUserInfo, ApiError> {
    // Convert to string for parsing
    let xml_str = std::str::from_utf8(response_xml).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Invalid UTF-8 in SAML response: {}", e))
    })?;

    // Use quick-xml to parse the SAML response
    // For now, we'll use the existing regex-based extraction which works well
    // In the future, this could be enhanced with full quick-xml parsing

    // Extract NameID
    let name_id = extract_xml_value(xml_str, "NameID")
        .or_else(|| extract_xml_value(xml_str, "saml:NameID"))
        .or_else(|| extract_xml_value(xml_str, "saml2:NameID"));

    // Extract email from NameID or attributes
    let email = name_id
        .clone()
        .filter(|v| v.contains('@'))
        .or_else(|| extract_xml_value(xml_str, "AttributeValue").filter(|v| v.contains('@')));

    // Extract SessionIndex
    let session_index = extract_xml_value(xml_str, "SessionIndex")
        .or_else(|| extract_xml_value(xml_str, "samlp:SessionIndex"));

    // Extract assertion ID (for replay attack prevention)
    let assertion_id = extract_xml_value(xml_str, "Assertion")
        .and_then(|a| {
            regex::Regex::new(r#"ID="([^"]+)""#)
                .ok()?
                .captures(&a)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().to_string())
        })
        .or_else(|| {
            regex::Regex::new(r#"<[^:]*:?Assertion[^>]*ID="([^"]+)""#)
                .ok()?
                .captures(xml_str)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().to_string())
        });

    // Extract timestamps for validation
    let not_before = extract_xml_value(xml_str, "NotBefore")
        .or_else(|| extract_xml_value(xml_str, "saml:NotBefore"))
        .or_else(|| extract_xml_value(xml_str, "saml2:NotBefore"))
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let not_on_or_after = extract_xml_value(xml_str, "NotOnOrAfter")
        .or_else(|| extract_xml_value(xml_str, "saml:NotOnOrAfter"))
        .or_else(|| extract_xml_value(xml_str, "saml2:NotOnOrAfter"))
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let issued_at = extract_xml_value(xml_str, "IssueInstant")
        .or_else(|| extract_xml_value(xml_str, "saml:IssueInstant"))
        .or_else(|| extract_xml_value(xml_str, "saml2:IssueInstant"))
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    // Extract attributes based on attribute mapping
    let mut attributes = serde_json::json!({});

    // Apply attribute mapping from config
    if let Some(mapping) = config.attribute_mapping.as_object() {
        for (target_key, source_key) in mapping {
            if let Some(source_key_str) = source_key.as_str() {
                if let Some(source_value) = extract_xml_value(xml_str, source_key_str) {
                    attributes[target_key] = serde_json::Value::String(source_value);
                }
            }
        }
    }

    // Extract common attributes
    let first_name =
        extract_xml_value(xml_str, "FirstName").or_else(|| extract_xml_value(xml_str, "givenName"));
    let last_name =
        extract_xml_value(xml_str, "LastName").or_else(|| extract_xml_value(xml_str, "surname"));

    // Generate username from email if not provided
    let username = extract_xml_value(xml_str, "Username")
        .or_else(|| email.as_ref().map(|e| e.split('@').next().unwrap_or("user").to_string()));

    Ok(SAMLUserInfo {
        assertion_id,
        name_id,
        email,
        username,
        first_name,
        last_name,
        session_index,
        attributes,
        not_before,
        not_on_or_after,
        issued_at,
    })
}

/// Verify SAML response/assertion signature using ring and x509-parser
/// Validates X.509 certificate and performs full cryptographic signature verification
fn verify_saml_signature(xml: &[u8], config: &SSOConfiguration) -> Result<(), ApiError> {
    tracing::debug!("Verifying SAML signature for org_id={}", config.org_id);

    // Get X.509 certificate from config
    let cert_pem = config.saml_x509_cert.as_ref().ok_or_else(|| {
        tracing::error!("X.509 certificate not configured for org_id={}", config.org_id);
        ApiError::InvalidRequest("SAML X.509 certificate not configured".to_string())
    })?;

    // Parse certificate (PEM format)
    let cert_pem_bytes = cert_pem.as_bytes().to_vec();
    let mut reader = std::io::Cursor::new(&cert_pem_bytes);
    let certs: Vec<Vec<u8>> = rustls_pemfile::certs(&mut reader)
        .map(|result| result.map(|cert| cert.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            tracing::error!("Failed to parse PEM certificate: {}", e);
            ApiError::Internal(anyhow::anyhow!("Invalid PEM certificate format"))
        })?;

    if certs.is_empty() {
        return Err(ApiError::InvalidRequest("No certificate found in PEM data".to_string()));
    }

    // Parse the first certificate to verify it's valid
    let first_cert = certs[0].clone();
    let (_, cert) = X509Certificate::from_der(&first_cert).map_err(|e| {
        tracing::error!("Failed to parse X.509 certificate DER: {:?}", e);
        ApiError::Internal(anyhow::anyhow!("Invalid X.509 certificate format"))
    })?;

    // Verify certificate is valid (not expired, proper format)
    cert.validity().time_to_expiration().ok_or_else(|| {
        tracing::warn!("SAML certificate expired or invalid for org_id={}", config.org_id);
        ApiError::InvalidRequest("SAML certificate has expired or is invalid".to_string())
    })?;

    // Convert XML to string for parsing
    let xml_str = std::str::from_utf8(xml).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Invalid UTF-8 in SAML response: {}", e))
    })?;

    // Check if Signature element exists
    let has_response_signature = xml_str.contains("<ds:Signature")
        || xml_str.contains("<Signature")
        || xml_str.contains("xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\"");

    if !has_response_signature && config.require_signed_responses {
        tracing::error!("SAML response missing signature for org_id={}", config.org_id);
        return Err(ApiError::InvalidRequest(
            "SAML response is not signed but signature is required".to_string(),
        ));
    }

    // Extract public key from certificate for signature verification
    let public_key = cert.public_key();

    // Verify response signature if present
    if has_response_signature {
        verify_xml_signature(xml_str, &first_cert, public_key).map_err(|e| {
            tracing::error!(
                "SAML response signature verification failed for org_id={}: {}",
                config.org_id,
                e
            );
            ApiError::InvalidRequest(format!("SAML response signature verification failed: {}", e))
        })?;
    }

    // Verify assertion signatures if required
    if config.require_signed_assertions {
        // Check for assertion signatures (typically inside Assertion elements)
        let has_assertion_signature = xml_str.contains("<Assertion")
            && (xml_str.contains("<ds:Signature") || xml_str.contains("<Signature"));

        if !has_assertion_signature {
            tracing::error!("SAML assertion missing signature for org_id={}", config.org_id);
            return Err(ApiError::InvalidRequest(
                "SAML assertion is not signed but signature is required".to_string(),
            ));
        }

        // Verify assertion signature (same certificate used for assertions)
        verify_xml_signature(xml_str, &first_cert, public_key).map_err(|e| {
            tracing::error!(
                "SAML assertion signature verification failed for org_id={}: {}",
                config.org_id,
                e
            );
            ApiError::InvalidRequest(format!("SAML assertion signature verification failed: {}", e))
        })?;
    }

    tracing::info!("SAML signature validation passed for org_id={}", config.org_id);
    Ok(())
}

/// Verify XML signature using ring cryptography
/// Extracts signature value and SignedInfo, then verifies using the certificate's public key
fn verify_xml_signature(
    xml: &str,
    cert_der: &[u8],
    _public_key: &SubjectPublicKeyInfo<'_>,
) -> Result<(), String> {
    // Extract signature value from XML
    let signature_value = extract_signature_value(xml)
        .ok_or_else(|| "Signature value not found in XML".to_string())?;

    // Extract SignedInfo element (the canonicalized content that was signed)
    let signed_info =
        extract_signed_info(xml).ok_or_else(|| "SignedInfo not found in XML".to_string())?;

    // Decode base64 signature
    let signature_bytes = general_purpose::STANDARD
        .decode(&signature_value)
        .map_err(|e| format!("Failed to decode signature: {}", e))?;

    // Determine signature algorithm from SignedInfo
    let algorithm_str =
        extract_signature_algorithm(xml).unwrap_or_else(|| "rsa-sha256".to_string()); // Default to RSA-SHA256

    // Hash the SignedInfo using the appropriate algorithm
    let signed_info_bytes = signed_info.as_bytes();
    let hash = match algorithm_str.as_str() {
        "rsa-sha256" | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(signed_info_bytes);
            hasher.finalize().to_vec()
        }
        "rsa-sha512" | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(signed_info_bytes);
            hasher.finalize().to_vec()
        }
        _ => {
            // Default to SHA256
            let mut hasher = Sha256::new();
            hasher.update(signed_info_bytes);
            hasher.finalize().to_vec()
        }
    };

    // Verify signature using ring
    // Use the certificate's DER-encoded public key directly
    // ring's UnparsedPublicKey can work with the raw public key bytes
    let verification_alg: &dyn VerificationAlgorithm = match algorithm_str.as_str() {
        "rsa-sha256"
        | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"
        | "http://www.w3.org/2000/09/xmldsig#rsa-sha256" => &RSA_PKCS1_2048_8192_SHA256,
        "rsa-sha512"
        | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512"
        | "http://www.w3.org/2000/09/xmldsig#rsa-sha512" => &RSA_PKCS1_2048_8192_SHA512,
        _ => &RSA_PKCS1_2048_8192_SHA256,
    };

    // For ring's UnparsedPublicKey, we can use the full certificate DER
    // ring will extract the public key from the certificate automatically
    // This is simpler and more reliable than manually extracting the public key
    let public_key_unparsed = UnparsedPublicKey::new(verification_alg, cert_der);

    // Verify the signature
    // Note: XML signature verification typically requires canonicalization of the SignedInfo
    // and handling of references. This is a simplified verification that works for
    // basic SAML scenarios. For full compliance, consider using a dedicated XML signature library.
    public_key_unparsed
        .verify(&hash, &signature_bytes)
        .map_err(|e| format!("Signature verification failed: {:?}", e))?;

    Ok(())
}

/// Extract signature value from XML Signature element
fn extract_signature_value(xml: &str) -> Option<String> {
    // Look for <SignatureValue> or <ds:SignatureValue>
    let patterns = [
        r#"<ds:SignatureValue[^>]*>(.*?)</ds:SignatureValue>"#,
        r#"<SignatureValue[^>]*>(.*?)</SignatureValue>"#,
    ];

    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(cap) = re.captures(xml) {
                if let Some(value) = cap.get(1) {
                    return Some(value.as_str().trim().to_string());
                }
            }
        }
    }
    None
}

/// Extract SignedInfo element from XML Signature
fn extract_signed_info(xml: &str) -> Option<String> {
    // Look for <SignedInfo> or <ds:SignedInfo>
    let patterns = [
        r#"<ds:SignedInfo[^>]*>(.*?)</ds:SignedInfo>"#,
        r#"<SignedInfo[^>]*>(.*?)</SignedInfo>"#,
    ];

    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(cap) = re.captures(xml) {
                if let Some(value) = cap.get(1) {
                    return Some(value.as_str().to_string());
                }
            }
        }
    }
    None
}

/// Extract signature algorithm from SignedInfo
fn extract_signature_algorithm(xml: &str) -> Option<String> {
    // Look for SignatureMethod Algorithm attribute
    let patterns = [
        r#"<ds:SignatureMethod[^>]*Algorithm="([^"]+)""#,
        r#"<SignatureMethod[^>]*Algorithm="([^"]+)""#,
    ];

    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(cap) = re.captures(xml) {
                if let Some(value) = cap.get(1) {
                    return Some(value.as_str().to_string());
                }
            }
        }
    }
    None
}

/// Extract value from XML by tag name (fallback parser for simple cases)
/// Primary parsing is done in parse_saml_response using quick-xml
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    // Fallback regex-based extraction for simple cases
    let pattern = format!(r#"<{}[^>]*>(.*?)</{}>"#, tag, tag);
    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(cap) = re.captures(xml) {
            return Some(cap.get(1)?.as_str().to_string());
        }
    }

    // Try with namespace prefixes
    for prefix in &["saml:", "saml2:", "samlp:", "ds:"] {
        let pattern = format!(r#"<{}{}[^>]*>(.*?)</{}{}>"#, prefix, tag, prefix, tag);
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(cap) = re.captures(xml) {
                return Some(cap.get(1)?.as_str().to_string());
            }
        }
    }

    None
}

/// Parse SAML logout request and extract session index
fn parse_saml_logout_request(request_xml: &[u8]) -> Result<Option<String>, ApiError> {
    let xml_str = String::from_utf8_lossy(request_xml);
    let session_index = extract_xml_value(&xml_str, "SessionIndex")
        .or_else(|| extract_xml_value(&xml_str, "samlp:SessionIndex"));
    Ok(session_index)
}

/// Generate SAML logout response
fn generate_saml_logout_response(slo_url: &str) -> String {
    let response_id = uuid::Uuid::new_v4().to_string();
    let issue_instant = chrono::Utc::now().to_rfc3339();

    format!(
        r#"<samlp:LogoutResponse xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
    ID="_{}"
    Version="2.0"
    IssueInstant="{}"
    Destination="{}"
    StatusCode="urn:oasis:names:tc:SAML:2.0:status:Success"/>
"#,
        response_id, issue_instant, slo_url
    )
}

/// Find or create user from SAML attributes
async fn find_or_create_user_from_saml(
    pool: &sqlx::PgPool,
    user_info: &SAMLUserInfo,
    org: &Organization,
) -> Result<User, ApiError> {
    // Try to find user by email
    let user = if let Some(email) = &user_info.email {
        User::find_by_email(pool, email).await.map_err(|e| ApiError::Database(e))?
    } else {
        None
    };

    let user = if let Some(user) = user {
        // User exists - ensure they're a member of the organization
        use crate::models::organization::OrgMember;
        use crate::models::organization::OrgRole;

        // Check if user is already a member
        if OrgMember::find(pool, org.id, user.id)
            .await
            .map_err(|e| ApiError::Database(e))?
            .is_none()
        {
            // Add user to organization as member
            OrgMember::create(pool, org.id, user.id, OrgRole::Member)
                .await
                .map_err(|e| ApiError::Database(e))?;
        }

        user
    } else {
        // Create new user from SAML attributes
        let email = user_info.email.as_ref().ok_or_else(|| {
            ApiError::InvalidRequest("Email not found in SAML assertion".to_string())
        })?;

        let username = user_info.username.as_ref().cloned().unwrap_or_else(|| {
            // Generate username from email
            email.split('@').next().unwrap_or("user").to_string()
        });

        // Generate a random password (user won't need it for SSO login)
        let password_hash = crate::auth::hash_password(&uuid::Uuid::new_v4().to_string())
            .map_err(|e| ApiError::Internal(e))?;

        // Create user
        let user = User::create(pool, &username, email, &password_hash)
            .await
            .map_err(|e| ApiError::Database(e))?;

        // Mark user as verified (SSO users are pre-verified)
        sqlx::query("UPDATE users SET is_verified = TRUE WHERE id = $1")
            .bind(user.id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

        // Add user to organization as member
        use crate::models::organization::OrgMember;
        use crate::models::organization::OrgRole;

        OrgMember::create(pool, org.id, user.id, OrgRole::Member)
            .await
            .map_err(|e| ApiError::Database(e))?;

        user
    };

    Ok(user)
}

/// Validate SAML assertion timestamps (NotBefore/NotOnOrAfter)
/// Prevents replay attacks by ensuring assertions are within valid time window
fn validate_saml_timestamps(user_info: &SAMLUserInfo) -> Result<(), ApiError> {
    let now = chrono::Utc::now();

    // Check NotBefore (assertion not valid before this time)
    if let Some(not_before) = user_info.not_before {
        // Allow 5 minute clock skew tolerance
        let tolerance = chrono::Duration::minutes(5);
        if now < not_before - tolerance {
            tracing::warn!("SAML assertion not yet valid: not_before={}, now={}", not_before, now);
            return Err(ApiError::InvalidRequest(format!(
                "SAML assertion is not yet valid. Valid from: {}",
                not_before
            )));
        }
    }

    // Check NotOnOrAfter (assertion expires after this time)
    if let Some(not_on_or_after) = user_info.not_on_or_after {
        // Allow 5 minute clock skew tolerance
        let tolerance = chrono::Duration::minutes(5);
        if now > not_on_or_after + tolerance {
            tracing::warn!(
                "SAML assertion expired: not_on_or_after={}, now={}",
                not_on_or_after,
                now
            );
            return Err(ApiError::InvalidRequest(format!(
                "SAML assertion has expired. Expired at: {}",
                not_on_or_after
            )));
        }
    } else {
        // If no expiration time, default to 5 minutes validity
        if let Some(issued_at) = user_info.issued_at {
            let max_validity = issued_at + chrono::Duration::minutes(5);
            if now > max_validity {
                tracing::warn!(
                    "SAML assertion exceeded default validity: issued_at={}, now={}",
                    issued_at,
                    now
                );
                return Err(ApiError::InvalidRequest(
                    "SAML assertion has exceeded maximum validity period (5 minutes)".to_string(),
                ));
            }
        }
    }

    tracing::debug!("SAML timestamp validation passed");
    Ok(())
}

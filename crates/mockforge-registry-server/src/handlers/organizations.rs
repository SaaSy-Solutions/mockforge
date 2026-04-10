//! Organization management handlers
//!
//! Provides endpoints for listing organizations, viewing details, and managing members

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{AuditEventType, OrgRole, Plan},
    AppState,
};

/// Create a new organization
pub async fn create_organization(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<CreateOrganizationRequest>,
) -> ApiResult<Json<OrganizationResponse>> {
    // Validate input
    if request.name.is_empty() {
        return Err(ApiError::InvalidRequest("Organization name is required".to_string()));
    }

    if request.slug.is_empty() {
        return Err(ApiError::InvalidRequest("Organization slug is required".to_string()));
    }

    // Validate slug format (alphanumeric, hyphens, underscores only)
    if !request.slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ApiError::InvalidRequest(
            "Organization slug must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        ));
    }

    // Check if slug is already taken
    if state.store.find_organization_by_slug(&request.slug).await?.is_some() {
        return Err(ApiError::InvalidRequest("Organization slug is already taken".to_string()));
    }

    // Create organization (defaults to Free plan)
    let plan = request.plan.as_deref().unwrap_or("free");
    let plan_enum = match plan {
        "free" => Plan::Free,
        "pro" => Plan::Pro,
        "team" => Plan::Team,
        _ => Plan::Free,
    };

    let org = state
        .store
        .create_organization(&request.name, &request.slug, user_id, plan_enum)
        .await?;

    Ok(Json(OrganizationResponse {
        id: org.id,
        name: org.name.clone(),
        slug: org.slug.clone(),
        plan: org.plan().to_string(),
        owner_id: org.owner_id,
        created_at: org.created_at,
    }))
}

/// List all organizations for the authenticated user
pub async fn list_organizations(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<Vec<OrganizationResponse>>> {
    // Get all organizations where user is owner or member
    let orgs = state.store.list_organizations_by_user(user_id).await?;

    // Convert to response format
    let responses: Vec<OrganizationResponse> = orgs
        .into_iter()
        .map(|org| OrganizationResponse {
            id: org.id,
            name: org.name.clone(),
            slug: org.slug.clone(),
            plan: org.plan().to_string(),
            owner_id: org.owner_id,
            created_at: org.created_at,
        })
        .collect();

    Ok(Json(responses))
}

/// Get organization details
pub async fn get_organization(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<OrganizationResponse>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has access (owner or member)
    if org.owner_id != user_id {
        let member = state.store.find_org_member(org_id, user_id).await?;
        if member.is_none() {
            return Err(ApiError::InvalidRequest(
                "You don't have access to this organization".to_string(),
            ));
        }
    }

    Ok(Json(OrganizationResponse {
        id: org.id,
        name: org.name.clone(),
        slug: org.slug.clone(),
        plan: org.plan().to_string(),
        owner_id: org.owner_id,
        created_at: org.created_at,
    }))
}

/// Get organization members
pub async fn get_organization_members(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MemberResponse>>> {
    // Verify user has access to this organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if org.owner_id != user_id {
        let member = state.store.find_org_member(org_id, user_id).await?;
        if member.is_none() {
            return Err(ApiError::InvalidRequest(
                "You don't have access to this organization".to_string(),
            ));
        }
    }

    // Store org fields before moving org
    let org_owner_id = org.owner_id;
    let org_created_at = org.created_at;
    let org_id_for_members = org.id;

    // Get all members (including owner)
    let members = state.store.list_org_members(org_id).await?;

    // Get user details for each member
    let mut member_responses = Vec::new();

    // Add owner as a member (if not already in members list)
    let owner_user = state
        .store
        .find_user_by_id(org_owner_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Owner user not found".to_string()))?;

    let owner_in_members = members.iter().any(|m| m.user_id == org_owner_id);
    if !owner_in_members {
        member_responses.push(MemberResponse {
            id: org_id_for_members, // Use org id as placeholder
            user_id: org_owner_id,
            username: owner_user.username,
            email: owner_user.email,
            role: "owner".to_string(),
            avatar_url: None, // User model doesn't have avatar_url field
            created_at: org_created_at,
        });
    }

    // Add other members
    for member in members {
        let user = state
            .store
            .find_user_by_id(member.user_id)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

        member_responses.push(MemberResponse {
            id: member.id,
            user_id: member.user_id,
            username: user.username,
            email: user.email,
            role: member.role().to_string(),
            avatar_url: None, // User model doesn't have avatar_url field
            created_at: member.created_at,
        });
    }

    Ok(Json(member_responses))
}

/// Add a member to an organization
pub async fn add_organization_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddMemberRequest>,
) -> ApiResult<Json<MemberResponse>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = state.store.find_org_member(org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Find user to add by email or user_id
    let target_user = if let Some(email) = &request.email {
        state
            .store
            .find_user_by_email(email)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?
    } else if let Some(user_id_param) = request.user_id {
        state
            .store
            .find_user_by_id(user_id_param)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?
    } else {
        return Err(ApiError::InvalidRequest(
            "Either email or user_id must be provided".to_string(),
        ));
    };

    // Check if user is already a member
    if org.owner_id == target_user.id {
        return Err(ApiError::InvalidRequest(
            "User is already the owner of this organization".to_string(),
        ));
    }

    if state.store.find_org_member(org_id, target_user.id).await?.is_some() {
        return Err(ApiError::InvalidRequest(
            "User is already a member of this organization".to_string(),
        ));
    }

    // Determine role (default to member)
    let role = match request.role.as_deref() {
        Some("admin") => OrgRole::Admin,
        Some("member") | None => OrgRole::Member,
        _ => {
            return Err(ApiError::InvalidRequest(
                "Invalid role. Must be 'admin' or 'member'".to_string(),
            ))
        }
    };

    // Add member
    let member = state.store.create_org_member(org_id, target_user.id, role).await?;

    // Record audit event
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::MemberAdded,
            format!(
                "Added member {} ({}) with role {}",
                target_user.username, target_user.email, role
            ),
            None,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    Ok(Json(MemberResponse {
        id: member.id,
        user_id: target_user.id,
        username: target_user.username,
        email: target_user.email,
        role: role.to_string(),
        avatar_url: None,
        created_at: member.created_at,
    }))
}

/// Remove a member from an organization
pub async fn remove_organization_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, member_user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = state.store.find_org_member(org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Prevent removing the owner
    if org.owner_id == member_user_id {
        return Err(ApiError::InvalidRequest("Cannot remove the organization owner".to_string()));
    }

    // Check if member exists
    let _member = state
        .store
        .find_org_member(org_id, member_user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Member not found".to_string()))?;

    // Get user details for audit log
    let target_user = state
        .store
        .find_user_by_id(member_user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Remove member
    state.store.delete_org_member(org_id, member_user_id).await?;

    // Record audit event
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::MemberRemoved,
            format!("Removed member {} ({})", target_user.username, target_user.email),
            None,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    Ok(Json(
        serde_json::json!({"success": true, "message": "Member removed successfully"}),
    ))
}

/// Update a member's role in an organization
pub async fn update_organization_member_role(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, member_user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> ApiResult<Json<MemberResponse>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = state.store.find_org_member(org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Prevent changing owner's role
    if org.owner_id == member_user_id {
        return Err(ApiError::InvalidRequest(
            "Cannot change the organization owner's role".to_string(),
        ));
    }

    // Check if member exists
    let member = state
        .store
        .find_org_member(org_id, member_user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Member not found".to_string()))?;

    // Parse new role
    let new_role = match request.role.as_str() {
        "admin" => OrgRole::Admin,
        "member" => OrgRole::Member,
        _ => {
            return Err(ApiError::InvalidRequest(
                "Invalid role. Must be 'admin' or 'member'".to_string(),
            ))
        }
    };

    // Update role
    state.store.update_org_member_role(org_id, member_user_id, new_role).await?;

    // Get user details
    let target_user = state
        .store
        .find_user_by_id(member_user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Record audit event
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::MemberRoleChanged,
            format!(
                "Changed role of {} ({}) from {} to {}",
                target_user.username,
                target_user.email,
                member.role(),
                new_role
            ),
            None,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    // Get updated member
    let updated_member = state
        .store
        .find_org_member(org_id, member_user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Member not found".to_string()))?;

    Ok(Json(MemberResponse {
        id: updated_member.id,
        user_id: target_user.id,
        username: target_user.username,
        email: target_user.email,
        role: new_role.to_string(),
        avatar_url: None,
        created_at: updated_member.created_at,
    }))
}

/// Update organization details
pub async fn update_organization(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateOrganizationRequest>,
) -> ApiResult<Json<OrganizationResponse>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner
    if org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // Update name if provided
    if let Some(name) = &request.name {
        if name.is_empty() {
            return Err(ApiError::InvalidRequest("Organization name cannot be empty".to_string()));
        }
        state.store.update_organization_name(org_id, name).await?;
    }

    // Update slug if provided
    if let Some(slug) = &request.slug {
        if slug.is_empty() {
            return Err(ApiError::InvalidRequest("Organization slug cannot be empty".to_string()));
        }

        // Validate slug format
        if !slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ApiError::InvalidRequest(
                "Organization slug must contain only alphanumeric characters, hyphens, and underscores".to_string(),
            ));
        }

        // Check if slug is already taken (by another org)
        if let Ok(Some(existing_org)) = state.store.find_organization_by_slug(slug).await {
            if existing_org.id != org_id {
                return Err(ApiError::InvalidRequest(
                    "Organization slug is already taken".to_string(),
                ));
            }
        }

        state.store.update_organization_slug(org_id, slug).await?;
    }

    // Update plan if provided
    if let Some(plan_str) = &request.plan {
        let new_plan = match plan_str.as_str() {
            "free" => Plan::Free,
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            _ => {
                return Err(ApiError::InvalidRequest(
                    "Invalid plan. Must be 'free', 'pro', or 'team'".to_string(),
                ))
            }
        };

        state.store.update_organization_plan(org_id, new_plan).await?;

        // Record audit event for plan change
        let ip_address = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let user_agent =
            headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

        state
            .store
            .record_audit_event(
                org_id,
                Some(user_id),
                AuditEventType::OrgPlanChanged,
                format!("Changed plan from {} to {}", org.plan(), new_plan),
                None,
                ip_address.as_deref(),
                user_agent.as_deref(),
            )
            .await;
    }

    // Get updated organization
    let updated_org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    Ok(Json(OrganizationResponse {
        id: updated_org.id,
        name: updated_org.name.clone(),
        slug: updated_org.slug.clone(),
        plan: updated_org.plan().to_string(),
        owner_id: updated_org.owner_id,
        created_at: updated_org.created_at,
    }))
}

/// Delete an organization
pub async fn delete_organization(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Get organization
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner
    if org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // Check if organization has active subscriptions (prevent deletion if billing is active)
    // This is a safety check - in production, you might want to handle subscription cancellation first
    if state.store.organization_has_active_subscription(org_id).await? {
        return Err(ApiError::InvalidRequest(
            "Cannot delete organization with active subscription. Please cancel subscription first.".to_string()
        ));
    }

    // Record audit event before deletion
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::OrgDeleted,
            format!("Deleted organization: {}", org.name),
            None,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    // Delete organization (cascade will handle related data)
    state.store.delete_organization(org_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "message": "Organization deleted successfully"}),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub plan: Option<String>, // Optional: defaults to "free"
}

#[derive(Debug, Serialize)]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub owner_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
    pub role: Option<String>, // "admin" or "member", defaults to "member"
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String, // "admin" or "member"
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub plan: Option<String>, // "free", "pro", or "team"
}

// ---------------------------------------------------------------------------
// Phase B endpoints — added for cloud-mode registry-admin unification
// ---------------------------------------------------------------------------

/// GET /api/v1/organizations/slug/:slug
pub async fn get_organization_by_slug(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(slug): Path<String>,
) -> ApiResult<Json<OrganizationResponse>> {
    let org = state
        .store
        .find_organization_by_slug(&slug)
        .await?
        .ok_or(ApiError::OrganizationNotFound)?;

    Ok(Json(OrganizationResponse {
        id: org.id,
        name: org.name,
        slug: org.slug,
        plan: org.plan,
        owner_id: org.owner_id,
        created_at: org.created_at,
    }))
}

/// GET /api/v1/organizations/:org_id/quota
pub async fn get_organization_quota(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let setting = state.store.get_org_setting(org_id, "quota").await?;
    let value = setting.map(|s| s.setting_value).unwrap_or_else(|| serde_json::json!({}));
    Ok(Json(serde_json::json!({ "org_id": org_id, "quota": value })))
}

/// PUT /api/v1/organizations/:org_id/quota
pub async fn set_organization_quota(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Json(quota): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    if !quota.is_object() {
        return Err(ApiError::InvalidRequest("quota body must be a JSON object".to_string()));
    }
    let updated = state.store.set_org_setting(org_id, "quota", quota).await?;
    Ok(Json(serde_json::json!({
        "org_id": org_id,
        "quota": updated.setting_value,
        "updated_at": updated.updated_at,
    })))
}

// ---------------------------------------------------------------------------
// Invitation flow — reuses org_settings under invite:{nonce} keys,
// matching the OSS admin pattern from registry_admin.rs.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct InvitePayload {
    kind: String,
    org_id: Uuid,
    email: String,
    role: String,
    nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role: Option<String>,
}

/// POST /api/v1/organizations/:org_id/invitations
pub async fn create_invitation(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateInvitationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !req.email.contains('@') {
        return Err(ApiError::InvalidRequest("email looks invalid".to_string()));
    }
    let role = req.role.as_deref().unwrap_or("member").to_string();
    // Validate the role
    match role.as_str() {
        "owner" | "admin" | "member" => {}
        _ => {
            return Err(ApiError::InvalidRequest(format!(
                "unknown role '{}' (expected owner/admin/member)",
                role
            )));
        }
    }

    state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or(ApiError::OrganizationNotFound)?;

    use base64::Engine;
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let payload = InvitePayload {
        kind: "invite".into(),
        org_id,
        email: req.email.clone(),
        role: role.clone(),
        nonce: nonce.clone(),
    };
    let payload_str = serde_json::to_string(&payload)
        .map_err(|e| ApiError::InvalidRequest(format!("encode: {}", e)))?;

    let setting_key = format!("invite:{}", nonce);
    state
        .store
        .set_org_setting(org_id, &setting_key, serde_json::to_value(&payload).unwrap())
        .await?;

    Ok(Json(serde_json::json!({
        "token": payload_str,
        "org_id": org_id,
        "email": req.email,
        "role": role,
    })))
}

/// GET /api/v1/invitations/:token
pub async fn get_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let payload: InvitePayload = serde_json::from_str(&token)
        .map_err(|_| ApiError::InvalidRequest("invalid invitation token".to_string()))?;
    if payload.kind != "invite" {
        return Err(ApiError::InvalidRequest("token is not an invitation".to_string()));
    }

    let setting_key = format!("invite:{}", payload.nonce);
    let setting =
        state
            .store
            .get_org_setting(payload.org_id, &setting_key)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidRequest("invitation not found or already accepted".to_string())
            })?;
    let stored: InvitePayload = serde_json::from_value(setting.setting_value)
        .map_err(|e| ApiError::InvalidRequest(format!("decode: {}", e)))?;
    if stored.nonce != payload.nonce || stored.email != payload.email {
        return Err(ApiError::InvalidRequest(
            "invitation not found or already accepted".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "org_id": stored.org_id,
        "email": stored.email,
        "role": stored.role,
    })))
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub username: String,
    pub password: String,
}

/// POST /api/v1/invitations/:token/accept
pub async fn accept_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<AcceptInvitationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.username.trim().is_empty() {
        return Err(ApiError::InvalidRequest("username must not be empty".to_string()));
    }
    if req.password.len() < 8 {
        return Err(ApiError::InvalidRequest("password must be at least 8 characters".to_string()));
    }

    let payload: InvitePayload = serde_json::from_str(&token)
        .map_err(|_| ApiError::InvalidRequest("invalid invitation token".to_string()))?;
    let setting_key = format!("invite:{}", payload.nonce);

    let setting =
        state
            .store
            .get_org_setting(payload.org_id, &setting_key)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidRequest("invitation not found or already accepted".to_string())
            })?;
    let stored: InvitePayload = serde_json::from_value(setting.setting_value)
        .map_err(|e| ApiError::InvalidRequest(format!("decode: {}", e)))?;
    if stored.nonce != payload.nonce || stored.email != payload.email {
        return Err(ApiError::InvalidRequest(
            "invitation not found or already accepted".to_string(),
        ));
    }

    // Check for duplicate username/email
    if state.store.find_user_by_username(&req.username).await?.is_some() {
        return Err(ApiError::InvalidRequest("username already taken".to_string()));
    }
    if state.store.find_user_by_email(&stored.email).await?.is_some() {
        return Err(ApiError::InvalidRequest("a user with this email already exists".to_string()));
    }

    let hash = crate::auth::hash_password(&req.password).map_err(ApiError::Internal)?;
    let created = state.store.create_user(&req.username, &stored.email, &hash).await?;
    state.store.mark_user_verified(created.id).await?;
    let user = state
        .store
        .find_user_by_id(created.id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("user vanished mid-accept".to_string()))?;

    let role = match stored.role.as_str() {
        "owner" => OrgRole::Owner,
        "admin" => OrgRole::Admin,
        _ => OrgRole::Member,
    };
    state.store.create_org_member(stored.org_id, user.id, role).await?;

    state.store.delete_org_setting(payload.org_id, &setting_key).await?;

    let jwt = crate::auth::create_access_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "is_verified": user.is_verified,
        },
        "org_id": stored.org_id,
        "role": stored.role,
        "token": jwt,
    })))
}

/// GET /api/v1/users/email/:email (admin only)
pub async fn find_user_by_email_admin(
    State(state): State<AppState>,
    AuthUser(caller_id): AuthUser,
    Path(email): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let caller = state
        .store
        .find_user_by_id(caller_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("caller not found".to_string()))?;
    if !caller.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    let user = state
        .store
        .find_user_by_email(&email)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest(format!("user '{}' not found", email)))?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
        "is_admin": user.is_admin,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })))
}

/// GET /api/v1/users/username/:username (admin only)
pub async fn find_user_by_username_admin(
    State(state): State<AppState>,
    AuthUser(caller_id): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let caller = state
        .store
        .find_user_by_id(caller_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("caller not found".to_string()))?;
    if !caller.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    let user = state
        .store
        .find_user_by_username(&username)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest(format!("user '{}' not found", username)))?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
        "is_admin": user.is_admin,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })))
}

/// POST /api/v1/users/:id/verify (admin only)
pub async fn mark_user_verified_admin(
    State(state): State<AppState>,
    AuthUser(caller_id): AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let caller = state
        .store
        .find_user_by_id(caller_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("caller not found".to_string()))?;
    if !caller.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    state.store.mark_user_verified(user_id).await?;
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("user not found".to_string()))?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
    })))
}

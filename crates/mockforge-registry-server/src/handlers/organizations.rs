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
    models::{AuditEventType, Organization, OrgMember, OrgRole, Plan, User, record_audit_event},
    AppState,
};

/// Create a new organization
pub async fn create_organization(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<CreateOrganizationRequest>,
) -> ApiResult<Json<OrganizationResponse>> {
    let pool = state.db.pool();

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
            "Organization slug must contain only alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }

    // Check if slug is already taken
    if Organization::find_by_slug(pool, &request.slug)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
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

    let org = Organization::create(pool, &request.name, &request.slug, user_id, plan_enum)
        .await
        .map_err(|e| ApiError::Database(e))?;

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
    let pool = state.db.pool();

    // Get all organizations where user is owner or member
    let orgs = Organization::find_by_user(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

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
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has access (owner or member)
    if org.owner_id != user_id {
        let member = OrgMember::find(pool, org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;
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
    let pool = state.db.pool();

    // Verify user has access to this organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if org.owner_id != user_id {
        let member = OrgMember::find(pool, org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;
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
    let members = OrgMember::find_by_org(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get user details for each member
    let mut member_responses = Vec::new();

    // Add owner as a member (if not already in members list)
    let owner_user = User::find_by_id(pool, org_owner_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
        let user = User::find_by_id(pool, member.user_id)
            .await
            .map_err(|e| ApiError::Database(e))?
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
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_id, user_id).await {
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
        User::find_by_email(pool, email)
            .await
            .map_err(|e| ApiError::Database(e))?
            .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?
    } else if let Some(user_id_param) = request.user_id {
        User::find_by_id(pool, user_id_param)
            .await
            .map_err(|e| ApiError::Database(e))?
            .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?
    } else {
        return Err(ApiError::InvalidRequest("Either email or user_id must be provided".to_string()));
    };

    // Check if user is already a member
    if org.owner_id == target_user.id {
        return Err(ApiError::InvalidRequest("User is already the owner of this organization".to_string()));
    }

    if OrgMember::find(pool, org_id, target_user.id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        return Err(ApiError::InvalidRequest("User is already a member of this organization".to_string()));
    }

    // Determine role (default to member)
    let role = match request.role.as_deref() {
        Some("admin") => OrgRole::Admin,
        Some("member") | None => OrgRole::Member,
        _ => return Err(ApiError::InvalidRequest("Invalid role. Must be 'admin' or 'member'".to_string())),
    };

    // Add member
    let member = OrgMember::create(pool, org_id, target_user.id, role)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit event
    let ip_address = headers.get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    record_audit_event(
        pool,
        org_id,
        Some(user_id),
        AuditEventType::MemberAdded,
        format!("Added member {} ({}) with role {}", target_user.username, target_user.email, role.to_string()),
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
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_id, user_id).await {
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
    let member = OrgMember::find(pool, org_id, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Member not found".to_string()))?;

    // Get user details for audit log
    let target_user = User::find_by_id(pool, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Remove member
    OrgMember::delete(pool, org_id, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit event
    let ip_address = headers.get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    record_audit_event(
        pool,
        org_id,
        Some(user_id),
        AuditEventType::MemberRemoved,
        format!("Removed member {} ({})", target_user.username, target_user.email),
        None,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true, "message": "Member removed successfully"})))
}

/// Update a member's role in an organization
pub async fn update_organization_member_role(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, member_user_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> ApiResult<Json<MemberResponse>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has permission (owner or admin)
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_id, user_id).await {
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
        return Err(ApiError::InvalidRequest("Cannot change the organization owner's role".to_string()));
    }

    // Check if member exists
    let member = OrgMember::find(pool, org_id, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Member not found".to_string()))?;

    // Parse new role
    let new_role = match request.role.as_str() {
        "admin" => OrgRole::Admin,
        "member" => OrgRole::Member,
        _ => return Err(ApiError::InvalidRequest("Invalid role. Must be 'admin' or 'member'".to_string())),
    };

    // Update role
    OrgMember::update_role(pool, org_id, member_user_id, new_role)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get user details
    let target_user = User::find_by_id(pool, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Record audit event
    let ip_address = headers.get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    record_audit_event(
        pool,
        org_id,
        Some(user_id),
        AuditEventType::MemberRoleChanged,
        format!("Changed role of {} ({}) from {} to {}", target_user.username, target_user.email, member.role().to_string(), new_role.to_string()),
        None,
        ip_address.as_deref(),
        user_agent.as_deref(),
    )
    .await;

    // Get updated member
    let updated_member = OrgMember::find(pool, org_id, member_user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
        sqlx::query("UPDATE organizations SET name = $1, updated_at = NOW() WHERE id = $2")
            .bind(name)
            .bind(org_id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;
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
        if let Ok(Some(existing_org)) = Organization::find_by_slug(pool, slug).await {
            if existing_org.id != org_id {
                return Err(ApiError::InvalidRequest("Organization slug is already taken".to_string()));
            }
        }

        sqlx::query("UPDATE organizations SET slug = $1, updated_at = NOW() WHERE id = $2")
            .bind(slug)
            .bind(org_id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;
    }

    // Update plan if provided
    if let Some(plan_str) = &request.plan {
        let new_plan = match plan_str.as_str() {
            "free" => Plan::Free,
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            _ => return Err(ApiError::InvalidRequest("Invalid plan. Must be 'free', 'pro', or 'team'".to_string())),
        };

        Organization::update_plan(pool, org_id, new_plan)
            .await
            .map_err(|e| ApiError::Database(e))?;

        // Record audit event for plan change
        let ip_address = headers.get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let user_agent = headers.get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        record_audit_event(
            pool,
            org_id,
            Some(user_id),
            AuditEventType::OrgPlanChanged,
            format!("Changed plan from {} to {}", org.plan().to_string(), new_plan.to_string()),
            None,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;
    }

    // Get updated organization
    let updated_org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner
    if org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // Check if organization has active subscriptions (prevent deletion if billing is active)
    // This is a safety check - in production, you might want to handle subscription cancellation first
    let has_active_subscription: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM subscriptions WHERE org_id = $1 AND status IN ('active', 'trialing'))"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    if has_active_subscription.0 {
        return Err(ApiError::InvalidRequest(
            "Cannot delete organization with active subscription. Please cancel subscription first.".to_string()
        ));
    }

    // Record audit event before deletion
    let ip_address = headers.get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    record_audit_event(
        pool,
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
    sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Organization deleted successfully"})))
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

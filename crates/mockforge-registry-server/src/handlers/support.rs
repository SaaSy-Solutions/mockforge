//! Support contact handlers

use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::OptionalAuthUser,
    models::Organization,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ContactRequest {
    pub subject: String,
    pub category: String, // "technical" | "billing" | "feature" | "bug" | "other"
    pub message: String,
    pub priority: String, // "low" | "normal" | "high" | "urgent"
}

#[derive(Debug, Serialize)]
pub struct ContactResponse {
    pub success: bool,
    pub message: String,
    pub ticket_id: Option<String>, // For future ticket system integration
}

/// Submit a support contact request
/// Can be called by authenticated or anonymous users
pub async fn submit_contact(
    State(state): State<AppState>,
    headers: HeaderMap,
    OptionalAuthUser(user_id): OptionalAuthUser,
    Json(request): Json<ContactRequest>,
) -> ApiResult<Json<ContactResponse>> {
    let pool = state.db.pool();

    // Validate input
    if request.subject.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Subject is required".to_string()));
    }

    if request.message.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Message is required".to_string()));
    }

    // Get user info if authenticated
    let (user_email, username, org_name, plan) = if let Some(user_id) = user_id {
        use crate::models::User;
        if let Some(user) =
            User::find_by_id(pool, user_id).await.map_err(|e| ApiError::Database(e))?
        {
            // Try to get org context for plan info
            let orgs = Organization::find_by_user(pool, user_id)
                .await
                .map_err(|e| ApiError::Database(e))?;
            let org = orgs.first();

            (
                Some(user.email.clone()),
                Some(user.username.clone()),
                org.map(|o| o.name.clone()),
                org.map(|o| o.plan().to_string()),
            )
        } else {
            (None, None, None, None)
        }
    } else {
        (None, None, None, None)
    };

    // Generate ticket ID (simple format for now)
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let ticket_suffix = uuid_str.split('-').next().unwrap_or(&uuid_str[..8]);
    let ticket_id = format!("SUP-{}", ticket_suffix);

    // Send email to support team
    let email_service = match EmailService::from_env() {
        Ok(svc) => svc,
        Err(e) => {
            tracing::warn!("Failed to create email service: {}", e);
            // Still return success - we don't want to fail the support request
            // just because email isn't configured
            return Ok(Json(ContactResponse {
                success: true,
                message: "Support request received. We'll respond within your plan's SLA."
                    .to_string(),
                ticket_id: Some(ticket_id),
            }));
        }
    };

    // Build email content
    let user_info = if let (Some(email), Some(name)) = (user_email.as_ref(), username.as_ref()) {
        format!("User: {} ({})\n", name, email)
    } else {
        "User: Anonymous\n".to_string()
    };

    let org_info = if let (Some(org), Some(plan_str)) = (org_name.as_ref(), plan.as_ref()) {
        format!("Organization: {} ({})\n", org, plan_str)
    } else {
        String::new()
    };

    let html_body = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: #667eea; color: white; padding: 20px; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 20px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .info-box {{ background: #f8f9fa; border-left: 4px solid #667eea; padding: 15px; margin: 15px 0; }}
        .priority {{ display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 12px; font-weight: bold; }}
        .priority-urgent {{ background: #e74c3c; color: white; }}
        .priority-high {{ background: #f39c12; color: white; }}
        .priority-normal {{ background: #3498db; color: white; }}
        .priority-low {{ background: #95a5a6; color: white; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Support Request: {}</h1>
        <p>Ticket ID: {}</p>
    </div>
    <div class="content">
        <div class="info-box">
            <p><strong>Category:</strong> {}</p>
            <p><strong>Priority:</strong> <span class="priority priority-{}">{}</span></p>
            <p><strong>{}</strong></p>
            <p><strong>{}</strong></p>
        </div>
        <h2>Message</h2>
        <div style="white-space: pre-wrap; background: #f8f9fa; padding: 15px; border-radius: 4px; margin: 15px 0;">
{}
        </div>
        <p style="font-size: 12px; color: #666; margin-top: 20px;">
            This support request was submitted through the MockForge Cloud support form.
        </p>
    </div>
</body>
</html>
"#,
        request.subject,
        ticket_id,
        request.category,
        request.priority,
        request.priority,
        user_info.trim(),
        org_info.trim(),
        request.message
    );

    let text_body = format!(
        r#"
Support Request: {}
Ticket ID: {}

Category: {}
Priority: {}

{}{}
Message:
{}

---
This support request was submitted through the MockForge Cloud support form.
"#,
        request.subject,
        ticket_id,
        request.category,
        request.priority,
        user_info,
        org_info,
        request.message
    );

    // Send to support email
    let support_email =
        std::env::var("SUPPORT_EMAIL").unwrap_or_else(|_| "support@mockforge.dev".to_string());

    let email_msg = crate::email::EmailMessage {
        to: support_email,
        subject: format!("[{}] {}", ticket_id, request.subject),
        html_body,
        text_body,
    };

    // Send email (non-blocking)
    let ticket_id_clone = ticket_id.clone();
    let subject_clone = request.subject.clone();
    let user_email_clone = user_email.clone();
    let username_clone = username.clone();
    tokio::spawn(async move {
        if let Err(e) = email_service.send(email_msg).await {
            tracing::warn!("Failed to send support request email: {}", e);
        }

        // Send confirmation email to user if authenticated
        if let (Some(email), Some(name)) = (user_email_clone, username_clone) {
            let confirmation_email = EmailService::generate_support_confirmation(
                &name,
                &email,
                &ticket_id_clone,
                &subject_clone,
            );
            if let Err(e) = email_service.send(confirmation_email).await {
                tracing::warn!("Failed to send support confirmation email: {}", e);
            }
        }
    });

    tracing::info!(
        "Support request submitted: ticket_id={}, category={}, priority={}",
        ticket_id,
        request.category,
        request.priority
    );

    Ok(Json(ContactResponse {
        success: true,
        message: "Support request submitted successfully. We'll respond within your plan's SLA."
            .to_string(),
        ticket_id: Some(ticket_id),
    }))
}

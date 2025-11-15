//! Consent screen handlers
//!
//! This module provides endpoints for OAuth2 consent screens with
//! permissions/scopes toggles and risk simulation integration.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json, Redirect},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::risk_engine::{RiskAction, RiskEngine};
use crate::handlers::oauth2_server::{AuthorizationCodeInfo, OAuth2ServerState};

/// Consent request parameters
#[derive(Debug, Deserialize)]
pub struct ConsentRequest {
    /// Client ID
    pub client_id: String,
    /// Scopes (space-separated)
    pub scope: Option<String>,
    /// State parameter
    pub state: Option<String>,
    /// Authorization code (if consent already given)
    pub code: Option<String>,
}

/// Consent decision request
#[derive(Debug, Deserialize)]
pub struct ConsentDecisionRequest {
    /// Client ID
    pub client_id: String,
    /// State parameter
    pub state: Option<String>,
    /// Whether consent was approved
    pub approved: bool,
    /// Approved scopes
    pub scopes: Vec<String>,
}

/// Consent screen state
#[derive(Clone)]
pub struct ConsentState {
    /// OAuth2 server state
    pub oauth2_state: OAuth2ServerState,
    /// Risk engine
    pub risk_engine: Arc<RiskEngine>,
}

/// Get consent screen
pub async fn get_consent_screen(
    State(state): State<ConsentState>,
    Query(params): Query<ConsentRequest>,
) -> Result<Html<String>, StatusCode> {
    // Check risk assessment
    // For mock server, use empty risk factors (can be overridden via risk simulation API)
    // In production, extract risk factors from request context (IP, device fingerprint, etc.)
    let risk_factors = HashMap::new();
    let risk_assessment = state
        .risk_engine
        .assess_risk("user-default", &risk_factors)
        .await;

    // If risk is too high, block or require additional verification
    if risk_assessment.recommended_action == RiskAction::Block {
        return Ok(Html(blocked_login_html()));
    }

    // Parse scopes
    let scopes = params
        .scope
        .as_ref()
        .map(|s| s.split(' ').map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    // Generate consent screen HTML
    let html = generate_consent_screen_html(&params.client_id, &scopes, params.state.as_deref());
    Ok(Html(html))
}

/// Submit consent decision
pub async fn submit_consent(
    State(state): State<ConsentState>,
    Json(request): Json<ConsentDecisionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !request.approved {
        return Ok(Json(serde_json::json!({
            "approved": false,
            "message": "Consent denied"
        })));
    }

    // Store consent decision and redirect back to OAuth2 flow
    // In a full implementation, this would store consent and redirect to authorization endpoint
    Ok(Json(serde_json::json!({
        "approved": true,
        "scopes": request.scopes,
        "message": "Consent approved"
    })))
}

/// Generate consent screen HTML
fn generate_consent_screen_html(client_id: &str, scopes: &[String], state: Option<&str>) -> String {
    let scope_items = scopes
        .iter()
        .map(|scope| {
            let description = get_scope_description(scope);
            format!(
                r#"
                <div class="scope-item">
                    <label class="scope-toggle">
                        <input type="checkbox" name="scope" value="{}" checked>
                        <span class="scope-name">{}</span>
                    </label>
                    <p class="scope-description">{}</p>
                </div>
                "#,
                scope, scope, description
            )
        })
        .collect::<String>();

    let state_param = state
        .map(|s| format!(r#"<input type="hidden" name="state" value="{}">"#, s))
        .unwrap_or_default();

    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Authorize Application - MockForge</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .consent-container {{
            background: white;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            max-width: 500px;
            width: 100%;
            padding: 40px;
            animation: slideUp 0.3s ease-out;
        }}
        @keyframes slideUp {{
            from {{
                opacity: 0;
                transform: translateY(20px);
            }}
            to {{
                opacity: 1;
                transform: translateY(0);
            }}
        }}
        .app-icon {{
            width: 64px;
            height: 64px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 16px;
            margin: 0 auto 24px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 32px;
            color: white;
        }}
        h1 {{
            font-size: 24px;
            font-weight: 600;
            text-align: center;
            margin-bottom: 8px;
            color: #1a1a1a;
        }}
        .client-id {{
            text-align: center;
            color: #666;
            font-size: 14px;
            margin-bottom: 32px;
        }}
        .permissions-title {{
            font-size: 16px;
            font-weight: 600;
            margin-bottom: 16px;
            color: #1a1a1a;
        }}
        .scope-item {{
            padding: 16px;
            border: 1px solid #e5e5e5;
            border-radius: 8px;
            margin-bottom: 12px;
            transition: all 0.2s;
        }}
        .scope-item:hover {{
            border-color: #667eea;
            background: #f8f9ff;
        }}
        .scope-toggle {{
            display: flex;
            align-items: center;
            cursor: pointer;
            margin-bottom: 8px;
        }}
        .scope-toggle input[type="checkbox"] {{
            width: 20px;
            height: 20px;
            margin-right: 12px;
            cursor: pointer;
            accent-color: #667eea;
        }}
        .scope-name {{
            font-weight: 500;
            color: #1a1a1a;
        }}
        .scope-description {{
            font-size: 13px;
            color: #666;
            margin-left: 32px;
            line-height: 1.5;
        }}
        .buttons {{
            display: flex;
            gap: 12px;
            margin-top: 32px;
        }}
        button {{
            flex: 1;
            padding: 14px 24px;
            border: none;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
        }}
        .btn-approve {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        .btn-approve:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
        }}
        .btn-deny {{
            background: #f5f5f5;
            color: #666;
        }}
        .btn-deny:hover {{
            background: #e5e5e5;
        }}
        .privacy-link {{
            text-align: center;
            margin-top: 24px;
            font-size: 13px;
            color: #666;
        }}
        .privacy-link a {{
            color: #667eea;
            text-decoration: none;
        }}
        .privacy-link a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="consent-container">
        <div class="app-icon">🔐</div>
        <h1>Authorize Application</h1>
        <p class="client-id">{}</p>

        <p class="permissions-title">This application is requesting the following permissions:</p>

        <form id="consent-form" method="POST" action="/consent/decision">
            <input type="hidden" name="client_id" value="{}">
            {}
            <div class="scopes">
                {}
            </div>

            <div class="buttons">
                <button type="submit" class="btn-approve" name="approved" value="true">
                    Approve
                </button>
                <button type="button" class="btn-deny" onclick="denyConsent()">
                    Deny
                </button>
            </div>
        </form>

        <div class="privacy-link">
            By approving, you agree to our <a href="/privacy">Privacy Policy</a> and <a href="/terms">Terms of Service</a>.
        </div>
    </div>

    <script>
        function denyConsent() {{
            document.getElementById('consent-form').innerHTML += '<input type="hidden" name="approved" value="false">';
            document.getElementById('consent-form').submit();
        }}
    </script>
</body>
</html>
        "#,
        client_id, client_id, state_param, scope_items
    )
}

/// Get scope description
fn get_scope_description(scope: &str) -> &str {
    match scope {
        "openid" => "Access your basic profile information",
        "profile" => "Access your profile information including name and picture",
        "email" => "Access your email address",
        "address" => "Access your address information",
        "phone" => "Access your phone number",
        "offline_access" => "Access your information while you're offline",
        _ => "Access to this permission",
    }
}

/// Generate blocked login HTML
fn blocked_login_html() -> String {
    r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login Blocked - MockForge</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .blocked-container {
            background: white;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            max-width: 400px;
            width: 100%;
            padding: 40px;
            text-align: center;
        }
        .icon {
            font-size: 64px;
            margin-bottom: 24px;
        }
        h1 {
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 16px;
            color: #1a1a1a;
        }
        p {
            color: #666;
            line-height: 1.6;
            margin-bottom: 24px;
        }
    </style>
</head>
<body>
    <div class="blocked-container">
        <div class="icon">🚫</div>
        <h1>Login Blocked</h1>
        <p>Your login attempt has been blocked due to security concerns. Please contact support if you believe this is an error.</p>
    </div>
</body>
</html>
    "#.to_string()
}

/// Create consent router
pub fn consent_router(state: ConsentState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/consent", get(get_consent_screen))
        .route("/consent/decision", post(submit_consent))
        .with_state(state)
}

//! OAuth authentication handlers
//!
//! Supports GitHub and Google OAuth 2.0 authentication flows

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::{
    auth::{create_token_pair, REFRESH_TOKEN_EXPIRY_DAYS},
    error::ApiError,
    models::{Organization, User},
    AppState,
};
use chrono::{Duration, Utc};

/// OAuth provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    GitHub,
    Google,
}

impl OAuthProvider {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "github" => Some(OAuthProvider::GitHub),
            "google" => Some(OAuthProvider::Google),
            _ => None,
        }
    }

    fn to_string(&self) -> &'static str {
        match self {
            OAuthProvider::GitHub => "github",
            OAuthProvider::Google => "google",
        }
    }
}

/// Initiate OAuth flow - redirects user to OAuth provider
pub async fn oauth_authorize(
    State(state): State<AppState>,
    Path(provider_str): Path<String>,
) -> Result<Redirect, ApiError> {
    let provider = OAuthProvider::from_str(&provider_str)
        .ok_or_else(|| ApiError::InvalidRequest("Invalid OAuth provider".to_string()))?;

    // Get OAuth client for provider
    let client = get_oauth_client(&state, provider).ok_or_else(|| {
        ApiError::InvalidRequest("OAuth not configured for this provider".to_string())
    })?;

    // Build authorization URL with CSRF state
    let (auth_url, csrf_state) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(get_default_scopes(provider))
        .url();

    // Store CSRF state token in Redis for verification in callback
    // This prevents CSRF attacks by ensuring the callback request originated from our authorization request
    // State tokens expire after 15 minutes (900 seconds) to limit exposure window
    let state_token = csrf_state.secret();
    let state_key = format!("oauth:state:{}", state_token);

    if let Some(redis) = &state.redis {
        // Store state token with provider info and timestamp for verification
        // Format: "provider:timestamp" (e.g., "github:1234567890")
        let state_value = format!("{}:{}", provider.to_string(), chrono::Utc::now().timestamp());
        redis
            .set_with_expiry(&state_key, &state_value, 900) // 15 minutes expiration
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to store OAuth state: {}", e)))?;
    } else {
        // If Redis is not available, we can't securely store state
        // In production, Redis should be required for OAuth to prevent CSRF attacks
        return Err(ApiError::Internal(anyhow::anyhow!(
            "OAuth requires Redis for CSRF protection. Please configure REDIS_URL."
        )));
    }

    // Redirect to OAuth provider
    Ok(Redirect::to(auth_url.as_str()))
}

/// OAuth callback handler - receives authorization code from provider
pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider_str): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<Response, ApiError> {
    let provider = OAuthProvider::from_str(&provider_str)
        .ok_or_else(|| ApiError::InvalidRequest("Invalid OAuth provider".to_string()))?;

    // Verify state token (CSRF protection)
    // This ensures the callback request originated from our authorization request
    // and prevents CSRF attacks where an attacker could trick a user into authorizing
    // their account on the attacker's behalf
    if let Some(state_token) = &params.state {
        let state_key = format!("oauth:state:{}", state_token);

        if let Some(redis) = &state.redis {
            // Retrieve and verify state token from Redis
            let stored_state = redis.get(&state_key).await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to verify OAuth state: {}", e))
            })?;

            match stored_state {
                Some(value) => {
                    // Verify provider matches the one in the stored state
                    // This prevents cross-provider CSRF attacks
                    let expected_prefix = format!("{}:", provider.to_string());
                    if !value.starts_with(&expected_prefix) {
                        return Err(ApiError::InvalidRequest(
                            "OAuth state token provider mismatch. Possible CSRF attack."
                                .to_string(),
                        ));
                    }

                    // Delete state token after verification (one-time use)
                    // This prevents replay attacks
                    let _ = redis.delete(&state_key).await;
                }
                None => {
                    // State token not found - either expired, already used, or invalid
                    return Err(ApiError::InvalidRequest(
                        "Invalid or expired OAuth state token. Please try again.".to_string(),
                    ));
                }
            }
        } else {
            // Redis is required for OAuth state verification
            // Without Redis, we cannot securely verify the state token
            return Err(ApiError::Internal(anyhow::anyhow!(
                "OAuth requires Redis for CSRF protection. Please configure REDIS_URL."
            )));
        }
    } else {
        // State parameter is required for CSRF protection
        return Err(ApiError::InvalidRequest(
            "Missing OAuth state parameter. This is required for security.".to_string(),
        ));
    }

    // Get OAuth client
    let client = get_oauth_client(&state, provider).ok_or_else(|| {
        ApiError::InvalidRequest("OAuth not configured for this provider".to_string())
    })?;

    // Exchange authorization code for access token
    let code = AuthorizationCode::new(params.code.clone());
    let token_result =
        client.exchange_code(code).request_async(async_http_client).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("OAuth token exchange failed: {}", e))
        })?;

    let access_token = token_result.access_token().secret();

    // Fetch user info from provider
    let user_info = fetch_user_info(provider, access_token).await?;

    let pool = state.db.pool();

    // Find or create user
    let user = match provider {
        OAuthProvider::GitHub => {
            // Check if user exists by GitHub ID
            let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE github_id = $1")
                .bind(&user_info.provider_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;

            if let Some(user) = existing {
                user
            } else {
                // Check if email already exists (link accounts)
                let email_user = User::find_by_email(pool, &user_info.email)
                    .await
                    .map_err(|e| ApiError::Database(e))?;

                if let Some(user) = email_user {
                    // Link GitHub account to existing user
                    sqlx::query("UPDATE users SET github_id = $1, auth_provider = 'github', avatar_url = $2 WHERE id = $3")
                        .bind(&user_info.provider_id)
                        .bind(user_info.avatar_url.as_deref())
                        .bind(user.id)
                        .execute(pool)
                        .await
                        .map_err(|e| ApiError::Database(e))?;
                    user
                } else {
                    // Create new user
                    create_oauth_user(pool, &user_info, provider).await?
                }
            }
        }
        OAuthProvider::Google => {
            // Check if user exists by Google ID
            let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE google_id = $1")
                .bind(&user_info.provider_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;

            if let Some(user) = existing {
                user
            } else {
                // Check if email already exists (link accounts)
                let email_user = User::find_by_email(pool, &user_info.email)
                    .await
                    .map_err(|e| ApiError::Database(e))?;

                if let Some(user) = email_user {
                    // Link Google account to existing user
                    sqlx::query("UPDATE users SET google_id = $1, auth_provider = 'google', avatar_url = $2 WHERE id = $3")
                        .bind(&user_info.provider_id)
                        .bind(user_info.avatar_url.as_deref())
                        .bind(user.id)
                        .execute(pool)
                        .await
                        .map_err(|e| ApiError::Database(e))?;
                    user
                } else {
                    // Create new user
                    create_oauth_user(pool, &user_info, provider).await?
                }
            }
        }
    };

    // Create personal organization if it doesn't exist
    let _personal_org = Organization::get_or_create_personal_org(pool, user.id, &user.username)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Send welcome email for new OAuth users (non-blocking)
    // Check if this is a new user by checking if they were just created
    let is_new_user = user.created_at > chrono::Utc::now() - chrono::Duration::minutes(1);
    if is_new_user {
        if let Ok(email_service) = crate::email::EmailService::from_env() {
            let welcome_email =
                crate::email::EmailService::generate_welcome_email(&user.username, &user.email);
            tokio::spawn(async move {
                if let Err(e) = email_service.send(welcome_email).await {
                    tracing::warn!("Failed to send welcome email: {}", e);
                }
            });
        }
    }

    // Generate token pair (access + refresh)
    let (token_pair, jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    // Store refresh token JTI in database for revocation support
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state.db.store_refresh_token_jti(&jti, user.id, expires_at).await.map_err(|e| {
        tracing::warn!("Failed to store refresh token JTI: {}", e);
        ApiError::Internal(e)
    })?;

    // Redirect to frontend with tokens (or return JSON)
    // For now, return JSON - frontend can handle redirect
    let response = serde_json::json!({
        "access_token": token_pair.access_token,
        "refresh_token": token_pair.refresh_token,
        "access_token_expires_at": token_pair.access_token_expires_at,
        "refresh_token_expires_at": token_pair.refresh_token_expires_at,
        "user_id": user.id.to_string(),
        "username": user.username,
        "email": user.email,
        "provider": provider.to_string(),
    });

    Ok(Json(response).into_response())
}

/// OAuth user info from provider
#[derive(Debug, Clone)]
struct OAuthUserInfo {
    provider_id: String,
    username: String,
    email: String,
    avatar_url: Option<String>,
}

/// Fetch user info from OAuth provider
async fn fetch_user_info(
    provider: OAuthProvider,
    access_token: &str,
) -> Result<OAuthUserInfo, ApiError> {
    match provider {
        OAuthProvider::GitHub => {
            let client = reqwest::Client::new();
            let response = client
                .get("https://api.github.com/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "MockForge")
                .send()
                .await
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("Failed to fetch GitHub user: {}", e))
                })?;

            let user: serde_json::Value = response.json().await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to parse GitHub response: {}", e))
            })?;

            Ok(OAuthUserInfo {
                provider_id: user["id"]
                    .as_u64()
                    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Invalid GitHub user ID")))?
                    .to_string(),
                username: user["login"]
                    .as_str()
                    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Invalid GitHub username")))?
                    .to_string(),
                email: user["email"].as_str().unwrap_or("").to_string(),
                avatar_url: user["avatar_url"].as_str().map(|s| s.to_string()),
            })
        }
        OAuthProvider::Google => {
            let client = reqwest::Client::new();
            let response = client
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("Failed to fetch Google user: {}", e))
                })?;

            let user: serde_json::Value = response.json().await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to parse Google response: {}", e))
            })?;

            Ok(OAuthUserInfo {
                provider_id: user["id"]
                    .as_str()
                    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Invalid Google user ID")))?
                    .to_string(),
                username: user["email"]
                    .as_str()
                    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Invalid Google email")))?
                    .split('@')
                    .next()
                    .unwrap_or("user")
                    .to_string(),
                email: user["email"]
                    .as_str()
                    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Invalid Google email")))?
                    .to_string(),
                avatar_url: user["picture"].as_str().map(|s| s.to_string()),
            })
        }
    }
}

/// Create new user from OAuth info
async fn create_oauth_user(
    pool: &sqlx::PgPool,
    user_info: &OAuthUserInfo,
    provider: OAuthProvider,
) -> Result<User, ApiError> {
    // Generate a placeholder password hash (OAuth users don't need passwords)
    let password_hash = bcrypt::hash("oauth_user_no_password", bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to hash password: {}", e)))?;

    // Ensure username is unique
    let mut username = user_info.username.clone();
    let mut counter = 0;
    while User::find_by_username(pool, &username)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        counter += 1;
        username = format!("{}{}", user_info.username, counter);
    }

    // Create user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email, password_hash, auth_provider, github_id, google_id, avatar_url, is_verified)
        VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)
        RETURNING *
        "#,
    )
    .bind(&username)
    .bind(&user_info.email)
    .bind(&password_hash)
    .bind(provider.to_string())
    .bind(if provider == OAuthProvider::GitHub {
        Some(&user_info.provider_id)
    } else {
        None
    })
    .bind(if provider == OAuthProvider::Google {
        Some(&user_info.provider_id)
    } else {
        None
    })
    .bind(user_info.avatar_url.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(user)
}

/// Get OAuth client for provider
fn get_oauth_client(state: &AppState, provider: OAuthProvider) -> Option<BasicClient> {
    match provider {
        OAuthProvider::GitHub => {
            let client_id = state.config.oauth_github_client_id.as_ref()?;
            let client_secret = state.config.oauth_github_client_secret.as_ref()?;

            Some(
                BasicClient::new(
                    ClientId::new(client_id.clone()),
                    Some(ClientSecret::new(client_secret.clone())),
                    AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).ok()?,
                    Some(
                        TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                            .ok()?,
                    ),
                )
                .set_redirect_uri(
                    RedirectUrl::new(format!(
                        "{}/api/v1/auth/oauth/github/callback",
                        state.config.app_base_url
                    ))
                    .ok()?,
                ),
            )
        }
        OAuthProvider::Google => {
            let client_id = state.config.oauth_google_client_id.as_ref()?;
            let client_secret = state.config.oauth_google_client_secret.as_ref()?;

            Some(
                BasicClient::new(
                    ClientId::new(client_id.clone()),
                    Some(ClientSecret::new(client_secret.clone())),
                    AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                        .ok()?,
                    Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).ok()?),
                )
                .set_redirect_uri(
                    RedirectUrl::new(format!(
                        "{}/api/v1/auth/oauth/google/callback",
                        state.config.app_base_url
                    ))
                    .ok()?,
                ),
            )
        }
    }
}

/// Get default scopes for provider
fn get_default_scopes(provider: OAuthProvider) -> Scope {
    match provider {
        OAuthProvider::GitHub => Scope::new("user:email".to_string()),
        OAuthProvider::Google => Scope::new("openid email profile".to_string()),
    }
}

/// Generate state token for CSRF protection
fn generate_state_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: Option<String>,
}

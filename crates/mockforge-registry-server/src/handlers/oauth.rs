//! OAuth authentication handlers
//!
//! Supports GitHub and Google OAuth 2.0 authentication flows

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, HttpRequest,
    HttpResponse, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::{
    auth::{create_token_pair, REFRESH_TOKEN_EXPIRY_DAYS},
    error::ApiError,
    models::User,
    AppState,
};
use chrono::{Duration, Utc};

/// Token-exchange transport for oauth2 4.x on the workspace reqwest client
/// (oauth2's bundled client is reqwest 0.11 on rustls 0.21). Redirects stay
/// disabled, as in oauth2's own client, so a token endpoint can't bounce the
/// authorization code to another host.
async fn token_http_client(request: HttpRequest) -> Result<HttpResponse, std::io::Error> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(std::io::Error::other)?;
    let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
        .map_err(std::io::Error::other)?;
    let mut builder = client.request(method, request.url.as_str()).body(request.body);
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    let response = builder.send().await.map_err(std::io::Error::other)?;
    let status_code = oauth2::http::StatusCode::from_u16(response.status().as_u16())
        .map_err(std::io::Error::other)?;
    let mut headers = oauth2::http::HeaderMap::new();
    for (name, value) in response.headers() {
        headers.append(
            oauth2::http::header::HeaderName::from_bytes(name.as_str().as_bytes())
                .map_err(std::io::Error::other)?,
            oauth2::http::HeaderValue::from_bytes(value.as_bytes())
                .map_err(std::io::Error::other)?,
        );
    }
    let body = response.bytes().await.map_err(std::io::Error::other)?.to_vec();
    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

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

    fn as_str(self) -> &'static str {
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
        let state_value = format!("{}:{}", provider.as_str(), Utc::now().timestamp());
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
                    let expected_prefix = format!("{}:", provider.as_str());
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
        client.exchange_code(code).request_async(token_http_client).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("OAuth token exchange failed: {}", e))
        })?;

    let access_token = token_result.access_token().secret();

    // Fetch user info from provider
    let user_info = fetch_user_info(provider, access_token).await?;

    // Find or create user
    let user = match provider {
        OAuthProvider::GitHub => {
            if let Some(user) = state.store.find_user_by_github_id(&user_info.provider_id).await? {
                user
            } else if let Some(user) = state.store.find_user_by_email(&user_info.email).await? {
                if !user.is_verified {
                    return Err(ApiError::ValidationFailed("Verify the existing account email using its original sign-in before connecting an OAuth provider".into()));
                }
                // Link GitHub account to existing user
                state
                    .store
                    .link_user_github_account(
                        user.id,
                        &user_info.provider_id,
                        user_info.avatar_url.as_deref(),
                    )
                    .await?;
                user
            } else {
                create_oauth_user(&state, &user_info, provider).await?
            }
        }
        OAuthProvider::Google => {
            if let Some(user) = state.store.find_user_by_google_id(&user_info.provider_id).await? {
                user
            } else if let Some(user) = state.store.find_user_by_email(&user_info.email).await? {
                if !user.is_verified {
                    return Err(ApiError::ValidationFailed("Verify the existing account email using its original sign-in before connecting an OAuth provider".into()));
                }
                // Link Google account to existing user
                state
                    .store
                    .link_user_google_account(
                        user.id,
                        &user_info.provider_id,
                        user_info.avatar_url.as_deref(),
                    )
                    .await?;
                user
            } else {
                create_oauth_user(&state, &user_info, provider).await?
            }
        }
    };

    // Create personal organization if it doesn't exist
    let _personal_org = state.store.get_or_create_personal_org(user.id, &user.username).await?;

    // Send welcome email for new OAuth users (non-blocking). Gated on the
    // user's `email_notifications` preference — new accounts default to
    // opted-in so a brand-new signup still gets the welcome.
    let is_new_user = user.created_at > Utc::now() - Duration::minutes(1);
    if is_new_user && user.email_notifications {
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
        .map_err(ApiError::Internal)?;

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
        "provider": provider.as_str(),
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

mod verified_identity;
use verified_identity::fetch_user_info;

/// Create new user from OAuth info
async fn create_oauth_user(
    state: &AppState,
    user_info: &OAuthUserInfo,
    provider: OAuthProvider,
) -> Result<User, ApiError> {
    // Generate a random password hash (OAuth users authenticate via provider, not password)
    let random_password: String = uuid::Uuid::new_v4().to_string();
    let password_hash = bcrypt::hash(&random_password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to hash password: {}", e)))?;

    // Ensure username is unique
    let mut username = user_info.username.clone();
    let mut counter = 0;
    while state.store.find_user_by_username(&username).await?.is_some() {
        counter += 1;
        username = format!("{}{}", user_info.username, counter);
    }

    // Create user
    let github_id = if provider == OAuthProvider::GitHub {
        Some(user_info.provider_id.as_str())
    } else {
        None
    };
    let google_id = if provider == OAuthProvider::Google {
        Some(user_info.provider_id.as_str())
    } else {
        None
    };

    let user = state
        .store
        .create_oauth_user(
            &username,
            &user_info.email,
            &password_hash,
            provider.as_str(),
            github_id,
            google_id,
            user_info.avatar_url.as_deref(),
        )
        .await?;

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
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    hex::encode(bytes)
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: Option<String>,
}

#[cfg(test)]
mod token_exchange_tests {
    use super::*;
    use axum::{http::HeaderMap, response::Redirect, routing::post, Router};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    async fn serve(app: Router) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (url, task)
    }

    fn client(token_url: String) -> BasicClient {
        BasicClient::new(
            ClientId::new("fixture-client".into()),
            Some(ClientSecret::new("fixture-secret".into())),
            AuthUrl::new("https://provider.invalid/authorize".into()).unwrap(),
            Some(TokenUrl::new(token_url).unwrap()),
        )
    }

    #[tokio::test]
    async fn exchanges_code_over_workspace_client() {
        let app = Router::new().route(
            "/token",
            post(|headers: HeaderMap, body: String| async move {
                assert!(headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("Basic ")));
                assert!(body.contains("grant_type=authorization_code"), "{body}");
                assert!(body.contains("code=fixture-code"), "{body}");
                (
                    [("content-type", "application/json")],
                    r#"{"access_token":"fixture-token","token_type":"bearer"}"#,
                )
            }),
        );
        let (url, task) = serve(app).await;
        let token = client(format!("{url}/token"))
            .exchange_code(AuthorizationCode::new("fixture-code".into()))
            .request_async(token_http_client)
            .await
            .unwrap();
        assert_eq!(token.access_token().secret(), "fixture-token");
        task.abort();
    }

    #[tokio::test]
    async fn token_endpoint_redirects_are_not_followed() {
        let followed = Arc::new(AtomicBool::new(false));
        let hit = followed.clone();
        let app = Router::new()
            .route("/token", post(|| async { Redirect::temporary("/elsewhere") }))
            .route(
                "/elsewhere",
                post(move || {
                    hit.store(true, Ordering::SeqCst);
                    async {
                        (
                            [("content-type", "application/json")],
                            r#"{"access_token":"stolen","token_type":"bearer"}"#,
                        )
                    }
                }),
            );
        let (url, task) = serve(app).await;
        let result = client(format!("{url}/token"))
            .exchange_code(AuthorizationCode::new("fixture-code".into()))
            .request_async(token_http_client)
            .await;
        assert!(result.is_err());
        assert!(!followed.load(Ordering::SeqCst), "redirect must not be followed");
        task.abort();
    }
}

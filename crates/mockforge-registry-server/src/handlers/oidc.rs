//! OIDC (OpenID Connect) SSO handlers
//!
//! Implements the OIDC authorization-code flow for Team-plan organizations:
//! discovery → authorize redirect → callback → **ID-token validation**
//! (signature via discovered JWKS, plus `iss` / `aud` / `exp` / `nonce`) →
//! JIT user provisioning → short-lived redirect token.
//!
//! Mirrors the Redis CSRF-state pattern in `handlers/oauth.rs` and the
//! Team-plan gate + final redirect shape in `handlers/sso.rs` (SAML).

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
};
use serde::Deserialize;

use crate::{
    error::ApiError,
    handlers::sso::{find_or_create_sso_user, record_sso_login_audit},
    models::Plan,
    AppState,
};

/// Redis TTL for a pending OIDC authorization (CSRF state + nonce), in seconds.
/// Mirrors the 15-minute window oauth.rs uses for its CSRF state.
const OIDC_STATE_TTL_SECS: u64 = 900;

/// Value stored in Redis under `oidc:state:{state}` while the user is at the IdP.
/// Serialized as JSON so the callback can recover the nonce + org binding.
#[derive(Debug, serde::Serialize, Deserialize)]
struct PendingOidcAuth {
    nonce: String,
    org_slug: String,
}

/// Query params on the IdP callback (`?code=...&state=...`).
#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
}

/// Build the OIDC client for an organization via discovery.
///
/// Performs `.well-known/openid-configuration` discovery against the issuer,
/// then constructs a [`CoreClient`] with the configured client id/secret and
/// the callback redirect URI. The redirect URI is an **API** URL
/// (`{app_base_url}/api/v1/sso/oidc/callback/{org_slug}`) using the same base
/// the SAML ACS URL is built from in `handlers/sso.rs`.
async fn build_oidc_client(
    state: &AppState,
    config: &crate::models::SSOConfiguration,
    org_slug: &str,
) -> Result<CoreClient, ApiError> {
    let issuer = config
        .oidc_issuer_url
        .as_deref()
        .ok_or_else(|| ApiError::InvalidRequest("OIDC issuer URL not configured".to_string()))?;
    let client_id = config
        .oidc_client_id
        .as_deref()
        .ok_or_else(|| ApiError::InvalidRequest("OIDC client ID not configured".to_string()))?;
    let client_secret = config
        .oidc_client_secret
        .as_deref()
        .ok_or_else(|| ApiError::InvalidRequest("OIDC client secret not configured".to_string()))?;

    let issuer_url = IssuerUrl::new(issuer.to_string())
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid OIDC issuer URL: {}", e)))?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
        .await
        .map_err(|e| {
            tracing::warn!("OIDC discovery failed for issuer {}: {}", issuer, e);
            ApiError::InvalidRequest("OIDC provider discovery failed".to_string())
        })?;

    let redirect_uri = RedirectUrl::new(format!(
        "{}/api/v1/sso/oidc/callback/{}",
        state.config.app_base_url, org_slug
    ))
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Invalid OIDC redirect URI: {}", e)))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string())),
    )
    .set_redirect_uri(redirect_uri);

    Ok(client)
}

/// Load the org's SSO config and assert it's an enabled, fully-configured OIDC
/// setup on a Team plan. Mirrors the SAML login gate.
async fn load_oidc_config(
    state: &AppState,
    org: &crate::models::Organization,
) -> Result<crate::models::SSOConfiguration, ApiError> {
    // Team-plan gate (identical to SAML login / SSO-config handlers).
    if org.plan() != Plan::Team {
        return Err(ApiError::InvalidRequest("SSO is only available for Team plans".to_string()));
    }

    let config = state.store.find_sso_config_by_org(org.id).await?.ok_or_else(|| {
        ApiError::InvalidRequest("SSO not configured for this organization".to_string())
    })?;

    if !config.enabled {
        return Err(ApiError::InvalidRequest(
            "SSO is not enabled for this organization".to_string(),
        ));
    }

    if config.provider != "oidc" {
        return Err(ApiError::InvalidRequest(
            "This organization is not configured for OIDC SSO".to_string(),
        ));
    }

    if config.oidc_issuer_url.is_none()
        || config.oidc_client_id.is_none()
        || config.oidc_client_secret.is_none()
    {
        return Err(ApiError::InvalidRequest(
            "OIDC configuration is incomplete (issuer URL, client ID, and client secret are required)"
                .to_string(),
        ));
    }

    Ok(config)
}

/// `GET /api/v1/sso/oidc/login/{org_slug}` — initiate the OIDC login flow.
///
/// Discovers the IdP, builds the authorization URL with a random CSRF state +
/// nonce, persists `{nonce, org_slug}` in Redis keyed by the state value, and
/// redirects the browser to the IdP.
pub async fn oidc_login(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
) -> Result<Response, ApiError> {
    let org = state
        .store
        .find_organization_by_slug(&org_slug)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let config = load_oidc_config(&state, &org).await?;
    let client = build_oidc_client(&state, &config, &org_slug).await?;

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    // Persist the nonce + org binding keyed by the CSRF state. Redis is
    // required here for the same reason as oauth.rs: without it we cannot
    // securely tie the callback back to this request (CSRF / nonce replay).
    let pending = PendingOidcAuth {
        nonce: nonce.secret().to_string(),
        org_slug: org_slug.clone(),
    };
    let state_key = format!("oidc:state:{}", csrf_token.secret());
    let state_value = serde_json::to_string(&pending)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to encode OIDC state: {}", e)))?;

    if let Some(redis) = &state.redis {
        redis
            .set_with_expiry(&state_key, &state_value, OIDC_STATE_TTL_SECS)
            .await
            .map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to store OIDC state: {}", e))
            })?;
    } else {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "OIDC SSO requires Redis for CSRF protection. Please configure REDIS_URL."
        )));
    }

    Ok(Redirect::to(auth_url.as_str()).into_response())
}

/// `GET /api/v1/sso/oidc/callback/{org_slug}` — complete the OIDC flow.
///
/// Verifies the CSRF state against Redis (one-time use), exchanges the code,
/// **validates the ID token** (signature + iss/aud/exp + nonce), provisions
/// the user, audits the login, and redirects to the same frontend URL shape
/// as SAML: `{app_base_url}/auth/sso/callback?token=...&org_slug=...`.
///
/// On any auth/crypto failure it redirects to `{app_base_url}/login?sso_error=...`
/// with a coarse error code — the underlying error is only ever logged, never
/// surfaced to the user.
pub async fn oidc_callback(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
    headers: HeaderMap,
    Query(params): Query<OidcCallbackParams>,
) -> Result<Response, ApiError> {
    let app_base_url = state.config.app_base_url.clone();
    let err_redirect = |code: &str| {
        Ok(Redirect::to(&format!("{}/login?sso_error={}", app_base_url, code)).into_response())
    };

    // 1. Required query params.
    let (code, csrf_state) = match (params.code, params.state) {
        (Some(c), Some(s)) => (c, s),
        _ => return err_redirect("invalid_state"),
    };

    // 2. Recover + consume the pending auth from Redis (one-time use).
    let Some(redis) = &state.redis else {
        tracing::error!("OIDC callback hit with no Redis configured");
        return err_redirect("invalid_state");
    };
    let state_key = format!("oidc:state:{}", csrf_state);
    let stored = match redis.get(&state_key).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("OIDC state lookup failed: {}", e);
            return err_redirect("invalid_state");
        }
    };
    let Some(stored) = stored else {
        return err_redirect("invalid_state");
    };
    // Consume immediately to prevent replay regardless of what follows.
    let _ = redis.delete(&state_key).await;

    let pending: PendingOidcAuth = match serde_json::from_str(&stored) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("OIDC state decode failed: {}", e);
            return err_redirect("invalid_state");
        }
    };

    // Path org_slug must match the one bound at login time.
    if pending.org_slug != org_slug {
        tracing::warn!(
            "OIDC callback org_slug mismatch: path={} state={}",
            org_slug,
            pending.org_slug
        );
        return err_redirect("invalid_state");
    }

    // 3. Reload org + config and rebuild the client.
    let org = match state.store.find_organization_by_slug(&org_slug).await? {
        Some(o) => o,
        None => return err_redirect("invalid_state"),
    };
    let config = match load_oidc_config(&state, &org).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("OIDC callback config load failed for org {}: {:?}", org_slug, e);
            return err_redirect("token_invalid");
        }
    };
    let client = match build_oidc_client(&state, &config, &org_slug).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("OIDC client build failed for org {}: {:?}", org_slug, e);
            return err_redirect("token_invalid");
        }
    };

    // 4. Exchange the authorization code for tokens.
    let token_response = match client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("OIDC code exchange failed for org {}: {}", org_slug, e);
            return err_redirect("token_invalid");
        }
    };

    // 5. Validate the ID token: signature (JWKS from discovery), iss, aud,
    //    exp, and nonce. Never leak the crypto error to the user.
    let id_token = match openidconnect::TokenResponse::id_token(&token_response) {
        Some(t) => t,
        None => {
            tracing::warn!("OIDC token response missing id_token for org {}", org_slug);
            return err_redirect("token_invalid");
        }
    };

    let nonce = Nonce::new(pending.nonce);
    let claims = match id_token.claims(&client.id_token_verifier(), &nonce) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("OIDC ID-token validation failed for org {}: {}", org_slug, e);
            return err_redirect("token_invalid");
        }
    };

    // 6. Extract identity from the validated claims.
    let (email, username) = match extract_identity(claims) {
        Ok(pair) => pair,
        Err(_) => return err_redirect("no_email"),
    };

    // 7. JIT-provision the user + issue a short-lived redirect token.
    let user = find_or_create_sso_user(&state, &email, username.as_deref(), &org).await?;

    record_sso_login_audit(&state, &org, &user, "oidc", &headers).await;

    let token = crate::auth::create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    // 8. Redirect to the app — SAME shape as the SAML ACS success path.
    let redirect_url =
        format!("{}/auth/sso/callback?token={}&org_slug={}", app_base_url, token, org_slug);
    Ok(Redirect::to(&redirect_url).into_response())
}

/// Pull `(email, username)` out of validated OIDC ID-token claims.
///
/// Email is **required** (it's the JIT-provisioning key) — absence is an
/// error. Username is best-effort: `preferred_username` then `name`, else
/// `None` (the provisioner falls back to the email local-part).
fn extract_identity(
    claims: &openidconnect::IdTokenClaims<
        openidconnect::EmptyAdditionalClaims,
        openidconnect::core::CoreGenderClaim,
    >,
) -> Result<(String, Option<String>), ApiError> {
    let email = claims
        .email()
        .map(|e| e.as_str().to_string())
        .ok_or_else(|| ApiError::InvalidRequest("OIDC ID token missing email claim".to_string()))?;

    let username = claims.preferred_username().map(|u| u.as_str().to_string()).or_else(|| {
        claims
            .name()
            .and_then(|localized| localized.get(None))
            .map(|n| n.as_str().to_string())
    });

    Ok((email, username))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openidconnect::{
        core::CoreIdTokenClaims, Audience, EmptyAdditionalClaims, EndUserUsername, IssuerUrl,
        StandardClaims, SubjectIdentifier,
    };

    fn base_claims(sub: &str) -> CoreIdTokenClaims {
        CoreIdTokenClaims::new(
            IssuerUrl::new("https://idp.example.com".to_string()).unwrap(),
            vec![Audience::new("client-123".to_string())],
            chrono::Utc::now() + chrono::Duration::hours(1),
            chrono::Utc::now(),
            StandardClaims::new(SubjectIdentifier::new(sub.to_string())),
            EmptyAdditionalClaims {},
        )
    }

    #[test]
    fn extract_identity_returns_email_and_preferred_username() {
        let claims = base_claims("user-1")
            .set_email(Some(openidconnect::EndUserEmail::new("jane@example.com".to_string())))
            .set_preferred_username(Some(EndUserUsername::new("jane".to_string())));

        let (email, username) = extract_identity(&claims).expect("email present => Ok");
        assert_eq!(email, "jane@example.com");
        assert_eq!(username.as_deref(), Some("jane"));
    }

    #[test]
    fn extract_identity_username_is_optional() {
        let claims = base_claims("user-2")
            .set_email(Some(openidconnect::EndUserEmail::new("noname@example.com".to_string())));

        let (email, username) = extract_identity(&claims).expect("email present => Ok");
        assert_eq!(email, "noname@example.com");
        assert_eq!(username, None);
    }

    #[test]
    fn extract_identity_errors_when_email_absent() {
        let claims = base_claims("user-3");
        let result = extract_identity(&claims);
        assert!(result.is_err(), "missing email claim must be an error");
    }
}

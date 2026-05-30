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
    url, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl,
    Scope,
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

    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, http_client_with_timeout)
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
    let app_base_url = state.config.app_base_url.clone();
    let err_redirect = |code: &str| {
        Ok(Redirect::to(&format!("{}/login?sso_error={}", app_base_url, code)).into_response())
    };

    let org = state
        .store
        .find_organization_by_slug(&org_slug)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let config = load_oidc_config(&state, &org).await?;

    // SECURITY (#746): fail early — SSO login is only safe once the org has
    // proven ownership of its email_domain. Without a verified domain the
    // callback would reject every assertion anyway, so refuse before the IdP
    // round-trip.
    if config.email_domain.is_none() || !config.domain_verified {
        return err_redirect("sso_domain_not_verified");
    }

    // SECURITY (#746, M4): the issuer URL is tenant-controlled and fetched
    // server-side during discovery. Block SSRF (loopback/private/link-local/
    // metadata addresses, non-HTTPS) before any outbound request.
    match config.oidc_issuer_url.as_deref() {
        Some(issuer) => match url::Url::parse(issuer) {
            Ok(parsed) => {
                if validate_public_https_url(&parsed).await.is_err() {
                    tracing::warn!("OIDC issuer URL blocked by SSRF guard for org {}", org_slug);
                    return err_redirect("issuer_blocked");
                }
            }
            Err(_) => return err_redirect("issuer_blocked"),
        },
        None => return err_redirect("issuer_blocked"),
    }

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

    // SECURITY (#746, M4): re-validate the tenant-controlled issuer URL before
    // any server-side fetch (discovery + token exchange) — SSRF guard.
    match config.oidc_issuer_url.as_deref().and_then(|i| url::Url::parse(i).ok()) {
        Some(parsed) => {
            if validate_public_https_url(&parsed).await.is_err() {
                tracing::warn!("OIDC issuer URL blocked by SSRF guard for org {}", org_slug);
                return err_redirect("issuer_blocked");
            }
        }
        None => return err_redirect("issuer_blocked"),
    }

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
        .request_async(http_client_with_timeout)
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

    // 6b. SECURITY (#746): the IdP can assert ANY email. Only trust an asserted
    // email whose domain matches the org's *verified* email_domain. This is the
    // cross-tenant takeover guard — it MUST run before provisioning.
    if let Err(e) = crate::handlers::sso::assert_email_in_verified_domain(&email, &config) {
        let code = match &e {
            ApiError::InvalidRequest(m) => m.clone(),
            _ => "domain_mismatch".to_string(),
        };
        tracing::warn!("OIDC domain-trust check failed for org {}: {:?}", org_slug, e);
        return err_redirect(&code);
    }

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

// ---------------------------------------------------------------------------
// SSRF guard + timeout HTTP client (#746 M4)
// ---------------------------------------------------------------------------

/// Reject a non-global IPv4 address (loopback / private / link-local /
/// unspecified / broadcast / CGNAT 100.64/10). `is_private` covers 10/8,
/// 172.16/12, 192.168/16; the rest std doesn't fold into one helper on stable.
fn is_blocked_ipv4(ip: std::net::Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local() // 169.254/16, incl. 169.254.169.254 metadata
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        // 100.64.0.0/10 — carrier-grade NAT, not publicly routable.
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 0x40)
}

/// Reject a non-global IPv6 address (loopback / unspecified / link-local /
/// unique-local fc00::/7). Also reject IPv4-mapped addresses whose embedded
/// v4 is blocked.
fn is_blocked_ipv6(ip: std::net::Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    let seg = ip.segments();
    // fe80::/10 link-local.
    if (seg[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // fc00::/7 unique-local.
    if (seg[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // IPv4-mapped (::ffff:0:0/96) — defer to the v4 rules.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(v4);
    }
    false
}

fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => is_blocked_ipv4(v4),
        std::net::IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

/// SECURITY (#746, M4): validate a tenant-controlled URL before fetching it
/// server-side. Requires HTTPS and a *public* host: IP literals are checked
/// directly; hostnames are resolved and rejected if **any** resolved address
/// is loopback/private/link-local/metadata/unique-local/unspecified. This is
/// the primary SSRF defense against issuer URLs pointing at internal services
/// or the cloud metadata endpoint (169.254.169.254).
async fn validate_public_https_url(url: &url::Url) -> Result<(), ApiError> {
    if url.scheme() != "https" {
        return Err(ApiError::InvalidRequest("issuer_must_be_https".into()));
    }

    let host = url
        .host_str()
        .ok_or_else(|| ApiError::InvalidRequest("issuer_no_host".into()))?;

    // IP literal? Check it directly without DNS.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(ApiError::InvalidRequest("issuer_blocked".into()));
        }
        return Ok(());
    }

    // Hostname: resolve and reject if ANY address is non-public.
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port)).await.map_err(|e| {
        tracing::warn!("OIDC issuer host resolution failed for {}: {}", host, e);
        ApiError::InvalidRequest("issuer_unresolvable".into())
    })?;

    let mut saw_any = false;
    for sa in addrs {
        saw_any = true;
        if is_blocked_ip(sa.ip()) {
            return Err(ApiError::InvalidRequest("issuer_blocked".into()));
        }
    }
    if !saw_any {
        return Err(ApiError::InvalidRequest("issuer_unresolvable".into()));
    }

    Ok(())
}

/// Shared reqwest client for OIDC discovery + token exchange, built once with a
/// 10s timeout and redirects disabled (following redirects re-opens the SSRF
/// hole the `validate_public_https_url` pre-check closes).
fn oidc_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            // Fall back to the default client if the builder somehow fails;
            // the SSRF pre-check is the primary defense regardless.
            .unwrap_or_default()
    })
}

/// Timeout-bounded async HTTP client matching openidconnect's
/// `FnOnce(HttpRequest) -> Future<Result<HttpResponse, _>>` contract, backed by
/// [`oidc_http_client`]. Mirrors `oauth2::reqwest::async_http_client` but reuses
/// a shared client carrying a 10s timeout + no-redirect policy.
///
/// openidconnect 3.5 / oauth2 4.4 model their `HttpRequest`/`HttpResponse` on
/// `http` 0.2, while reqwest 0.12 speaks `http` 1.x — so method/header/status
/// values are bridged across the two versions via their byte/string forms.
async fn http_client_with_timeout(
    request: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, openidconnect::reqwest::Error<reqwest::Error>> {
    use openidconnect::reqwest::Error;

    let client = oidc_http_client();

    // Bridge http 0.2 Method -> reqwest (http 1.x) Method via its str form.
    let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
        .map_err(|e| Error::Other(format!("invalid HTTP method: {e}")))?;

    let mut request_builder = client.request(method, request.url.as_str()).body(request.body);
    for (name, value) in &request.headers {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }
    let req = request_builder.build().map_err(Error::Reqwest)?;

    let response = client.execute(req).await.map_err(Error::Reqwest)?;

    // Bridge reqwest (http 1.x) StatusCode -> http 0.2 StatusCode.
    let status_code = openidconnect::http::StatusCode::from_u16(response.status().as_u16())
        .map_err(|e| Error::Other(format!("invalid status code: {e}")))?;

    // Bridge reqwest (http 1.x) headers -> http 0.2 HeaderMap.
    let mut headers = openidconnect::http::HeaderMap::new();
    for (name, value) in response.headers() {
        if let (Ok(n), Ok(v)) = (
            openidconnect::http::header::HeaderName::from_bytes(name.as_str().as_bytes()),
            openidconnect::http::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            headers.append(n, v);
        }
    }

    let chunks = response.bytes().await.map_err(Error::Reqwest)?;
    Ok(openidconnect::HttpResponse {
        status_code,
        headers,
        body: chunks.to_vec(),
    })
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

    // ---- SSRF guard (#746 M4) ----

    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn ssrf_blocks_loopback_private_linklocal_metadata() {
        assert!(is_blocked_ip(ip("127.0.0.1")));
        assert!(is_blocked_ip(ip("10.0.0.5")));
        assert!(is_blocked_ip(ip("172.16.4.2")));
        assert!(is_blocked_ip(ip("192.168.1.1")));
        assert!(is_blocked_ip(ip("169.254.169.254"))); // cloud metadata
        assert!(is_blocked_ip(ip("0.0.0.0")));
        assert!(is_blocked_ip(ip("100.64.0.1"))); // CGNAT
        assert!(is_blocked_ip(ip("::1"))); // ipv6 loopback
        assert!(is_blocked_ip(ip("fc00::1"))); // ULA
        assert!(is_blocked_ip(ip("fe80::1"))); // ipv6 link-local
        assert!(is_blocked_ip(ip("::ffff:127.0.0.1"))); // ipv4-mapped loopback
    }

    #[test]
    fn ssrf_allows_public_ips() {
        assert!(!is_blocked_ip(ip("8.8.8.8")));
        assert!(!is_blocked_ip(ip("1.1.1.1")));
        assert!(!is_blocked_ip(ip("2606:4700:4700::1111")));
    }

    #[tokio::test]
    async fn ssrf_rejects_non_https_scheme() {
        let u = url::Url::parse("http://idp.example.com/").unwrap();
        assert!(validate_public_https_url(&u).await.is_err());
    }

    #[tokio::test]
    async fn ssrf_rejects_https_ip_literal_loopback() {
        let u = url::Url::parse("https://127.0.0.1/").unwrap();
        assert!(validate_public_https_url(&u).await.is_err());
    }

    #[tokio::test]
    async fn ssrf_rejects_https_metadata_literal() {
        let u = url::Url::parse("https://169.254.169.254/latest/meta-data").unwrap();
        assert!(validate_public_https_url(&u).await.is_err());
    }

    #[tokio::test]
    async fn ssrf_allows_https_public_ip_literal() {
        let u = url::Url::parse("https://8.8.8.8/").unwrap();
        assert!(validate_public_https_url(&u).await.is_ok());
    }
}

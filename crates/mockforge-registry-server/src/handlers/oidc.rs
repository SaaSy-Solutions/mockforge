//! OpenID Connect (OIDC) SSO login flow (#746).
//!
//! Completes the OIDC side of SSO: discovery, the authorization-code redirect
//! with PKCE + state + nonce, code/token exchange, and ID-token validation
//! against the issuer's JWKS (RS256 signature + iss + aud + exp + nonce). On
//! success it provisions the user through the SAME domain-ownership gate as the
//! SAML path (`super::sso::provision_sso_user`, #833/#746/#778), so an OIDC IdP
//! can never create or absorb an account for an email domain the org has not
//! proven it owns.
//!
//! Security properties:
//! - Every URL the server FETCHES (issuer discovery, token endpoint, JWKS) is
//!   SSRF-guarded via `sso_domain::fetch_url_is_safe`, including the endpoints
//!   the IdP-controlled discovery document points at.
//! - Flow state (state, nonce, PKCE verifier, resolved endpoints) is carried in
//!   a short-lived HS256-signed cookie keyed by the server's jwt_secret, so it
//!   is tamper-evident and needs no shared store.
//! - `state` is compared in constant time (CSRF); `nonce` binds the ID token to
//!   this exact login attempt (replay defense).

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Redirect, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::{error::ApiError, sso_domain, AppState};

const FLOW_COOKIE: &str = "mf_oidc_flow";
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const FLOW_TTL_SECS: i64 = 600;

/// Discovered OIDC endpoints (the subset we use).
#[derive(Debug, Clone, Deserialize)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

/// A single JWK (RSA public key) from the issuer's JWKS.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    pub kty: String,
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
}

/// A JSON Web Key Set.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// ID-token claims read AFTER signature + iss/aud/exp/nonce validation.
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<serde_json::Value>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub exp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: String,
}

/// Flow state carried in the signed cookie between initiate and callback.
#[derive(Debug, Serialize, Deserialize)]
struct FlowState {
    org_id: String,
    org_slug: String,
    state: String,
    nonce: String,
    pkce_verifier: String,
    issuer: String,
    token_endpoint: String,
    jwks_uri: String,
    client_id: String,
    redirect_uri: String,
    exp: i64,
}

fn http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        // Never auto-follow redirects: a 3xx to an internal host would defeat
        // the per-URL SSRF guard.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("http client build: {e}")))
}

/// Fetch and validate the issuer's discovery document. SSRF-guards the issuer
/// and every endpoint the (IdP-controlled) document points at.
pub async fn discover(client: &reqwest::Client, issuer: &str) -> Result<OidcDiscovery, ApiError> {
    if !sso_domain::fetch_url_is_safe(issuer) {
        return Err(ApiError::InvalidRequest("OIDC issuer URL is not allowed".into()));
    }
    let url = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));
    let disc: OidcDiscovery = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("OIDC discovery request failed: {e}")))?
        .error_for_status()
        .map_err(|e| ApiError::InvalidRequest(format!("OIDC discovery HTTP error: {e}")))?
        .json()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("OIDC discovery parse error: {e}")))?;

    // OIDC Discovery 1.0: the document's `issuer` MUST equal the requested issuer
    // (ignoring a trailing slash). Prevents an issuer impersonating another.
    if disc.issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
        return Err(ApiError::InvalidRequest(
            "OIDC discovery issuer does not match the configured issuer".into(),
        ));
    }

    for ep in [
        &disc.authorization_endpoint,
        &disc.token_endpoint,
        &disc.jwks_uri,
    ] {
        if !sso_domain::fetch_url_is_safe(ep) {
            return Err(ApiError::InvalidRequest(
                "OIDC discovery returned a disallowed (internal) endpoint URL".into(),
            ));
        }
    }
    Ok(disc)
}

/// Fetch the issuer's JWKS (SSRF-guarded).
pub async fn fetch_jwks(client: &reqwest::Client, jwks_uri: &str) -> Result<Jwks, ApiError> {
    if !sso_domain::fetch_url_is_safe(jwks_uri) {
        return Err(ApiError::InvalidRequest("JWKS URL is not allowed".into()));
    }
    client
        .get(jwks_uri)
        .send()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("JWKS request failed: {e}")))?
        .error_for_status()
        .map_err(|e| ApiError::InvalidRequest(format!("JWKS HTTP error: {e}")))?
        .json()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("JWKS parse error: {e}")))
}

/// Exchange an authorization code for tokens at the token endpoint (SSRF-guarded),
/// returning the raw ID token. PKCE `code_verifier` is included.
#[allow(clippy::too_many_arguments)]
pub async fn exchange_code(
    client: &reqwest::Client,
    token_endpoint: &str,
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
    pkce_verifier: &str,
) -> Result<String, ApiError> {
    if !sso_domain::fetch_url_is_safe(token_endpoint) {
        return Err(ApiError::InvalidRequest("token endpoint URL is not allowed".into()));
    }
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code_verifier", pkce_verifier),
    ];
    let resp: TokenResponse = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("token exchange request failed: {e}")))?
        .error_for_status()
        .map_err(|e| ApiError::InvalidRequest(format!("token exchange HTTP error: {e}")))?
        .json()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("token response parse error: {e}")))?;
    Ok(resp.id_token)
}

/// Validate an OIDC ID token: RS256 signature via the JWKS, plus iss, aud, exp,
/// and nonce. Returns the validated claims. This is the security-critical step.
pub fn validate_id_token(
    id_token: &str,
    jwks: &Jwks,
    issuer: &str,
    client_id: &str,
    expected_nonce: &str,
) -> Result<IdTokenClaims, ApiError> {
    let header = decode_header(id_token)
        .map_err(|e| ApiError::InvalidRequest(format!("ID token header invalid: {e}")))?;
    if header.alg != Algorithm::RS256 {
        return Err(ApiError::InvalidRequest("ID token must be signed with RS256".into()));
    }

    let jwk = select_rsa_jwk(jwks, header.kid.as_deref())
        .ok_or_else(|| ApiError::InvalidRequest("no matching JWKS key for ID token".into()))?;
    let (n, e) = match (&jwk.n, &jwk.e) {
        (Some(n), Some(e)) => (n, e),
        _ => return Err(ApiError::InvalidRequest("JWKS key missing RSA modulus/exponent".into())),
    };
    let key = DecodingKey::from_rsa_components(n, e)
        .map_err(|e| ApiError::InvalidRequest(format!("invalid JWKS RSA key: {e}")))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[client_id]);
    validation.validate_exp = true;

    let data = decode::<IdTokenClaims>(id_token, &key, &validation)
        .map_err(|e| ApiError::InvalidRequest(format!("ID token validation failed: {e}")))?;

    // Nonce binds the token to THIS login attempt (replay / token-injection defense).
    if data.claims.nonce.as_deref() != Some(expected_nonce) {
        return Err(ApiError::InvalidRequest("ID token nonce mismatch".into()));
    }
    Ok(data.claims)
}

/// True only if `email_verified` is present and explicitly false. Different IdPs
/// encode it as a JSON bool or the string "false"; an absent claim is not
/// treated as unverified (the domain-ownership gate is the backstop there).
fn email_explicitly_unverified(value: &Option<serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(b)) => !b,
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// Select the JWK to verify with. If the token names a `kid`, require an exact
/// match (never fall back to an arbitrary key); otherwise use the sole RSA key.
fn select_rsa_jwk<'a>(jwks: &'a Jwks, kid: Option<&str>) -> Option<&'a Jwk> {
    match kid {
        Some(kid) => jwks.keys.iter().find(|k| k.kty == "RSA" && k.kid.as_deref() == Some(kid)),
        None => jwks.keys.iter().find(|k| k.kty == "RSA"),
    }
}

/// Constant-time byte comparison (for the CSRF `state` check).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 256 bits of URL-safe randomness for state / nonce.
fn random_token() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

/// PKCE (S256): a high-entropy verifier and its base64url-SHA256 challenge.
fn generate_pkce() -> (String, String) {
    let verifier = format!(
        "{}{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn encode_flow(secret: &str, flow: &FlowState) -> Result<String, ApiError> {
    jsonwebtoken::encode(
        &Header::default(),
        flow,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("flow-state encode: {e}")))
}

fn decode_flow(secret: &str, token: &str) -> Result<FlowState, ApiError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    jsonwebtoken::decode::<FlowState>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|_| ApiError::InvalidRequest("OIDC login session expired or invalid".into()))
}

fn set_flow_cookie(token: &str) -> String {
    format!(
        "{FLOW_COOKIE}={token}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={FLOW_TTL_SECS}"
    )
}

fn clear_flow_cookie() -> String {
    format!("{FLOW_COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

fn read_flow_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{FLOW_COOKIE}=");
    raw.split(';')
        .map(str::trim)
        .find_map(|kv| kv.strip_prefix(&prefix).map(str::to_string))
}

fn app_base_url() -> String {
    std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string())
}

/// Begin OIDC login: discover the issuer, then redirect the user to the IdP's
/// authorization endpoint with PKCE + state + nonce, stashing flow state in a
/// signed cookie.
pub async fn initiate_oidc_login(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
) -> Result<Response, ApiError> {
    use crate::models::{sso::SSOProvider, Plan};

    let org = state
        .store
        .find_organization_by_slug(&org_slug)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;
    // Gate on the *effective* plan (#870): a Team org whose subscription is
    // canceled/past-due/unpaid is treated as Free, so a dropped Stripe
    // webhook can't keep OIDC login working indefinitely.
    if org.plan() != Plan::Team
        || crate::handlers::entitlements::effective_plan(&state, &org).await? != Plan::Team
    {
        return Err(ApiError::InvalidRequest("SSO is only available for Team plans".into()));
    }
    let config = state
        .store
        .find_sso_config_by_org(org.id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("SSO not configured".into()))?;
    if !config.enabled {
        return Err(ApiError::InvalidRequest("SSO is not enabled".into()));
    }
    if config.provider() != SSOProvider::Oidc {
        return Err(ApiError::InvalidRequest("SSO provider is not OIDC".into()));
    }
    let issuer = config
        .oidc_issuer_url
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::InvalidRequest("OIDC issuer not configured".into()))?;
    let client_id = config
        .oidc_client_id
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::InvalidRequest("OIDC client_id not configured".into()))?;

    // SSRF guard at the boundary, before the discovery fetch.
    sso_domain::validate_issuer_url(&issuer)?;

    let client = http_client()?;
    let disc = discover(&client, &issuer).await?;

    let state_tok = random_token();
    let nonce = random_token();
    let (verifier, challenge) = generate_pkce();
    let redirect_uri = format!("{}/api/v1/sso/oidc/callback/{}", app_base_url(), org_slug);

    let flow = FlowState {
        org_id: org.id.to_string(),
        org_slug: org_slug.clone(),
        state: state_tok.clone(),
        nonce: nonce.clone(),
        pkce_verifier: verifier,
        issuer: disc.issuer.clone(),
        token_endpoint: disc.token_endpoint.clone(),
        jwks_uri: disc.jwks_uri.clone(),
        client_id: client_id.clone(),
        redirect_uri: redirect_uri.clone(),
        exp: (Utc::now() + chrono::Duration::seconds(FLOW_TTL_SECS)).timestamp(),
    };
    let cookie = encode_flow(&state.config.jwt_secret, &flow)?;

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
        disc.authorization_endpoint,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid email profile"),
        urlencoding::encode(&state_tok),
        urlencoding::encode(&nonce),
        urlencoding::encode(&challenge),
    );

    let mut resp = Redirect::to(&auth_url).into_response();
    let cookie_value = HeaderValue::from_str(&set_flow_cookie(&cookie))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("cookie header: {e}")))?;
    resp.headers_mut().insert(header::SET_COOKIE, cookie_value);
    Ok(resp)
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// OIDC redirect callback: verify state (CSRF), exchange the code, validate the
/// ID token against the JWKS, run the domain-ownership gate, provision the user,
/// mint a session, and redirect into the app.
pub async fn oidc_callback(
    State(state): State<AppState>,
    Path(org_slug): Path<String>,
    headers: HeaderMap,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Response, ApiError> {
    use crate::models::sso::SSOProvider;

    if let Some(err) = params.error.filter(|e| !e.is_empty()) {
        return Err(ApiError::InvalidRequest(format!("OIDC provider returned an error: {err}")));
    }
    let code = params
        .code
        .ok_or_else(|| ApiError::InvalidRequest("missing authorization code".into()))?;
    let state_param = params
        .state
        .ok_or_else(|| ApiError::InvalidRequest("missing state parameter".into()))?;

    let cookie = read_flow_cookie(&headers)
        .ok_or_else(|| ApiError::InvalidRequest("missing OIDC login session cookie".into()))?;
    let flow = decode_flow(&state.config.jwt_secret, &cookie)?;

    // CSRF: the IdP-returned state must equal the state we issued.
    if !constant_time_eq(state_param.as_bytes(), flow.state.as_bytes()) {
        return Err(ApiError::InvalidRequest("OIDC state mismatch (possible CSRF)".into()));
    }
    if flow.org_slug != org_slug {
        return Err(ApiError::InvalidRequest(
            "OIDC session does not match this organization".into(),
        ));
    }
    let org_id = uuid::Uuid::parse_str(&flow.org_id)
        .map_err(|_| ApiError::InvalidRequest("invalid org in OIDC session".into()))?;

    let org = state
        .store
        .find_organization_by_slug(&org_slug)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;
    if org.id != org_id {
        return Err(ApiError::InvalidRequest("OIDC session org mismatch".into()));
    }
    let config = state
        .store
        .find_sso_config_by_org(org.id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("SSO not configured".into()))?;
    if !config.enabled || config.provider() != SSOProvider::Oidc {
        return Err(ApiError::InvalidRequest("OIDC SSO is not enabled".into()));
    }
    // client_secret lives only in config (never in the cookie).
    let client_secret = config.oidc_client_secret.clone().unwrap_or_default();

    let client = http_client()?;
    let id_token = exchange_code(
        &client,
        &flow.token_endpoint,
        &code,
        &flow.redirect_uri,
        &flow.client_id,
        &client_secret,
        &flow.pkce_verifier,
    )
    .await?;
    let jwks = fetch_jwks(&client, &flow.jwks_uri).await?;
    let claims = validate_id_token(&id_token, &jwks, &flow.issuer, &flow.client_id, &flow.nonce)?;

    // Reject an email the IdP explicitly marks unverified. The domain-ownership
    // gate is the primary backstop, but an `email_verified: false` claim means
    // the IdP itself does not vouch for the address, so we must not provision it.
    if email_explicitly_unverified(&claims.email_verified) {
        return Err(ApiError::InvalidRequest("OIDC email is not verified by the IdP".into()));
    }

    let email = claims
        .email
        .clone()
        .filter(|e| !e.is_empty())
        .ok_or_else(|| ApiError::InvalidRequest("OIDC ID token has no email claim".into()))?;
    let username_hint = claims.preferred_username.clone().or_else(|| claims.name.clone());

    // Domain-ownership gate + provisioning, identical to the SAML path (#833).
    let super::sso::ProvisionedUser { user, jit_created } =
        super::sso::provision_sso_user(&state, &org, &email, username_hint.as_deref()).await?;

    // Mirror saml_acs: create an SSO session and mint a short-lived app token.
    let session_expires = Utc::now() + chrono::Duration::hours(8);
    let _session = crate::models::SSOSession::create(
        state.db.pool(),
        org.id,
        user.id,
        None,
        Some(&email),
        session_expires,
    )
    .await
    .map_err(ApiError::Database)?;

    let token = crate::auth::create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    // Audit the successful SSO login (#871). The domain-ownership gate
    // (`assert_email_in_verified_domain`, enforced inside `provision_sso_user`)
    // has already passed for this user, identical to the SAML path.
    state
        .store
        .record_audit_event(
            org.id,
            Some(user.id),
            crate::models::AuditEventType::LoginSucceeded,
            format!("SSO login via OIDC for {}", email),
            Some(serde_json::json!({
                "method": "oidc",
                "jit_created": jit_created,
                "email": email,
            })),
            None, // IdP redirect callback; no meaningful end-user IP/UA here.
            None,
        )
        .await;

    let redirect_url =
        format!("{}/auth/sso/callback?token={}&org_slug={}", app_base_url(), token, org_slug);

    let mut resp = Redirect::to(&redirect_url).into_response();
    let clear = HeaderValue::from_str(&clear_flow_cookie())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("cookie header: {e}")))?;
    resp.headers_mut().insert(header::SET_COOKIE, clear);
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::EncodingKey;

    // Throwaway RSA test key (2048-bit). NOT used anywhere but these tests.
    const TEST_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC2VVJnet1rm4wn
rbmtew8EKznxOgTLTCxYGHlkUHVpKxp5CfMqlbzRU6jxMTlTKhvq/NZw+qb2wMj2
nAwQLC31MG9HKoOu3AZeZMv7Y5Sl8QG2Y6cMFpjo7WnRh2Uk8vsy0C9VWRTsqJTT
q3vxFMbMCiMx3Kpw4mQf9k2RACkDLjL3SkxtU/BExdXI69lVjULWsvLDlvlBNpAc
uLhcxqZ1XAakgBNWzzgQFiHSKE5y+lm13k8eypCnjXznL4t5h1KkTYFkOwTtJf/k
RBQzpL2OgMlJWQ6V73qtQ+5XZrwDYaw9ccKT2lBrGS15zv3ww+hXApxeDpQ6l8Xw
GcNWJWwlAgMBAAECggEASSrl/YaNchAiZw3M0/Ps67RY9RdeMyKnLNbtZ7bt1r0o
S2gVv4IFGk8jHV6ubVQZjevWNdIvzBdCzcuC/75q1tiP3xQNcc7zc0+pl4C3dvvG
vyUwNKagx9/1tdJKYVBsQ1DNncc4oVtpFaPcAbtfpyNuSiUN9Gy01yqkp8pTquVh
GoNtd+QR2dLo6TdNi8eY5wSBPnm5UTidKp5x/niMK7bRcAqMLcNsa0sl1jPjNsIP
gktfHli4Ebo3lEVPmOFxOPy2pMDg5uOeVJ6rXC1cWxT8Nna8HzofyPR7xWCsZhUj
KHQiJ33IlU9uSxu7PUI22KUZzyfz7tP2qPv/Sg9SRQKBgQD4DV2kw8D1TcIddgBO
2EFM6oSfNuUT2tyd0S4CMqz4uXdBceIJGFzTPogh8oTZeXaEI6s0LlCTnFBIrMP6
SPsTc4nJFADRifKNupzOecK1rBQztl5V0xSagaWOprpVFVQsGFo565lxCYed3d1v
qUvcjDwKz6B7YW9Sdl8twx9HTwKBgQC8LOZ+THOSD3MhcUJwylJ3V7h2RY6/dNYm
tieaA5Eo6vfoAjWSAQIlmP5PwkNqR7yGEX68bESfGD2/A4ouueqobr4s0U7AxPnd
rznrHx8Wm6czaPUR90B4tGayX7WuUq4GbC2TbUs9IWGOQrW3WRBgLejMo/lkiysk
iBSViOn4SwKBgFgXFwxuYFY9ORSRVWaqsfYIyvRn4E5+yR5arQYmzPq/krRxJx6n
wj9a06mKoNdCpW4j5KbxU7g4KOLGSArYZCHyRBpeujOv0621ef5xi05NQBdlSncc
MRL1u7+/Qij5HB1UwKYVHzbfdYQAyKTg8InwW1pTheCLJ6eXVhHAW5lNAoGBAK82
J4/l455WYF79NF4NJOgWd504ewft5BC7fvg65ghxcE9I71R5N+SGJhVhzp/BF9rF
o3oSXXq9eZDH3PxRBBu8sbrNUUTQo880fvtcSPgmCnMmATqvPAqn/w+LaoFcXsmA
JJenJm1PDaUGnGiRt1u2o5MYAvkJVCx5wKDTkPctAoGBALmd6dGutwK69GAlkJTa
NM0NtyMGQAoLIXurbECjBksLdf+hQGIJcZkjMeHnkneP2JqjB7efpfvNAHrZ7L4M
pc8kGe5L8vP15KTflqbCciQCQrPX+hQ+vPFLeG02G7KkdG51ERvcggcCm07FZ764
DwzWpw1tFHUYMntWNYCKWcbI
-----END PRIVATE KEY-----";

    // The matching public RSA params (n base64url, e=AQAB) for the JWKS.
    const TEST_N: &str = "tlVSZ3rda5uMJ625rXsPBCs58ToEy0wsWBh5ZFB1aSsaeQnzKpW80VOo8TE5Uyob6vzWcPqm9sDI9pwMECwt9TBvRyqDrtwGXmTL-2OUpfEBtmOnDBaY6O1p0YdlJPL7MtAvVVkU7KiU06t78RTGzAojMdyqcOJkH_ZNkQApAy4y90pMbVPwRMXVyOvZVY1C1rLyw5b5QTaQHLi4XMamdVwGpIATVs84EBYh0ihOcvpZtd5PHsqQp4185y-LeYdSpE2BZDsE7SX_5EQUM6S9joDJSVkOle96rUPuV2a8A2GsPXHCk9pQaxktec798MPoVwKcXg6UOpfF8BnDViVsJQ";

    const ISSUER: &str = "https://idp.example.com";
    const CLIENT_ID: &str = "mockforge-client";
    const NONCE: &str = "test-nonce-123";

    fn jwks(kid: Option<&str>) -> Jwks {
        Jwks {
            keys: vec![Jwk {
                kty: "RSA".into(),
                kid: kid.map(str::to_string),
                alg: Some("RS256".into()),
                n: Some(TEST_N.into()),
                e: Some("AQAB".into()),
            }],
        }
    }

    fn sign(kid: Option<&str>, claims: serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = kid.map(str::to_string);
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).expect("test key");
        jsonwebtoken::encode(&header, &claims, &key).expect("sign")
    }

    fn valid_claims() -> serde_json::Value {
        let exp = (Utc::now() + chrono::Duration::hours(1)).timestamp();
        serde_json::json!({
            "iss": ISSUER,
            "aud": CLIENT_ID,
            "exp": exp,
            "sub": "user-1",
            "email": "alice@acme.com",
            "nonce": NONCE,
        })
    }

    #[test]
    fn valid_id_token_passes() {
        let token = sign(Some("k1"), valid_claims());
        let claims = validate_id_token(&token, &jwks(Some("k1")), ISSUER, CLIENT_ID, NONCE)
            .expect("should validate");
        assert_eq!(claims.email.as_deref(), Some("alice@acme.com"));
        assert_eq!(claims.sub, "user-1");
    }

    #[test]
    fn nonce_mismatch_rejected() {
        let token = sign(Some("k1"), valid_claims());
        let err = validate_id_token(&token, &jwks(Some("k1")), ISSUER, CLIENT_ID, "wrong-nonce")
            .unwrap_err();
        assert!(err.to_string().contains("nonce"), "got: {err}");
    }

    #[test]
    fn wrong_audience_rejected() {
        let token = sign(Some("k1"), valid_claims());
        assert!(
            validate_id_token(&token, &jwks(Some("k1")), ISSUER, "other-client", NONCE).is_err()
        );
    }

    #[test]
    fn wrong_issuer_rejected() {
        let token = sign(Some("k1"), valid_claims());
        assert!(validate_id_token(
            &token,
            &jwks(Some("k1")),
            "https://evil.example.com",
            CLIENT_ID,
            NONCE
        )
        .is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let mut claims = valid_claims();
        claims["exp"] = serde_json::json!((Utc::now() - chrono::Duration::hours(1)).timestamp());
        let token = sign(Some("k1"), claims);
        assert!(validate_id_token(&token, &jwks(Some("k1")), ISSUER, CLIENT_ID, NONCE).is_err());
    }

    #[test]
    fn tampered_signature_rejected() {
        let mut token = sign(Some("k1"), valid_claims());
        // Flip the last char of the signature segment.
        let last = token.pop().unwrap();
        token.push(if last == 'A' { 'B' } else { 'A' });
        assert!(validate_id_token(&token, &jwks(Some("k1")), ISSUER, CLIENT_ID, NONCE).is_err());
    }

    #[test]
    fn unknown_kid_rejected() {
        // Token signed under kid "k1" but JWKS only advertises "other".
        let token = sign(Some("k1"), valid_claims());
        assert!(validate_id_token(&token, &jwks(Some("other")), ISSUER, CLIENT_ID, NONCE).is_err());
    }

    #[test]
    fn pkce_challenge_is_sha256_of_verifier() {
        let (verifier, challenge) = generate_pkce();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, expected);
    }

    #[test]
    fn flow_cookie_roundtrips_and_rejects_tamper() {
        let secret = "test-secret";
        let flow = FlowState {
            org_id: "11111111-1111-1111-1111-111111111111".into(),
            org_slug: "acme".into(),
            state: "s".into(),
            nonce: "n".into(),
            pkce_verifier: "v".into(),
            issuer: ISSUER.into(),
            token_endpoint: format!("{ISSUER}/token"),
            jwks_uri: format!("{ISSUER}/jwks"),
            client_id: CLIENT_ID.into(),
            redirect_uri: "https://app/cb".into(),
            exp: (Utc::now() + chrono::Duration::minutes(5)).timestamp(),
        };
        let cookie = encode_flow(secret, &flow).unwrap();
        let back = decode_flow(secret, &cookie).unwrap();
        assert_eq!(back.org_slug, "acme");
        // A different secret must reject the cookie.
        assert!(decode_flow("other-secret", &cookie).is_err());
    }

    #[test]
    fn email_verified_only_rejects_explicit_false() {
        assert!(email_explicitly_unverified(&Some(serde_json::json!(false))));
        assert!(email_explicitly_unverified(&Some(serde_json::json!("false"))));
        assert!(email_explicitly_unverified(&Some(serde_json::json!("False"))));
        // Verified, or absent, must NOT be treated as unverified.
        assert!(!email_explicitly_unverified(&Some(serde_json::json!(true))));
        assert!(!email_explicitly_unverified(&Some(serde_json::json!("true"))));
        assert!(!email_explicitly_unverified(&None));
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }

    #[test]
    fn read_flow_cookie_parses_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "other=1; mf_oidc_flow=abc.def.ghi; x=2".parse().unwrap());
        assert_eq!(read_flow_cookie(&headers).as_deref(), Some("abc.def.ghi"));
    }
}

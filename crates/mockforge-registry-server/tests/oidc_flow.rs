//! Integration test for the OIDC SSO client layer (#746) against a real, local
//! mock IdP (an axum server that serves discovery + JWKS + token and signs an
//! RS256 ID token). Exercises discovery, code/token exchange, JWKS fetch, and
//! full ID-token validation (signature + iss + aud + exp + nonce) end-to-end
//! over HTTP.
//!
//! Requires the SSRF escape hatch (`MOCKFORGE_SSO_ALLOW_INSECURE_ISSUERS`) so the
//! guard permits the localhost mock IdP. Every test in this binary needs the
//! relaxed guard, so it is set once per test (the production guard is unit-tested
//! separately in `sso_domain`).

use std::sync::Arc;

use axum::{
    extract::State as AxState,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::json;

use mockforge_registry_server::handlers::oidc::{
    discover, exchange_code, fetch_jwks, validate_id_token,
};

const CLIENT_ID: &str = "mockforge-client-1";
const ID_TOKEN_NONCE: &str = "nonce-xyz-789";
const KID: &str = "k1";

// Throwaway RSA test key (2048-bit) + matching public modulus. Test-only.
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

const TEST_N: &str = "tlVSZ3rda5uMJ625rXsPBCs58ToEy0wsWBh5ZFB1aSsaeQnzKpW80VOo8TE5Uyob6vzWcPqm9sDI9pwMECwt9TBvRyqDrtwGXmTL-2OUpfEBtmOnDBaY6O1p0YdlJPL7MtAvVVkU7KiU06t78RTGzAojMdyqcOJkH_ZNkQApAy4y90pMbVPwRMXVyOvZVY1C1rLyw5b5QTaQHLi4XMamdVwGpIATVs84EBYh0ihOcvpZtd5PHsqQp4185y-LeYdSpE2BZDsE7SX_5EQUM6S9joDJSVkOle96rUPuV2a8A2GsPXHCk9pQaxktec798MPoVwKcXg6UOpfF8BnDViVsJQ";

async fn discovery(AxState(base): AxState<Arc<String>>) -> Json<serde_json::Value> {
    Json(json!({
        "issuer": *base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "jwks_uri": format!("{base}/jwks"),
    }))
}

async fn jwks() -> Json<serde_json::Value> {
    Json(json!({
        "keys": [{
            "kty": "RSA",
            "kid": KID,
            "alg": "RS256",
            "use": "sig",
            "n": TEST_N,
            "e": "AQAB",
        }]
    }))
}

async fn token(AxState(base): AxState<Arc<String>>) -> Json<serde_json::Value> {
    let exp = (Utc::now() + Duration::hours(1)).timestamp();
    let claims = json!({
        "iss": *base,
        "aud": CLIENT_ID,
        "exp": exp,
        "iat": Utc::now().timestamp(),
        "sub": "subject-1",
        "email": "bob@acme.com",
        "email_verified": true,
        "preferred_username": "bob",
        "nonce": ID_TOKEN_NONCE,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_string());
    let id_token = jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).unwrap(),
    )
    .unwrap();
    Json(json!({ "id_token": id_token, "access_token": "access-1", "token_type": "Bearer" }))
}

/// Spawn the mock IdP on an ephemeral localhost port; returns its base URL.
async fn spawn_mock_idp() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/token", post(token))
        .with_state(Arc::new(base.clone()));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

#[tokio::test]
async fn oidc_full_flow_against_mock_idp() {
    std::env::set_var("MOCKFORGE_SSO_ALLOW_INSECURE_ISSUERS", "1");
    let base = spawn_mock_idp().await;
    let client = reqwest::Client::new();

    // Discovery resolves the endpoints.
    let disc = discover(&client, &base).await.expect("discovery should succeed");
    assert_eq!(disc.issuer, base);
    assert_eq!(disc.token_endpoint, format!("{base}/token"));

    // Code -> token exchange returns an ID token.
    let id_token = exchange_code(
        &client,
        &disc.token_endpoint,
        "auth-code-123",
        "https://app.example.com/cb",
        CLIENT_ID,
        "client-secret",
        "pkce-verifier",
    )
    .await
    .expect("token exchange should succeed");

    // JWKS fetch + full ID-token validation.
    let jwks = fetch_jwks(&client, &disc.jwks_uri).await.expect("jwks fetch should succeed");
    let claims = validate_id_token(&id_token, &jwks, &disc.issuer, CLIENT_ID, ID_TOKEN_NONCE)
        .expect("ID token should validate");
    assert_eq!(claims.email.as_deref(), Some("bob@acme.com"));
    assert_eq!(claims.sub, "subject-1");
    assert_eq!(claims.preferred_username.as_deref(), Some("bob"));
}

#[tokio::test]
async fn oidc_rejects_wrong_nonce_from_idp() {
    std::env::set_var("MOCKFORGE_SSO_ALLOW_INSECURE_ISSUERS", "1");
    let base = spawn_mock_idp().await;
    let client = reqwest::Client::new();
    let disc = discover(&client, &base).await.unwrap();
    let id_token = exchange_code(
        &client,
        &disc.token_endpoint,
        "code",
        "https://app/cb",
        CLIENT_ID,
        "secret",
        "verifier",
    )
    .await
    .unwrap();
    let jwks = fetch_jwks(&client, &disc.jwks_uri).await.unwrap();
    // The IdP minted the token with ID_TOKEN_NONCE; validating with a different
    // expected nonce must fail (replay / token-injection defense).
    let err =
        validate_id_token(&id_token, &jwks, &disc.issuer, CLIENT_ID, "attacker-nonce").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("nonce"), "got: {err}");
}

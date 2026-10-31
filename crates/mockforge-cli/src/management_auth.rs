//! Management-token plumbing for CLI calls to the HTTP data port's control plane.
//!
//! The data port (default 3000) requires `X-MockForge-Management-Token` on every
//! control-plane route (`/__mockforge/*`, `/api/chaos/*`, ...). Servers read the
//! expected value from `MOCKFORGE_MANAGEMENT_TOKEN`; CLI commands send the same
//! variable (or an explicit `--management-token`).

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::StatusCode;

/// Environment variable carrying the management token for client and server.
pub(crate) const MANAGEMENT_TOKEN_ENV: &str = "MOCKFORGE_MANAGEMENT_TOKEN";

/// Header the data port checks for the management token.
pub(crate) const MANAGEMENT_TOKEN_HEADER: &str = "x-mockforge-management-token";

type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn normalize(token: Option<&str>) -> Option<&str> {
    token.map(str::trim).filter(|t| !t.is_empty())
}

/// Read the management token from `MOCKFORGE_MANAGEMENT_TOKEN`, ignoring blanks.
#[cfg_attr(not(any(feature = "mqtt", feature = "smtp")), allow(dead_code))]
pub(crate) fn token_from_env() -> Option<String> {
    let raw = std::env::var(MANAGEMENT_TOKEN_ENV).ok();
    normalize(raw.as_deref()).map(str::to_owned)
}

/// Build the default header map carrying `token`, if one is set.
fn management_headers(token: Option<&str>) -> Result<HeaderMap, BoxError> {
    let mut headers = HeaderMap::new();
    if let Some(token) = normalize(token) {
        let mut value = HeaderValue::from_str(token).map_err(|_| {
            format!("{MANAGEMENT_TOKEN_ENV} contains characters not valid in an HTTP header")
        })?;
        value.set_sensitive(true);
        headers.insert(MANAGEMENT_TOKEN_HEADER, value);
    }
    Ok(headers)
}

/// HTTP client that sends the management token header on every request when
/// `token` is set (blank tokens are treated as unset).
pub(crate) fn management_client(token: Option<&str>) -> Result<reqwest::Client, BoxError> {
    Ok(reqwest::Client::builder().default_headers(management_headers(token)?).build()?)
}

/// [`management_client`] with the token taken from `MOCKFORGE_MANAGEMENT_TOKEN`.
#[cfg_attr(not(any(feature = "mqtt", feature = "smtp")), allow(dead_code))]
pub(crate) fn management_client_from_env() -> Result<reqwest::Client, BoxError> {
    management_client(token_from_env().as_deref())
}

/// Actionable message for a 401 from a control-plane route, if `status` is 401.
pub(crate) fn unauthorized_hint(status: StatusCode) -> Option<String> {
    (status == StatusCode::UNAUTHORIZED).then(|| {
        format!(
            "Management API rejected the request (401 Unauthorized). Set \
             {MANAGEMENT_TOKEN_ENV} (or pass --management-token where supported) \
             to the token the MockForge server was started with."
        )
    })
}

/// Print [`unauthorized_hint`] to stderr when `status` is 401.
pub(crate) fn print_unauthorized_hint(status: StatusCode) {
    if let Some(hint) = unauthorized_hint(status) {
        eprintln!("   {hint}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::HeaderMap as AxumHeaders, routing::get, Router};

    /// Serve one route that echoes the management header (or "<none>").
    async fn echo_server() -> String {
        let app = Router::new().route(
            "/api/chaos/profiles",
            get(|headers: AxumHeaders| async move {
                headers
                    .get(MANAGEMENT_TOKEN_HEADER)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("<none>")
                    .to_string()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binding an ephemeral loopback port should succeed in tests");
        let addr = listener.local_addr().expect("bound listener has a local address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}/api/chaos/profiles")
    }

    #[tokio::test]
    async fn management_client_sends_token_header() {
        let url = echo_server().await;
        let client = management_client(Some("s3cret-token")).expect("valid token");
        let body = client.get(&url).send().await.expect("request").text().await.expect("body");
        assert_eq!(body, "s3cret-token");
    }

    #[tokio::test]
    async fn management_client_without_token_sends_no_header() {
        let url = echo_server().await;
        for token in [None, Some(""), Some("   ")] {
            let client = management_client(token).expect("no token is fine");
            let body = client.get(&url).send().await.expect("request").text().await.expect("body");
            assert_eq!(body, "<none>", "token {token:?} must not produce a header");
        }
    }

    #[test]
    fn management_headers_rejects_invalid_header_value() {
        let err = management_headers(Some("bad\ntoken")).expect_err("newline is invalid");
        assert!(err.to_string().contains(MANAGEMENT_TOKEN_ENV));
    }

    #[test]
    fn management_headers_marks_token_sensitive() {
        let headers = management_headers(Some("abc")).expect("valid token");
        let value = headers.get(MANAGEMENT_TOKEN_HEADER).expect("header present");
        assert!(value.is_sensitive());
        assert_eq!(value, "abc");
    }

    #[test]
    fn unauthorized_hint_only_for_401_and_names_env_var() {
        let hint = unauthorized_hint(StatusCode::UNAUTHORIZED).expect("401 has a hint");
        assert!(hint.contains(MANAGEMENT_TOKEN_ENV));
        assert!(unauthorized_hint(StatusCode::FORBIDDEN).is_none());
        assert!(unauthorized_hint(StatusCode::NOT_FOUND).is_none());
    }
}

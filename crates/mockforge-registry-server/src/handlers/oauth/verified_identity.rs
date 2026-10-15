//! Provider-owned email proof before any email-based account lookup or linking.
use super::{OAuthProvider, OAuthUserInfo};
use crate::error::ApiError;
use serde_json::Value;
use validator::ValidateEmail;

fn verified_email(value: Option<&str>) -> Result<String, ApiError> {
    let email = value.unwrap_or_default().trim().to_lowercase();
    if email.len() > 255 || !email.validate_email() {
        return Err(ApiError::InvalidRequest(
            "OAuth provider did not supply a valid verified email".into(),
        ));
    }
    Ok(email)
}

async fn provider_json(
    client: &reqwest::Client,
    url: &str,
    token: &str,
) -> Result<Value, ApiError> {
    let response = client
        .get(url)
        .bearer_auth(token)
        .header("User-Agent", "MockForge")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| ApiError::InvalidRequest("Unable to confirm OAuth provider identity".into()))?
        .error_for_status()
        .map_err(|_| {
            ApiError::InvalidRequest("OAuth provider rejected identity verification".into())
        })?;
    response
        .json()
        .await
        .map_err(|_| ApiError::InvalidRequest("Invalid OAuth provider identity response".into()))
}

pub(super) async fn fetch_user_info(
    provider: OAuthProvider,
    token: &str,
) -> Result<OAuthUserInfo, ApiError> {
    let (profile, emails) = match provider {
        OAuthProvider::GitHub => {
            ("https://api.github.com/user", "https://api.github.com/user/emails?per_page=100")
        }
        OAuthProvider::Google => ("https://www.googleapis.com/oauth2/v2/userinfo", ""),
    };
    fetch_at(provider, token, profile, emails).await
}

// Endpoint injection is private to this module's local transport tests. Production
// callers always use the fixed provider URLs above, with no environment override.
async fn fetch_at(
    provider: OAuthProvider,
    token: &str,
    profile_url: &str,
    email_url: &str,
) -> Result<OAuthUserInfo, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("OAuth identity client unavailable")))?;
    let user = provider_json(&client, profile_url, token).await?;
    match provider {
        OAuthProvider::GitHub => {
            let emails = provider_json(&client, email_url, token).await?;
            let rows = emails.as_array().ok_or_else(|| {
                ApiError::InvalidRequest("Invalid GitHub email verification response".into())
            })?;
            let mut primary = rows.iter().filter(|row| {
                row["primary"].as_bool() == Some(true) && row["verified"].as_bool() == Some(true)
            });
            let email = verified_email(primary.next().and_then(|row| row["email"].as_str()))?;
            if primary.next().is_some() {
                return Err(ApiError::InvalidRequest(
                    "Ambiguous GitHub primary email identity".into(),
                ));
            }
            let provider_id = user["id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or_else(|| ApiError::InvalidRequest("Invalid GitHub identity".into()))?
                .to_string();
            let username = user["login"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ApiError::InvalidRequest("Invalid GitHub identity".into()))?
                .to_string();
            Ok(OAuthUserInfo {
                provider_id,
                username,
                email,
                avatar_url: user["avatar_url"].as_str().map(str::to_owned),
            })
        }
        OAuthProvider::Google => {
            if user["verified_email"].as_bool() != Some(true) {
                return Err(ApiError::InvalidRequest(
                    "Google must verify email ownership before sign-in".into(),
                ));
            }
            let email = verified_email(user["email"].as_str())?;
            let provider_id = user["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ApiError::InvalidRequest("Invalid Google identity".into()))?
                .to_string();
            let username = email.split('@').next().unwrap_or("user").to_string();
            Ok(OAuthUserInfo {
                provider_id,
                username,
                email,
                avatar_url: user["picture"].as_str().map(str::to_owned),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};
    use serde_json::json;
    struct Server {
        url: String,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    async fn serve(profile: Value, emails: Value) -> Server {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route(
                "/profile",
                get(move || {
                    let profile = profile.clone();
                    async move { Json(profile) }
                }),
            )
            .route(
                "/emails",
                get(move || {
                    let emails = emails.clone();
                    async move { Json(emails) }
                }),
            );
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Server { url, task }
    }
    async fn github(emails: Value) -> Result<OAuthUserInfo, ApiError> {
        let server =
            serve(json!({"id":17,"login":"fixture","email":"attacker@example.invalid"}), emails)
                .await;
        fetch_at(
            OAuthProvider::GitHub,
            "synthetic-fixture-only",
            &format!("{}/profile", server.url),
            &format!("{}/emails", server.url),
        )
        .await
    }
    #[tokio::test]
    async fn github_uses_verified_primary_instead_of_public_profile_email() {
        let info =
            github(json!([{"email":"private@example.invalid","primary":true,"verified":true}]))
                .await
                .unwrap();
        assert_eq!(info.email, "private@example.invalid");
        assert_eq!(info.provider_id, "17");
    }
    #[tokio::test]
    async fn github_rejects_unverified_nonprimary_missing_and_ambiguous_emails() {
        for emails in [
            json!([]),
            json!([{"email":"victim@example.invalid","primary":true,"verified":false}]),
            json!([{"email":"victim@example.invalid","primary":false,"verified":true}]),
            json!([{"email":"","primary":true,"verified":true}]),
            json!([{"email":"a@example.invalid","primary":true,"verified":true},{"email":"b@example.invalid","primary":true,"verified":true}]),
        ] {
            assert!(github(emails).await.is_err());
        }
    }
    #[tokio::test]
    async fn google_requires_explicit_verified_email_and_canonicalizes_identity() {
        for verified in [json!(false), Value::Null, json!("true"), json!(true)] {
            let server=serve(json!({"id":"fixture-id","email":" Owner@Example.invalid ","verified_email":verified}),json!([])).await;
            let result = fetch_at(
                OAuthProvider::Google,
                "synthetic-fixture-only",
                &format!("{}/profile", server.url),
                "",
            )
            .await;
            if verified == json!(true) {
                assert_eq!(result.unwrap().email, "owner@example.invalid");
            } else {
                assert!(result.is_err());
            }
        }
    }
    #[tokio::test]
    async fn provider_error_status_cannot_be_accepted_as_identity_json() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/profile", listener.local_addr().unwrap());
        let app = Router::new().route(
            "/profile",
            get(|| async {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    Json(json!({"id":"fake","email":"fake@example.invalid","verified_email":true})),
                )
            }),
        );
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let result = fetch_at(OAuthProvider::Google, "synthetic-fixture-only", &url, "").await;
        task.abort();
        let _ = task.await;
        assert!(result.is_err());
    }
}

//! Bounded, tenant-scoped OpenAPI document loading for hosted deployments.

use crate::{models::HostedMock, storage::PluginStorage};
use anyhow::{anyhow, bail, Result};
use futures_util::StreamExt;
use mockforge_bench::ssrf::{check_url, guarded_client_builder, Policy};

const MAX_SPEC_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn fingerprint(source: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(source.as_bytes()))
}

/// Refresh a durable document snapshot during an authorized deployment.
/// Requests use the snapshot and never fetch owner-controlled URLs themselves.
pub(crate) async fn refresh(
    pool: &sqlx::PgPool,
    storage: &PluginStorage,
    deployment: &HostedMock,
) -> Result<HostedMock> {
    let Some(document) = load(storage, deployment).await? else {
        return Ok(deployment.clone());
    };
    // Validate the document and router before committing it as executable data.
    let spec = mockforge_openapi::OpenApiSpec::from_json(document.clone())
        .map_err(|_| anyhow!("Invalid OpenAPI specification"))?;
    super::local_runtime::validate_spec(&spec, &deployment.config_json)
        .map_err(anyhow::Error::msg)?;
    let id = deployment.id;
    let org_id = deployment.org_id;
    let digest = fingerprint(deployment.openapi_spec_url.as_deref().unwrap_or_default());
    let updated = crate::store::with_org_context(pool, org_id, |tx| Box::pin(async move {
        Ok(sqlx::query_as::<_, HostedMock>("UPDATE hosted_mocks SET metadata_json = COALESCE(metadata_json, '{}'::jsonb) || jsonb_build_object('runtime_openapi', $3::jsonb, 'runtime_openapi_source_sha256', $4::text), updated_at=NOW() WHERE id=$1 AND org_id=$2 AND deleted_at IS NULL RETURNING *")
            .bind(id).bind(org_id).bind(document).bind(digest).fetch_one(&mut **tx).await?)
    })).await?;
    Ok(updated)
}

pub(crate) async fn load(
    storage: &PluginStorage,
    deployment: &HostedMock,
) -> Result<Option<serde_json::Value>> {
    let Some(source) = deployment.openapi_spec_url.as_deref() else {
        return Ok(None);
    };
    let bytes = match storage.read_owned_spec(deployment.org_id, source).await? {
        Some(bytes) => bytes,
        None => fetch_public(source, Policy::strict()).await?,
    };
    let text = std::str::from_utf8(&bytes).map_err(|_| anyhow!("Specification must be UTF-8"))?;
    let document = serde_json::from_str(text)
        .or_else(|_| serde_yaml::from_str::<serde_json::Value>(text))
        .map_err(|_| anyhow!("Specification must be valid JSON or YAML"))?;
    Ok(Some(document))
}

async fn fetch_public(source: &str, policy: Policy) -> Result<Vec<u8>> {
    let url = url::Url::parse(source).map_err(|_| anyhow!("Invalid specification URL"))?;
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Specification URLs must not contain login credentials");
    }
    check_url(&url, policy)
        .map_err(|_| anyhow!("Specification URL must use a public HTTP or HTTPS endpoint"))?;
    let client = guarded_client_builder(policy)
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| anyhow!("Could not fetch specification"))?;
    if !response.status().is_success() {
        bail!("Specification endpoint returned an unsuccessful response");
    }
    if response.content_length().is_some_and(|len| len > MAX_SPEC_BYTES as u64) {
        bail!("Specification exceeds 2 MiB");
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| anyhow!("Could not read specification"))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_SPEC_BYTES {
            bail!("Specification exceeds 2 MiB");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Response, StatusCode},
        routing::get,
        Router,
    };

    #[tokio::test]
    async fn public_fetch_rejects_private_targets_and_credentials_before_connecting() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let source = format!("http://{address}/spec");
        assert!(fetch_public(&source, Policy::strict()).await.is_err());
        assert!(fetch_public("file:///etc/passwd", Policy::strict()).await.is_err());
        assert!(fetch_public(
            &format!("http://fixture:fixture@{address}/spec"),
            Policy::for_test()
        )
        .await
        .is_err());
        assert!(tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn public_fetch_is_bounded_and_checks_error_responses() {
        let router = Router::new()
            .route("/spec", get(|| async { "{\"openapi\":\"3.0.0\"}" }))
            .route("/failure", get(|| async { StatusCode::BAD_GATEWAY }))
            .route(
                "/oversize",
                get(|| async {
                    Response::builder().body(Body::from(vec![b'x'; MAX_SPEC_BYTES + 1])).unwrap()
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        assert_eq!(
            fetch_public(&format!("{base}/spec"), Policy::for_test()).await.unwrap(),
            b"{\"openapi\":\"3.0.0\"}"
        );
        assert!(fetch_public(&format!("{base}/failure"), Policy::for_test()).await.is_err());
        assert!(fetch_public(&format!("{base}/oversize"), Policy::for_test()).await.is_err());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }
}

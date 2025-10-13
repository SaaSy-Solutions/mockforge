//! Plugin binary storage (S3-compatible)

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client as S3Client,
};

use crate::config::Config;

#[derive(Clone)]
pub struct PluginStorage {
    client: S3Client,
    bucket: String,
}

impl PluginStorage {
    pub async fn new(config: &Config) -> Result<Self> {
        let aws_config = if let Some(endpoint) = &config.s3_endpoint {
            // Custom endpoint (MinIO, etc.)
            let credentials = Credentials::new(
                std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_default(),
                std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default(),
                None,
                None,
                "static",
            );

            aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(config.s3_region.clone()))
                .credentials_provider(credentials)
                .endpoint_url(endpoint)
                .load()
                .await
        } else {
            // AWS S3
            aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(config.s3_region.clone()))
                .load()
                .await
        };

        let client = S3Client::new(&aws_config);

        Ok(Self {
            client,
            bucket: config.s3_bucket.clone(),
        })
    }

    pub async fn upload_plugin(
        &self,
        plugin_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        let key = format!("plugins/{}/{}.wasm", plugin_name, version);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type("application/wasm")
            .send()
            .await?;

        // Return download URL
        let url = if let Ok(endpoint) = std::env::var("S3_ENDPOINT") {
            format!("{}/{}/{}", endpoint, self.bucket, key)
        } else {
            format!("https://{}.s3.amazonaws.com/{}", self.bucket, key)
        };

        Ok(url)
    }

    pub async fn download_plugin(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.client.get_object().bucket(&self.bucket).key(key).send().await?;

        let bytes = response.body.collect().await?;
        Ok(bytes.to_vec())
    }

    pub async fn delete_plugin(&self, key: &str) -> Result<()> {
        self.client.delete_object().bucket(&self.bucket).key(key).send().await?;

        Ok(())
    }
}

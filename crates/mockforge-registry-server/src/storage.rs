//! Plugin binary storage (S3-compatible with local filesystem fallback)

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client as S3Client,
};
use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Clone)]
enum StorageBackend {
    S3 { client: S3Client, bucket: String },
    Local { base_dir: PathBuf },
}

#[derive(Clone)]
pub struct PluginStorage {
    backend: StorageBackend,
}

impl PluginStorage {
    pub async fn new(config: &Config) -> Result<Self> {
        // Determine whether S3 is usable: we need a real bucket name AND either
        // an explicit endpoint with credentials, or default AWS credentials.
        let use_s3 = if config.s3_endpoint.is_some() {
            // Custom endpoint requires explicit credentials
            std::env::var("AWS_ACCESS_KEY_ID")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .is_some()
                && std::env::var("AWS_SECRET_ACCESS_KEY")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .is_some()
        } else {
            // For AWS S3, check if the bucket is the default placeholder and
            // if AWS credentials are likely available
            config.s3_bucket != "mockforge-plugins"
                || std::env::var("AWS_ACCESS_KEY_ID").is_ok()
                || std::env::var("AWS_PROFILE").is_ok()
                || std::env::var("AWS_ROLE_ARN").is_ok()
        };

        if use_s3 {
            let aws_config = if let Some(endpoint) = &config.s3_endpoint {
                let access_key_id = std::env::var("AWS_ACCESS_KEY_ID")
                    .context("AWS_ACCESS_KEY_ID is required when using custom S3 endpoint")?;
                let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY")
                    .context("AWS_SECRET_ACCESS_KEY is required when using custom S3 endpoint")?;

                tracing::info!("Using custom S3 endpoint: {} with explicit credentials", endpoint);

                let credentials =
                    Credentials::new(access_key_id, secret_access_key, None, None, "static");

                aws_config::defaults(BehaviorVersion::latest())
                    .region(Region::new(config.s3_region.clone()))
                    .credentials_provider(credentials)
                    .endpoint_url(endpoint)
                    .load()
                    .await
            } else {
                tracing::info!(
                    "Using AWS S3 with default credentials provider chain (region: {})",
                    config.s3_region
                );

                aws_config::defaults(BehaviorVersion::latest())
                    .region(Region::new(config.s3_region.clone()))
                    .load()
                    .await
            };

            let client = S3Client::new(&aws_config);
            let bucket = config.s3_bucket.clone();

            // Validate S3 connectivity — fall back to local storage if unreachable
            match client.head_bucket().bucket(&bucket).send().await {
                Ok(_) => {
                    tracing::info!("S3 storage verified (bucket: {})", bucket);
                    return Ok(Self {
                        backend: StorageBackend::S3 { client, bucket },
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        "S3 health check failed (bucket: {}): {}. Falling back to local storage.",
                        bucket,
                        e
                    );
                    // Fall through to local storage
                }
            }
        }

        // Local filesystem fallback
        let base_dir = PathBuf::from(
            std::env::var("STORAGE_PATH").unwrap_or_else(|_| "./data/storage".to_string()),
        );

        // Ensure base directory exists
        tokio::fs::create_dir_all(&base_dir).await.with_context(|| {
            format!("Failed to create local storage directory: {}", base_dir.display())
        })?;

        tracing::info!("Using local filesystem storage at: {}", base_dir.display());

        Ok(Self {
            backend: StorageBackend::Local { base_dir },
        })
    }

    /// Sanitize a name/version for use in S3 keys or local file paths
    /// Removes dangerous characters and path traversal attempts
    fn sanitize_key_component(component: &str) -> String {
        component
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .take(100) // Limit length
            .collect::<String>()
            .trim_matches('.')
            .trim_matches('-')
            .trim_matches('_')
            .to_lowercase()
    }

    /// Write data to a local file path, creating parent directories as needed
    async fn local_write(base_dir: &Path, key: &str, data: Vec<u8>) -> Result<String> {
        let file_path = base_dir.join(key);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        tokio::fs::write(&file_path, &data)
            .await
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

        // Return relative path as the URL (the registry server can serve these)
        Ok(format!("/storage/{key}"))
    }

    /// Read data from a local file path
    async fn local_read(base_dir: &Path, key: &str) -> Result<Vec<u8>> {
        let file_path = base_dir.join(key);
        tokio::fs::read(&file_path)
            .await
            .with_context(|| format!("Failed to read file: {}", file_path.display()))
    }

    /// Delete a local file
    async fn local_delete(base_dir: &Path, key: &str) -> Result<()> {
        let file_path = base_dir.join(key);
        if file_path.exists() {
            tokio::fs::remove_file(&file_path)
                .await
                .with_context(|| format!("Failed to delete file: {}", file_path.display()))?;
        }
        Ok(())
    }

    /// Build an S3 download URL for the given bucket and key
    fn s3_url(bucket: &str, key: &str) -> String {
        if let Ok(endpoint) = std::env::var("S3_ENDPOINT") {
            format!("{}/{}/{}", endpoint, bucket, key)
        } else {
            format!("https://{}.s3.amazonaws.com/{}", bucket, key)
        }
    }

    pub async fn upload_plugin(
        &self,
        plugin_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        let safe_name = Self::sanitize_key_component(plugin_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Plugin name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        let key = format!("plugins/{}/{}.wasm", safe_name, safe_version);

        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.into())
                    .content_type("application/wasm")
                    .send()
                    .await?;
                Ok(Self::s3_url(bucket, &key))
            }
            StorageBackend::Local { base_dir } => Self::local_write(base_dir, &key, data).await,
        }
    }

    pub async fn upload_template(
        &self,
        template_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        let safe_name = Self::sanitize_key_component(template_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Template name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        let extension = if data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B {
            "tar.gz"
        } else if data.len() >= 4
            && data[0] == 0x50
            && data[1] == 0x4B
            && (data[2] == 0x03 || data[2] == 0x05 || data[2] == 0x07)
            && (data[3] == 0x04 || data[3] == 0x06 || data[3] == 0x08)
        {
            "zip"
        } else {
            "tar.gz"
        };

        let key = format!("templates/{}/{}.{}", safe_name, safe_version, extension);

        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.into())
                    .content_type(if extension == "zip" {
                        "application/zip"
                    } else {
                        "application/gzip"
                    })
                    .send()
                    .await?;
                Ok(Self::s3_url(bucket, &key))
            }
            StorageBackend::Local { base_dir } => Self::local_write(base_dir, &key, data).await,
        }
    }

    pub async fn upload_scenario(
        &self,
        scenario_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        let safe_name = Self::sanitize_key_component(scenario_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Scenario name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        let extension = if data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B {
            "tar.gz"
        } else if data.len() >= 4
            && data[0] == 0x50
            && data[1] == 0x4B
            && (data[2] == 0x03 || data[2] == 0x05 || data[2] == 0x07)
            && (data[3] == 0x04 || data[3] == 0x06 || data[3] == 0x08)
        {
            "zip"
        } else {
            "tar.gz"
        };

        let key = format!("scenarios/{}/{}.{}", safe_name, safe_version, extension);

        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.into())
                    .content_type(if extension == "zip" {
                        "application/zip"
                    } else {
                        "application/gzip"
                    })
                    .send()
                    .await?;
                Ok(Self::s3_url(bucket, &key))
            }
            StorageBackend::Local { base_dir } => Self::local_write(base_dir, &key, data).await,
        }
    }

    pub async fn download_plugin(&self, key: &str) -> Result<Vec<u8>> {
        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                let response = client.get_object().bucket(bucket).key(key).send().await?;
                let bytes = response.body.collect().await?;
                Ok(bytes.to_vec())
            }
            StorageBackend::Local { base_dir } => Self::local_read(base_dir, key).await,
        }
    }

    pub async fn delete_plugin(&self, key: &str) -> Result<()> {
        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client.delete_object().bucket(bucket).key(key).send().await?;
                Ok(())
            }
            StorageBackend::Local { base_dir } => Self::local_delete(base_dir, key).await,
        }
    }

    /// Upload an OpenAPI spec file for a hosted mock deployment
    pub async fn upload_spec(
        &self,
        org_id: &str,
        spec_name: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        let safe_org = Self::sanitize_key_component(org_id);
        let safe_name = Self::sanitize_key_component(spec_name);

        if safe_org.is_empty() {
            anyhow::bail!("Org ID cannot be empty after sanitization");
        }
        if safe_name.is_empty() {
            anyhow::bail!("Spec name cannot be empty after sanitization");
        }

        let key = format!("specs/{}/{}.json", safe_org, safe_name);

        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.into())
                    .content_type("application/json")
                    .send()
                    .await?;
                Ok(Self::s3_url(bucket, &key))
            }
            StorageBackend::Local { base_dir } => Self::local_write(base_dir, &key, data).await,
        }
    }

    /// Health check - verify storage connectivity
    pub async fn health_check(&self) -> Result<()> {
        match &self.backend {
            StorageBackend::S3 { client, bucket } => {
                client
                    .head_bucket()
                    .bucket(bucket)
                    .send()
                    .await
                    .context("S3 bucket health check failed")?;
                Ok(())
            }
            StorageBackend::Local { base_dir } => {
                // Verify directory exists and is writable
                let test_file = base_dir.join(".health_check");
                tokio::fs::write(&test_file, b"ok")
                    .await
                    .context("Local storage health check failed: cannot write")?;
                tokio::fs::remove_file(&test_file)
                    .await
                    .context("Local storage health check failed: cannot delete")?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_key_component() {
        // Normal names should be lowercased
        assert_eq!(PluginStorage::sanitize_key_component("MyPlugin"), "myplugin");

        // Alphanumeric with hyphens, underscores, and dots should be preserved
        assert_eq!(PluginStorage::sanitize_key_component("my-plugin_v1.0"), "my-plugin_v1.0");

        // Dangerous characters should be removed
        assert_eq!(PluginStorage::sanitize_key_component("my/plugin"), "myplugin");
        assert_eq!(PluginStorage::sanitize_key_component("../evil"), "evil");
        assert_eq!(PluginStorage::sanitize_key_component("plugin<script>"), "pluginscript");

        // Path traversal attempts should be sanitized
        assert_eq!(PluginStorage::sanitize_key_component("../../etc/passwd"), "etcpasswd");

        // Special characters should be removed
        assert_eq!(PluginStorage::sanitize_key_component("plugin@#$%"), "plugin");

        // Leading/trailing dots, hyphens, underscores should be trimmed
        assert_eq!(PluginStorage::sanitize_key_component("...plugin..."), "plugin");
        assert_eq!(PluginStorage::sanitize_key_component("---plugin---"), "plugin");
        assert_eq!(PluginStorage::sanitize_key_component("___plugin___"), "plugin");

        // Mixed case with special chars
        assert_eq!(
            PluginStorage::sanitize_key_component("My!Super@Plugin#2024"),
            "mysuperplugin2024"
        );

        // Long strings should be truncated to 100 characters
        let long_name = "a".repeat(150);
        assert_eq!(PluginStorage::sanitize_key_component(&long_name).len(), 100);

        // Empty after sanitization
        assert_eq!(PluginStorage::sanitize_key_component("@#$%^&*()"), "");
    }

    #[test]
    fn test_sanitize_key_component_versions() {
        // Semantic versions should be preserved
        assert_eq!(PluginStorage::sanitize_key_component("1.0.0"), "1.0.0");
        assert_eq!(PluginStorage::sanitize_key_component("2.3.4-alpha"), "2.3.4-alpha");
        assert_eq!(PluginStorage::sanitize_key_component("1.0.0-beta.1"), "1.0.0-beta.1");

        // Version with invalid characters (slashes removed, dots preserved)
        assert_eq!(PluginStorage::sanitize_key_component("1.0.0/../../etc"), "1.0.0....etc");
    }

    #[test]
    fn test_sanitize_key_component_unicode() {
        // Unicode should be removed (only ASCII alphanumeric allowed)
        // Trailing hyphen is also trimmed
        assert_eq!(PluginStorage::sanitize_key_component("plugin-中文"), "plugin");
        assert_eq!(PluginStorage::sanitize_key_component("émoji-😀"), "moji");
    }

    #[test]
    fn test_sanitize_key_component_edge_cases() {
        // Empty string
        assert_eq!(PluginStorage::sanitize_key_component(""), "");

        // Only special characters
        assert_eq!(PluginStorage::sanitize_key_component("!@#$%^&*()"), "");

        // Whitespace should be removed
        assert_eq!(PluginStorage::sanitize_key_component("my plugin"), "myplugin");

        // Tabs and newlines should be removed
        assert_eq!(PluginStorage::sanitize_key_component("my\tplugin\n"), "myplugin");
    }

    #[test]
    fn test_sanitize_key_component_security() {
        // Path traversal attempts
        assert_eq!(PluginStorage::sanitize_key_component("../"), "");
        assert_eq!(PluginStorage::sanitize_key_component("..\\"), "");
        assert_eq!(
            PluginStorage::sanitize_key_component("../../../../../../etc/passwd"),
            "etcpasswd"
        );

        // Null bytes
        assert_eq!(PluginStorage::sanitize_key_component("plugin\0evil"), "pluginevil");

        // Windows path separators
        assert_eq!(
            PluginStorage::sanitize_key_component("C:\\Windows\\System32"),
            "cwindowssystem32"
        );
    }

    #[tokio::test]
    async fn test_local_storage_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path().to_path_buf();

        let data = b"test plugin data".to_vec();
        let key = "plugins/test-plugin/1.0.0.wasm";

        // Write
        let url = PluginStorage::local_write(&base_dir, key, data.clone()).await.unwrap();
        assert_eq!(url, format!("/storage/{key}"));

        // Read
        let read_data = PluginStorage::local_read(&base_dir, key).await.unwrap();
        assert_eq!(read_data, data);

        // Delete
        PluginStorage::local_delete(&base_dir, key).await.unwrap();
        assert!(PluginStorage::local_read(&base_dir, key).await.is_err());
    }

    #[tokio::test]
    async fn test_local_storage_health_check() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PluginStorage {
            backend: StorageBackend::Local {
                base_dir: temp_dir.path().to_path_buf(),
            },
        };
        assert!(storage.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_local_upload_spec() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PluginStorage {
            backend: StorageBackend::Local {
                base_dir: temp_dir.path().to_path_buf(),
            },
        };

        let spec_data = br#"{"openapi":"3.0.0","info":{"title":"Test","version":"1.0"}}"#.to_vec();
        let url = storage.upload_spec("org123", "my-api", spec_data.clone()).await.unwrap();
        assert!(url.contains("specs/org123/my-api.json"));

        // Verify file was written
        let read_back =
            tokio::fs::read(temp_dir.path().join("specs/org123/my-api.json")).await.unwrap();
        assert_eq!(read_back, spec_data);
    }

    #[tokio::test]
    async fn test_local_upload_plugin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PluginStorage {
            backend: StorageBackend::Local {
                base_dir: temp_dir.path().to_path_buf(),
            },
        };

        let plugin_data = vec![0u8; 100];
        let url = storage.upload_plugin("my-plugin", "1.0.0", plugin_data).await.unwrap();
        assert!(url.contains("plugins/my-plugin/1.0.0.wasm"));
    }

    #[tokio::test]
    async fn test_local_download_and_delete_plugin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = PluginStorage {
            backend: StorageBackend::Local {
                base_dir: temp_dir.path().to_path_buf(),
            },
        };

        let plugin_data = vec![42u8; 50];
        storage.upload_plugin("test-dl", "2.0.0", plugin_data.clone()).await.unwrap();

        let key = "plugins/test-dl/2.0.0.wasm";
        let downloaded = storage.download_plugin(key).await.unwrap();
        assert_eq!(downloaded, plugin_data);

        storage.delete_plugin(key).await.unwrap();
        assert!(storage.download_plugin(key).await.is_err());
    }
}

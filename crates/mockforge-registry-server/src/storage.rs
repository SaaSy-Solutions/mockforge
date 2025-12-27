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

    /// Sanitize a name/version for use in S3 keys
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

    pub async fn upload_plugin(
        &self,
        plugin_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        // Sanitize plugin name and version to prevent path traversal
        let safe_name = Self::sanitize_key_component(plugin_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Plugin name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        let key = format!("plugins/{}/{}.wasm", safe_name, safe_version);

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

    pub async fn upload_template(
        &self,
        template_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        // Sanitize template name and version to prevent path traversal
        let safe_name = Self::sanitize_key_component(template_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Template name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        // Determine file extension based on content (tar.gz or zip)
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
            "tar.gz" // Default
        };

        let key = format!("templates/{}/{}.{}", safe_name, safe_version, extension);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type(if extension == "zip" {
                "application/zip"
            } else {
                "application/gzip"
            })
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

    pub async fn upload_scenario(
        &self,
        scenario_name: &str,
        version: &str,
        data: Vec<u8>,
    ) -> Result<String> {
        // Sanitize scenario name and version to prevent path traversal
        let safe_name = Self::sanitize_key_component(scenario_name);
        let safe_version = Self::sanitize_key_component(version);

        if safe_name.is_empty() {
            anyhow::bail!("Scenario name cannot be empty after sanitization");
        }
        if safe_version.is_empty() {
            anyhow::bail!("Version cannot be empty after sanitization");
        }

        // Determine file extension based on content (tar.gz or zip)
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
            "tar.gz" // Default
        };

        let key = format!("scenarios/{}/{}.{}", safe_name, safe_version, extension);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type(if extension == "zip" {
                "application/zip"
            } else {
                "application/gzip"
            })
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
}

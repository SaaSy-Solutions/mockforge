//! Operation-aware latency/failure profiles (per operationId and per tag).
use globwalk::GlobWalkerBuilder;
use rand::{rng, Rng};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

/// Latency and failure profile for request simulation
#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    /// Fixed latency in milliseconds
    pub fixed_ms: Option<u64>,
    /// Random jitter to add to fixed latency (milliseconds)
    pub jitter_ms: Option<u64>,
    /// Probability of failure (0.0 to 1.0)
    pub fail_p: Option<f64>,
    /// HTTP status code to return on failure
    pub fail_status: Option<u16>,
}

/// Collection of latency profiles organized by operation ID and tags
#[derive(Debug, Default, Clone)]
pub struct LatencyProfiles {
    /// Profiles keyed by OpenAPI operation ID
    by_operation: HashMap<String, Profile>,
    /// Profiles keyed by OpenAPI tag
    by_tag: HashMap<String, Profile>,
}

impl LatencyProfiles {
    /// Load latency profiles from files matching a glob pattern
    ///
    /// # Arguments
    /// * `pattern` - Glob pattern to match profile files (e.g., "profiles/*.yaml")
    ///
    /// # Returns
    /// `Ok(LatencyProfiles)` on success, `Err` if files cannot be read or parsed
    pub async fn load_from_glob(pattern: &str) -> anyhow::Result<Self> {
        let mut result = LatencyProfiles::default();
        for dir_entry in GlobWalkerBuilder::from_patterns(".", &[pattern]).build()? {
            let path = dir_entry?.path().to_path_buf();
            if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                let text = tokio::fs::read_to_string(&path).await?;
                let cfg: HashMap<String, Profile> = serde_yaml::from_str(&text)?;
                for (k, v) in cfg {
                    if let Some(rest) = k.strip_prefix("operation:") {
                        result.by_operation.insert(rest.to_string(), v);
                    } else if let Some(rest) = k.strip_prefix("tag:") {
                        result.by_tag.insert(rest.to_string(), v);
                    }
                }
            }
        }
        Ok(result)
    }

    /// Check if a fault should be injected for the given operation or tags
    ///
    /// Returns the HTTP status code and error message if a fault should be injected,
    /// otherwise returns None.
    ///
    /// # Arguments
    /// * `operation_id` - OpenAPI operation ID to check for operation-specific profile
    /// * `tags` - List of tags to check for tag-specific profiles
    ///
    /// # Returns
    /// `Some((status_code, message))` if fault should be injected, `None` otherwise
    pub async fn maybe_fault(&self, operation_id: &str, tags: &[String]) -> Option<(u16, String)> {
        let profile = self
            .by_operation
            .get(operation_id)
            .or_else(|| tags.iter().find_map(|t| self.by_tag.get(t)));
        if let Some(p) = profile {
            let base = p.fixed_ms.unwrap_or(0);
            let jitter = p.jitter_ms.unwrap_or(0);
            let mut rng = rng();
            let extra: u64 = if jitter > 0 {
                rng.random_range(0..=jitter)
            } else {
                0
            };
            sleep(Duration::from_millis(base + extra)).await;
            if let Some(fp) = p.fail_p {
                let roll: f64 = rng.random();
                if roll < fp {
                    return Some((
                        p.fail_status.unwrap_or(500),
                        format!("Injected failure (p={:.2})", fp),
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let profile = Profile {
            fixed_ms: Some(100),
            jitter_ms: Some(20),
            fail_p: Some(0.1),
            fail_status: Some(503),
        };

        assert_eq!(profile.fixed_ms, Some(100));
        assert_eq!(profile.jitter_ms, Some(20));
        assert_eq!(profile.fail_p, Some(0.1));
        assert_eq!(profile.fail_status, Some(503));
    }

    #[test]
    fn test_latency_profiles_default() {
        let profiles = LatencyProfiles::default();
        assert!(profiles.by_operation.is_empty());
        assert!(profiles.by_tag.is_empty());
    }

    #[tokio::test]
    async fn test_maybe_fault_no_profile() {
        let profiles = LatencyProfiles::default();
        let result = profiles.maybe_fault("test_op", &[]).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_fault_with_operation_profile_no_failure() {
        let mut profiles = LatencyProfiles::default();
        profiles.by_operation.insert(
            "test_op".to_string(),
            Profile {
                fixed_ms: Some(1),
                jitter_ms: Some(1),
                fail_p: Some(0.0),
                fail_status: Some(500),
            },
        );

        let result = profiles.maybe_fault("test_op", &[]).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_fault_with_tag_profile() {
        let mut profiles = LatencyProfiles::default();
        profiles.by_tag.insert(
            "slow".to_string(),
            Profile {
                fixed_ms: Some(1),
                jitter_ms: None,
                fail_p: Some(0.0),
                fail_status: None,
            },
        );

        let tags = vec!["slow".to_string()];
        let result = profiles.maybe_fault("unknown_op", &tags).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_maybe_fault_guaranteed_failure() {
        let mut profiles = LatencyProfiles::default();
        profiles.by_operation.insert(
            "failing_op".to_string(),
            Profile {
                fixed_ms: Some(0),
                jitter_ms: None,
                fail_p: Some(1.0),
                fail_status: Some(503),
            },
        );

        let result = profiles.maybe_fault("failing_op", &[]).await;
        assert!(result.is_some());
        let (status, _message) = result.unwrap();
        assert_eq!(status, 503);
    }

    #[tokio::test]
    async fn test_maybe_fault_operation_priority_over_tag() {
        let mut profiles = LatencyProfiles::default();

        profiles.by_operation.insert(
            "test_op".to_string(),
            Profile {
                fixed_ms: Some(1),
                jitter_ms: None,
                fail_p: Some(0.0),
                fail_status: Some(500),
            },
        );

        profiles.by_tag.insert(
            "test_tag".to_string(),
            Profile {
                fixed_ms: Some(100),
                jitter_ms: None,
                fail_p: Some(1.0),
                fail_status: Some(503),
            },
        );

        let tags = vec!["test_tag".to_string()];
        let result = profiles.maybe_fault("test_op", &tags).await;

        // Operation profile should take priority, so no failure
        assert!(result.is_none());
    }

    #[test]
    fn test_profile_deserialization() {
        let yaml = r#"
        fixed_ms: 100
        jitter_ms: 20
        fail_p: 0.1
        fail_status: 503
        "#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.fixed_ms, Some(100));
        assert_eq!(profile.jitter_ms, Some(20));
        assert_eq!(profile.fail_p, Some(0.1));
        assert_eq!(profile.fail_status, Some(503));
    }

    #[test]
    fn test_profile_partial_deserialization() {
        let yaml = r#"
        fixed_ms: 50
        "#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.fixed_ms, Some(50));
        assert!(profile.jitter_ms.is_none());
        assert!(profile.fail_p.is_none());
        assert!(profile.fail_status.is_none());
    }
}

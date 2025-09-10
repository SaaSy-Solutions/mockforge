//! Operation-aware latency/failure profiles (per operationId and per tag).
use globwalk::GlobWalkerBuilder;
use rand::{rng, Rng};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub fixed_ms: Option<u64>,
    pub jitter_ms: Option<u64>,
    pub fail_p: Option<f64>,
    pub fail_status: Option<u16>,
}

#[derive(Debug, Default, Clone)]
pub struct LatencyProfiles {
    by_operation: HashMap<String, Profile>,
    by_tag: HashMap<String, Profile>,
}

impl LatencyProfiles {
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

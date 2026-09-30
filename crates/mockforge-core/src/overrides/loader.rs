//! Override loading functionality
//!
//! Rules come from YAML files (glob patterns or absolute paths, usually via
//! `MOCKFORGE_HTTP_OVERRIDES_GLOB`) and from an inline JSON array in
//! `MOCKFORGE_HTTP_OVERRIDES`, which is how the registry hands a hosted mock
//! its stored rules.

use globwalk::GlobWalkerBuilder;
use std::path::{Path, PathBuf};

use super::models::{OverrideRule, Overrides, PatchOp};
use crate::templating::expand_tokens as core_expand_tokens;

/// Environment variable holding comma-separated override file globs
pub const OVERRIDES_GLOB_ENV: &str = "MOCKFORGE_HTTP_OVERRIDES_GLOB";
/// Environment variable holding an inline JSON array of override rules
pub const OVERRIDES_INLINE_ENV: &str = "MOCKFORGE_HTTP_OVERRIDES";

impl Overrides {
    /// Combine every configured source, in order: `config_rules` (from
    /// `mockforge.yaml`), the file globs, then the inline JSON rules.
    pub async fn load(config_rules: Vec<OverrideRule>) -> anyhow::Result<Self> {
        let mut rules = config_rules;
        if let Ok(env_patterns) = std::env::var(OVERRIDES_GLOB_ENV) {
            let patterns: Vec<String> =
                env_patterns.split(',').map(|s| s.trim().to_string()).collect();
            rules.extend(load_rule_files(&patterns).await?);
        }
        if let Ok(inline) = std::env::var(OVERRIDES_INLINE_ENV) {
            let inline_rules: Vec<OverrideRule> = serde_json::from_str(&inline).map_err(|e| {
                anyhow::anyhow!("{OVERRIDES_INLINE_ENV} is not a JSON rule array: {e}")
            })?;
            rules.extend(inline_rules);
        }
        Ok(Overrides::new(rules)?)
    }
}

async fn load_rule_files(patterns: &[String]) -> anyhow::Result<Vec<OverrideRule>> {
    let mut rules = Vec::new();
    for pat in patterns {
        let path = Path::new(pat);
        let files: Vec<PathBuf> = if path.is_absolute() && path.is_file() {
            vec![path.to_path_buf()]
        } else {
            GlobWalkerBuilder::from_patterns(".", &[pat])
                .build()?
                .map(|entry| entry.map(|e| e.path().to_path_buf()))
                .collect::<Result<_, _>>()?
        };
        for file in files {
            if file.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                let text = tokio::fs::read_to_string(&file).await?;
                let mut file_rules: Vec<OverrideRule> = serde_yaml::from_str(&text)?;
                // File rules have always had their tokens expanded once at load
                // time; rules set through the API keep them verbatim.
                for op in file_rules.iter_mut().flat_map(|r| r.patch.iter_mut()) {
                    if let PatchOp::Add { value, .. } | PatchOp::Replace { value, .. } = op {
                        *value = core_expand_tokens(value);
                    }
                }
                rules.extend(file_rules);
            }
        }
    }
    Ok(rules)
}

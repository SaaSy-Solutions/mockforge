//! Persistent configuration loaded from `~/.config/mockforge/tui.toml`.
//!
//! CLI arguments always override config file values.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level TUI configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TuiConfig {
    /// Admin server URL (e.g. `http://localhost:9080`).
    pub admin_url: String,

    /// Dashboard refresh interval in seconds.
    pub refresh_interval: u64,

    /// Color theme: `"dark"` or `"light"`.
    pub theme: String,

    /// Last-used tab index (restored on startup).
    pub last_tab: Option<usize>,

    /// Optional log file path.
    pub log_file: Option<String>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            admin_url: "http://localhost:9080".into(),
            refresh_interval: 2,
            theme: "dark".into(),
            last_tab: None,
            log_file: None,
        }
    }
}

impl TuiConfig {
    /// Resolve the config file path: `~/.config/mockforge/tui.toml`.
    pub fn path() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".config").join("mockforge").join("tui.toml"))
    }

    /// Load config from the default path. Returns `Default` if the file
    /// doesn't exist or can't be parsed.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(&p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save config to the default path, creating parent directories as needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path =
            Self::path().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    /// Returns `true` if the theme is "light".
    pub fn is_light_theme(&self) -> bool {
        self.theme.eq_ignore_ascii_case("light")
    }
}

/// Simple home directory lookup via `$HOME`.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.admin_url, "http://localhost:9080");
        assert_eq!(cfg.refresh_interval, 2);
        assert_eq!(cfg.theme, "dark");
        assert!(cfg.last_tab.is_none());
        assert!(cfg.log_file.is_none());
    }

    #[test]
    fn deserialize_minimal_toml() {
        let toml_str = r#"admin_url = "http://remote:9080""#;
        let cfg: TuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.admin_url, "http://remote:9080");
        // Defaults fill in the rest.
        assert_eq!(cfg.refresh_interval, 2);
        assert_eq!(cfg.theme, "dark");
    }

    #[test]
    fn deserialize_full_toml() {
        let toml_str = r#"
admin_url = "http://prod:9090"
refresh_interval = 5
theme = "light"
last_tab = 3
log_file = "/tmp/tui.log"
"#;
        let cfg: TuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.admin_url, "http://prod:9090");
        assert_eq!(cfg.refresh_interval, 5);
        assert_eq!(cfg.theme, "light");
        assert_eq!(cfg.last_tab, Some(3));
        assert_eq!(cfg.log_file.as_deref(), Some("/tmp/tui.log"));
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let cfg = TuiConfig {
            admin_url: "http://test:8080".into(),
            refresh_interval: 10,
            theme: "light".into(),
            last_tab: Some(5),
            log_file: Some("/var/log/tui.log".into()),
        };
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let deserialized: TuiConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn is_light_theme_case_insensitive() {
        let mut cfg = TuiConfig::default();
        assert!(!cfg.is_light_theme());

        cfg.theme = "light".into();
        assert!(cfg.is_light_theme());

        cfg.theme = "Light".into();
        assert!(cfg.is_light_theme());

        cfg.theme = "LIGHT".into();
        assert!(cfg.is_light_theme());

        cfg.theme = "dark".into();
        assert!(!cfg.is_light_theme());
    }

    #[test]
    fn unknown_fields_ignored() {
        let toml_str = r#"
admin_url = "http://localhost:9080"
unknown_field = "should be ignored"
"#;
        let cfg: TuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.admin_url, "http://localhost:9080");
    }

    #[test]
    fn config_path_is_under_home() {
        // Only testable if $HOME is set.
        if let Some(path) = TuiConfig::path() {
            assert!(path.ends_with(".config/mockforge/tui.toml"));
        }
    }

    #[test]
    fn load_returns_default_when_no_file() {
        // In test env, the config file almost certainly doesn't exist.
        let cfg = TuiConfig::load();
        assert_eq!(cfg, TuiConfig::default());
    }
}

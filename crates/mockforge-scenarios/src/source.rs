//! Scenario source parsing and detection
//!
//! Handles parsing and detection of scenario sources (local, URL, Git, registry)

use crate::error::{Result, ScenarioError};
use std::path::{Path, PathBuf};

/// Scenario source type
#[derive(Debug, Clone)]
pub enum ScenarioSource {
    /// Local file path or directory
    Local(PathBuf),

    /// HTTP/HTTPS URL
    Url {
        /// URL to download the scenario from
        url: String,
        /// Optional SHA-256 checksum for verification
        checksum: Option<String>,
    },

    /// Git repository
    Git {
        /// Repository URL
        url: String,
        /// Optional branch, tag, or commit
        reference: Option<String>,
        /// Optional subdirectory within the repository
        subdirectory: Option<String>,
    },

    /// Registry name
    Registry {
        /// Scenario name in the registry
        name: String,
        /// Optional version string (defaults to latest)
        version: Option<String>,
    },
}

impl ScenarioSource {
    /// Parse a scenario source from a string
    ///
    /// Automatically detects the source type:
    /// - Starts with "http://" or "https://" → URL
    /// - Contains ".git" or starts with "git@" → Git
    /// - Contains "/" or "\" → Local path
    /// - Otherwise → Registry name
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        // Check for URL
        if input.starts_with("http://") || input.starts_with("https://") {
            // Check if it's a Git repository URL
            if input.contains(".git")
                || input.contains("github.com")
                || input.contains("gitlab.com")
                || input.contains("bitbucket.org")
            {
                let (url, reference, subdirectory) = Self::parse_git_url(input)?;
                return Ok(ScenarioSource::Git {
                    url,
                    reference,
                    subdirectory,
                });
            }
            return Ok(ScenarioSource::Url {
                url: input.to_string(),
                checksum: None,
            });
        }

        // Check for SSH Git URL
        if input.starts_with("git@") {
            let (url, reference, subdirectory) = Self::parse_git_url(input)?;
            return Ok(ScenarioSource::Git {
                url,
                reference,
                subdirectory,
            });
        }

        // Check for local path
        if input.contains('/') || input.contains('\\') || Path::new(input).exists() {
            return Ok(ScenarioSource::Local(PathBuf::from(input)));
        }

        // Parse as registry reference
        let (name, version) = if let Some((n, v)) = input.split_once('@') {
            (n.to_string(), Some(v.to_string()))
        } else {
            (input.to_string(), None)
        };

        Ok(ScenarioSource::Registry { name, version })
    }

    /// Parse Git URL with optional reference and subdirectory
    ///
    /// Supports formats:
    /// - `https://github.com/user/repo`
    /// - `https://github.com/user/repo#branch`
    /// - `https://github.com/user/repo#tag`
    /// - `https://github.com/user/repo#main:scenarios/my-scenario`
    fn parse_git_url(input: &str) -> Result<(String, Option<String>, Option<String>)> {
        // Check for subdirectory syntax: url#ref:subdir
        if let Some((base, rest)) = input.split_once('#') {
            if let Some((reference, subdirectory)) = rest.split_once(':') {
                return Ok((
                    base.to_string(),
                    Some(reference.to_string()),
                    Some(subdirectory.to_string()),
                ));
            } else {
                return Ok((base.to_string(), Some(rest.to_string()), None));
            }
        }

        Ok((input.to_string(), None, None))
    }
}

impl std::fmt::Display for ScenarioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioSource::Local(path) => write!(f, "local:{}", path.display()),
            ScenarioSource::Url { url, .. } => write!(f, "url:{}", url),
            ScenarioSource::Git {
                url,
                reference,
                subdirectory,
            } => {
                write!(f, "git:{}", url)?;
                if let Some(ref ref_str) = reference {
                    write!(f, "#{}", ref_str)?;
                }
                if let Some(ref subdir) = subdirectory {
                    write!(f, ":{}", subdir)?;
                }
                Ok(())
            }
            ScenarioSource::Registry { name, version } => {
                if let Some(v) = version {
                    write!(f, "registry:{}@{}", name, v)
                } else {
                    write!(f, "registry:{}", name)
                }
            }
        }
    }
}

/// Source type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    /// Local file system
    Local,
    /// HTTP/HTTPS URL
    Url,
    /// Git repository
    Git,
    /// Registry
    Registry,
}

impl ScenarioSource {
    /// Get the source type
    pub fn source_type(&self) -> SourceType {
        match self {
            ScenarioSource::Local(_) => SourceType::Local,
            ScenarioSource::Url { .. } => SourceType::Url,
            ScenarioSource::Git { .. } => SourceType::Git,
            ScenarioSource::Registry { .. } => SourceType::Registry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_path() {
        let source = ScenarioSource::parse("./scenarios/my-scenario").unwrap();
        assert!(matches!(source, ScenarioSource::Local(_)));
    }

    #[test]
    fn test_parse_url() {
        let source = ScenarioSource::parse("https://example.com/scenario.zip").unwrap();
        assert!(matches!(source, ScenarioSource::Url { .. }));
    }

    #[test]
    fn test_parse_git_url() {
        let source = ScenarioSource::parse("https://github.com/user/repo").unwrap();
        assert!(matches!(source, ScenarioSource::Git { .. }));
    }

    #[test]
    fn test_parse_git_url_with_branch() {
        let source = ScenarioSource::parse("https://github.com/user/repo#main").unwrap();
        match source {
            ScenarioSource::Git { reference, .. } => {
                assert_eq!(reference, Some("main".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_git_url_with_subdirectory() {
        let source =
            ScenarioSource::parse("https://github.com/user/repo#main:scenarios/my-scenario")
                .unwrap();
        match source {
            ScenarioSource::Git {
                reference,
                subdirectory,
                ..
            } => {
                assert_eq!(reference, Some("main".to_string()));
                assert_eq!(subdirectory, Some("scenarios/my-scenario".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_parse_registry() {
        let source = ScenarioSource::parse("ecommerce-store").unwrap();
        assert!(matches!(source, ScenarioSource::Registry { .. }));
    }

    #[test]
    fn test_parse_registry_with_version() {
        let source = ScenarioSource::parse("ecommerce-store@1.0.0").unwrap();
        match source {
            ScenarioSource::Registry { name, version } => {
                assert_eq!(name, "ecommerce-store");
                assert_eq!(version, Some("1.0.0".to_string()));
            }
            _ => panic!("Expected Registry source"),
        }
    }
}


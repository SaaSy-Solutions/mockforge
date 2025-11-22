//! Pillar metadata system for compile-time pillar tagging
//!
//! This module provides types and utilities for tagging modules, functions, and features
//! with MockForge pillars. Pillars help organize code, enable pillar-based queries for
//! test coverage and production usage, and guide users to relevant features.
//!
//! ## The Five Pillars
//!
//! - **[Reality]** – Everything that makes mocks feel like a real, evolving backend
//! - **[Contracts]** – Schema, drift, validation, and safety nets
//! - **[DevX]** – SDKs, generators, playgrounds, ergonomics
//! - **[Cloud]** – Registry, orgs, governance, monetization, marketplace
//! - **[AI]** – LLM/voice flows, AI diff/assist, generative behaviors
//!
//! ## Usage
//!
//! Tag modules in their documentation comments:
//!
//! ```rust
//! //! Pillars: [Reality][AI]
//! //!
//! //! This module implements Smart Personas with relationship graphs
//! //! and AI-powered data generation.
//! ```
//!
//! Or use the `Pillar` enum programmatically:
//!
//! ```rust
//! use mockforge_core::pillars::{Pillar, PillarMetadata};
//!
//! let metadata = PillarMetadata::new()
//!     .with_pillar(Pillar::Reality)
//!     .with_pillar(Pillar::Ai);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// MockForge pillar identifier
///
/// Every feature in MockForge maps to one or more pillars. This structure helps:
/// - Communicate value clearly in changelogs, docs, and marketing
/// - Prioritize development based on pillar investment
/// - Maintain consistency across features and releases
/// - Guide users to the right features for their needs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Pillar {
    /// Reality pillar - everything that makes mocks feel like a real, evolving backend
    ///
    /// Key capabilities:
    /// - Realistic data generation with relationships and constraints
    /// - Stateful behavior and persistence
    /// - Network condition simulation (latency, packet loss, failures)
    /// - Time-based mutations and temporal simulation
    /// - Progressive data evolution and drift
    /// - Multi-protocol support
    Reality,
    /// Contracts pillar - schema, drift, validation, and safety nets
    ///
    /// Key capabilities:
    /// - OpenAPI/GraphQL schema validation
    /// - Request/response validation with detailed error reporting
    /// - Contract drift detection and monitoring
    /// - Automatic API sync and change detection
    /// - Schema-driven mock generation
    Contracts,
    /// DevX pillar - SDKs, generators, playgrounds, ergonomics
    ///
    /// Key capabilities:
    /// - Multi-language SDKs (Rust, Node.js, Python, Go, Java, .NET)
    /// - Client code generation (React, Vue, Angular, Svelte)
    /// - Interactive playgrounds and admin UI
    /// - CLI tooling and configuration management
    /// - Comprehensive documentation and examples
    /// - Plugin system for extensibility
    DevX,
    /// Cloud pillar - registry, orgs, governance, monetization, marketplace
    ///
    /// Key capabilities:
    /// - Organization and user management
    /// - Scenario marketplace and sharing
    /// - Registry server for mock distribution
    /// - Cloud workspaces and synchronization
    /// - Governance and access controls
    /// - Monetization infrastructure
    Cloud,
    /// AI pillar - LLM/voice flows, AI diff/assist, generative behaviors
    ///
    /// Key capabilities:
    /// - LLM-powered mock generation
    /// - AI-driven data synthesis
    /// - Voice interface for mock creation
    /// - Intelligent contract analysis
    /// - Generative data behaviors
    /// - Natural language to mock conversion
    Ai,
}

impl Pillar {
    /// Convert pillar to lowercase string representation
    ///
    /// # Examples
    ///
    /// ```
    /// use mockforge_core::pillars::Pillar;
    ///
    /// assert_eq!(Pillar::Reality.as_str(), "reality");
    /// assert_eq!(Pillar::Contracts.as_str(), "contracts");
    /// assert_eq!(Pillar::DevX.as_str(), "devx");
    /// assert_eq!(Pillar::Cloud.as_str(), "cloud");
    /// assert_eq!(Pillar::Ai.as_str(), "ai");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Pillar::Reality => "reality",
            Pillar::Contracts => "contracts",
            Pillar::DevX => "devx",
            Pillar::Cloud => "cloud",
            Pillar::Ai => "ai",
        }
    }

    /// Parse pillar from string (case-insensitive)
    ///
    /// # Examples
    ///
    /// ```
    /// use mockforge_core::pillars::Pillar;
    ///
    /// assert_eq!(Pillar::from_str("reality"), Some(Pillar::Reality));
    /// assert_eq!(Pillar::from_str("REALITY"), Some(Pillar::Reality));
    /// assert_eq!(Pillar::from_str("Contracts"), Some(Pillar::Contracts));
    /// assert_eq!(Pillar::from_str("devx"), Some(Pillar::DevX));
    /// assert_eq!(Pillar::from_str("invalid"), None);
    /// ```
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "reality" => Some(Pillar::Reality),
            "contracts" => Some(Pillar::Contracts),
            "devx" => Some(Pillar::DevX),
            "cloud" => Some(Pillar::Cloud),
            "ai" => Some(Pillar::Ai),
            _ => None,
        }
    }

    /// Get display name for the pillar (with brackets for changelog format)
    ///
    /// # Examples
    ///
    /// ```
    /// use mockforge_core::pillars::Pillar;
    ///
    /// assert_eq!(Pillar::Reality.display_name(), "[Reality]");
    /// assert_eq!(Pillar::Contracts.display_name(), "[Contracts]");
    /// ```
    pub fn display_name(&self) -> String {
        match self {
            Pillar::Reality => "[Reality]".to_string(),
            Pillar::Contracts => "[Contracts]".to_string(),
            Pillar::DevX => "[DevX]".to_string(),
            Pillar::Cloud => "[Cloud]".to_string(),
            Pillar::Ai => "[AI]".to_string(),
        }
    }

    /// Get all pillars
    ///
    /// # Examples
    ///
    /// ```
    /// use mockforge_core::pillars::Pillar;
    ///
    /// let all = Pillar::all();
    /// assert_eq!(all.len(), 5);
    /// assert!(all.contains(&Pillar::Reality));
    /// assert!(all.contains(&Pillar::Contracts));
    /// assert!(all.contains(&Pillar::DevX));
    /// assert!(all.contains(&Pillar::Cloud));
    /// assert!(all.contains(&Pillar::Ai));
    /// ```
    pub fn all() -> Vec<Pillar> {
        vec![
            Pillar::Reality,
            Pillar::Contracts,
            Pillar::DevX,
            Pillar::Cloud,
            Pillar::Ai,
        ]
    }
}

impl fmt::Display for Pillar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Pillar metadata for tagging modules, functions, or features
///
/// This type allows associating one or more pillars with code entities.
/// It's used for compile-time tagging and can be extracted from documentation
/// comments or set programmatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PillarMetadata {
    /// Set of pillars associated with this entity
    pillars: HashSet<Pillar>,
}

impl PillarMetadata {
    /// Create new empty pillar metadata
    pub fn new() -> Self {
        Self {
            pillars: HashSet::new(),
        }
    }

    /// Create pillar metadata with a single pillar
    pub fn with_pillar(mut self, pillar: Pillar) -> Self {
        self.pillars.insert(pillar);
        self
    }

    /// Create pillar metadata with multiple pillars
    pub fn with_pillars(mut self, pillars: &[Pillar]) -> Self {
        for pillar in pillars {
            self.pillars.insert(*pillar);
        }
        self
    }

    /// Add a pillar to existing metadata
    pub fn add_pillar(&mut self, pillar: Pillar) {
        self.pillars.insert(pillar);
    }

    /// Check if metadata contains a specific pillar
    pub fn has_pillar(&self, pillar: Pillar) -> bool {
        self.pillars.contains(&pillar)
    }

    /// Get all pillars in this metadata
    pub fn pillars(&self) -> Vec<Pillar> {
        let mut result: Vec<Pillar> = self.pillars.iter().copied().collect();
        // Sort for consistent output
        result.sort_by_key(|p| p.as_str());
        result
    }

    /// Check if metadata is empty
    pub fn is_empty(&self) -> bool {
        self.pillars.is_empty()
    }

    /// Format pillars as changelog-style tags (e.g., "[Reality][AI]")
    pub fn to_changelog_tags(&self) -> String {
        let mut pillars: Vec<Pillar> = self.pillars.iter().copied().collect();
        // Sort for consistent output
        pillars.sort_by_key(|p| p.as_str());
        pillars.iter().map(|p| p.display_name()).collect::<Vec<_>>().join("")
    }

    /// Parse pillar metadata from documentation comment
    ///
    /// Looks for patterns like:
    /// - `Pillars: [Reality][AI]`
    /// - `Pillar: [Reality]`
    /// - `//! Pillars: [Reality][AI]`
    ///
    /// # Examples
    ///
    /// ```
    /// use mockforge_core::pillars::{Pillar, PillarMetadata};
    ///
    /// let doc = "//! Pillars: [Reality][AI]\n//! This module does something";
    /// let metadata = PillarMetadata::from_doc_comment(doc).unwrap();
    /// assert!(metadata.has_pillar(Pillar::Reality));
    /// assert!(metadata.has_pillar(Pillar::Ai));
    /// ```
    pub fn from_doc_comment(doc: &str) -> Option<Self> {
        // Look for "Pillars:" or "Pillar:" followed by bracket notation
        let re = regex::Regex::new(r"(?i)(?:pillars?):\s*(\[[^\]]+\])+").ok()?;
        let caps = re.captures(doc)?;
        let full_match = caps.get(0)?;

        // Extract all [Pillar] tags
        let tag_re = regex::Regex::new(r"\[([^\]]+)\]").ok()?;
        let mut metadata = Self::new();

        for cap in tag_re.captures_iter(full_match.as_str()) {
            if let Some(pillar_name) = cap.get(1) {
                if let Some(pillar) = Pillar::from_str(pillar_name.as_str()) {
                    metadata.add_pillar(pillar);
                }
            }
        }

        if metadata.is_empty() {
            None
        } else {
            Some(metadata)
        }
    }
}

impl Default for PillarMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse pillar tags from a list of scenario tags
///
/// Extracts pillar tags in the format `[PillarName]` from a list of tags.
/// Pillar tags are recognized in formats like:
/// - `[Cloud]`
/// - `[Contracts]`
/// - `[Reality]`
/// - `[Cloud][Contracts][Reality]` (multiple pillars in one tag)
///
/// # Examples
///
/// ```
/// use mockforge_core::pillars::{Pillar, parse_pillar_tags_from_scenario_tags};
///
/// let tags = vec!["[Cloud]".to_string(), "auth".to_string(), "[Contracts][Reality]".to_string()];
/// let pillars = parse_pillar_tags_from_scenario_tags(&tags);
/// assert!(pillars.contains(&Pillar::Cloud));
/// assert!(pillars.contains(&Pillar::Contracts));
/// assert!(pillars.contains(&Pillar::Reality));
/// assert_eq!(pillars.len(), 3);
/// ```
pub fn parse_pillar_tags_from_scenario_tags(tags: &[String]) -> Vec<Pillar> {
    let mut pillars = Vec::new();
    let tag_re = regex::Regex::new(r"\[([^\]]+)\]").ok();

    if tag_re.is_none() {
        return pillars;
    }

    let tag_re = tag_re.unwrap();

    for tag in tags {
        // Extract all [PillarName] patterns from the tag
        for cap in tag_re.captures_iter(tag) {
            if let Some(pillar_name) = cap.get(1) {
                if let Some(pillar) = Pillar::from_str(pillar_name.as_str()) {
                    if !pillars.contains(&pillar) {
                        pillars.push(pillar);
                    }
                }
            }
        }
    }

    pillars
}

/// Check if a tag string contains pillar tags
///
/// Returns true if the tag contains at least one valid pillar tag in bracket notation.
///
/// # Examples
///
/// ```
/// use mockforge_core::pillars::has_pillar_tags;
///
/// assert!(has_pillar_tags("[Cloud]"));
/// assert!(has_pillar_tags("[Contracts][Reality]"));
/// assert!(has_pillar_tags("auth-[Cloud]-test"));
/// assert!(!has_pillar_tags("auth"));
/// assert!(!has_pillar_tags("[Invalid]"));
/// ```
pub fn has_pillar_tags(tag: &str) -> bool {
    let tag_re = regex::Regex::new(r"\[([^\]]+)\]").ok();

    if let Some(tag_re) = tag_re {
        for cap in tag_re.captures_iter(tag) {
            if let Some(pillar_name) = cap.get(1) {
                if Pillar::from_str(pillar_name.as_str()).is_some() {
                    return true;
                }
            }
        }
    }

    false
}

/// Extract pillar metadata from scenario tags
///
/// Parses all pillar tags from a list of scenario tags and returns them as PillarMetadata.
///
/// # Examples
///
/// ```
/// use mockforge_core::pillars::{Pillar, pillar_metadata_from_scenario_tags};
///
/// let tags = vec!["[Cloud]".to_string(), "[Contracts]".to_string(), "auth".to_string()];
/// let metadata = pillar_metadata_from_scenario_tags(&tags);
/// assert!(metadata.has_pillar(Pillar::Cloud));
/// assert!(metadata.has_pillar(Pillar::Contracts));
/// assert!(!metadata.has_pillar(Pillar::Reality));
/// ```
pub fn pillar_metadata_from_scenario_tags(tags: &[String]) -> PillarMetadata {
    let pillars = parse_pillar_tags_from_scenario_tags(tags);
    PillarMetadata::from(pillars)
}

impl From<Vec<Pillar>> for PillarMetadata {
    fn from(pillars: Vec<Pillar>) -> Self {
        Self {
            pillars: pillars.into_iter().collect(),
        }
    }
}

impl From<&[Pillar]> for PillarMetadata {
    fn from(pillars: &[Pillar]) -> Self {
        Self {
            pillars: pillars.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pillar_as_str() {
        assert_eq!(Pillar::Reality.as_str(), "reality");
        assert_eq!(Pillar::Contracts.as_str(), "contracts");
        assert_eq!(Pillar::DevX.as_str(), "devx");
        assert_eq!(Pillar::Cloud.as_str(), "cloud");
        assert_eq!(Pillar::Ai.as_str(), "ai");
    }

    #[test]
    fn test_pillar_from_str() {
        assert_eq!(Pillar::from_str("reality"), Some(Pillar::Reality));
        assert_eq!(Pillar::from_str("REALITY"), Some(Pillar::Reality));
        assert_eq!(Pillar::from_str("Contracts"), Some(Pillar::Contracts));
        assert_eq!(Pillar::from_str("devx"), Some(Pillar::DevX));
        assert_eq!(Pillar::from_str("invalid"), None);
    }

    #[test]
    fn test_pillar_display_name() {
        assert_eq!(Pillar::Reality.display_name(), "[Reality]");
        assert_eq!(Pillar::Contracts.display_name(), "[Contracts]");
        assert_eq!(Pillar::DevX.display_name(), "[DevX]");
        assert_eq!(Pillar::Cloud.display_name(), "[Cloud]");
        assert_eq!(Pillar::Ai.display_name(), "[AI]");
    }

    #[test]
    fn test_pillar_metadata() {
        let mut metadata = PillarMetadata::new();
        assert!(metadata.is_empty());

        metadata.add_pillar(Pillar::Reality);
        assert!(!metadata.is_empty());
        assert!(metadata.has_pillar(Pillar::Reality));
        assert!(!metadata.has_pillar(Pillar::Contracts));

        metadata.add_pillar(Pillar::Ai);
        assert!(metadata.has_pillar(Pillar::Reality));
        assert!(metadata.has_pillar(Pillar::Ai));
        assert_eq!(metadata.pillars().len(), 2);
    }

    #[test]
    fn test_pillar_metadata_builder() {
        let metadata = PillarMetadata::new().with_pillar(Pillar::Reality).with_pillar(Pillar::Ai);

        assert!(metadata.has_pillar(Pillar::Reality));
        assert!(metadata.has_pillar(Pillar::Ai));
        assert_eq!(metadata.pillars().len(), 2);
    }

    #[test]
    fn test_pillar_metadata_changelog_tags() {
        let metadata = PillarMetadata::from(vec![Pillar::Reality, Pillar::Ai]);
        let tags = metadata.to_changelog_tags();
        // Should be sorted alphabetically: [AI][Reality]
        assert!(tags.contains("[AI]"));
        assert!(tags.contains("[Reality]"));
    }

    #[test]
    fn test_pillar_metadata_from_doc_comment() {
        let doc = "//! Pillars: [Reality][AI]\n//! This module does something";
        let metadata = PillarMetadata::from_doc_comment(doc).unwrap();
        assert!(metadata.has_pillar(Pillar::Reality));
        assert!(metadata.has_pillar(Pillar::Ai));

        let doc2 = "/// Pillar: [Contracts]";
        let metadata2 = PillarMetadata::from_doc_comment(doc2).unwrap();
        assert!(metadata2.has_pillar(Pillar::Contracts));

        let doc3 = "//! No pillars here";
        assert!(PillarMetadata::from_doc_comment(doc3).is_none());
    }

    #[test]
    fn test_parse_pillar_tags_from_scenario_tags() {
        use super::{parse_pillar_tags_from_scenario_tags, Pillar};

        let tags = vec!["[Cloud]".to_string(), "auth".to_string(), "[Contracts][Reality]".to_string()];
        let pillars = parse_pillar_tags_from_scenario_tags(&tags);
        assert!(pillars.contains(&Pillar::Cloud));
        assert!(pillars.contains(&Pillar::Contracts));
        assert!(pillars.contains(&Pillar::Reality));
        assert_eq!(pillars.len(), 3);

        let tags2 = vec!["normal".to_string(), "test".to_string()];
        let pillars2 = parse_pillar_tags_from_scenario_tags(&tags2);
        assert!(pillars2.is_empty());

        let tags3 = vec!["[Cloud][Contracts][Reality]".to_string()];
        let pillars3 = parse_pillar_tags_from_scenario_tags(&tags3);
        assert_eq!(pillars3.len(), 3);
        assert!(pillars3.contains(&Pillar::Cloud));
        assert!(pillars3.contains(&Pillar::Contracts));
        assert!(pillars3.contains(&Pillar::Reality));
    }

    #[test]
    fn test_has_pillar_tags() {
        use super::has_pillar_tags;

        assert!(has_pillar_tags("[Cloud]"));
        assert!(has_pillar_tags("[Contracts][Reality]"));
        assert!(has_pillar_tags("auth-[Cloud]-test"));
        assert!(!has_pillar_tags("auth"));
        assert!(!has_pillar_tags("[Invalid]"));
        assert!(!has_pillar_tags(""));
    }

    #[test]
    fn test_pillar_metadata_from_scenario_tags() {
        use super::{pillar_metadata_from_scenario_tags, Pillar};

        let tags = vec!["[Cloud]".to_string(), "[Contracts]".to_string(), "auth".to_string()];
        let metadata = pillar_metadata_from_scenario_tags(&tags);
        assert!(metadata.has_pillar(Pillar::Cloud));
        assert!(metadata.has_pillar(Pillar::Contracts));
        assert!(!metadata.has_pillar(Pillar::Reality));

        let tags2 = vec!["[Cloud][Contracts][Reality]".to_string()];
        let metadata2 = pillar_metadata_from_scenario_tags(&tags2);
        assert!(metadata2.has_pillar(Pillar::Cloud));
        assert!(metadata2.has_pillar(Pillar::Contracts));
        assert!(metadata2.has_pillar(Pillar::Reality));
    }
}

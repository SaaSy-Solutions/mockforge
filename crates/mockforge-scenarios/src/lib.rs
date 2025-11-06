//! # MockForge Scenarios Marketplace
//!
//! A marketplace system for sharing and importing complete mock system configurations.
//! Users can discover, install, and use pre-built scenarios that include configuration
//! files, fixtures, OpenAPI specs, and example data.
//!
//! ## Overview
//!
//! The scenarios marketplace allows users to:
//!
//! - Discover community-built mock scenarios
//! - Install scenarios from various sources (local, URL, Git, registry)
//! - Apply scenarios to their workspace with one command
//! - Share their own scenarios with the community
//!
//! ## Example Scenarios
//!
//! - **E-commerce Store**: Complete e-commerce API with shopping carts, products, and orders
//! - **Chat API**: Real-time chat API with typing indicators and message history
//! - **Weather + Geolocation**: Weather service with geolocation-based queries
//!
//! ## Quick Start
//!
//! ```bash
//! # Search for scenarios
//! mockforge scenario search ecommerce
//!
//! # Install a scenario
//! mockforge scenario install ecommerce-store
//!
//! # Apply scenario to current workspace
//! mockforge scenario use ecommerce-store
//! ```
//!
//! ## Scenario Format
//!
//! Scenarios are packaged as directories containing:
//!
//! - `scenario.yaml` - Scenario manifest with metadata
//! - `config.yaml` - MockForge configuration
//! - `openapi.json` - OpenAPI specification (optional)
//! - `fixtures/` - Protocol-specific fixtures
//! - `examples/` - Example data files
//! - `README.md` - Documentation

pub mod error;
pub mod manifest;
pub mod package;
pub mod installer;
pub mod registry;
pub mod source;
pub mod storage;

// Re-export commonly used types
pub use error::{ScenarioError, Result};
pub use manifest::{ScenarioManifest, ScenarioCategory, CompatibilityInfo, PluginDependency};
pub use package::{ScenarioPackage, PackageValidation};
pub use installer::{ScenarioInstaller, InstallOptions};
pub use registry::{ScenarioRegistry, RegistryClient, ScenarioRegistryEntry, ScenarioSearchQuery, ScenarioSearchResults, ScenarioSortOrder, ScenarioPublishRequest, ScenarioPublishResponse};
pub use source::{ScenarioSource, SourceType};
pub use storage::{ScenarioStorage, InstalledScenario};

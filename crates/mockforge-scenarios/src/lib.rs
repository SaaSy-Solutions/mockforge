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
pub mod installer;
pub mod manifest;
pub mod package;
pub mod registry;
pub mod source;
pub mod storage;

// Re-export commonly used types
pub use error::{Result, ScenarioError};
pub use installer::{InstallOptions, ScenarioInstaller};
pub use manifest::{CompatibilityInfo, PluginDependency, ScenarioCategory, ScenarioManifest};
pub use package::{PackageValidation, ScenarioPackage};
pub use registry::{
    RegistryClient, ScenarioPublishRequest, ScenarioPublishResponse, ScenarioRegistry,
    ScenarioRegistryEntry, ScenarioSearchQuery, ScenarioSearchResults, ScenarioSortOrder,
};
pub use source::{ScenarioSource, SourceType};
pub use storage::{InstalledScenario, ScenarioStorage};

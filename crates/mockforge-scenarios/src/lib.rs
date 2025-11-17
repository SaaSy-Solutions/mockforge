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

pub mod domain_pack;
pub mod error;
pub mod installer;
pub mod manifest;
pub mod mockai_integration;
pub mod package;
pub mod preview;
pub mod registry;
pub mod schema_alignment;
pub mod source;
pub mod state_machine;
pub mod storage;
pub mod studio_pack;
pub mod vbr_integration;

// Re-export commonly used types
pub use domain_pack::{
    DomainPackInfo, DomainPackInstaller, DomainPackManifest, FieldRealityRule, PackScenario,
    StudioChaosRule, StudioContractDiff, StudioPersona, StudioRealityBlend,
};
pub use error::{Result, ScenarioError};
pub use installer::{InstallOptions, ScenarioInstaller};
pub use manifest::{CompatibilityInfo, PluginDependency, ScenarioCategory, ScenarioManifest};
pub use mockai_integration::{
    apply_mockai_config, MockAIConfigDefinition, MockAIIntegrationConfig, MockAIMergeMode,
};
pub use package::{PackageValidation, ScenarioPackage};
pub use preview::{CompatibilityCheck, OpenApiEndpoint, ScenarioPreview};
pub use registry::{
    RegistryClient, ScenarioPublishRequest, ScenarioPublishResponse, ScenarioRegistry,
    ScenarioRegistryEntry, ScenarioReview, ScenarioReviewSubmission, ScenarioSearchQuery,
    ScenarioSearchResults, ScenarioSortOrder,
};
pub use schema_alignment::{
    align_openapi_specs, align_vbr_entities, ConflictType, MergeStrategy, OpenApiAlignmentResult,
    SchemaAlignmentConfig, SchemaConflict,
};
pub use source::{ScenarioSource, SourceType};
pub use state_machine::{ScenarioStateMachineManager, StateHistoryEntry, StateInstance};
pub use storage::{InstalledScenario, ScenarioStorage};
#[cfg(feature = "studio-packs")]
pub use studio_pack::packs::{
    create_ecommerce_peak_day_pack, create_fintech_fraud_lab_pack,
    create_healthcare_outage_drill_pack,
};
pub use studio_pack::{StudioPackInstallResult, StudioPackInstaller};
pub use vbr_integration::{
    apply_vbr_entities, VbrEntityDefinition, VbrIntegrationConfig, VbrMergeMode,
};

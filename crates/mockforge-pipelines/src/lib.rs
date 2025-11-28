//! # `MockForge` Pipelines
//!
//! Event-driven pipeline orchestration for `MockForge`.
//!
//! This crate provides a GitHub Actions-like pipeline system for automating
//! mock lifecycle management, including:
//!
//! - Schema change detection → auto-regenerate SDKs
//! - Scenario publication → auto-promote to test → notify teams
//! - Drift threshold exceeded → auto-generate Git PRs
//!
//! ## Overview
//!
//! Pipelines are defined in YAML and triggered by events. Each pipeline consists
//! of steps that execute sequentially or in parallel. Steps can be:
//!
//! - `regenerate_sdk` - Regenerate client SDKs for specified languages
//! - `auto_promote` - Automatically promote scenarios/personas/configs
//! - `notify` - Send notifications to teams (Slack, email, webhooks)
//! - `create_pr` - Create Git pull requests
//!
//! ## Example Pipeline
//!
//! ```yaml
//! name: schema-change-pipeline
//! triggers:
//!   - event: schema.changed
//!     filters:
//!       workspace_id: "workspace-123"
//!       schema_type: ["openapi", "protobuf"]
//!
//! steps:
//!   - name: regenerate-sdks
//!     type: regenerate_sdk
//!     config:
//!       languages: ["typescript", "python", "rust"]
//!       workspace_id: "{{workspace_id}}"
//!
//!   - name: notify-teams
//!     type: notify
//!     config:
//!       channels: ["#api-team", "#frontend-team"]
//!       message: "SDKs regenerated for {{workspace_id}}"
//! ```
//!
//! ## Event System
//!
//! Events are emitted by various `MockForge` components and trigger pipelines:
//!
//! - `schema.changed` - OpenAPI/Protobuf schema modified
//! - `scenario.published` - New scenario published
//! - `drift.threshold_exceeded` - Drift budget exceeded
//! - `promotion.completed` - Promotion completed
//! - `workspace.created` - New workspace created

pub mod events;
pub mod pipeline;
pub mod steps;

pub use events::{publish_event, PipelineEvent, PipelineEventBus, PipelineEventType};
pub use pipeline::{Pipeline, PipelineDefinition, PipelineExecutor, PipelineStep};
pub use steps::{PipelineStepExecutor, StepContext, StepResult};

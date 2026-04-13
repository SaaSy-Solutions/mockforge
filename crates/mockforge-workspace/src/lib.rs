//! Workspace management for MockForge
//!
//! This crate contains workspace-related functionality extracted from `mockforge-core`.
//! Currently includes:
//! - `git_watch`: Git repository watching and polling for spec/fixture sync
//!
//! More modules (workspace, workspace_persistence, sync_watcher, encryption, multi_tenant)
//! will be migrated here in future work once their dependencies on other core modules
//! (snapshots, templating) are resolved.

pub mod git_watch;

pub use git_watch::{GitWatchConfig, GitWatchService};

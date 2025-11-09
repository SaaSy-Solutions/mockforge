//! # MockForge Collaboration
//!
//! Cloud collaboration features for MockForge including team workspaces,
//! real-time synchronization, version control, and role-based access control.
//!
//! ## Features
//!
//! - **Team Workspaces**: Shared environments for collaborative mock development
//! - **Real-time Sync**: WebSocket-based synchronization across team members
//! - **Role-Based Access Control**: Admin, Editor, and Viewer roles
//! - **Version Control**: Git-style history and versioned snapshots
//! - **Self-Hosted Option**: Run your own team collaboration server
//! - **Conflict Resolution**: Intelligent merging of concurrent changes
//!
//! ## Quick Start
//!
//! ### Creating a Collaborative Workspace
//!
//! ```rust,no_run
//! use mockforge_collab::{
//!     CollabServer, CollabConfig, TeamWorkspace, UserRole,
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create collaboration server
//!     let config = CollabConfig::default();
//!     let server = CollabServer::new(config).await?;
//!
//!     // Start the server
//!     server.run("127.0.0.1:8080").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Connecting to a Collaborative Workspace
//!
//! ```rust,no_run
//! use mockforge_collab::{CollabClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ClientConfig {
//!         server_url: "ws://localhost:8080".to_string(),
//!         auth_token: "your-token".to_string(),
//!     };
//!
//!     let client = CollabClient::connect(config).await?;
//!
//!     // Subscribe to workspace changes
//!     client.subscribe_to_workspace("workspace-id").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod auth;
pub mod backup;
pub mod client;
pub mod config;
pub mod conflict;
pub mod core_bridge;
pub mod error;
pub mod events;
pub mod history;
pub mod merge;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod server;
pub mod sync;
pub mod user;
pub mod websocket;
pub mod workspace;

pub use auth::{AuthService, Credentials, Session, Token};
pub use backup::{BackupService, StorageBackend, WorkspaceBackup};
pub use client::{ClientConfig, CollabClient, ConnectionState};
pub use config::CollabConfig;
pub use conflict::{ConflictResolution, ConflictResolver, MergeStrategy};
pub use core_bridge::CoreBridge;
pub use error::{CollabError, Result};
pub use events::{ChangeEvent, ChangeType, EventBus, EventListener};
pub use history::{Commit, History, Snapshot, VersionControl};
pub use merge::MergeService;
pub use models::{TeamWorkspace, User, UserRole, WorkspaceMember};
pub use permissions::{Permission, PermissionChecker, RolePermissions};
pub use server::CollabServer;
pub use sync::{SyncEngine, SyncMessage, SyncState};
pub use workspace::{WorkspaceManager, WorkspaceService};

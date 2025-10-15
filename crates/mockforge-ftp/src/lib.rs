//! # MockForge FTP
//!
//! FTP protocol support for MockForge.
//!
//! This crate provides FTP-specific functionality for creating mock FTP servers,
//! including virtual file systems, fixture-driven responses, and file transfer simulation.
//!
//! ## Overview
//!
//! MockForge FTP enables you to:
//!
//! - **Serve FTP servers**: Mock FTP protocol for file transfer testing
//! - **Virtual file system**: In-memory and template-based file generation
//! - **Fixture management**: Pre-configured file structures and content
//! - **Upload handling**: Configurable upload validation and storage
//! - **Protocol compliance**: Standard FTP commands and responses
//!
//! ## Quick Start
//!
//! ### Basic FTP Server
//!
//! ```rust,no_run
//! use mockforge_ftp::FtpServer;
//! use mockforge_core::config::FtpConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = FtpConfig::default();
//!     let server = FtpServer::new(config).await?;
//!
//!     server.start().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Key Features
//!
//! ### Virtual File System
//! - In-memory file storage with metadata
//! - Template-based content generation
//! - Pattern-based file creation (random, zeros, incremental)
//!
//! ### Fixture System
//! - YAML-based fixture definitions
//! - Upload rules and validation
//! - Multiple storage options (memory, file, discard)
//!
//! ### FTP Protocol Support
//! - Standard FTP commands (LIST, RETR, STOR, DELE, etc.)
//! - Passive and active mode support
//! - Authentication and anonymous access
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
//! - [`libunftp`](https://docs.rs/libunftp): Underlying FTP server library

pub mod server;
pub mod vfs;
pub mod fixtures;
pub mod spec_registry;
pub mod commands;
pub mod storage;

// Re-export main types
pub use server::FtpServer;
pub use vfs::{VirtualFileSystem, VirtualFile, FileContent, FileMetadata, GenerationPattern};
pub use fixtures::{FtpFixture, VirtualFileConfig, UploadRule, UploadStorage, FileValidation};
pub use spec_registry::FtpSpecRegistry;

//! Encryption module for MockForge Security Core
//!
//! This module provides encryption algorithms, key management, key derivation,
//! key rotation, auto-encryption policies, and error handling.

pub mod algorithms;
pub mod auto_encryption;
pub mod derivation;
pub mod errors;
pub mod key_management;
pub mod key_rotation;

pub use errors::*;
pub use key_management::{FileKeyStorage, KeyStorage, KeyStore as KeyManagementStore};

//! Core types for contract diff analysis
//!
//! These types are re-exported from `mockforge-foundation::contract_diff_types`
//! so the canonical definitions live at the bottom of the dep graph and both
//! `mockforge-core` and `mockforge-contracts` use the same underlying types.

pub use mockforge_foundation::contract_diff_types::{
    CapturedRequest, ConfidenceLevel, ContractDiffConfig, ContractDiffResult, CorrectionProposal,
    DiffMetadata, Mismatch, MismatchSeverity, MismatchType, PatchOperation, Recommendation,
};

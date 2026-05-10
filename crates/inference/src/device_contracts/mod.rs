//! Canonical device and runtime-variant contracts for execution planning.
//!
//! This module is pure DTO and validation code. Backend adapters can translate
//! these contracts into backend-local flags after the scheduler has selected a
//! concrete decision, but invalid raw device strings must be rejected here
//! instead of becoming executable defaults.

mod ids;
mod planning;

#[cfg(test)]
mod tests;

pub use ids::{BackendId, InferenceDeviceId, RuntimeVariantId};
pub use planning::{
    BackendExecutionCandidate, BackendExecutionDecision, BackendObservedThroughputHint,
    BackendResourceEstimate, DeviceResolutionDecision, DeviceResolutionDiagnostic,
    DeviceResolutionDiagnosticCode, DeviceResolutionDiagnosticSeverity, DeviceResolutionRequest,
    InferenceDeviceClass, InferenceDevicePolicy, RuntimeVariantCapability,
};
use thiserror::Error;

/// Device/runtime contract validation failure.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceContractError {
    /// A required identifier was empty after trimming.
    #[error("{field} must not be empty")]
    EmptyIdentifier {
        /// Contract field that failed validation.
        field: &'static str,
    },
    /// An identifier exceeded its bounded wire-contract length.
    #[error("{field} must be at most {max_len} bytes, got {actual_len}")]
    IdentifierTooLong {
        /// Contract field that failed validation.
        field: &'static str,
        /// Maximum accepted byte length.
        max_len: usize,
        /// Actual byte length.
        actual_len: usize,
    },
    /// An identifier did not match the canonical lowercase identifier shape.
    #[error("{field} has invalid identifier shape: {value}")]
    InvalidIdentifier {
        /// Contract field that failed validation.
        field: &'static str,
        /// Invalid value.
        value: String,
    },
    /// No backend candidates were available for a scheduler decision.
    #[error("backend execution decision requires one candidate, got none")]
    EmptyBackendCandidates,
    /// More than one candidate was supplied where one selected choice is required.
    #[error("backend execution decision requires one candidate, got {count}")]
    AmbiguousBackendCandidates {
        /// Number of candidates supplied.
        count: usize,
    },
}

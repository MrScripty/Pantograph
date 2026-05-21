use serde::{Deserialize, Serialize};

/// Typed dependency-environment action requested by graph or frontend callers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentAction {
    Resolve,
    Check,
    Install,
}

/// Dependency-environment readiness state reported after resolve/check/install.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentReadinessState {
    Unknown,
    Resolved,
    Ready,
    Missing,
    Unavailable,
    Invalid,
    Failed,
    NotImplemented,
}

/// Dependency-environment install state reported by host dependency actions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentInstallState {
    NotRequested,
    NotInstalled,
    Installing,
    Installed,
    Failed,
    Blocked,
    NotImplemented,
}

/// Validation state for dependency-environment contracts and resolved facts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentValidationState {
    Valid,
    Invalid,
    Stale,
    Unavailable,
    NotImplemented,
}

/// High-level failure state for dependency-environment results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentFailureState {
    InvalidRequest,
    RequirementsUnavailable,
    EnvironmentUnavailable,
    CheckFailed,
    InstallFailed,
    NotImplemented,
    InternalError,
}

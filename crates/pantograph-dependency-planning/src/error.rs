use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyPlanningContractError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("ready dependency planning results require a load target")]
    ReadyResultMissingLoadTarget,
    #[error("dependency planning result state is {state}, but a load target was provided")]
    NonReadyResultHasLoadTarget { state: &'static str },
    #[error("failed to serialize {field} for canonical dependency planning identity")]
    CanonicalSerializationFailed { field: &'static str },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PumasArtifactEntryPathError {
    #[error("Pumas artifact entry path is required")]
    Missing,
    #[error("Pumas artifact entry path exceeds maximum length {max_len}")]
    TooLong { max_len: usize },
    #[error("Pumas artifact entry path contains invalid characters")]
    InvalidCharacters,
    #[error("Pumas artifact entry path must not use a URI scheme")]
    UnsupportedUri,
    #[error("Pumas artifact entry path must be root-relative and must not traverse roots")]
    LocalPath,
    #[error("Pumas artifact entry path contains an invalid segment")]
    InvalidSegment,
}

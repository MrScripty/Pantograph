use thiserror::Error;

/// Public scheduler contract validation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerContractError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
}

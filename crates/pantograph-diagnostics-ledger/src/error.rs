use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiagnosticsLedgerError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} is too long")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains control characters")]
    InvalidField { field: &'static str },
    #[error("query time range is invalid")]
    InvalidTimeRange,
    #[error("query page size {requested} exceeds maximum {max}")]
    QueryLimitExceeded { requested: u32, max: u32 },
    #[error("unsupported diagnostics ledger schema version {found}")]
    UnsupportedSchemaVersion { found: i64 },
    #[error("diagnostics ledger storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("diagnostics ledger serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

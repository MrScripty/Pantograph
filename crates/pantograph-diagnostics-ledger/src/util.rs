use chrono::Utc;

use crate::DiagnosticsLedgerError;

pub(crate) const MAX_ID_LEN: usize = 128;
pub(crate) const MAX_JSON_LEN: usize = 65_536;

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(crate) fn validate_required_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), DiagnosticsLedgerError> {
    validate_optional_text(field, Some(value), max_len)?;
    if value.trim().is_empty() {
        return Err(DiagnosticsLedgerError::MissingField { field });
    }
    Ok(())
}

pub(crate) fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), DiagnosticsLedgerError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > max_len {
        return Err(DiagnosticsLedgerError::FieldTooLong { field, max_len });
    }
    if value.chars().any(char::is_control) {
        return Err(DiagnosticsLedgerError::InvalidField { field });
    }
    Ok(())
}

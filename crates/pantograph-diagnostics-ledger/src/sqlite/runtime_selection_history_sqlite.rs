use rusqlite::{params, Row};

use super::SqliteDiagnosticsLedger;
use crate::runtime_selection_history::{
    RuntimeSelectionHistoryQuery, RuntimeSelectionHistoryRunStatus, RuntimeSelectionHistorySample,
    RuntimeSelectionHistorySummary,
};
use crate::DiagnosticsLedgerError;

pub(super) fn runtime_selection_history_summary(
    ledger: &SqliteDiagnosticsLedger,
    query: RuntimeSelectionHistoryQuery,
) -> Result<RuntimeSelectionHistorySummary, DiagnosticsLedgerError> {
    query.validate()?;
    let mut stmt = ledger.conn.prepare(
        "SELECT status, duration_ms, accepted_at_ms, started_at_ms
         FROM run_list_projection
         WHERE workflow_id = ?1
           AND selected_task_id = ?2
           AND selected_model_id = ?3
           AND selected_backend_key = ?4
           AND selected_runtime_variant_id = ?5
           AND selected_device_class = ?6
           AND ((?7 IS NULL AND selected_device_id IS NULL) OR selected_device_id = ?7)
           AND status IN ('completed', 'failed', 'cancelled')
         ORDER BY completed_at_ms DESC, last_event_seq DESC
         LIMIT ?8",
    )?;
    let rows = stmt.query_map(
        params![
            query.key.workflow_id.as_str(),
            query.key.task_id.as_str(),
            query.key.model_id.as_str(),
            query.key.selected_backend_key.as_str(),
            query.key.selected_runtime_variant_id.as_str(),
            query.key.selected_device_class.as_str(),
            query.key.selected_device_id.as_deref(),
            query.sample_limit,
        ],
        runtime_selection_history_sample_from_row,
    )?;
    let samples = rows.collect::<Result<Vec<_>, _>>()?;
    RuntimeSelectionHistorySummary::from_samples(&query, samples)
}

fn runtime_selection_history_sample_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<RuntimeSelectionHistorySample> {
    Ok(RuntimeSelectionHistorySample {
        status: runtime_selection_history_status_from_db(&row.get::<_, String>(0)?)?,
        duration_ms: optional_i64_to_u64("duration_ms", row.get(1)?)?,
        accepted_at_ms: row.get(2)?,
        started_at_ms: row.get(3)?,
    })
}

fn optional_i64_to_u64(field: &'static str, value: Option<i64>) -> rusqlite::Result<Option<u64>> {
    value
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(DiagnosticsLedgerError::InvalidField { field }),
                )
            })
        })
        .transpose()
}

fn runtime_selection_history_status_from_db(
    value: &str,
) -> rusqlite::Result<RuntimeSelectionHistoryRunStatus> {
    match value {
        "completed" => Ok(RuntimeSelectionHistoryRunStatus::Completed),
        "failed" => Ok(RuntimeSelectionHistoryRunStatus::Failed),
        "cancelled" => Ok(RuntimeSelectionHistoryRunStatus::Cancelled),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(DiagnosticsLedgerError::InvalidField {
                field: "runtime_selection_history_status",
            }),
        )),
    }
}

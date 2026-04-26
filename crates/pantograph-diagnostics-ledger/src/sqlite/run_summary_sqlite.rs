use rusqlite::{params, Row};

use super::SqliteDiagnosticsLedger;
use crate::timing::{
    WorkflowRunSummaryProjection, WorkflowRunSummaryQuery, WorkflowRunSummaryRecord,
    WorkflowRunSummaryStatus,
};
use crate::DiagnosticsLedgerError;

pub(super) fn upsert_workflow_run_summary(
    ledger: &mut SqliteDiagnosticsLedger,
    record: WorkflowRunSummaryRecord,
) -> Result<(), DiagnosticsLedgerError> {
    record.validate()?;
    ledger.conn.execute(
        "INSERT INTO workflow_run_summaries
            (workflow_run_id, workflow_id, session_id, graph_fingerprint, status,
             started_at_ms, ended_at_ms, duration_ms, node_count_at_start, event_count,
             last_error, recorded_at_ms)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(workflow_run_id) DO UPDATE SET
            workflow_id = excluded.workflow_id,
            session_id = excluded.session_id,
            graph_fingerprint = excluded.graph_fingerprint,
            status = excluded.status,
            started_at_ms = excluded.started_at_ms,
            ended_at_ms = excluded.ended_at_ms,
            duration_ms = excluded.duration_ms,
            node_count_at_start = excluded.node_count_at_start,
            event_count = excluded.event_count,
            last_error = excluded.last_error,
            recorded_at_ms = excluded.recorded_at_ms",
        params![
            record.workflow_run_id.as_str(),
            record.workflow_id.as_str(),
            record.session_id.as_deref(),
            record.graph_fingerprint.as_deref(),
            record.status.as_db(),
            record.started_at_ms,
            record.ended_at_ms,
            record.duration_ms.map(|duration| duration as i64),
            record.node_count_at_start as i64,
            record.event_count as i64,
            record.last_error.as_deref(),
            record.recorded_at_ms,
        ],
    )?;
    Ok(())
}

pub(super) fn query_workflow_run_summaries(
    ledger: &SqliteDiagnosticsLedger,
    query: WorkflowRunSummaryQuery,
) -> Result<WorkflowRunSummaryProjection, DiagnosticsLedgerError> {
    query.validate()?;
    let mut stmt = ledger.conn.prepare(
        "SELECT workflow_run_id, workflow_id, session_id, graph_fingerprint, status,
                started_at_ms, ended_at_ms, duration_ms, node_count_at_start,
                event_count, last_error, recorded_at_ms
         FROM workflow_run_summaries
         WHERE (?1 IS NULL OR workflow_id = ?1)
           AND (?2 IS NULL OR workflow_run_id = ?2)
         ORDER BY started_at_ms DESC
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_id.as_deref(),
            query.workflow_run_id.as_deref(),
            query.limit as i64,
        ],
        workflow_run_summary_from_row,
    )?;
    let mut runs = Vec::new();
    for row in rows {
        runs.push(row?);
    }
    Ok(WorkflowRunSummaryProjection { runs })
}

fn workflow_run_summary_from_row(
    row: &Row<'_>,
) -> Result<WorkflowRunSummaryRecord, rusqlite::Error> {
    let status: String = row.get(4)?;
    let duration_ms = row
        .get::<_, Option<i64>>(7)?
        .map(|duration| duration as u64);
    let node_count_at_start = row.get::<_, i64>(8)?.max(0) as usize;
    let event_count = row.get::<_, i64>(9)?.max(0) as usize;
    Ok(WorkflowRunSummaryRecord {
        workflow_run_id: row.get(0)?,
        workflow_id: row.get(1)?,
        session_id: row.get(2)?,
        graph_fingerprint: row.get(3)?,
        status: WorkflowRunSummaryStatus::from_db(&status).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        started_at_ms: row.get(5)?,
        ended_at_ms: row.get(6)?,
        duration_ms,
        node_count_at_start,
        event_count,
        last_error: row.get(10)?,
        recorded_at_ms: row.get(11)?,
    })
}

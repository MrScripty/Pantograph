use rusqlite::params;

use super::SqliteDiagnosticsLedger;
use crate::timing::{
    PruneTimingObservationsCommand, PruneTimingObservationsResult, WorkflowTimingExpectation,
    WorkflowTimingExpectationQuery, WorkflowTimingObservation, WorkflowTimingObservationStatus,
    MIN_TIMING_EXPECTATION_SAMPLE_COUNT,
};
use crate::util::{validate_required_text, MAX_ID_LEN};
use crate::DiagnosticsLedgerError;

pub(super) fn record_timing_observation(
    ledger: &mut SqliteDiagnosticsLedger,
    observation: WorkflowTimingObservation,
) -> Result<(), DiagnosticsLedgerError> {
    observation.validate()?;
    ledger.conn.execute(
        "INSERT OR IGNORE INTO workflow_timing_observations
            (observation_key, observation_scope, workflow_run_id, workflow_id,
             graph_fingerprint, node_id, node_type, runtime_id, status, started_at_ms,
             ended_at_ms, duration_ms, recorded_at_ms)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            observation.observation_key.as_str(),
            observation.scope.as_db(),
            observation.workflow_run_id.as_str(),
            observation.workflow_id.as_str(),
            observation.graph_fingerprint.as_str(),
            observation.node_id.as_deref(),
            observation.node_type.as_deref(),
            observation.runtime_id.as_deref(),
            observation.status.as_db(),
            observation.started_at_ms,
            observation.ended_at_ms,
            observation.duration_ms as i64,
            observation.recorded_at_ms,
        ],
    )?;
    Ok(())
}

pub(super) fn workflow_ids_for_timing_graph_fingerprint(
    ledger: &SqliteDiagnosticsLedger,
    graph_fingerprint: &str,
) -> Result<Vec<String>, DiagnosticsLedgerError> {
    validate_required_text("graph_fingerprint", graph_fingerprint, MAX_ID_LEN)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT DISTINCT workflow_id
         FROM workflow_timing_observations
         WHERE graph_fingerprint = ?1
         ORDER BY workflow_id",
    )?;
    let rows = stmt.query_map(params![graph_fingerprint], |row| row.get::<_, String>(0))?;
    let mut workflow_ids = Vec::new();
    for row in rows {
        workflow_ids.push(row?);
    }
    Ok(workflow_ids)
}

pub(super) fn timing_expectation(
    ledger: &SqliteDiagnosticsLedger,
    query: WorkflowTimingExpectationQuery,
) -> Result<WorkflowTimingExpectation, DiagnosticsLedgerError> {
    query.validate()?;
    let mut durations_ms = query_completed_durations(ledger, &query, true)?;
    if query.runtime_id.is_some() && durations_ms.len() < MIN_TIMING_EXPECTATION_SAMPLE_COUNT {
        durations_ms = query_completed_durations(ledger, &query, false)?;
    }
    Ok(WorkflowTimingExpectation::from_completed_durations(
        &query,
        durations_ms,
    ))
}

fn query_completed_durations(
    ledger: &SqliteDiagnosticsLedger,
    query: &WorkflowTimingExpectationQuery,
    refine_runtime: bool,
) -> Result<Vec<u64>, DiagnosticsLedgerError> {
    let mut stmt = ledger.conn.prepare(
        "SELECT duration_ms
         FROM workflow_timing_observations
         WHERE observation_scope = ?1
           AND workflow_id = ?2
           AND graph_fingerprint = ?3
           AND (?4 IS NULL OR node_id = ?4)
           AND (?5 IS NULL OR node_type = ?5 OR node_type IS NULL)
           AND (?6 IS NULL OR runtime_id = ?6 OR runtime_id IS NULL)
           AND status = ?7
         ORDER BY recorded_at_ms DESC",
    )?;
    let runtime_id = refine_runtime
        .then_some(query.runtime_id.as_deref())
        .flatten();
    let rows = stmt.query_map(
        params![
            query.scope.as_db(),
            query.workflow_id.as_str(),
            query.graph_fingerprint.as_str(),
            query.node_id.as_deref(),
            query.node_type.as_deref(),
            runtime_id,
            WorkflowTimingObservationStatus::Completed.as_db(),
        ],
        |row| row.get::<_, i64>(0),
    )?;
    let mut durations_ms = Vec::new();
    for row in rows {
        let duration_ms = row?;
        if duration_ms >= 0 {
            durations_ms.push(duration_ms as u64);
        }
    }
    Ok(durations_ms)
}

pub(super) fn prune_timing_observations(
    ledger: &mut SqliteDiagnosticsLedger,
    command: PruneTimingObservationsCommand,
) -> Result<PruneTimingObservationsResult, DiagnosticsLedgerError> {
    let tx = ledger.conn.transaction()?;
    let count = tx.query_row(
        "SELECT COUNT(*)
         FROM workflow_timing_observations
         WHERE recorded_at_ms < ?1",
        params![command.prune_recorded_before_ms],
        |row| row.get::<_, i64>(0),
    )? as u64;
    tx.execute(
        "DELETE FROM workflow_timing_observations
         WHERE recorded_at_ms < ?1",
        params![command.prune_recorded_before_ms],
    )?;
    tx.commit()?;
    Ok(PruneTimingObservationsResult {
        pruned_observation_count: count,
        prune_recorded_before_ms: command.prune_recorded_before_ms,
    })
}

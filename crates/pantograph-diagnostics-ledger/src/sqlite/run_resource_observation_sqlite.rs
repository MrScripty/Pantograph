use rusqlite::params;

use super::SqliteDiagnosticsLedger;
use crate::{
    DiagnosticEventKind, DiagnosticEventPayload, DiagnosticsLedgerError, RunMemoryFailureKind,
    RunResourceObservation, RunResourceObservationRollupQuery,
};

pub(super) fn run_resource_observation_rollup(
    ledger: &SqliteDiagnosticsLedger,
    query: RunResourceObservationRollupQuery,
) -> Result<Option<RunResourceObservation>, DiagnosticsLedgerError> {
    let mut stmt = ledger.conn.prepare(
        "SELECT payload_json
         FROM diagnostic_events
         WHERE workflow_run_id = ?1
           AND event_kind = ?2
         ORDER BY event_seq ASC",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_str(),
            DiagnosticEventKind::InferenceExecutionDiagnosticObserved.as_db(),
        ],
        |row| row.get::<_, String>(0),
    )?;

    let mut rollup = RunResourceObservationRollup::default();
    for row in rows {
        let payload_json = row?;
        let payload: DiagnosticEventPayload = serde_json::from_str(&payload_json)?;
        let DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) = payload else {
            continue;
        };
        if let Some(observation) = payload.resource_observation.as_ref() {
            rollup.observe(observation);
        }
    }

    Ok(rollup.into_observation())
}

#[derive(Default)]
struct RunResourceObservationRollup {
    peak_ram_bytes: Option<u64>,
    peak_vram_bytes: Option<u64>,
    memory_failure_kind: Option<RunMemoryFailureKind>,
}

impl RunResourceObservationRollup {
    fn observe(&mut self, observation: &crate::InferenceResourceObservationDiagnosticSummary) {
        self.peak_ram_bytes = max_optional_u64(self.peak_ram_bytes, observation.peak_ram_bytes);
        self.peak_vram_bytes = max_optional_u64(self.peak_vram_bytes, observation.peak_vram_bytes);
        if observation.memory_failure_kind == Some(RunMemoryFailureKind::OutOfMemory) {
            self.memory_failure_kind = Some(RunMemoryFailureKind::OutOfMemory);
        }
    }

    fn into_observation(self) -> Option<RunResourceObservation> {
        if self.peak_ram_bytes.is_none()
            && self.peak_vram_bytes.is_none()
            && self.memory_failure_kind.is_none()
        {
            return None;
        }

        Some(RunResourceObservation {
            peak_ram_bytes: self.peak_ram_bytes,
            peak_vram_bytes: self.peak_vram_bytes,
            memory_failure_kind: self.memory_failure_kind,
        })
    }
}

fn max_optional_u64(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

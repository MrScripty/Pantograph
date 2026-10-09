use rusqlite::{params, OptionalExtension};

use super::SqliteDiagnosticsLedger;
use crate::{
    DiagnosticsLedgerError, RuntimeHostObservationQuery, RuntimeHostObservationSummary,
    RuntimeHostRequestObservation,
};

pub(super) fn record(
    ledger: &mut SqliteDiagnosticsLedger,
    observation: RuntimeHostRequestObservation,
) -> Result<(), DiagnosticsLedgerError> {
    observation.validate()?;
    let payload = serde_json::to_string(&observation)?;
    let tx = ledger
        .conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT payload_json FROM runtime_host_request_observations WHERE observation_id = ?1",
            [&observation.observation_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        if existing != payload {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "observation_id_conflict",
            });
        }
    } else {
        tx.execute(
            "INSERT INTO runtime_host_request_observations
                (observation_id, host_epoch, request_fingerprint, recorded_at_ms, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                observation.observation_id,
                observation.profile.host_epoch,
                observation.profile.request_fingerprint,
                observation.recorded_at_ms,
                payload,
            ],
        )?;
    }
    tx.execute(
        "DELETE FROM runtime_host_request_observations WHERE observation_id IN (
            SELECT observation_id FROM runtime_host_request_observations
            ORDER BY recorded_at_ms DESC, observation_id DESC LIMIT -1 OFFSET ?1
         )",
        [crate::RUNTIME_HOST_OBSERVATION_STORED_LIMIT],
    )?;
    tx.commit()?;
    Ok(())
}

pub(super) fn summary(
    ledger: &SqliteDiagnosticsLedger,
    query: RuntimeHostObservationQuery,
) -> Result<RuntimeHostObservationSummary, DiagnosticsLedgerError> {
    query.validate()?;
    let mut stmt = ledger.conn.prepare(
        "SELECT payload_json FROM runtime_host_request_observations
         WHERE host_epoch = ?1 AND request_fingerprint = ?2
           AND recorded_at_ms >= ?3 AND recorded_at_ms <= ?4
         ORDER BY recorded_at_ms DESC, observation_id DESC LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![
            query.profile.host_epoch,
            query.profile.request_fingerprint,
            query.since_ms,
            query.until_ms,
            query.sample_limit,
        ],
        |row| row.get::<_, String>(0),
    )?;
    let mut observations = Vec::new();
    for row in rows {
        let observation: RuntimeHostRequestObservation = serde_json::from_str(&row?)?;
        observation.validate()?;
        if observation.profile != query.profile
            || observation.recorded_at_ms < query.since_ms
            || observation.recorded_at_ms > query.until_ms
        {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "stored_observation_query_mismatch",
            });
        }
        observations.push(observation);
    }
    Ok(RuntimeHostObservationSummary::from_observations(
        query.profile,
        observations,
    ))
}

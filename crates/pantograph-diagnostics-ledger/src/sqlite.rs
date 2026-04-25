use std::path::Path;

use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId,
};
use rusqlite::{params, Connection, Row};

use crate::records::{
    DiagnosticsProjection, DiagnosticsQuery, DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement, OutputModality,
    PruneUsageEventsCommand, PruneUsageEventsResult, RetentionClass, UsageEventStatus,
    UsageLineage,
};
use crate::schema::{apply_schema, current_schema_version, migrate_schema, SCHEMA_VERSION};
use crate::timing::{
    PruneTimingObservationsCommand, PruneTimingObservationsResult, WorkflowTimingExpectation,
    WorkflowTimingExpectationQuery, WorkflowTimingObservation, WorkflowTimingObservationStatus,
};
use crate::util::now_ms;
use crate::{DiagnosticsLedgerError, DiagnosticsLedgerRepository};

pub struct SqliteDiagnosticsLedger {
    conn: Connection,
}

impl SqliteDiagnosticsLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DiagnosticsLedgerError> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, DiagnosticsLedgerError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, DiagnosticsLedgerError> {
        let mut ledger = Self { conn };
        ledger.initialize()?;
        Ok(ledger)
    }

    pub fn initialize(&mut self) -> Result<(), DiagnosticsLedgerError> {
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        let version = current_schema_version(&self.conn)?;
        if let Some(found) = version {
            migrate_schema(&mut self.conn, found)?;
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        apply_schema(&tx)?;
        tx.commit()?;
        Ok(())
    }
}

impl DiagnosticsLedgerRepository for SqliteDiagnosticsLedger {
    fn record_usage_event(
        &mut self,
        event: ModelLicenseUsageEvent,
    ) -> Result<(), DiagnosticsLedgerError> {
        event.validate()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO model_license_usage_events
                (usage_event_id, client_id, client_session_id, bucket_id, workflow_run_id,
                 workflow_id, node_id, node_type, model_id, model_revision, model_hash,
                 model_modality, runtime_backend, guarantee_level, status, started_at_ms,
                 completed_at_ms, retention_class, schema_version, correlation_id)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                 ?16, ?17, ?18, ?19, ?20)",
            params![
                event.usage_event_id.as_str(),
                event.client_id.as_str(),
                event.client_session_id.as_str(),
                event.bucket_id.as_str(),
                event.workflow_run_id.as_str(),
                event.workflow_id.as_str(),
                event.lineage.node_id.as_str(),
                event.lineage.node_type.as_str(),
                event.model.model_id.as_str(),
                event.model.model_revision.as_deref(),
                event.model.model_hash.as_deref(),
                event.model.model_modality.as_deref(),
                event.model.runtime_backend.as_deref(),
                event.guarantee_level.as_db(),
                event.status.as_db(),
                event.started_at_ms,
                event.completed_at_ms,
                event.retention_class.as_db(),
                SCHEMA_VERSION,
                event.correlation_id.as_deref(),
            ],
        )?;
        tx.execute(
            "INSERT INTO license_snapshots
                (usage_event_id, license_value, source_metadata_json,
                 model_metadata_snapshot_json, unavailable_reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.usage_event_id.as_str(),
                event.license_snapshot.license_value.as_deref(),
                event.license_snapshot.source_metadata_json.as_deref(),
                event
                    .license_snapshot
                    .model_metadata_snapshot_json
                    .as_deref(),
                event.license_snapshot.unavailable_reason.as_deref(),
            ],
        )?;
        tx.execute(
            "INSERT INTO model_output_measurements
                (usage_event_id, modality, item_count, character_count, byte_size,
                 token_count, width, height, pixel_count, duration_ms, sample_rate_hz,
                 channels, frame_count, vector_count, dimensions, numeric_representation,
                 top_level_shape, schema_id, schema_digest, unavailable_reasons_json)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                 ?16, ?17, ?18, ?19, ?20)",
            params![
                event.usage_event_id.as_str(),
                event.output_measurement.modality.as_db(),
                event
                    .output_measurement
                    .item_count
                    .map(|value| value as i64),
                event
                    .output_measurement
                    .character_count
                    .map(|value| value as i64),
                event.output_measurement.byte_size.map(|value| value as i64),
                event
                    .output_measurement
                    .token_count
                    .map(|value| value as i64),
                event.output_measurement.width.map(|value| value as i64),
                event.output_measurement.height.map(|value| value as i64),
                event
                    .output_measurement
                    .pixel_count
                    .map(|value| value as i64),
                event
                    .output_measurement
                    .duration_ms
                    .map(|value| value as i64),
                event
                    .output_measurement
                    .sample_rate_hz
                    .map(|value| value as i64),
                event.output_measurement.channels.map(|value| value as i64),
                event
                    .output_measurement
                    .frame_count
                    .map(|value| value as i64),
                event
                    .output_measurement
                    .vector_count
                    .map(|value| value as i64),
                event
                    .output_measurement
                    .dimensions
                    .map(|value| value as i64),
                event.output_measurement.numeric_representation.as_deref(),
                event.output_measurement.top_level_shape.as_deref(),
                event.output_measurement.schema_id.as_deref(),
                event.output_measurement.schema_digest.as_deref(),
                serde_json::to_string(&event.output_measurement.unavailable_reasons)?,
            ],
        )?;
        tx.execute(
            "INSERT INTO usage_lineage
                (usage_event_id, node_id, node_type, port_ids_json,
                 composed_parent_chain_json, effective_contract_version,
                 effective_contract_digest, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.usage_event_id.as_str(),
                event.lineage.node_id.as_str(),
                event.lineage.node_type.as_str(),
                serde_json::to_string(&event.lineage.port_ids)?,
                serde_json::to_string(&event.lineage.composed_parent_chain)?,
                event.lineage.effective_contract_version.as_deref(),
                event.lineage.effective_contract_digest.as_deref(),
                event.lineage.metadata_json.as_deref(),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn query_usage_events(
        &self,
        query: DiagnosticsQuery,
    ) -> Result<DiagnosticsProjection, DiagnosticsLedgerError> {
        query.validate()?;
        let guarantee = query.guarantee_level.map(ExecutionGuaranteeLevel::as_db);
        let offset = i64::from(query.page) * i64::from(query.page_size);
        let mut stmt = self.conn.prepare(QUERY_EVENTS_SQL)?;
        let mut rows = stmt.query(params![
            query.client_id.as_ref().map(ClientId::as_str),
            query
                .client_session_id
                .as_ref()
                .map(ClientSessionId::as_str),
            query.bucket_id.as_ref().map(BucketId::as_str),
            query.workflow_run_id.as_ref().map(WorkflowRunId::as_str),
            query.workflow_id.as_ref().map(WorkflowId::as_str),
            query.node_id,
            query.model_id,
            query.license_value,
            guarantee,
            query.started_at_ms,
            query.ended_before_ms,
            i64::from(query.page_size),
            offset,
        ])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(event_from_row(row)?);
        }
        Ok(DiagnosticsProjection {
            events,
            page: query.page,
            page_size: query.page_size,
            may_have_pruned_usage: self.query_may_have_pruned_usage(query.started_at_ms)?,
        })
    }

    fn retention_policy(&self) -> Result<DiagnosticsRetentionPolicy, DiagnosticsLedgerError> {
        let policy = self.conn.query_row(
            "SELECT policy_id, retention_class, retention_days, applied_at_ms, explanation
             FROM diagnostics_retention_policy
             WHERE retention_class = ?1",
            params![RetentionClass::Standard.as_db()],
            |row| {
                Ok(DiagnosticsRetentionPolicy {
                    policy_id: row.get(0)?,
                    retention_class: RetentionClass::from_db(&row.get::<_, String>(1)?)
                        .map_err(to_sql_error)?,
                    retention_days: row.get::<_, i64>(2)? as u32,
                    applied_at_ms: row.get(3)?,
                    explanation: row.get(4)?,
                })
            },
        )?;
        Ok(policy)
    }

    fn prune_usage_events(
        &mut self,
        command: PruneUsageEventsCommand,
    ) -> Result<PruneUsageEventsResult, DiagnosticsLedgerError> {
        let tx = self.conn.transaction()?;
        let count = tx.query_row(
            "SELECT COUNT(*)
             FROM model_license_usage_events
             WHERE retention_class = ?1
               AND COALESCE(completed_at_ms, started_at_ms) < ?2",
            params![
                command.retention_class.as_db(),
                command.prune_completed_before_ms
            ],
            |row| row.get::<_, i64>(0),
        )? as u64;
        tx.execute(
            "DELETE FROM model_license_usage_events
             WHERE retention_class = ?1
               AND COALESCE(completed_at_ms, started_at_ms) < ?2",
            params![
                command.retention_class.as_db(),
                command.prune_completed_before_ms
            ],
        )?;
        tx.commit()?;
        Ok(PruneUsageEventsResult {
            pruned_event_count: count,
            retention_class: command.retention_class,
            prune_completed_before_ms: command.prune_completed_before_ms,
        })
    }

    fn record_timing_observation(
        &mut self,
        observation: WorkflowTimingObservation,
    ) -> Result<(), DiagnosticsLedgerError> {
        observation.validate()?;
        self.conn.execute(
            "INSERT OR IGNORE INTO workflow_timing_observations
                (observation_key, observation_scope, execution_id, workflow_id, workflow_name,
                 graph_fingerprint, node_id, node_type, runtime_id, status, started_at_ms,
                 ended_at_ms, duration_ms, recorded_at_ms)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                observation.observation_key.as_str(),
                observation.scope.as_db(),
                observation.execution_id.as_str(),
                observation.workflow_id.as_str(),
                observation.workflow_name.as_deref(),
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

    fn timing_expectation(
        &self,
        query: WorkflowTimingExpectationQuery,
    ) -> Result<WorkflowTimingExpectation, DiagnosticsLedgerError> {
        query.validate()?;
        let mut stmt = self.conn.prepare(
            "SELECT duration_ms
             FROM workflow_timing_observations
             WHERE observation_scope = ?1
               AND workflow_id = ?2
               AND graph_fingerprint = ?3
               AND (?4 IS NULL OR node_id = ?4)
               AND (?5 IS NULL OR node_type = ?5)
               AND (?6 IS NULL OR runtime_id = ?6)
               AND status = ?7
             ORDER BY recorded_at_ms DESC",
        )?;
        let rows = stmt.query_map(
            params![
                query.scope.as_db(),
                query.workflow_id.as_str(),
                query.graph_fingerprint.as_str(),
                query.node_id.as_deref(),
                query.node_type.as_deref(),
                query.runtime_id.as_deref(),
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
        Ok(WorkflowTimingExpectation::from_completed_durations(
            &query,
            durations_ms,
        ))
    }

    fn prune_timing_observations(
        &mut self,
        command: PruneTimingObservationsCommand,
    ) -> Result<PruneTimingObservationsResult, DiagnosticsLedgerError> {
        let tx = self.conn.transaction()?;
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
}

impl SqliteDiagnosticsLedger {
    fn query_may_have_pruned_usage(
        &self,
        started_at_ms: Option<i64>,
    ) -> Result<bool, DiagnosticsLedgerError> {
        let policy = self.retention_policy()?;
        let cutoff = now_ms() - i64::from(policy.retention_days) * 24 * 60 * 60 * 1000;
        Ok(match started_at_ms {
            Some(start) => start < cutoff,
            None => true,
        })
    }
}

const QUERY_EVENTS_SQL: &str = r#"
SELECT
    e.usage_event_id, e.client_id, e.client_session_id, e.bucket_id,
    e.workflow_run_id, e.workflow_id, e.node_id, e.node_type, e.model_id,
    e.model_revision, e.model_hash, e.model_modality, e.runtime_backend,
    e.guarantee_level, e.status, e.started_at_ms, e.completed_at_ms,
    e.retention_class, e.correlation_id,
    l.license_value, l.source_metadata_json, l.model_metadata_snapshot_json,
    l.unavailable_reason,
    m.modality, m.item_count, m.character_count, m.byte_size, m.token_count,
    m.width, m.height, m.pixel_count, m.duration_ms, m.sample_rate_hz,
    m.channels, m.frame_count, m.vector_count, m.dimensions,
    m.numeric_representation, m.top_level_shape, m.schema_id, m.schema_digest,
    m.unavailable_reasons_json,
    u.port_ids_json, u.composed_parent_chain_json, u.effective_contract_version,
    u.effective_contract_digest, u.metadata_json
FROM model_license_usage_events e
JOIN license_snapshots l ON l.usage_event_id = e.usage_event_id
JOIN model_output_measurements m ON m.usage_event_id = e.usage_event_id
JOIN usage_lineage u ON u.usage_event_id = e.usage_event_id
WHERE (?1 IS NULL OR e.client_id = ?1)
  AND (?2 IS NULL OR e.client_session_id = ?2)
  AND (?3 IS NULL OR e.bucket_id = ?3)
  AND (?4 IS NULL OR e.workflow_run_id = ?4)
  AND (?5 IS NULL OR e.workflow_id = ?5)
  AND (?6 IS NULL OR e.node_id = ?6)
  AND (?7 IS NULL OR e.model_id = ?7)
  AND (?8 IS NULL OR l.license_value = ?8)
  AND (?9 IS NULL OR e.guarantee_level = ?9)
  AND (?10 IS NULL OR e.started_at_ms >= ?10)
  AND (?11 IS NULL OR e.started_at_ms < ?11)
ORDER BY e.started_at_ms DESC, e.usage_event_id DESC
LIMIT ?12 OFFSET ?13
"#;

fn event_from_row(row: &Row<'_>) -> Result<ModelLicenseUsageEvent, DiagnosticsLedgerError> {
    let usage_event_id = UsageEventId::try_from(row.get::<_, String>(0)?).map_err(|_| {
        DiagnosticsLedgerError::InvalidField {
            field: "usage_event_id",
        }
    })?;
    let client_id = ClientId::try_from(row.get::<_, String>(1)?)
        .map_err(|_| DiagnosticsLedgerError::InvalidField { field: "client_id" })?;
    let client_session_id = ClientSessionId::try_from(row.get::<_, String>(2)?).map_err(|_| {
        DiagnosticsLedgerError::InvalidField {
            field: "client_session_id",
        }
    })?;
    let bucket_id = BucketId::try_from(row.get::<_, String>(3)?)
        .map_err(|_| DiagnosticsLedgerError::InvalidField { field: "bucket_id" })?;
    let workflow_run_id = WorkflowRunId::try_from(row.get::<_, String>(4)?).map_err(|_| {
        DiagnosticsLedgerError::InvalidField {
            field: "workflow_run_id",
        }
    })?;
    let workflow_id = WorkflowId::try_from(row.get::<_, String>(5)?).map_err(|_| {
        DiagnosticsLedgerError::InvalidField {
            field: "workflow_id",
        }
    })?;
    let node_id: String = row.get(6)?;
    let node_type: String = row.get(7)?;
    let model_id: String = row.get(8)?;
    let guarantee_level = ExecutionGuaranteeLevel::from_db(&row.get::<_, String>(13)?)?;
    let status = UsageEventStatus::from_db(&row.get::<_, String>(14)?)?;
    let retention_class = RetentionClass::from_db(&row.get::<_, String>(17)?)?;
    let modality = OutputModality::from_db(&row.get::<_, String>(23)?)?;

    Ok(ModelLicenseUsageEvent {
        usage_event_id,
        client_id,
        client_session_id,
        bucket_id,
        workflow_run_id,
        workflow_id,
        model: ModelIdentity {
            model_id,
            model_revision: row.get(9)?,
            model_hash: row.get(10)?,
            model_modality: row.get(11)?,
            runtime_backend: row.get(12)?,
        },
        lineage: UsageLineage {
            node_id,
            node_type,
            port_ids: serde_json::from_str(&row.get::<_, String>(42)?)?,
            composed_parent_chain: serde_json::from_str(&row.get::<_, String>(43)?)?,
            effective_contract_version: row.get(44)?,
            effective_contract_digest: row.get(45)?,
            metadata_json: row.get(46)?,
        },
        license_snapshot: LicenseSnapshot {
            license_value: row.get(19)?,
            source_metadata_json: row.get(20)?,
            model_metadata_snapshot_json: row.get(21)?,
            unavailable_reason: row.get(22)?,
        },
        output_measurement: ModelOutputMeasurement {
            modality,
            item_count: row.get::<_, Option<i64>>(24)?.map(|value| value as u64),
            character_count: row.get::<_, Option<i64>>(25)?.map(|value| value as u64),
            byte_size: row.get::<_, Option<i64>>(26)?.map(|value| value as u64),
            token_count: row.get::<_, Option<i64>>(27)?.map(|value| value as u64),
            width: row.get::<_, Option<i64>>(28)?.map(|value| value as u64),
            height: row.get::<_, Option<i64>>(29)?.map(|value| value as u64),
            pixel_count: row.get::<_, Option<i64>>(30)?.map(|value| value as u64),
            duration_ms: row.get::<_, Option<i64>>(31)?.map(|value| value as u64),
            sample_rate_hz: row.get::<_, Option<i64>>(32)?.map(|value| value as u64),
            channels: row.get::<_, Option<i64>>(33)?.map(|value| value as u64),
            frame_count: row.get::<_, Option<i64>>(34)?.map(|value| value as u64),
            vector_count: row.get::<_, Option<i64>>(35)?.map(|value| value as u64),
            dimensions: row.get::<_, Option<i64>>(36)?.map(|value| value as u64),
            numeric_representation: row.get(37)?,
            top_level_shape: row.get(38)?,
            schema_id: row.get(39)?,
            schema_digest: row.get(40)?,
            unavailable_reasons: serde_json::from_str(&row.get::<_, String>(41)?)?,
        },
        guarantee_level,
        status,
        retention_class,
        started_at_ms: row.get(15)?,
        completed_at_ms: row.get(16)?,
        correlation_id: row.get(18)?,
    })
}

fn to_sql_error(error: DiagnosticsLedgerError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

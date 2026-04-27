use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use rusqlite::{params, types::Type, OptionalExtension, Row};
use uuid::Uuid;

use crate::event::{
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, ProjectionStateRecord, ProjectionStateUpdate, ProjectionStatus,
    SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    DIAGNOSTIC_EVENT_SCHEMA_VERSION, MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES,
    SCHEDULER_TIMELINE_PROJECTION_NAME, SCHEDULER_TIMELINE_PROJECTION_VERSION,
};
use crate::records::MAX_PAGE_SIZE;
use crate::util::now_ms;
use crate::{DiagnosticsLedgerError, SqliteDiagnosticsLedger};

pub(super) fn append_diagnostic_event(
    ledger: &mut SqliteDiagnosticsLedger,
    request: DiagnosticEventAppendRequest,
) -> Result<DiagnosticEventRecord, DiagnosticsLedgerError> {
    request.validate()?;
    let payload_json = serde_json::to_string(&request.payload)?;
    if payload_json.len() > MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES {
        return Err(DiagnosticsLedgerError::EventPayloadTooLarge {
            max: MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES,
        });
    }
    let payload_hash = format!(
        "diagnostic-event-blake3:{}",
        blake3::hash(payload_json.as_bytes())
    );
    let payload_size_bytes = payload_json.len() as u64;
    let event_id = format!("devent_{}", Uuid::new_v4().simple());
    let event_kind = request.payload.event_kind();
    let recorded_at_ms = now_ms();

    let tx = ledger.conn.transaction()?;
    tx.execute(
        "INSERT INTO diagnostic_events
            (event_id, event_kind, schema_version, source_component, source_instance_id,
             occurred_at_ms, recorded_at_ms, workflow_run_id, workflow_id, workflow_version_id,
             workflow_semantic_version, node_id, node_type, node_version, runtime_id,
             runtime_version, model_id, model_version, client_id, client_session_id, bucket_id,
             scheduler_policy_id, retention_policy_id, privacy_class, event_retention_class,
             payload_hash, payload_size_bytes, payload_ref, payload_json)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
             ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
        params![
            event_id.as_str(),
            event_kind.as_db(),
            DIAGNOSTIC_EVENT_SCHEMA_VERSION,
            request.source_component.as_db(),
            request.source_instance_id.as_deref(),
            request.occurred_at_ms,
            recorded_at_ms,
            request.workflow_run_id.as_ref().map(|id| id.as_str()),
            request.workflow_id.as_ref().map(|id| id.as_str()),
            request.workflow_version_id.as_ref().map(|id| id.as_str()),
            request.workflow_semantic_version.as_deref(),
            request.node_id.as_deref(),
            request.node_type.as_deref(),
            request.node_version.as_deref(),
            request.runtime_id.as_deref(),
            request.runtime_version.as_deref(),
            request.model_id.as_deref(),
            request.model_version.as_deref(),
            request.client_id.as_ref().map(|id| id.as_str()),
            request.client_session_id.as_ref().map(|id| id.as_str()),
            request.bucket_id.as_ref().map(|id| id.as_str()),
            request.scheduler_policy_id.as_deref(),
            request.retention_policy_id.as_deref(),
            request.privacy_class.as_db(),
            request.retention_class.as_db(),
            payload_hash.as_str(),
            payload_size_bytes as i64,
            request.payload_ref.as_deref(),
            payload_json.as_str(),
        ],
    )?;
    let event_seq = tx.last_insert_rowid();
    tx.commit()?;

    Ok(DiagnosticEventRecord {
        event_seq,
        event_id,
        event_kind,
        schema_version: DIAGNOSTIC_EVENT_SCHEMA_VERSION,
        source_component: request.source_component,
        source_instance_id: request.source_instance_id,
        occurred_at_ms: request.occurred_at_ms,
        recorded_at_ms,
        workflow_run_id: request.workflow_run_id,
        workflow_id: request.workflow_id,
        workflow_version_id: request.workflow_version_id,
        workflow_semantic_version: request.workflow_semantic_version,
        node_id: request.node_id,
        node_type: request.node_type,
        node_version: request.node_version,
        runtime_id: request.runtime_id,
        runtime_version: request.runtime_version,
        model_id: request.model_id,
        model_version: request.model_version,
        client_id: request.client_id,
        client_session_id: request.client_session_id,
        bucket_id: request.bucket_id,
        scheduler_policy_id: request.scheduler_policy_id,
        retention_policy_id: request.retention_policy_id,
        privacy_class: request.privacy_class,
        retention_class: request.retention_class,
        payload_hash,
        payload_size_bytes,
        payload_ref: request.payload_ref,
        payload_json,
    })
}

pub(super) fn diagnostic_events_after(
    ledger: &SqliteDiagnosticsLedger,
    last_event_seq: i64,
    limit: u32,
) -> Result<Vec<DiagnosticEventRecord>, DiagnosticsLedgerError> {
    if limit > MAX_PAGE_SIZE {
        return Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: limit,
            max: MAX_PAGE_SIZE,
        });
    }
    if last_event_seq < 0 {
        return Err(DiagnosticsLedgerError::InvalidField {
            field: "last_event_seq",
        });
    }
    let mut stmt = ledger.conn.prepare(
        "SELECT event_seq, event_id, event_kind, schema_version, source_component,
                source_instance_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
                workflow_id, workflow_version_id, workflow_semantic_version, node_id,
                node_type, node_version, runtime_id, runtime_version, model_id,
                model_version, client_id, client_session_id, bucket_id, scheduler_policy_id,
                retention_policy_id, privacy_class, event_retention_class, payload_hash,
                payload_size_bytes, payload_ref, payload_json
         FROM diagnostic_events
         WHERE event_seq > ?1
         ORDER BY event_seq
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![last_event_seq, limit], diagnostic_event_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn projection_state(
    ledger: &SqliteDiagnosticsLedger,
    projection_name: &str,
) -> Result<Option<ProjectionStateRecord>, DiagnosticsLedgerError> {
    let mut stmt = ledger.conn.prepare(
        "SELECT projection_name, projection_version, last_applied_event_seq, status,
                rebuilt_at_ms, updated_at_ms
         FROM projection_state
         WHERE projection_name = ?1",
    )?;
    stmt.query_row(params![projection_name], projection_state_from_row)
        .optional()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn upsert_projection_state(
    ledger: &mut SqliteDiagnosticsLedger,
    update: ProjectionStateUpdate,
) -> Result<ProjectionStateRecord, DiagnosticsLedgerError> {
    update.validate()?;
    let updated_at_ms = now_ms();
    ledger.conn.execute(
        "INSERT INTO projection_state
            (projection_name, projection_version, last_applied_event_seq, status,
             rebuilt_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(projection_name) DO UPDATE SET
            projection_version = excluded.projection_version,
            last_applied_event_seq = excluded.last_applied_event_seq,
            status = excluded.status,
            rebuilt_at_ms = excluded.rebuilt_at_ms,
            updated_at_ms = excluded.updated_at_ms",
        params![
            update.projection_name.as_str(),
            update.projection_version,
            update.last_applied_event_seq,
            update.status.as_db(),
            update.rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    Ok(ProjectionStateRecord {
        projection_name: update.projection_name,
        projection_version: update.projection_version,
        last_applied_event_seq: update.last_applied_event_seq,
        status: update.status,
        rebuilt_at_ms: update.rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn drain_scheduler_timeline_projection(
    ledger: &mut SqliteDiagnosticsLedger,
    limit: u32,
) -> Result<ProjectionStateRecord, DiagnosticsLedgerError> {
    if limit > MAX_PAGE_SIZE {
        return Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: limit,
            max: MAX_PAGE_SIZE,
        });
    }
    let tx = ledger.conn.transaction()?;
    let mut state = {
        let mut stmt = tx.prepare(
            "SELECT projection_name, projection_version, last_applied_event_seq, status,
                    rebuilt_at_ms, updated_at_ms
             FROM projection_state
             WHERE projection_name = ?1",
        )?;
        stmt.query_row(
            params![SCHEDULER_TIMELINE_PROJECTION_NAME],
            projection_state_from_row,
        )
        .optional()?
    };

    let mut last_applied_event_seq = state
        .as_ref()
        .map(|state| state.last_applied_event_seq)
        .unwrap_or(0);
    let mut rebuilt_at_ms = state.as_ref().and_then(|state| state.rebuilt_at_ms);
    if state
        .as_ref()
        .map(|state| state.projection_version != SCHEDULER_TIMELINE_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM scheduler_timeline_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
        state = None;
    }

    let events = {
        let mut stmt = tx.prepare(
            "SELECT event_seq, event_id, event_kind, schema_version, source_component,
                    source_instance_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
                    workflow_id, workflow_version_id, workflow_semantic_version, node_id,
                    node_type, node_version, runtime_id, runtime_version, model_id,
                    model_version, client_id, client_session_id, bucket_id, scheduler_policy_id,
                    retention_policy_id, privacy_class, event_retention_class, payload_hash,
                    payload_size_bytes, payload_ref, payload_json
             FROM diagnostic_events
             WHERE event_seq > ?1
               AND event_kind IN (
                    'scheduler.estimate_produced',
                    'scheduler.queue_placement',
                    'run.started',
                    'run.terminal',
                    'run.snapshot_accepted'
               )
             ORDER BY event_seq
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(
            params![last_applied_event_seq, limit],
            diagnostic_event_from_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    for event in &events {
        if let Some(record) = scheduler_timeline_record_from_event(event)? {
            insert_scheduler_timeline_projection(&tx, &record)?;
        }
        last_applied_event_seq = event.event_seq;
    }

    let updated_at_ms = now_ms();
    tx.execute(
        "INSERT INTO projection_state
            (projection_name, projection_version, last_applied_event_seq, status,
             rebuilt_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(projection_name) DO UPDATE SET
            projection_version = excluded.projection_version,
            last_applied_event_seq = excluded.last_applied_event_seq,
            status = excluded.status,
            rebuilt_at_ms = excluded.rebuilt_at_ms,
            updated_at_ms = excluded.updated_at_ms",
        params![
            SCHEDULER_TIMELINE_PROJECTION_NAME,
            SCHEDULER_TIMELINE_PROJECTION_VERSION,
            last_applied_event_seq,
            ProjectionStatus::Current.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: SCHEDULER_TIMELINE_PROJECTION_NAME.to_string(),
        projection_version: SCHEDULER_TIMELINE_PROJECTION_VERSION,
        last_applied_event_seq,
        status: ProjectionStatus::Current,
        rebuilt_at_ms: state
            .as_ref()
            .and_then(|state| state.rebuilt_at_ms)
            .or(rebuilt_at_ms),
        updated_at_ms,
    })
}

pub(super) fn query_scheduler_timeline_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: SchedulerTimelineProjectionQuery,
) -> Result<Vec<SchedulerTimelineProjectionRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT event_seq, event_id, event_kind, source_component, occurred_at_ms,
                recorded_at_ms, workflow_run_id, workflow_id, workflow_version_id,
                workflow_semantic_version, scheduler_policy_id, retention_policy_id,
                summary, detail, payload_json
         FROM scheduler_timeline_projection
         WHERE (?1 IS NULL OR workflow_run_id = ?1)
           AND (?2 IS NULL OR workflow_id = ?2)
           AND event_seq > ?3
         ORDER BY event_seq
         LIMIT ?4",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.workflow_id.as_ref().map(|id| id.as_str()),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        scheduler_timeline_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

fn diagnostic_event_from_row(row: &Row<'_>) -> rusqlite::Result<DiagnosticEventRecord> {
    Ok(DiagnosticEventRecord {
        event_seq: row.get(0)?,
        event_id: row.get(1)?,
        event_kind: row
            .get::<_, String>(2)
            .and_then(parse_diagnostic_event_kind)?,
        schema_version: row.get(3)?,
        source_component: row
            .get::<_, String>(4)
            .and_then(parse_event_source_component)?,
        source_instance_id: row.get(5)?,
        occurred_at_ms: row.get(6)?,
        recorded_at_ms: row.get(7)?,
        workflow_run_id: row
            .get::<_, Option<String>>(8)?
            .map(WorkflowRunId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_id: row
            .get::<_, Option<String>>(9)?
            .map(WorkflowId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_version_id: row
            .get::<_, Option<String>>(10)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_semantic_version: row.get(11)?,
        node_id: row.get(12)?,
        node_type: row.get(13)?,
        node_version: row.get(14)?,
        runtime_id: row.get(15)?,
        runtime_version: row.get(16)?,
        model_id: row.get(17)?,
        model_version: row.get(18)?,
        client_id: row
            .get::<_, Option<String>>(19)?
            .map(ClientId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        client_session_id: row
            .get::<_, Option<String>>(20)?
            .map(ClientSessionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        bucket_id: row
            .get::<_, Option<String>>(21)?
            .map(BucketId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        scheduler_policy_id: row.get(22)?,
        retention_policy_id: row.get(23)?,
        privacy_class: row
            .get::<_, String>(24)
            .and_then(parse_event_privacy_class)?,
        retention_class: row
            .get::<_, String>(25)
            .and_then(parse_event_retention_class)?,
        payload_hash: row.get(26)?,
        payload_size_bytes: row
            .get::<_, i64>(27)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        payload_ref: row.get(28)?,
        payload_json: row.get(29)?,
    })
}

fn scheduler_timeline_record_from_event(
    event: &DiagnosticEventRecord,
) -> Result<Option<SchedulerTimelineProjectionRecord>, DiagnosticsLedgerError> {
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let (summary, detail) = match payload {
        DiagnosticEventPayload::SchedulerEstimateProduced(payload) => {
            let detail = if payload.reasons.is_empty() {
                None
            } else {
                Some(payload.reasons.join("; "))
            };
            ("scheduler estimate produced".to_string(), detail)
        }
        DiagnosticEventPayload::SchedulerQueuePlacement(payload) => (
            format!("queued at position {}", payload.queue_position),
            Some(format!("priority {}", payload.priority)),
        ),
        DiagnosticEventPayload::RunStarted(payload) => {
            let detail = match (
                payload.queue_wait_ms,
                payload.scheduler_decision_reason.as_deref(),
            ) {
                (Some(queue_wait_ms), Some(reason)) => {
                    Some(format!("queue wait {queue_wait_ms} ms; {reason}"))
                }
                (Some(queue_wait_ms), None) => Some(format!("queue wait {queue_wait_ms} ms")),
                (None, Some(reason)) => Some(reason.to_string()),
                (None, None) => None,
            };
            ("run started".to_string(), detail)
        }
        DiagnosticEventPayload::RunTerminal(payload) => {
            let summary = format!("run {:?}", payload.status).to_lowercase();
            (summary, payload.error)
        }
        DiagnosticEventPayload::RunSnapshotAccepted(payload) => (
            "run snapshot accepted".to_string(),
            Some(payload.workflow_run_snapshot_id),
        ),
        _ => return Ok(None),
    };
    Ok(Some(SchedulerTimelineProjectionRecord {
        event_seq: event.event_seq,
        event_id: event.event_id.clone(),
        event_kind: event.event_kind,
        source_component: event.source_component,
        occurred_at_ms: event.occurred_at_ms,
        recorded_at_ms: event.recorded_at_ms,
        workflow_run_id: event.workflow_run_id.clone().ok_or(
            DiagnosticsLedgerError::MissingField {
                field: "workflow_run_id",
            },
        )?,
        workflow_id: event
            .workflow_id
            .clone()
            .ok_or(DiagnosticsLedgerError::MissingField {
                field: "workflow_id",
            })?,
        workflow_version_id: event.workflow_version_id.clone(),
        workflow_semantic_version: event.workflow_semantic_version.clone(),
        scheduler_policy_id: event.scheduler_policy_id.clone(),
        retention_policy_id: event.retention_policy_id.clone(),
        summary,
        detail,
        payload_json: event.payload_json.clone(),
    }))
}

fn insert_scheduler_timeline_projection(
    tx: &rusqlite::Transaction<'_>,
    record: &SchedulerTimelineProjectionRecord,
) -> Result<(), DiagnosticsLedgerError> {
    tx.execute(
        "INSERT OR IGNORE INTO scheduler_timeline_projection
            (event_seq, event_id, event_kind, source_component, occurred_at_ms,
             recorded_at_ms, workflow_run_id, workflow_id, workflow_version_id,
             workflow_semantic_version, scheduler_policy_id, retention_policy_id,
             summary, detail, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            record.event_seq,
            record.event_id.as_str(),
            record.event_kind.as_db(),
            record.source_component.as_db(),
            record.occurred_at_ms,
            record.recorded_at_ms,
            record.workflow_run_id.as_str(),
            record.workflow_id.as_str(),
            record
                .workflow_version_id
                .as_ref()
                .map(|workflow_version_id| workflow_version_id.as_str()),
            record.workflow_semantic_version.as_deref(),
            record.scheduler_policy_id.as_deref(),
            record.retention_policy_id.as_deref(),
            record.summary.as_str(),
            record.detail.as_deref(),
            record.payload_json.as_str(),
        ],
    )?;
    Ok(())
}

fn scheduler_timeline_projection_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<SchedulerTimelineProjectionRecord> {
    Ok(SchedulerTimelineProjectionRecord {
        event_seq: row.get(0)?,
        event_id: row.get(1)?,
        event_kind: row
            .get::<_, String>(2)
            .and_then(parse_diagnostic_event_kind)?,
        source_component: row
            .get::<_, String>(3)
            .and_then(parse_event_source_component)?,
        occurred_at_ms: row.get(4)?,
        recorded_at_ms: row.get(5)?,
        workflow_run_id: row
            .get::<_, String>(6)
            .and_then(|value| WorkflowRunId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_id: row
            .get::<_, String>(7)
            .and_then(|value| WorkflowId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_version_id: row
            .get::<_, Option<String>>(8)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_semantic_version: row.get(9)?,
        scheduler_policy_id: row.get(10)?,
        retention_policy_id: row.get(11)?,
        summary: row.get(12)?,
        detail: row.get(13)?,
        payload_json: row.get(14)?,
    })
}

fn projection_state_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectionStateRecord> {
    Ok(ProjectionStateRecord {
        projection_name: row.get(0)?,
        projection_version: row.get(1)?,
        last_applied_event_seq: row.get(2)?,
        status: row.get::<_, String>(3).and_then(parse_projection_status)?,
        rebuilt_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
    })
}

fn parse_diagnostic_event_kind(value: String) -> rusqlite::Result<DiagnosticEventKind> {
    DiagnosticEventKind::from_db(&value).map_err(sqlite_conversion_error)
}

fn parse_event_source_component(value: String) -> rusqlite::Result<DiagnosticEventSourceComponent> {
    DiagnosticEventSourceComponent::from_db(&value).map_err(sqlite_conversion_error)
}

fn parse_event_privacy_class(value: String) -> rusqlite::Result<DiagnosticEventPrivacyClass> {
    DiagnosticEventPrivacyClass::from_db(&value).map_err(sqlite_conversion_error)
}

fn parse_event_retention_class(value: String) -> rusqlite::Result<DiagnosticEventRetentionClass> {
    DiagnosticEventRetentionClass::from_db(&value).map_err(sqlite_conversion_error)
}

fn parse_projection_status(value: String) -> rusqlite::Result<ProjectionStatus> {
    ProjectionStatus::from_db(&value).map_err(sqlite_conversion_error)
}

fn sqlite_conversion_error<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
}

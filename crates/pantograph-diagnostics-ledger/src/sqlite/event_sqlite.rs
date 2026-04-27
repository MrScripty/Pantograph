use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use rusqlite::{params, types::Type, OptionalExtension, Row};
use uuid::Uuid;

use crate::event::{
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, IoArtifactProjectionQuery, IoArtifactProjectionRecord,
    ProjectionStateRecord, ProjectionStateUpdate, ProjectionStatus, RunDetailProjectionQuery,
    RunDetailProjectionRecord, RunListProjectionQuery, RunListProjectionRecord,
    RunListProjectionStatus, SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    DIAGNOSTIC_EVENT_SCHEMA_VERSION, IO_ARTIFACT_PROJECTION_NAME, IO_ARTIFACT_PROJECTION_VERSION,
    MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES, RUN_DETAIL_PROJECTION_NAME, RUN_DETAIL_PROJECTION_VERSION,
    RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION, SCHEDULER_TIMELINE_PROJECTION_NAME,
    SCHEDULER_TIMELINE_PROJECTION_VERSION,
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
           AND (?3 IS NULL OR scheduler_policy_id = ?3)
           AND event_seq > ?4
         ORDER BY event_seq
         LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.workflow_id.as_ref().map(|id| id.as_str()),
            query.scheduler_policy_id.as_deref(),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        scheduler_timeline_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn drain_run_list_projection(
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
    let state = {
        let mut stmt = tx.prepare(
            "SELECT projection_name, projection_version, last_applied_event_seq, status,
                    rebuilt_at_ms, updated_at_ms
             FROM projection_state
             WHERE projection_name = ?1",
        )?;
        stmt.query_row(params![RUN_LIST_PROJECTION_NAME], projection_state_from_row)
            .optional()?
    };

    let mut last_applied_event_seq = state
        .as_ref()
        .map(|state| state.last_applied_event_seq)
        .unwrap_or(0);
    let mut rebuilt_at_ms = state.as_ref().and_then(|state| state.rebuilt_at_ms);
    if state
        .as_ref()
        .map(|state| state.projection_version != RUN_LIST_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM run_list_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
    }

    let events = diagnostic_projection_events_after(&tx, last_applied_event_seq, limit)?;
    for event in &events {
        apply_run_list_projection_event(&tx, event)?;
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
            RUN_LIST_PROJECTION_NAME,
            RUN_LIST_PROJECTION_VERSION,
            last_applied_event_seq,
            ProjectionStatus::Current.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: RUN_LIST_PROJECTION_NAME.to_string(),
        projection_version: RUN_LIST_PROJECTION_VERSION,
        last_applied_event_seq,
        status: ProjectionStatus::Current,
        rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn query_run_list_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: RunListProjectionQuery,
) -> Result<Vec<RunListProjectionRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT workflow_run_id, workflow_id, workflow_version_id,
                workflow_semantic_version, status, accepted_at_ms, enqueued_at_ms,
                started_at_ms, completed_at_ms, duration_ms, scheduler_policy_id,
                retention_policy_id, last_event_seq, last_updated_at_ms
         FROM run_list_projection
         WHERE (?1 IS NULL OR workflow_id = ?1)
           AND (?2 IS NULL OR workflow_version_id = ?2)
           AND (?3 IS NULL OR workflow_semantic_version = ?3)
           AND (?4 IS NULL OR status = ?4)
           AND (?5 IS NULL OR scheduler_policy_id = ?5)
           AND last_event_seq > ?6
         ORDER BY last_updated_at_ms DESC, last_event_seq DESC
         LIMIT ?7",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_id.as_ref().map(|id| id.as_str()),
            query
                .workflow_version_id
                .as_ref()
                .map(|workflow_version_id| workflow_version_id.as_str()),
            query.workflow_semantic_version.as_deref(),
            query.status.map(|status| status.as_db()),
            query.scheduler_policy_id.as_deref(),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        run_list_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn drain_run_detail_projection(
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
    let state = {
        let mut stmt = tx.prepare(
            "SELECT projection_name, projection_version, last_applied_event_seq, status,
                    rebuilt_at_ms, updated_at_ms
             FROM projection_state
             WHERE projection_name = ?1",
        )?;
        stmt.query_row(
            params![RUN_DETAIL_PROJECTION_NAME],
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
        .map(|state| state.projection_version != RUN_DETAIL_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM run_detail_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
    }

    let events = diagnostic_projection_events_after(&tx, last_applied_event_seq, limit)?;
    for event in &events {
        apply_run_detail_projection_event(&tx, event)?;
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
            RUN_DETAIL_PROJECTION_NAME,
            RUN_DETAIL_PROJECTION_VERSION,
            last_applied_event_seq,
            ProjectionStatus::Current.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: RUN_DETAIL_PROJECTION_NAME.to_string(),
        projection_version: RUN_DETAIL_PROJECTION_VERSION,
        last_applied_event_seq,
        status: ProjectionStatus::Current,
        rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn query_run_detail_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: RunDetailProjectionQuery,
) -> Result<Option<RunDetailProjectionRecord>, DiagnosticsLedgerError> {
    let mut stmt = ledger.conn.prepare(
        "SELECT workflow_run_id, workflow_id, workflow_version_id,
                workflow_semantic_version, status, accepted_at_ms, enqueued_at_ms,
                started_at_ms, completed_at_ms, duration_ms, scheduler_policy_id,
                retention_policy_id, client_id, client_session_id, bucket_id,
                workflow_run_snapshot_id, workflow_presentation_revision_id,
                latest_estimate_json, latest_queue_placement_json, started_payload_json,
                terminal_payload_json, terminal_error, timeline_event_count,
                last_event_seq, last_updated_at_ms
         FROM run_detail_projection
         WHERE workflow_run_id = ?1",
    )?;
    stmt.query_row(
        params![query.workflow_run_id.as_str()],
        run_detail_projection_from_row,
    )
    .optional()
    .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn drain_io_artifact_projection(
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
    let state = {
        let mut stmt = tx.prepare(
            "SELECT projection_name, projection_version, last_applied_event_seq, status,
                    rebuilt_at_ms, updated_at_ms
             FROM projection_state
             WHERE projection_name = ?1",
        )?;
        stmt.query_row(
            params![IO_ARTIFACT_PROJECTION_NAME],
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
        .map(|state| state.projection_version != IO_ARTIFACT_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM io_artifact_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
    }

    let events = io_artifact_events_after(&tx, last_applied_event_seq, limit)?;
    for event in &events {
        if let Some(record) = io_artifact_projection_record_from_event(event)? {
            insert_io_artifact_projection(&tx, &record)?;
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
            IO_ARTIFACT_PROJECTION_NAME,
            IO_ARTIFACT_PROJECTION_VERSION,
            last_applied_event_seq,
            ProjectionStatus::Current.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: IO_ARTIFACT_PROJECTION_NAME.to_string(),
        projection_version: IO_ARTIFACT_PROJECTION_VERSION,
        last_applied_event_seq,
        status: ProjectionStatus::Current,
        rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn query_io_artifact_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: IoArtifactProjectionQuery,
) -> Result<Vec<IoArtifactProjectionRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT event_seq, event_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
                workflow_id, workflow_version_id, workflow_semantic_version, node_id,
                node_type, node_version, runtime_id, runtime_version, model_id,
                model_version, artifact_id, artifact_role, media_type, size_bytes,
                content_hash, payload_ref, retention_policy_id
         FROM io_artifact_projection
         WHERE workflow_run_id = ?1
           AND (?2 IS NULL OR node_id = ?2)
           AND (?3 IS NULL OR artifact_role = ?3)
           AND (?4 IS NULL OR media_type = ?4)
           AND (?5 IS NULL OR retention_policy_id = ?5)
           AND (?6 IS NULL OR runtime_id = ?6)
           AND (?7 IS NULL OR model_id = ?7)
           AND event_seq > ?8
         ORDER BY event_seq
         LIMIT ?9",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_str(),
            query.node_id.as_deref(),
            query.artifact_role.as_deref(),
            query.media_type.as_deref(),
            query.retention_policy_id.as_deref(),
            query.runtime_id.as_deref(),
            query.model_id.as_deref(),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        io_artifact_projection_from_row,
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

fn diagnostic_projection_events_after(
    tx: &rusqlite::Transaction<'_>,
    last_applied_event_seq: i64,
    limit: u32,
) -> Result<Vec<DiagnosticEventRecord>, DiagnosticsLedgerError> {
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
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

fn io_artifact_events_after(
    tx: &rusqlite::Transaction<'_>,
    last_applied_event_seq: i64,
    limit: u32,
) -> Result<Vec<DiagnosticEventRecord>, DiagnosticsLedgerError> {
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
           AND event_kind = 'io.artifact_observed'
         ORDER BY event_seq
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(
        params![last_applied_event_seq, limit],
        diagnostic_event_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
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

fn io_artifact_projection_record_from_event(
    event: &DiagnosticEventRecord,
) -> Result<Option<IoArtifactProjectionRecord>, DiagnosticsLedgerError> {
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let DiagnosticEventPayload::IoArtifactObserved(payload) = payload else {
        return Ok(None);
    };

    Ok(Some(IoArtifactProjectionRecord {
        event_seq: event.event_seq,
        event_id: event.event_id.clone(),
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
        node_id: event.node_id.clone(),
        node_type: event.node_type.clone(),
        node_version: event.node_version.clone(),
        runtime_id: event.runtime_id.clone(),
        runtime_version: event.runtime_version.clone(),
        model_id: event.model_id.clone(),
        model_version: event.model_version.clone(),
        artifact_id: payload.artifact_id,
        artifact_role: payload.artifact_role,
        media_type: payload.media_type,
        size_bytes: payload.size_bytes,
        content_hash: payload.content_hash,
        payload_ref: event.payload_ref.clone(),
        retention_policy_id: event.retention_policy_id.clone(),
    }))
}

fn apply_run_list_projection_event(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
) -> Result<(), DiagnosticsLedgerError> {
    let Some(workflow_run_id) = event.workflow_run_id.as_ref() else {
        return Ok(());
    };
    let Some(workflow_id) = event.workflow_id.as_ref() else {
        return Ok(());
    };
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let status = match &payload {
        DiagnosticEventPayload::RunSnapshotAccepted(_) => RunListProjectionStatus::Accepted,
        DiagnosticEventPayload::SchedulerEstimateProduced(_) => RunListProjectionStatus::Accepted,
        DiagnosticEventPayload::SchedulerQueuePlacement(_) => RunListProjectionStatus::Queued,
        DiagnosticEventPayload::RunStarted(_) => RunListProjectionStatus::Running,
        DiagnosticEventPayload::RunTerminal(payload) => match payload.status {
            crate::event::RunTerminalStatus::Completed => RunListProjectionStatus::Completed,
            crate::event::RunTerminalStatus::Failed => RunListProjectionStatus::Failed,
            crate::event::RunTerminalStatus::Cancelled => RunListProjectionStatus::Cancelled,
        },
        _ => return Ok(()),
    };
    let accepted_at_ms = matches!(&payload, DiagnosticEventPayload::RunSnapshotAccepted(_))
        .then_some(event.occurred_at_ms);
    let enqueued_at_ms = matches!(&payload, DiagnosticEventPayload::SchedulerQueuePlacement(_))
        .then_some(event.occurred_at_ms);
    let started_at_ms =
        matches!(&payload, DiagnosticEventPayload::RunStarted(_)).then_some(event.occurred_at_ms);
    let (completed_at_ms, duration_ms) = match payload {
        DiagnosticEventPayload::RunTerminal(payload) => (
            Some(event.occurred_at_ms),
            payload.duration_ms.map(|value| value as i64),
        ),
        _ => (None, None),
    };

    tx.execute(
        "INSERT INTO run_list_projection
            (workflow_run_id, workflow_id, workflow_version_id, workflow_semantic_version,
             status, accepted_at_ms, enqueued_at_ms, started_at_ms, completed_at_ms,
             duration_ms, scheduler_policy_id, retention_policy_id, last_event_seq,
             last_updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(workflow_run_id) DO UPDATE SET
            workflow_id = excluded.workflow_id,
            workflow_version_id = COALESCE(excluded.workflow_version_id, workflow_version_id),
            workflow_semantic_version = COALESCE(excluded.workflow_semantic_version, workflow_semantic_version),
            status = excluded.status,
            accepted_at_ms = COALESCE(accepted_at_ms, excluded.accepted_at_ms),
            enqueued_at_ms = COALESCE(enqueued_at_ms, excluded.enqueued_at_ms),
            started_at_ms = COALESCE(started_at_ms, excluded.started_at_ms),
            completed_at_ms = COALESCE(excluded.completed_at_ms, completed_at_ms),
            duration_ms = COALESCE(excluded.duration_ms, duration_ms),
            scheduler_policy_id = COALESCE(excluded.scheduler_policy_id, scheduler_policy_id),
            retention_policy_id = COALESCE(excluded.retention_policy_id, retention_policy_id),
            last_event_seq = excluded.last_event_seq,
            last_updated_at_ms = excluded.last_updated_at_ms",
        params![
            workflow_run_id.as_str(),
            workflow_id.as_str(),
            event
                .workflow_version_id
                .as_ref()
                .map(|workflow_version_id| workflow_version_id.as_str()),
            event.workflow_semantic_version.as_deref(),
            status.as_db(),
            accepted_at_ms,
            enqueued_at_ms,
            started_at_ms,
            completed_at_ms,
            duration_ms,
            event.scheduler_policy_id.as_deref(),
            event.retention_policy_id.as_deref(),
            event.event_seq,
            event.occurred_at_ms,
        ],
    )?;
    Ok(())
}

fn apply_run_detail_projection_event(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
) -> Result<(), DiagnosticsLedgerError> {
    let Some(workflow_run_id) = event.workflow_run_id.as_ref() else {
        return Ok(());
    };
    let Some(workflow_id) = event.workflow_id.as_ref() else {
        return Ok(());
    };
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let status = match &payload {
        DiagnosticEventPayload::RunSnapshotAccepted(_) => RunListProjectionStatus::Accepted,
        DiagnosticEventPayload::SchedulerEstimateProduced(_) => RunListProjectionStatus::Accepted,
        DiagnosticEventPayload::SchedulerQueuePlacement(_) => RunListProjectionStatus::Queued,
        DiagnosticEventPayload::RunStarted(_) => RunListProjectionStatus::Running,
        DiagnosticEventPayload::RunTerminal(payload) => match payload.status {
            crate::event::RunTerminalStatus::Completed => RunListProjectionStatus::Completed,
            crate::event::RunTerminalStatus::Failed => RunListProjectionStatus::Failed,
            crate::event::RunTerminalStatus::Cancelled => RunListProjectionStatus::Cancelled,
        },
        _ => return Ok(()),
    };

    let accepted_at_ms = matches!(payload, DiagnosticEventPayload::RunSnapshotAccepted(_))
        .then_some(event.occurred_at_ms);
    let enqueued_at_ms = matches!(payload, DiagnosticEventPayload::SchedulerQueuePlacement(_))
        .then_some(event.occurred_at_ms);
    let started_at_ms =
        matches!(payload, DiagnosticEventPayload::RunStarted(_)).then_some(event.occurred_at_ms);
    let (completed_at_ms, duration_ms, terminal_error) = match &payload {
        DiagnosticEventPayload::RunTerminal(payload) => (
            Some(event.occurred_at_ms),
            payload.duration_ms.map(|value| value as i64),
            payload.error.as_deref(),
        ),
        _ => (None, None, None),
    };
    let (workflow_run_snapshot_id, workflow_presentation_revision_id) = match &payload {
        DiagnosticEventPayload::RunSnapshotAccepted(payload) => (
            Some(payload.workflow_run_snapshot_id.as_str()),
            Some(payload.workflow_presentation_revision_id.as_str()),
        ),
        _ => (None, None),
    };
    let latest_estimate_json = matches!(
        &payload,
        DiagnosticEventPayload::SchedulerEstimateProduced(_)
    )
    .then_some(event.payload_json.as_str());
    let latest_queue_placement_json =
        matches!(&payload, DiagnosticEventPayload::SchedulerQueuePlacement(_))
            .then_some(event.payload_json.as_str());
    let started_payload_json = matches!(&payload, DiagnosticEventPayload::RunStarted(_))
        .then_some(event.payload_json.as_str());
    let terminal_payload_json = matches!(&payload, DiagnosticEventPayload::RunTerminal(_))
        .then_some(event.payload_json.as_str());

    tx.execute(
        "INSERT INTO run_detail_projection
            (workflow_run_id, workflow_id, workflow_version_id, workflow_semantic_version,
             status, accepted_at_ms, enqueued_at_ms, started_at_ms, completed_at_ms,
             duration_ms, scheduler_policy_id, retention_policy_id, client_id,
             client_session_id, bucket_id, workflow_run_snapshot_id,
             workflow_presentation_revision_id, latest_estimate_json,
             latest_queue_placement_json, started_payload_json, terminal_payload_json,
             terminal_error, timeline_event_count, last_event_seq, last_updated_at_ms)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
             ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)
         ON CONFLICT(workflow_run_id) DO UPDATE SET
            workflow_id = excluded.workflow_id,
            workflow_version_id = COALESCE(excluded.workflow_version_id, workflow_version_id),
            workflow_semantic_version = COALESCE(excluded.workflow_semantic_version, workflow_semantic_version),
            status = excluded.status,
            accepted_at_ms = COALESCE(accepted_at_ms, excluded.accepted_at_ms),
            enqueued_at_ms = COALESCE(enqueued_at_ms, excluded.enqueued_at_ms),
            started_at_ms = COALESCE(started_at_ms, excluded.started_at_ms),
            completed_at_ms = COALESCE(excluded.completed_at_ms, completed_at_ms),
            duration_ms = COALESCE(excluded.duration_ms, duration_ms),
            scheduler_policy_id = COALESCE(excluded.scheduler_policy_id, scheduler_policy_id),
            retention_policy_id = COALESCE(excluded.retention_policy_id, retention_policy_id),
            client_id = COALESCE(excluded.client_id, client_id),
            client_session_id = COALESCE(excluded.client_session_id, client_session_id),
            bucket_id = COALESCE(excluded.bucket_id, bucket_id),
            workflow_run_snapshot_id = COALESCE(excluded.workflow_run_snapshot_id, workflow_run_snapshot_id),
            workflow_presentation_revision_id = COALESCE(excluded.workflow_presentation_revision_id, workflow_presentation_revision_id),
            latest_estimate_json = COALESCE(excluded.latest_estimate_json, latest_estimate_json),
            latest_queue_placement_json = COALESCE(excluded.latest_queue_placement_json, latest_queue_placement_json),
            started_payload_json = COALESCE(excluded.started_payload_json, started_payload_json),
            terminal_payload_json = COALESCE(excluded.terminal_payload_json, terminal_payload_json),
            terminal_error = COALESCE(excluded.terminal_error, terminal_error),
            timeline_event_count = timeline_event_count + 1,
            last_event_seq = excluded.last_event_seq,
            last_updated_at_ms = excluded.last_updated_at_ms",
        params![
            workflow_run_id.as_str(),
            workflow_id.as_str(),
            event
                .workflow_version_id
                .as_ref()
                .map(|workflow_version_id| workflow_version_id.as_str()),
            event.workflow_semantic_version.as_deref(),
            status.as_db(),
            accepted_at_ms,
            enqueued_at_ms,
            started_at_ms,
            completed_at_ms,
            duration_ms,
            event.scheduler_policy_id.as_deref(),
            event.retention_policy_id.as_deref(),
            event.client_id.as_ref().map(|client_id| client_id.as_str()),
            event
                .client_session_id
                .as_ref()
                .map(|client_session_id| client_session_id.as_str()),
            event.bucket_id.as_ref().map(|bucket_id| bucket_id.as_str()),
            workflow_run_snapshot_id,
            workflow_presentation_revision_id,
            latest_estimate_json,
            latest_queue_placement_json,
            started_payload_json,
            terminal_payload_json,
            terminal_error,
            1_i64,
            event.event_seq,
            event.occurred_at_ms,
        ],
    )?;
    Ok(())
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

fn insert_io_artifact_projection(
    tx: &rusqlite::Transaction<'_>,
    record: &IoArtifactProjectionRecord,
) -> Result<(), DiagnosticsLedgerError> {
    tx.execute(
        "INSERT OR IGNORE INTO io_artifact_projection
            (event_seq, event_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
             workflow_id, workflow_version_id, workflow_semantic_version, node_id,
             node_type, node_version, runtime_id, runtime_version, model_id,
             model_version, artifact_id, artifact_role, media_type, size_bytes,
             content_hash, payload_ref, retention_policy_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
        params![
            record.event_seq,
            record.event_id.as_str(),
            record.occurred_at_ms,
            record.recorded_at_ms,
            record.workflow_run_id.as_str(),
            record.workflow_id.as_str(),
            record
                .workflow_version_id
                .as_ref()
                .map(|workflow_version_id| workflow_version_id.as_str()),
            record.workflow_semantic_version.as_deref(),
            record.node_id.as_deref(),
            record.node_type.as_deref(),
            record.node_version.as_deref(),
            record.runtime_id.as_deref(),
            record.runtime_version.as_deref(),
            record.model_id.as_deref(),
            record.model_version.as_deref(),
            record.artifact_id.as_str(),
            record.artifact_role.as_str(),
            record.media_type.as_deref(),
            record.size_bytes.map(|value| value as i64),
            record.content_hash.as_deref(),
            record.payload_ref.as_deref(),
            record.retention_policy_id.as_deref(),
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

fn run_list_projection_from_row(row: &Row<'_>) -> rusqlite::Result<RunListProjectionRecord> {
    Ok(RunListProjectionRecord {
        workflow_run_id: row
            .get::<_, String>(0)
            .and_then(|value| WorkflowRunId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_id: row
            .get::<_, String>(1)
            .and_then(|value| WorkflowId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_version_id: row
            .get::<_, Option<String>>(2)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_semantic_version: row.get(3)?,
        status: row
            .get::<_, String>(4)
            .and_then(parse_run_list_projection_status)?,
        accepted_at_ms: row.get(5)?,
        enqueued_at_ms: row.get(6)?,
        started_at_ms: row.get(7)?,
        completed_at_ms: row.get(8)?,
        duration_ms: row
            .get::<_, Option<i64>>(9)?
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX)),
        scheduler_policy_id: row.get(10)?,
        retention_policy_id: row.get(11)?,
        last_event_seq: row.get(12)?,
        last_updated_at_ms: row.get(13)?,
    })
}

fn run_detail_projection_from_row(row: &Row<'_>) -> rusqlite::Result<RunDetailProjectionRecord> {
    Ok(RunDetailProjectionRecord {
        workflow_run_id: row
            .get::<_, String>(0)
            .and_then(|value| WorkflowRunId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_id: row
            .get::<_, String>(1)
            .and_then(|value| WorkflowId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_version_id: row
            .get::<_, Option<String>>(2)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_semantic_version: row.get(3)?,
        status: row
            .get::<_, String>(4)
            .and_then(parse_run_list_projection_status)?,
        accepted_at_ms: row.get(5)?,
        enqueued_at_ms: row.get(6)?,
        started_at_ms: row.get(7)?,
        completed_at_ms: row.get(8)?,
        duration_ms: row
            .get::<_, Option<i64>>(9)?
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX)),
        scheduler_policy_id: row.get(10)?,
        retention_policy_id: row.get(11)?,
        client_id: row
            .get::<_, Option<String>>(12)?
            .map(ClientId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        client_session_id: row
            .get::<_, Option<String>>(13)?
            .map(ClientSessionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        bucket_id: row
            .get::<_, Option<String>>(14)?
            .map(BucketId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_run_snapshot_id: row.get(15)?,
        workflow_presentation_revision_id: row.get(16)?,
        latest_estimate_json: row.get(17)?,
        latest_queue_placement_json: row.get(18)?,
        started_payload_json: row.get(19)?,
        terminal_payload_json: row.get(20)?,
        terminal_error: row.get(21)?,
        timeline_event_count: row
            .get::<_, i64>(22)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        last_event_seq: row.get(23)?,
        last_updated_at_ms: row.get(24)?,
    })
}

fn io_artifact_projection_from_row(row: &Row<'_>) -> rusqlite::Result<IoArtifactProjectionRecord> {
    Ok(IoArtifactProjectionRecord {
        event_seq: row.get(0)?,
        event_id: row.get(1)?,
        occurred_at_ms: row.get(2)?,
        recorded_at_ms: row.get(3)?,
        workflow_run_id: row
            .get::<_, String>(4)
            .and_then(|value| WorkflowRunId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_id: row
            .get::<_, String>(5)
            .and_then(|value| WorkflowId::try_from(value).map_err(sqlite_conversion_error))?,
        workflow_version_id: row
            .get::<_, Option<String>>(6)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        workflow_semantic_version: row.get(7)?,
        node_id: row.get(8)?,
        node_type: row.get(9)?,
        node_version: row.get(10)?,
        runtime_id: row.get(11)?,
        runtime_version: row.get(12)?,
        model_id: row.get(13)?,
        model_version: row.get(14)?,
        artifact_id: row.get(15)?,
        artifact_role: row.get(16)?,
        media_type: row.get(17)?,
        size_bytes: row
            .get::<_, Option<i64>>(18)?
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX)),
        content_hash: row.get(19)?,
        payload_ref: row.get(20)?,
        retention_policy_id: row.get(21)?,
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

fn parse_run_list_projection_status(value: String) -> rusqlite::Result<RunListProjectionStatus> {
    RunListProjectionStatus::from_db(&value).map_err(sqlite_conversion_error)
}

fn sqlite_conversion_error<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
}

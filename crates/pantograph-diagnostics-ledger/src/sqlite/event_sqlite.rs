use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use rusqlite::{params, types::Type, OptionalExtension, Row};
use uuid::Uuid;

use crate::event::{
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRecord, DiagnosticEventRetentionClass,
    DiagnosticEventSourceComponent, IoArtifactProjectionQuery, IoArtifactProjectionRecord,
    IoArtifactRetentionState, IoArtifactRetentionSummaryQuery, IoArtifactRetentionSummaryRecord,
    LibraryUsageProjectionQuery, LibraryUsageProjectionRecord, NodeExecutionProjectionStatus,
    NodeStatusProjectionQuery, NodeStatusProjectionRecord, ProjectionStateRecord,
    ProjectionStateUpdate, ProjectionStatus, RetentionArtifactStateChangedPayload,
    RunDetailProjectionQuery, RunDetailProjectionRecord, RunListFacetKind, RunListFacetRecord,
    RunListProjectionQuery, RunListProjectionRecord, RunListProjectionStatus,
    SchedulerQueueControlAction, SchedulerQueueControlActorScope, SchedulerQueueControlOutcome,
    SchedulerQueueControlPayload, SchedulerTimelineProjectionQuery,
    SchedulerTimelineProjectionRecord, DIAGNOSTIC_EVENT_SCHEMA_VERSION,
    IO_ARTIFACT_PROJECTION_NAME, IO_ARTIFACT_PROJECTION_VERSION, LIBRARY_USAGE_PROJECTION_NAME,
    LIBRARY_USAGE_PROJECTION_VERSION, MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES,
    NODE_STATUS_PROJECTION_NAME, NODE_STATUS_PROJECTION_VERSION, RUN_DETAIL_PROJECTION_NAME,
    RUN_DETAIL_PROJECTION_VERSION, RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION,
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
                    'scheduler.queue_control',
                    'scheduler.run_delayed',
                    'scheduler.model_lifecycle_changed',
                    'scheduler.run_admitted',
                    'scheduler.reservation_changed',
                    'run.started',
                    'run.terminal',
                    'run.snapshot_accepted',
                    'node.execution_status'
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
                retention_policy_id, client_id, client_session_id, bucket_id,
                workflow_execution_session_id,
                scheduler_queue_position, scheduler_priority,
                estimate_confidence, estimated_queue_wait_ms, estimated_duration_ms,
                scheduler_reason, last_event_seq, last_updated_at_ms
         FROM run_list_projection
         WHERE (?1 IS NULL OR workflow_id = ?1)
           AND (?2 IS NULL OR workflow_version_id = ?2)
           AND (?3 IS NULL OR workflow_semantic_version = ?3)
           AND (?4 IS NULL OR status = ?4)
           AND (?5 IS NULL OR scheduler_policy_id = ?5)
           AND (?6 IS NULL OR retention_policy_id = ?6)
           AND (?7 IS NULL OR client_id = ?7)
           AND (?8 IS NULL OR client_session_id = ?8)
           AND (?9 IS NULL OR bucket_id = ?9)
           AND (?10 IS NULL OR accepted_at_ms >= ?10)
           AND (?11 IS NULL OR accepted_at_ms <= ?11)
           AND last_event_seq > ?12
         ORDER BY last_updated_at_ms DESC, last_event_seq DESC
         LIMIT ?13",
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
            query.retention_policy_id.as_deref(),
            query.client_id.as_ref().map(|id| id.as_str()),
            query.client_session_id.as_ref().map(|id| id.as_str()),
            query.bucket_id.as_ref().map(|id| id.as_str()),
            query.accepted_at_from_ms,
            query.accepted_at_to_ms,
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        run_list_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn query_run_list_facets(
    ledger: &SqliteDiagnosticsLedger,
    query: RunListProjectionQuery,
) -> Result<Vec<RunListFacetRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut facets = Vec::new();
    query_run_list_facet(
        ledger,
        &query,
        RunListFacetKind::WorkflowVersion,
        "COALESCE(workflow_semantic_version, workflow_version_id, 'Unversioned')",
        &mut facets,
    )?;
    query_run_list_facet(
        ledger,
        &query,
        RunListFacetKind::Status,
        "status",
        &mut facets,
    )?;
    query_run_list_facet(
        ledger,
        &query,
        RunListFacetKind::SchedulerPolicy,
        "COALESCE(scheduler_policy_id, 'Unassigned')",
        &mut facets,
    )?;
    query_run_list_facet(
        ledger,
        &query,
        RunListFacetKind::RetentionPolicy,
        "COALESCE(retention_policy_id, 'Unassigned')",
        &mut facets,
    )?;
    Ok(facets)
}

fn query_run_list_facet(
    ledger: &SqliteDiagnosticsLedger,
    query: &RunListProjectionQuery,
    facet_kind: RunListFacetKind,
    expression: &'static str,
    facets: &mut Vec<RunListFacetRecord>,
) -> Result<(), DiagnosticsLedgerError> {
    let sql = format!(
        "SELECT {expression}, COUNT(*)
         FROM run_list_projection
         WHERE (?1 IS NULL OR workflow_id = ?1)
           AND (?2 IS NULL OR workflow_version_id = ?2)
           AND (?3 IS NULL OR workflow_semantic_version = ?3)
           AND (?4 IS NULL OR status = ?4)
           AND (?5 IS NULL OR scheduler_policy_id = ?5)
           AND (?6 IS NULL OR retention_policy_id = ?6)
           AND (?7 IS NULL OR client_id = ?7)
           AND (?8 IS NULL OR client_session_id = ?8)
           AND (?9 IS NULL OR bucket_id = ?9)
           AND (?10 IS NULL OR accepted_at_ms >= ?10)
           AND (?11 IS NULL OR accepted_at_ms <= ?11)
         GROUP BY {expression}
         ORDER BY COUNT(*) DESC, {expression}"
    );
    let mut stmt = ledger.conn.prepare(&sql)?;
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
            query.retention_policy_id.as_deref(),
            query.client_id.as_ref().map(|id| id.as_str()),
            query.client_session_id.as_ref().map(|id| id.as_str()),
            query.bucket_id.as_ref().map(|id| id.as_str()),
            query.accepted_at_from_ms,
            query.accepted_at_to_ms,
        ],
        |row| {
            Ok(RunListFacetRecord {
                facet_kind,
                facet_value: row.get(0)?,
                run_count: row
                    .get::<_, i64>(1)
                    .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
            })
        },
    )?;
    facets.extend(rows.collect::<Result<Vec<_>, _>>()?);
    Ok(())
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
                workflow_run_snapshot_id, workflow_execution_session_id,
                workflow_presentation_revision_id, latest_estimate_json,
                latest_queue_placement_json, started_payload_json, terminal_payload_json,
                terminal_error, scheduler_queue_position, scheduler_priority,
                estimate_confidence, estimated_queue_wait_ms, estimated_duration_ms,
                scheduler_reason, timeline_event_count, last_event_seq,
                last_updated_at_ms
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
        apply_io_artifact_projection_event(&tx, event)?;
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
                model_version, artifact_id, artifact_role, producer_node_id,
                producer_port_id, consumer_node_id, consumer_port_id, media_type,
                size_bytes, content_hash, payload_ref, retention_state,
                retention_reason, retention_policy_id
         FROM io_artifact_projection
         WHERE (?1 IS NULL OR workflow_run_id = ?1)
           AND (?2 IS NULL OR node_id = ?2)
           AND (?3 IS NULL OR producer_node_id = ?3)
           AND (?4 IS NULL OR consumer_node_id = ?4)
           AND (?5 IS NULL OR artifact_role = ?5)
           AND (?6 IS NULL OR media_type = ?6)
           AND (?7 IS NULL OR retention_state = ?7)
           AND (?8 IS NULL OR retention_policy_id = ?8)
           AND (?9 IS NULL OR runtime_id = ?9)
           AND (?10 IS NULL OR model_id = ?10)
           AND event_seq > ?11
         ORDER BY event_seq
         LIMIT ?12",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.node_id.as_deref(),
            query.producer_node_id.as_deref(),
            query.consumer_node_id.as_deref(),
            query.artifact_role.as_deref(),
            query.media_type.as_deref(),
            query.retention_state.map(IoArtifactRetentionState::as_db),
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

pub(super) fn query_io_artifact_retention_summary(
    ledger: &SqliteDiagnosticsLedger,
    query: IoArtifactRetentionSummaryQuery,
) -> Result<Vec<IoArtifactRetentionSummaryRecord>, DiagnosticsLedgerError> {
    query.validate()?;
    let mut stmt = ledger.conn.prepare(
        "SELECT retention_state, COUNT(*)
         FROM io_artifact_projection
         WHERE (?1 IS NULL OR workflow_run_id = ?1)
           AND (?2 IS NULL OR node_id = ?2)
           AND (?3 IS NULL OR producer_node_id = ?3)
           AND (?4 IS NULL OR consumer_node_id = ?4)
           AND (?5 IS NULL OR artifact_role = ?5)
           AND (?6 IS NULL OR media_type = ?6)
           AND (?7 IS NULL OR retention_policy_id = ?7)
           AND (?8 IS NULL OR runtime_id = ?8)
           AND (?9 IS NULL OR model_id = ?9)
         GROUP BY retention_state
         ORDER BY retention_state",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.node_id.as_deref(),
            query.producer_node_id.as_deref(),
            query.consumer_node_id.as_deref(),
            query.artifact_role.as_deref(),
            query.media_type.as_deref(),
            query.retention_policy_id.as_deref(),
            query.runtime_id.as_deref(),
            query.model_id.as_deref(),
        ],
        io_artifact_retention_summary_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn query_expirable_io_artifact_projection(
    ledger: &SqliteDiagnosticsLedger,
    cutoff_occurred_before_ms: i64,
    limit: u32,
) -> Result<Vec<IoArtifactProjectionRecord>, DiagnosticsLedgerError> {
    if limit > MAX_PAGE_SIZE {
        return Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: limit,
            max: MAX_PAGE_SIZE,
        });
    }
    let mut stmt = ledger.conn.prepare(
        "SELECT event_seq, event_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
                workflow_id, workflow_version_id, workflow_semantic_version, node_id,
                node_type, node_version, runtime_id, runtime_version, model_id,
                model_version, artifact_id, artifact_role, producer_node_id,
                producer_port_id, consumer_node_id, consumer_port_id, media_type,
                size_bytes, content_hash, payload_ref, retention_state,
                retention_reason, retention_policy_id
         FROM io_artifact_projection
         WHERE retention_state = ?1
           AND occurred_at_ms < ?2
         ORDER BY occurred_at_ms, event_seq
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![
            IoArtifactRetentionState::Retained.as_db(),
            cutoff_occurred_before_ms,
            limit,
        ],
        io_artifact_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn drain_node_status_projection(
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
            params![NODE_STATUS_PROJECTION_NAME],
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
        .map(|state| state.projection_version != NODE_STATUS_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM node_status_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
    }

    let events = node_status_events_after(&tx, last_applied_event_seq, limit)?;
    for event in &events {
        apply_node_status_projection_event(&tx, event)?;
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
            NODE_STATUS_PROJECTION_NAME,
            NODE_STATUS_PROJECTION_VERSION,
            last_applied_event_seq,
            ProjectionStatus::Current.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: NODE_STATUS_PROJECTION_NAME.to_string(),
        projection_version: NODE_STATUS_PROJECTION_VERSION,
        last_applied_event_seq,
        status: ProjectionStatus::Current,
        rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn query_node_status_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: NodeStatusProjectionQuery,
) -> Result<Vec<NodeStatusProjectionRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT workflow_run_id, workflow_id, workflow_version_id,
                workflow_semantic_version, node_id, node_type, node_version, runtime_id,
                runtime_version, model_id, model_version, status, started_at_ms,
                completed_at_ms, duration_ms, error, last_event_seq, last_updated_at_ms
         FROM node_status_projection
         WHERE (?1 IS NULL OR workflow_run_id = ?1)
           AND (?2 IS NULL OR node_id = ?2)
           AND (?3 IS NULL OR status = ?3)
           AND last_event_seq > ?4
         ORDER BY last_event_seq
         LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.node_id.as_deref(),
            query.status.map(NodeExecutionProjectionStatus::as_db),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        node_status_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn drain_library_usage_projection(
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
            params![LIBRARY_USAGE_PROJECTION_NAME],
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
        .map(|state| state.projection_version != LIBRARY_USAGE_PROJECTION_VERSION)
        .unwrap_or(false)
    {
        tx.execute("DELETE FROM library_usage_projection", [])?;
        tx.execute("DELETE FROM library_usage_run_projection", [])?;
        last_applied_event_seq = 0;
        rebuilt_at_ms = Some(now_ms());
    }

    let mut events =
        library_usage_events_after(&tx, last_applied_event_seq, limit.saturating_add(1))?;
    let has_more_events = events.len() > limit as usize;
    if has_more_events {
        events.truncate(limit as usize);
    }
    for event in &events {
        apply_library_usage_projection_event(&tx, event)?;
        last_applied_event_seq = event.event_seq;
    }

    let updated_at_ms = now_ms();
    let projection_status = if has_more_events {
        ProjectionStatus::Rebuilding
    } else {
        ProjectionStatus::Current
    };
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
            LIBRARY_USAGE_PROJECTION_NAME,
            LIBRARY_USAGE_PROJECTION_VERSION,
            last_applied_event_seq,
            projection_status.as_db(),
            rebuilt_at_ms,
            updated_at_ms,
        ],
    )?;
    tx.commit()?;

    Ok(ProjectionStateRecord {
        projection_name: LIBRARY_USAGE_PROJECTION_NAME.to_string(),
        projection_version: LIBRARY_USAGE_PROJECTION_VERSION,
        last_applied_event_seq,
        status: projection_status,
        rebuilt_at_ms,
        updated_at_ms,
    })
}

pub(super) fn query_library_usage_projection(
    ledger: &SqliteDiagnosticsLedger,
    query: LibraryUsageProjectionQuery,
) -> Result<Vec<LibraryUsageProjectionRecord>, DiagnosticsLedgerError> {
    query.validate(MAX_PAGE_SIZE)?;
    let mut stmt = ledger.conn.prepare(
        "SELECT asset_id, total_access_count, run_access_count, total_network_bytes,
                last_accessed_at_ms, last_operation, last_cache_status,
                last_workflow_run_id, last_workflow_id, last_workflow_version_id,
                last_workflow_semantic_version, last_client_id, last_client_session_id,
                last_bucket_id, last_event_seq, last_updated_at_ms
         FROM library_usage_projection
         WHERE (?1 IS NULL OR asset_id = ?1)
           AND ((?2 IS NULL AND ?3 IS NULL AND ?4 IS NULL) OR EXISTS (
                SELECT 1 FROM library_usage_run_projection run_link
                WHERE run_link.asset_id = library_usage_projection.asset_id
                  AND (?2 IS NULL OR run_link.workflow_run_id = ?2)
                  AND (?3 IS NULL OR run_link.workflow_id = ?3)
                  AND (?4 IS NULL OR run_link.workflow_version_id = ?4)
           ))
           AND last_event_seq > ?5
         ORDER BY last_accessed_at_ms DESC, last_event_seq DESC
         LIMIT ?6",
    )?;
    let rows = stmt.query_map(
        params![
            query.asset_id.as_deref(),
            query.workflow_run_id.as_ref().map(|id| id.as_str()),
            query.workflow_id.as_ref().map(|id| id.as_str()),
            query.workflow_version_id.as_ref().map(|id| id.as_str()),
            query.after_event_seq.unwrap_or(0),
            query.limit,
        ],
        library_usage_projection_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(DiagnosticsLedgerError::from)
}

pub(super) fn rebuild_projection(
    ledger: &mut SqliteDiagnosticsLedger,
    projection_name: &str,
    batch_size: u32,
) -> Result<ProjectionStateRecord, DiagnosticsLedgerError> {
    if batch_size > MAX_PAGE_SIZE {
        return Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: batch_size,
            max: MAX_PAGE_SIZE,
        });
    }
    let batch_size = batch_size.max(1);
    reset_projection(ledger, projection_name)?;

    let mut previous_event_seq = -1;
    loop {
        let state = match projection_name {
            SCHEDULER_TIMELINE_PROJECTION_NAME => {
                drain_scheduler_timeline_projection(ledger, batch_size)?
            }
            RUN_LIST_PROJECTION_NAME => drain_run_list_projection(ledger, batch_size)?,
            RUN_DETAIL_PROJECTION_NAME => drain_run_detail_projection(ledger, batch_size)?,
            IO_ARTIFACT_PROJECTION_NAME => drain_io_artifact_projection(ledger, batch_size)?,
            NODE_STATUS_PROJECTION_NAME => drain_node_status_projection(ledger, batch_size)?,
            LIBRARY_USAGE_PROJECTION_NAME => drain_library_usage_projection(ledger, batch_size)?,
            _ => {
                return Err(DiagnosticsLedgerError::InvalidField {
                    field: "projection_name",
                });
            }
        };
        if state.last_applied_event_seq == previous_event_seq {
            return Ok(state);
        }
        previous_event_seq = state.last_applied_event_seq;
    }
}

fn reset_projection(
    ledger: &mut SqliteDiagnosticsLedger,
    projection_name: &str,
) -> Result<(), DiagnosticsLedgerError> {
    let tx = ledger.conn.transaction()?;
    match projection_name {
        SCHEDULER_TIMELINE_PROJECTION_NAME => {
            tx.execute("DELETE FROM scheduler_timeline_projection", [])?;
        }
        RUN_LIST_PROJECTION_NAME => {
            tx.execute("DELETE FROM run_list_projection", [])?;
        }
        RUN_DETAIL_PROJECTION_NAME => {
            tx.execute("DELETE FROM run_detail_projection", [])?;
        }
        IO_ARTIFACT_PROJECTION_NAME => {
            tx.execute("DELETE FROM io_artifact_projection", [])?;
        }
        NODE_STATUS_PROJECTION_NAME => {
            tx.execute("DELETE FROM node_status_projection", [])?;
        }
        LIBRARY_USAGE_PROJECTION_NAME => {
            tx.execute("DELETE FROM library_usage_projection", [])?;
            tx.execute("DELETE FROM library_usage_run_projection", [])?;
        }
        _ => {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "projection_name",
            });
        }
    }
    tx.execute(
        "DELETE FROM projection_state WHERE projection_name = ?1",
        params![projection_name],
    )?;
    tx.commit()?;
    Ok(())
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
                'scheduler.queue_control',
                'scheduler.run_delayed',
                'scheduler.model_lifecycle_changed',
                'scheduler.run_admitted',
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
           AND event_kind IN (
                'io.artifact_observed',
                'retention.artifact_state_changed'
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

fn node_status_events_after(
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
           AND event_kind = 'node.execution_status'
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

fn library_usage_events_after(
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
           AND event_kind = 'library.asset_accessed'
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
            let mut details = Vec::new();
            if let Some(cache_state) = payload.model_cache_state {
                details.push(cache_state.summary().to_string());
            }
            if !payload.blocking_conditions.is_empty() {
                details.push(format!(
                    "blocking: {}",
                    payload
                        .blocking_conditions
                        .into_iter()
                        .map(|condition| condition.summary())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !payload.missing_asset_ids.is_empty() {
                details.push(format!(
                    "missing asset(s): {}",
                    payload.missing_asset_ids.join(", ")
                ));
            }
            if !payload.candidate_runtime_ids.is_empty() {
                details.push(format!(
                    "candidate runtime(s): {}",
                    payload.candidate_runtime_ids.join(", ")
                ));
            }
            details.extend(payload.reasons);
            let detail = if details.is_empty() {
                None
            } else {
                Some(details.join("; "))
            };
            ("scheduler estimate produced".to_string(), detail)
        }
        DiagnosticEventPayload::SchedulerQueuePlacement(payload) => (
            format!("queued at position {}", payload.queue_position),
            Some(format!("priority {}", payload.priority)),
        ),
        DiagnosticEventPayload::SchedulerQueueControl(payload) => {
            let summary = format!(
                "queue {} {}",
                queue_control_action_label(payload.action),
                queue_control_outcome_label(payload.outcome)
            );
            let detail = scheduler_queue_control_detail(&payload);
            (summary, detail)
        }
        DiagnosticEventPayload::SchedulerRunDelayed(payload) => {
            let detail = match (
                payload.delayed_until_ms,
                payload.fairness_context.as_deref(),
            ) {
                (Some(delayed_until_ms), Some(fairness_context)) => Some(format!(
                    "{}; delayed until {delayed_until_ms}; {fairness_context}",
                    payload.reason
                )),
                (Some(delayed_until_ms), None) => Some(format!(
                    "{}; delayed until {delayed_until_ms}",
                    payload.reason
                )),
                (None, Some(fairness_context)) => {
                    Some(format!("{}; {fairness_context}", payload.reason))
                }
                (None, None) => Some(payload.reason),
            };
            ("run delayed".to_string(), detail)
        }
        DiagnosticEventPayload::SchedulerModelLifecycleChanged(payload) => {
            let summary = payload.summary().to_string();
            let cache_state = payload.cache_state.map(|state| state.summary());
            let detail = match (
                payload.duration_ms,
                cache_state,
                payload.reason.as_deref(),
                payload.error.as_deref(),
            ) {
                (Some(duration_ms), Some(cache_state), Some(reason), Some(error)) => Some(format!(
                    "{duration_ms} ms; {cache_state}; {reason}; {error}"
                )),
                (Some(duration_ms), Some(cache_state), Some(reason), None) => {
                    Some(format!("{duration_ms} ms; {cache_state}; {reason}"))
                }
                (Some(duration_ms), Some(cache_state), None, Some(error)) => {
                    Some(format!("{duration_ms} ms; {cache_state}; {error}"))
                }
                (Some(duration_ms), Some(cache_state), None, None) => {
                    Some(format!("{duration_ms} ms; {cache_state}"))
                }
                (Some(duration_ms), None, Some(reason), Some(error)) => {
                    Some(format!("{duration_ms} ms; {reason}; {error}"))
                }
                (Some(duration_ms), None, Some(reason), None) => {
                    Some(format!("{duration_ms} ms; {reason}"))
                }
                (Some(duration_ms), None, None, Some(error)) => {
                    Some(format!("{duration_ms} ms; {error}"))
                }
                (Some(duration_ms), None, None, None) => Some(format!("{duration_ms} ms")),
                (None, Some(cache_state), Some(reason), Some(error)) => {
                    Some(format!("{cache_state}; {reason}; {error}"))
                }
                (None, Some(cache_state), Some(reason), None) => {
                    Some(format!("{cache_state}; {reason}"))
                }
                (None, Some(cache_state), None, Some(error)) => {
                    Some(format!("{cache_state}; {error}"))
                }
                (None, Some(cache_state), None, None) => Some(cache_state.to_string()),
                (None, None, Some(reason), Some(error)) => Some(format!("{reason}; {error}")),
                (None, None, Some(reason), None) => Some(reason.to_string()),
                (None, None, None, Some(error)) => Some(error.to_string()),
                (None, None, None, None) => None,
            };
            (summary, detail)
        }
        DiagnosticEventPayload::SchedulerRunAdmitted(payload) => {
            let mut details = Vec::new();
            if let Some(queue_wait_ms) = payload.queue_wait_ms {
                details.push(format!("queue wait {queue_wait_ms} ms"));
            }
            details.push(payload.decision_reason);
            if let Some(runtime_id) = payload.selected_runtime_id.as_deref() {
                details.push(format!("selected runtime {runtime_id}"));
            }
            if let Some(device_id) = payload.selected_device_id.as_deref() {
                details.push(format!("selected device {device_id}"));
            }
            if let Some(network_node_id) = payload.selected_network_node_id.as_deref() {
                details.push(format!("selected network node {network_node_id}"));
            }
            if !payload.reserved_model_ids.is_empty() {
                details.push(format!(
                    "reserved model(s): {}",
                    payload.reserved_model_ids.join(", ")
                ));
            }
            let detail = (!details.is_empty()).then(|| details.join("; "));
            ("run admitted".to_string(), detail)
        }
        DiagnosticEventPayload::SchedulerReservationChanged(payload) => {
            let mut details = vec![payload.reservation_id.clone()];
            if let Some(runtime_id) = payload.selected_runtime_id.as_deref() {
                details.push(format!("selected runtime {runtime_id}"));
            }
            if let Some(device_id) = payload.selected_device_id.as_deref() {
                details.push(format!("selected device {device_id}"));
            }
            if let Some(network_node_id) = payload.selected_network_node_id.as_deref() {
                details.push(format!("selected network node {network_node_id}"));
            }
            if !payload.reserved_model_ids.is_empty() {
                details.push(format!(
                    "reserved model(s): {}",
                    payload.reserved_model_ids.join(", ")
                ));
            }
            if let Some(reason) = payload.reason.as_deref() {
                details.push(reason.to_string());
            }
            (payload.summary(), Some(details.join("; ")))
        }
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
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            let node_id = event
                .node_id
                .as_deref()
                .ok_or(DiagnosticsLedgerError::MissingField { field: "node_id" })?;
            let summary = format!("node {node_id} {}", payload.status.as_db());
            let detail = match (payload.duration_ms, payload.error) {
                (Some(duration_ms), Some(error)) => Some(format!("{duration_ms} ms; {error}")),
                (Some(duration_ms), None) => Some(format!("{duration_ms} ms")),
                (None, Some(error)) => Some(error),
                (None, None) => None,
            };
            (summary, detail)
        }
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

fn queue_control_action_label(action: SchedulerQueueControlAction) -> &'static str {
    match action {
        SchedulerQueueControlAction::Cancel => "cancel",
        SchedulerQueueControlAction::PushToFront => "push to front",
        SchedulerQueueControlAction::Reprioritize => "reprioritize",
    }
}

fn queue_control_outcome_label(outcome: SchedulerQueueControlOutcome) -> &'static str {
    match outcome {
        SchedulerQueueControlOutcome::Accepted => "accepted",
        SchedulerQueueControlOutcome::Denied => "denied",
    }
}

fn queue_control_actor_scope_label(scope: SchedulerQueueControlActorScope) -> &'static str {
    match scope {
        SchedulerQueueControlActorScope::BackendControlApi => "backend control API",
        SchedulerQueueControlActorScope::ClientSession => "client session",
        SchedulerQueueControlActorScope::GuiAdmin => "GUI admin",
    }
}

fn scheduler_queue_control_detail(payload: &SchedulerQueueControlPayload) -> Option<String> {
    let mut parts = vec![queue_control_actor_scope_label(payload.actor_scope).to_string()];
    if let Some(session_id) = payload.requested_session_id.as_deref() {
        parts.push(format!("requested session {session_id}"));
    }
    if let Some(session_id) = payload.effective_session_id.as_deref() {
        parts.push(format!("effective session {session_id}"));
    }
    if let Some(position) = payload.previous_queue_position {
        parts.push(format!("position {position}"));
    }
    match (payload.previous_priority, payload.new_priority) {
        (Some(previous), Some(new)) => parts.push(format!("priority {previous} -> {new}")),
        (Some(previous), None) => parts.push(format!("priority {previous}")),
        (None, Some(new)) => parts.push(format!("priority {new}")),
        (None, None) => {}
    }
    if let Some(reason) = payload.reason.as_deref() {
        parts.push(reason.to_string());
    }
    (!parts.is_empty()).then(|| parts.join("; "))
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
        artifact_role: payload.artifact_role.as_db().to_string(),
        producer_node_id: payload.producer_node_id,
        producer_port_id: payload.producer_port_id,
        consumer_node_id: payload.consumer_node_id,
        consumer_port_id: payload.consumer_port_id,
        media_type: payload.media_type,
        size_bytes: payload.size_bytes,
        content_hash: payload.content_hash,
        payload_ref: event.payload_ref.clone(),
        retention_state: payload.retention_state.unwrap_or_else(|| {
            io_artifact_retention_state_from_payload_ref(event.payload_ref.as_deref())
        }),
        retention_reason: payload.retention_reason,
        retention_policy_id: event.retention_policy_id.clone(),
    }))
}

fn apply_io_artifact_projection_event(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
) -> Result<(), DiagnosticsLedgerError> {
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    match payload {
        DiagnosticEventPayload::IoArtifactObserved(_) => {
            if let Some(record) = io_artifact_projection_record_from_event(event)? {
                insert_io_artifact_projection(tx, &record)?;
            }
        }
        DiagnosticEventPayload::RetentionArtifactStateChanged(payload) => {
            apply_io_artifact_retention_state_change(tx, event, &payload)?;
        }
        _ => {}
    }
    Ok(())
}

fn apply_io_artifact_retention_state_change(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
    payload: &RetentionArtifactStateChangedPayload,
) -> Result<(), DiagnosticsLedgerError> {
    let workflow_run_id =
        event
            .workflow_run_id
            .as_ref()
            .ok_or(DiagnosticsLedgerError::MissingField {
                field: "workflow_run_id",
            })?;
    let clear_payload_ref = matches!(
        payload.retention_state,
        IoArtifactRetentionState::MetadataOnly
            | IoArtifactRetentionState::TooLarge
            | IoArtifactRetentionState::Expired
            | IoArtifactRetentionState::Deleted
    );

    tx.execute(
        "UPDATE io_artifact_projection
         SET event_seq = ?1,
             event_id = ?2,
             occurred_at_ms = ?3,
             recorded_at_ms = ?4,
             payload_ref = CASE
                WHEN ?5 IS NOT NULL THEN ?5
                WHEN ?6 THEN NULL
                ELSE payload_ref
             END,
             retention_state = ?7,
             retention_reason = ?8,
             retention_policy_id = COALESCE(?9, retention_policy_id)
         WHERE workflow_run_id = ?10
           AND artifact_id = ?11",
        params![
            event.event_seq,
            event.event_id.as_str(),
            event.occurred_at_ms,
            event.recorded_at_ms,
            event.payload_ref.as_deref(),
            clear_payload_ref,
            payload.retention_state.as_db(),
            payload.reason.as_str(),
            event.retention_policy_id.as_deref(),
            workflow_run_id.as_str(),
            payload.artifact_id.as_str(),
        ],
    )?;
    Ok(())
}

fn io_artifact_retention_state_from_payload_ref(
    payload_ref: Option<&str>,
) -> IoArtifactRetentionState {
    if payload_ref.is_some_and(|reference| !reference.trim().is_empty()) {
        IoArtifactRetentionState::Retained
    } else {
        IoArtifactRetentionState::MetadataOnly
    }
}

fn apply_node_status_projection_event(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
) -> Result<(), DiagnosticsLedgerError> {
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let DiagnosticEventPayload::NodeExecutionStatus(payload) = payload else {
        return Ok(());
    };
    let workflow_run_id =
        event
            .workflow_run_id
            .as_ref()
            .ok_or(DiagnosticsLedgerError::MissingField {
                field: "workflow_run_id",
            })?;
    let workflow_id = event
        .workflow_id
        .as_ref()
        .ok_or(DiagnosticsLedgerError::MissingField {
            field: "workflow_id",
        })?;
    let node_id = event
        .node_id
        .as_ref()
        .ok_or(DiagnosticsLedgerError::MissingField { field: "node_id" })?;

    tx.execute(
        "INSERT INTO node_status_projection
            (workflow_run_id, workflow_id, workflow_version_id,
             workflow_semantic_version, node_id, node_type, node_version, runtime_id,
             runtime_version, model_id, model_version, status, started_at_ms,
             completed_at_ms, duration_ms, error, last_event_seq, last_updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(workflow_run_id, node_id) DO UPDATE SET
            workflow_id = excluded.workflow_id,
            workflow_version_id = excluded.workflow_version_id,
            workflow_semantic_version = excluded.workflow_semantic_version,
            node_type = COALESCE(excluded.node_type, node_status_projection.node_type),
            node_version = COALESCE(excluded.node_version, node_status_projection.node_version),
            runtime_id = COALESCE(excluded.runtime_id, node_status_projection.runtime_id),
            runtime_version = COALESCE(excluded.runtime_version, node_status_projection.runtime_version),
            model_id = COALESCE(excluded.model_id, node_status_projection.model_id),
            model_version = COALESCE(excluded.model_version, node_status_projection.model_version),
            status = excluded.status,
            started_at_ms = COALESCE(excluded.started_at_ms, node_status_projection.started_at_ms),
            completed_at_ms = excluded.completed_at_ms,
            duration_ms = COALESCE(excluded.duration_ms, node_status_projection.duration_ms),
            error = excluded.error,
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
            node_id.as_str(),
            event.node_type.as_deref(),
            event.node_version.as_deref(),
            event.runtime_id.as_deref(),
            event.runtime_version.as_deref(),
            event.model_id.as_deref(),
            event.model_version.as_deref(),
            payload.status.as_db(),
            payload.started_at_ms,
            payload.completed_at_ms,
            payload.duration_ms.map(|value| value as i64),
            payload.error.as_deref(),
            event.event_seq,
            event.occurred_at_ms,
        ],
    )?;
    Ok(())
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
        DiagnosticEventPayload::SchedulerQueueControl(payload)
            if payload.action == crate::event::SchedulerQueueControlAction::Cancel =>
        {
            RunListProjectionStatus::Cancelled
        }
        DiagnosticEventPayload::SchedulerQueueControl(_) => RunListProjectionStatus::Queued,
        DiagnosticEventPayload::SchedulerRunDelayed(_) => RunListProjectionStatus::Delayed,
        DiagnosticEventPayload::SchedulerRunAdmitted(_) => RunListProjectionStatus::Running,
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
    let (completed_at_ms, duration_ms) = match &payload {
        DiagnosticEventPayload::RunTerminal(payload) => (
            Some(event.occurred_at_ms),
            payload.duration_ms.map(|value| value as i64),
        ),
        _ => (None, None),
    };
    let scheduler_facts = scheduler_projection_facts(&payload);
    let workflow_execution_session_id = match &payload {
        DiagnosticEventPayload::RunSnapshotAccepted(payload) => {
            Some(payload.workflow_execution_session_id.as_str())
        }
        _ => None,
    };

    tx.execute(
        "INSERT INTO run_list_projection
            (workflow_run_id, workflow_id, workflow_version_id, workflow_semantic_version,
             status, accepted_at_ms, enqueued_at_ms, started_at_ms, completed_at_ms,
             duration_ms, scheduler_policy_id, retention_policy_id, client_id,
             client_session_id, bucket_id, workflow_execution_session_id,
             scheduler_queue_position, scheduler_priority, estimate_confidence,
             estimated_queue_wait_ms, estimated_duration_ms, scheduler_reason,
             last_event_seq, last_updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
             ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
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
            workflow_execution_session_id = COALESCE(excluded.workflow_execution_session_id, workflow_execution_session_id),
            scheduler_queue_position = COALESCE(excluded.scheduler_queue_position, scheduler_queue_position),
            scheduler_priority = COALESCE(excluded.scheduler_priority, scheduler_priority),
            estimate_confidence = COALESCE(excluded.estimate_confidence, estimate_confidence),
            estimated_queue_wait_ms = COALESCE(excluded.estimated_queue_wait_ms, estimated_queue_wait_ms),
            estimated_duration_ms = COALESCE(excluded.estimated_duration_ms, estimated_duration_ms),
            scheduler_reason = COALESCE(excluded.scheduler_reason, scheduler_reason),
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
            event.client_id.as_ref().map(|id| id.as_str()),
            event.client_session_id.as_ref().map(|id| id.as_str()),
            event.bucket_id.as_ref().map(|id| id.as_str()),
            workflow_execution_session_id,
            scheduler_facts.queue_position.map(i64::from),
            scheduler_facts.priority.map(i64::from),
            scheduler_facts.estimate_confidence.as_deref(),
            scheduler_facts.estimated_queue_wait_ms.map(|value| value as i64),
            scheduler_facts.estimated_duration_ms.map(|value| value as i64),
            scheduler_facts.reason.as_deref(),
            event.event_seq,
            event.occurred_at_ms,
        ],
    )?;
    Ok(())
}

struct SchedulerProjectionFacts {
    queue_position: Option<u32>,
    priority: Option<i32>,
    estimate_confidence: Option<String>,
    estimated_queue_wait_ms: Option<u64>,
    estimated_duration_ms: Option<u64>,
    reason: Option<String>,
}

fn scheduler_projection_facts(payload: &DiagnosticEventPayload) -> SchedulerProjectionFacts {
    match payload {
        DiagnosticEventPayload::SchedulerEstimateProduced(payload) => SchedulerProjectionFacts {
            queue_position: None,
            priority: None,
            estimate_confidence: Some(payload.confidence.clone()),
            estimated_queue_wait_ms: payload.estimated_queue_wait_ms,
            estimated_duration_ms: payload.estimated_duration_ms,
            reason: payload.reasons.first().cloned(),
        },
        DiagnosticEventPayload::SchedulerQueuePlacement(payload) => SchedulerProjectionFacts {
            queue_position: Some(payload.queue_position),
            priority: Some(payload.priority),
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: None,
        },
        DiagnosticEventPayload::SchedulerQueueControl(payload) => SchedulerProjectionFacts {
            queue_position: payload.previous_queue_position,
            priority: payload.new_priority.or(payload.previous_priority),
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: payload.reason.clone(),
        },
        DiagnosticEventPayload::SchedulerRunDelayed(payload) => SchedulerProjectionFacts {
            queue_position: None,
            priority: None,
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: Some(payload.reason.clone()),
        },
        DiagnosticEventPayload::SchedulerModelLifecycleChanged(payload) => {
            SchedulerProjectionFacts {
                queue_position: None,
                priority: None,
                estimate_confidence: None,
                estimated_queue_wait_ms: None,
                estimated_duration_ms: None,
                reason: payload.reason.clone(),
            }
        }
        DiagnosticEventPayload::SchedulerRunAdmitted(payload) => SchedulerProjectionFacts {
            queue_position: None,
            priority: None,
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: Some(payload.decision_reason.clone()),
        },
        DiagnosticEventPayload::RunStarted(payload) => SchedulerProjectionFacts {
            queue_position: None,
            priority: None,
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: payload.scheduler_decision_reason.clone(),
        },
        _ => SchedulerProjectionFacts {
            queue_position: None,
            priority: None,
            estimate_confidence: None,
            estimated_queue_wait_ms: None,
            estimated_duration_ms: None,
            reason: None,
        },
    }
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
        DiagnosticEventPayload::SchedulerQueueControl(payload)
            if payload.action == crate::event::SchedulerQueueControlAction::Cancel =>
        {
            RunListProjectionStatus::Cancelled
        }
        DiagnosticEventPayload::SchedulerQueueControl(_) => RunListProjectionStatus::Queued,
        DiagnosticEventPayload::SchedulerRunDelayed(_) => RunListProjectionStatus::Delayed,
        DiagnosticEventPayload::SchedulerRunAdmitted(_) => RunListProjectionStatus::Running,
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
    let (
        workflow_run_snapshot_id,
        workflow_execution_session_id,
        workflow_presentation_revision_id,
    ) = match &payload {
        DiagnosticEventPayload::RunSnapshotAccepted(payload) => (
            Some(payload.workflow_run_snapshot_id.as_str()),
            Some(payload.workflow_execution_session_id.as_str()),
            Some(payload.workflow_presentation_revision_id.as_str()),
        ),
        _ => (None, None, None),
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
    let scheduler_facts = scheduler_projection_facts(&payload);

    tx.execute(
        "INSERT INTO run_detail_projection
            (workflow_run_id, workflow_id, workflow_version_id, workflow_semantic_version,
             status, accepted_at_ms, enqueued_at_ms, started_at_ms, completed_at_ms,
             duration_ms, scheduler_policy_id, retention_policy_id, client_id,
             client_session_id, bucket_id, workflow_run_snapshot_id,
             workflow_execution_session_id, workflow_presentation_revision_id, latest_estimate_json,
             latest_queue_placement_json, started_payload_json, terminal_payload_json,
             terminal_error, scheduler_queue_position, scheduler_priority,
             estimate_confidence, estimated_queue_wait_ms, estimated_duration_ms,
             scheduler_reason, timeline_event_count, last_event_seq, last_updated_at_ms)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
             ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
             ?29, ?30, ?31, ?32)
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
            workflow_execution_session_id = COALESCE(excluded.workflow_execution_session_id, workflow_execution_session_id),
            workflow_presentation_revision_id = COALESCE(excluded.workflow_presentation_revision_id, workflow_presentation_revision_id),
            latest_estimate_json = COALESCE(excluded.latest_estimate_json, latest_estimate_json),
            latest_queue_placement_json = COALESCE(excluded.latest_queue_placement_json, latest_queue_placement_json),
            started_payload_json = COALESCE(excluded.started_payload_json, started_payload_json),
            terminal_payload_json = COALESCE(excluded.terminal_payload_json, terminal_payload_json),
            terminal_error = COALESCE(excluded.terminal_error, terminal_error),
            scheduler_queue_position = COALESCE(excluded.scheduler_queue_position, scheduler_queue_position),
            scheduler_priority = COALESCE(excluded.scheduler_priority, scheduler_priority),
            estimate_confidence = COALESCE(excluded.estimate_confidence, estimate_confidence),
            estimated_queue_wait_ms = COALESCE(excluded.estimated_queue_wait_ms, estimated_queue_wait_ms),
            estimated_duration_ms = COALESCE(excluded.estimated_duration_ms, estimated_duration_ms),
            scheduler_reason = COALESCE(excluded.scheduler_reason, scheduler_reason),
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
            workflow_execution_session_id,
            workflow_presentation_revision_id,
            latest_estimate_json,
            latest_queue_placement_json,
            started_payload_json,
            terminal_payload_json,
            terminal_error,
            scheduler_facts.queue_position.map(i64::from),
            scheduler_facts.priority.map(i64::from),
            scheduler_facts.estimate_confidence.as_deref(),
            scheduler_facts.estimated_queue_wait_ms.map(|value| value as i64),
            scheduler_facts.estimated_duration_ms.map(|value| value as i64),
            scheduler_facts.reason.as_deref(),
            1_i64,
            event.event_seq,
            event.occurred_at_ms,
        ],
    )?;
    Ok(())
}

fn apply_library_usage_projection_event(
    tx: &rusqlite::Transaction<'_>,
    event: &DiagnosticEventRecord,
) -> Result<(), DiagnosticsLedgerError> {
    let payload: DiagnosticEventPayload = serde_json::from_str(&event.payload_json)?;
    let DiagnosticEventPayload::LibraryAssetAccessed(payload) = payload else {
        return Ok(());
    };
    let run_access_increment = if let Some(workflow_run_id) = event.workflow_run_id.as_ref() {
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO library_usage_run_projection
                (asset_id, workflow_run_id, workflow_id, workflow_version_id,
                 workflow_semantic_version, first_event_seq, last_event_seq,
                 last_accessed_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                payload.asset_id.as_str(),
                workflow_run_id.as_str(),
                event.workflow_id.as_ref().map(|id| id.as_str()),
                event.workflow_version_id.as_ref().map(|id| id.as_str()),
                event.workflow_semantic_version.as_deref(),
                event.event_seq,
                event.event_seq,
                event.occurred_at_ms,
            ],
        )?;
        tx.execute(
            "UPDATE library_usage_run_projection
             SET workflow_id = COALESCE(?3, workflow_id),
                 workflow_version_id = COALESCE(?4, workflow_version_id),
                 workflow_semantic_version = COALESCE(?5, workflow_semantic_version),
                 last_event_seq = ?6,
                 last_accessed_at_ms = ?7
             WHERE asset_id = ?1 AND workflow_run_id = ?2",
            params![
                payload.asset_id.as_str(),
                workflow_run_id.as_str(),
                event.workflow_id.as_ref().map(|id| id.as_str()),
                event.workflow_version_id.as_ref().map(|id| id.as_str()),
                event.workflow_semantic_version.as_deref(),
                event.event_seq,
                event.occurred_at_ms,
            ],
        )?;
        i64::try_from(inserted).unwrap_or(0)
    } else {
        0
    };

    tx.execute(
        "INSERT INTO library_usage_projection
            (asset_id, total_access_count, run_access_count, total_network_bytes,
             last_accessed_at_ms, last_operation, last_cache_status,
             last_workflow_run_id, last_workflow_id, last_workflow_version_id,
             last_workflow_semantic_version, last_client_id, last_client_session_id,
             last_bucket_id, last_event_seq, last_updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(asset_id) DO UPDATE SET
            total_access_count = total_access_count + 1,
            run_access_count = run_access_count + ?3,
            total_network_bytes = total_network_bytes + ?4,
            last_accessed_at_ms = excluded.last_accessed_at_ms,
            last_operation = excluded.last_operation,
            last_cache_status = excluded.last_cache_status,
            last_workflow_run_id = COALESCE(excluded.last_workflow_run_id, last_workflow_run_id),
            last_workflow_id = COALESCE(excluded.last_workflow_id, last_workflow_id),
            last_workflow_version_id = COALESCE(excluded.last_workflow_version_id, last_workflow_version_id),
            last_workflow_semantic_version = COALESCE(excluded.last_workflow_semantic_version, last_workflow_semantic_version),
            last_client_id = COALESCE(excluded.last_client_id, last_client_id),
            last_client_session_id = COALESCE(excluded.last_client_session_id, last_client_session_id),
            last_bucket_id = COALESCE(excluded.last_bucket_id, last_bucket_id),
            last_event_seq = excluded.last_event_seq,
            last_updated_at_ms = excluded.last_updated_at_ms",
        params![
            payload.asset_id.as_str(),
            1_i64,
            run_access_increment,
            payload.network_bytes.unwrap_or(0) as i64,
            event.occurred_at_ms,
            payload.operation.as_db(),
            payload.cache_status.as_ref().map(|status| status.as_db()),
            event.workflow_run_id.as_ref().map(|id| id.as_str()),
            event.workflow_id.as_ref().map(|id| id.as_str()),
            event
                .workflow_version_id
                .as_ref()
                .map(|id| id.as_str()),
            event.workflow_semantic_version.as_deref(),
            event.client_id.as_ref().map(|id| id.as_str()),
            event.client_session_id.as_ref().map(|id| id.as_str()),
            event.bucket_id.as_ref().map(|id| id.as_str()),
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
        "DELETE FROM io_artifact_projection
         WHERE workflow_run_id = ?1
           AND artifact_id = ?2",
        params![record.workflow_run_id.as_str(), record.artifact_id.as_str()],
    )?;
    tx.execute(
        "INSERT INTO io_artifact_projection
            (event_seq, event_id, occurred_at_ms, recorded_at_ms, workflow_run_id,
             workflow_id, workflow_version_id, workflow_semantic_version, node_id,
             node_type, node_version, runtime_id, runtime_version, model_id,
             model_version, artifact_id, artifact_role, producer_node_id,
             producer_port_id, consumer_node_id, consumer_port_id, media_type,
             size_bytes, content_hash, payload_ref, retention_state,
             retention_reason, retention_policy_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24,
                 ?25, ?26, ?27, ?28)",
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
            record.producer_node_id.as_deref(),
            record.producer_port_id.as_deref(),
            record.consumer_node_id.as_deref(),
            record.consumer_port_id.as_deref(),
            record.media_type.as_deref(),
            record.size_bytes.map(|value| value as i64),
            record.content_hash.as_deref(),
            record.payload_ref.as_deref(),
            record.retention_state.as_db(),
            record.retention_reason.as_deref(),
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
        workflow_execution_session_id: row.get(15)?,
        scheduler_queue_position: row.get::<_, Option<i64>>(16)?.map(i64_to_u32_saturating),
        scheduler_priority: row.get::<_, Option<i64>>(17)?.map(i64_to_i32_saturating),
        estimate_confidence: row.get(18)?,
        estimated_queue_wait_ms: row.get::<_, Option<i64>>(19)?.map(i64_to_u64_saturating),
        estimated_duration_ms: row.get::<_, Option<i64>>(20)?.map(i64_to_u64_saturating),
        scheduler_reason: row.get(21)?,
        last_event_seq: row.get(22)?,
        last_updated_at_ms: row.get(23)?,
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
        workflow_execution_session_id: row.get(16)?,
        workflow_presentation_revision_id: row.get(17)?,
        latest_estimate_json: row.get(18)?,
        latest_queue_placement_json: row.get(19)?,
        started_payload_json: row.get(20)?,
        terminal_payload_json: row.get(21)?,
        terminal_error: row.get(22)?,
        scheduler_queue_position: row.get::<_, Option<i64>>(23)?.map(i64_to_u32_saturating),
        scheduler_priority: row.get::<_, Option<i64>>(24)?.map(i64_to_i32_saturating),
        estimate_confidence: row.get(25)?,
        estimated_queue_wait_ms: row.get::<_, Option<i64>>(26)?.map(i64_to_u64_saturating),
        estimated_duration_ms: row.get::<_, Option<i64>>(27)?.map(i64_to_u64_saturating),
        scheduler_reason: row.get(28)?,
        timeline_event_count: row
            .get::<_, i64>(29)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        last_event_seq: row.get(30)?,
        last_updated_at_ms: row.get(31)?,
    })
}

fn i64_to_u32_saturating(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(if value < 0 { 0 } else { u32::MAX })
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

fn i64_to_u64_saturating(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
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
        producer_node_id: row.get(17)?,
        producer_port_id: row.get(18)?,
        consumer_node_id: row.get(19)?,
        consumer_port_id: row.get(20)?,
        media_type: row.get(21)?,
        size_bytes: row
            .get::<_, Option<i64>>(22)?
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX)),
        content_hash: row.get(23)?,
        payload_ref: row.get(24)?,
        retention_state: row.get::<_, String>(25).and_then(|value| {
            IoArtifactRetentionState::from_db(&value).map_err(sqlite_conversion_error)
        })?,
        retention_reason: row.get(26)?,
        retention_policy_id: row.get(27)?,
    })
}

fn io_artifact_retention_summary_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<IoArtifactRetentionSummaryRecord> {
    Ok(IoArtifactRetentionSummaryRecord {
        retention_state: row.get::<_, String>(0).and_then(|value| {
            IoArtifactRetentionState::from_db(&value).map_err(sqlite_conversion_error)
        })?,
        artifact_count: row
            .get::<_, i64>(1)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
    })
}

fn node_status_projection_from_row(row: &Row<'_>) -> rusqlite::Result<NodeStatusProjectionRecord> {
    Ok(NodeStatusProjectionRecord {
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
        node_id: row.get(4)?,
        node_type: row.get(5)?,
        node_version: row.get(6)?,
        runtime_id: row.get(7)?,
        runtime_version: row.get(8)?,
        model_id: row.get(9)?,
        model_version: row.get(10)?,
        status: row
            .get::<_, String>(11)
            .and_then(parse_node_execution_projection_status)?,
        started_at_ms: row.get(12)?,
        completed_at_ms: row.get(13)?,
        duration_ms: row.get::<_, Option<i64>>(14)?.map(i64_to_u64_saturating),
        error: row.get(15)?,
        last_event_seq: row.get(16)?,
        last_updated_at_ms: row.get(17)?,
    })
}

fn library_usage_projection_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<LibraryUsageProjectionRecord> {
    Ok(LibraryUsageProjectionRecord {
        asset_id: row.get(0)?,
        total_access_count: row
            .get::<_, i64>(1)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        run_access_count: row
            .get::<_, i64>(2)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        total_network_bytes: row
            .get::<_, i64>(3)
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))?,
        last_accessed_at_ms: row.get(4)?,
        last_operation: row.get(5)?,
        last_cache_status: row.get(6)?,
        last_workflow_run_id: row
            .get::<_, Option<String>>(7)?
            .map(WorkflowRunId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_workflow_id: row
            .get::<_, Option<String>>(8)?
            .map(WorkflowId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_workflow_version_id: row
            .get::<_, Option<String>>(9)?
            .map(WorkflowVersionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_workflow_semantic_version: row.get(10)?,
        last_client_id: row
            .get::<_, Option<String>>(11)?
            .map(ClientId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_client_session_id: row
            .get::<_, Option<String>>(12)?
            .map(ClientSessionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_bucket_id: row
            .get::<_, Option<String>>(13)?
            .map(BucketId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
        last_event_seq: row.get(14)?,
        last_updated_at_ms: row.get(15)?,
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

fn parse_node_execution_projection_status(
    value: String,
) -> rusqlite::Result<NodeExecutionProjectionStatus> {
    NodeExecutionProjectionStatus::from_db(&value).map_err(sqlite_conversion_error)
}

fn sqlite_conversion_error<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
}

use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use rusqlite::Connection;

use crate::{
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    DiagnosticsLedgerError, DiagnosticsLedgerRepository, DiagnosticsQuery, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement,
    OutputMeasurementUnavailableReason, OutputModality, ProjectionStateUpdate, ProjectionStatus,
    PruneTimingObservationsCommand, PruneUsageEventsCommand, RetentionClass,
    RetentionPolicyChangedPayload, RunSnapshotAcceptedPayload, SchedulerEstimateProducedPayload,
    SchedulerQueuePlacementPayload, SchedulerTimelineProjectionQuery, SqliteDiagnosticsLedger,
    UsageEventStatus, UsageLineage, WorkflowRunSummaryQuery, WorkflowRunSummaryRecord,
    WorkflowRunSummaryStatus, WorkflowTimingExpectation, WorkflowTimingExpectationComparison,
    WorkflowTimingExpectationQuery, WorkflowTimingObservation, WorkflowTimingObservationScope,
    WorkflowTimingObservationStatus, DEFAULT_STANDARD_RETENTION_DAYS,
    MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES, SCHEDULER_TIMELINE_PROJECTION_NAME,
    SCHEDULER_TIMELINE_PROJECTION_VERSION,
};

#[test]
fn record_and_query_usage_event_preserves_snapshot_and_measurement() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let event = sample_event("usage_alpha", "model-a", 10, 20);

    ledger
        .record_usage_event(event.clone())
        .expect("event is recorded");

    let projection = ledger
        .query_usage_events(DiagnosticsQuery {
            model_id: Some("model-a".to_string()),
            ..DiagnosticsQuery::default()
        })
        .expect("events query succeeds");

    assert_eq!(projection.events, vec![event]);
    assert!(projection.may_have_pruned_usage);

    let version_projection = ledger
        .query_usage_events(DiagnosticsQuery {
            workflow_version_id: Some(
                WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap(),
            ),
            workflow_semantic_version: Some("1.0.0".to_string()),
            ..DiagnosticsQuery::default()
        })
        .expect("version query succeeds");
    assert_eq!(version_projection.events.len(), 1);
}

#[test]
fn license_snapshot_is_time_of_use_and_not_rewritten_by_later_events() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let mut original = sample_event("usage_original", "model-a", 10, 20);
    original.license_snapshot.license_value = Some("mit".to_string());
    let mut later = sample_event("usage_later", "model-a", 30, 40);
    later.license_snapshot.license_value = Some("apache-2.0".to_string());

    ledger
        .record_usage_event(original.clone())
        .expect("original event is recorded");
    ledger
        .record_usage_event(later)
        .expect("later event is recorded");

    let projection = ledger
        .query_usage_events(DiagnosticsQuery {
            license_value: Some("mit".to_string()),
            ..DiagnosticsQuery::default()
        })
        .expect("license query succeeds");

    assert_eq!(projection.events, vec![original]);
}

#[test]
fn query_usage_events_filters_by_node_contract_version_and_digest() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let matched = sample_event("usage_matched", "model-a", 10, 20);
    let mut other_version = sample_event("usage_other_version", "model-a", 30, 40);
    other_version.lineage.effective_contract_version = Some("2".to_string());
    let mut other_digest = sample_event("usage_other_digest", "model-a", 50, 60);
    other_digest.lineage.effective_contract_digest = Some("digest-2".to_string());

    ledger
        .record_usage_event(matched.clone())
        .expect("matched event is recorded");
    ledger
        .record_usage_event(other_version)
        .expect("other version event is recorded");
    ledger
        .record_usage_event(other_digest)
        .expect("other digest event is recorded");

    let projection = ledger
        .query_usage_events(DiagnosticsQuery {
            node_contract_version: Some("1".to_string()),
            node_contract_digest: Some("digest-1".to_string()),
            ..DiagnosticsQuery::default()
        })
        .expect("node contract query succeeds");

    assert_eq!(projection.events, vec![matched]);
}

#[test]
fn query_rejects_unbounded_page_size_and_invalid_time_range() {
    let ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let oversized = ledger.query_usage_events(DiagnosticsQuery {
        page_size: 501,
        ..DiagnosticsQuery::default()
    });
    assert!(matches!(
        oversized,
        Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: 501,
            max: 500
        })
    ));

    let invalid_range = ledger.query_usage_events(DiagnosticsQuery {
        started_at_ms: Some(10),
        ended_before_ms: Some(10),
        ..DiagnosticsQuery::default()
    });
    assert!(matches!(
        invalid_range,
        Err(DiagnosticsLedgerError::InvalidTimeRange)
    ));
}

#[test]
fn prune_deletes_complete_events_without_rewriting_retained_snapshots() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let pruned = sample_event("usage_pruned", "model-a", 10, 20);
    let retained = sample_event("usage_retained", "model-b", 100, 200);
    ledger
        .record_usage_event(pruned)
        .expect("old event is recorded");
    ledger
        .record_usage_event(retained.clone())
        .expect("retained event is recorded");

    let result = ledger
        .prune_usage_events(PruneUsageEventsCommand {
            retention_class: RetentionClass::Standard,
            prune_completed_before_ms: 50,
        })
        .expect("prune succeeds");

    assert_eq!(result.pruned_event_count, 1);
    let projection = ledger
        .query_usage_events(DiagnosticsQuery::default())
        .expect("events query succeeds");
    assert_eq!(projection.events, vec![retained]);
}

#[test]
fn persisted_events_survive_reopen() {
    let temp = tempfile::NamedTempFile::new().expect("temp file");
    let path = temp.path().to_path_buf();
    let event = sample_event("usage_persisted", "model-a", 10, 20);

    {
        let mut ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger opens");
        ledger
            .record_usage_event(event.clone())
            .expect("event is recorded");
    }

    let ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger reopens");
    let projection = ledger
        .query_usage_events(DiagnosticsQuery::default())
        .expect("events query succeeds");

    assert_eq!(projection.events, vec![event]);
}

#[test]
fn persisted_timing_observations_survive_reopen_and_project_expectations() {
    let temp = tempfile::NamedTempFile::new().expect("temp file");
    let path = temp.path().to_path_buf();

    {
        let mut ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger opens");
        for (index, duration_ms) in [100, 200, 220, 300, 500].into_iter().enumerate() {
            ledger
                .record_timing_observation(sample_timing_observation(index, duration_ms))
                .expect("timing observation is recorded");
        }
    }

    let ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger reopens");
    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(450)))
        .expect("timing expectation projects");

    assert_eq!(expectation.sample_count, 5);
    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::SlowerThanExpected
    );
    assert_eq!(expectation.median_duration_ms, Some(220));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

#[test]
fn timing_expectation_matches_unknown_optional_runtime_history() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    for (index, duration_ms) in [100, 200].into_iter().enumerate() {
        let mut observation = sample_timing_observation(index, duration_ms);
        observation.runtime_id = None;
        ledger
            .record_timing_observation(observation)
            .expect("unknown runtime observation is recorded");
    }
    let mut matching_runtime = sample_timing_observation(3, 300);
    matching_runtime.runtime_id = Some("llama.cpp".to_string());
    ledger
        .record_timing_observation(matching_runtime)
        .expect("matching runtime observation is recorded");
    let mut unrelated_runtime = sample_timing_observation(4, 500);
    unrelated_runtime.runtime_id = Some("pytorch".to_string());
    ledger
        .record_timing_observation(unrelated_runtime)
        .expect("unrelated runtime observation is recorded");

    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(350)))
        .expect("timing expectation projects");

    assert_eq!(expectation.sample_count, 3);
    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::SlowerThanExpected
    );
    assert_eq!(expectation.median_duration_ms, Some(200));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

#[test]
fn timing_expectation_falls_back_when_runtime_refinement_has_too_little_history() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    for (index, duration_ms) in [100, 200, 300].into_iter().enumerate() {
        let mut observation = sample_timing_observation(index, duration_ms);
        observation.runtime_id = Some("pytorch".to_string());
        ledger
            .record_timing_observation(observation)
            .expect("runtime observation is recorded");
    }

    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(250)))
        .expect("timing expectation projects");

    assert_eq!(expectation.sample_count, 3);
    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::WithinExpectedRange
    );
    assert_eq!(expectation.median_duration_ms, Some(200));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

#[test]
fn lists_distinct_workflow_ids_for_timing_graph_fingerprint() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let mut first = sample_timing_observation(1, 100);
    first.workflow_id = "workflow-a".to_string();
    let mut duplicate = sample_timing_observation(2, 200);
    duplicate.workflow_id = "workflow-a".to_string();
    let mut second = sample_timing_observation(3, 300);
    second.workflow_id = "workflow-b".to_string();

    ledger
        .record_timing_observation(first)
        .expect("first timing observation is recorded");
    ledger
        .record_timing_observation(duplicate)
        .expect("duplicate workflow timing observation is recorded");
    ledger
        .record_timing_observation(second)
        .expect("second workflow timing observation is recorded");

    let workflow_ids = ledger
        .workflow_ids_for_timing_graph_fingerprint("graph_alpha")
        .expect("workflow ids load");

    assert_eq!(workflow_ids, vec!["workflow-a", "workflow-b"]);
}

#[test]
fn duplicate_timing_observation_does_not_inflate_history() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let observation = sample_timing_observation(1, 200);

    ledger
        .record_timing_observation(observation.clone())
        .expect("timing observation is recorded");
    ledger
        .record_timing_observation(observation)
        .expect("duplicate timing observation is ignored");

    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(200)))
        .expect("timing expectation projects");

    assert_eq!(expectation.sample_count, 1);
    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::InsufficientHistory
    );
}

#[test]
fn prune_timing_observations_deletes_old_observations() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let mut old = sample_timing_observation(1, 200);
    old.recorded_at_ms = 10;
    let mut retained = sample_timing_observation(2, 300);
    retained.recorded_at_ms = 100;

    ledger
        .record_timing_observation(old)
        .expect("old observation is recorded");
    ledger
        .record_timing_observation(retained)
        .expect("retained observation is recorded");

    let result = ledger
        .prune_timing_observations(PruneTimingObservationsCommand {
            prune_recorded_before_ms: 50,
        })
        .expect("timing prune succeeds");

    assert_eq!(result.pruned_observation_count, 1);
    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(300)))
        .expect("timing expectation projects");
    assert_eq!(expectation.sample_count, 1);
}

#[test]
fn existing_v1_schema_migrates_to_timing_observation_schema() {
    let conn = Connection::open_in_memory().expect("connection opens");
    conn.execute_batch(
        "CREATE TABLE ledger_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );
        INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
        VALUES (1, 0, 'pantograph-diagnostics-ledger-v1');",
    )
    .expect("v1 schema marker is installed");

    let mut ledger = SqliteDiagnosticsLedger::from_connection(conn).expect("ledger migrates");
    ledger
        .record_timing_observation(sample_timing_observation(1, 200))
        .expect("timing observation can be recorded after migration");
}

#[test]
fn existing_v2_timing_schema_drops_incompatible_timing_rows() {
    let conn = Connection::open_in_memory().expect("connection opens");
    conn.execute_batch(
        "CREATE TABLE ledger_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );
        INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
        VALUES (2, 0, 'pantograph-diagnostics-ledger-v2');
        CREATE TABLE workflow_timing_observations (
            observation_key TEXT PRIMARY KEY,
            observation_scope TEXT NOT NULL,
            execution_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            workflow_name TEXT,
            graph_fingerprint TEXT NOT NULL,
            node_id TEXT,
            node_type TEXT,
            runtime_id TEXT,
            status TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL,
            ended_at_ms INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            recorded_at_ms INTEGER NOT NULL
        );
        INSERT INTO workflow_timing_observations
            (observation_key, observation_scope, execution_id, workflow_id, workflow_name,
             graph_fingerprint, node_id, node_type, runtime_id, status, started_at_ms,
             ended_at_ms, duration_ms, recorded_at_ms)
        VALUES
            ('node:old-run:node-1', 'node', 'old-run', 'workflow_alpha', 'Workflow Alpha',
             'graph_alpha', 'node-1', 'text-generation', 'llama.cpp', 'completed',
             1000, 1200, 200, 2000);",
    )
    .expect("v2 timing schema is installed");

    let mut ledger = SqliteDiagnosticsLedger::from_connection(conn).expect("ledger migrates");

    let expectation = ledger
        .timing_expectation(sample_timing_query(Some(200)))
        .expect("timing expectation projects after migration");
    assert_eq!(expectation.sample_count, 0);
    ledger
        .record_timing_observation(sample_timing_observation(1, 200))
        .expect("new workflow_run_id timing observation can be recorded");
}

#[test]
fn workflow_run_summary_upsert_and_query_preserves_latest_run_state() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let mut record = sample_run_summary("run-1", "workflow_alpha", 1_000);

    ledger
        .upsert_workflow_run_summary(record.clone())
        .expect("run summary is recorded");
    record.status = WorkflowRunSummaryStatus::Completed;
    record.ended_at_ms = Some(1_250);
    record.duration_ms = Some(250);
    record.event_count = 5;
    record.recorded_at_ms = 1_260;
    ledger
        .upsert_workflow_run_summary(record.clone())
        .expect("run summary is updated");

    let projection = ledger
        .query_workflow_run_summaries(WorkflowRunSummaryQuery {
            workflow_id: Some("workflow_alpha".to_string()),
            workflow_run_id: None,
            limit: 10,
        })
        .expect("run summaries query succeeds");

    assert_eq!(projection.runs, vec![record]);
}

#[test]
fn diagnostic_event_ledger_appends_typed_events_and_reads_by_cursor() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let first = ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_alpha"))
        .expect("first diagnostic event appends");
    let second = ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_beta"))
        .expect("second diagnostic event appends");

    assert!(first.event_seq > 0);
    assert!(second.event_seq > first.event_seq);
    assert_eq!(
        first.event_kind,
        DiagnosticEventKind::SchedulerEstimateProduced
    );
    assert_eq!(first.schema_version, 1);
    assert!(first.payload_hash.starts_with("diagnostic-event-blake3:"));
    assert_eq!(first.payload_size_bytes, first.payload_json.len() as u64);

    let events = ledger
        .diagnostic_events_after(0, 10)
        .expect("diagnostic events load");
    assert_eq!(events, vec![first.clone(), second.clone()]);

    let after_first = ledger
        .diagnostic_events_after(first.event_seq, 10)
        .expect("diagnostic events load after first cursor");
    assert_eq!(after_first, vec![second]);

    let after_second = ledger
        .diagnostic_events_after(after_first[0].event_seq, 10)
        .expect("diagnostic events load after second cursor");
    assert!(after_second.is_empty());
}

#[test]
fn diagnostic_event_ledger_rejects_unbounded_cursor_queries() {
    let ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let oversized = ledger.diagnostic_events_after(0, 501);
    assert!(matches!(
        oversized,
        Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: 501,
            max: 500
        })
    ));

    let negative_cursor = ledger.diagnostic_events_after(-1, 10);
    assert!(matches!(
        negative_cursor,
        Err(DiagnosticsLedgerError::InvalidField {
            field: "last_event_seq"
        })
    ));
}

#[test]
fn diagnostic_event_ledger_validates_run_scope_and_event_source() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let mut missing_run = sample_scheduler_event("workflow_run_alpha");
    missing_run.workflow_run_id = None;

    let result = ledger.append_diagnostic_event(missing_run);
    assert!(matches!(
        result,
        Err(DiagnosticsLedgerError::MissingField {
            field: "workflow_run_id"
        })
    ));

    let mut wrong_source = sample_scheduler_event("workflow_run_alpha");
    wrong_source.source_component = DiagnosticEventSourceComponent::Library;

    let result = ledger.append_diagnostic_event(wrong_source);
    assert!(matches!(
        result,
        Err(DiagnosticsLedgerError::InvalidEventSource {
            event_kind: "scheduler.estimate_produced",
            source_component: "library"
        })
    ));
}

#[test]
fn diagnostic_event_ledger_rejects_oversized_payloads() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let request = DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Retention,
        source_instance_id: Some("retention-local".to_string()),
        occurred_at_ms: 10,
        workflow_run_id: None,
        workflow_id: None,
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: None,
        client_session_id: None,
        bucket_id: None,
        scheduler_policy_id: None,
        retention_policy_id: Some("retention_standard".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RetentionPolicyChanged(RetentionPolicyChangedPayload {
            policy_id: "retention_standard".to_string(),
            reason: "x".repeat(MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES + 1),
        }),
    };

    let result = ledger.append_diagnostic_event(request);
    assert!(matches!(
        result,
        Err(DiagnosticsLedgerError::EventPayloadTooLarge { max })
            if max == MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES
    ));
}

#[test]
fn projection_state_tracks_incremental_event_cursors() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let event = ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_alpha"))
        .expect("diagnostic event appends");

    let current = ledger
        .upsert_projection_state(ProjectionStateUpdate {
            projection_name: "scheduler_timeline".to_string(),
            projection_version: 1,
            last_applied_event_seq: event.event_seq,
            status: ProjectionStatus::Current,
            rebuilt_at_ms: Some(20),
        })
        .expect("projection state stores");
    assert_eq!(current.projection_name, "scheduler_timeline");
    assert_eq!(current.last_applied_event_seq, event.event_seq);
    assert_eq!(current.status, ProjectionStatus::Current);

    let loaded = ledger
        .projection_state("scheduler_timeline")
        .expect("projection state query succeeds")
        .expect("projection state exists");
    assert_eq!(loaded, current);

    let needs_rebuild = ledger
        .upsert_projection_state(ProjectionStateUpdate {
            projection_name: "scheduler_timeline".to_string(),
            projection_version: 2,
            last_applied_event_seq: 0,
            status: ProjectionStatus::NeedsRebuild,
            rebuilt_at_ms: None,
        })
        .expect("projection state updates");
    assert_eq!(needs_rebuild.projection_version, 2);
    assert_eq!(needs_rebuild.last_applied_event_seq, 0);
    assert_eq!(needs_rebuild.status, ProjectionStatus::NeedsRebuild);
}

#[test]
fn scheduler_timeline_projection_drains_events_incrementally() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let snapshot_event = ledger
        .append_diagnostic_event(sample_run_snapshot_event("workflow_run_alpha"))
        .expect("run snapshot event appends");
    let estimate_event = ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_alpha"))
        .expect("scheduler estimate event appends");
    let queue_event = ledger
        .append_diagnostic_event(sample_scheduler_queue_event("workflow_run_alpha", 0))
        .expect("scheduler queue event appends");

    let state = ledger
        .drain_scheduler_timeline_projection(10)
        .expect("scheduler timeline projection drains");
    assert_eq!(state.projection_name, SCHEDULER_TIMELINE_PROJECTION_NAME);
    assert_eq!(
        state.projection_version,
        SCHEDULER_TIMELINE_PROJECTION_VERSION
    );
    assert_eq!(state.last_applied_event_seq, queue_event.event_seq);
    assert_eq!(state.status, ProjectionStatus::Current);

    let records = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            ..SchedulerTimelineProjectionQuery::default()
        })
        .expect("scheduler timeline projection loads");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].event_seq, snapshot_event.event_seq);
    assert_eq!(records[0].summary, "run snapshot accepted");
    assert_eq!(records[1].event_seq, estimate_event.event_seq);
    assert_eq!(records[1].summary, "scheduler estimate produced");
    assert_eq!(records[1].detail.as_deref(), Some("model already loaded"));
    assert_eq!(records[2].event_seq, queue_event.event_seq);
    assert_eq!(records[2].summary, "queued at position 0");
    assert_eq!(records[2].detail.as_deref(), Some("priority 7"));

    let after_first = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
            after_event_seq: Some(snapshot_event.event_seq),
            ..SchedulerTimelineProjectionQuery::default()
        })
        .expect("scheduler timeline projection cursor query loads");
    assert_eq!(after_first.len(), 2);

    let no_new_state = ledger
        .drain_scheduler_timeline_projection(10)
        .expect("scheduler timeline projection drains idempotently");
    assert_eq!(no_new_state.last_applied_event_seq, queue_event.event_seq);
    let records_after_duplicate_drain = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery::default())
        .expect("scheduler timeline projection loads after duplicate drain");
    assert_eq!(records_after_duplicate_drain.len(), 3);

    let later_event = ledger
        .append_diagnostic_event(sample_scheduler_queue_event("workflow_run_alpha", 1))
        .expect("later scheduler queue event appends");
    let later_state = ledger
        .drain_scheduler_timeline_projection(10)
        .expect("scheduler timeline projection drains later event");
    assert_eq!(later_state.last_applied_event_seq, later_event.event_seq);
    let records_after_later_event = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery::default())
        .expect("scheduler timeline projection loads after later event");
    assert_eq!(records_after_later_event.len(), 4);
}

#[test]
fn scheduler_timeline_projection_query_rejects_unbounded_requests() {
    let ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let oversized = ledger.query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
        limit: 501,
        ..SchedulerTimelineProjectionQuery::default()
    });
    assert!(matches!(
        oversized,
        Err(DiagnosticsLedgerError::QueryLimitExceeded {
            requested: 501,
            max: 500
        })
    ));

    let negative_cursor =
        ledger.query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
            after_event_seq: Some(-1),
            ..SchedulerTimelineProjectionQuery::default()
        });
    assert!(matches!(
        negative_cursor,
        Err(DiagnosticsLedgerError::InvalidField {
            field: "after_event_seq"
        })
    ));
}

#[test]
fn existing_v5_schema_adds_usage_lineage_contract_indexes() {
    let temp = tempfile::NamedTempFile::new().expect("temp file");
    let path = temp.path().to_path_buf();
    {
        let conn = Connection::open(&path).expect("connection opens");
        conn.execute_batch(
            "CREATE TABLE ledger_schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL,
                checksum TEXT NOT NULL
            );
            INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
            VALUES (5, 0, 'pantograph-diagnostics-ledger-v5');
            CREATE TABLE usage_lineage (
                usage_event_id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                node_type TEXT NOT NULL,
                port_ids_json TEXT NOT NULL,
                composed_parent_chain_json TEXT NOT NULL,
                effective_contract_version TEXT,
                effective_contract_digest TEXT,
                metadata_json TEXT
            );",
        )
        .expect("v5 usage lineage schema is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }

    let conn = Connection::open(&path).expect("connection reopens");
    let has_version_index = sqlite_index_exists(&conn, "idx_usage_lineage_contract_version");
    let has_digest_index = sqlite_index_exists(&conn, "idx_usage_lineage_contract_digest");

    assert!(has_version_index);
    assert!(has_digest_index);
}

#[test]
fn existing_v6_schema_adds_diagnostic_event_ledger_tables() {
    let conn = Connection::open_in_memory().expect("connection opens");
    conn.execute_batch(
        "CREATE TABLE ledger_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );
        INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
        VALUES (6, 0, 'pantograph-diagnostics-ledger-v6');",
    )
    .expect("v6 schema marker is installed");

    let mut ledger = SqliteDiagnosticsLedger::from_connection(conn).expect("ledger migrates");
    let event = ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_alpha"))
        .expect("diagnostic event appends after migration");
    ledger
        .upsert_projection_state(ProjectionStateUpdate {
            projection_name: "scheduler_timeline".to_string(),
            projection_version: 1,
            last_applied_event_seq: event.event_seq,
            status: ProjectionStatus::Current,
            rebuilt_at_ms: None,
        })
        .expect("projection state stores after migration");
}

#[test]
fn existing_v7_schema_adds_scheduler_timeline_projection_table() {
    let temp = tempfile::NamedTempFile::new().expect("temp file");
    let path = temp.path().to_path_buf();
    {
        let conn = Connection::open(&path).expect("connection opens");
        conn.execute_batch(
            "CREATE TABLE ledger_schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL,
                checksum TEXT NOT NULL
            );
            INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
            VALUES (7, 0, 'pantograph-diagnostics-ledger-v7');",
        )
        .expect("v7 schema marker is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_table_exists(&conn, "scheduler_timeline_projection"));
    assert!(sqlite_index_exists(&conn, "idx_scheduler_timeline_run_seq"));
}

#[test]
fn unsupported_schema_version_is_rejected() {
    let conn = Connection::open_in_memory().expect("connection opens");
    conn.execute_batch(
        "CREATE TABLE ledger_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );
        INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
        VALUES (999, 0, 'future');",
    )
    .expect("future schema is installed");

    let result = SqliteDiagnosticsLedger::from_connection(conn);

    assert!(matches!(
        result,
        Err(DiagnosticsLedgerError::UnsupportedSchemaVersion { found: 999 })
    ));
}

fn sqlite_index_exists(conn: &Connection, index_name: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1
        )",
        [index_name],
        |row| row.get::<_, bool>(0),
    )
    .expect("index lookup succeeds")
}

fn sqlite_table_exists(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        [table_name],
        |row| row.get::<_, bool>(0),
    )
    .expect("table lookup succeeds")
}

#[test]
fn retention_policy_uses_standard_local_default() {
    let ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let policy = ledger.retention_policy().expect("policy loads");

    assert_eq!(policy.retention_class, RetentionClass::Standard);
    assert_eq!(policy.retention_days, DEFAULT_STANDARD_RETENTION_DAYS);
}

#[test]
fn timing_expectation_reports_insufficient_history_until_minimum_samples_exist() {
    let query = sample_timing_query(Some(150));

    let expectation = WorkflowTimingExpectation::from_completed_durations(&query, vec![100, 200]);

    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::InsufficientHistory
    );
    assert_eq!(expectation.sample_count, 2);
    assert_eq!(expectation.current_duration_ms, Some(150));
    assert_eq!(expectation.median_duration_ms, None);
    assert_eq!(expectation.typical_min_duration_ms, None);
    assert_eq!(expectation.typical_max_duration_ms, None);
}

#[test]
fn timing_expectation_classifies_current_duration_against_typical_range() {
    let within = WorkflowTimingExpectation::from_completed_durations(
        &sample_timing_query(Some(220)),
        vec![100, 200, 220, 300, 500],
    );
    assert_eq!(
        within.comparison,
        WorkflowTimingExpectationComparison::WithinExpectedRange
    );
    assert_eq!(within.median_duration_ms, Some(220));
    assert_eq!(within.typical_min_duration_ms, Some(200));
    assert_eq!(within.typical_max_duration_ms, Some(300));

    let faster = WorkflowTimingExpectation::from_completed_durations(
        &sample_timing_query(Some(120)),
        vec![100, 200, 220, 300, 500],
    );
    assert_eq!(
        faster.comparison,
        WorkflowTimingExpectationComparison::FasterThanExpected
    );

    let slower = WorkflowTimingExpectation::from_completed_durations(
        &sample_timing_query(Some(450)),
        vec![100, 200, 220, 300, 500],
    );
    assert_eq!(
        slower.comparison,
        WorkflowTimingExpectationComparison::SlowerThanExpected
    );
}

#[test]
fn timing_expectation_does_not_report_incomplete_duration_as_faster_than_usual() {
    let mut query = sample_timing_query(Some(120));
    query.current_duration_is_complete = false;

    let expectation =
        WorkflowTimingExpectation::from_completed_durations(&query, vec![100, 200, 220, 300, 500]);

    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::WithinExpectedRange
    );

    query.current_duration_ms = Some(550);
    let overdue =
        WorkflowTimingExpectation::from_completed_durations(&query, vec![100, 200, 220, 300, 500]);
    assert_eq!(
        overdue.comparison,
        WorkflowTimingExpectationComparison::SlowerThanExpected
    );
}

fn sample_timing_query(current_duration_ms: Option<u64>) -> WorkflowTimingExpectationQuery {
    WorkflowTimingExpectationQuery {
        scope: WorkflowTimingObservationScope::Node,
        workflow_id: "workflow_alpha".to_string(),
        graph_fingerprint: "graph_alpha".to_string(),
        node_id: Some("node-1".to_string()),
        node_type: Some("text-generation".to_string()),
        runtime_id: Some("llama.cpp".to_string()),
        current_duration_ms,
        current_duration_is_complete: true,
    }
}

fn sample_timing_observation(index: usize, duration_ms: u64) -> WorkflowTimingObservation {
    WorkflowTimingObservation {
        observation_key: format!("node:exec-{index}:node-1"),
        scope: WorkflowTimingObservationScope::Node,
        workflow_run_id: format!("exec-{index}"),
        workflow_id: "workflow_alpha".to_string(),
        graph_fingerprint: "graph_alpha".to_string(),
        node_id: Some("node-1".to_string()),
        node_type: Some("text-generation".to_string()),
        runtime_id: Some("llama.cpp".to_string()),
        status: WorkflowTimingObservationStatus::Completed,
        started_at_ms: 1_000,
        ended_at_ms: 1_000 + duration_ms as i64,
        duration_ms,
        recorded_at_ms: 2_000 + index as i64,
    }
}

fn sample_run_summary(
    workflow_run_id: &str,
    workflow_id: &str,
    started_at_ms: i64,
) -> WorkflowRunSummaryRecord {
    WorkflowRunSummaryRecord {
        workflow_run_id: workflow_run_id.to_string(),
        workflow_id: workflow_id.to_string(),
        session_id: Some("session_alpha".to_string()),
        graph_fingerprint: Some("graph_alpha".to_string()),
        status: WorkflowRunSummaryStatus::Running,
        started_at_ms,
        ended_at_ms: None,
        duration_ms: None,
        node_count_at_start: 2,
        event_count: 1,
        last_error: None,
        recorded_at_ms: started_at_ms,
    }
}

fn sample_scheduler_event(workflow_run_id: &str) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("scheduler-local".to_string()),
        occurred_at_ms: 1_000,
        workflow_run_id: Some(WorkflowRunId::try_from(workflow_run_id.to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client_alpha".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session_alpha".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket_alpha".to_string()).unwrap()),
        scheduler_policy_id: Some("scheduler_default".to_string()),
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::SchedulerEstimateProduced(
            SchedulerEstimateProducedPayload {
                estimate_version: "estimate-v1".to_string(),
                confidence: "medium".to_string(),
                estimated_queue_wait_ms: Some(1_500),
                estimated_duration_ms: Some(2_500),
                reasons: vec!["model already loaded".to_string()],
            },
        ),
    }
}

fn sample_scheduler_queue_event(
    workflow_run_id: &str,
    queue_position: u32,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("scheduler-local".to_string()),
        occurred_at_ms: 1_010 + i64::from(queue_position),
        workflow_run_id: Some(WorkflowRunId::try_from(workflow_run_id.to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client_alpha".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session_alpha".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket_alpha".to_string()).unwrap()),
        scheduler_policy_id: Some("scheduler_default".to_string()),
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::SchedulerQueuePlacement(SchedulerQueuePlacementPayload {
            queue_position,
            priority: 7,
            scheduler_policy_id: "scheduler_default".to_string(),
        }),
    }
}

fn sample_run_snapshot_event(workflow_run_id: &str) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::WorkflowService,
        source_instance_id: Some("workflow-service".to_string()),
        occurred_at_ms: 990,
        workflow_run_id: Some(WorkflowRunId::try_from(workflow_run_id.to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client_alpha".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session_alpha".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket_alpha".to_string()).unwrap()),
        scheduler_policy_id: Some("scheduler_default".to_string()),
        retention_policy_id: Some("retention_default".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RunSnapshotAccepted(RunSnapshotAcceptedPayload {
            workflow_run_snapshot_id: "runsnap_alpha".to_string(),
            workflow_presentation_revision_id: "wfpres_alpha".to_string(),
        }),
    }
}

fn sample_event(
    usage_suffix: &str,
    model_id: &str,
    started_at_ms: i64,
    completed_at_ms: i64,
) -> ModelLicenseUsageEvent {
    ModelLicenseUsageEvent {
        usage_event_id: UsageEventId::try_from(usage_suffix.to_string()).unwrap(),
        client_id: ClientId::try_from("client_alpha".to_string()).unwrap(),
        client_session_id: ClientSessionId::try_from("session_alpha".to_string()).unwrap(),
        bucket_id: BucketId::try_from("bucket_alpha".to_string()).unwrap(),
        workflow_run_id: WorkflowRunId::try_from("run_alpha".to_string()).unwrap(),
        workflow_id: WorkflowId::try_from("workflow_alpha".to_string()).unwrap(),
        workflow_version_id: Some(
            pantograph_runtime_attribution::WorkflowVersionId::try_from("wfver_alpha".to_string())
                .unwrap(),
        ),
        workflow_semantic_version: Some("1.0.0".to_string()),
        model: ModelIdentity {
            model_id: model_id.to_string(),
            model_revision: Some("rev-1".to_string()),
            model_hash: Some("sha256:abc".to_string()),
            model_modality: Some("text".to_string()),
            runtime_backend: Some("pytorch".to_string()),
        },
        lineage: UsageLineage {
            node_id: "node-1".to_string(),
            node_type: "text-generation".to_string(),
            port_ids: vec!["out".to_string()],
            composed_parent_chain: vec!["parent-a".to_string()],
            effective_contract_version: Some("1".to_string()),
            effective_contract_digest: Some("digest-1".to_string()),
            metadata_json: Some(r#"{"path":"root/node-1"}"#.to_string()),
        },
        license_snapshot: LicenseSnapshot {
            license_value: Some("mit".to_string()),
            source_metadata_json: Some(r#"{"source":"pumas"}"#.to_string()),
            model_metadata_snapshot_json: Some(r#"{"model":"snapshot"}"#.to_string()),
            unavailable_reason: None,
        },
        output_measurement: ModelOutputMeasurement {
            modality: OutputModality::Text,
            item_count: Some(1),
            character_count: Some(42),
            byte_size: Some(42),
            token_count: None,
            width: None,
            height: None,
            pixel_count: None,
            duration_ms: None,
            sample_rate_hz: None,
            channels: None,
            frame_count: None,
            vector_count: None,
            dimensions: None,
            numeric_representation: None,
            top_level_shape: None,
            schema_id: None,
            schema_digest: None,
            unavailable_reasons: vec![OutputMeasurementUnavailableReason::TokenizerUnavailable],
        },
        guarantee_level: ExecutionGuaranteeLevel::ManagedFull,
        status: UsageEventStatus::Completed,
        retention_class: RetentionClass::Standard,
        started_at_ms,
        completed_at_ms: Some(completed_at_ms),
        correlation_id: Some("corr-1".to_string()),
    }
}

use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use rusqlite::Connection;

use crate::{
    DiagnosticEventAppendRequest, DiagnosticEventKind, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    DiagnosticsLedgerError, DiagnosticsLedgerRepository, DiagnosticsQuery, ExecutionGuaranteeLevel,
    IoArtifactObservedPayload, IoArtifactProjectionQuery, IoArtifactRetentionState,
    LibraryAssetAccessedPayload, LibraryUsageProjectionQuery, LicenseSnapshot, ModelIdentity,
    ModelLicenseUsageEvent, ModelOutputMeasurement, NodeExecutionProjectionStatus,
    NodeExecutionStatusPayload, NodeStatusProjectionQuery, OutputMeasurementUnavailableReason,
    OutputModality, ProjectionStateUpdate, ProjectionStatus, PruneTimingObservationsCommand,
    PruneUsageEventsCommand, RetentionArtifactStateChangedPayload, RetentionClass,
    RetentionPolicyChangedPayload, RunDetailProjectionQuery, RunListProjectionQuery,
    RunListProjectionStatus, RunSnapshotAcceptedPayload, RunStartedPayload, RunTerminalPayload,
    RunTerminalStatus, SchedulerEstimateProducedPayload, SchedulerQueuePlacementPayload,
    SchedulerTimelineProjectionQuery, SqliteDiagnosticsLedger, UpdateRetentionPolicyCommand,
    UsageEventStatus, UsageLineage, WorkflowRunSummaryQuery, WorkflowRunSummaryRecord,
    WorkflowRunSummaryStatus, WorkflowTimingExpectation, WorkflowTimingExpectationComparison,
    WorkflowTimingExpectationQuery, WorkflowTimingObservation, WorkflowTimingObservationScope,
    WorkflowTimingObservationStatus, DEFAULT_STANDARD_RETENTION_DAYS, IO_ARTIFACT_PROJECTION_NAME,
    IO_ARTIFACT_PROJECTION_VERSION, LIBRARY_USAGE_PROJECTION_NAME,
    LIBRARY_USAGE_PROJECTION_VERSION, MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES,
    NODE_STATUS_PROJECTION_NAME, NODE_STATUS_PROJECTION_VERSION, RUN_DETAIL_PROJECTION_NAME,
    RUN_DETAIL_PROJECTION_VERSION, RUN_LIST_PROJECTION_NAME, RUN_LIST_PROJECTION_VERSION,
    SCHEDULER_TIMELINE_PROJECTION_NAME, SCHEDULER_TIMELINE_PROJECTION_VERSION,
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
fn diagnostic_event_ledger_rejects_unsafe_payload_refs() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    for payload_ref in [
        "/tmp/artifact.bin",
        "file:///tmp/artifact.bin",
        "https://example.test/artifact.bin",
        "artifact://../artifact.bin",
        "artifact:///absolute",
        "artifact://with space",
    ] {
        let mut request = sample_io_artifact_event(
            "workflow_run_alpha",
            "node_prompt",
            "workflow_input",
            "artifact_prompt",
        );
        request.payload_ref = Some(payload_ref.to_string());

        let result = ledger.append_diagnostic_event(request);
        assert!(
            matches!(
                result,
                Err(DiagnosticsLedgerError::InvalidField {
                    field: "payload_ref"
                })
            ),
            "expected payload_ref {payload_ref:?} to be rejected"
        );
    }

    let mut safe_request = sample_io_artifact_event(
        "workflow_run_alpha",
        "node_prompt",
        "workflow_input",
        "artifact_prompt",
    );
    safe_request.payload_ref = Some("pantograph://artifacts/run-alpha/output".to_string());
    ledger
        .append_diagnostic_event(safe_request)
        .expect("safe payload ref appends");
}

#[test]
fn diagnostic_event_ledger_rejects_unsafe_library_asset_ids() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    for asset_id in [
        "/tmp/model.bin",
        "file:///tmp/model.bin",
        "https://example.test/model.bin",
        "org/../model",
        "org//model",
        "org\\model",
        "org/model with space",
    ] {
        let request = sample_library_asset_access_event(asset_id, Some("workflow_run_alpha"), 1);

        let result = ledger.append_diagnostic_event(request);
        assert!(
            matches!(
                result,
                Err(DiagnosticsLedgerError::InvalidField { field: "asset_id" })
            ),
            "expected asset_id {asset_id:?} to be rejected"
        );
    }

    for asset_id in [
        "org/model",
        "model:local",
        "pumas://models/org/model",
        "pantograph://library/local-model",
        "hf://org/model",
    ] {
        let request = sample_library_asset_access_event(asset_id, Some("workflow_run_alpha"), 1);
        ledger
            .append_diagnostic_event(request)
            .expect("safe library asset id appends");
    }
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
    let started_event = ledger
        .append_diagnostic_event(sample_run_started_event("workflow_run_alpha"))
        .expect("run started event appends");
    let terminal_event = ledger
        .append_diagnostic_event(sample_run_terminal_event("workflow_run_alpha"))
        .expect("run terminal event appends");

    let state = ledger
        .drain_scheduler_timeline_projection(10)
        .expect("scheduler timeline projection drains");
    assert_eq!(state.projection_name, SCHEDULER_TIMELINE_PROJECTION_NAME);
    assert_eq!(
        state.projection_version,
        SCHEDULER_TIMELINE_PROJECTION_VERSION
    );
    assert_eq!(state.last_applied_event_seq, terminal_event.event_seq);
    assert_eq!(state.status, ProjectionStatus::Current);

    let records = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            ..SchedulerTimelineProjectionQuery::default()
        })
        .expect("scheduler timeline projection loads");
    assert_eq!(records.len(), 5);
    assert_eq!(records[0].event_seq, snapshot_event.event_seq);
    assert_eq!(records[0].summary, "run snapshot accepted");
    assert_eq!(records[1].event_seq, estimate_event.event_seq);
    assert_eq!(records[1].summary, "scheduler estimate produced");
    assert_eq!(records[1].detail.as_deref(), Some("model already loaded"));
    assert_eq!(records[2].event_seq, queue_event.event_seq);
    assert_eq!(records[2].summary, "queued at position 0");
    assert_eq!(records[2].detail.as_deref(), Some("priority 7"));
    assert_eq!(records[3].event_seq, started_event.event_seq);
    assert_eq!(records[3].summary, "run started");
    assert_eq!(
        records[3].detail.as_deref(),
        Some("queue wait 10 ms; warm_session_reused")
    );
    assert_eq!(records[4].event_seq, terminal_event.event_seq);
    assert_eq!(records[4].summary, "run completed");
    assert_eq!(records[4].detail.as_deref(), None);

    let after_first = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery {
            after_event_seq: Some(snapshot_event.event_seq),
            ..SchedulerTimelineProjectionQuery::default()
        })
        .expect("scheduler timeline projection cursor query loads");
    assert_eq!(after_first.len(), 4);

    let no_new_state = ledger
        .drain_scheduler_timeline_projection(10)
        .expect("scheduler timeline projection drains idempotently");
    assert_eq!(
        no_new_state.last_applied_event_seq,
        terminal_event.event_seq
    );
    let records_after_duplicate_drain = ledger
        .query_scheduler_timeline_projection(SchedulerTimelineProjectionQuery::default())
        .expect("scheduler timeline projection loads after duplicate drain");
    assert_eq!(records_after_duplicate_drain.len(), 5);

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
    assert_eq!(records_after_later_event.len(), 6);
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
fn run_list_projection_drains_lifecycle_events_incrementally() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event("workflow_run_alpha"))
        .expect("run snapshot event appends");
    ledger
        .append_diagnostic_event(sample_scheduler_queue_event("workflow_run_alpha", 0))
        .expect("scheduler queue event appends");
    ledger
        .append_diagnostic_event(sample_run_started_event("workflow_run_alpha"))
        .expect("run started event appends");
    let terminal_event = ledger
        .append_diagnostic_event(sample_run_terminal_event("workflow_run_alpha"))
        .expect("run terminal event appends");

    let state = ledger
        .drain_run_list_projection(10)
        .expect("run list projection drains");
    assert_eq!(state.projection_name, RUN_LIST_PROJECTION_NAME);
    assert_eq!(state.projection_version, RUN_LIST_PROJECTION_VERSION);
    assert_eq!(state.last_applied_event_seq, terminal_event.event_seq);

    let records = ledger
        .query_run_list_projection(RunListProjectionQuery::default())
        .expect("run list projection loads");
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.workflow_run_id.as_str(), "workflow_run_alpha");
    assert_eq!(record.workflow_id.as_str(), "workflow_alpha");
    assert_eq!(record.status, RunListProjectionStatus::Completed);
    assert_eq!(record.accepted_at_ms, Some(990));
    assert_eq!(record.enqueued_at_ms, Some(1_010));
    assert_eq!(record.started_at_ms, Some(1_020));
    assert_eq!(record.completed_at_ms, Some(1_100));
    assert_eq!(record.duration_ms, Some(80));
    assert_eq!(record.last_event_seq, terminal_event.event_seq);
    assert_eq!(
        record.scheduler_policy_id.as_deref(),
        Some("scheduler_default")
    );
    assert_eq!(
        record.retention_policy_id.as_deref(),
        Some("retention_default")
    );
    assert_eq!(record.scheduler_queue_position, Some(0));
    assert_eq!(record.scheduler_priority, Some(7));
    assert_eq!(
        record.scheduler_reason.as_deref(),
        Some("warm_session_reused")
    );

    let completed = ledger
        .query_run_list_projection(RunListProjectionQuery {
            status: Some(RunListProjectionStatus::Completed),
            ..RunListProjectionQuery::default()
        })
        .expect("run list status filter loads");
    assert_eq!(completed.len(), 1);

    let retained = ledger
        .query_run_list_projection(RunListProjectionQuery {
            retention_policy_id: Some("retention_default".to_string()),
            ..RunListProjectionQuery::default()
        })
        .expect("run list retention filter loads");
    assert_eq!(retained.len(), 1);
}

#[test]
fn run_detail_projection_drains_lifecycle_events_incrementally() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event("workflow_run_alpha"))
        .expect("run snapshot event appends");
    ledger
        .append_diagnostic_event(sample_scheduler_event("workflow_run_alpha"))
        .expect("scheduler estimate event appends");
    ledger
        .append_diagnostic_event(sample_scheduler_queue_event("workflow_run_alpha", 0))
        .expect("scheduler queue event appends");
    ledger
        .append_diagnostic_event(sample_run_started_event("workflow_run_alpha"))
        .expect("run started event appends");
    let terminal_event = ledger
        .append_diagnostic_event(sample_run_terminal_event("workflow_run_alpha"))
        .expect("run terminal event appends");

    let state = ledger
        .drain_run_detail_projection(10)
        .expect("run detail projection drains");
    assert_eq!(state.projection_name, RUN_DETAIL_PROJECTION_NAME);
    assert_eq!(state.projection_version, RUN_DETAIL_PROJECTION_VERSION);
    assert_eq!(state.last_applied_event_seq, terminal_event.event_seq);

    let record = ledger
        .query_run_detail_projection(RunDetailProjectionQuery {
            workflow_run_id: WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
        })
        .expect("run detail projection loads")
        .expect("run detail exists");
    assert_eq!(record.workflow_run_id.as_str(), "workflow_run_alpha");
    assert_eq!(record.workflow_id.as_str(), "workflow_alpha");
    assert_eq!(record.status, RunListProjectionStatus::Completed);
    assert_eq!(record.accepted_at_ms, Some(990));
    assert_eq!(record.enqueued_at_ms, Some(1_010));
    assert_eq!(record.started_at_ms, Some(1_020));
    assert_eq!(record.completed_at_ms, Some(1_100));
    assert_eq!(record.duration_ms, Some(80));
    assert_eq!(
        record.client_id.as_ref().map(|id| id.as_str()),
        Some("client_alpha")
    );
    assert_eq!(
        record.client_session_id.as_ref().map(|id| id.as_str()),
        Some("session_alpha")
    );
    assert_eq!(
        record.bucket_id.as_ref().map(|id| id.as_str()),
        Some("bucket_alpha")
    );
    assert_eq!(
        record.workflow_run_snapshot_id.as_deref(),
        Some("runsnap_alpha")
    );
    assert_eq!(
        record.workflow_presentation_revision_id.as_deref(),
        Some("wfpres_alpha")
    );
    assert!(record.latest_estimate_json.is_some());
    assert!(record.latest_queue_placement_json.is_some());
    assert!(record.started_payload_json.is_some());
    assert!(record.terminal_payload_json.is_some());
    assert_eq!(record.scheduler_queue_position, Some(0));
    assert_eq!(record.scheduler_priority, Some(7));
    assert_eq!(record.estimate_confidence.as_deref(), Some("medium"));
    assert_eq!(record.estimated_queue_wait_ms, Some(1_500));
    assert_eq!(record.estimated_duration_ms, Some(2_500));
    assert_eq!(
        record.scheduler_reason.as_deref(),
        Some("warm_session_reused")
    );
    assert_eq!(record.timeline_event_count, 5);
    assert_eq!(record.last_event_seq, terminal_event.event_seq);

    let no_new_state = ledger
        .drain_run_detail_projection(10)
        .expect("run detail projection drains idempotently");
    assert_eq!(
        no_new_state.last_applied_event_seq,
        terminal_event.event_seq
    );
    let after_idempotent = ledger
        .query_run_detail_projection(RunDetailProjectionQuery {
            workflow_run_id: WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
        })
        .expect("run detail projection loads after idempotent drain")
        .expect("run detail exists after idempotent drain");
    assert_eq!(after_idempotent.timeline_event_count, 5);
}

#[test]
fn io_artifact_projection_drains_artifact_events_incrementally() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let input_event = ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "workflow_run_alpha",
            "node_prompt",
            "workflow_input",
            "artifact_prompt",
        ))
        .expect("input artifact event appends");
    let output_event = ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "workflow_run_alpha",
            "node_image",
            "node_output",
            "artifact_image",
        ))
        .expect("output artifact event appends");

    let state = ledger
        .drain_io_artifact_projection(10)
        .expect("io artifact projection drains");
    assert_eq!(state.projection_name, IO_ARTIFACT_PROJECTION_NAME);
    assert_eq!(state.projection_version, IO_ARTIFACT_PROJECTION_VERSION);
    assert_eq!(state.last_applied_event_seq, output_event.event_seq);

    let records = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("io artifact projection loads");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].event_seq, input_event.event_seq);
    assert_eq!(records[0].artifact_id, "artifact_prompt");
    assert_eq!(records[0].artifact_role, "workflow_input");
    assert_eq!(
        records[0].payload_ref.as_deref(),
        Some("artifact://artifact_prompt")
    );
    assert_eq!(
        records[0].retention_state,
        IoArtifactRetentionState::Retained
    );
    assert_eq!(records[0].retention_reason, None);
    assert_eq!(records[1].event_seq, output_event.event_seq);
    assert_eq!(records[1].media_type.as_deref(), Some("image/png"));
    assert_eq!(records[1].size_bytes, Some(1_024));

    let global_records = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: None,
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("global io artifact projection loads");
    assert_eq!(global_records.len(), 2);

    let node_records = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: Some("node_image".to_string()),
            artifact_role: None,
            media_type: Some("image/png".to_string()),
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_policy_id: Some("retention_default".to_string()),
            runtime_id: Some("runtime_alpha".to_string()),
            model_id: None,
            after_event_seq: Some(input_event.event_seq),
            limit: 10,
        })
        .expect("io artifact node filter loads");
    assert_eq!(node_records.len(), 1);
    assert_eq!(node_records[0].artifact_id, "artifact_image");

    let no_new_state = ledger
        .drain_io_artifact_projection(10)
        .expect("io artifact projection drains idempotently");
    assert_eq!(no_new_state.last_applied_event_seq, output_event.event_seq);
    let after_idempotent = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("io artifact projection loads after idempotent drain");
    assert_eq!(after_idempotent.len(), 2);
}

#[test]
fn io_artifact_projection_applies_retention_state_changes() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "workflow_run_alpha",
            "node_image",
            "node_output",
            "artifact_image",
        ))
        .expect("io artifact event appends");
    let retention_event = ledger
        .append_diagnostic_event(sample_retention_artifact_state_changed_event(
            "workflow_run_alpha",
            "artifact_image",
            IoArtifactRetentionState::Expired,
            "global retention window elapsed",
        ))
        .expect("retention state event appends");

    let state = ledger
        .drain_io_artifact_projection(10)
        .expect("io artifact projection drains retention state");
    assert_eq!(state.last_applied_event_seq, retention_event.event_seq);

    let records = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("io artifact projection loads after retention event");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event_seq, retention_event.event_seq);
    assert_eq!(records[0].artifact_id, "artifact_image");
    assert_eq!(records[0].artifact_role, "node_output");
    assert_eq!(records[0].payload_ref, None);
    assert_eq!(
        records[0].retention_state,
        IoArtifactRetentionState::Expired
    );
    assert_eq!(
        records[0].retention_reason.as_deref(),
        Some("global retention window elapsed")
    );

    ledger
        .rebuild_projection(IO_ARTIFACT_PROJECTION_NAME, 1)
        .expect("io artifact projection rebuilds from retention ledger events");
    let rebuilt_records = ledger
        .query_io_artifact_projection(IoArtifactProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Expired),
            retention_policy_id: Some("retention_default".to_string()),
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("rebuilt io artifact projection loads");
    assert_eq!(rebuilt_records.len(), 1);
    assert_eq!(rebuilt_records[0].event_seq, retention_event.event_seq);
    assert_eq!(rebuilt_records[0].payload_ref, None);
}

#[test]
fn node_status_projection_keeps_latest_status_per_node() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_node_status_event(
            "workflow_run_alpha",
            "node_image",
            NodeExecutionProjectionStatus::Running,
            10,
        ))
        .expect("running status appends");
    let completed_event = ledger
        .append_diagnostic_event(sample_node_status_event(
            "workflow_run_alpha",
            "node_image",
            NodeExecutionProjectionStatus::Completed,
            20,
        ))
        .expect("completed status appends");

    let state = ledger
        .drain_node_status_projection(10)
        .expect("node status projection drains");
    assert_eq!(state.projection_name, NODE_STATUS_PROJECTION_NAME);
    assert_eq!(state.projection_version, NODE_STATUS_PROJECTION_VERSION);
    assert_eq!(state.last_applied_event_seq, completed_event.event_seq);

    let records = ledger
        .query_node_status_projection(NodeStatusProjectionQuery {
            workflow_run_id: Some(
                WorkflowRunId::try_from("workflow_run_alpha".to_string()).unwrap(),
            ),
            node_id: Some("node_image".to_string()),
            status: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("node status projection loads");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].node_id, "node_image");
    assert_eq!(records[0].status, NodeExecutionProjectionStatus::Completed);
    assert_eq!(records[0].started_at_ms, Some(20));
    assert_eq!(records[0].completed_at_ms, Some(120));
    assert_eq!(records[0].duration_ms, Some(100));
    assert_eq!(records[0].last_event_seq, completed_event.event_seq);
}

#[test]
fn projection_rebuild_resets_projection_rows_and_cursor() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event("workflow_run_alpha"))
        .expect("run snapshot event appends");
    let terminal_event = ledger
        .append_diagnostic_event(sample_run_terminal_event("workflow_run_alpha"))
        .expect("run terminal event appends");
    ledger
        .drain_run_list_projection(10)
        .expect("run list projection drains");

    let stale_state = ledger
        .upsert_projection_state(ProjectionStateUpdate {
            projection_name: RUN_LIST_PROJECTION_NAME.to_string(),
            projection_version: RUN_LIST_PROJECTION_VERSION,
            last_applied_event_seq: 0,
            status: ProjectionStatus::NeedsRebuild,
            rebuilt_at_ms: None,
        })
        .expect("stale projection state stores");
    assert_eq!(stale_state.last_applied_event_seq, 0);

    let rebuilt = ledger
        .rebuild_projection(RUN_LIST_PROJECTION_NAME, 1)
        .expect("run list projection rebuilds");
    assert_eq!(rebuilt.projection_name, RUN_LIST_PROJECTION_NAME);
    assert_eq!(rebuilt.last_applied_event_seq, terminal_event.event_seq);
    assert_eq!(rebuilt.status, ProjectionStatus::Current);

    let records = ledger
        .query_run_list_projection(RunListProjectionQuery::default())
        .expect("rebuilt run list projection loads");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, RunListProjectionStatus::Completed);
}

#[test]
fn library_usage_projection_drains_asset_events_incrementally() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    let first_event = ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "asset_alpha",
            Some("workflow_run_alpha"),
            1_024,
        ))
        .expect("library asset access appends");
    let second_event = ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "asset_alpha",
            Some("workflow_run_alpha"),
            2_048,
        ))
        .expect("library asset access appends");
    let third_event = ledger
        .append_diagnostic_event(sample_library_asset_access_event("asset_beta", None, 0))
        .expect("library asset access appends");

    let state = ledger
        .drain_library_usage_projection(10)
        .expect("library usage projection drains");
    assert_eq!(state.projection_name, LIBRARY_USAGE_PROJECTION_NAME);
    assert_eq!(state.projection_version, LIBRARY_USAGE_PROJECTION_VERSION);
    assert_eq!(state.last_applied_event_seq, third_event.event_seq);

    let records = ledger
        .query_library_usage_projection(LibraryUsageProjectionQuery {
            asset_id: Some("asset_alpha".to_string()),
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("library usage projection loads");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].asset_id, "asset_alpha");
    assert_eq!(records[0].total_access_count, 2);
    assert_eq!(records[0].run_access_count, 1);
    assert_eq!(records[0].total_network_bytes, 3_072);
    assert_eq!(records[0].last_event_seq, second_event.event_seq);
    assert_eq!(
        records[0]
            .last_workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("workflow_run_alpha")
    );

    let by_workflow = ledger
        .query_library_usage_projection(LibraryUsageProjectionQuery {
            asset_id: None,
            workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
            workflow_version_id: None,
            after_event_seq: Some(first_event.event_seq),
            limit: 10,
        })
        .expect("library usage workflow filter loads");
    assert_eq!(by_workflow.len(), 1);
    assert_eq!(by_workflow[0].asset_id, "asset_alpha");

    let no_new_state = ledger
        .drain_library_usage_projection(10)
        .expect("library usage projection drains idempotently");
    assert_eq!(no_new_state.last_applied_event_seq, third_event.event_seq);
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
fn existing_v8_schema_adds_run_list_projection_table() {
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
            VALUES (8, 0, 'pantograph-diagnostics-ledger-v8');",
        )
        .expect("v8 schema marker is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_table_exists(&conn, "run_list_projection"));
    assert!(sqlite_index_exists(
        &conn,
        "idx_run_list_projection_updated"
    ));
    assert!(sqlite_index_exists(
        &conn,
        "idx_run_list_projection_retention_updated"
    ));
    assert!(sqlite_index_exists(
        &conn,
        "idx_run_list_projection_status_queue"
    ));
    assert!(sqlite_column_exists(
        &conn,
        "run_list_projection",
        "scheduler_queue_position"
    ));
}

#[test]
fn existing_v9_schema_adds_run_detail_projection_table() {
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
            VALUES (9, 0, 'pantograph-diagnostics-ledger-v9');",
        )
        .expect("v9 schema marker is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_table_exists(&conn, "run_detail_projection"));
    assert!(sqlite_index_exists(
        &conn,
        "idx_run_detail_projection_workflow_updated"
    ));
    assert!(sqlite_column_exists(
        &conn,
        "run_detail_projection",
        "scheduler_reason"
    ));
}

#[test]
fn existing_v10_schema_adds_io_artifact_projection_table() {
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
            VALUES (10, 0, 'pantograph-diagnostics-ledger-v10');",
        )
        .expect("v10 schema marker is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_table_exists(&conn, "io_artifact_projection"));
    assert!(sqlite_index_exists(
        &conn,
        "idx_io_artifact_projection_run_seq"
    ));
}

#[test]
fn existing_v11_schema_adds_library_usage_projection_table() {
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
            VALUES (11, 0, 'pantograph-diagnostics-ledger-v11');",
        )
        .expect("v11 schema marker is installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_table_exists(&conn, "library_usage_projection"));
    assert!(sqlite_table_exists(&conn, "library_usage_run_projection"));
    assert!(sqlite_index_exists(
        &conn,
        "idx_library_usage_projection_accessed"
    ));
}

#[test]
fn existing_v12_schema_adds_scheduler_projection_fact_columns() {
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
            VALUES (12, 0, 'pantograph-diagnostics-ledger-v12');
            CREATE TABLE run_list_projection (
                workflow_run_id TEXT PRIMARY KEY,
                status TEXT NOT NULL
            );
            CREATE TABLE run_detail_projection (
                workflow_run_id TEXT PRIMARY KEY
            );",
        )
        .expect("v12 schema marker and old projection tables are installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_column_exists(
        &conn,
        "run_list_projection",
        "estimate_confidence"
    ));
    assert!(sqlite_column_exists(
        &conn,
        "run_detail_projection",
        "estimated_duration_ms"
    ));
    assert!(sqlite_index_exists(
        &conn,
        "idx_run_list_projection_status_queue"
    ));
}

#[test]
fn existing_v14_schema_adds_io_artifact_retention_state_columns() {
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
            VALUES (14, 0, 'pantograph-diagnostics-ledger-v14');
            CREATE TABLE io_artifact_projection (
                event_seq INTEGER PRIMARY KEY,
                payload_ref TEXT
            );
            INSERT INTO io_artifact_projection (event_seq, payload_ref)
            VALUES (1, NULL), (2, 'artifact://retained');",
        )
        .expect("v14 schema marker and old artifact table are installed");
    }
    {
        let _ledger = SqliteDiagnosticsLedger::open(&path).expect("ledger migrates");
    }
    let conn = Connection::open(&path).expect("connection reopens");

    assert!(sqlite_column_exists(
        &conn,
        "io_artifact_projection",
        "retention_state"
    ));
    assert!(sqlite_column_exists(
        &conn,
        "io_artifact_projection",
        "retention_reason"
    ));
    assert!(sqlite_index_exists(
        &conn,
        "idx_io_artifact_projection_retention_state_seq"
    ));
    let metadata_state = conn
        .query_row(
            "SELECT retention_state FROM io_artifact_projection WHERE event_seq = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("metadata-only state loads");
    let retained_state = conn
        .query_row(
            "SELECT retention_state FROM io_artifact_projection WHERE event_seq = 2",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("retained state loads");
    assert_eq!(metadata_state, "metadata_only");
    assert_eq!(retained_state, "retained");
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

fn sqlite_column_exists(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .expect("table info statement prepares");
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("table info query succeeds");
    let exists = columns
        .map(|column| column.expect("column row loads"))
        .any(|column| column == column_name);
    exists
}

#[test]
fn retention_policy_uses_standard_local_default() {
    let ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let policy = ledger.retention_policy().expect("policy loads");

    assert_eq!(policy.retention_class, RetentionClass::Standard);
    assert_eq!(policy.retention_days, DEFAULT_STANDARD_RETENTION_DAYS);
}

#[test]
fn update_retention_policy_updates_standard_policy() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let policy = ledger
        .update_retention_policy(UpdateRetentionPolicyCommand {
            retention_class: RetentionClass::Standard,
            retention_days: 90,
            explanation: "Short local retention for test".to_string(),
        })
        .expect("policy updates");

    assert_eq!(policy.retention_class, RetentionClass::Standard);
    assert_eq!(policy.retention_days, 90);
    assert_eq!(policy.explanation, "Short local retention for test");
    assert_eq!(
        ledger
            .retention_policy()
            .expect("policy loads")
            .retention_days,
        90
    );
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

fn sample_run_started_event(workflow_run_id: &str) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("scheduler-local".to_string()),
        occurred_at_ms: 1_020,
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
        payload: DiagnosticEventPayload::RunStarted(RunStartedPayload {
            queue_wait_ms: Some(10),
            scheduler_decision_reason: Some("warm_session_reused".to_string()),
        }),
    }
}

fn sample_run_terminal_event(workflow_run_id: &str) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::WorkflowService,
        source_instance_id: Some("workflow-service".to_string()),
        occurred_at_ms: 1_100,
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
        payload: DiagnosticEventPayload::RunTerminal(RunTerminalPayload {
            status: RunTerminalStatus::Completed,
            duration_ms: Some(80),
            error: None,
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

fn sample_io_artifact_event(
    workflow_run_id: &str,
    node_id: &str,
    artifact_role: &str,
    artifact_id: &str,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: Some("node-executor".to_string()),
        occurred_at_ms: 1_200,
        workflow_run_id: Some(WorkflowRunId::try_from(workflow_run_id.to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: Some(node_id.to_string()),
        node_type: Some("artifact-node".to_string()),
        node_version: Some("1.0.0".to_string()),
        runtime_id: Some("runtime_alpha".to_string()),
        runtime_version: Some("0.1.0".to_string()),
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client_alpha".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session_alpha".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket_alpha".to_string()).unwrap()),
        scheduler_policy_id: Some("scheduler_default".to_string()),
        retention_policy_id: Some("retention_default".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SensitiveReference,
        retention_class: DiagnosticEventRetentionClass::PayloadReference,
        payload_ref: Some(format!("artifact://{artifact_id}")),
        payload: DiagnosticEventPayload::IoArtifactObserved(IoArtifactObservedPayload {
            artifact_id: artifact_id.to_string(),
            artifact_role: artifact_role.to_string(),
            media_type: Some("image/png".to_string()),
            size_bytes: Some(1_024),
            content_hash: Some("blake3:artifact-hash".to_string()),
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_reason: None,
        }),
    }
}

fn sample_retention_artifact_state_changed_event(
    workflow_run_id: &str,
    artifact_id: &str,
    retention_state: IoArtifactRetentionState,
    reason: &str,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Retention,
        source_instance_id: Some("retention-local".to_string()),
        occurred_at_ms: 1_400,
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
        scheduler_policy_id: None,
        retention_policy_id: Some("retention_default".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RetentionArtifactStateChanged(
            RetentionArtifactStateChangedPayload {
                artifact_id: artifact_id.to_string(),
                retention_state,
                reason: reason.to_string(),
            },
        ),
    }
}

fn sample_node_status_event(
    workflow_run_id: &str,
    node_id: &str,
    status: NodeExecutionProjectionStatus,
    started_at_ms: i64,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: Some("node-executor".to_string()),
        occurred_at_ms: started_at_ms,
        workflow_run_id: Some(WorkflowRunId::try_from(workflow_run_id.to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: Some(node_id.to_string()),
        node_type: Some("status-node".to_string()),
        node_version: Some("1.0.0".to_string()),
        runtime_id: Some("runtime_alpha".to_string()),
        runtime_version: Some("0.1.0".to_string()),
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
        payload: DiagnosticEventPayload::NodeExecutionStatus(NodeExecutionStatusPayload {
            status,
            started_at_ms: Some(started_at_ms),
            completed_at_ms: (status == NodeExecutionProjectionStatus::Completed)
                .then_some(started_at_ms + 100),
            duration_ms: (status == NodeExecutionProjectionStatus::Completed).then_some(100),
            error: None,
        }),
    }
}

fn sample_library_asset_access_event(
    asset_id: &str,
    workflow_run_id: Option<&str>,
    network_bytes: u64,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Library,
        source_instance_id: Some("pumas-library".to_string()),
        occurred_at_ms: 1_300,
        workflow_run_id: workflow_run_id.map(|id| WorkflowRunId::try_from(id.to_string()).unwrap()),
        workflow_id: workflow_run_id
            .map(|_| WorkflowId::try_from("workflow_alpha".to_string()).unwrap()),
        workflow_version_id: workflow_run_id
            .map(|_| WorkflowVersionId::try_from("wfver_alpha".to_string()).unwrap()),
        workflow_semantic_version: workflow_run_id.map(|_| "1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: Some(asset_id.to_string()),
        model_version: Some("main".to_string()),
        client_id: Some(ClientId::try_from("client_alpha".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session_alpha".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket_alpha".to_string()).unwrap()),
        scheduler_policy_id: None,
        retention_policy_id: Some("retention_default".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::LibraryAssetAccessed(LibraryAssetAccessedPayload {
            asset_id: asset_id.to_string(),
            operation: "download".to_string(),
            cache_status: Some("miss".to_string()),
            network_bytes: Some(network_bytes),
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

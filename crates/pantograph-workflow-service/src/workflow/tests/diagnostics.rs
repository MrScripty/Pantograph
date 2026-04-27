use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    ExecutionGuaranteeLevel, IoArtifactObservedPayload, IoArtifactRetentionState,
    LibraryAssetAccessedPayload, LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent,
    ModelOutputMeasurement, NodeExecutionProjectionStatus, NodeExecutionStatusPayload,
    OutputModality, ProjectionStatus, RetentionArtifactStateChangedPayload, RetentionClass,
    RunListFacetKind, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload, RunStartedPayload,
    RunTerminalPayload, RunTerminalStatus, SchedulerEstimateProducedPayload,
    SchedulerQueuePlacementPayload, UsageEventStatus, UsageLineage,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};

use super::*;

#[test]
fn workflow_diagnostics_usage_query_delegates_to_ledger_and_summarizes_events() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .record_usage_event(sample_event("usage-a", "model-a", Some("mit")))
        .expect("usage a");
    ledger
        .record_usage_event(sample_event("usage-b", "model-a", Some("mit")))
        .expect("usage b");
    ledger
        .record_usage_event(sample_event("usage-c", "model-b", Some("apache-2.0")))
        .expect("usage c");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            model_id: Some("model-a".to_string()),
            page_size: Some(10),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        })
        .expect("diagnostics query");

    assert_eq!(response.events.len(), 2);
    assert_eq!(response.summaries.len(), 1);
    assert_eq!(response.summaries[0].model_id, "model-a");
    assert_eq!(response.summaries[0].license_value.as_deref(), Some("mit"));
    assert_eq!(response.summaries[0].event_count, 2);
    assert_eq!(response.page_size, 10);
    assert_eq!(response.retention_policy.retention_days, 365);

    let by_version = service
        .workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            workflow_version_id: Some("wfver-a".to_string()),
            workflow_semantic_version: Some("1.0.0".to_string()),
            page_size: Some(10),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        })
        .expect("diagnostics version query");
    assert_eq!(by_version.events.len(), 3);

    let by_node_contract = service
        .workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            node_contract_version: Some("1.0.0".to_string()),
            node_contract_digest: Some("digest-a".to_string()),
            page_size: Some(10),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        })
        .expect("diagnostics node contract query");
    assert_eq!(by_node_contract.events.len(), 3);
}

#[test]
fn workflow_diagnostics_usage_query_validates_ids_and_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id =
        service.workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            client_id: Some("bad\nid".to_string()),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_page =
        service.workflow_diagnostics_usage_query(WorkflowDiagnosticsUsageQueryRequest {
            page_size: Some(501),
            ..WorkflowDiagnosticsUsageQueryRequest::default()
        });
    assert!(matches!(
        oversized_page,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_scheduler_timeline_query_drains_and_reads_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event())
        .expect("run snapshot event");
    ledger
        .append_diagnostic_event(sample_scheduler_estimate_event())
        .expect("scheduler estimate event");
    ledger
        .append_diagnostic_event(sample_scheduler_queue_event())
        .expect("scheduler queue event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        })
        .expect("scheduler timeline query");

    assert_eq!(response.events.len(), 3);
    assert_eq!(response.events[0].summary, "run snapshot accepted");
    assert_eq!(response.events[1].summary, "scheduler estimate produced");
    assert_eq!(response.events[2].summary, "queued at position 0");
    assert_eq!(response.projection_state.last_applied_event_seq, 3);

    let cursor_response = service
        .workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            after_event_seq: Some(response.events[0].event_seq),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        })
        .expect("scheduler timeline cursor query");
    assert_eq!(cursor_response.events.len(), 2);
}

#[test]
fn workflow_scheduler_timeline_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id =
        service.workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            workflow_run_id: Some("bad\nid".to_string()),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_limit =
        service.workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            limit: Some(501),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        });
    assert!(matches!(
        oversized_limit,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_projection_batch =
        service.workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            projection_batch_size: Some(501),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        });
    assert!(matches!(
        oversized_projection_batch,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_run_list_query_drains_and_reads_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event())
        .expect("run snapshot event");
    ledger
        .append_diagnostic_event(sample_scheduler_queue_event())
        .expect("scheduler queue event");
    ledger
        .append_diagnostic_event(sample_run_started_event())
        .expect("run started event");
    ledger
        .append_diagnostic_event(sample_run_terminal_event())
        .expect("run terminal event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            workflow_id: Some("workflow-a".to_string()),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list query");

    assert_eq!(response.runs.len(), 1);
    assert_eq!(response.runs[0].workflow_run_id.as_str(), "run-a");
    assert_eq!(response.runs[0].status, RunListProjectionStatus::Completed);
    assert_eq!(response.runs[0].duration_ms, Some(15));
    assert_eq!(response.runs[0].scheduler_queue_position, Some(0));
    assert_eq!(response.runs[0].scheduler_priority, Some(7));
    assert!(response.facets.iter().any(|facet| {
        facet.facet_kind == RunListFacetKind::WorkflowVersion
            && facet.facet_value == "1.0.0"
            && facet.run_count == 1
    }));
    assert_eq!(response.projection_state.last_applied_event_seq, 4);

    let retention_response = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            retention_policy_id: Some("ephemeral".to_string()),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list retention query");
    assert_eq!(retention_response.runs.len(), 1);
    assert_eq!(retention_response.runs[0].workflow_run_id.as_str(), "run-a");
}

#[test]
fn workflow_run_list_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id = service.workflow_run_list_query(WorkflowRunListQueryRequest {
        workflow_id: Some("bad\nid".to_string()),
        ..WorkflowRunListQueryRequest::default()
    });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_projection_batch = service.workflow_run_list_query(WorkflowRunListQueryRequest {
        projection_batch_size: Some(501),
        ..WorkflowRunListQueryRequest::default()
    });
    assert!(matches!(
        oversized_projection_batch,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_run_detail_query_drains_and_reads_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event())
        .expect("run snapshot event");
    ledger
        .append_diagnostic_event(sample_scheduler_estimate_event())
        .expect("scheduler estimate event");
    ledger
        .append_diagnostic_event(sample_scheduler_queue_event())
        .expect("scheduler queue event");
    ledger
        .append_diagnostic_event(sample_run_started_event())
        .expect("run started event");
    ledger
        .append_diagnostic_event(sample_run_terminal_event())
        .expect("run terminal event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: "run-a".to_string(),
            projection_batch_size: Some(10),
        })
        .expect("run detail query");

    let run = response.run.expect("run detail exists");
    assert_eq!(run.workflow_run_id.as_str(), "run-a");
    assert_eq!(run.workflow_id.as_str(), "workflow-a");
    assert_eq!(run.status, RunListProjectionStatus::Completed);
    assert_eq!(run.duration_ms, Some(15));
    assert_eq!(run.workflow_run_snapshot_id.as_deref(), Some("runsnap-a"));
    assert_eq!(
        run.workflow_presentation_revision_id.as_deref(),
        Some("wfpres-a")
    );
    assert!(run.latest_estimate_json.is_some());
    assert!(run.latest_queue_placement_json.is_some());
    assert!(run.started_payload_json.is_some());
    assert!(run.terminal_payload_json.is_some());
    assert_eq!(run.scheduler_queue_position, Some(0));
    assert_eq!(run.scheduler_priority, Some(7));
    assert_eq!(run.estimate_confidence.as_deref(), Some("low"));
    assert_eq!(run.scheduler_reason.as_deref(), Some("warm_session_reused"));
    assert_eq!(run.timeline_event_count, 5);
    assert_eq!(response.projection_state.last_applied_event_seq, 5);
}

#[test]
fn workflow_run_detail_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id = service.workflow_run_detail_query(WorkflowRunDetailQueryRequest {
        workflow_run_id: "bad\nid".to_string(),
        projection_batch_size: None,
    });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_projection_batch =
        service.workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: "run-a".to_string(),
            projection_batch_size: Some(501),
        });
    assert!(matches!(
        oversized_projection_batch,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_io_artifact_query_drains_and_reads_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-a",
            "node_output",
            "artifact-a",
        ))
        .expect("io artifact event");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-b",
            "workflow_output",
            "artifact-b",
        ))
        .expect("io artifact event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-b".to_string()),
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("io artifact query");

    assert_eq!(response.artifacts.len(), 1);
    assert_eq!(response.artifacts[0].artifact_id, "artifact-b");
    assert_eq!(response.artifacts[0].artifact_role, "workflow_output");
    assert_eq!(
        response.artifacts[0].retention_state,
        IoArtifactRetentionState::Retained
    );
    assert_eq!(
        response.artifacts[0].payload_ref.as_deref(),
        Some("artifact://artifact-b")
    );
    assert_eq!(response.retention_summary.len(), 1);
    assert_eq!(
        response.retention_summary[0].retention_state,
        IoArtifactRetentionState::Retained
    );
    assert_eq!(response.retention_summary[0].artifact_count, 1);
    assert_eq!(response.projection_state.last_applied_event_seq, 2);

    let global_response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: None,
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("global io artifact query");
    assert_eq!(global_response.artifacts.len(), 2);
    assert_eq!(global_response.retention_summary[0].artifact_count, 2);
}

#[test]
fn workflow_io_artifact_query_exposes_expired_retention_state() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-a",
            "workflow_output",
            "artifact-expired",
        ))
        .expect("io artifact event");
    ledger
        .append_diagnostic_event(sample_retention_artifact_state_changed_event(
            "artifact-expired",
            IoArtifactRetentionState::Expired,
        ))
        .expect("retention state change event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Expired),
            retention_policy_id: Some("ephemeral".to_string()),
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("expired io artifact query");

    assert_eq!(response.artifacts.len(), 1);
    assert_eq!(response.artifacts[0].artifact_id, "artifact-expired");
    assert_eq!(
        response.artifacts[0].retention_state,
        IoArtifactRetentionState::Expired
    );
    assert_eq!(response.artifacts[0].payload_ref, None);
    assert_eq!(
        response.artifacts[0].retention_reason.as_deref(),
        Some("retention policy expired payload")
    );
    assert_eq!(response.retention_summary.len(), 1);
    assert_eq!(
        response.retention_summary[0].retention_state,
        IoArtifactRetentionState::Expired
    );
    assert_eq!(response.retention_summary[0].artifact_count, 1);
}

#[test]
fn workflow_io_artifact_query_supports_no_active_run_browsing() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-a",
            "workflow_output",
            "artifact-a",
        ))
        .expect("first io artifact event");
    let mut second_artifact = sample_io_artifact_event("node-b", "workflow_output", "artifact-b");
    second_artifact.workflow_run_id = Some(WorkflowRunId::try_from("run-b".to_string()).unwrap());
    second_artifact.workflow_id = Some(WorkflowId::try_from("workflow-b".to_string()).unwrap());
    ledger
        .append_diagnostic_event(second_artifact)
        .expect("second io artifact event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: None,
            node_id: None,
            artifact_role: Some("workflow_output".to_string()),
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_policy_id: None,
            runtime_id: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("global retained io artifact query");

    assert_eq!(response.artifacts.len(), 2);
    assert!(response
        .artifacts
        .iter()
        .any(|artifact| artifact.workflow_run_id.as_str() == "run-a"));
    assert!(response
        .artifacts
        .iter()
        .any(|artifact| artifact.workflow_run_id.as_str() == "run-b"));
    assert_eq!(response.retention_summary.len(), 1);
    assert_eq!(response.retention_summary[0].artifact_count, 2);
}

#[test]
fn workflow_io_artifact_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id = service.workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
        workflow_run_id: Some("bad\nid".to_string()),
        node_id: None,
        artifact_role: None,
        media_type: None,
        retention_state: None,
        retention_policy_id: None,
        runtime_id: None,
        model_id: None,
        after_event_seq: None,
        limit: None,
        projection_batch_size: None,
    });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_limit = service.workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
        workflow_run_id: Some("run-a".to_string()),
        node_id: None,
        artifact_role: None,
        media_type: None,
        retention_state: None,
        retention_policy_id: None,
        runtime_id: None,
        model_id: None,
        after_event_seq: None,
        limit: Some(501),
        projection_batch_size: None,
    });
    assert!(matches!(
        oversized_limit,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_node_status_query_projects_latest_node_states() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_node_status_event(
            "node-a",
            NodeExecutionProjectionStatus::Running,
            40,
        ))
        .expect("running node status");
    ledger
        .append_diagnostic_event(sample_node_status_event(
            "node-a",
            NodeExecutionProjectionStatus::Completed,
            60,
        ))
        .expect("completed node status");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_node_status_query(WorkflowNodeStatusQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            status: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("node status query");

    assert_eq!(response.nodes.len(), 1);
    assert_eq!(response.nodes[0].node_id, "node-a");
    assert_eq!(
        response.nodes[0].status,
        NodeExecutionProjectionStatus::Completed
    );
    assert_eq!(response.nodes[0].duration_ms, Some(120));
    assert_eq!(response.projection_state.projection_name, "node_status");
}

#[test]
fn workflow_projection_rebuild_delegates_to_ledger() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event())
        .expect("run snapshot event");
    ledger
        .append_diagnostic_event(sample_run_terminal_event())
        .expect("run terminal event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_projection_rebuild(WorkflowProjectionRebuildRequest {
            projection_name: "run_list".to_string(),
            batch_size: Some(1),
        })
        .expect("projection rebuild");

    assert_eq!(response.projection_state.projection_name, "run_list");
    assert_eq!(response.projection_state.last_applied_event_seq, 2);
}

#[test]
fn workflow_projection_rebuild_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let oversized = service.workflow_projection_rebuild(WorkflowProjectionRebuildRequest {
        projection_name: "run_list".to_string(),
        batch_size: Some(501),
    });
    assert!(matches!(
        oversized,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let unknown = service.workflow_projection_rebuild(WorkflowProjectionRebuildRequest {
        projection_name: "unknown".to_string(),
        batch_size: None,
    });
    assert!(matches!(
        unknown,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_library_usage_query_drains_and_reads_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "model-a",
            Some("run-a"),
            128,
        ))
        .expect("library access event");
    ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "model-a",
            Some("run-a"),
            256,
        ))
        .expect("library access event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("model-a".to_string()),
            workflow_id: Some("workflow-a".to_string()),
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("library usage query");

    assert_eq!(response.assets.len(), 1);
    assert_eq!(response.assets[0].asset_id, "model-a");
    assert_eq!(response.assets[0].total_access_count, 2);
    assert_eq!(response.assets[0].run_access_count, 1);
    assert_eq!(response.assets[0].total_network_bytes, 384);
    assert_eq!(response.projection_state.last_applied_event_seq, 2);
}

#[test]
fn workflow_library_usage_query_preserves_catching_up_projection_state() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "model-a",
            Some("run-a"),
            128,
        ))
        .expect("library access event");
    ledger
        .append_diagnostic_event(sample_library_asset_access_event(
            "model-a",
            Some("run-a"),
            256,
        ))
        .expect("library access event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let catching_up = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("model-a".to_string()),
            workflow_id: Some("workflow-a".to_string()),
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(1),
        })
        .expect("library usage catching-up query");

    assert_eq!(catching_up.assets.len(), 1);
    assert_eq!(catching_up.assets[0].total_access_count, 1);
    assert_eq!(catching_up.projection_state.last_applied_event_seq, 1);
    assert_eq!(
        catching_up.projection_state.status,
        ProjectionStatus::Rebuilding
    );

    let current = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("model-a".to_string()),
            workflow_id: Some("workflow-a".to_string()),
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("library usage current query");

    assert_eq!(current.assets[0].total_access_count, 2);
    assert_eq!(current.projection_state.last_applied_event_seq, 2);
    assert_eq!(current.projection_state.status, ProjectionStatus::Current);
}

#[test]
fn workflow_library_usage_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let invalid_id = service.workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
        asset_id: None,
        workflow_id: Some("bad\nid".to_string()),
        workflow_version_id: None,
        after_event_seq: None,
        limit: None,
        projection_batch_size: None,
    });
    assert!(matches!(
        invalid_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let invalid_asset_id = service.workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
        asset_id: Some("https://example.test/model".to_string()),
        workflow_id: None,
        workflow_version_id: None,
        after_event_seq: None,
        limit: None,
        projection_batch_size: None,
    });
    assert!(matches!(
        invalid_asset_id,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_limit = service.workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
        asset_id: None,
        workflow_id: None,
        workflow_version_id: None,
        after_event_seq: None,
        limit: Some(501),
        projection_batch_size: None,
    });
    assert!(matches!(
        oversized_limit,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_retention_policy_query_reads_current_policy() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let response = service
        .workflow_retention_policy_query(WorkflowRetentionPolicyQueryRequest {})
        .expect("retention policy query");

    assert_eq!(response.retention_policy.policy_id, "standard-local-v1");
    assert_eq!(response.retention_policy.policy_version, 1);
    assert_eq!(response.retention_policy.retention_days, 365);
}

#[test]
fn workflow_retention_policy_update_changes_policy_and_records_event() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let response = service
        .workflow_retention_policy_update(WorkflowRetentionPolicyUpdateRequest {
            retention_days: 120,
            explanation: "Keep local diagnostics for one development cycle".to_string(),
            reason: "Developer changed global I/O retention settings".to_string(),
        })
        .expect("retention policy update");

    assert_eq!(response.retention_policy.policy_id, "standard-local-v1");
    assert_eq!(response.retention_policy.policy_version, 2);
    assert_eq!(response.retention_policy.retention_days, 120);
    assert_eq!(
        service
            .workflow_retention_policy_query(WorkflowRetentionPolicyQueryRequest {})
            .expect("query updated policy")
            .retention_policy
            .retention_days,
        120
    );

    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_kind,
        pantograph_diagnostics_ledger::DiagnosticEventKind::RetentionPolicyChanged
    );
    assert_eq!(
        events[0].retention_policy_id.as_deref(),
        Some("standard-local-v1")
    );
    assert!(events[0].payload_json.contains("\"policy_version\":2"));
    assert!(events[0].payload_json.contains("\"retention_days\":120"));
}

fn sample_run_snapshot_event() -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::WorkflowService,
        source_instance_id: Some("workflow-service".to_string()),
        occurred_at_ms: 10,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RunSnapshotAccepted(RunSnapshotAcceptedPayload {
            workflow_run_snapshot_id: "runsnap-a".to_string(),
            workflow_presentation_revision_id: "wfpres-a".to_string(),
            node_versions: vec![RunSnapshotNodeVersionPayload {
                node_id: "node-a".to_string(),
                node_type: "text-output".to_string(),
                contract_version: "1.0.0".to_string(),
                behavior_digest: "digest-a".to_string(),
            }],
        }),
    }
}

fn sample_run_started_event() -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("workflow-session-scheduler".to_string()),
        occurred_at_ms: 13,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RunStarted(RunStartedPayload {
            queue_wait_ms: Some(1),
            scheduler_decision_reason: Some("warm_session_reused".to_string()),
        }),
    }
}

fn sample_run_terminal_event() -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::WorkflowService,
        source_instance_id: Some("workflow-service".to_string()),
        occurred_at_ms: 28,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RunTerminal(RunTerminalPayload {
            status: RunTerminalStatus::Completed,
            duration_ms: Some(15),
            error: None,
        }),
    }
}

fn sample_scheduler_estimate_event() -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("workflow-session-scheduler".to_string()),
        occurred_at_ms: 11,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::SchedulerEstimateProduced(
            SchedulerEstimateProducedPayload {
                estimate_version: "session-scheduler-v1".to_string(),
                confidence: "low".to_string(),
                estimated_queue_wait_ms: None,
                estimated_duration_ms: None,
                reasons: vec!["next admission candidate".to_string()],
            },
        ),
    }
}

fn sample_scheduler_queue_event() -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("workflow-session-scheduler".to_string()),
        occurred_at_ms: 12,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::SchedulerQueuePlacement(SchedulerQueuePlacementPayload {
            queue_position: 0,
            priority: 7,
            scheduler_policy_id: "priority_then_fifo".to_string(),
        }),
    }
}

fn sample_io_artifact_event(
    node_id: &str,
    artifact_role: &str,
    artifact_id: &str,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: Some("node-executor".to_string()),
        occurred_at_ms: 30,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: Some(node_id.to_string()),
        node_type: Some("artifact-node".to_string()),
        node_version: Some("1.0.0".to_string()),
        runtime_id: Some("runtime-a".to_string()),
        runtime_version: Some("0.1.0".to_string()),
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SensitiveReference,
        retention_class: DiagnosticEventRetentionClass::PayloadReference,
        payload_ref: Some(format!("artifact://{artifact_id}")),
        payload: DiagnosticEventPayload::IoArtifactObserved(IoArtifactObservedPayload {
            artifact_id: artifact_id.to_string(),
            artifact_role: artifact_role.to_string(),
            media_type: Some("text/plain".to_string()),
            size_bytes: Some(42),
            content_hash: Some("blake3:test".to_string()),
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_reason: None,
        }),
    }
}

fn sample_retention_artifact_state_changed_event(
    artifact_id: &str,
    retention_state: IoArtifactRetentionState,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Retention,
        source_instance_id: Some("retention-worker".to_string()),
        occurred_at_ms: 40,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::RetentionArtifactStateChanged(
            RetentionArtifactStateChangedPayload {
                artifact_id: artifact_id.to_string(),
                retention_state,
                reason: "retention policy expired payload".to_string(),
            },
        ),
    }
}

fn sample_node_status_event(
    node_id: &str,
    status: NodeExecutionProjectionStatus,
    started_at_ms: i64,
) -> DiagnosticEventAppendRequest {
    DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: Some("node-executor".to_string()),
        occurred_at_ms: started_at_ms,
        workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
        workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        node_id: Some(node_id.to_string()),
        node_type: Some("status-node".to_string()),
        node_version: Some("1.0.0".to_string()),
        runtime_id: Some("runtime-a".to_string()),
        runtime_version: Some("0.1.0".to_string()),
        model_id: None,
        model_version: None,
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: Some("ephemeral".to_string()),
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::NodeExecutionStatus(NodeExecutionStatusPayload {
            status,
            started_at_ms: Some(started_at_ms),
            completed_at_ms: (status == NodeExecutionProjectionStatus::Completed)
                .then_some(started_at_ms + 120),
            duration_ms: (status == NodeExecutionProjectionStatus::Completed).then_some(120),
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
        occurred_at_ms: 31,
        workflow_run_id: workflow_run_id.map(|id| WorkflowRunId::try_from(id.to_string()).unwrap()),
        workflow_id: workflow_run_id
            .map(|_| WorkflowId::try_from("workflow-a".to_string()).unwrap()),
        workflow_version_id: workflow_run_id
            .map(|_| WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: workflow_run_id.map(|_| "1.0.0".to_string()),
        node_id: None,
        node_type: None,
        node_version: None,
        runtime_id: None,
        runtime_version: None,
        model_id: Some(asset_id.to_string()),
        model_version: Some("main".to_string()),
        client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
        client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
        bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
        scheduler_policy_id: None,
        retention_policy_id: Some("ephemeral".to_string()),
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
    usage_id: &str,
    model_id: &str,
    license_value: Option<&str>,
) -> ModelLicenseUsageEvent {
    ModelLicenseUsageEvent {
        usage_event_id: UsageEventId::try_from(usage_id.to_string()).unwrap(),
        client_id: ClientId::try_from("client-a".to_string()).unwrap(),
        client_session_id: ClientSessionId::try_from("session-a".to_string()).unwrap(),
        bucket_id: BucketId::try_from("bucket-a".to_string()).unwrap(),
        workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).unwrap(),
        workflow_id: WorkflowId::try_from("workflow-a".to_string()).unwrap(),
        workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
        workflow_semantic_version: Some("1.0.0".to_string()),
        model: ModelIdentity {
            model_id: model_id.to_string(),
            model_revision: Some("rev-1".to_string()),
            model_hash: None,
            model_modality: Some("text".to_string()),
            runtime_backend: Some("pytorch".to_string()),
        },
        lineage: UsageLineage {
            node_id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            port_ids: vec!["text".to_string()],
            composed_parent_chain: Vec::new(),
            effective_contract_version: Some("1.0.0".to_string()),
            effective_contract_digest: Some("digest-a".to_string()),
            metadata_json: None,
        },
        license_snapshot: LicenseSnapshot {
            license_value: license_value.map(str::to_string),
            source_metadata_json: Some(r#"{"source":"pumas"}"#.to_string()),
            model_metadata_snapshot_json: Some(r#"{"model":"snapshot"}"#.to_string()),
            unavailable_reason: None,
        },
        output_measurement: ModelOutputMeasurement {
            modality: OutputModality::Text,
            item_count: Some(1),
            character_count: Some(10),
            byte_size: Some(10),
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
            unavailable_reasons: Vec::new(),
        },
        guarantee_level: ExecutionGuaranteeLevel::ManagedFull,
        status: UsageEventStatus::Completed,
        retention_class: RetentionClass::Standard,
        started_at_ms: 10,
        completed_at_ms: Some(20),
        correlation_id: None,
    }
}

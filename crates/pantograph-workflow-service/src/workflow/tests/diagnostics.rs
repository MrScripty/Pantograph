use std::collections::BTreeMap;

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    ExecutionGuaranteeLevel, IoArtifactObservedPayload, IoArtifactRetentionState, IoArtifactRole,
    LibraryAssetAccessedPayload, LibraryAssetCacheStatus, LibraryAssetOperation, LicenseSnapshot,
    ModelIdentity, ModelLicenseUsageEvent, ModelOutputMeasurement, NodeExecutionCacheStatus,
    NodeExecutionProjectionStatus, NodeExecutionStatusPayload, OutputModality, ProjectionStatus,
    RetentionArtifactStateChangedPayload, RetentionClass, RetentionPolicyActorScope,
    RunListFacetKind, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload, RunStartedPayload,
    RunTerminalPayload, RunTerminalStatus, SchedulerEstimateBlockingCondition,
    SchedulerEstimateProducedPayload, SchedulerModelCacheState, SchedulerQueuePlacementPayload,
    UsageEventStatus, UsageLineage,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};

use super::super::diagnostic_errors::{
    registered_workflow_diagnostic_error_phases, WorkflowDiagnosticCausalityPolicy,
    WorkflowDiagnosticErrorPhase, WorkflowDiagnosticErrorRecordRequest,
    WorkflowDiagnosticProjectionEffect, WorkflowDiagnosticProjectionScope,
    WorkflowDiagnosticRunContext, WorkflowDiagnosticRuntimeModelScope,
    WorkflowDiagnosticTransportScope,
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
    assert_eq!(
        oversized_limit
            .expect_err("oversized limit should fail")
            .code(),
        WorkflowErrorCode::InvalidRequest
    );

    let oversized_projection_batch =
        service.workflow_scheduler_timeline_query(WorkflowSchedulerTimelineQueryRequest {
            projection_batch_size: Some(501),
            ..WorkflowSchedulerTimelineQueryRequest::default()
        });
    assert!(matches!(
        oversized_projection_batch,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let invalid_accepted_range = service.workflow_run_list_query(WorkflowRunListQueryRequest {
        accepted_at_from_ms: Some(20),
        accepted_at_to_ms: Some(10),
        ..WorkflowRunListQueryRequest::default()
    });
    assert_eq!(
        invalid_accepted_range
            .expect_err("invalid accepted range should fail")
            .code(),
        WorkflowErrorCode::InvalidRequest
    );
}

#[test]
fn workflow_diagnostic_error_recorder_appends_runtime_model_error() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");
    let error =
        WorkflowServiceError::RuntimeNotReady("llama.cpp exited before ready\n".to_string());
    let outcome = service
        .record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::runtime_model_load_failed(
                sample_runtime_model_error_scope(),
                &error,
            )
            .with_source_instance_id("workflow-session-scheduler")
            .with_cause("process exited with code 127\u{0000}"),
        )
        .expect("diagnostic error appends");

    assert!(outcome.event_id.is_some());
    assert!(outcome.diagnostics_unavailable.is_none());
    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger guard");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_kind,
        pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
    );
    assert!(events[0].payload_json.contains("runtime_model_load"));
    assert!(events[0]
        .payload_json
        .contains("llama.cpp exited before ready"));
    assert!(!events[0].payload_json.contains("\\n"));
}

#[test]
fn workflow_artifact_api_records_write_failure_with_run_context() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let store = ArtifactStore::open(temp.path(), artifact_policy_with_one_byte_limit())
        .expect("artifact store");
    let service = WorkflowService::with_ephemeral_diagnostics_ledger()
        .expect("service")
        .with_artifact_store(store);

    let error = service
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact-a".to_string()),
            payload_kind: ArtifactPayloadKind::Text,
            media_type: "text/plain".to_string(),
            format: None,
            attribution: sample_artifact_attribution(),
            artifact_role: Some("workflow_output".to_string()),
            parent_artifact_id: None,
            revision_index: None,
            body: b"too large".to_vec(),
        })
        .expect_err("oversized artifact fails");

    let diagnostics = error.diagnostics().expect("diagnostics link");
    assert_eq!(diagnostics.workflow_run_id.as_deref(), Some("run-artifact"));
    assert!(diagnostics.diagnostic_event_id.is_some());

    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger guard");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_kind,
        pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
    );
    assert!(events[0].payload_json.contains("artifact_failed"));
    assert_eq!(
        events[0].payload_ref.as_deref(),
        Some("artifact://artifact-a")
    );
}

#[test]
fn workflow_diagnostic_error_recorder_reports_unavailable_when_ledger_missing() {
    let service = WorkflowService::new();
    let error = WorkflowServiceError::Internal("route failed".to_string());
    let outcome = service
        .record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::transport_failed(
                WorkflowDiagnosticTransportScope {
                    workflow_run_id: None,
                    workflow_id: None,
                },
                &error,
            ),
        )
        .expect("missing diagnostics ledger is explicit");

    assert_eq!(outcome.event_id, None);
    assert_eq!(
        outcome.diagnostics_unavailable.as_deref(),
        Some("diagnostics ledger is not configured")
    );
}

#[test]
fn workflow_diagnostic_error_recorder_validates_registered_scope() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");
    let error = WorkflowServiceError::RuntimeNotReady("runtime missing".to_string());
    let result = service.record_workflow_diagnostic_error_if_configured(
        WorkflowDiagnosticErrorRecordRequest::transport_failed(
            WorkflowDiagnosticTransportScope {
                workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
                workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
            },
            &error,
        )
        .with_source_component(DiagnosticEventSourceComponent::Scheduler),
    );

    assert!(matches!(result, Err(WorkflowServiceError::Internal(_))));
}

#[test]
fn workflow_diagnostic_error_registry_declares_phase_contracts() {
    let registry = registered_workflow_diagnostic_error_phases();

    assert_eq!(registry.len(), 13);
    for entry in registry {
        assert!(!entry.phase_id.starts_with("inference"));
        assert!(!entry.code.starts_with("inference"));
    }
    for phase_id in [
        "runtime_preflight",
        "runtime_model_load",
        "runtime_launch",
        "model_dependency",
        "node_execution",
        "output_validation",
    ] {
        assert!(
            registry.iter().any(|entry| entry.phase_id == phase_id),
            "missing canonical diagnostic error phase {phase_id}"
        );
    }
    let runtime_model_load = registry
        .iter()
        .find(|entry| entry.phase == WorkflowDiagnosticErrorPhase::RuntimeModelLoad)
        .expect("runtime model load phase is registered");
    assert_eq!(runtime_model_load.phase_id, "runtime_model_load");
    assert_eq!(
        runtime_model_load.scope_kind,
        pantograph_diagnostics_ledger::DiagnosticErrorScopeKind::RuntimeModel
    );
    assert_eq!(
        runtime_model_load.causality_policy,
        WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly
    );
    assert_eq!(
        runtime_model_load.projection_effect,
        WorkflowDiagnosticProjectionEffect::FatalRunFailure
    );

    assert!(registry.iter().any(|entry| {
        entry.phase == WorkflowDiagnosticErrorPhase::Projection
            && entry.projection_effect == WorkflowDiagnosticProjectionEffect::DiagnosticsOnly
    }));
    assert!(registry.iter().any(|entry| {
        entry.phase == WorkflowDiagnosticErrorPhase::ManagedBinary
            && entry.phase_id == "managed_binary"
            && entry.projection_effect == WorkflowDiagnosticProjectionEffect::FatalRunFailure
    }));
}

#[test]
fn workflow_diagnostic_error_recorder_reports_unavailable_on_append_failure() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");
    let error = WorkflowServiceError::RuntimeNotReady("runtime missing".to_string());
    let outcome = service
        .record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::runtime_model_load_failed(
                sample_runtime_model_error_scope(),
                &error,
            )
            .with_related_event_id("bad\nid"),
        )
        .expect("append failure is reported as diagnostics unavailable");

    assert_eq!(outcome.event_id, None);
    assert!(outcome
        .diagnostics_unavailable
        .as_deref()
        .unwrap_or_default()
        .contains("diagnostics ledger append failed"));
}

#[test]
fn workflow_diagnostic_error_recorder_appends_global_projection_error() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");
    let error = WorkflowServiceError::Internal("projection replay failed".to_string());
    let outcome = service
        .record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::projection_failed(
                WorkflowDiagnosticProjectionScope {
                    workflow_run_id: None,
                    workflow_id: None,
                    projection_name: "run_list".to_string(),
                    operation: "drain".to_string(),
                },
                &error,
            ),
        )
        .expect("projection diagnostic appends");

    assert!(outcome.event_id.is_some());
    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger guard");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert_eq!(events.len(), 1);
    assert!(events[0].payload_json.contains("projection_failed"));
    assert!(events[0].payload_json.contains("run_list.drain"));
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
    assert_eq!(
        response.runs[0].client_id.as_ref().map(|id| id.as_str()),
        Some("client-a")
    );
    assert_eq!(
        response.runs[0]
            .client_session_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("session-a")
    );
    assert_eq!(
        response.runs[0].bucket_id.as_ref().map(|id| id.as_str()),
        Some("bucket-a")
    );
    assert_eq!(
        response.runs[0].workflow_execution_session_id.as_deref(),
        Some("exec-session-a")
    );
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

    let scoped_response = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            client_id: Some("client-a".to_string()),
            client_session_id: Some("session-a".to_string()),
            bucket_id: Some("bucket-a".to_string()),
            accepted_at_from_ms: Some(1),
            accepted_at_to_ms: Some(20),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list scope query");
    assert_eq!(scoped_response.runs.len(), 1);
    assert_eq!(scoped_response.runs[0].workflow_run_id.as_str(), "run-a");
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
fn workflow_marks_abandoned_nonterminal_runs_failed() {
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
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let repaired = service
        .workflow_mark_abandoned_nonterminal_runs("startup repair")
        .expect("abandoned run repair succeeds");

    assert_eq!(repaired, 1);
    let response = service
        .workflow_run_list_query(WorkflowRunListQueryRequest {
            workflow_id: Some("workflow-a".to_string()),
            limit: Some(10),
            projection_batch_size: Some(10),
            ..WorkflowRunListQueryRequest::default()
        })
        .expect("run list query");
    assert_eq!(response.runs.len(), 1);
    assert_eq!(response.runs[0].status, RunListProjectionStatus::Failed);
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
    let mut node_event = sample_node_status_event(
        "node-inference",
        NodeExecutionProjectionStatus::Completed,
        1_760_000_000_010,
    );
    node_event.model_id = Some("pumas://models/tiny-gguf".to_string());
    if let DiagnosticEventPayload::NodeExecutionStatus(payload) = &mut node_event.payload {
        payload.task_id = Some("text_generation".to_string());
        payload.selected_backend_key = Some("llama_cpp".to_string());
        payload.execution_cache_status = Some(NodeExecutionCacheStatus::FreshExecution);
    }
    ledger
        .append_diagnostic_event(node_event)
        .expect("node status event");
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
        run.workflow_execution_session_id.as_deref(),
        Some("exec-session-a")
    );
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
    assert_eq!(run.selected_backend_key.as_deref(), Some("llama_cpp"));
    assert_eq!(
        run.selected_model_id.as_deref(),
        Some("pumas://models/tiny-gguf")
    );
    assert_eq!(run.selected_task_id.as_deref(), Some("text_generation"));
    assert_eq!(run.timeline_event_count, 5);
    assert_eq!(response.projection_state.last_applied_event_seq, 6);
    assert_eq!(response.node_statuses.len(), 1);
    assert_eq!(response.node_statuses[0].node_id, "node-inference");
    assert_eq!(
        response.node_statuses[0].task_id.as_deref(),
        Some("text_generation")
    );
    assert_eq!(
        response.node_statuses[0].selected_backend_key.as_deref(),
        Some("llama_cpp")
    );
    assert_eq!(
        response.node_statuses[0].model_id.as_deref(),
        Some("pumas://models/tiny-gguf")
    );
    assert_eq!(
        response.node_statuses[0].execution_cache_status,
        Some(NodeExecutionCacheStatus::FreshExecution)
    );
    assert_eq!(response.node_projection_state.last_applied_event_seq, 6);
}

#[test]
fn workflow_run_inspection_query_returns_factual_run_snapshot_parts() {
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
    ledger
        .append_diagnostic_event(sample_node_status_event(
            "node-inference",
            NodeExecutionProjectionStatus::Completed,
            1_760_000_000_010,
        ))
        .expect("node status event");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-inference",
            "node_output",
            "artifact-a",
        ))
        .expect("io artifact event");
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_diagnostics_ledger(ledger);

    let response = service
        .workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
            workflow_run_id: "run-a".to_string(),
            artifact_limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("run inspection query");

    assert!(response.run_graph.is_none());
    assert_eq!(
        response
            .run
            .as_ref()
            .map(|run| run.workflow_run_id.as_str()),
        Some("run-a")
    );
    assert_eq!(response.node_statuses.len(), 1);
    assert_eq!(response.node_statuses[0].node_id, "node-inference");
    assert_eq!(response.io_artifacts.len(), 1);
    assert_eq!(response.io_artifacts[0].artifact_id, "artifact-a");
    assert_eq!(response.retention_summary.len(), 1);
    assert_eq!(response.run_projection_state.projection_name, "run_detail");
    assert_eq!(
        response.node_projection_state.projection_name,
        "node_status"
    );
    assert_eq!(response.io_projection_state.projection_name, "io_artifact");
}

#[test]
fn workflow_run_inspection_query_validates_bounds() {
    let service = WorkflowService::with_ephemeral_attribution_store()
        .expect("service")
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let oversized_projection_batch =
        service.workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
            workflow_run_id: "run-a".to_string(),
            artifact_limit: Some(10),
            projection_batch_size: Some(501),
        });
    assert!(matches!(
        oversized_projection_batch,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));

    let oversized_artifact_limit =
        service.workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
            workflow_run_id: "run-a".to_string(),
            artifact_limit: Some(501),
            projection_batch_size: Some(10),
        });
    assert!(matches!(
        oversized_artifact_limit,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
}

#[test]
fn workflow_scheduler_estimate_query_returns_estimate_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_run_snapshot_event())
        .expect("run snapshot event");
    ledger
        .append_diagnostic_event(sample_scheduler_estimate_event())
        .expect("scheduler estimate event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_scheduler_estimate_query(WorkflowSchedulerEstimateQueryRequest {
            workflow_run_id: "run-a".to_string(),
            projection_batch_size: Some(10),
        })
        .expect("scheduler estimate query");

    let estimate = response.estimate.expect("estimate exists");
    assert_eq!(estimate.workflow_run_id, "run-a");
    assert_eq!(estimate.workflow_id, "workflow-a");
    assert_eq!(estimate.workflow_version_id.as_deref(), Some("wfver-a"));
    assert_eq!(
        estimate.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert!(estimate.latest_estimate_json.is_some());
    assert_eq!(estimate.estimate_confidence.as_deref(), Some("low"));
    assert_eq!(estimate.estimated_queue_wait_ms, None);
    assert_eq!(estimate.estimated_duration_ms, None);
    assert_eq!(
        estimate.model_cache_state,
        Some(SchedulerModelCacheState::Unknown)
    );
    assert_eq!(response.projection_state.last_applied_event_seq, 2);
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
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-b",
            "node_input",
            "artifact-c",
        ))
        .expect("io artifact event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-b".to_string()),
            producer_node_id: Some("node-b".to_string()),
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
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
        response.artifacts[0].producer_node_id.as_deref(),
        Some("node-b")
    );
    assert_eq!(
        response.artifacts[0].producer_port_id.as_deref(),
        Some("out")
    );
    assert_eq!(response.artifacts[0].consumer_node_id, None);
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
    assert_eq!(response.projection_state.last_applied_event_seq, 3);

    let consumer_response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: Some("node-b".to_string()),
            artifact_role: Some("node_input".to_string()),
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("consumer io artifact query");

    assert_eq!(consumer_response.artifacts.len(), 1);
    assert_eq!(consumer_response.artifacts[0].artifact_id, "artifact-c");
    assert_eq!(
        consumer_response.artifacts[0].consumer_node_id.as_deref(),
        Some("node-b")
    );
    assert_eq!(consumer_response.retention_summary.len(), 1);
    assert_eq!(consumer_response.retention_summary[0].artifact_count, 1);

    let global_response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: None,
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("global io artifact query");
    assert_eq!(global_response.artifacts.len(), 3);
    assert_eq!(global_response.retention_summary[0].artifact_count, 3);
}

#[test]
fn workflow_io_artifact_query_groups_node_input_and_output_records_by_run_node() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    for (node_id, artifact_role, artifact_id) in [
        ("node-a", "node_input", "artifact-a-in"),
        ("node-a", "node_output", "artifact-a-out"),
        ("node-b", "node_output", "artifact-b-out"),
    ] {
        ledger
            .append_diagnostic_event(sample_io_artifact_event(
                node_id,
                artifact_role,
                artifact_id,
            ))
            .expect("io artifact event");
    }
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_policy_id: Some("ephemeral".to_string()),
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("io artifact query");

    let mut roles_by_node: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for artifact in &response.artifacts {
        roles_by_node
            .entry(artifact.node_id.clone().expect("node id"))
            .or_default()
            .push(artifact.artifact_role.clone());
    }

    assert_eq!(response.artifacts.len(), 3);
    assert_eq!(
        roles_by_node.get("node-a").cloned(),
        Some(vec!["node_input".to_string(), "node_output".to_string()])
    );
    assert_eq!(
        roles_by_node.get("node-b").cloned(),
        Some(vec!["node_output".to_string()])
    );
    assert_eq!(
        response.retention_summary[0].retention_state,
        IoArtifactRetentionState::Retained
    );
    assert_eq!(response.retention_summary[0].artifact_count, 3);
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
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Expired),
            retention_policy_id: Some("ephemeral".to_string()),
            runtime_id: None,
            selected_backend_key: None,
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
fn workflow_io_artifact_query_exposes_deleted_retention_state() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-a",
            "node_output",
            "artifact-deleted",
        ))
        .expect("io artifact event");
    ledger
        .append_diagnostic_event(sample_retention_artifact_state_changed_event(
            "artifact-deleted",
            IoArtifactRetentionState::Deleted,
        ))
        .expect("retention state change event");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: Some("node_output".to_string()),
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Deleted),
            retention_policy_id: Some("ephemeral".to_string()),
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("deleted io artifact query");

    assert_eq!(response.artifacts.len(), 1);
    assert_eq!(response.artifacts[0].artifact_id, "artifact-deleted");
    assert_eq!(
        response.artifacts[0].retention_state,
        IoArtifactRetentionState::Deleted
    );
    assert_eq!(response.artifacts[0].payload_ref, None);
    assert_eq!(
        response.artifacts[0].retention_reason.as_deref(),
        Some("retention policy expired payload")
    );
    assert_eq!(response.retention_summary.len(), 1);
    assert_eq!(
        response.retention_summary[0].retention_state,
        IoArtifactRetentionState::Deleted
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
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: Some("workflow_output".to_string()),
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
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
        producer_node_id: None,
        consumer_node_id: None,
        artifact_role: None,
        media_type: None,
        retention_state: None,
        retention_policy_id: None,
        runtime_id: None,
        selected_backend_key: None,
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
        producer_node_id: None,
        consumer_node_id: None,
        artifact_role: None,
        media_type: None,
        retention_state: None,
        retention_policy_id: None,
        runtime_id: None,
        selected_backend_key: None,
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
    assert_eq!(
        unknown.expect_err("unknown projection should fail").code(),
        WorkflowErrorCode::InvalidRequest
    );
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
            workflow_run_id: None,
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

    let active_run_assets = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: None,
            workflow_run_id: Some("run-a".to_string()),
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("library usage active-run query");
    assert_eq!(active_run_assets.assets.len(), 1);
    assert_eq!(active_run_assets.assets[0].asset_id, "model-a");
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
            workflow_run_id: None,
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
            workflow_run_id: None,
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
        workflow_run_id: None,
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
        workflow_run_id: None,
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
        workflow_run_id: None,
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
fn workflow_library_asset_access_record_appends_typed_event() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let response = service
        .workflow_library_asset_access_record(WorkflowLibraryAssetAccessRecordRequest {
            asset_id: "pumas://models".to_string(),
            operation: LibraryAssetOperation::Search,
            cache_status: Some(LibraryAssetCacheStatus::Unknown),
            network_bytes: None,
            source_instance_id: Some("puma-lib-port-options".to_string()),
        })
        .expect("library asset access event records");

    assert_eq!(response.event_seq, Some(1));
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
        pantograph_diagnostics_ledger::DiagnosticEventKind::LibraryAssetAccessed
    );
    assert_eq!(
        events[0].source_component,
        DiagnosticEventSourceComponent::Library
    );
    assert_eq!(
        events[0].source_instance_id.as_deref(),
        Some("puma-lib-port-options")
    );
    assert!(events[0]
        .payload_json
        .contains("\"asset_id\":\"pumas://models\""));
    assert!(events[0].payload_json.contains("\"operation\":\"search\""));
}

#[test]
fn workflow_library_asset_access_record_rejects_invalid_assets_without_event() {
    let service = WorkflowService::with_ephemeral_diagnostics_ledger().expect("service");

    let rejected =
        service.workflow_library_asset_access_record(WorkflowLibraryAssetAccessRecordRequest {
            asset_id: "../unsafe-model".to_string(),
            operation: LibraryAssetOperation::Download,
            cache_status: Some(LibraryAssetCacheStatus::Miss),
            network_bytes: Some(128),
            source_instance_id: Some("pumas-hf-download".to_string()),
        });

    assert!(matches!(
        rejected,
        Err(WorkflowServiceError::InvalidRequest(_))
    ));
    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert!(
        events.is_empty(),
        "rejected Library audit commands must not append events"
    );
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
    assert!(events[0]
        .payload_json
        .contains("\"actor_scope\":\"gui_admin\""));
}

#[test]
fn workflow_retention_cleanup_expires_artifacts_through_projection() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");
    ledger
        .append_diagnostic_event(sample_io_artifact_event(
            "node-a",
            "workflow_output",
            "artifact-expired",
        ))
        .expect("artifact event appends");
    let service = WorkflowService::new().with_diagnostics_ledger(ledger);

    let response = service
        .workflow_retention_cleanup_apply(WorkflowRetentionCleanupRequest {
            limit: Some(10),
            reason: "developer requested cleanup".to_string(),
        })
        .expect("retention cleanup applies");

    assert_eq!(response.cleanup.policy_id, "standard-local-v1");
    assert_eq!(response.cleanup.policy_version, 1);
    assert_eq!(response.cleanup.expired_artifact_count, 1);
    assert!(response.cleanup.last_event_seq.is_some());

    let artifacts = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: Some(IoArtifactRetentionState::Expired),
            retention_policy_id: Some("standard-local-v1".to_string()),
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("expired artifact query loads")
        .artifacts;
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].artifact_id, "artifact-expired");
    assert_eq!(artifacts[0].payload_ref, None);
    assert_eq!(
        artifacts[0].retention_reason.as_deref(),
        Some("developer requested cleanup; policy_version=1")
    );

    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger guard");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events load")
    };
    assert!(events.iter().any(|event| {
        event.event_kind
            == pantograph_diagnostics_ledger::DiagnosticEventKind::RetentionArtifactStateChanged
            && event.payload_json.contains("\"actor_scope\":\"gui_admin\"")
    }));
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
            workflow_execution_session_id: "exec-session-a".to_string(),
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
            canonical_error_event_id: None,
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
                model_cache_state: Some(SchedulerModelCacheState::Unknown),
                blocking_conditions: vec![
                    SchedulerEstimateBlockingCondition::RuntimeAdmissionPending,
                ],
                missing_asset_ids: Vec::new(),
                candidate_runtime_ids: Vec::new(),
                candidate_device_ids: Vec::new(),
                candidate_network_node_ids: Vec::new(),
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
            artifact_role: io_artifact_role(artifact_role),
            producer_node_id: matches!(artifact_role, "node_output" | "workflow_output")
                .then(|| "node-b".to_string()),
            producer_port_id: matches!(artifact_role, "node_output" | "workflow_output")
                .then(|| "out".to_string()),
            consumer_node_id: matches!(artifact_role, "node_input" | "workflow_input")
                .then(|| "node-b".to_string()),
            consumer_port_id: matches!(artifact_role, "node_input" | "workflow_input")
                .then(|| "in".to_string()),
            media_type: Some("text/plain".to_string()),
            size_bytes: Some(42),
            content_hash: Some("blake3:test".to_string()),
            retention_state: Some(IoArtifactRetentionState::Retained),
            retention_reason: None,
            payload_kind: None,
            lifecycle_state: None,
            access_modes: Vec::new(),
            read_handle: None,
            stream_handle: None,
            format: None,
        }),
    }
}

fn io_artifact_role(artifact_role: &str) -> IoArtifactRole {
    match artifact_role {
        "node_input" => IoArtifactRole::NodeInput,
        "node_output" => IoArtifactRole::NodeOutput,
        "workflow_input" => IoArtifactRole::WorkflowInput,
        "workflow_output" => IoArtifactRole::WorkflowOutput,
        _ => panic!("unsupported test artifact role: {artifact_role}"),
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
                actor_scope: RetentionPolicyActorScope::Maintenance,
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
            canonical_error_event_id: None,
            task_id: None,
            selected_backend_key: None,
            execution_cache_status: None,
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
            operation: LibraryAssetOperation::Download,
            cache_status: Some(LibraryAssetCacheStatus::Miss),
            network_bytes: Some(network_bytes),
        }),
    }
}

fn sample_artifact_attribution() -> ArtifactAttribution {
    ArtifactAttribution {
        workflow_run_id: "run-artifact".to_string(),
        workflow_id: Some("workflow-artifact".to_string()),
        workflow_version_id: Some("wfver-artifact".to_string()),
        node_id: Some("node-artifact".to_string()),
        port_id: Some("image".to_string()),
        model_id: None,
        runtime_id: None,
    }
}

fn artifact_policy_with_one_byte_limit() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "artifact-test-policy".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024),
        max_memory_bytes: Some(1024),
        max_single_artifact_bytes: Some(1),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

fn sample_runtime_model_error_scope() -> WorkflowDiagnosticRuntimeModelScope {
    WorkflowDiagnosticRuntimeModelScope {
        run: WorkflowDiagnosticRunContext {
            workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).unwrap(),
            workflow_id: WorkflowId::try_from("workflow-a".to_string()).unwrap(),
            workflow_version_id: Some(WorkflowVersionId::try_from("wfver-a".to_string()).unwrap()),
            workflow_semantic_version: Some("1.0.0".to_string()),
            client_id: Some(ClientId::try_from("client-a".to_string()).unwrap()),
            client_session_id: Some(ClientSessionId::try_from("session-a".to_string()).unwrap()),
            bucket_id: Some(BucketId::try_from("bucket-a".to_string()).unwrap()),
            scheduler_policy_id: Some("priority_then_fifo".to_string()),
            retention_policy_id: Some("ephemeral".to_string()),
        },
        runtime_id: "llama_cpp".to_string(),
        runtime_version: Some("b5012".to_string()),
        model_id: Some("qwen-27b".to_string()),
        model_version: Some("main".to_string()),
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

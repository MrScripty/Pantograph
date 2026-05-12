use super::*;
use crate::{WorkflowTechnicalFitCandidateSetSummary, WorkflowTechnicalFitSelectionPolicyTrace};

#[tokio::test]
async fn workflow_execution_session_lifecycle_create_run_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("generic-run".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    assert_eq!(created.runtime_capabilities.len(), 1);

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("hello session")
    );

    let closed = service
        .close_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

    let err = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("closed session should not run");
    assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));
}

#[tokio::test]
async fn workflow_execution_session_records_retained_node_io_artifact_bodies() {
    let host = MockWorkflowHost::new(8, 1024);
    let temp = tempfile::tempdir().expect("temp artifact store");
    let artifact_store =
        ArtifactStore::open(temp.path(), retained_io_test_artifact_policy()).expect("store");
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"))
        .with_artifact_store(artifact_store);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-retained-io".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("retained text"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let node_output_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
                && event
                    .payload_json
                    .contains("\"artifact_role\":\"node_output\"")
        })
        .expect("node output artifact event");
    assert!(!diagnostic_events.iter().any(|event| {
        event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
            && event
                .payload_json
                .contains("\"artifact_role\":\"node_input\"")
    }));
    assert_eq!(
        node_output_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    let payload: serde_json::Value =
        serde_json::from_str(&node_output_event.payload_json).expect("payload json");
    assert_eq!(payload["retention_state"], "retained");
    assert_eq!(payload["payload_kind"], "text");
    assert!(payload["artifact_fact_id"]
        .as_str()
        .is_some_and(|artifact_fact_id| artifact_fact_id.starts_with("workflow-io-fact-")));
    assert!(payload["payload_artifact_id"]
        .as_str()
        .is_some_and(|payload_artifact_id| payload_artifact_id.starts_with("workflow-io-")));
    assert!(payload["logical_payload_lineage_id"]
        .as_str()
        .is_some_and(|lineage_id| lineage_id.starts_with("workflow-io-lineage-")));
    assert_eq!(payload["producer_node_id"], "text-output-1");
    assert_eq!(payload["producer_port_id"], "text");
    assert!(payload["consumer_node_id"].is_null());
    assert!(payload["consumer_port_id"].is_null());
    let workflow_output_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
                && event
                    .payload_json
                    .contains("\"artifact_role\":\"workflow_output\"")
        })
        .expect("workflow output artifact event");
    let workflow_output_payload: serde_json::Value =
        serde_json::from_str(&workflow_output_event.payload_json).expect("workflow output payload");
    assert_eq!(
        workflow_output_payload["payload_artifact_id"],
        payload["payload_artifact_id"]
    );
    assert_eq!(
        workflow_output_payload["logical_payload_lineage_id"],
        payload["logical_payload_lineage_id"]
    );
    assert_ne!(
        workflow_output_payload["artifact_fact_id"],
        payload["artifact_fact_id"]
    );
    let artifact_id = payload["artifact_id"]
        .as_str()
        .expect("artifact id")
        .to_string();
    assert!(payload["read_handle"].as_str().is_some());

    let retained = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id,
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read retained node output artifact");
    assert_eq!(retained.body, b"retained text");
    let stats = service
        .artifact_store_stats()
        .expect("artifact store stats");
    assert_eq!(stats.retained_body_count, 2);
    assert_eq!(stats.retained_body_bytes, 26);
}

#[tokio::test]
async fn workflow_execution_session_run_passes_logical_session_id_in_run_options() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run keep-alive session");

    let recorded = host
        .recorded_run_options
        .lock()
        .expect("run options lock poisoned");
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].workflow_execution_session_id.as_deref(),
        Some(created.session_id.as_str())
    );
    assert_eq!(recorded[0].timeout_ms, None);
}

#[tokio::test]
async fn workflow_execution_session_repeated_runs_create_distinct_backend_run_ids() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create session");

    let first = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("first run");

    let second = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("second run");

    assert_ne!(first.workflow_run_id, created.session_id);
    assert_ne!(second.workflow_run_id, created.session_id);
    assert_ne!(first.workflow_run_id, second.workflow_run_id);
    assert!(first.workflow_run_id.starts_with("run_"));
    assert!(second.workflow_run_id.starts_with("run_"));

    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("session status");
    assert_eq!(status.session.run_count, 2);
}

#[tokio::test]
async fn workflow_execution_session_run_rejects_stale_graph_before_queue_admission() {
    let host = StaleWorkflowGraphHost::new();
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-stale".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.0.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("stale graph should be rejected before queue admission");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(error.message().contains("retired_node_type"));
    let Some(WorkflowErrorDetails::Graph(details)) = error.details() else {
        panic!("stale graph rejection should expose typed graph details");
    };
    assert!(details.graph_diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::WorkflowGraphDiagnosticCode::RetiredNodeType
            && diagnostic.node_id.as_deref() == Some("diffusion")
    }));
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: created.session_id,
        })
        .await
        .expect("list queue after rejected run");
    assert!(queue.items.is_empty());
    assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_execution_session_run_records_snapshot_before_execution() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candidate-managed-llama".to_string()),
            selected_runtime_id: Some("managed-llama-slot".to_string()),
            selected_runtime_variant_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: Some("model-a".to_string()),
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::RuntimeRequirements,
                Some("candidate-managed-llama"),
            )],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        },
    );
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-snapshot".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("snapshotted"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: Some(5000),
                priority: Some(7),
            },
        )
        .await
        .expect("run session");

    let snapshot = service
        .workflow_run_snapshot(&response.workflow_run_id)
        .expect("query snapshot")
        .expect("snapshot");
    assert_eq!(snapshot.workflow_run_id.as_str(), response.workflow_run_id);
    assert_eq!(snapshot.workflow_id.as_str(), "wf-snapshot");
    assert_eq!(snapshot.workflow_execution_session_id, created.session_id);
    assert_eq!(snapshot.workflow_execution_session_kind, "workflow");
    assert_eq!(snapshot.usage_profile, None);
    assert!(!snapshot.keep_alive);
    assert_eq!(snapshot.retention_policy, "ephemeral");
    assert_eq!(snapshot.scheduler_policy, "priority_then_fifo");
    assert_eq!(snapshot.workflow_semantic_version, "1.2.3");
    assert!(snapshot
        .workflow_presentation_revision_id
        .as_str()
        .starts_with("wfpres_"));
    assert_eq!(snapshot.priority, 7);
    assert_eq!(snapshot.timeout_ms, Some(5000));
    assert!(snapshot
        .workflow_execution_fingerprint
        .starts_with("workflow-exec-blake3:"));
    assert!(snapshot.inputs_json.contains("snapshotted"));
    assert!(snapshot.graph_settings_json.contains("text-input-1"));
    assert!(snapshot.runtime_requirements_json.contains("model-a"));
    assert!(snapshot
        .capability_models_json
        .contains("sha256:hash-model-a"));
    assert!(snapshot.runtime_capabilities_json.contains("llama_cpp"));

    let version_projection = service
        .workflow_run_version_projection(&response.workflow_run_id)
        .expect("query run version projection")
        .expect("projection");
    assert_eq!(
        version_projection.snapshot.workflow_run_id.as_str(),
        response.workflow_run_id
    );
    assert_eq!(
        version_projection.workflow_version.workflow_version_id,
        snapshot.workflow_version_id
    );
    assert_eq!(
        version_projection
            .presentation_revision
            .workflow_presentation_revision_id,
        snapshot.workflow_presentation_revision_id
    );
    assert_eq!(
        version_projection.workflow_version.semantic_version,
        "1.2.3"
    );
    assert!(version_projection
        .presentation_revision
        .presentation_metadata_json
        .contains("text-input-1"));
    assert!(version_projection
        .workflow_version
        .executable_topology_json
        .contains("text-input-1"));

    let run_graph = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
        })
        .expect("query run graph")
        .run_graph
        .expect("run graph");
    assert_eq!(run_graph.workflow_run_id, response.workflow_run_id);
    assert_eq!(run_graph.workflow_id, "wf-snapshot");
    assert_eq!(run_graph.workflow_semantic_version, "1.2.3");
    assert_eq!(
        run_graph.workflow_version_id,
        snapshot.workflow_version_id.as_str()
    );
    assert_eq!(
        run_graph.workflow_presentation_revision_id,
        snapshot.workflow_presentation_revision_id.as_str()
    );
    assert_eq!(run_graph.graph.nodes.len(), 2);
    assert_eq!(run_graph.graph.edges.len(), 1);
    assert_eq!(run_graph.graph.nodes[0].id, "text-input-1");
    assert_eq!(run_graph.graph.nodes[0].node_type, "text-input");
    assert_eq!(run_graph.graph.nodes[0].position.x, 0.0);
    assert_eq!(run_graph.graph.edges[0].id, "edge");
    assert!(!run_graph.executable_topology.nodes[0]
        .contract_version
        .is_empty());

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    assert_eq!(diagnostic_events.len(), 17);
    let event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::RunSnapshotAccepted
        })
        .expect("run snapshot accepted event");
    assert_eq!(
        event.event_kind,
        pantograph_diagnostics_ledger::DiagnosticEventKind::RunSnapshotAccepted
    );
    assert_eq!(
        event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::WorkflowService
    );
    assert_eq!(
        event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        event.workflow_version_id.as_ref(),
        Some(&snapshot.workflow_version_id)
    );
    assert_eq!(event.workflow_semantic_version.as_deref(), Some("1.2.3"));
    assert_eq!(
        event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert_eq!(event.retention_policy_id.as_deref(), Some("ephemeral"));
    assert!(event
        .payload_json
        .contains(snapshot.workflow_run_snapshot_id.as_str()));
    let snapshot_payload: serde_json::Value =
        serde_json::from_str(&event.payload_json).expect("snapshot payload json");
    assert_eq!(
        snapshot_payload["node_versions"].as_array().unwrap().len(),
        2
    );
    assert!(snapshot_payload["node_versions"][0]["contract_version"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(snapshot_payload["node_versions"][0]["behavior_digest"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let estimate_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerEstimateProduced
        })
        .expect("scheduler estimate event");
    assert_eq!(
        estimate_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert!(estimate_event.event_seq > event.event_seq);
    assert_eq!(
        estimate_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        estimate_event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert!(estimate_event
        .payload_json
        .contains("\"estimate_version\":\"session-scheduler-v1\""));
    assert!(estimate_event
        .payload_json
        .contains("\"confidence\":\"estimated\""));
    assert!(estimate_event
        .payload_json
        .contains("\"model_cache_state\":\"unknown\""));
    assert!(estimate_event.payload_json.contains(
        "\"blocking_conditions\":[\"runtime_admission_pending\",\"model_cache_unknown\"]"
    ));
    assert!(estimate_event
        .payload_json
        .contains("\"missing_asset_ids\":[]"));
    assert!(estimate_event
        .payload_json
        .contains("\"candidate_runtime_ids\":[\"llama_cpp\"]"));
    assert!(estimate_event
        .payload_json
        .contains("requires backend(s): llama_cpp"));
    assert!(estimate_event
        .payload_json
        .contains("requires model(s): model-a"));
    assert!(estimate_event
        .payload_json
        .contains("requires extension(s): inference_gateway"));
    assert!(estimate_event
        .payload_json
        .contains("estimated peak memory: 1024 MB VRAM, 2048 MB RAM"));
    assert!(estimate_event
        .payload_json
        .contains("candidate runtime(s): llama_cpp"));

    let queue_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerQueuePlacement
        })
        .expect("scheduler queue placement event");
    assert_eq!(
        queue_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        queue_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        queue_event.workflow_version_id.as_ref(),
        Some(&snapshot.workflow_version_id)
    );
    assert!(queue_event.event_seq > estimate_event.event_seq);
    assert_eq!(
        queue_event.scheduler_policy_id.as_deref(),
        Some("priority_then_fifo")
    );
    assert_eq!(
        queue_event.retention_policy_id.as_deref(),
        Some("ephemeral")
    );
    assert!(queue_event.payload_json.contains("\"queue_position\":0"));
    assert!(queue_event.payload_json.contains("\"priority\":7"));

    let admitted_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerRunAdmitted
        })
        .expect("scheduler run admitted event");
    assert_eq!(
        admitted_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        admitted_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(
        admitted_event.runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert!(admitted_event.event_seq > queue_event.event_seq);
    assert!(admitted_event.payload_json.contains("\"decision_reason\":"));
    assert!(admitted_event.payload_json.contains("\"queue_wait_ms\":"));
    assert!(admitted_event
        .payload_json
        .contains("\"selected_runtime_id\":\"managed-llama-slot\""));
    assert!(admitted_event
        .payload_json
        .contains("\"selected_backend_key\":\"llama_cpp\""));
    assert!(admitted_event
        .payload_json
        .contains("\"reserved_model_ids\":[\"model-a\"]"));

    let reservation_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerReservationChanged
        })
        .collect::<Vec<_>>();
    assert_eq!(reservation_events.len(), 2);
    assert!(reservation_events.iter().all(|event| event.source_component
        == pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler));
    assert!(reservation_events.iter().all(|event| event
        .workflow_run_id
        .as_ref()
        .map(|id| id.as_str())
        == Some(response.workflow_run_id.as_str())));
    assert_eq!(
        reservation_events[0].runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert_eq!(
        reservation_events[1].runtime_id.as_deref(),
        Some("managed-llama-slot")
    );
    assert!(reservation_events.iter().all(|event| event
        .payload_json
        .contains("\"resource_kind\":\"runtime_slot\"")));
    assert!(reservation_events.iter().all(|event| event
        .payload_json
        .contains("\"reserved_model_ids\":[\"model-a\"]")));
    assert!(reservation_events[0].event_seq > admitted_event.event_seq);
    assert!(reservation_events[0]
        .payload_json
        .contains("\"transition\":\"created\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"reason\":\"local runtime slot admitted\""));

    let started_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunStarted
        })
        .expect("run started event");
    assert_eq!(
        started_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler
    );
    assert_eq!(
        started_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(started_event.event_seq > reservation_events[0].event_seq);
    assert!(started_event
        .payload_json
        .contains("\"scheduler_decision_reason\":"));

    let model_lifecycle_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
        })
        .collect::<Vec<_>>();
    assert_eq!(model_lifecycle_events.len(), 5);
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.source_component
            == pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Scheduler));
    assert!(model_lifecycle_events.iter().all(|event| event
        .workflow_run_id
        .as_ref()
        .map(|id| id.as_str())
        == Some(response.workflow_run_id.as_str())));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.workflow_version_id.as_ref() == Some(&snapshot.workflow_version_id)));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("model-a")));
    assert!(model_lifecycle_events
        .iter()
        .all(|event| event.runtime_id.as_deref() == Some("managed-llama-slot")));
    assert!(model_lifecycle_events[0].event_seq > started_event.event_seq);
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"transition\":\"load_requested\""));
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"cache_state\":\"load_requested\""));
    assert!(model_lifecycle_events[0]
        .payload_json
        .contains("\"reason\":\"runtime admission requested required models\""));
    let load_requested_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[0].payload_json)
            .expect("load requested payload json");
    let load_dependency_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[1].payload_json)
            .expect("load dependency payload json");
    let timing_attempt_id = load_requested_payload["timing_attempt_id"]
        .as_str()
        .expect("load requested timing attempt id");
    assert!(timing_attempt_id.starts_with("timing_attempt_"));
    assert!(model_lifecycle_events[1].event_seq > model_lifecycle_events[0].event_seq);
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"transition\":\"load_dependency_resolved\""));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"cache_state\":\"load_requested\""));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"reason\":\"runtime admission resolved required model dependencies\""));
    assert_eq!(
        load_dependency_payload["timing_attempt_id"].as_str(),
        Some(timing_attempt_id)
    );
    assert!(model_lifecycle_events.iter().all(|event| !event
        .payload_json
        .contains("\"transition\":\"load_completed\"")));
    assert!(model_lifecycle_events[1]
        .payload_json
        .contains("\"duration_ms\":"));

    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("run terminal event");
    assert_eq!(
        terminal_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::WorkflowService
    );
    assert_eq!(
        terminal_event
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(terminal_event.event_seq > model_lifecycle_events[1].event_seq);
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"completed\""));
    assert!(terminal_event.payload_json.contains("\"duration_ms\":"));
    assert!(reservation_events[1].event_seq > terminal_event.event_seq);
    assert!(reservation_events[1]
        .payload_json
        .contains("\"transition\":\"released\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_runtime_id\":\"managed-llama-slot\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"reason\":\"workflow run finished\""));

    let io_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
        })
        .collect::<Vec<_>>();
    assert_eq!(io_events.len(), 3);
    assert!(io_events[0].event_seq > reservation_events[1].event_seq);
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_input\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_output\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"node_output\"")));
    assert!(io_events
        .iter()
        .all(|event| event.node_type.as_deref() == Some("text-output")));
    assert!(io_events.iter().all(|event| event
        .payload_json
        .contains("\"retention_state\":\"metadata_only\"")));
    let last_io_event_seq = io_events
        .iter()
        .map(|event| event.event_seq)
        .max()
        .expect("last io event");
    assert!(model_lifecycle_events[2].event_seq > last_io_event_seq);
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"transition\":\"unload_scheduled\""));
    let unload_scheduled_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[2].payload_json)
            .expect("unload scheduled payload json");
    let unload_started_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[3].payload_json)
            .expect("unload started payload json");
    let unload_completed_payload: serde_json::Value =
        serde_json::from_str(&model_lifecycle_events[4].payload_json)
            .expect("unload completed payload json");
    let unload_timing_attempt_id = unload_scheduled_payload["timing_attempt_id"]
        .as_str()
        .expect("unload scheduled timing attempt id");
    assert!(unload_timing_attempt_id.starts_with("timing_attempt_"));
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"cache_state\":\"unload_requested\""));
    assert!(model_lifecycle_events[2]
        .payload_json
        .contains("\"reason\":\"keep-alive disabled after run completion\""));
    assert!(model_lifecycle_events[3].event_seq > model_lifecycle_events[2].event_seq);
    assert!(model_lifecycle_events[3]
        .payload_json
        .contains("\"transition\":\"unload_started\""));
    assert_eq!(
        unload_started_payload["timing_attempt_id"].as_str(),
        Some(unload_timing_attempt_id)
    );
    assert!(model_lifecycle_events[3]
        .payload_json
        .contains("\"cache_state\":\"unload_requested\""));
    assert!(model_lifecycle_events[4].event_seq > model_lifecycle_events[3].event_seq);
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"transition\":\"unload_completed\""));
    assert_eq!(
        unload_completed_payload["timing_attempt_id"].as_str(),
        Some(unload_timing_attempt_id)
    );
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"cache_state\":\"unloaded\""));
    assert!(model_lifecycle_events[4]
        .payload_json
        .contains("\"duration_ms\":"));

    let library_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::LibraryAssetAccessed
        })
        .expect("library asset access event");
    assert_eq!(
        library_event.source_component,
        pantograph_diagnostics_ledger::DiagnosticEventSourceComponent::Library
    );
    assert_eq!(
        library_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert_eq!(library_event.model_id.as_deref(), Some("model-a"));
    assert!(library_event
        .payload_json
        .contains("\"asset_id\":\"pumas://models/model-a\""));
    assert!(library_event
        .payload_json
        .contains("\"operation\":\"run_usage\""));
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![WorkflowDiagnosticsProjectionKind::LibraryUsage],
            workflow_run_id: Some(response.workflow_run_id.clone()),
            workflow_id: Some("workflow-a".to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 100,
        })
        .expect("library usage projection refresh");

    let library_usage = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("pumas://models/model-a".to_string()),
            workflow_run_id: Some(response.workflow_run_id.clone()),
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(100),
        })
        .expect("library usage query");
    assert_eq!(library_usage.assets.len(), 1);
    assert_eq!(library_usage.assets[0].asset_id, "pumas://models/model-a");
    assert_eq!(library_usage.assets[0].run_access_count, 1);
}

#[tokio::test]
async fn workflow_execution_session_records_load_completed_only_with_runtime_proof() {
    let mut host = MockWorkflowHost::with_runtime_load_proof(
        8,
        1024,
        WorkflowSessionRuntimeLoadProof {
            backend_key: "llama_cpp".to_string(),
            runtime_id: Some("managed-llama-slot".to_string()),
            model_id: Some("model-a".to_string()),
            active_model_path: Some("/models/model-a.gguf".to_string()),
            requested_model_active: true,
        },
    );
    host.technical_fit_decision = Some(WorkflowTechnicalFitDecision {
        selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
        selected_candidate_id: Some("candidate-managed-llama".to_string()),
        selected_runtime_id: Some("managed-llama-slot".to_string()),
        selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
        selected_backend_key: Some("llama_cpp".to_string()),
        selected_model_id: Some("model-a".to_string()),
        selected_device_class: None,
        selected_device_id: None,
        resource_estimate: None,
        observed_throughput_hint: None,
        device_diagnostics: Vec::new(),
        reasons: vec![WorkflowTechnicalFitReason::new(
            WorkflowTechnicalFitReasonCode::RuntimeRequirements,
            Some("candidate-managed-llama"),
        )],
        selection_policy_trace: Some(WorkflowTechnicalFitSelectionPolicyTrace {
            policy_version: 1,
            candidate_set_summary: Some(WorkflowTechnicalFitCandidateSetSummary {
                total_candidate_count: 2,
                eligible_candidate_count: 2,
                rejected_candidate_count: 0,
                eligible_candidate_ids: vec![
                    "candidate-managed-llama".to_string(),
                    "candidate-pytorch".to_string(),
                ],
            }),
            ranking_reason: Some("candidate_priority".to_string()),
            exploration_reason: Some("equal_priority_seeded_choice".to_string()),
            seed_basis: Some(
                "workflow:wf-runtime-proof|snapshot:123|candidates:candidate-managed-llama,candidate-pytorch"
                    .to_string(),
            ),
        }),
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    });
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-proof".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run session");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let admission_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerRunAdmitted
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .expect("scheduler admission event");
    assert!(admission_event
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
    assert!(admission_event
        .payload_json
        .contains("\"selected_backend_key\":\"llama_cpp\""));
    assert!(admission_event
        .payload_json
        .contains("\"technical_fit_selection_policy_trace\""));
    assert!(admission_event
        .payload_json
        .contains("\"ranking_reason\":\"candidate_priority\""));
    assert!(admission_event
        .payload_json
        .contains("\"exploration_reason\":\"equal_priority_seeded_choice\""));

    let lifecycle_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
                && event
                    .workflow_run_id
                    .as_ref()
                    .map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .collect::<Vec<_>>();

    let load_requested = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_requested\"")
        })
        .expect("load requested event");
    let dependency_resolved = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_dependency_resolved\"")
        })
        .expect("dependency resolved event");
    let load_completed = lifecycle_events
        .iter()
        .find(|event| {
            event
                .payload_json
                .contains("\"transition\":\"load_completed\"")
        })
        .expect("load completed event");

    assert!(dependency_resolved.event_seq > load_requested.event_seq);
    assert!(load_completed.event_seq > dependency_resolved.event_seq);
    assert!(load_completed
        .payload_json
        .contains("\"cache_state\":\"loaded\""));
    assert!(load_completed
        .payload_json
        .contains("\"reason\":\"runtime admission proved requested model active\""));
    assert!(lifecycle_events.iter().all(|event| event
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\"")));
    let reservation_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerReservationChanged
                && event.workflow_run_id.as_ref().map(|id| id.as_str())
                    == Some(response.workflow_run_id.as_str())
        })
        .collect::<Vec<_>>();
    assert!(reservation_events[0]
        .payload_json
        .contains("\"transition\":\"created\""));
    assert!(reservation_events[0]
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"transition\":\"released\""));
    assert!(reservation_events[1]
        .payload_json
        .contains("\"selected_runtime_variant_id\":\"llama_cpp.cuda\""));
}

#[tokio::test]
async fn attributed_workflow_execution_session_carries_client_bucket_into_run_events() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("local gui".to_string()),
            metadata_json: None,
        })
        .expect("register client");
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("launch".to_string()),
        })
        .expect("open client session");

    let created = service
        .create_attributed_workflow_execution_session(
            &host,
            WorkflowExecutionSessionAttributedCreateRequest {
                workflow_id: "wf-attributed".to_string(),
                usage_profile: Some("developer".to_string()),
                keep_alive: false,
                attribution: WorkflowExecutionSessionAttributionRequest {
                    credential: registered.credential_proof_request(),
                    client_session_id: opened.session.client_session_id.as_str().to_string(),
                    bucket_selection: BucketSelection::Default,
                },
            },
        )
        .await
        .expect("create attributed session");

    assert_eq!(
        created
            .attribution
            .as_ref()
            .map(|context| context.client_id.as_str()),
        Some(registered.client.client_id.as_str())
    );
    assert_eq!(
        created
            .attribution
            .as_ref()
            .map(|context| context.bucket_id.as_str()),
        Some(opened.default_bucket.bucket_id.as_str())
    );

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("attributed"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run attributed session");

    let snapshot = service
        .workflow_run_snapshot(&response.workflow_run_id)
        .expect("query snapshot")
        .expect("snapshot");
    assert_eq!(
        snapshot.client_id,
        Some(registered.client.client_id.clone())
    );
    assert_eq!(
        snapshot.client_session_id,
        Some(opened.session.client_session_id.clone())
    );
    assert_eq!(snapshot.bucket_id, Some(opened.default_bucket.bucket_id));

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert!(diagnostic_events
        .iter()
        .all(|event| event.client_id.as_ref() == Some(&registered.client.client_id)));
    assert!(diagnostic_events
        .iter()
        .all(|event| event.client_session_id.as_ref() == Some(&opened.session.client_session_id)));
}

#[tokio::test]
async fn keep_alive_session_loads_runtime_with_keep_alive_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

    service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowExecutionSessionRetentionHint::KeepAlive]
    );
}

#[tokio::test]
async fn one_shot_session_run_loads_runtime_with_ephemeral_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create one-shot session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("run one-shot session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowExecutionSessionRetentionHint::Ephemeral]
    );
}

#[tokio::test]
async fn workflow_execution_session_run_records_failed_terminal_event_with_sanitized_error() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-control-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("runtime-error-control"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime error should fail the run");
    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("failed terminal event");
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"failed\""));
    assert!(terminal_event
        .payload_json
        .contains("llama.cpp stderr line"));
    assert!(!terminal_event.payload_json.chars().any(char::is_control));
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical node execution error event");
    assert!(error_event.payload_json.contains("node_execution"));
    assert!(error_event.payload_json.contains("backend not ready"));
    assert!(!error_event.payload_json.contains("\\n"));

    let terminal_workflow_run_id = terminal_event
        .workflow_run_id
        .as_ref()
        .expect("terminal event workflow run id")
        .as_str()
        .to_string();
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus,
            ],
            workflow_run_id: Some(terminal_workflow_run_id.clone()),
            workflow_id: terminal_event
                .workflow_id
                .as_ref()
                .map(|workflow_id| workflow_id.as_str().to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 20,
        })
        .expect("projection refresh");
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: terminal_workflow_run_id,
            projection_batch_size: Some(20),
        })
        .expect("run detail query")
        .run
        .expect("run detail");
    assert_eq!(detail.status, RunListProjectionStatus::Failed);
    assert!(!detail
        .terminal_error
        .as_deref()
        .unwrap_or_default()
        .chars()
        .any(char::is_control));
}

#[tokio::test]
async fn workflow_execution_session_run_snapshot_failure_records_canonical_error() {
    let host = FailingRunSnapshotHost::new();
    let service = WorkflowService::with_max_sessions(2)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().expect("store"))
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-snapshot-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("snapshot failure should fail the run");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical run snapshot error event");

    assert!(error_event.payload_json.contains("run_snapshot"));
    assert!(error_event.payload_json.contains("run_snapshot_failed"));
    assert_eq!(
        error
            .diagnostics()
            .and_then(|diagnostics| diagnostics.diagnostic_event_id.as_deref()),
        Some(error_event.event_id.as_str())
    );
}

#[tokio::test]
async fn workflow_execution_session_runtime_load_failure_records_canonical_error() {
    let host = FailingRuntimeLoadHost::new();
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-load-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime load should fail the run");
    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest { session_id })
        .await
        .expect("session status after runtime-load failure");
    assert_eq!(
        status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(status.session.run_count, 1);

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical runtime load error event");
    assert!(error_event.payload_json.contains("runtime_model_load"));
    assert!(error_event
        .payload_json
        .contains("runtime_model_load_failed"));
    assert!(error_event.payload_json.contains("llama.cpp spawn failed"));
    assert!(!error_event.payload_json.contains("\\n"));
    assert_eq!(
        error
            .diagnostics()
            .and_then(|diagnostics| diagnostics.diagnostic_event_id.as_deref()),
        Some(error_event.event_id.as_str())
    );

    let lifecycle_failed_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged
                && event.payload_json.contains("load_failed")
        })
        .expect("failed scheduler model lifecycle event");
    assert!(lifecycle_failed_event.payload_json.contains(&format!(
        "\"canonical_error_event_id\":\"{}\"",
        error_event.event_id
    )));

    let terminal_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
        })
        .expect("failed terminal event");
    assert!(terminal_event.payload_json.contains(&format!(
        "\"canonical_error_event_id\":\"{}\"",
        error_event.event_id
    )));
    let terminal_workflow_run_id = terminal_event
        .workflow_run_id
        .as_ref()
        .expect("terminal event workflow run id")
        .as_str()
        .to_string();
    service
        .workflow_diagnostics_projection_refresh(WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus,
            ],
            workflow_run_id: Some(terminal_workflow_run_id.clone()),
            workflow_id: terminal_event
                .workflow_id
                .as_ref()
                .map(|workflow_id| workflow_id.as_str().to_string()),
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 30,
        })
        .expect("projection refresh");
    let detail = service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: terminal_workflow_run_id,
            projection_batch_size: Some(30),
        })
        .expect("run detail query")
        .run
        .expect("run detail");
    assert_eq!(detail.status, RunListProjectionStatus::Failed);
}

#[tokio::test]
async fn workflow_execution_session_preserves_run_error_when_execution_diagnostics_unavailable() {
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let diagnostics_ledger = service
        .diagnostics_ledger
        .as_ref()
        .expect("diagnostics ledger configured")
        .clone();
    let host = FailingRunWithPoisonedDiagnosticsHost::new(diagnostics_ledger);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-execution-diagnostics-unavailable".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("workflow run should preserve execution failure");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(error.message().contains("workflow execution failed"));
    let diagnostics = error
        .diagnostics()
        .expect("diagnostics unavailable link should be attached");
    assert!(diagnostics.diagnostic_event_id.is_none());
    assert!(diagnostics
        .diagnostics_unavailable
        .as_deref()
        .unwrap_or_default()
        .contains("diagnostics ledger lock poisoned"));
}

#[tokio::test]
async fn workflow_execution_session_preserves_unload_error_when_unload_diagnostics_unavailable() {
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let diagnostics_ledger = service
        .diagnostics_ledger
        .as_ref()
        .expect("diagnostics ledger configured")
        .clone();
    let host = FailingUnloadWithPoisonedDiagnosticsHost::new(diagnostics_ledger);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-unload-diagnostics-unavailable".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("workflow run should preserve unload failure");

    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    assert!(error.message().contains("runtime unload failed"));
    assert!(!error.message().contains("diagnostics ledger lock poisoned"));
}

#[tokio::test]
async fn workflow_execution_session_runtime_load_failure_uses_phase_hint() {
    let host =
        FailingRuntimeLoadHost::with_phase_hint(WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary);
    let service = WorkflowService::with_max_sessions(2)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-runtime-load-error".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let error = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                workflow_semantic_version: "1.2.3".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect_err("runtime load should fail the run");
    assert_eq!(
        error.runtime_diagnostic_phase_hint(),
        Some(WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary)
    );

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 30,
        )
        .expect("diagnostic events")
    };
    let error_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("canonical runtime load error event");

    assert!(error_event.payload_json.contains("managed_binary"));
    assert!(error_event.payload_json.contains("managed_binary_failed"));
}

fn retained_io_test_artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "retained-io-test-policy".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(1024 * 1024),
        max_single_artifact_bytes: Some(1024 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

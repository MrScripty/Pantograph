use super::*;

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
async fn workflow_execution_session_run_records_snapshot_before_execution() {
    let host = MockWorkflowHost::new(8, 1024);
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

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert_eq!(diagnostic_events.len(), 1);
    let event = &diagnostic_events[0];
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

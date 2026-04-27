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
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    assert_eq!(diagnostic_events.len(), 9);
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
    assert_eq!(
        snapshot_payload["node_versions"][0]["contract_version"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        true
    );
    assert_eq!(
        snapshot_payload["node_versions"][0]["behavior_digest"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        true
    );

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
        .contains("\"confidence\":\"low\""));

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
    assert!(admitted_event.event_seq > queue_event.event_seq);
    assert!(admitted_event.payload_json.contains("\"decision_reason\":"));
    assert!(admitted_event.payload_json.contains("\"queue_wait_ms\":"));

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
    assert!(started_event.event_seq > admitted_event.event_seq);
    assert!(started_event
        .payload_json
        .contains("\"scheduler_decision_reason\":"));

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
    assert!(terminal_event.event_seq > started_event.event_seq);
    assert!(terminal_event
        .payload_json
        .contains("\"status\":\"completed\""));
    assert!(terminal_event.payload_json.contains("\"duration_ms\":"));

    let io_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::IoArtifactObserved
        })
        .collect::<Vec<_>>();
    assert_eq!(io_events.len(), 2);
    assert!(io_events[0].event_seq > terminal_event.event_seq);
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_input\"")));
    assert!(io_events.iter().any(|event| event
        .payload_json
        .contains("\"artifact_role\":\"workflow_output\"")));
    assert!(io_events.iter().all(|event| event
        .payload_json
        .contains("\"retention_state\":\"metadata_only\"")));

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

    let library_usage = service
        .workflow_library_usage_query(WorkflowLibraryUsageQueryRequest {
            asset_id: Some("pumas://models/model-a".to_string()),
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: Some(10),
            projection_batch_size: Some(10),
        })
        .expect("library usage query");
    assert_eq!(library_usage.assets.len(), 1);
    assert_eq!(library_usage.assets[0].asset_id, "pumas://models/model-a");
    assert_eq!(library_usage.assets[0].run_access_count, 1);
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

use super::*;

impl WorkflowService {
    async fn workflow_run<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_run_internal(host, request, None, None, None, None)
            .await
    }
}

#[tokio::test]
async fn workflow_run_returns_host_outputs() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();
    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect("workflow_run");

    assert!(!response.workflow_run_id.trim().is_empty());
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].value, serde_json::json!("hello world"));
}

#[tokio::test]
async fn workflow_run_rejects_invalid_workflow_semantic_version() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("expected invalid semantic version");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("workflow_semantic_version"))
    );
}

#[tokio::test]
async fn workflow_run_fails_when_host_returns_runtime_error() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("runtime-error object"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("expected runtime error");

    assert_eq!(err.code(), WorkflowErrorCode::RuntimeNotReady);
}

#[tokio::test]
async fn workflow_run_honors_blocking_backend_technical_fit_decision() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        10,
        256,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_runtime_variant_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![
                WorkflowTechnicalFitReason::new(
                    WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                    None,
                ),
                WorkflowTechnicalFitReason::new(
                    WorkflowTechnicalFitReasonCode::MissingCandidateData,
                    None,
                ),
            ],
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        },
    );
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("technical-fit decision should block run");

    assert_eq!(err.code(), WorkflowErrorCode::RuntimeNotReady);
    assert!(err
        .to_string()
        .contains("technical-fit could not select a ready runtime"));
}

#[tokio::test]
async fn workflow_run_returns_internal_when_host_emits_invalid_output_shape() {
    let host = MockWorkflowHost::with_invalid_output_binding(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("invalid host output should be internal");

    assert_eq!(err.code(), WorkflowErrorCode::InternalError);
    assert!(err
        .to_string()
        .contains("outputs.0.port_id must be non-empty"));
}

#[tokio::test]
async fn workflow_run_rejects_zero_timeout_ms() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: Some(0),
            },
        )
        .await
        .expect_err("expected invalid timeout");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("timeout_ms"));
}

#[tokio::test]
async fn workflow_run_timeout_cancels_host_within_grace_window() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let host = TimeoutAwareHost::new(cancelled.clone());
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-timeout".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: Some(25),
            },
        )
        .await
        .expect_err("expected timeout");

    assert_eq!(err.code(), WorkflowErrorCode::RuntimeTimeout);
    assert!(cancelled.load(Ordering::SeqCst));
}

#[tokio::test]
async fn workflow_run_rejects_empty_node_id() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("bad"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("expected invalid request");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[tokio::test]
async fn workflow_run_rejects_oversized_payload() {
    let host = MockWorkflowHost::new(10, 8);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("this is too large"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("expected capability violation");

    assert!(matches!(err, WorkflowServiceError::CapabilityViolation(_)));
}

#[tokio::test]
async fn workflow_run_accepts_discovered_output_targets() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let io = service
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("workflow io");
    let target_node = &io.outputs[0];
    let target_port = &target_node.ports[0];

    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: target_node.node_id.clone(),
                    port_id: target_port.port_id.clone(),
                    value: serde_json::json!("ok"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: target_node.node_id.clone(),
                    port_id: target_port.port_id.clone(),
                }]),
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect("workflow run with discovered target");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, target_node.node_id);
    assert_eq!(response.outputs[0].port_id, target_port.port_id);
}

#[tokio::test]
async fn workflow_run_rejects_non_discovered_output_targets() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "stream".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("non-discovered target should fail early");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[tokio::test]
async fn workflow_run_returns_output_not_produced_when_target_missing() {
    let host = MockWorkflowHost::with_missing_requested_output(8, 1024);
    let service = WorkflowService::new()
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("expected output_not_produced");

    assert_eq!(err.code(), WorkflowErrorCode::OutputNotProduced);
    assert!(err
        .to_string()
        .contains("requested output target 'text-output-1.text' was not produced"));
    let events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    let error_event = events
        .iter()
        .find(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::DiagnosticErrorOccurred
        })
        .expect("output validation diagnostic event");
    assert!(error_event.payload_json.contains("output_validation"));
    assert!(error_event
        .payload_json
        .contains("output_validation_failed"));
    assert_eq!(
        err.diagnostics()
            .and_then(|diagnostics| diagnostics.diagnostic_event_id.as_deref()),
        Some(error_event.event_id.as_str())
    );
}

#[tokio::test]
async fn workflow_run_rejects_duplicate_input_bindings() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![
                    WorkflowPortBinding {
                        node_id: "text-input-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("first"),
                    },
                    WorkflowPortBinding {
                        node_id: "text-input-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("second"),
                    },
                ],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
            },
        )
        .await
        .expect_err("duplicate bindings should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate binding"));
}

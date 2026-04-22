use super::*;

#[tokio::test]
async fn workflow_preflight_reports_missing_required_inputs_and_invalid_targets() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "stream".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(!response.can_run);
    assert_eq!(response.graph_fingerprint, "preflight-graph");
    assert_eq!(response.missing_required_inputs.len(), 1);
    assert_eq!(response.missing_required_inputs[0].node_id, "text-input-1");
    assert_eq!(response.missing_required_inputs[0].port_id, "text");
    assert_eq!(response.invalid_targets.len(), 1);
    assert_eq!(response.invalid_targets[0].node_id, "text-output-1");
    assert_eq!(response.invalid_targets[0].port_id, "stream");
    assert!(
        response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata"))
    );
}

#[tokio::test]
async fn workflow_preflight_can_run_when_inputs_and_targets_are_valid() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);
    assert_eq!(response.graph_fingerprint, "preflight-graph");
    assert!(response.missing_required_inputs.is_empty());
    assert!(response.invalid_targets.is_empty());
    assert!(
        response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata"))
    );
}

#[tokio::test]
async fn workflow_preflight_surfaces_backend_technical_fit_decision() {
    let host = PreflightHost::with_technical_fit_decision(
        WorkflowHostCapabilities {
            max_input_bindings: 16,
            max_output_targets: 16,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "estimated".to_string(),
                required_models: Vec::new(),
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: Vec::new(),
        },
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ConservativeFallback,
                Some("llama_cpp"),
            )],
        },
    );
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);
    assert_eq!(
        response.technical_fit_decision,
        Some(WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason {
                code: WorkflowTechnicalFitReasonCode::ConservativeFallback,
                candidate_id: Some("llama_cpp".to_string()),
            }],
        })
    );
    assert!(response.blocking_runtime_issues.is_empty());
    assert!(response.runtime_warnings.iter().any(|issue| {
        issue
            .message
            .contains("selected 'llama_cpp' conservatively")
    }));
}

#[tokio::test]
async fn workflow_preflight_blocks_selected_technical_fit_runtime_when_capability_is_not_ready() {
    let host = PreflightHost::with_technical_fit_decision(
        WorkflowHostCapabilities {
            max_input_bindings: 16,
            max_output_targets: 16,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "estimated".to_string(),
                required_models: Vec::new(),
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![WorkflowRuntimeCapability {
                runtime_id: "llama_cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: false,
                configured: false,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Failed),
                selected_version: Some("b8248".to_string()),
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: Some("validation failed".to_string()),
            }],
        },
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ConservativeFallback,
                Some("llama_cpp"),
            )],
        },
    );
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(!response.can_run);
    assert_eq!(response.blocking_runtime_issues.len(), 1);
    assert!(
        response.blocking_runtime_issues[0]
            .message
            .contains("validation failed")
    );
}

#[tokio::test]
async fn workflow_preflight_rejects_duplicate_output_targets() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let err = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![
                    WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    },
                    WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    },
                ]),
                override_selection: None,
            },
        )
        .await
        .expect_err("duplicate targets should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate target"));
}

#[tokio::test]
async fn workflow_preflight_normalizes_override_selection_into_technical_fit_request() {
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: Arc::new(AtomicUsize::new(0)),
        runtime_capabilities_calls: Arc::new(AtomicUsize::new(0)),
        graph_fingerprint: Arc::new(Mutex::new("graph-a".to_string())),
        technical_fit_requests: technical_fit_requests.clone(),
    };
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: Some(" model-a ".to_string()),
                    backend_key: Some("llama.cpp".to_string()),
                }),
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);

    let requests = technical_fit_requests
        .lock()
        .expect("technical-fit requests lock poisoned");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
}

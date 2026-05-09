use std::collections::BTreeMap;

use pantograph_diagnostics_ledger::{
    DiagnosticEventPayload, DiagnosticsLedgerRepository, DiagnosticsQuery, ExecutionGuaranteeLevel,
    LicenseSnapshot, ModelIdentity, ModelOutputMeasurement, NodeExecutionProjectionStatus,
    OutputMeasurementUnavailableReason, OutputModality, SqliteDiagnosticsLedger,
};
use pantograph_node_contracts::{
    EffectiveNodeContract, NodeAuthoringMetadata, NodeCapabilityRequirement, NodeCategory,
    NodeExecutionSemantics, NodeInstanceContext, NodeInstanceId, NodeTypeContract, NodeTypeId,
    PortContract, PortId, PortRequirement, PortValueType,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttribution, WorkflowRunId,
};
use pantograph_workflow_service::{
    ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowIoArtifactQueryRequest,
    WorkflowNodeStatusQueryRequest, WorkflowService,
};

use super::{
    build_kv_cache_diagnostic_event_ledger_append_request,
    inference_diagnostic_event_ledger_append_request_with_duration,
    NodeExecutionWorkflowLedgerNodeContext, NodeExecutionWorkflowLedgerSink,
};
use crate::{
    inference_diagnostic_event_ledger_append_request,
    inference_lifecycle_event_ledger_append_request, InferenceLifecycleLedgerRecorder,
    InferenceLifecycleWorkflowLedgerSink, ManagedCapabilityKind, ManagedCapabilityRoute,
    ManagedModelUsageSubmission, ModelExecutionCapability, NodeCancellationToken,
    NodeExecutionContext, NodeExecutionContextInput, NodeExecutionGuaranteeEvidence,
    NodeLineageContext, NodeManagedCapabilities, NodeProgressHandle, RuntimeLedgerSubmissionError,
};

#[test]
fn model_execution_capability_submits_usage_event_to_ledger() {
    let context = context();
    let capability = capability_for(&context);
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens");

    let submitted = capability
        .submit_usage_event(&mut ledger, &context, submission())
        .expect("usage submitted");

    assert_eq!(submitted.event.client_id.as_str(), "client-a");
    assert_eq!(submitted.event.workflow_run_id.as_str(), "run-a");
    assert_eq!(submitted.event.workflow_id.as_str(), "workflow-a");
    assert_eq!(submitted.event.lineage.node_id, "node-a");
    assert_eq!(submitted.event.lineage.node_type, "llm-inference");
    assert_eq!(submitted.event.lineage.port_ids, vec!["text".to_string()]);
    assert_eq!(
        submitted.event.lineage.composed_parent_chain,
        vec!["composed-parent".to_string()]
    );
    assert_eq!(
        submitted.event.guarantee_level,
        ExecutionGuaranteeLevel::ManagedFull
    );

    let projection = ledger
        .query_usage_events(DiagnosticsQuery::default())
        .expect("query succeeds");
    assert_eq!(projection.events, vec![submitted.event]);
}

#[test]
fn unavailable_measurement_downgrades_managed_full_guarantee() {
    let context = context();
    let capability = capability_for(&context);
    let mut usage = submission();
    usage.output_measurement.unavailable_reasons =
        vec![OutputMeasurementUnavailableReason::TokenizerUnavailable];

    let event = capability
        .build_usage_event(&context, usage)
        .expect("event builds");

    assert_eq!(
        event.guarantee_level,
        ExecutionGuaranteeLevel::ManagedPartial
    );
}

#[test]
fn capability_route_must_match_execution_context() {
    let context = context();
    let mut route = ManagedCapabilityRoute::from_context(
        ManagedCapabilityKind::ModelExecution,
        "llm",
        &context,
        true,
        true,
        None,
    );
    route.node_id = NodeInstanceId::try_from("other-node".to_string()).expect("node id");
    let capability = ModelExecutionCapability::new(route);

    let result = capability.build_usage_event(&context, submission());

    assert!(matches!(
        result,
        Err(RuntimeLedgerSubmissionError::ContextMismatch)
    ));
}

#[test]
fn unavailable_model_capability_is_not_recorded_as_usage() {
    let context = context();
    let capability = ModelExecutionCapability::new(ManagedCapabilityRoute::from_context(
        ManagedCapabilityKind::ModelExecution,
        "llm",
        &context,
        true,
        false,
        Some("model runtime unavailable".to_string()),
    ));

    let result = capability.build_usage_event(&context, submission());

    assert!(matches!(
        result,
        Err(RuntimeLedgerSubmissionError::CapabilityUnavailable)
    ));
}

#[test]
fn inference_lifecycle_event_adapter_builds_node_status_event_with_backend_context() {
    let context = context();
    let event = inference::InferenceRequestLifecycleEvent {
        request_id: Some("req-a".to_string()),
        phase: inference::InferenceLifecyclePhase::BackendExecution,
        kind: inference::InferenceRequestLifecycleEventKind::Failed,
        occurred_at_ms: 123,
        task_id: Some("text_generation".to_string()),
        backend_key: Some("pytorch".to_string()),
        runtime_id: Some("pytorch.transformers".to_string()),
        runtime_instance_id: Some("python-runtime:pytorch:1".to_string()),
        selected_device_id: Some("cuda:0".to_string()),
        selected_network_node_id: Some("local-node-alpha".to_string()),
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        resolved_artifact_kind: None,
        usage: None,
        cache_handle_id: None,
        artifact_refs: Vec::new(),
        detail: Some("backend failed".to_string()),
        canonical_error_event_id: Some("diagnostic-error-inference-a".to_string()),
        compatibility_report: None,
        compatibility_issues: Vec::new(),
        option_diagnostics: Vec::new(),
    };

    let request = inference_lifecycle_event_ledger_append_request(&context, &event)
        .expect("failed lifecycle event should map to ledger request");

    assert_eq!(
        request.source_instance_id.as_deref(),
        Some("python-runtime:pytorch:1")
    );
    assert_eq!(request.runtime_id.as_deref(), Some("pytorch.transformers"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    assert_eq!(request.node_id.as_deref(), Some("node-a"));
    assert_eq!(request.node_type.as_deref(), Some("llm-inference"));
    assert_eq!(request.occurred_at_ms, 123);
    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Failed);
            assert_eq!(payload.completed_at_ms, Some(123));
            assert_eq!(payload.error.as_deref(), Some("backend failed"));
            assert_eq!(
                payload.canonical_error_event_id.as_deref(),
                Some("diagnostic-error-inference-a")
            );
            assert_eq!(payload.task_id.as_deref(), Some("text_generation"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_event_adapter_drops_path_shaped_runtime_metadata() {
    let context = context();
    let mut event =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 124);
    event.runtime_instance_id = Some("/tmp/private/runtime.sock".to_string());
    event.runtime_id = Some("file:///tmp/private/pytorch-runtime".to_string());
    event.backend_key = Some("/tmp/private/backend".to_string());
    event.model_id = Some("/tmp/private/model.gguf".to_string());

    let request = inference_lifecycle_event_ledger_append_request(&context, &event)
        .expect("failed lifecycle event should map after unsafe metadata is dropped");

    assert!(request.source_instance_id.is_none());
    assert!(request.runtime_id.is_none());
    assert!(request.model_id.is_none());
    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert!(payload.selected_backend_key.is_none());
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Failed);
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_event_adapter_maps_contract_only_task_validation_failure() {
    let context = context();
    let event = inference::InferenceRequestLifecycleEvent {
        request_id: Some("exec-a:llm-inference-1:video_understanding".to_string()),
        phase: inference::InferenceLifecyclePhase::TaskValidation,
        kind: inference::InferenceRequestLifecycleEventKind::Failed,
        occurred_at_ms: 126,
        task_id: Some("video_understanding".to_string()),
        backend_key: Some("vllm".to_string()),
        runtime_id: Some("vllm".to_string()),
        runtime_instance_id: None,
        selected_device_id: None,
        selected_network_node_id: None,
        model_id: Some("pumas://models/video-understanding".to_string()),
        resolved_artifact_kind: None,
        usage: None,
        cache_handle_id: None,
        artifact_refs: Vec::new(),
        detail: Some(
            "Canonical inference task 'video_understanding' is contract-only at this execution boundary: task request contract has execution_supported=false for input kind 'video_understanding'."
                .to_string(),
        ),
        canonical_error_event_id: None,
        compatibility_report: None,
        compatibility_issues: Vec::new(),
        option_diagnostics: vec![inference::OptionCompatibilityDiagnostic {
            option_path: "video_understanding.max_frames".to_string(),
            state: inference::OptionSupportState::BackendUnavailable,
            backend_key: Some("vllm".to_string()),
            message: Some(
                "video_understanding is contract-only at this execution boundary; option support is deferred to an executable video backend"
                    .to_string(),
            ),
        }],
    };

    let request = inference_lifecycle_event_ledger_append_request(&context, &event)
        .expect("contract-only task validation failure should map to ledger request");

    assert_eq!(request.runtime_id.as_deref(), Some("vllm"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/video-understanding")
    );
    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Failed);
            assert_eq!(payload.completed_at_ms, Some(126));
            assert_eq!(payload.task_id.as_deref(), Some("video_understanding"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("vllm"));
            assert!(payload
                .error
                .as_deref()
                .is_some_and(|error| error.contains("execution_supported=false")));
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }

    let diagnostic_request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("contract-only task option diagnostics should map to durable diagnostic payload");
    match diagnostic_request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.task_id, "video_understanding");
            assert_eq!(payload.lifecycle_phase.as_deref(), Some("task_validation"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("vllm"));
            assert_eq!(payload.option_support_counts.backend_unavailable, 1);
            assert_eq!(payload.option_diagnostics.len(), 1);
            assert_eq!(
                payload.option_diagnostics[0].option_path,
                "video_understanding.max_frames"
            );
            assert_eq!(payload.option_diagnostics[0].state, "backend_unavailable");
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_event_adapter_bounds_failed_node_status_error() {
    let context = context();
    let mut event =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 125);
    event.detail = Some(format!("backend failed\n{}", "x".repeat(8_192)));

    let request = inference_lifecycle_event_ledger_append_request(&context, &event)
        .expect("failed lifecycle event should map to bounded ledger request");

    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            let error = payload.error.expect("bounded error detail");
            assert!(error.starts_with("backend failed "));
            assert!(!error.contains('\n'));
            assert!(error.len() <= pantograph_diagnostics_ledger::MAX_DIAGNOSTIC_ERROR_TEXT_LEN);
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_failed_event_preserves_canonical_error_link() {
    let context = context();
    let mut event =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 125);
    event.canonical_error_event_id = Some("diagnostic-error-runtime-model-load-1".to_string());

    let request = inference_lifecycle_event_ledger_append_request(&context, &event)
        .expect("failed lifecycle event should map to node status");

    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(
                payload.canonical_error_event_id.as_deref(),
                Some("diagnostic-error-runtime-model-load-1")
            );
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Failed);
            assert_eq!(payload.task_id.as_deref(), Some("text_generation"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
        }
        DiagnosticEventPayload::DiagnosticErrorOccurred(_) => {
            panic!("inference lifecycle adapter must not create a parallel diagnostic error event")
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_cleanup_event_is_not_persisted_as_node_status() {
    let context = context();
    let event = inference::InferenceRequestLifecycleEvent {
        request_id: Some("req-a".to_string()),
        phase: inference::InferenceLifecyclePhase::BackendExecution,
        kind: inference::InferenceRequestLifecycleEventKind::CleanupCompleted,
        occurred_at_ms: 124,
        task_id: Some("text_generation".to_string()),
        backend_key: Some("pytorch".to_string()),
        runtime_id: Some("pytorch.transformers".to_string()),
        runtime_instance_id: Some("python-runtime:pytorch:1".to_string()),
        selected_device_id: None,
        selected_network_node_id: None,
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        resolved_artifact_kind: None,
        usage: None,
        cache_handle_id: None,
        artifact_refs: Vec::new(),
        detail: None,
        canonical_error_event_id: None,
        compatibility_report: None,
        compatibility_issues: Vec::new(),
        option_diagnostics: Vec::new(),
    };

    assert!(inference_lifecycle_event_ledger_append_request(&context, &event).is_none());
}

#[test]
fn inference_diagnostic_event_adapter_builds_option_support_summary() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.option_diagnostics = vec![
        inference::OptionCompatibilityDiagnostic {
            option_path: "sampling.temperature".to_string(),
            state: inference::OptionSupportState::Mapped,
            backend_key: Some("pytorch".to_string()),
            message: Some(
                "mapped to backend temperature SECRET_PROMPT raw prompt --model /tmp/private/model.gguf"
                    .to_string(),
            ),
        },
        inference::OptionCompatibilityDiagnostic {
            option_path: "stopping.stop_strings".to_string(),
            state: inference::OptionSupportState::Unsupported,
            backend_key: Some("pytorch".to_string()),
            message: Some(
                "not mapped by this backend boundary PYTHON_KWARGS {'trust_remote_code': true}"
                    .to_string(),
            ),
        },
    ];
    event.compatibility_report = Some(inference::InferenceCompatibilityReportSummary {
        status: "rejected".to_string(),
        compatible: false,
        task: "supported".to_string(),
        model_source: "unsupported".to_string(),
        preprocessing: "supported".to_string(),
        postprocessing: "supported".to_string(),
    });
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "unsupported_model_artifact".to_string(),
        phase: inference::InferenceLifecyclePhase::ModelPackageResolution,
        message: "backend does not declare support for this artifact".to_string(),
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        path: Some("model.gguf".to_string()),
    }];
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(8),
        completion_tokens: Some(5),
        total_tokens: Some(13),
    });
    event.cache_handle_id = Some("kv-checkpoint-1".to_string());
    event.artifact_refs = vec!["artifact://audio.wav".to_string()];
    event.resolved_artifact_kind = Some("gguf".to_string());
    event.selected_device_id = Some("cuda:0".to_string());
    event.selected_network_node_id = Some("local-node-alpha".to_string());

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with option diagnostics should map");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("SECRET_PROMPT"));
    assert!(!payload_json.contains("--model"));
    assert!(!payload_json.contains("/tmp/private/model.gguf"));
    assert!(!payload_json.contains("PYTHON_KWARGS"));
    assert!(!payload_json.contains("trust_remote_code"));

    assert_eq!(request.node_id.as_deref(), Some("node-a"));
    assert_eq!(request.runtime_id.as_deref(), Some("pytorch.transformers"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.request_id, "req-a");
            assert_eq!(payload.task_id, "text_generation");
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
            assert_eq!(payload.selected_device_id.as_deref(), Some("cuda:0"));
            assert_eq!(
                payload.selected_network_node_id.as_deref(),
                Some("local-node-alpha")
            );
            assert_eq!(payload.resolved_artifact_kind.as_deref(), Some("gguf"));
            assert_eq!(
                payload.usage.as_ref().and_then(|usage| usage.total_tokens),
                Some(13)
            );
            assert_eq!(payload.cache_handle_id.as_deref(), Some("kv-checkpoint-1"));
            assert_eq!(
                payload.artifact_refs,
                vec!["artifact://audio.wav".to_string()]
            );
            assert_eq!(
                payload
                    .compatibility_report
                    .as_ref()
                    .map(|report| (report.status.as_str(), report.model_source.as_str())),
                Some(("rejected", "unsupported"))
            );
            assert_eq!(payload.compatibility_issue_count, 1);
            assert_eq!(
                payload.compatibility_issues[0].phase,
                "model_package_resolution"
            );
            assert_eq!(
                payload.compatibility_issues[0].path.as_deref(),
                Some("model.gguf")
            );
            assert_eq!(payload.option_support_counts.mapped, 1);
            assert_eq!(payload.option_support_counts.unsupported, 1);
            assert_eq!(payload.option_diagnostics.len(), 2);
            assert_eq!(
                payload.option_diagnostics[0].option_path,
                "sampling.temperature"
            );
            assert_eq!(payload.option_diagnostics[0].state, "mapped");
            assert!(payload
                .option_diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message.is_none()));
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_image_generation_bounded_lifecycle_summary() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.task_id = Some("image_generation".to_string());
    event.model_id = Some("image/example/tiny-diffusers".to_string());
    event.runtime_id = Some("pytorch.diffusers".to_string());
    event.backend_key = Some("pytorch".to_string());
    event.resolved_artifact_kind = Some("diffusers_bundle".to_string());
    event.detail = Some(
        "SECRET_PROMPT image prompt SECRET_IMAGE_BYTES aW1hZ2U= BACKEND_FLAG --model /tmp/private/diffusers"
            .to_string(),
    );
    event.option_diagnostics = vec![
        inference::OptionCompatibilityDiagnostic {
            option_path: "image_generation.width".to_string(),
            state: inference::OptionSupportState::Honored,
            backend_key: Some("pytorch".to_string()),
            message: Some("width honored for SECRET_PROMPT".to_string()),
        },
        inference::OptionCompatibilityDiagnostic {
            option_path: "image_generation.scheduler".to_string(),
            state: inference::OptionSupportState::Mapped,
            backend_key: Some("pytorch".to_string()),
            message: Some("mapped scheduler without storing aW1hZ2U=".to_string()),
        },
    ];
    event.compatibility_report = Some(inference::InferenceCompatibilityReportSummary {
        status: "accepted".to_string(),
        compatible: true,
        task: "supported".to_string(),
        model_source: "supported".to_string(),
        preprocessing: "supported".to_string(),
        postprocessing: "supported".to_string(),
    });
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "optional_component_missing".to_string(),
        phase: inference::InferenceLifecyclePhase::Preprocessing,
        message: "optional safety checker not present".to_string(),
        model_id: Some("image/example/tiny-diffusers".to_string()),
        path: Some("safety_checker/model.safetensors".to_string()),
    }];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("image-generation lifecycle diagnostics should map");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("SECRET_PROMPT"));
    assert!(!payload_json.contains("SECRET_IMAGE_BYTES"));
    assert!(!payload_json.contains("aW1hZ2U="));
    assert!(!payload_json.contains("BACKEND_FLAG"));
    assert!(!payload_json.contains("/tmp/private/diffusers"));

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.task_id, "image_generation");
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
            assert_eq!(
                payload.selected_backend_family.as_deref(),
                Some("transformers_pytorch")
            );
            assert_eq!(
                payload.resolved_artifact_kind.as_deref(),
                Some("diffusers_bundle")
            );
            assert_eq!(
                payload
                    .compatibility_report
                    .as_ref()
                    .map(|report| (report.status.as_str(), report.task.as_str())),
                Some(("accepted", "supported"))
            );
            assert_eq!(payload.compatibility_issue_count, 1);
            assert_eq!(payload.compatibility_issues[0].phase, "preprocessing");
            assert_eq!(payload.option_support_counts.honored, 1);
            assert_eq!(payload.option_support_counts.mapped, 1);
            assert_eq!(payload.option_diagnostics.len(), 2);
            assert!(payload
                .option_diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message.is_none()));
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_omits_absolute_issue_path_when_model_id_is_stable() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        176,
    );
    event.compatibility_report = Some(inference::InferenceCompatibilityReportSummary {
        status: "rejected".to_string(),
        compatible: false,
        task: "supported".to_string(),
        model_source: "unsupported".to_string(),
        preprocessing: "supported".to_string(),
        postprocessing: "supported".to_string(),
    });
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "unsupported_model_artifact".to_string(),
        phase: inference::InferenceLifecyclePhase::ModelPackageResolution,
        message: "backend does not declare support for this artifact".to_string(),
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        path: Some("/media/models/private/tiny-transformers/model.gguf".to_string()),
    }];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("compatibility lifecycle summary should map");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.compatibility_issue_count, 1);
            assert_eq!(
                payload.compatibility_issues[0].model_id.as_deref(),
                Some("pumas://models/tiny-transformers")
            );
            assert!(
                payload.compatibility_issues[0].path.is_none(),
                "stable model ids should replace absolute local compatibility issue paths"
            );
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_compatibility_only_summary() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        176,
    );
    event.compatibility_report = Some(inference::InferenceCompatibilityReportSummary {
        status: "rejected".to_string(),
        compatible: false,
        task: "supported".to_string(),
        model_source: "unsupported".to_string(),
        preprocessing: "supported".to_string(),
        postprocessing: "supported".to_string(),
    });
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "unsupported_model_artifact".to_string(),
        phase: inference::InferenceLifecyclePhase::ModelPackageResolution,
        message: "backend does not declare support for this artifact".to_string(),
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        path: Some("model.gguf".to_string()),
    }];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("compatibility-only lifecycle summary should map");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
            assert_eq!(payload.option_diagnostics.len(), 0);
            assert_eq!(payload.option_support_counts, Default::default());
            assert_eq!(
                payload
                    .compatibility_report
                    .as_ref()
                    .map(|report| report.status.as_str()),
                Some("rejected")
            );
            assert_eq!(payload.compatibility_issue_count, 1);
            assert_eq!(
                payload.compatibility_issues[0].kind,
                "unsupported_model_artifact"
            );
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_task_validation_compatibility_summary() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        177,
    );
    event.phase = inference::InferenceLifecyclePhase::TaskValidation;
    event.compatibility_report = Some(inference::InferenceCompatibilityReportSummary {
        status: "rejected".to_string(),
        compatible: false,
        task: "supported".to_string(),
        model_source: "unsupported".to_string(),
        preprocessing: "supported".to_string(),
        postprocessing: "supported".to_string(),
    });
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "unsupported_model_artifact".to_string(),
        phase: inference::InferenceLifecyclePhase::ModelPackageResolution,
        message: "backend does not declare support for this artifact".to_string(),
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        path: Some("model.gguf".to_string()),
    }];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("task-validation compatibility summary should map");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.lifecycle_phase.as_deref(), Some("task_validation"));
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
            assert_eq!(payload.task_id, "text_generation");
            assert_eq!(
                payload
                    .compatibility_report
                    .as_ref()
                    .map(|report| (report.status.as_str(), report.model_source.as_str())),
                Some(("rejected", "unsupported"))
            );
            assert_eq!(payload.compatibility_issue_count, 1);
            assert_eq!(
                payload.compatibility_issues[0].phase,
                "model_package_resolution"
            );
            assert_eq!(payload.option_diagnostics.len(), 0);
            assert_eq!(payload.option_support_counts, Default::default());
            assert!(payload.usage.is_none());
            assert!(payload.cache_handle_id.is_none());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_usage_and_cache_summary() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        177,
    );
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(21),
        completion_tokens: Some(34),
        total_tokens: Some(55),
    });
    event.cache_handle_id = Some("kv-checkpoint-2".to_string());
    event.detail = Some(
        "SECRET_PROMPT raw prompt text SECRET_RESULT generated text SECRET_TENSOR [1,2,3] \
         SECRET_DOCUMENT rerank document SECRET_EMBEDDING [0.1,0.2,0.3] \
         PYTHON_KWARGS {'trust_remote_code': true} BACKEND_FLAG --model /tmp/private/model.gguf"
            .to_string(),
    );

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with usage/cache metadata should map");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("SECRET_PROMPT"));
    assert!(!payload_json.contains("SECRET_RESULT"));
    assert!(!payload_json.contains("SECRET_TENSOR"));
    assert!(!payload_json.contains("SECRET_DOCUMENT"));
    assert!(!payload_json.contains("SECRET_EMBEDDING"));
    assert!(!payload_json.contains("PYTHON_KWARGS"));
    assert!(!payload_json.contains("BACKEND_FLAG"));
    assert!(!payload_json.contains("/tmp/private/model.gguf"));

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(
                payload.usage.as_ref().and_then(|usage| usage.prompt_tokens),
                Some(21)
            );
            assert_eq!(
                payload
                    .usage
                    .as_ref()
                    .and_then(|usage| usage.completion_tokens),
                Some(34)
            );
            assert_eq!(
                payload.usage.as_ref().and_then(|usage| usage.total_tokens),
                Some(55)
            );
            assert_eq!(payload.cache_handle_id.as_deref(), Some("kv-checkpoint-2"));
            assert_eq!(payload.option_diagnostics.len(), 0);
            assert!(payload.compatibility_report.is_none());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_drops_path_shaped_runtime_metadata() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        178,
    );
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(1),
        completion_tokens: Some(2),
        total_tokens: Some(3),
    });
    event.runtime_instance_id = Some("/tmp/private/runtime.sock".to_string());
    event.runtime_id = Some("file:///tmp/private/pytorch-runtime".to_string());
    event.backend_key = Some("/tmp/private/backend".to_string());
    event.model_id = Some("/tmp/private/model.gguf".to_string());
    event.selected_device_id = Some("/tmp/private/gpu0".to_string());
    event.selected_network_node_id = Some("~/private-node".to_string());
    event.compatibility_issues = vec![inference::InferenceCompatibilityIssueSummary {
        kind: "unsupported_model_artifact".to_string(),
        phase: inference::InferenceLifecyclePhase::ModelPackageResolution,
        message: "backend does not declare support for this artifact".to_string(),
        model_id: Some("/tmp/private/model.gguf".to_string()),
        path: Some("/tmp/private/model.gguf".to_string()),
    }];
    event.option_diagnostics = vec![inference::OptionCompatibilityDiagnostic {
        option_path: "sampling.temperature".to_string(),
        state: inference::OptionSupportState::Mapped,
        backend_key: Some("/tmp/private/backend".to_string()),
        message: None,
    }];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with usage should map");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("/tmp/private"));
    assert!(!payload_json.contains("file:///tmp/private"));
    assert!(!payload_json.contains("~/private-node"));

    assert!(request.source_instance_id.is_none());
    assert!(request.runtime_id.is_none());
    assert!(request.model_id.is_none());
    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert!(payload.selected_backend_key.is_none());
            assert!(payload.selected_backend_family.is_none());
            assert!(payload.selected_device_id.is_none());
            assert!(payload.selected_network_node_id.is_none());
            assert!(payload.compatibility_issues[0].model_id.is_none());
            assert!(payload.compatibility_issues[0].path.is_none());
            assert!(payload.option_diagnostics[0].backend_key.is_none());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_drops_local_path_cache_handle_id() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        177,
    );
    event.cache_handle_id = Some("/tmp/private/kv-cache.bin".to_string());

    assert!(inference_diagnostic_event_ledger_append_request(&context, &event).is_none());
}

#[test]
fn inference_diagnostic_event_adapter_filters_local_artifact_refs_before_ledger() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.artifact_refs = vec![
        "artifact://audio.wav".to_string(),
        "/tmp/private.wav".to_string(),
        "file:///tmp/private.wav".to_string(),
        "~/private.wav".to_string(),
        "C:\\Users\\jeremy\\private.wav".to_string(),
    ];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("stable artifact ref should keep lifecycle diagnostic persistable");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("/tmp/private.wav"));
    assert!(!payload_json.contains("file:///tmp/private.wav"));
    assert!(!payload_json.contains("~/private.wav"));
    assert!(!payload_json.contains("C:\\Users\\jeremy\\private.wav"));

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.artifact_refs, vec!["artifact://audio.wav"]);
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_task_validation_artifact_refs() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.phase = inference::InferenceLifecyclePhase::TaskValidation;
    event.artifact_refs = vec![
        "artifact://audio.wav".to_string(),
        "/tmp/private.wav".to_string(),
    ];

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("stable task-validation artifact ref should keep diagnostic persistable");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.lifecycle_phase.as_deref(), Some("task_validation"));
            assert_eq!(payload.artifact_refs, vec!["artifact://audio.wav"]);
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_drops_unsafe_only_artifact_refs() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.artifact_refs = vec![
        "/tmp/private.wav".to_string(),
        "file:///tmp/private.wav".to_string(),
        "~/private.wav".to_string(),
        "C:\\Users\\jeremy\\private.wav".to_string(),
    ];

    assert!(inference_diagnostic_event_ledger_append_request(&context, &event).is_none());
}

#[test]
fn inference_diagnostic_event_adapter_persists_bounded_resolved_artifact_kind() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.usage = None;
    event.cache_handle_id = None;
    event.compatibility_report = None;
    event.compatibility_issues.clear();
    event.option_diagnostics.clear();
    event.artifact_refs.clear();
    event.resolved_artifact_kind = Some("hf_compatible_directory".to_string());

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("bounded artifact kind should keep lifecycle diagnostic persistable");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(
                payload.resolved_artifact_kind.as_deref(),
                Some("hf_compatible_directory")
            );
            assert!(payload.artifact_refs.is_empty());
            assert!(payload.usage.is_none());
            assert!(payload.cache_handle_id.is_none());
            assert!(payload.compatibility_report.is_none());
            assert!(payload.compatibility_issues.is_empty());
            assert!(payload.option_diagnostics.is_empty());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_filters_unsafe_resolved_artifact_kind() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(2),
        completion_tokens: Some(3),
        total_tokens: Some(5),
    });
    event.resolved_artifact_kind = Some("/tmp/private/model.gguf".to_string());

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("usage should keep lifecycle diagnostic persistable");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("/tmp/private/model.gguf"));

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert!(payload.resolved_artifact_kind.is_none());
            assert_eq!(
                payload.usage.as_ref().and_then(|usage| usage.total_tokens),
                Some(5)
            );
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_carries_known_lifecycle_duration() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(8),
        completion_tokens: Some(5),
        total_tokens: Some(13),
    });

    let request =
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, Some(75))
            .expect("completed backend lifecycle with usage metadata should map");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.duration_ms, Some(75));
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_cancelled_lifecycle_duration() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Cancelled,
        175,
    );
    event.detail = Some("SECRET_PROMPT should not leak".to_string());

    let request =
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, Some(75))
            .expect("cancelled backend lifecycle with duration should map");

    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("SECRET_PROMPT"));
    assert_eq!(request.runtime_id.as_deref(), Some("pytorch.transformers"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.request_id, "req-a");
            assert_eq!(payload.task_id, "text_generation");
            assert_eq!(payload.duration_ms, Some(75));
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("cancelled"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
            assert_eq!(
                payload.selected_backend_family.as_deref(),
                Some("transformers_pytorch")
            );
            assert!(payload.usage.is_none());
            assert!(payload.cache_handle_id.is_none());
            assert!(payload.kv_cache.is_none());
            assert!(payload.compatibility_report.is_none());
            assert_eq!(payload.compatibility_issue_count, 0);
            assert!(payload.compatibility_issues.is_empty());
            assert_eq!(payload.option_support_counts, Default::default());
            assert!(payload.option_diagnostics.is_empty());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_normalizes_selected_backend_family() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        176,
    );
    event.backend_key = Some("llama.cpp".to_string());
    event.runtime_id = Some("runtime.llamacpp.1".to_string());
    event.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(3),
        completion_tokens: Some(2),
        total_tokens: Some(5),
    });

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with usage metadata should map");

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.selected_backend_key.as_deref(), Some("llama.cpp"));
            assert_eq!(
                payload.selected_backend_family.as_deref(),
                Some("llama_cpp")
            );
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_failed_lifecycle_duration_without_detail() {
    let context = context();
    let mut event =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 175);
    event.detail =
        Some("SECRET_PROMPT backend failure text should stay out of summaries".to_string());

    let request =
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, Some(75))
            .expect("failed backend lifecycle with duration should map");

    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("SECRET_PROMPT"));
    assert_eq!(request.runtime_id.as_deref(), Some("pytorch.transformers"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.request_id, "req-a");
            assert_eq!(payload.task_id, "text_generation");
            assert_eq!(payload.duration_ms, Some(75));
            assert_eq!(
                payload.lifecycle_phase.as_deref(),
                Some("backend_execution")
            );
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("failed"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
            assert!(payload.usage.is_none());
            assert!(payload.cache_handle_id.is_none());
            assert!(payload.kv_cache.is_none());
            assert!(payload.compatibility_report.is_none());
            assert_eq!(payload.compatibility_issue_count, 0);
            assert!(payload.compatibility_issues.is_empty());
            assert_eq!(payload.option_support_counts, Default::default());
            assert!(payload.option_diagnostics.is_empty());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn inference_diagnostic_event_adapter_persists_duration_only_non_backend_lifecycle() {
    let context = context();
    for (phase, phase_label, duration_ms) in [
        (
            inference::InferenceLifecyclePhase::ModelPackageResolution,
            "model_package_resolution",
            65,
        ),
        (
            inference::InferenceLifecyclePhase::Preprocessing,
            "preprocessing",
            75,
        ),
        (
            inference::InferenceLifecyclePhase::Postprocessing,
            "postprocessing",
            85,
        ),
        (
            inference::InferenceLifecyclePhase::ResultProjection,
            "result_projection",
            25,
        ),
    ] {
        let mut event = inference_lifecycle_event(
            inference::InferenceRequestLifecycleEventKind::Completed,
            175 + duration_ms,
        );
        event.phase = phase;
        event.usage = None;
        event.cache_handle_id = None;
        event.compatibility_report = None;
        event.compatibility_issues.clear();
        event.option_diagnostics.clear();

        let request = inference_diagnostic_event_ledger_append_request_with_duration(
            &context,
            &event,
            Some(duration_ms),
        )
        .expect("completed duration-only non-backend lifecycle should map");

        let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
        assert!(!payload_json.contains("SECRET_PROMPT"));
        match request.payload {
            DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
                assert_eq!(payload.duration_ms, Some(duration_ms));
                assert_eq!(payload.lifecycle_phase.as_deref(), Some(phase_label));
                assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("completed"));
                assert!(payload.usage.is_none());
                assert!(payload.cache_handle_id.is_none());
                assert!(payload.kv_cache.is_none());
                assert!(payload.compatibility_report.is_none());
                assert_eq!(payload.compatibility_issue_count, 0);
                assert!(payload.compatibility_issues.is_empty());
                assert_eq!(payload.option_support_counts, Default::default());
                assert!(payload.option_diagnostics.is_empty());
            }
            other => panic!("expected inference execution diagnostic payload, got {other:?}"),
        }
    }
}

#[test]
fn inference_diagnostic_event_adapter_skips_durationless_lifecycle_without_diagnostics() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    event.phase = inference::InferenceLifecyclePhase::Preprocessing;
    event.usage = None;
    event.cache_handle_id = None;
    event.compatibility_report = None;
    event.compatibility_issues.clear();
    event.option_diagnostics.clear();

    assert!(
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, None)
            .is_none()
    );
}

#[test]
fn inference_diagnostic_event_adapter_skips_durationless_cancelled_lifecycle_without_diagnostics() {
    let context = context();
    let mut event = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Cancelled,
        175,
    );
    event.usage = None;
    event.cache_handle_id = None;
    event.compatibility_report = None;
    event.compatibility_issues.clear();
    event.option_diagnostics.clear();

    assert!(
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, None)
            .is_none()
    );
}

#[test]
fn inference_diagnostic_event_adapter_skips_durationless_failed_lifecycle_without_diagnostics() {
    let context = context();
    let mut event =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 175);
    event.usage = None;
    event.cache_handle_id = None;
    event.compatibility_report = None;
    event.compatibility_issues.clear();
    event.option_diagnostics.clear();

    assert!(
        inference_diagnostic_event_ledger_append_request_with_duration(&context, &event, None)
            .is_none()
    );
}

#[test]
fn kv_cache_progress_detail_maps_to_bounded_inference_diagnostic_summary() {
    let workflow_id = WorkflowId::try_from("workflow-a".to_string()).expect("workflow id");
    let workflow_run_id = WorkflowRunId::try_from("run-a".to_string()).expect("run id");
    let contexts_by_node_id = BTreeMap::from([(
        "node-a".to_string(),
        NodeExecutionWorkflowLedgerNodeContext {
            node_id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
        },
    )]);

    let request = build_kv_cache_diagnostic_event_ledger_append_request(
        &workflow_id,
        &workflow_run_id,
        "run-a",
        &contexts_by_node_id,
        &node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            progress: 0.4,
            message: Some("cache restored".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::Truncate,
                    outcome: node_engine::KvCacheEventOutcome::Truncated,
                    cache_id: Some("cache-1".to_string()),
                    backend_key: Some("llamacpp".to_string()),
                    reuse_source: Some("llamacpp_slot".to_string()),
                    token_count: Some(64),
                    reason: Some("truncated_cache".to_string()),
                    option_diagnostics: vec![node_engine::KvCacheOptionDiagnostic {
                        option_path: "kv_cache.token_position".to_string(),
                        state: node_engine::KvCacheOptionSupportState::Honored,
                        backend_key: Some("llamacpp".to_string()),
                        message: Some(
                            "used as the truncation target TOKEN_ARRAY [1,2,3] LOGITS [0.1,0.2] /tmp/private/cache.bin"
                                .to_string(),
                        ),
                    }],
                },
            )),
            occurred_at_ms: Some(175),
        },
    )
    .expect("kv cache progress should map");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("TOKEN_ARRAY"));
    assert!(!payload_json.contains("LOGITS"));
    assert!(!payload_json.contains("/tmp/private/cache.bin"));

    assert_eq!(request.node_id.as_deref(), Some("node-a"));
    assert_eq!(request.runtime_id.as_deref(), Some("llamacpp"));
    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert_eq!(payload.request_id, "node-a:kv_cache");
            assert_eq!(payload.task_id, "kv_cache");
            assert_eq!(payload.lifecycle_phase.as_deref(), Some("kv_cache"));
            assert_eq!(payload.lifecycle_event_kind.as_deref(), Some("progress"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("llamacpp"));
            let kv_cache = payload.kv_cache.expect("kv cache summary");
            assert_eq!(kv_cache.action, "truncate");
            assert_eq!(kv_cache.outcome, "truncated");
            assert_eq!(kv_cache.cache_id.as_deref(), Some("cache-1"));
            assert_eq!(kv_cache.backend_key.as_deref(), Some("llamacpp"));
            assert_eq!(kv_cache.reuse_source.as_deref(), Some("llamacpp_slot"));
            assert_eq!(kv_cache.token_count, Some(64));
            assert_eq!(kv_cache.reason.as_deref(), Some("truncated_cache"));
            assert_eq!(payload.option_support_counts.honored, 1);
            assert_eq!(payload.option_diagnostics.len(), 1);
            assert_eq!(
                payload.option_diagnostics[0].option_path,
                "kv_cache.token_position"
            );
            assert_eq!(payload.option_diagnostics[0].state, "honored");
            assert!(payload.option_diagnostics[0].message.is_none());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn kv_cache_progress_detail_drops_path_shaped_metadata_before_ledger() {
    let workflow_id = WorkflowId::try_from("workflow-a".to_string()).expect("workflow id");
    let workflow_run_id = WorkflowRunId::try_from("run-a".to_string()).expect("run id");
    let contexts_by_node_id = BTreeMap::from([(
        "node-a".to_string(),
        NodeExecutionWorkflowLedgerNodeContext {
            node_id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
        },
    )]);

    let request = build_kv_cache_diagnostic_event_ledger_append_request(
        &workflow_id,
        &workflow_run_id,
        "run-a",
        &contexts_by_node_id,
        &node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            progress: 0.4,
            message: Some("cache restore attempted".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::RestoreInput,
                    outcome: node_engine::KvCacheEventOutcome::Miss,
                    cache_id: Some("/tmp/private/kv-cache.bin".to_string()),
                    backend_key: Some("/tmp/private/llamacpp".to_string()),
                    reuse_source: Some("file:///tmp/private/reuse-source".to_string()),
                    token_count: Some(64),
                    reason: Some("fallback used /tmp/private/history.bin".to_string()),
                    option_diagnostics: vec![node_engine::KvCacheOptionDiagnostic {
                        option_path: "kv_cache.reuse".to_string(),
                        state: node_engine::KvCacheOptionSupportState::Ignored,
                        backend_key: Some("/tmp/private/llamacpp".to_string()),
                        message: None,
                    }],
                },
            )),
            occurred_at_ms: Some(175),
        },
    )
    .expect("kv cache progress should map even when unsafe metadata is dropped");
    let payload_json = serde_json::to_string(&request.payload).expect("payload serializes");
    assert!(!payload_json.contains("/tmp/private"));
    assert!(!payload_json.contains("file:///tmp/private"));
    assert!(request.runtime_id.is_none());

    match request.payload {
        DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(payload) => {
            assert!(payload.selected_backend_key.is_none());
            assert!(payload.selected_backend_family.is_none());
            let kv_cache = payload.kv_cache.expect("kv cache summary");
            assert!(kv_cache.cache_id.is_none());
            assert!(kv_cache.reuse_source.is_none());
            assert!(kv_cache.reason.is_none());
            assert!(kv_cache.backend_key.is_none());
            assert_eq!(kv_cache.token_count, Some(64));
            assert!(payload.option_diagnostics[0].backend_key.is_none());
        }
        other => panic!("expected inference execution diagnostic payload, got {other:?}"),
    }
}

#[test]
fn kv_cache_workflow_sink_returns_diagnostics_unavailable_and_forwards_event_on_append_failure() {
    let service =
        std::sync::Arc::new(WorkflowService::with_ephemeral_diagnostics_ledger().expect("service"));
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let inner = std::sync::Arc::new(node_engine::VecEventSink::new());
    let sink = NodeExecutionWorkflowLedgerSink::try_new(
        service,
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
        Some(inner.clone()),
    )
    .expect("sink");
    let event = node_engine::WorkflowEvent::TaskProgress {
        task_id: "node-a".to_string(),
        execution_id: "run-a".to_string(),
        progress: 0.4,
        message: Some("cache restore attempted".to_string()),
        detail: Some(node_engine::TaskProgressDetail::KvCache(
            node_engine::KvCacheExecutionDiagnostics {
                action: node_engine::KvCacheEventAction::RestoreInput,
                outcome: node_engine::KvCacheEventOutcome::Unsupported,
                cache_id: Some("cache-1".to_string()),
                backend_key: Some("llamacpp".to_string()),
                reuse_source: None,
                token_count: None,
                reason: Some("x".repeat(2_048)),
                option_diagnostics: Vec::new(),
            },
        )),
        occurred_at_ms: Some(175),
    };

    let error =
        node_engine::EventSink::send(&sink, event.clone()).expect_err("append failure is returned");

    assert!(error.message.contains("diagnostics_unavailable"));
    assert!(error.message.contains("KV cache diagnostic"));
    assert_eq!(inner.events(), vec![event]);
}

#[test]
fn node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let artifact_store =
        ArtifactStore::open(temp.path(), retained_node_io_test_artifact_policy()).expect("store");
    let service = std::sync::Arc::new(
        WorkflowService::with_ephemeral_diagnostics_ledger()
            .expect("service")
            .with_artifact_store(artifact_store),
    );
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let sink = NodeExecutionWorkflowLedgerSink::try_new(
        service.clone(),
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
        None,
    )
    .expect("sink");

    node_engine::EventSink::send(
        &sink,
        node_engine::WorkflowEvent::TaskCompleted {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            output: Some(serde_json::json!({
                "response": "retained intermediate text"
            })),
            occurred_at_ms: Some(200),
        },
    )
    .expect("node output artifact should record");

    let artifacts = service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: Some("node_output".to_string()),
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
    assert_eq!(artifacts.artifacts.len(), 1);
    let artifact = &artifacts.artifacts[0];
    assert_eq!(artifact.producer_node_id.as_deref(), Some("node-a"));
    assert_eq!(artifact.producer_port_id.as_deref(), Some("response"));
    assert!(artifact
        .payload_ref
        .as_deref()
        .is_some_and(|payload_ref| payload_ref.starts_with("artifact://workflow-io-")));
    assert!(artifact.payload_ref.is_some());
    assert_eq!(artifact.media_type.as_deref(), Some("text/plain"));

    let body = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: artifact.artifact_id.clone(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("artifact body should be readable");
    assert_eq!(body.body, b"retained intermediate text");
}

#[test]
fn inference_lifecycle_recorder_projects_terminal_duration_after_started() {
    let context = context();
    let mut recorder = InferenceLifecycleLedgerRecorder::new();

    let started =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Started, 100);
    let started_request = recorder
        .append_request(&context, &started)
        .expect("started event should map");
    match started_request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Running);
            assert_eq!(payload.started_at_ms, Some(100));
            assert_eq!(payload.duration_ms, None);
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }

    let completed = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    let completed_request = recorder
        .append_request(&context, &completed)
        .expect("completed event should map");
    match completed_request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Completed);
            assert_eq!(payload.completed_at_ms, Some(175));
            assert_eq!(payload.duration_ms, Some(75));
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_recorder_leaves_duration_empty_without_matching_started() {
    let context = context();
    let mut recorder = InferenceLifecycleLedgerRecorder::new();

    let failed =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 175);
    let request = recorder
        .append_request(&context, &failed)
        .expect("failed event should map");

    assert_eq!(request.runtime_id.as_deref(), Some("pytorch.transformers"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Failed);
            assert_eq!(payload.completed_at_ms, Some(175));
            assert_eq!(payload.duration_ms, None);
            assert_eq!(payload.error.as_deref(), Some("backend failed"));
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_recorder_cleanup_clears_tracked_start_without_persisting() {
    let context = context();
    let mut recorder = InferenceLifecycleLedgerRecorder::new();

    let started =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Started, 100);
    assert!(recorder.append_request(&context, &started).is_some());

    let cleanup = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::CleanupCompleted,
        125,
    );
    assert!(recorder.append_request(&context, &cleanup).is_none());

    let completed = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    let request = recorder
        .append_request(&context, &completed)
        .expect("completed event should map");
    match request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => {
            assert_eq!(payload.status, NodeExecutionProjectionStatus::Completed);
            assert_eq!(payload.duration_ms, None);
        }
        other => panic!("expected node execution status payload, got {other:?}"),
    }
}

#[test]
fn inference_lifecycle_workflow_sink_records_cancelled_node_status_to_workflow_ledger() {
    let service =
        std::sync::Arc::new(WorkflowService::with_ephemeral_diagnostics_ledger().expect("service"));
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let sink = InferenceLifecycleWorkflowLedgerSink::try_new(
        service.clone(),
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
    )
    .expect("sink");

    let mut started =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Started, 100);
    started.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, started)
        .expect("started lifecycle records");

    let mut cancelled = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Cancelled,
        175,
    );
    cancelled.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, cancelled)
        .expect("cancelled lifecycle records");

    let response = service
        .workflow_node_status_query(WorkflowNodeStatusQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            projection_batch_size: Some(10),
            ..WorkflowNodeStatusQueryRequest::default()
        })
        .expect("node status query");

    assert_eq!(response.nodes.len(), 1);
    assert_eq!(response.nodes[0].node_id, "node-a");
    assert_eq!(
        response.nodes[0].status,
        NodeExecutionProjectionStatus::Cancelled
    );
    assert_eq!(response.nodes[0].duration_ms, Some(75));
    assert_eq!(
        response.nodes[0].runtime_id.as_deref(),
        Some("pytorch.transformers")
    );
    assert_eq!(
        response.nodes[0].model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
}

#[test]
fn inference_lifecycle_workflow_sink_records_failed_node_status_to_workflow_ledger() {
    let service =
        std::sync::Arc::new(WorkflowService::with_ephemeral_diagnostics_ledger().expect("service"));
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let sink = InferenceLifecycleWorkflowLedgerSink::try_new(
        service.clone(),
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
    )
    .expect("sink");

    let mut started =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Started, 100);
    started.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, started)
        .expect("started lifecycle records");

    let mut failed =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Failed, 175);
    failed.request_id = Some("run-a:node-a:LLM".to_string());
    failed.detail = Some("backend failed".to_string());
    failed.canonical_error_event_id = Some("diagnostic-error-inference-workflow".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, failed)
        .expect("failed lifecycle records");

    let response = service
        .workflow_node_status_query(WorkflowNodeStatusQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            projection_batch_size: Some(10),
            ..WorkflowNodeStatusQueryRequest::default()
        })
        .expect("node status query");

    assert_eq!(response.nodes.len(), 1);
    assert_eq!(
        response.nodes[0].status,
        NodeExecutionProjectionStatus::Failed
    );
    assert_eq!(response.nodes[0].duration_ms, Some(75));
    assert_eq!(
        response.nodes[0].selected_backend_key.as_deref(),
        Some("pytorch")
    );
    assert_eq!(response.nodes[0].error.as_deref(), Some("backend failed"));
    assert_eq!(
        response.nodes[0].canonical_error_event_id.as_deref(),
        Some("diagnostic-error-inference-workflow")
    );
}

#[test]
fn inference_lifecycle_workflow_sink_returns_diagnostics_unavailable_on_append_failure() {
    let service =
        std::sync::Arc::new(WorkflowService::with_ephemeral_diagnostics_ledger().expect("service"));
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let sink = InferenceLifecycleWorkflowLedgerSink::try_new(
        service.clone(),
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
    )
    .expect("sink");

    let mut completed = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    completed.request_id = Some(format!("run-a:node-a:{}", "x".repeat(512)));
    completed.usage = Some(inference::InferenceUsage {
        prompt_tokens: Some(8),
        completion_tokens: Some(5),
        total_tokens: Some(13),
    });

    let error = inference::InferenceRequestLifecycleEventSink::record(&sink, completed)
        .expect_err("oversized diagnostic request id returns unavailable error");

    assert_eq!(error.code, "diagnostics_unavailable");
    assert!(error
        .message
        .contains("failed to record inference lifecycle diagnostic"));
}

#[test]
fn inference_lifecycle_workflow_sink_records_node_status_to_workflow_ledger() {
    let service =
        std::sync::Arc::new(WorkflowService::with_ephemeral_diagnostics_ledger().expect("service"));
    let graph = node_engine::WorkflowGraph {
        id: "workflow-a".to_string(),
        name: "Workflow A".to_string(),
        nodes: vec![node_engine::GraphNode {
            id: "node-a".to_string(),
            node_type: "llm-inference".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let sink = InferenceLifecycleWorkflowLedgerSink::try_new(
        service.clone(),
        "workflow-a",
        "run-a",
        "run-a",
        &graph,
    )
    .expect("sink");

    let mut started =
        inference_lifecycle_event(inference::InferenceRequestLifecycleEventKind::Started, 100);
    started.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, started)
        .expect("started lifecycle records");

    let mut completed = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    completed.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, completed)
        .expect("completed lifecycle records");

    let response = service
        .workflow_node_status_query(WorkflowNodeStatusQueryRequest {
            workflow_run_id: Some("run-a".to_string()),
            node_id: Some("node-a".to_string()),
            projection_batch_size: Some(10),
            ..WorkflowNodeStatusQueryRequest::default()
        })
        .expect("node status query");

    assert_eq!(response.nodes.len(), 1);
    assert_eq!(response.nodes[0].node_id, "node-a");
    assert_eq!(
        response.nodes[0].status,
        NodeExecutionProjectionStatus::Completed
    );
    assert_eq!(
        response.nodes[0].runtime_id.as_deref(),
        Some("pytorch.transformers")
    );
    assert_eq!(
        response.nodes[0].model_id.as_deref(),
        Some("pumas://models/tiny-transformers")
    );
    assert_eq!(response.nodes[0].duration_ms, Some(75));
}

fn inference_lifecycle_event(
    kind: inference::InferenceRequestLifecycleEventKind,
    occurred_at_ms: u64,
) -> inference::InferenceRequestLifecycleEvent {
    let detail = if kind == inference::InferenceRequestLifecycleEventKind::Failed {
        Some("backend failed".to_string())
    } else {
        None
    };

    inference::InferenceRequestLifecycleEvent {
        request_id: Some("req-a".to_string()),
        phase: inference::InferenceLifecyclePhase::BackendExecution,
        kind,
        occurred_at_ms,
        task_id: Some("text_generation".to_string()),
        backend_key: Some("pytorch".to_string()),
        runtime_id: Some("pytorch.transformers".to_string()),
        runtime_instance_id: Some("python-runtime:pytorch:1".to_string()),
        selected_device_id: None,
        selected_network_node_id: None,
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        resolved_artifact_kind: None,
        usage: None,
        cache_handle_id: None,
        artifact_refs: Vec::new(),
        detail,
        canonical_error_event_id: None,
        compatibility_report: None,
        compatibility_issues: Vec::new(),
        option_diagnostics: Vec::new(),
    }
}

fn context() -> NodeExecutionContext {
    let node_type = NodeTypeId::try_from("llm-inference".to_string()).expect("node type");
    let contract = NodeTypeContract {
        node_type: node_type.clone(),
        category: NodeCategory::Processing,
        label: "LLM".to_string(),
        description: "Large language model inference".to_string(),
        inputs: vec![PortContract::input(
            PortId::try_from("prompt".to_string()).expect("port id"),
            "Prompt",
            PortValueType::Prompt,
            PortRequirement::Required,
        )],
        outputs: vec![PortContract::output(
            PortId::try_from("text".to_string()).expect("port id"),
            "Text",
            PortValueType::String,
        )],
        execution_semantics: NodeExecutionSemantics::Batch,
        capability_requirements: vec![NodeCapabilityRequirement::required("llm")],
        inference_tasks: Vec::new(),
        authoring: NodeAuthoringMetadata::default(),
        contract_version: Some("v1".to_string()),
        contract_digest: Some("digest-a".to_string()),
    };

    NodeExecutionContext::new(NodeExecutionContextInput {
        workflow_id: WorkflowId::try_from("workflow-a".to_string()).expect("workflow id"),
        attribution: WorkflowRunAttribution {
            client_id: ClientId::try_from("client-a".to_string()).expect("client id"),
            client_session_id: ClientSessionId::try_from("session-a".to_string())
                .expect("session id"),
            bucket_id: BucketId::try_from("bucket-a".to_string()).expect("bucket id"),
            workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).expect("run id"),
        },
        effective_contract: EffectiveNodeContract::from_static(
            NodeInstanceContext {
                node_instance_id: NodeInstanceId::try_from("node-a".to_string()).expect("node id"),
                node_type,
                graph_revision: Some("rev-a".to_string()),
                configuration: None,
            },
            contract,
        ),
        attempt: 1,
        created_at_ms: 100,
        cancellation: NodeCancellationToken::new(),
        progress: NodeProgressHandle::new(),
        lineage: NodeLineageContext {
            parent_composed_node_id: None,
            composed_node_stack: vec![
                NodeInstanceId::try_from("composed-parent".to_string()).expect("parent id")
            ],
            lineage_segment_id: Some("segment-a".to_string()),
        },
        capabilities: NodeManagedCapabilities::default(),
        guarantee_evidence: NodeExecutionGuaranteeEvidence::managed_full(),
    })
    .expect("context")
}

fn capability_for(context: &NodeExecutionContext) -> ModelExecutionCapability {
    ModelExecutionCapability::new(ManagedCapabilityRoute::from_context(
        ManagedCapabilityKind::ModelExecution,
        "llm",
        context,
        true,
        true,
        None,
    ))
}

fn submission() -> ManagedModelUsageSubmission {
    ManagedModelUsageSubmission::completed(
        ModelIdentity {
            model_id: "llm/imported/test".to_string(),
            model_revision: Some("rev-1".to_string()),
            model_hash: Some("sha256:abc".to_string()),
            model_modality: Some("text".to_string()),
            runtime_backend: Some("pytorch".to_string()),
        },
        LicenseSnapshot {
            license_value: Some("mit".to_string()),
            source_metadata_json: Some(r#"{"source":"pumas"}"#.to_string()),
            model_metadata_snapshot_json: Some(r#"{"model":"snapshot"}"#.to_string()),
            unavailable_reason: None,
        },
        ModelOutputMeasurement {
            modality: OutputModality::Text,
            item_count: Some(1),
            character_count: Some(11),
            byte_size: Some(11),
            token_count: Some(3),
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
        110,
        150,
    )
}

fn retained_node_io_test_artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "retained-node-io-test-policy".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(1024 * 1024),
        max_single_artifact_bytes: Some(1024 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

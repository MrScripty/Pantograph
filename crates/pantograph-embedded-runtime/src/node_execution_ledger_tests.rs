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
use pantograph_workflow_service::{WorkflowNodeStatusQueryRequest, WorkflowService};

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
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        usage: None,
        cache_handle_id: None,
        detail: Some("backend failed".to_string()),
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
            assert_eq!(payload.task_id.as_deref(), Some("text_generation"));
            assert_eq!(payload.selected_backend_key.as_deref(), Some("pytorch"));
        }
        other => panic!("expected node execution status payload, got {other:?}"),
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
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        usage: None,
        cache_handle_id: None,
        detail: None,
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
            message: Some("mapped to backend temperature".to_string()),
        },
        inference::OptionCompatibilityDiagnostic {
            option_path: "stopping.stop_strings".to_string(),
            state: inference::OptionSupportState::Unsupported,
            backend_key: Some("pytorch".to_string()),
            message: Some("not mapped by this backend boundary".to_string()),
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

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with option diagnostics should map");

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
            assert_eq!(
                payload.usage.as_ref().and_then(|usage| usage.total_tokens),
                Some(13)
            );
            assert_eq!(payload.cache_handle_id.as_deref(), Some("kv-checkpoint-1"));
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
            assert_eq!(payload.option_support_counts.mapped, 1);
            assert_eq!(payload.option_support_counts.unsupported, 1);
            assert_eq!(payload.option_diagnostics.len(), 2);
            assert_eq!(
                payload.option_diagnostics[0].option_path,
                "sampling.temperature"
            );
            assert_eq!(payload.option_diagnostics[0].state, "mapped");
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

    let request = inference_diagnostic_event_ledger_append_request(&context, &event)
        .expect("completed backend lifecycle with usage/cache metadata should map");

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
    inference::InferenceRequestLifecycleEventSink::record(&sink, started);

    let mut completed = inference_lifecycle_event(
        inference::InferenceRequestLifecycleEventKind::Completed,
        175,
    );
    completed.request_id = Some("run-a:node-a:LLM".to_string());
    inference::InferenceRequestLifecycleEventSink::record(&sink, completed);

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
        model_id: Some("pumas://models/tiny-transformers".to_string()),
        usage: None,
        cache_handle_id: None,
        detail,
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

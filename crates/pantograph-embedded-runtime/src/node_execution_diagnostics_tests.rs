use pantograph_node_contracts::{
    EffectiveNodeContract, NodeAuthoringMetadata, NodeCapabilityRequirement, NodeCategory,
    NodeExecutionSemantics, NodeInstanceContext, NodeInstanceId, NodeTypeContract, NodeTypeId,
    PortContract, PortId, PortRequirement, PortValueType,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttribution, WorkflowRunId,
};
use std::sync::Arc;

use node_engine::EventSink;

use crate::{
    adapt_node_engine_diagnostic_event, NodeCancellationToken, NodeExecutionContext,
    NodeExecutionContextInput, NodeExecutionDiagnosticEventKind, NodeExecutionDiagnosticsRecorder,
    NodeExecutionGuarantee, NodeExecutionGuaranteeEvidence, NodeLineageContext,
    NodeManagedCapabilities, NodeProgressHandle,
};

fn context() -> NodeExecutionContext {
    context_with_guarantee(NodeExecutionGuaranteeEvidence::managed_full())
}

fn context_with_guarantee(
    guarantee_evidence: NodeExecutionGuaranteeEvidence,
) -> NodeExecutionContext {
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
        attempt: 2,
        created_at_ms: 100,
        cancellation: NodeCancellationToken::new(),
        progress: NodeProgressHandle::new(),
        lineage: NodeLineageContext {
            parent_composed_node_id: Some(
                NodeInstanceId::try_from("node-parent".to_string()).expect("parent id"),
            ),
            composed_node_stack: Vec::new(),
            lineage_segment_id: Some("segment-a".to_string()),
        },
        capabilities: NodeManagedCapabilities::default(),
        guarantee_evidence,
    })
    .expect("context")
}

#[test]
fn adapter_enriches_task_started_with_runtime_context() {
    let event = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            occurred_at_ms: Some(125),
        },
    )
    .expect("diagnostic event");

    assert_eq!(event.kind, NodeExecutionDiagnosticEventKind::Started);
    assert_eq!(event.client_id.as_str(), "client-a");
    assert_eq!(event.client_session_id.as_str(), "session-a");
    assert_eq!(event.bucket_id.as_str(), "bucket-a");
    assert_eq!(event.workflow_id.as_str(), "workflow-a");
    assert_eq!(event.workflow_run_id.as_str(), "run-a");
    assert_eq!(event.node_id.as_str(), "node-a");
    assert_eq!(event.node_type.as_str(), "llm-inference");
    assert_eq!(event.attempt, 2);
    assert_eq!(event.occurred_at_ms, 125);
    assert_eq!(event.guarantee, NodeExecutionGuarantee::ManagedFull);
    assert_eq!(event.contract_digest.as_deref(), Some("digest-a"));
    assert_eq!(
        event.lineage.lineage_segment_id.as_deref(),
        Some("segment-a")
    );
}

#[test]
fn adapter_ignores_events_for_other_nodes_or_runs() {
    assert!(adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "node-b".to_string(),
            execution_id: "run-a".to_string(),
            occurred_at_ms: Some(125),
        },
    )
    .is_none());

    assert!(adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "node-a".to_string(),
            execution_id: "run-b".to_string(),
            occurred_at_ms: Some(125),
        },
    )
    .is_none());
}

#[test]
fn adapter_captures_completed_output_summaries_from_contract_ports() {
    let event = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskCompleted {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            output: Some(serde_json::json!({ "text": "hello" })),
            occurred_at_ms: Some(140),
        },
    )
    .expect("diagnostic event");

    assert_eq!(event.kind, NodeExecutionDiagnosticEventKind::Completed);
    assert_eq!(event.output_summaries.len(), 1);
    assert_eq!(event.output_summaries[0].port_id.as_str(), "text");
    assert_eq!(
        event.output_summaries[0].value_type,
        Some(PortValueType::String)
    );
    assert_eq!(event.output_summaries[0].byte_count, Some(5));
}

#[test]
fn adapter_projects_failure_progress_stream_and_cancellation_details() {
    let failed = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskFailed {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            error: "boom".to_string(),
            occurred_at_ms: Some(150),
        },
    )
    .expect("failed event");
    assert_eq!(failed.kind, NodeExecutionDiagnosticEventKind::Failed);
    assert_eq!(failed.error.as_deref(), Some("boom"));

    let progress = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            progress: 0.75,
            message: Some("almost".to_string()),
            detail: None,
            occurred_at_ms: Some(160),
        },
    )
    .expect("progress event");
    assert_eq!(progress.kind, NodeExecutionDiagnosticEventKind::Progress);
    assert_eq!(progress.progress, Some(0.75));
    assert_eq!(progress.message.as_deref(), Some("almost"));

    let stream = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::TaskStream {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            port: "text".to_string(),
            data: serde_json::json!("chunk"),
            occurred_at_ms: Some(170),
        },
    )
    .expect("stream event");
    assert_eq!(stream.kind, NodeExecutionDiagnosticEventKind::Stream);
    assert_eq!(stream.port_id.as_ref().map(PortId::as_str), Some("text"));
    assert_eq!(stream.output_summaries[0].byte_count, Some(5));

    let cancelled = adapt_node_engine_diagnostic_event(
        &context(),
        &node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: "workflow-a".to_string(),
            execution_id: "run-a".to_string(),
            error: "cancelled".to_string(),
            occurred_at_ms: Some(180),
        },
    )
    .expect("cancelled event");
    assert_eq!(cancelled.kind, NodeExecutionDiagnosticEventKind::Cancelled);
    assert_eq!(cancelled.error.as_deref(), Some("cancelled"));
}

#[test]
fn recorder_collects_diagnostics_and_forwards_original_events() {
    let forwarded = Arc::new(node_engine::VecEventSink::new());
    let recorder = NodeExecutionDiagnosticsRecorder::forwarding_to(forwarded.clone());
    recorder
        .register_context(context())
        .expect("register context");

    recorder
        .send(node_engine::WorkflowEvent::TaskStarted {
            task_id: "node-a".to_string(),
            execution_id: "run-a".to_string(),
            occurred_at_ms: Some(125),
        })
        .expect("record event");

    let diagnostics = recorder.events().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].kind,
        NodeExecutionDiagnosticEventKind::Started
    );
    assert!(matches!(
        forwarded.events().as_slice(),
        [node_engine::WorkflowEvent::TaskStarted { task_id, .. }] if task_id == "node-a"
    ));
}

#[test]
fn recorder_preserves_reduced_guarantee_classification() {
    let mut evidence = NodeExecutionGuaranteeEvidence::managed_full();
    evidence.unavailable_measurements = true;
    let recorder = NodeExecutionDiagnosticsRecorder::new();
    recorder
        .register_context(context_with_guarantee(evidence))
        .expect("register context");

    recorder
        .send(node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: "workflow-a".to_string(),
            execution_id: "run-a".to_string(),
            error: "cancelled".to_string(),
            occurred_at_ms: Some(180),
        })
        .expect("record event");

    let diagnostics = recorder.events().expect("diagnostics");
    assert_eq!(
        diagnostics[0].kind,
        NodeExecutionDiagnosticEventKind::Cancelled
    );
    assert_eq!(
        diagnostics[0].guarantee,
        NodeExecutionGuarantee::ManagedPartial
    );
}

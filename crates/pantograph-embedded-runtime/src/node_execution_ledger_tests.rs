use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, DiagnosticsQuery, ExecutionGuaranteeLevel, LicenseSnapshot,
    ModelIdentity, ModelOutputMeasurement, OutputMeasurementUnavailableReason, OutputModality,
    SqliteDiagnosticsLedger,
};
use pantograph_node_contracts::{
    EffectiveNodeContract, NodeAuthoringMetadata, NodeCapabilityRequirement, NodeCategory,
    NodeExecutionSemantics, NodeInstanceContext, NodeInstanceId, NodeTypeContract, NodeTypeId,
    PortContract, PortId, PortRequirement, PortValueType,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttribution, WorkflowRunId,
};

use crate::{
    ManagedCapabilityKind, ManagedCapabilityRoute, ManagedModelUsageSubmission,
    ModelExecutionCapability, NodeCancellationToken, NodeExecutionContext,
    NodeExecutionContextInput, NodeExecutionGuaranteeEvidence, NodeLineageContext,
    NodeManagedCapabilities, NodeProgressHandle, RuntimeLedgerSubmissionError,
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

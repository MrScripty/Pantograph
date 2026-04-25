use std::collections::BTreeMap;

use pantograph_node_contracts::{
    EffectiveNodeContract, NodeAuthoringMetadata, NodeCapabilityRequirement, NodeCategory,
    NodeExecutionSemantics, NodeInstanceContext, NodeInstanceId, NodeTypeContract, NodeTypeId,
    PortContract, PortId, PortRequirement, PortValueType,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttribution, WorkflowRunId,
};

use super::*;

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
    let effective_contract = EffectiveNodeContract::from_static(
        NodeInstanceContext {
            node_instance_id: NodeInstanceId::try_from("node-a".to_string()).expect("node id"),
            node_type,
            graph_revision: Some("rev-a".to_string()),
            configuration: None,
        },
        contract,
    );

    NodeExecutionContext::new(NodeExecutionContextInput {
        workflow_id: WorkflowId::try_from("workflow-a".to_string()).expect("workflow id"),
        attribution: WorkflowRunAttribution {
            client_id: ClientId::try_from("client-a".to_string()).expect("client id"),
            client_session_id: ClientSessionId::try_from("session-a".to_string())
                .expect("session id"),
            bucket_id: BucketId::try_from("bucket-a".to_string()).expect("bucket id"),
            workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).expect("run id"),
        },
        effective_contract,
        attempt: 1,
        created_at_ms: 100,
        cancellation: NodeCancellationToken::new(),
        progress: NodeProgressHandle::new(),
        lineage: NodeLineageContext::default(),
        capabilities: NodeManagedCapabilities::default(),
        guarantee_evidence: NodeExecutionGuaranteeEvidence::managed_full(),
    })
    .expect("context")
}

#[test]
fn context_preserves_runtime_owned_attribution_and_contract() {
    let context = context();

    assert_eq!(context.workflow_id().as_str(), "workflow-a");
    assert_eq!(context.workflow_run_id().as_str(), "run-a");
    assert_eq!(context.node_id().as_str(), "node-a");
    assert_eq!(context.node_type().as_str(), "llm-inference");
    assert_eq!(context.attempt(), 1);
    assert_eq!(context.guarantee(), NodeExecutionGuarantee::ManagedFull);
    assert_eq!(
        context.effective_contract().static_contract.contract_digest,
        Some("digest-a".to_string())
    );
}

#[test]
fn cancellation_token_is_shared_across_context_clones() {
    let context = context();
    let cancellation = context.cancellation().clone();

    assert!(context.ensure_not_cancelled().is_ok());
    cancellation.cancel();

    assert_eq!(
        context.ensure_not_cancelled(),
        Err(NodeExecutionError::Cancelled)
    );
}

#[test]
fn progress_handle_records_valid_progress_with_attribution() {
    let context = context();

    let event = context
        .report_progress(0.5, Some("halfway".to_string()), 125)
        .expect("progress");

    assert_eq!(event.workflow_id.as_str(), "workflow-a");
    assert_eq!(event.workflow_run_id.as_str(), "run-a");
    assert_eq!(event.node_id.as_str(), "node-a");
    assert_eq!(event.progress, 0.5);
    assert_eq!(
        context.progress().events().expect("events").as_slice(),
        &[event]
    );
}

#[test]
fn progress_handle_rejects_invalid_progress() {
    let context = context();

    assert_eq!(
        context.report_progress(1.5, None, 125),
        Err(NodeExecutionError::InvalidProgress)
    );
}

#[test]
fn guarantee_classification_downgrades_observability_gaps() {
    let mut evidence = NodeExecutionGuaranteeEvidence::managed_full();
    assert_eq!(evidence.classify(), NodeExecutionGuarantee::ManagedFull);

    evidence.unavailable_measurements = true;
    assert_eq!(evidence.classify(), NodeExecutionGuarantee::ManagedPartial);

    evidence.escape_hatch_used = true;
    assert_eq!(
        evidence.classify(),
        NodeExecutionGuarantee::EscapeHatchDetected
    );

    evidence.escape_hatch_used = false;
    evidence.attribution_resolved = false;
    assert_eq!(
        evidence.classify(),
        NodeExecutionGuarantee::UnsafeOrUnobserved
    );
}

#[test]
fn managed_capabilities_route_requirements_through_context() {
    let context = context();

    let capabilities = NodeManagedCapabilities::from_requirements(
        &context,
        &[
            NodeCapabilityRequirement::required("llm"),
            NodeCapabilityRequirement::required("kv_cache"),
            NodeCapabilityRequirement::required("resource:file"),
        ],
    );

    assert_eq!(capabilities.model_execution.len(), 1);
    assert_eq!(capabilities.cache.len(), 1);
    assert_eq!(capabilities.resource_access.len(), 1);
    assert_eq!(capabilities.progress.len(), 1);
    assert_eq!(capabilities.diagnostics.len(), 1);
    assert_eq!(
        capabilities.model_execution[0].route.workflow_run_id(),
        context.workflow_run_id()
    );
    assert!(capabilities.model_execution[0].ensure_available().is_ok());
}

#[test]
fn execution_input_and_output_capture_port_summaries() {
    let port_id = PortId::try_from("text".to_string()).expect("port id");
    let output = NodeExecutionOutput::new(BTreeMap::from([(
        port_id.clone(),
        serde_json::json!("hello"),
    )]));

    assert_eq!(output.values[&port_id], serde_json::json!("hello"));
    assert_eq!(output.summaries[0].byte_count, Some(5));
    assert_eq!(output.summaries[0].value_kind.as_deref(), Some("string"));
}

#[test]
fn context_rejects_zero_attempt() {
    let mut input = NodeExecutionContextInput {
        workflow_id: WorkflowId::try_from("workflow-a".to_string()).expect("workflow id"),
        attribution: WorkflowRunAttribution {
            client_id: ClientId::try_from("client-a".to_string()).expect("client id"),
            client_session_id: ClientSessionId::try_from("session-a".to_string())
                .expect("session id"),
            bucket_id: BucketId::try_from("bucket-a".to_string()).expect("bucket id"),
            workflow_run_id: WorkflowRunId::try_from("run-a".to_string()).expect("run id"),
        },
        effective_contract: context().effective_contract().clone(),
        attempt: 1,
        created_at_ms: 100,
        cancellation: NodeCancellationToken::new(),
        progress: NodeProgressHandle::new(),
        lineage: NodeLineageContext::default(),
        capabilities: NodeManagedCapabilities::default(),
        guarantee_evidence: NodeExecutionGuaranteeEvidence::managed_full(),
    };
    input.attempt = 0;

    assert_eq!(
        NodeExecutionContext::new(input).expect_err("invalid attempt"),
        NodeExecutionError::InvalidAttempt
    );
}

#[test]
fn lineage_context_projects_composed_parent_stack() {
    let base = NodeLineageContext::primitive().with_lineage_segment("outer-segment");
    let parent = NodeInstanceId::try_from("tool-loop".to_string()).expect("parent id");
    let nested_parent =
        NodeInstanceId::try_from("inner-composed-node".to_string()).expect("nested parent id");

    let lineage = base
        .enter_composed_node(parent.clone(), None)
        .enter_composed_node(nested_parent.clone(), Some("inner-segment".to_string()));

    assert_eq!(lineage.parent_composed_node_id, Some(nested_parent));
    assert_eq!(
        lineage.composed_parent_chain(),
        &[
            parent,
            NodeInstanceId::try_from("inner-composed-node".to_string()).expect("nested parent id")
        ]
    );
    assert_eq!(lineage.lineage_segment_id.as_deref(), Some("inner-segment"));
}

#[test]
fn lineage_context_inherits_outer_segment_when_entering_parent_without_segment() {
    let parent = NodeInstanceId::try_from("node-group".to_string()).expect("parent id");
    let lineage = NodeLineageContext::primitive()
        .with_lineage_segment("group-boundary")
        .enter_composed_node(parent.clone(), None);

    assert_eq!(lineage.parent_composed_node_id, Some(parent));
    assert_eq!(
        lineage
            .composed_parent_chain()
            .iter()
            .map(|node_id| node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["node-group"]
    );
    assert_eq!(
        lineage.lineage_segment_id.as_deref(),
        Some("group-boundary")
    );
}

impl ManagedCapabilityRoute {
    fn workflow_run_id(&self) -> &WorkflowRunId {
        &self.attribution.workflow_run_id
    }
}

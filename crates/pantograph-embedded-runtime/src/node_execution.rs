//! Runtime-created node execution context and managed capability contracts.
//!
//! Stage 03 keeps these contracts in the embedded runtime so normal node code
//! receives attribution, cancellation, progress, and capability routing facts
//! from the runtime rather than constructing observability state itself.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use pantograph_node_contracts::{
    EffectiveNodeContract, NodeInstanceId, NodeTypeId, PortId, PortValueType,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunAttribution, WorkflowRunId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[path = "node_execution_capabilities.rs"]
mod capabilities;

pub use capabilities::{
    CacheCapability, DiagnosticsCapability, ExternalToolCapability, ManagedCapabilityKind,
    ManagedCapabilityRoute, ModelExecutionCapability, NodeManagedCapabilities,
    ResourceAccessCapability,
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NodeExecutionError {
    #[error("node execution attempt must be greater than zero")]
    InvalidAttempt,
    #[error("node execution was cancelled")]
    Cancelled,
    #[error("progress value must be finite and between 0.0 and 1.0")]
    InvalidProgress,
    #[error("progress reporting is unavailable")]
    ProgressUnavailable,
    #[error("capability '{capability_id}' is unavailable")]
    CapabilityUnavailable { capability_id: String },
}

pub type NodeExecutionResult = Result<NodeExecutionOutput, NodeExecutionError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionInput {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<PortId, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summaries: Vec<NodeOutputSummary>,
}

impl NodeExecutionInput {
    pub fn new(values: BTreeMap<PortId, serde_json::Value>) -> Self {
        let summaries = values
            .iter()
            .map(|(port_id, value)| NodeOutputSummary::from_value(port_id.clone(), value))
            .collect();
        Self { values, summaries }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionOutput {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<PortId, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summaries: Vec<NodeOutputSummary>,
}

impl NodeExecutionOutput {
    pub fn new(values: BTreeMap<PortId, serde_json::Value>) -> Self {
        let summaries = values
            .iter()
            .map(|(port_id, value)| NodeOutputSummary::from_value(port_id.clone(), value))
            .collect();
        Self { values, summaries }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeOutputSummary {
    pub port_id: PortId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<PortValueType>,
    pub present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_count: Option<usize>,
}

impl NodeOutputSummary {
    pub fn unavailable(port_id: PortId, value_type: Option<PortValueType>) -> Self {
        Self {
            port_id,
            value_type,
            present: false,
            value_kind: None,
            byte_count: None,
            item_count: None,
        }
    }

    pub fn from_value(port_id: PortId, value: &serde_json::Value) -> Self {
        let (value_kind, byte_count, item_count) = summarize_json_value(value);
        Self {
            port_id,
            value_type: None,
            present: !value.is_null(),
            value_kind: Some(value_kind),
            byte_count,
            item_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeExecutionContextInput {
    pub workflow_id: WorkflowId,
    pub attribution: WorkflowRunAttribution,
    pub effective_contract: EffectiveNodeContract,
    pub attempt: u32,
    pub created_at_ms: u64,
    pub cancellation: NodeCancellationToken,
    pub progress: NodeProgressHandle,
    pub lineage: NodeLineageContext,
    pub capabilities: NodeManagedCapabilities,
    pub guarantee_evidence: NodeExecutionGuaranteeEvidence,
}

#[derive(Debug, Clone)]
pub struct NodeExecutionContext {
    workflow_id: WorkflowId,
    attribution: WorkflowRunAttribution,
    effective_contract: EffectiveNodeContract,
    attempt: u32,
    created_at_ms: u64,
    cancellation: NodeCancellationToken,
    progress: NodeProgressHandle,
    lineage: NodeLineageContext,
    capabilities: NodeManagedCapabilities,
    guarantee: NodeExecutionGuarantee,
}

impl NodeExecutionContext {
    pub fn new(input: NodeExecutionContextInput) -> Result<Self, NodeExecutionError> {
        if input.attempt == 0 {
            return Err(NodeExecutionError::InvalidAttempt);
        }

        Ok(Self {
            workflow_id: input.workflow_id,
            attribution: input.attribution,
            effective_contract: input.effective_contract,
            attempt: input.attempt,
            created_at_ms: input.created_at_ms,
            cancellation: input.cancellation,
            progress: input.progress,
            lineage: input.lineage,
            capabilities: input.capabilities,
            guarantee: input.guarantee_evidence.classify(),
        })
    }

    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    pub fn attribution(&self) -> &WorkflowRunAttribution {
        &self.attribution
    }

    pub fn workflow_run_id(&self) -> &WorkflowRunId {
        &self.attribution.workflow_run_id
    }

    pub fn node_id(&self) -> &NodeInstanceId {
        &self.effective_contract.context.node_instance_id
    }

    pub fn node_type(&self) -> &NodeTypeId {
        &self.effective_contract.context.node_type
    }

    pub fn effective_contract(&self) -> &EffectiveNodeContract {
        &self.effective_contract
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    pub fn cancellation(&self) -> &NodeCancellationToken {
        &self.cancellation
    }

    pub fn progress(&self) -> &NodeProgressHandle {
        &self.progress
    }

    pub fn lineage(&self) -> &NodeLineageContext {
        &self.lineage
    }

    pub fn capabilities(&self) -> &NodeManagedCapabilities {
        &self.capabilities
    }

    pub fn guarantee(&self) -> NodeExecutionGuarantee {
        self.guarantee
    }

    pub fn ensure_not_cancelled(&self) -> Result<(), NodeExecutionError> {
        if self.cancellation.is_cancelled() {
            Err(NodeExecutionError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub fn report_progress(
        &self,
        progress: f32,
        message: Option<String>,
        occurred_at_ms: u64,
    ) -> Result<NodeProgressEvent, NodeExecutionError> {
        self.progress.report(NodeProgressEvent {
            workflow_id: self.workflow_id.clone(),
            workflow_run_id: self.attribution.workflow_run_id.clone(),
            node_id: self.node_id().clone(),
            node_type: self.node_type().clone(),
            attempt: self.attempt,
            progress,
            message,
            occurred_at_ms,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl NodeCancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeProgressEvent {
    pub workflow_id: WorkflowId,
    pub workflow_run_id: WorkflowRunId,
    pub node_id: NodeInstanceId,
    pub node_type: NodeTypeId,
    pub attempt: u32,
    pub progress: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub occurred_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NodeProgressHandle {
    events: Arc<Mutex<Vec<NodeProgressEvent>>>,
}

impl NodeProgressHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn report(
        &self,
        event: NodeProgressEvent,
    ) -> Result<NodeProgressEvent, NodeExecutionError> {
        if !event.progress.is_finite() || !(0.0..=1.0).contains(&event.progress) {
            return Err(NodeExecutionError::InvalidProgress);
        }
        let mut events = self
            .events
            .lock()
            .map_err(|_| NodeExecutionError::ProgressUnavailable)?;
        events.push(event.clone());
        Ok(event)
    }

    pub fn events(&self) -> Result<Vec<NodeProgressEvent>, NodeExecutionError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| NodeExecutionError::ProgressUnavailable)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeLineageContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_composed_node_id: Option<NodeInstanceId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composed_node_stack: Vec<NodeInstanceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage_segment_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionGuarantee {
    ManagedFull,
    ManagedPartial,
    EscapeHatchDetected,
    UnsafeOrUnobserved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionGuaranteeEvidence {
    pub attribution_resolved: bool,
    pub effective_contract_resolved: bool,
    pub runtime_context_created: bool,
    pub baseline_lifecycle_events: bool,
    pub managed_capability_routing: bool,
    #[serde(default)]
    pub unavailable_measurements: bool,
    #[serde(default)]
    pub escape_hatch_used: bool,
}

impl NodeExecutionGuaranteeEvidence {
    pub fn managed_full() -> Self {
        Self {
            attribution_resolved: true,
            effective_contract_resolved: true,
            runtime_context_created: true,
            baseline_lifecycle_events: true,
            managed_capability_routing: true,
            unavailable_measurements: false,
            escape_hatch_used: false,
        }
    }

    pub fn classify(&self) -> NodeExecutionGuarantee {
        if self.escape_hatch_used {
            return NodeExecutionGuarantee::EscapeHatchDetected;
        }

        let managed_path = self.attribution_resolved
            && self.effective_contract_resolved
            && self.runtime_context_created
            && self.baseline_lifecycle_events
            && self.managed_capability_routing;

        if managed_path && self.unavailable_measurements {
            NodeExecutionGuarantee::ManagedPartial
        } else if managed_path {
            NodeExecutionGuarantee::ManagedFull
        } else {
            NodeExecutionGuarantee::UnsafeOrUnobserved
        }
    }
}

fn summarize_json_value(value: &serde_json::Value) -> (String, Option<usize>, Option<usize>) {
    match value {
        serde_json::Value::Null => ("null".to_string(), None, None),
        serde_json::Value::Bool(_) => ("boolean".to_string(), None, None),
        serde_json::Value::Number(_) => ("number".to_string(), None, None),
        serde_json::Value::String(value) => ("string".to_string(), Some(value.len()), None),
        serde_json::Value::Array(values) => ("array".to_string(), None, Some(values.len())),
        serde_json::Value::Object(values) => ("object".to_string(), None, Some(values.len())),
    }
}

#[cfg(test)]
#[path = "node_execution_tests.rs"]
mod tests;

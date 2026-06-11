use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyOverridePatchV1,
    DependencyReadinessDescriptorFingerprint, DependencyReadinessGraphRevision,
    DependencyReadinessValidationSessionId, DependencyReadinessValidationSnapshotId,
    DependencyRequirementsId,
};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerEstimateHint, SchedulerNodeId,
    SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTraitSetting, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};
use serde::{Deserialize, Serialize};

use crate::graph::WorkflowRuntimeSourceContext;

pub const WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION: u16 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskGraph {
    #[serde(default = "default_workflow_scheduler_task_graph_schema_version")]
    pub schema_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub tasks: Vec<WorkflowSchedulerTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTask {
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub node_type: String,
    pub execution_class: WorkflowSchedulerTaskExecutionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_task_ids: Vec<SchedulerTaskId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_bindings: Vec<WorkflowSchedulerTaskInputBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedulable_intent: Option<SchedulableTaskIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedulable_intent_template: Option<WorkflowSchedulerTaskIntentTemplate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_runtime_task_template: Option<WorkflowSchedulerNonRuntimeTaskTemplate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_input_task_template: Option<WorkflowSchedulerSourceInputTemplate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_descriptor_fingerprint: Option<InferenceInterfaceFingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_source_context: Option<WorkflowRuntimeSourceContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskExecutionClass {
    RuntimeInference,
    SourceInput,
    NonRuntimeNodeEngine,
    PumasMaterialization,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskInputBinding {
    pub source_node_id: SchedulerNodeId,
    pub source_task_id: SchedulerTaskId,
    pub source_port_id: String,
    pub target_port_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskIntentTemplate {
    pub task_type: pantograph_dependency_planning::DependencyTaskId,
    #[serde(default)]
    pub constraints: SchedulerRuntimeDeviceConstraints,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_settings: Vec<SchedulerTraitSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimate_hints: Vec<SchedulerEstimateHint>,
    pub dependency_readiness_source: WorkflowSchedulerDependencyReadinessSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerDependencyReadinessSource {
    pub graph_revision: DependencyReadinessGraphRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_session_id: Option<DependencyReadinessValidationSessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_snapshot_id: Option<DependencyReadinessValidationSnapshotId>,
    pub descriptor_fingerprint: DependencyReadinessDescriptorFingerprint,
    pub dependency_requirements_id: DependencyRequirementsId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
    pub dependency_override_fingerprint: DependencyOverrideFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "template_type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum WorkflowSchedulerNonRuntimeTaskTemplate {
    TextOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "template_type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum WorkflowSchedulerSourceInputTemplate {
    Text { port_id: String },
    Boolean { port_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskProjectionDiagnostic {
    pub severity: WorkflowSchedulerTaskProjectionDiagnosticSeverity,
    pub code: WorkflowSchedulerTaskProjectionDiagnosticCode,
    pub node_id: SchedulerNodeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerTaskProjectionDiagnosticSeverity {
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerTaskProjectionDiagnosticCode {
    InvalidNodeId,
    MissingInferenceDescriptor,
    StaleInferenceDescriptor,
    UnavailableInferenceDescriptor,
    InvalidInferenceDescriptor,
    MissingResourceEstimates,
    MissingNonRuntimeTemplateValue,
    InvalidNonRuntimeTemplateValue,
    UnsupportedNonRuntimeTaskTemplate,
}

fn default_workflow_scheduler_task_graph_schema_version() -> u16 {
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION
}

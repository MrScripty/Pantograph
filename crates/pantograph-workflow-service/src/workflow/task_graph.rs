use std::collections::{BTreeMap, BTreeSet};

use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_node_contracts::NodeTypeContract;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerEstimateHint, SchedulerNodeId,
    SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTraitSetting, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};

use super::task_execution_classification::classify_workflow_scheduler_task;
use super::task_graph_contracts::{
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerSourceInputTemplate,
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskIntentTemplate,
    WorkflowSchedulerTaskProjectionDiagnostic, WorkflowSchedulerTaskProjectionDiagnosticCode,
    WorkflowSchedulerTaskProjectionDiagnosticSeverity,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};
use super::WorkflowServiceError;
use crate::graph::{workflow_executable_topology, WorkflowGraph};

const PORT_TEXT: &str = "text";
const PORT_VALUE: &str = "value";
const NODE_TYPE_BOOLEAN_INPUT: &str = "boolean-input";
const NODE_TYPE_TEXT_INPUT: &str = "text-input";
const NODE_TYPE_TEXT_OUTPUT: &str = "text-output";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowSchedulerInferenceTaskProjections {
    by_node_id: BTreeMap<SchedulerNodeId, WorkflowSchedulerInferenceTaskProjection>,
}

impl WorkflowSchedulerInferenceTaskProjections {
    pub fn from_records(
        records: Vec<WorkflowSchedulerInferenceTaskProjection>,
    ) -> Result<Self, WorkflowServiceError> {
        let mut by_node_id = BTreeMap::new();
        for record in records {
            let node_id = record.node_id().clone();
            if by_node_id.insert(node_id.clone(), record).is_some() {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "duplicate inference task projection for node '{}'",
                    node_id.as_str()
                )));
            }
        }
        Ok(Self { by_node_id })
    }

    pub fn get(
        &self,
        node_id: &SchedulerNodeId,
    ) -> Option<&WorkflowSchedulerInferenceTaskProjection> {
        self.by_node_id.get(node_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowSchedulerInferenceTaskProjection {
    Ready(WorkflowSchedulerReadyInferenceTaskProjection),
    Blocked(WorkflowSchedulerBlockedInferenceTaskProjection),
}

impl WorkflowSchedulerInferenceTaskProjection {
    fn node_id(&self) -> &SchedulerNodeId {
        match self {
            Self::Ready(record) => &record.node_id,
            Self::Blocked(record) => &record.node_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSchedulerReadyInferenceTaskProjection {
    pub node_id: SchedulerNodeId,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub task_type: DependencyTaskId,
    pub model_ref: PumasModelRef,
    pub constraints: SchedulerRuntimeDeviceConstraints,
    pub trait_settings: Vec<SchedulerTraitSetting>,
    pub estimate_hints: Vec<SchedulerEstimateHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSchedulerBlockedInferenceTaskProjection {
    pub node_id: SchedulerNodeId,
    pub descriptor_fingerprint: Option<InferenceInterfaceFingerprint>,
    pub reason: WorkflowSchedulerBlockedInferenceTaskProjectionReason,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowSchedulerBlockedInferenceTaskProjectionReason {
    Missing,
    Stale,
    Unavailable,
    Invalid,
}

pub fn workflow_scheduler_task_graph(
    workflow_id: &WorkflowId,
    workflow_run_id: &WorkflowRunId,
    graph: &WorkflowGraph,
) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
    workflow_scheduler_task_graph_with_inference_projections(
        workflow_id,
        workflow_run_id,
        graph,
        &WorkflowSchedulerInferenceTaskProjections::default(),
    )
}

pub fn workflow_scheduler_task_graph_with_inference_projections(
    workflow_id: &WorkflowId,
    workflow_run_id: &WorkflowRunId,
    graph: &WorkflowGraph,
    inference_task_projections: &WorkflowSchedulerInferenceTaskProjections,
) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
    let workflow_id = scheduler_workflow_id(workflow_id)?;
    let workflow_run_id = scheduler_workflow_run_id(workflow_run_id)?;
    let topology = workflow_executable_topology(graph)?;
    let node_contracts = builtin_contracts_by_node_type()?;

    let mut incoming_edges = BTreeMap::<&str, Vec<_>>::new();
    for edge in &topology.edges {
        incoming_edges
            .entry(edge.target_node_id.as_str())
            .or_default()
            .push(edge);
    }

    let mut tasks = Vec::with_capacity(topology.nodes.len());
    for node in &topology.nodes {
        let node_id = scheduler_node_id(&node.node_id)?;
        let task_id = scheduler_task_id(&node.node_id)?;
        let input_bindings = input_bindings(node.node_id.as_str(), &incoming_edges)?;
        let dependency_task_ids = dependency_task_ids(&input_bindings);
        let execution_class = classify_workflow_scheduler_task(
            &node.node_type,
            node_contracts.get(node.node_type.as_str()),
        );
        let (
            schedulable_intent,
            schedulable_intent_template,
            inference_descriptor_fingerprint,
            diagnostics,
        ) = schedulable_intent_for_node(
            &workflow_id,
            &workflow_run_id,
            &node_id,
            &task_id,
            execution_class,
            inference_task_projections.get(&node_id),
        );
        let (non_runtime_task_template, non_runtime_diagnostics) =
            non_runtime_task_template_for_node(
                &node_id,
                &node.node_type,
                execution_class,
                &input_bindings,
            );
        let (source_input_task_template, source_input_diagnostics) =
            source_input_task_template_for_node(&node_id, &node.node_type, execution_class);
        let mut diagnostics = diagnostics;
        diagnostics.extend(non_runtime_diagnostics);
        diagnostics.extend(source_input_diagnostics);

        tasks.push(WorkflowSchedulerTask {
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            node_id,
            task_id,
            node_type: node.node_type.clone(),
            execution_class,
            dependency_task_ids,
            input_bindings,
            schedulable_intent,
            schedulable_intent_template,
            non_runtime_task_template,
            source_input_task_template,
            inference_descriptor_fingerprint,
            diagnostics,
        });
    }

    Ok(WorkflowSchedulerTaskGraph {
        schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id,
        workflow_run_id,
        tasks,
    })
}

fn builtin_contracts_by_node_type(
) -> Result<BTreeMap<String, NodeTypeContract>, WorkflowServiceError> {
    workflow_nodes::builtin_node_contracts()
        .map_err(|error| {
            WorkflowServiceError::CapabilityViolation(format!(
                "failed to load built-in node contracts: {error}"
            ))
        })
        .map(|contracts| {
            contracts
                .into_iter()
                .map(|contract| (contract.node_type.as_str().to_string(), contract))
                .collect()
        })
}

fn input_bindings(
    node_id: &str,
    incoming_edges: &BTreeMap<&str, Vec<&crate::graph::WorkflowExecutableTopologyEdge>>,
) -> Result<Vec<WorkflowSchedulerTaskInputBinding>, WorkflowServiceError> {
    let mut bindings = Vec::new();
    if let Some(edges) = incoming_edges.get(node_id) {
        for edge in edges {
            bindings.push(WorkflowSchedulerTaskInputBinding {
                source_node_id: scheduler_node_id(&edge.source_node_id)?,
                source_task_id: scheduler_task_id(&edge.source_node_id)?,
                source_port_id: edge.source_port_id.clone(),
                target_port_id: edge.target_port_id.clone(),
            });
        }
    }
    bindings.sort_by(|left, right| {
        left.source_task_id
            .cmp(&right.source_task_id)
            .then_with(|| left.source_port_id.cmp(&right.source_port_id))
            .then_with(|| left.target_port_id.cmp(&right.target_port_id))
    });
    Ok(bindings)
}

fn dependency_task_ids(bindings: &[WorkflowSchedulerTaskInputBinding]) -> Vec<SchedulerTaskId> {
    let mut seen = BTreeSet::new();
    bindings
        .iter()
        .filter_map(|binding| {
            seen.insert(binding.source_task_id.clone())
                .then(|| binding.source_task_id.clone())
        })
        .collect()
}

fn schedulable_intent_for_node(
    workflow_id: &SchedulerWorkflowId,
    workflow_run_id: &SchedulerWorkflowRunId,
    node_id: &SchedulerNodeId,
    task_id: &SchedulerTaskId,
    execution_class: WorkflowSchedulerTaskExecutionClass,
    inference_task_projection: Option<&WorkflowSchedulerInferenceTaskProjection>,
) -> (
    Option<SchedulableTaskIntent>,
    Option<WorkflowSchedulerTaskIntentTemplate>,
    Option<InferenceInterfaceFingerprint>,
    Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) {
    if execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
        return (None, None, None, Vec::new());
    }

    let Some(inference_task_projection) = inference_task_projection else {
        return (
            None,
            None,
            None,
            vec![diagnostic(
                node_id,
                None,
                WorkflowSchedulerTaskProjectionDiagnosticCode::MissingInferenceDescriptor,
                "inference scheduler tasks require a current validated inference descriptor",
            )],
        );
    };

    match inference_task_projection {
        WorkflowSchedulerInferenceTaskProjection::Ready(projection) => {
            let template = WorkflowSchedulerTaskIntentTemplate {
                task_type: projection.task_type.clone(),
                constraints: projection.constraints.clone(),
                trait_settings: projection.trait_settings.clone(),
                dependency_override_patches: Vec::new(),
                estimate_hints: projection.estimate_hints.clone(),
            };
            (
                Some(SchedulableTaskIntent {
                    contract_version:
                        pantograph_scheduler::SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
                    workflow_id: workflow_id.clone(),
                    workflow_run_id: workflow_run_id.clone(),
                    node_id: node_id.clone(),
                    task_id: task_id.clone(),
                    fairness_key: None,
                    task_type: template.task_type.clone(),
                    model_ref: projection.model_ref.clone(),
                    constraints: template.constraints.clone(),
                    trait_settings: template.trait_settings.clone(),
                    dependency_override_patches: template.dependency_override_patches.clone(),
                    estimate_hints: template.estimate_hints.clone(),
                }),
                Some(template),
                Some(projection.descriptor_fingerprint.clone()),
                Vec::new(),
            )
        }
        WorkflowSchedulerInferenceTaskProjection::Blocked(projection) => (
            None,
            None,
            projection.descriptor_fingerprint.clone(),
            vec![diagnostic(
                node_id,
                None,
                diagnostic_code_for_blocked_inference_projection(projection.reason),
                projection.message.clone(),
            )],
        ),
    }
}

fn non_runtime_task_template_for_node(
    node_id: &SchedulerNodeId,
    node_type: &str,
    execution_class: WorkflowSchedulerTaskExecutionClass,
    input_bindings: &[WorkflowSchedulerTaskInputBinding],
) -> (
    Option<WorkflowSchedulerNonRuntimeTaskTemplate>,
    Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) {
    if execution_class != WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine {
        return (None, Vec::new());
    }

    match node_type {
        NODE_TYPE_TEXT_OUTPUT => text_output_template(node_id, input_bindings),
        _ => (
            None,
            vec![diagnostic(
                node_id,
                None,
                WorkflowSchedulerTaskProjectionDiagnosticCode::UnsupportedNonRuntimeTaskTemplate,
                format!("non-runtime task type '{node_type}' has no typed scheduler template"),
            )],
        ),
    }
}

fn source_input_task_template_for_node(
    node_id: &SchedulerNodeId,
    node_type: &str,
    execution_class: WorkflowSchedulerTaskExecutionClass,
) -> (
    Option<WorkflowSchedulerSourceInputTemplate>,
    Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) {
    if execution_class != WorkflowSchedulerTaskExecutionClass::SourceInput {
        return (None, Vec::new());
    }

    match node_type {
        NODE_TYPE_TEXT_INPUT => (
            Some(WorkflowSchedulerSourceInputTemplate::Text {
                port_id: PORT_TEXT.to_string(),
            }),
            Vec::new(),
        ),
        NODE_TYPE_BOOLEAN_INPUT => (
            Some(WorkflowSchedulerSourceInputTemplate::Boolean {
                port_id: PORT_VALUE.to_string(),
            }),
            Vec::new(),
        ),
        _ => (
            None,
            vec![diagnostic(
                node_id,
                None,
                WorkflowSchedulerTaskProjectionDiagnosticCode::UnsupportedNonRuntimeTaskTemplate,
                format!("source-input task type '{node_type}' has no typed scheduler template"),
            )],
        ),
    }
}

fn text_output_template(
    node_id: &SchedulerNodeId,
    input_bindings: &[WorkflowSchedulerTaskInputBinding],
) -> (
    Option<WorkflowSchedulerNonRuntimeTaskTemplate>,
    Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) {
    if input_bindings
        .iter()
        .any(|binding| binding.target_port_id == PORT_TEXT)
    {
        (
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
            Vec::new(),
        )
    } else {
        (
            None,
            vec![diagnostic(
                node_id,
                Some(PORT_TEXT),
                WorkflowSchedulerTaskProjectionDiagnosticCode::MissingNonRuntimeTemplateValue,
                "text-output scheduler template requires a materialized upstream text input",
            )],
        )
    }
}

fn diagnostic_code_for_blocked_inference_projection(
    reason: WorkflowSchedulerBlockedInferenceTaskProjectionReason,
) -> WorkflowSchedulerTaskProjectionDiagnosticCode {
    match reason {
        WorkflowSchedulerBlockedInferenceTaskProjectionReason::Missing => {
            WorkflowSchedulerTaskProjectionDiagnosticCode::MissingInferenceDescriptor
        }
        WorkflowSchedulerBlockedInferenceTaskProjectionReason::Stale => {
            WorkflowSchedulerTaskProjectionDiagnosticCode::StaleInferenceDescriptor
        }
        WorkflowSchedulerBlockedInferenceTaskProjectionReason::Unavailable => {
            WorkflowSchedulerTaskProjectionDiagnosticCode::UnavailableInferenceDescriptor
        }
        WorkflowSchedulerBlockedInferenceTaskProjectionReason::Invalid => {
            WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidInferenceDescriptor
        }
    }
}

fn scheduler_workflow_id(value: &WorkflowId) -> Result<SchedulerWorkflowId, WorkflowServiceError> {
    SchedulerWorkflowId::parse(value.as_str()).map_err(map_scheduler_id_error)
}

fn scheduler_workflow_run_id(
    value: &WorkflowRunId,
) -> Result<SchedulerWorkflowRunId, WorkflowServiceError> {
    SchedulerWorkflowRunId::parse(value.as_str()).map_err(map_scheduler_id_error)
}

fn scheduler_node_id(value: &str) -> Result<SchedulerNodeId, WorkflowServiceError> {
    SchedulerNodeId::parse(value).map_err(map_scheduler_id_error)
}

fn scheduler_task_id(value: &str) -> Result<SchedulerTaskId, WorkflowServiceError> {
    SchedulerTaskId::parse(value).map_err(map_scheduler_id_error)
}

fn map_scheduler_id_error(
    error: pantograph_scheduler::SchedulerContractError,
) -> WorkflowServiceError {
    WorkflowServiceError::CapabilityViolation(format!(
        "workflow scheduler task graph has invalid scheduler identifier: {error}"
    ))
}

fn diagnostic(
    node_id: &SchedulerNodeId,
    port_id: Option<&str>,
    code: WorkflowSchedulerTaskProjectionDiagnosticCode,
    message: impl Into<String>,
) -> WorkflowSchedulerTaskProjectionDiagnostic {
    WorkflowSchedulerTaskProjectionDiagnostic {
        severity: WorkflowSchedulerTaskProjectionDiagnosticSeverity::Error,
        code,
        node_id: node_id.clone(),
        port_id: port_id.map(str::to_string),
        message: message.into(),
    }
}

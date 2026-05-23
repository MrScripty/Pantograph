use std::collections::{BTreeMap, BTreeSet};

use pantograph_dependency_planning::{
    DependencyTaskId, DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerEstimateHint, SchedulerNodeId,
    SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTraitId, SchedulerTraitSetting,
    SchedulerTraitValue, SchedulerWorkflowId, SchedulerWorkflowRunId,
};
use serde_json::Value;

use super::task_graph_contracts::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskInputBinding,
    WorkflowSchedulerTaskIntentTemplate, WorkflowSchedulerTaskProjectionDiagnostic,
    WorkflowSchedulerTaskProjectionDiagnosticCode,
    WorkflowSchedulerTaskProjectionDiagnosticSeverity,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};
use super::WorkflowServiceError;
use crate::graph::{workflow_executable_topology, WorkflowGraph};

const NODE_TYPE_LLM_INFERENCE: &str = "llm-inference";
const PORT_PUMAS_MODEL_REF: &str = "pumas_model_ref";
const PORT_TASK_KIND: &str = "task_kind";
const PORT_RUNTIME: &str = "runtime";
const PORT_DEVICE: &str = "device";
const PORT_DENOISING_SCHEDULER: &str = "denoising_scheduler";

pub fn workflow_scheduler_task_graph(
    workflow_id: &WorkflowId,
    workflow_run_id: &WorkflowRunId,
    graph: &WorkflowGraph,
) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
    let workflow_id = scheduler_workflow_id(workflow_id)?;
    let workflow_run_id = scheduler_workflow_run_id(workflow_run_id)?;
    let topology = workflow_executable_topology(graph)?;
    let node_data_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), &node.data))
        .collect::<BTreeMap<_, _>>();

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
        let data = node_data_by_id.get(node.node_id.as_str()).copied();
        let (schedulable_intent, schedulable_intent_template, diagnostics) =
            schedulable_intent_for_node(
                &workflow_id,
                &workflow_run_id,
                &node_id,
                &task_id,
                &node.node_type,
                data,
                &input_bindings,
            );

        tasks.push(WorkflowSchedulerTask {
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            node_id,
            task_id,
            node_type: node.node_type.clone(),
            dependency_task_ids,
            input_bindings,
            schedulable_intent,
            schedulable_intent_template,
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
    node_type: &str,
    data: Option<&Value>,
    input_bindings: &[WorkflowSchedulerTaskInputBinding],
) -> (
    Option<SchedulableTaskIntent>,
    Option<WorkflowSchedulerTaskIntentTemplate>,
    Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) {
    if node_type != NODE_TYPE_LLM_INFERENCE {
        return (None, None, Vec::new());
    }

    let Some(data) = data else {
        return (
            None,
            None,
            vec![
                diagnostic(
                    node_id,
                    Some(PORT_PUMAS_MODEL_REF),
                    WorkflowSchedulerTaskProjectionDiagnosticCode::MissingPumasModelRef,
                    "inference scheduler tasks require canonical pumas_model_ref input",
                ),
                diagnostic(
                    node_id,
                    Some(PORT_TASK_KIND),
                    WorkflowSchedulerTaskProjectionDiagnosticCode::MissingTaskKind,
                    "inference scheduler tasks require explicit task_kind input",
                ),
            ],
        );
    };

    let mut diagnostics = Vec::new();
    let model_ref =
        optional_materializable_model_ref(node_id, data, input_bindings, &mut diagnostics);
    let task_type = required_task_type(node_id, data, &mut diagnostics);
    let constraints = runtime_device_constraints(node_id, data, &mut diagnostics);
    let trait_settings = trait_settings(node_id, data, &mut diagnostics);
    let estimate_hints = Vec::<SchedulerEstimateHint>::new();

    let schedulable_intent_template = if diagnostics.is_empty() {
        if let (Some(task_type), Some(constraints), Some(trait_settings)) =
            (task_type, constraints, trait_settings)
        {
            Some(WorkflowSchedulerTaskIntentTemplate {
                task_type,
                constraints,
                trait_settings,
                dependency_override_patches: Vec::new(),
                estimate_hints,
            })
        } else {
            None
        }
    } else {
        None
    };

    if let (Some(model_ref), Some(template)) = (model_ref, schedulable_intent_template.as_ref()) {
        if diagnostics.is_empty() {
            return (
                Some(SchedulableTaskIntent {
                    contract_version:
                        pantograph_scheduler::SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
                    workflow_id: workflow_id.clone(),
                    workflow_run_id: workflow_run_id.clone(),
                    node_id: node_id.clone(),
                    task_id: task_id.clone(),
                    fairness_key: None,
                    task_type: template.task_type.clone(),
                    model_ref,
                    constraints: template.constraints.clone(),
                    trait_settings: template.trait_settings.clone(),
                    dependency_override_patches: template.dependency_override_patches.clone(),
                    estimate_hints: template.estimate_hints.clone(),
                }),
                schedulable_intent_template,
                diagnostics,
            );
        }
    }

    (None, schedulable_intent_template, diagnostics)
}

fn optional_materializable_model_ref(
    node_id: &SchedulerNodeId,
    data: &Value,
    input_bindings: &[WorkflowSchedulerTaskInputBinding],
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<PumasModelRef> {
    let Some(value) = data.get(PORT_PUMAS_MODEL_REF) else {
        if !input_bindings
            .iter()
            .any(|binding| binding.target_port_id == PORT_PUMAS_MODEL_REF)
        {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                WorkflowSchedulerTaskProjectionDiagnosticCode::MissingPumasModelRef,
                "inference scheduler tasks require canonical pumas_model_ref input",
            ));
        }
        return None;
    };
    match serde_json::from_value::<PumasModelRef>(value.clone()) {
        Ok(model_ref) => {
            if let Err(error) = model_ref.validate() {
                diagnostics.push(diagnostic(
                    node_id,
                    Some(PORT_PUMAS_MODEL_REF),
                    WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidPumasModelRef,
                    format!("pumas_model_ref is invalid: {error}"),
                ));
                None
            } else {
                Some(model_ref)
            }
        }
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidPumasModelRef,
                format!("pumas_model_ref must match the scheduler PumasModelRef contract: {error}"),
            ));
            None
        }
    }
}

fn required_task_type(
    node_id: &SchedulerNodeId,
    data: &Value,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<DependencyTaskId> {
    let Some(value) = data.get(PORT_TASK_KIND).and_then(Value::as_str) else {
        diagnostics.push(diagnostic(
            node_id,
            Some(PORT_TASK_KIND),
            WorkflowSchedulerTaskProjectionDiagnosticCode::MissingTaskKind,
            "inference scheduler tasks require explicit task_kind input",
        ));
        return None;
    };
    match DependencyTaskId::parse(value) {
        Ok(task_type) => Some(task_type),
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_TASK_KIND),
                WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidTaskKind,
                format!("task_kind is invalid: {error}"),
            ));
            None
        }
    }
}

fn runtime_device_constraints(
    node_id: &SchedulerNodeId,
    data: &Value,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<SchedulerRuntimeDeviceConstraints> {
    let requested_runtime_id = optional_runtime_id(node_id, data, diagnostics)?;
    let requested_device_id = optional_device_id(node_id, data, diagnostics)?;
    Some(SchedulerRuntimeDeviceConstraints {
        requested_runtime_id,
        requested_device_id,
    })
}

fn optional_runtime_id(
    node_id: &SchedulerNodeId,
    data: &Value,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<Option<RuntimeIntentId>> {
    let Some(value) = data.get(PORT_RUNTIME) else {
        return Some(None);
    };
    let Some(value) = value.as_str().filter(|value| !value.trim().is_empty()) else {
        return Some(None);
    };
    match RuntimeIntentId::parse(value) {
        Ok(runtime_id) => Some(Some(runtime_id)),
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_RUNTIME),
                WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidRuntimeRequirement,
                format!("runtime must be a valid scheduler runtime requirement: {error}"),
            ));
            None
        }
    }
}

fn optional_device_id(
    node_id: &SchedulerNodeId,
    data: &Value,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<Option<DeviceIntentId>> {
    let Some(value) = data.get(PORT_DEVICE) else {
        return Some(None);
    };
    let Some(value) = value.as_str().filter(|value| !value.trim().is_empty()) else {
        return Some(None);
    };
    match DeviceIntentId::parse(value) {
        Ok(device_id) => Some(Some(device_id)),
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_DEVICE),
                WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidDeviceRequirement,
                format!("device must be a valid scheduler device requirement: {error}"),
            ));
            None
        }
    }
}

fn trait_settings(
    node_id: &SchedulerNodeId,
    data: &Value,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<Vec<SchedulerTraitSetting>> {
    let mut settings = Vec::new();
    push_optional_trait_setting(
        node_id,
        data,
        PORT_DENOISING_SCHEDULER,
        &mut settings,
        diagnostics,
    )?;
    Some(settings)
}

fn push_optional_trait_setting(
    node_id: &SchedulerNodeId,
    data: &Value,
    key: &'static str,
    settings: &mut Vec<SchedulerTraitSetting>,
    diagnostics: &mut Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
) -> Option<()> {
    let Some(value) = data.get(key) else {
        return Some(());
    };
    if value.is_null() {
        return Some(());
    }
    let trait_id = match SchedulerTraitId::parse(key) {
        Ok(trait_id) => trait_id,
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(key),
                WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidTraitSetting,
                format!("trait id is invalid: {error}"),
            ));
            return None;
        }
    };
    let value = match scheduler_trait_value(value) {
        Ok(value) => value,
        Err(message) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(key),
                WorkflowSchedulerTaskProjectionDiagnosticCode::UnsupportedTraitValue,
                message,
            ));
            return None;
        }
    };
    settings.push(SchedulerTraitSetting { trait_id, value });
    Some(())
}

fn scheduler_trait_value(value: &Value) -> Result<SchedulerTraitValue, String> {
    match value {
        Value::String(value) => Ok(SchedulerTraitValue::String(value.clone())),
        Value::Bool(value) => Ok(SchedulerTraitValue::Bool(*value)),
        Value::Number(number) => {
            if let Some(value) = number.as_u64() {
                Ok(SchedulerTraitValue::U64(value))
            } else if let Some(value) = number.as_i64() {
                Ok(SchedulerTraitValue::I64(value))
            } else {
                Err("scheduler trait values do not yet support floating-point numbers".to_string())
            }
        }
        _ => Err("scheduler trait values must be string, boolean, or integer values".to_string()),
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

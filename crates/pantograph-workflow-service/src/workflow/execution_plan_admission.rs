use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};

use crate::technical_fit::{WorkflowTechnicalFitDecision, WorkflowTechnicalFitDeviceClass};

use super::{
    WorkflowCapabilityModel, WorkflowExecutionPlan, WorkflowExecutionPlanError,
    WorkflowExecutionPlanModelRef, WorkflowExecutionPlanNodeDecision, WorkflowInferenceDeviceClass,
    WorkflowInferenceTaskId,
};

pub(crate) fn build_workflow_execution_plan_from_admission(
    workflow_run_id: &str,
    workflow_id: &str,
    capability_models: &[WorkflowCapabilityModel],
    technical_fit_decision: Option<&WorkflowTechnicalFitDecision>,
) -> Result<Option<WorkflowExecutionPlan>, WorkflowExecutionPlanError> {
    let Some(decision) = technical_fit_decision.map(WorkflowTechnicalFitDecision::normalized)
    else {
        return Ok(None);
    };
    let Some(selected_candidate_id) = decision.selected_candidate_id.as_deref() else {
        return Ok(None);
    };

    let selected_model_id = selected_field(&decision.selected_model_id, "selected_model_id")?;
    let selected_model_ref = selected_model_ref(selected_model_id)?;
    let selected_model = selected_capability_model(capability_models, &selected_model_ref)?;
    let node_id = selected_node_id(selected_model)?;
    let selected_task_id = selected_task_id(selected_model)?;
    let selected_device_class = selected_device_class(decision.selected_device_class)?;
    let selected_backend_key =
        selected_field(&decision.selected_backend_key, "selected_backend_key")?;
    let selected_runtime_id = selected_field(&decision.selected_runtime_id, "selected_runtime_id")?;
    let selected_runtime_variant_id = selected_field(
        &decision.selected_runtime_variant_id,
        "selected_runtime_variant_id",
    )?;

    let mut node_decision = WorkflowExecutionPlanNodeDecision::new(
        node_id,
        selected_backend_key,
        selected_runtime_id,
        selected_runtime_variant_id,
        selected_device_class,
        selected_task_id,
    )?
    .with_selected_model_ref(selected_model_ref.as_str())?
    .with_policy_trace_ids(policy_trace_ids(selected_candidate_id, &decision))?;

    if let Some(selected_device_id) = decision.selected_device_id.as_deref() {
        node_decision = node_decision.with_selected_device_id(selected_device_id)?;
    }

    let workflow_run_id =
        WorkflowRunId::try_from(workflow_run_id.to_string()).map_err(|error| {
            WorkflowExecutionPlanError::InvalidAttributionId {
                field: "workflow_run_id",
                message: error.to_string(),
            }
        })?;
    let workflow_id = WorkflowId::try_from(workflow_id.to_string()).map_err(|error| {
        WorkflowExecutionPlanError::InvalidAttributionId {
            field: "workflow_id",
            message: error.to_string(),
        }
    })?;

    WorkflowExecutionPlan::new(workflow_run_id, workflow_id, vec![node_decision]).map(Some)
}

fn selected_field<'a>(
    value: &'a Option<String>,
    field: &'static str,
) -> Result<&'a str, WorkflowExecutionPlanError> {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(WorkflowExecutionPlanError::MissingSelectedDecisionFact { field })
}

fn selected_capability_model<'a>(
    models: &'a [WorkflowCapabilityModel],
    selected_model_ref: &WorkflowExecutionPlanModelRef,
) -> Result<&'a WorkflowCapabilityModel, WorkflowExecutionPlanError> {
    let matches = models
        .iter()
        .filter(|model| {
            WorkflowExecutionPlanModelRef::parse(model.model_id.as_str())
                .is_ok_and(|model_ref| &model_ref == selected_model_ref)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(WorkflowExecutionPlanError::SelectedModelNotFound {
            model_id: selected_model_ref.as_str().to_string(),
        }),
        [model] => Ok(*model),
        _ => Err(WorkflowExecutionPlanError::AmbiguousSelectedModel {
            model_id: selected_model_ref.as_str().to_string(),
            count: matches.len(),
        }),
    }
}

fn selected_node_id(model: &WorkflowCapabilityModel) -> Result<&str, WorkflowExecutionPlanError> {
    match model.node_ids.as_slice() {
        [node_id] if !node_id.trim().is_empty() => Ok(node_id.as_str()),
        [] => Err(WorkflowExecutionPlanError::MissingSelectedDecisionFact { field: "node_id" }),
        _ => Err(WorkflowExecutionPlanError::AmbiguousNodeMapping {
            model_id: model.model_id.clone(),
            count: model.node_ids.len(),
        }),
    }
}

fn selected_task_id(
    model: &WorkflowCapabilityModel,
) -> Result<WorkflowInferenceTaskId, WorkflowExecutionPlanError> {
    let mut tasks = Vec::new();
    for role in &model.roles {
        if let Some(task) = task_id_from_fact(role) {
            push_unique_task(&mut tasks, task);
        }
    }
    if let Some(model_type) = model.model_type.as_deref().and_then(task_id_from_fact) {
        push_unique_task(&mut tasks, model_type);
    }

    match tasks.len() {
        0 => Err(WorkflowExecutionPlanError::MissingSelectedDecisionFact {
            field: "selected_task_id",
        }),
        1 => Ok(tasks[0]),
        count => Err(WorkflowExecutionPlanError::AmbiguousSelectedTask {
            model_id: model.model_id.clone(),
            count,
        }),
    }
}

fn push_unique_task(tasks: &mut Vec<WorkflowInferenceTaskId>, task: WorkflowInferenceTaskId) {
    if !tasks.contains(&task) {
        tasks.push(task);
    }
}

fn task_id_from_fact(value: &str) -> Option<WorkflowInferenceTaskId> {
    match value.trim().to_ascii_lowercase().as_str() {
        "chat" | "chat_completion" => Some(WorkflowInferenceTaskId::ChatCompletion),
        "diffusion" | "image" | "image_generation" | "text_to_image" => {
            Some(WorkflowInferenceTaskId::ImageGeneration)
        }
        "embedding" | "embeddings" => Some(WorkflowInferenceTaskId::Embedding),
        "image_understanding" | "vision" => Some(WorkflowInferenceTaskId::ImageUnderstanding),
        "rerank" | "reranking" => Some(WorkflowInferenceTaskId::Rerank),
        "text_generation" => Some(WorkflowInferenceTaskId::TextGeneration),
        _ => None,
    }
}

fn selected_device_class(
    selected_device_class: Option<WorkflowTechnicalFitDeviceClass>,
) -> Result<WorkflowInferenceDeviceClass, WorkflowExecutionPlanError> {
    match selected_device_class {
        Some(WorkflowTechnicalFitDeviceClass::Cpu) => Ok(WorkflowInferenceDeviceClass::Cpu),
        Some(WorkflowTechnicalFitDeviceClass::Cuda) => Ok(WorkflowInferenceDeviceClass::Cuda),
        Some(WorkflowTechnicalFitDeviceClass::Metal) => Ok(WorkflowInferenceDeviceClass::Metal),
        Some(WorkflowTechnicalFitDeviceClass::Mps) => Ok(WorkflowInferenceDeviceClass::Mps),
        None => Err(WorkflowExecutionPlanError::MissingSelectedDecisionFact {
            field: "selected_device_class",
        }),
    }
}

fn policy_trace_ids(
    selected_candidate_id: &str,
    decision: &WorkflowTechnicalFitDecision,
) -> Vec<String> {
    let Some(trace) = decision.selection_policy_trace.as_ref() else {
        return vec![selected_candidate_id.to_string()];
    };
    vec![format!("technical_fit_policy_v{}", trace.policy_version)]
}

fn selected_model_ref(
    selected_model_id: &str,
) -> Result<WorkflowExecutionPlanModelRef, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanModelRef::parse(selected_model_id).map_err(|error| {
        WorkflowExecutionPlanError::InvalidSelectedModelRef {
            value: selected_model_id.to_string(),
            message: error.to_string(),
        }
    })
}

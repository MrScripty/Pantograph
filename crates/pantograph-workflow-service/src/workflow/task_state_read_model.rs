use std::collections::{BTreeMap, BTreeSet};

use pantograph_scheduler::{
    SchedulerTaskExecutionIntent, SchedulerTaskId, SchedulerTaskState,
    SchedulerTaskStateDiagnostic, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTraitValue,
};
use serde::{Deserialize, Serialize};

use super::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskProjectionDiagnostic, WorkflowService,
    WorkflowServiceError, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

/// Current schema version for workflow-visible scheduler task state.
pub const WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION: u16 = 1;

/// Presentation-neutral task-state fact for graph editor and run inspection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateReadModel {
    #[serde(default = "default_workflow_scheduler_task_state_read_model_schema_version")]
    pub schema_version: u16,
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub node_id: String,
    pub task_id: String,
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_bindings: Vec<WorkflowSchedulerTaskStateInputBindingReadModel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projection_diagnostics: Vec<WorkflowSchedulerTaskProjectionDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_kind: Option<WorkflowSchedulerTaskStateExecutionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_runtime_task_kind: Option<String>,
    pub state: SchedulerTaskStateKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_settings: Vec<WorkflowSchedulerTaskStateTraitSettingReadModel>,
}

/// Path-free immutable input binding fact joined from the scheduler task graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateInputBindingReadModel {
    pub source_node_id: String,
    pub source_task_id: String,
    pub source_port_id: String,
    pub target_port_id: String,
}

/// Path-free user-facing trait setting supplied to the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateTraitSettingReadModel {
    pub trait_id: String,
    pub value: SchedulerTraitValue,
}

/// Path-free task execution category visible to graph and run inspectors.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskStateExecutionKind {
    Runtime,
    SourceInput,
    NonRuntime,
}

/// Request for active-run scheduler task-state read models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateReadModelQueryRequest {
    pub session_id: String,
    pub workflow_run_id: String,
}

/// Response containing path-free scheduler task-state read models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateReadModelQueryResponse {
    pub session_id: String,
    pub workflow_run_id: String,
    #[serde(default)]
    pub tasks: Vec<WorkflowSchedulerTaskStateReadModel>,
}

impl WorkflowService {
    pub async fn workflow_get_scheduler_task_state_read_models(
        &self,
        request: WorkflowSchedulerTaskStateReadModelQueryRequest,
    ) -> Result<WorkflowSchedulerTaskStateReadModelQueryResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let state = {
            let mut store = self.session_store_guard()?;
            store.touch_session(session_id)?;
            store.active_run_scheduler_task_state(session_id, workflow_run_id)?
        };
        let tasks = match state {
            Some((task_graph, records)) => {
                workflow_scheduler_task_state_read_models(&task_graph, &records)?
            }
            None => Vec::new(),
        };
        Ok(WorkflowSchedulerTaskStateReadModelQueryResponse {
            session_id: session_id.to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            tasks,
        })
    }
}

/// Projects durable scheduler task-state records into presentation-neutral task
/// state joined with immutable task graph facts without exposing transition
/// ids, runtime handoff, or Pumas load targets.
pub fn workflow_scheduler_task_state_read_models(
    task_graph: &WorkflowSchedulerTaskGraph,
    records: &[SchedulerTaskStateRecord],
) -> Result<Vec<WorkflowSchedulerTaskStateReadModel>, WorkflowServiceError> {
    validate_task_graph_for_read_model(task_graph)?;
    let tasks_by_id = task_graph
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str().to_string(), task))
        .collect::<BTreeMap<_, _>>();
    if tasks_by_id.len() != task_graph.tasks.len() {
        return Err(WorkflowServiceError::Internal(
            "scheduler task graph contains duplicate task ids".to_string(),
        ));
    }

    let mut records_by_task_id = BTreeMap::new();
    for record in records {
        record
            .validate()
            .map_err(map_scheduler_task_state_record_error)?;
        validate_record_matches_task_graph(task_graph, record)?;
        let task_id = record.task_id.as_str().to_string();
        if records_by_task_id.insert(task_id.clone(), record).is_some() {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task state read model contains duplicate task-state record '{}'",
                task_id
            )));
        }
    }

    let mut missing_records = Vec::new();
    for task in &task_graph.tasks {
        if !records_by_task_id.contains_key(task.task_id.as_str()) {
            missing_records.push(task.task_id.as_str().to_string());
        }
    }
    if !missing_records.is_empty() {
        return Err(WorkflowServiceError::Internal(format!(
            "scheduler task state read model is missing task-state records for tasks: {}",
            missing_records.join(", ")
        )));
    }

    let extra_records = records_by_task_id
        .keys()
        .filter(|task_id| !tasks_by_id.contains_key(*task_id))
        .cloned()
        .collect::<Vec<_>>();
    if !extra_records.is_empty() {
        return Err(WorkflowServiceError::Internal(format!(
            "scheduler task state read model contains records outside the task graph: {}",
            extra_records.join(", ")
        )));
    }

    let mut read_models = Vec::with_capacity(task_graph.tasks.len());
    for task in &task_graph.tasks {
        let record = records_by_task_id
            .get(task.task_id.as_str())
            .expect("record existence checked above");
        read_models.push(read_model_from_record(task_graph, task, record));
    }
    Ok(read_models)
}

fn read_model_from_record(
    task_graph: &WorkflowSchedulerTaskGraph,
    task: &WorkflowSchedulerTask,
    record: &SchedulerTaskStateRecord,
) -> WorkflowSchedulerTaskStateReadModel {
    WorkflowSchedulerTaskStateReadModel {
        schema_version: WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION,
        workflow_id: task_graph.workflow_id.as_str().to_string(),
        workflow_run_id: task_graph.workflow_run_id.as_str().to_string(),
        node_id: task.node_id.as_str().to_string(),
        task_id: task.task_id.as_str().to_string(),
        node_type: task.node_type.clone(),
        dependency_task_ids: task
            .dependency_task_ids
            .iter()
            .map(|task_id| task_id.as_str().to_string())
            .collect(),
        input_bindings: task
            .input_bindings
            .iter()
            .map(input_binding_read_model)
            .collect(),
        projection_diagnostics: task.diagnostics.clone(),
        state_diagnostics: scheduler_state_diagnostics(&record.state),
        execution_kind: execution_kind_read_model(task, record.state.execution_intent()),
        task_type: record
            .state
            .task_intent()
            .map(|intent| intent.task_type.as_str().to_string()),
        model_id: record
            .state
            .task_intent()
            .map(|intent| intent.model_ref.model_id.clone()),
        non_runtime_task_kind: record
            .state
            .execution_intent()
            .and_then(SchedulerTaskExecutionIntent::non_runtime_task_intent)
            .map(|intent| intent.task_kind.as_str().to_string()),
        state: record.state.kind(),
        requested_runtime_id: record.state.task_intent().and_then(|intent| {
            intent
                .constraints
                .requested_runtime_id
                .as_ref()
                .map(ToString::to_string)
        }),
        requested_device_id: record.state.task_intent().and_then(|intent| {
            intent
                .constraints
                .requested_device_id
                .as_ref()
                .map(ToString::to_string)
        }),
        trait_settings: record
            .state
            .task_intent()
            .map(|intent| {
                intent
                    .trait_settings
                    .iter()
                    .map(|setting| WorkflowSchedulerTaskStateTraitSettingReadModel {
                        trait_id: setting.trait_id.as_str().to_string(),
                        value: setting.value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn scheduler_state_diagnostics(state: &SchedulerTaskState) -> Vec<SchedulerTaskStateDiagnostic> {
    match state {
        SchedulerTaskState::AwaitingInputs { diagnostics }
        | SchedulerTaskState::InputUnavailable { diagnostics }
        | SchedulerTaskState::Invalid { diagnostics }
        | SchedulerTaskState::PausedDeferred { diagnostics, .. }
        | SchedulerTaskState::RetryableFailed { diagnostics, .. }
        | SchedulerTaskState::TerminalFailed { diagnostics } => diagnostics.clone(),
        SchedulerTaskState::Ready { .. }
        | SchedulerTaskState::WaitingDependencyReadiness { .. }
        | SchedulerTaskState::WaitingResources { .. }
        | SchedulerTaskState::WaitingBatch { .. }
        | SchedulerTaskState::Running { .. }
        | SchedulerTaskState::Completed { .. } => Vec::new(),
        _ => Vec::new(),
    }
}

fn execution_kind_read_model(
    task: &WorkflowSchedulerTask,
    execution_intent: Option<&SchedulerTaskExecutionIntent>,
) -> Option<WorkflowSchedulerTaskStateExecutionKind> {
    if task.execution_class == WorkflowSchedulerTaskExecutionClass::SourceInput {
        return Some(WorkflowSchedulerTaskStateExecutionKind::SourceInput);
    }

    match execution_intent {
        Some(SchedulerTaskExecutionIntent::Runtime { .. }) => {
            Some(WorkflowSchedulerTaskStateExecutionKind::Runtime)
        }
        Some(SchedulerTaskExecutionIntent::NonRuntime { .. }) => {
            Some(WorkflowSchedulerTaskStateExecutionKind::NonRuntime)
        }
        Some(_) | None => None,
    }
}

fn validate_task_graph_for_read_model(
    task_graph: &WorkflowSchedulerTaskGraph,
) -> Result<(), WorkflowServiceError> {
    if task_graph.schema_version != WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION {
        return Err(WorkflowServiceError::Internal(format!(
            "unsupported scheduler task graph schema version {}",
            task_graph.schema_version
        )));
    }
    let mut seen_task_ids = BTreeSet::<&SchedulerTaskId>::new();
    for task in &task_graph.tasks {
        if task.workflow_id != task_graph.workflow_id {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task '{}' workflow id does not match task graph",
                task.task_id.as_str()
            )));
        }
        if task.workflow_run_id != task_graph.workflow_run_id {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task '{}' workflow run id does not match task graph",
                task.task_id.as_str()
            )));
        }
        if !seen_task_ids.insert(&task.task_id) {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task graph contains duplicate task '{}'",
                task.task_id.as_str()
            )));
        }
    }
    Ok(())
}

fn validate_record_matches_task_graph(
    task_graph: &WorkflowSchedulerTaskGraph,
    record: &SchedulerTaskStateRecord,
) -> Result<(), WorkflowServiceError> {
    if record.workflow_id != task_graph.workflow_id {
        return Err(WorkflowServiceError::Internal(format!(
            "scheduler task-state record '{}' workflow id does not match task graph",
            record.task_id.as_str()
        )));
    }
    if record.workflow_run_id != task_graph.workflow_run_id {
        return Err(WorkflowServiceError::Internal(format!(
            "scheduler task-state record '{}' workflow run id does not match task graph",
            record.task_id.as_str()
        )));
    }
    Ok(())
}

fn input_binding_read_model(
    binding: &WorkflowSchedulerTaskInputBinding,
) -> WorkflowSchedulerTaskStateInputBindingReadModel {
    WorkflowSchedulerTaskStateInputBindingReadModel {
        source_node_id: binding.source_node_id.as_str().to_string(),
        source_task_id: binding.source_task_id.as_str().to_string(),
        source_port_id: binding.source_port_id.clone(),
        target_port_id: binding.target_port_id.clone(),
    }
}

fn default_workflow_scheduler_task_state_read_model_schema_version() -> u16 {
    WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION
}

fn map_scheduler_task_state_record_error(
    error: pantograph_scheduler::SchedulerContractError,
) -> WorkflowServiceError {
    WorkflowServiceError::Internal(format!("invalid scheduler task-state record: {error}"))
}

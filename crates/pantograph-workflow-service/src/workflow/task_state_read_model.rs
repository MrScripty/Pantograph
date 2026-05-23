use std::collections::BTreeSet;

use pantograph_scheduler::{
    SchedulerQueueTaskRecord, SchedulerQueueTaskState, SchedulerTraitValue,
};
use serde::{Deserialize, Serialize};

use super::WorkflowServiceError;

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
    pub task_type: String,
    pub model_id: String,
    pub state: SchedulerQueueTaskState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_settings: Vec<WorkflowSchedulerTaskStateTraitSettingReadModel>,
}

/// Path-free user-facing trait setting supplied to the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskStateTraitSettingReadModel {
    pub trait_id: String,
    pub value: SchedulerTraitValue,
}

/// Projects durable scheduler queue records into presentation-neutral task
/// state without exposing transition ids, runtime handoff, or Pumas load
/// targets.
pub fn workflow_scheduler_task_state_read_models(
    records: &[SchedulerQueueTaskRecord],
) -> Result<Vec<WorkflowSchedulerTaskStateReadModel>, WorkflowServiceError> {
    let mut seen_task_ids = BTreeSet::new();
    let mut read_models = Vec::with_capacity(records.len());
    for record in records {
        record
            .validate()
            .map_err(map_scheduler_queue_record_error)?;
        let task_id = record.task_id.as_str().to_string();
        if !seen_task_ids.insert(task_id.clone()) {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task state read model contains duplicate task '{}'",
                task_id
            )));
        }
        read_models.push(read_model_from_record(record));
    }
    read_models.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    Ok(read_models)
}

fn read_model_from_record(
    record: &SchedulerQueueTaskRecord,
) -> WorkflowSchedulerTaskStateReadModel {
    WorkflowSchedulerTaskStateReadModel {
        schema_version: WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION,
        workflow_id: record.workflow_id.as_str().to_string(),
        workflow_run_id: record.workflow_run_id.as_str().to_string(),
        node_id: record.node_id.as_str().to_string(),
        task_id: record.task_id.as_str().to_string(),
        task_type: record.task_intent.task_type.as_str().to_string(),
        model_id: record.task_intent.model_ref.model_id.clone(),
        state: record.state,
        requested_runtime_id: record
            .task_intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(ToString::to_string),
        requested_device_id: record
            .task_intent
            .constraints
            .requested_device_id
            .as_ref()
            .map(ToString::to_string),
        trait_settings: record
            .task_intent
            .trait_settings
            .iter()
            .map(|setting| WorkflowSchedulerTaskStateTraitSettingReadModel {
                trait_id: setting.trait_id.as_str().to_string(),
                value: setting.value.clone(),
            })
            .collect(),
    }
}

fn default_workflow_scheduler_task_state_read_model_schema_version() -> u16 {
    WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION
}

fn map_scheduler_queue_record_error(
    error: pantograph_scheduler::SchedulerContractError,
) -> WorkflowServiceError {
    WorkflowServiceError::Internal(format!("invalid scheduler task record: {error}"))
}

use std::collections::BTreeSet;

use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord, SchedulerTraitValue};
use serde::{Deserialize, Serialize};

use super::{WorkflowService, WorkflowServiceError};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub state: SchedulerTaskStateKind,
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

        let records = {
            let mut store = self.session_store_guard()?;
            store.touch_session(session_id)?;
            store.active_run_scheduler_task_records(session_id, workflow_run_id)?
        };
        let tasks = workflow_scheduler_task_state_read_models(&records)?;
        Ok(WorkflowSchedulerTaskStateReadModelQueryResponse {
            session_id: session_id.to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            tasks,
        })
    }
}

/// Projects durable scheduler task-state records into presentation-neutral task
/// state without exposing transition ids, runtime handoff, or Pumas load
/// targets.
pub fn workflow_scheduler_task_state_read_models(
    records: &[SchedulerTaskStateRecord],
) -> Result<Vec<WorkflowSchedulerTaskStateReadModel>, WorkflowServiceError> {
    let mut seen_task_ids = BTreeSet::new();
    let mut read_models = Vec::with_capacity(records.len());
    for record in records {
        record
            .validate()
            .map_err(map_scheduler_task_state_record_error)?;
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
    record: &SchedulerTaskStateRecord,
) -> WorkflowSchedulerTaskStateReadModel {
    WorkflowSchedulerTaskStateReadModel {
        schema_version: WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION,
        workflow_id: record.workflow_id.as_str().to_string(),
        workflow_run_id: record.workflow_run_id.as_str().to_string(),
        node_id: record.node_id.as_str().to_string(),
        task_id: record.task_id.as_str().to_string(),
        task_type: record
            .state
            .task_intent()
            .map(|intent| intent.task_type.as_str().to_string()),
        model_id: record
            .state
            .task_intent()
            .map(|intent| intent.model_ref.model_id.clone()),
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

fn default_workflow_scheduler_task_state_read_model_schema_version() -> u16 {
    WORKFLOW_SCHEDULER_TASK_STATE_READ_MODEL_SCHEMA_VERSION
}

fn map_scheduler_task_state_record_error(
    error: pantograph_scheduler::SchedulerContractError,
) -> WorkflowServiceError {
    WorkflowServiceError::Internal(format!("invalid scheduler task-state record: {error}"))
}

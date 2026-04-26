use pantograph_workflow_service::WorkflowServiceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugSnapshotRequest {
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub include_trace: Option<bool>,
    #[serde(default)]
    pub include_completed: Option<bool>,
}

impl RuntimeDebugSnapshotRequest {
    pub(crate) fn normalized(&self) -> Self {
        Self {
            workflow_run_id: normalize_optional_filter(&self.workflow_run_id),
            session_id: normalize_optional_filter(&self.session_id),
            workflow_id: normalize_optional_filter(&self.workflow_id),
            include_trace: self.include_trace,
            include_completed: self.include_completed,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), WorkflowServiceError> {
        validate_optional_filter(&self.workflow_run_id, "workflow_run_id")?;
        validate_optional_filter(&self.session_id, "session_id")?;
        validate_optional_filter(&self.workflow_id, "workflow_id")?;
        Ok(())
    }
}

fn normalize_optional_filter(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).map(ToOwned::to_owned)
}

fn validate_optional_filter(
    value: &Option<String>,
    field_name: &'static str,
) -> Result<(), WorkflowServiceError> {
    if let Some(value) = value {
        if value.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "runtime debug snapshot request field '{}' must not be blank",
                field_name
            )));
        }
    }

    Ok(())
}

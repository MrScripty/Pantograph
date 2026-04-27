use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, DiagnosticsQuery, DiagnosticsRetentionPolicy,
    ExecutionGuaranteeLevel, IoArtifactProjectionQuery, IoArtifactProjectionRecord,
    LibraryUsageProjectionQuery, LibraryUsageProjectionRecord, ModelLicenseUsageEvent,
    ProjectionStateRecord, RunDetailProjectionQuery, RunDetailProjectionRecord,
    RunListProjectionQuery, RunListProjectionRecord, RunListProjectionStatus,
    SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
};
use serde::{Deserialize, Serialize};

use super::{WorkflowService, WorkflowServiceError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_contract_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guarantee_level: Option<ExecutionGuaranteeLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_before_ms: Option<i64>,
    #[serde(default)]
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageQueryResponse {
    pub events: Vec<ModelLicenseUsageEvent>,
    pub summaries: Vec<WorkflowDiagnosticsUsageSummary>,
    pub retention_policy: DiagnosticsRetentionPolicy,
    pub page: u32,
    pub page_size: u32,
    pub may_have_pruned_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsUsageSummary {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_value: Option<String>,
    pub guarantee_level: ExecutionGuaranteeLevel,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerTimelineQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerTimelineQueryResponse {
    pub events: Vec<SchedulerTimelineProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunListQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RunListProjectionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunListQueryResponse {
    pub runs: Vec<RunListProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunDetailQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunDetailQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunDetailProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoArtifactQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoArtifactQueryResponse {
    pub artifacts: Vec<IoArtifactProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryUsageQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryUsageQueryResponse {
    pub assets: Vec<LibraryUsageProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowProjectionRebuildRequest {
    pub projection_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowProjectionRebuildResponse {
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyQueryRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyQueryResponse {
    pub retention_policy: DiagnosticsRetentionPolicy,
}

impl WorkflowService {
    pub fn workflow_diagnostics_usage_query(
        &self,
        request: WorkflowDiagnosticsUsageQueryRequest,
    ) -> Result<WorkflowDiagnosticsUsageQueryResponse, WorkflowServiceError> {
        let query = request.into_query()?;
        let ledger = self.diagnostics_ledger_guard()?;
        let projection = ledger
            .query_usage_events(query)
            .map_err(WorkflowServiceError::from)?;
        let retention_policy = ledger
            .retention_policy()
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowDiagnosticsUsageQueryResponse {
            summaries: summarize_usage(&projection.events),
            events: projection.events,
            retention_policy,
            page: projection.page,
            page_size: projection.page_size,
            may_have_pruned_usage: projection.may_have_pruned_usage,
        })
    }

    pub fn workflow_scheduler_timeline_query(
        &self,
        request: WorkflowSchedulerTimelineQueryRequest,
    ) -> Result<WorkflowSchedulerTimelineQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_scheduler_timeline_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_scheduler_timeline_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let events = ledger
            .query_scheduler_timeline_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowSchedulerTimelineQueryResponse {
            events,
            projection_state,
        })
    }

    pub fn workflow_run_list_query(
        &self,
        request: WorkflowRunListQueryRequest,
    ) -> Result<WorkflowRunListQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_run_list_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_run_list_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let runs = ledger
            .query_run_list_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRunListQueryResponse {
            runs,
            projection_state,
        })
    }

    pub fn workflow_run_detail_query(
        &self,
        request: WorkflowRunDetailQueryRequest,
    ) -> Result<WorkflowRunDetailQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_run_detail_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_run_detail_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let run = ledger
            .query_run_detail_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRunDetailQueryResponse {
            run,
            projection_state,
        })
    }

    pub fn workflow_io_artifact_query(
        &self,
        request: WorkflowIoArtifactQueryRequest,
    ) -> Result<WorkflowIoArtifactQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_io_artifact_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_io_artifact_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let artifacts = ledger
            .query_io_artifact_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowIoArtifactQueryResponse {
            artifacts,
            projection_state,
        })
    }

    pub fn workflow_library_usage_query(
        &self,
        request: WorkflowLibraryUsageQueryRequest,
    ) -> Result<WorkflowLibraryUsageQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_library_usage_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_library_usage_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let assets = ledger
            .query_library_usage_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowLibraryUsageQueryResponse {
            assets,
            projection_state,
        })
    }

    pub fn workflow_projection_rebuild(
        &self,
        request: WorkflowProjectionRebuildRequest,
    ) -> Result<WorkflowProjectionRebuildResponse, WorkflowServiceError> {
        let batch_size = request.batch_size.unwrap_or(500).max(1);
        if batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "batch_size exceeds maximum 500".to_string(),
            ));
        }
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .rebuild_projection(&request.projection_name, batch_size)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowProjectionRebuildResponse { projection_state })
    }

    pub fn workflow_retention_policy_query(
        &self,
        _request: WorkflowRetentionPolicyQueryRequest,
    ) -> Result<WorkflowRetentionPolicyQueryResponse, WorkflowServiceError> {
        let ledger = self.diagnostics_ledger_guard()?;
        let retention_policy = ledger
            .retention_policy()
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionPolicyQueryResponse { retention_policy })
    }
}

impl WorkflowDiagnosticsUsageQueryRequest {
    fn into_query(self) -> Result<DiagnosticsQuery, WorkflowServiceError> {
        let query = DiagnosticsQuery {
            client_id: parse_optional_id("client_id", self.client_id)?,
            client_session_id: parse_optional_id("client_session_id", self.client_session_id)?,
            bucket_id: parse_optional_id("bucket_id", self.bucket_id)?,
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            workflow_semantic_version: self.workflow_semantic_version,
            node_id: self.node_id,
            node_contract_version: self.node_contract_version,
            node_contract_digest: self.node_contract_digest,
            model_id: self.model_id,
            license_value: self.license_value,
            guarantee_level: self.guarantee_level,
            started_at_ms: self.started_at_ms,
            ended_before_ms: self.ended_before_ms,
            page: self.page,
            page_size: self
                .page_size
                .unwrap_or_else(|| DiagnosticsQuery::default().page_size)
                .max(1),
        };
        query.validate().map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

impl WorkflowSchedulerTimelineQueryRequest {
    fn into_scheduler_timeline_query(
        self,
    ) -> Result<SchedulerTimelineProjectionQuery, WorkflowServiceError> {
        Ok(SchedulerTimelineProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            scheduler_policy_id: self.scheduler_policy_id,
            after_event_seq: self.after_event_seq,
            limit: self
                .limit
                .unwrap_or_else(|| SchedulerTimelineProjectionQuery::default().limit)
                .max(1),
        })
    }
}

impl WorkflowRunListQueryRequest {
    fn into_run_list_query(self) -> Result<RunListProjectionQuery, WorkflowServiceError> {
        Ok(RunListProjectionQuery {
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            workflow_semantic_version: self.workflow_semantic_version,
            status: self.status,
            scheduler_policy_id: self.scheduler_policy_id,
            after_event_seq: self.after_event_seq,
            limit: self
                .limit
                .unwrap_or_else(|| RunListProjectionQuery::default().limit)
                .max(1),
        })
    }
}

impl WorkflowRunDetailQueryRequest {
    fn into_run_detail_query(self) -> Result<RunDetailProjectionQuery, WorkflowServiceError> {
        Ok(RunDetailProjectionQuery {
            workflow_run_id: parse_id("workflow_run_id", self.workflow_run_id)?,
        })
    }
}

impl WorkflowIoArtifactQueryRequest {
    fn into_io_artifact_query(self) -> Result<IoArtifactProjectionQuery, WorkflowServiceError> {
        let query = IoArtifactProjectionQuery {
            workflow_run_id: parse_id("workflow_run_id", self.workflow_run_id)?,
            node_id: self.node_id,
            artifact_role: self.artifact_role,
            media_type: self.media_type,
            retention_policy_id: self.retention_policy_id,
            runtime_id: self.runtime_id,
            model_id: self.model_id,
            after_event_seq: self.after_event_seq,
            limit: self.limit.unwrap_or(100).max(1),
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

impl WorkflowLibraryUsageQueryRequest {
    fn into_library_usage_query(self) -> Result<LibraryUsageProjectionQuery, WorkflowServiceError> {
        let query = LibraryUsageProjectionQuery {
            asset_id: self.asset_id,
            workflow_id: parse_optional_id("workflow_id", self.workflow_id)?,
            workflow_version_id: parse_optional_id(
                "workflow_version_id",
                self.workflow_version_id,
            )?,
            after_event_seq: self.after_event_seq,
            limit: self.limit.unwrap_or(100).max(1),
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

fn parse_id<T>(field: &'static str, value: String) -> Result<T, WorkflowServiceError>
where
    T: TryFrom<String>,
    T::Error: std::fmt::Display,
{
    T::try_from(value).map_err(|error| {
        WorkflowServiceError::InvalidRequest(format!("invalid {}: {}", field, error))
    })
}

fn parse_optional_id<T>(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<T>, WorkflowServiceError>
where
    T: TryFrom<String>,
    T::Error: std::fmt::Display,
{
    value
        .map(|value| {
            T::try_from(value).map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!("invalid {}: {}", field, error))
            })
        })
        .transpose()
}

fn summarize_usage(events: &[ModelLicenseUsageEvent]) -> Vec<WorkflowDiagnosticsUsageSummary> {
    let mut summaries = Vec::<WorkflowDiagnosticsUsageSummary>::new();
    for event in events {
        if let Some(summary) = summaries.iter_mut().find(|summary| {
            summary.model_id == event.model.model_id
                && summary.license_value == event.license_snapshot.license_value
                && summary.guarantee_level == event.guarantee_level
        }) {
            summary.event_count += 1;
        } else {
            summaries.push(WorkflowDiagnosticsUsageSummary {
                model_id: event.model.model_id.clone(),
                license_value: event.license_snapshot.license_value.clone(),
                guarantee_level: event.guarantee_level,
                event_count: 1,
            });
        }
    }

    summaries
}

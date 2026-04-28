use pantograph_diagnostics_ledger::{
    ApplyArtifactRetentionPolicyCommand, ApplyArtifactRetentionPolicyResult,
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    DiagnosticsQuery, DiagnosticsRetentionPolicy, ExecutionGuaranteeLevel,
    IoArtifactProjectionQuery, IoArtifactProjectionRecord, IoArtifactRetentionState,
    IoArtifactRetentionSummaryQuery, IoArtifactRetentionSummaryRecord, LibraryAssetAccessedPayload,
    LibraryAssetCacheStatus, LibraryAssetOperation, LibraryUsageProjectionQuery,
    LibraryUsageProjectionRecord, ModelLicenseUsageEvent, NodeExecutionProjectionStatus,
    NodeStatusProjectionQuery, NodeStatusProjectionRecord, ProjectionStateRecord, RetentionClass,
    RetentionPolicyActorScope, RetentionPolicyChangedPayload, RunDetailProjectionQuery,
    RunDetailProjectionRecord, RunListFacetRecord, RunListProjectionQuery, RunListProjectionRecord,
    RunListProjectionStatus, SchedulerTimelineProjectionQuery, SchedulerTimelineProjectionRecord,
    UpdateRetentionPolicyCommand,
};
use serde::{Deserialize, Serialize};

use crate::scheduler::unix_timestamp_ms;

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
    pub retention_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at_from_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at_to_ms: Option<i64>,
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
    pub facets: Vec<RunListFacetRecord>,
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
pub struct WorkflowSchedulerEstimateQueryRequest {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerEstimateQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<WorkflowSchedulerEstimateRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerEstimateRecord {
    pub workflow_run_id: String,
    pub workflow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_semantic_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_estimate_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_queue_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoArtifactQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_state: Option<IoArtifactRetentionState>,
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
    pub retention_summary: Vec<IoArtifactRetentionSummaryRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowNodeStatusQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<NodeExecutionProjectionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_event_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowNodeStatusQueryResponse {
    pub nodes: Vec<NodeStatusProjectionRecord>,
    pub projection_state: ProjectionStateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryUsageQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
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
pub struct WorkflowLibraryAssetAccessRecordRequest {
    pub asset_id: String,
    pub operation: LibraryAssetOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_status: Option<LibraryAssetCacheStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLibraryAssetAccessRecordResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_seq: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyUpdateRequest {
    pub retention_days: u32,
    pub explanation: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionPolicyUpdateResponse {
    pub retention_policy: DiagnosticsRetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionCleanupRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRetentionCleanupResponse {
    pub cleanup: ApplyArtifactRetentionPolicyResult,
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
        let facet_query = query.clone();
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_run_list_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let runs = ledger
            .query_run_list_projection(query)
            .map_err(WorkflowServiceError::from)?;
        let facets = ledger
            .query_run_list_facets(facet_query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRunListQueryResponse {
            runs,
            facets,
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

    pub fn workflow_scheduler_estimate_query(
        &self,
        request: WorkflowSchedulerEstimateQueryRequest,
    ) -> Result<WorkflowSchedulerEstimateQueryResponse, WorkflowServiceError> {
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
        let estimate = ledger
            .query_run_detail_projection(query)
            .map_err(WorkflowServiceError::from)?
            .map(WorkflowSchedulerEstimateRecord::from);

        Ok(WorkflowSchedulerEstimateQueryResponse {
            estimate,
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
        let summary_query = io_artifact_retention_summary_query(&query);
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_io_artifact_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let artifacts = ledger
            .query_io_artifact_projection(query)
            .map_err(WorkflowServiceError::from)?;
        let retention_summary = ledger
            .query_io_artifact_retention_summary(summary_query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowIoArtifactQueryResponse {
            artifacts,
            retention_summary,
            projection_state,
        })
    }

    pub fn workflow_node_status_query(
        &self,
        request: WorkflowNodeStatusQueryRequest,
    ) -> Result<WorkflowNodeStatusQueryResponse, WorkflowServiceError> {
        let projection_batch_size = request.projection_batch_size.unwrap_or(500).max(1);
        if projection_batch_size > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "projection_batch_size exceeds maximum 500".to_string(),
            ));
        }
        let query = request.into_node_status_query()?;
        let mut ledger = self.diagnostics_ledger_guard()?;
        let projection_state = ledger
            .drain_node_status_projection(projection_batch_size)
            .map_err(WorkflowServiceError::from)?;
        let nodes = ledger
            .query_node_status_projection(query)
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowNodeStatusQueryResponse {
            nodes,
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

    pub fn workflow_library_asset_access_record(
        &self,
        request: WorkflowLibraryAssetAccessRecordRequest,
    ) -> Result<WorkflowLibraryAssetAccessRecordResponse, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(WorkflowLibraryAssetAccessRecordResponse { event_seq: None });
        };
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        let event = ledger
            .append_diagnostic_event(DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Library,
                source_instance_id: request
                    .source_instance_id
                    .or_else(|| Some("workflow-library-audit".to_string())),
                occurred_at_ms: unix_timestamp_ms() as i64,
                workflow_run_id: None,
                workflow_id: None,
                workflow_version_id: None,
                workflow_semantic_version: None,
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                scheduler_policy_id: None,
                retention_policy_id: None,
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::LibraryAssetAccessed(
                    LibraryAssetAccessedPayload {
                        asset_id: request.asset_id,
                        operation: request.operation,
                        cache_status: request.cache_status,
                        network_bytes: request.network_bytes,
                    },
                ),
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowLibraryAssetAccessRecordResponse {
            event_seq: Some(event.event_seq),
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

    pub fn workflow_retention_policy_update(
        &self,
        request: WorkflowRetentionPolicyUpdateRequest,
    ) -> Result<WorkflowRetentionPolicyUpdateResponse, WorkflowServiceError> {
        if request.reason.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "reason must be non-empty".to_string(),
            ));
        }

        let mut ledger = self.diagnostics_ledger_guard()?;
        let retention_policy = ledger
            .update_retention_policy(UpdateRetentionPolicyCommand {
                retention_class: RetentionClass::Standard,
                retention_days: request.retention_days,
                explanation: request.explanation,
            })
            .map_err(WorkflowServiceError::from)?;
        ledger
            .append_diagnostic_event(DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Retention,
                source_instance_id: Some("workflow-retention-policy".to_string()),
                occurred_at_ms: unix_timestamp_ms() as i64,
                workflow_run_id: None,
                workflow_id: None,
                workflow_version_id: None,
                workflow_semantic_version: None,
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                scheduler_policy_id: None,
                retention_policy_id: Some(retention_policy.policy_id.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RetentionPolicyChanged(
                    RetentionPolicyChangedPayload {
                        policy_id: retention_policy.policy_id.clone(),
                        policy_version: retention_policy.policy_version,
                        retention_days: retention_policy.retention_days,
                        actor_scope: RetentionPolicyActorScope::GuiAdmin,
                        reason: request.reason,
                    },
                ),
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionPolicyUpdateResponse { retention_policy })
    }

    pub fn workflow_retention_cleanup_apply(
        &self,
        request: WorkflowRetentionCleanupRequest,
    ) -> Result<WorkflowRetentionCleanupResponse, WorkflowServiceError> {
        if request.reason.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "reason must be non-empty".to_string(),
            ));
        }
        let limit = request.limit.unwrap_or(500).max(1);
        if limit > 500 {
            return Err(WorkflowServiceError::InvalidRequest(
                "limit exceeds maximum 500".to_string(),
            ));
        }

        let mut ledger = self.diagnostics_ledger_guard()?;
        let cleanup = ledger
            .apply_artifact_retention_policy(ApplyArtifactRetentionPolicyCommand {
                retention_class: RetentionClass::Standard,
                now_ms: unix_timestamp_ms() as i64,
                limit,
                reason: request.reason,
            })
            .map_err(WorkflowServiceError::from)?;

        Ok(WorkflowRetentionCleanupResponse { cleanup })
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
            retention_policy_id: self.retention_policy_id,
            client_id: parse_optional_id("client_id", self.client_id)?,
            client_session_id: parse_optional_id("client_session_id", self.client_session_id)?,
            bucket_id: parse_optional_id("bucket_id", self.bucket_id)?,
            accepted_at_from_ms: self.accepted_at_from_ms,
            accepted_at_to_ms: self.accepted_at_to_ms,
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

impl WorkflowSchedulerEstimateQueryRequest {
    fn into_run_detail_query(self) -> Result<RunDetailProjectionQuery, WorkflowServiceError> {
        Ok(RunDetailProjectionQuery {
            workflow_run_id: parse_id("workflow_run_id", self.workflow_run_id)?,
        })
    }
}

impl From<RunDetailProjectionRecord> for WorkflowSchedulerEstimateRecord {
    fn from(run: RunDetailProjectionRecord) -> Self {
        Self {
            workflow_run_id: run.workflow_run_id.to_string(),
            workflow_id: run.workflow_id.to_string(),
            workflow_version_id: run.workflow_version_id.map(|value| value.to_string()),
            workflow_semantic_version: run.workflow_semantic_version,
            scheduler_policy_id: run.scheduler_policy_id,
            latest_estimate_json: run.latest_estimate_json,
            estimate_confidence: run.estimate_confidence,
            estimated_queue_wait_ms: run.estimated_queue_wait_ms,
            estimated_duration_ms: run.estimated_duration_ms,
            last_event_seq: run.last_event_seq,
            last_updated_at_ms: run.last_updated_at_ms,
        }
    }
}

impl WorkflowIoArtifactQueryRequest {
    fn into_io_artifact_query(self) -> Result<IoArtifactProjectionQuery, WorkflowServiceError> {
        let query = IoArtifactProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            node_id: self.node_id,
            artifact_role: self.artifact_role,
            media_type: self.media_type,
            retention_state: self.retention_state,
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

fn io_artifact_retention_summary_query(
    query: &IoArtifactProjectionQuery,
) -> IoArtifactRetentionSummaryQuery {
    IoArtifactRetentionSummaryQuery {
        workflow_run_id: query.workflow_run_id.clone(),
        node_id: query.node_id.clone(),
        artifact_role: query.artifact_role.clone(),
        media_type: query.media_type.clone(),
        retention_policy_id: query.retention_policy_id.clone(),
        runtime_id: query.runtime_id.clone(),
        model_id: query.model_id.clone(),
    }
}

impl WorkflowNodeStatusQueryRequest {
    fn into_node_status_query(self) -> Result<NodeStatusProjectionQuery, WorkflowServiceError> {
        let query = NodeStatusProjectionQuery {
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
            node_id: self.node_id,
            status: self.status,
            after_event_seq: self.after_event_seq,
            limit: self.limit.unwrap_or(250).max(1),
        };
        query.validate(500).map_err(WorkflowServiceError::from)?;
        Ok(query)
    }
}

impl WorkflowLibraryUsageQueryRequest {
    fn into_library_usage_query(self) -> Result<LibraryUsageProjectionQuery, WorkflowServiceError> {
        let query = LibraryUsageProjectionQuery {
            asset_id: self.asset_id,
            workflow_run_id: parse_optional_id("workflow_run_id", self.workflow_run_id)?,
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

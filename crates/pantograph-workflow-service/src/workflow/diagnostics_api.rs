use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, DiagnosticsQuery, DiagnosticsRetentionPolicy,
    ExecutionGuaranteeLevel, ModelLicenseUsageEvent,
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

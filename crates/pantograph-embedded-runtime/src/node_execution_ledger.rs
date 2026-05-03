use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerError,
    DiagnosticsLedgerRepository, ExecutionGuaranteeLevel, LicenseSnapshot, ModelIdentity,
    ModelLicenseUsageEvent, ModelOutputMeasurement, NodeExecutionProjectionStatus,
    NodeExecutionStatusPayload, RetentionClass, UsageEventStatus, UsageLineage,
};
use pantograph_runtime_attribution::UsageEventId;
use thiserror::Error;

use crate::{
    ManagedCapabilityKind, ModelExecutionCapability, NodeExecutionContext, NodeExecutionGuarantee,
};

#[derive(Debug, Error)]
pub enum RuntimeLedgerSubmissionError {
    #[error("model execution capability route does not match node execution context")]
    ContextMismatch,
    #[error("model execution capability is unavailable")]
    CapabilityUnavailable,
    #[error("model usage completed before it started")]
    InvalidTimeRange,
    #[error("diagnostics ledger submission failed: {0}")]
    Ledger(#[from] DiagnosticsLedgerError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedModelUsageSubmission {
    pub model: ModelIdentity,
    pub license_snapshot: LicenseSnapshot,
    pub output_measurement: ModelOutputMeasurement,
    pub status: UsageEventStatus,
    pub started_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub output_port_ids: Vec<String>,
    pub correlation_id: Option<String>,
    pub retention_class: RetentionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedModelUsageEvent {
    pub event: ModelLicenseUsageEvent,
}

pub fn inference_lifecycle_event_ledger_append_request(
    context: &NodeExecutionContext,
    event: &inference::InferenceRequestLifecycleEvent,
) -> Option<DiagnosticEventAppendRequest> {
    let status = match event.kind {
        inference::InferenceRequestLifecycleEventKind::Started => {
            NodeExecutionProjectionStatus::Running
        }
        inference::InferenceRequestLifecycleEventKind::Completed => {
            NodeExecutionProjectionStatus::Completed
        }
        inference::InferenceRequestLifecycleEventKind::Failed => {
            NodeExecutionProjectionStatus::Failed
        }
        inference::InferenceRequestLifecycleEventKind::Cancelled => {
            NodeExecutionProjectionStatus::Cancelled
        }
        inference::InferenceRequestLifecycleEventKind::CleanupCompleted => return None,
    };
    let occurred_at_ms = i64::try_from(event.occurred_at_ms).unwrap_or(i64::MAX);
    let terminal = matches!(
        status,
        NodeExecutionProjectionStatus::Completed
            | NodeExecutionProjectionStatus::Failed
            | NodeExecutionProjectionStatus::Cancelled
    );

    Some(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: event.runtime_instance_id.clone(),
        occurred_at_ms,
        workflow_run_id: Some(context.attribution().workflow_run_id.clone()),
        workflow_id: Some(context.workflow_id().clone()),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(context.node_id().as_str().to_string()),
        node_type: Some(context.node_type().as_str().to_string()),
        node_version: context
            .effective_contract()
            .static_contract
            .contract_version
            .clone(),
        runtime_id: event.backend_key.clone(),
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: Some(context.attribution().client_id.clone()),
        client_session_id: Some(context.attribution().client_session_id.clone()),
        bucket_id: Some(context.attribution().bucket_id.clone()),
        scheduler_policy_id: None,
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::NodeExecutionStatus(NodeExecutionStatusPayload {
            status,
            started_at_ms: if event.kind == inference::InferenceRequestLifecycleEventKind::Started {
                Some(occurred_at_ms)
            } else {
                None
            },
            completed_at_ms: if terminal { Some(occurred_at_ms) } else { None },
            duration_ms: None,
            error: event.detail.clone(),
        }),
    })
}

impl ManagedModelUsageSubmission {
    pub fn completed(
        model: ModelIdentity,
        license_snapshot: LicenseSnapshot,
        output_measurement: ModelOutputMeasurement,
        started_at_ms: i64,
        completed_at_ms: i64,
    ) -> Self {
        Self {
            model,
            license_snapshot,
            output_measurement,
            status: UsageEventStatus::Completed,
            started_at_ms,
            completed_at_ms: Some(completed_at_ms),
            output_port_ids: Vec::new(),
            correlation_id: None,
            retention_class: RetentionClass::Standard,
        }
    }
}

impl ModelExecutionCapability {
    pub fn submit_usage_event(
        &self,
        ledger: &mut impl DiagnosticsLedgerRepository,
        context: &NodeExecutionContext,
        submission: ManagedModelUsageSubmission,
    ) -> Result<SubmittedModelUsageEvent, RuntimeLedgerSubmissionError> {
        let event = self.build_usage_event(context, submission)?;
        ledger.record_usage_event(event.clone())?;
        Ok(SubmittedModelUsageEvent { event })
    }

    pub fn build_usage_event(
        &self,
        context: &NodeExecutionContext,
        submission: ManagedModelUsageSubmission,
    ) -> Result<ModelLicenseUsageEvent, RuntimeLedgerSubmissionError> {
        self.validate_for_context(context)?;
        if let Some(completed_at_ms) = submission.completed_at_ms {
            if completed_at_ms < submission.started_at_ms {
                return Err(RuntimeLedgerSubmissionError::InvalidTimeRange);
            }
        }

        Ok(ModelLicenseUsageEvent {
            usage_event_id: UsageEventId::generate(),
            client_id: context.attribution().client_id.clone(),
            client_session_id: context.attribution().client_session_id.clone(),
            bucket_id: context.attribution().bucket_id.clone(),
            workflow_run_id: context.attribution().workflow_run_id.clone(),
            workflow_id: context.workflow_id().clone(),
            workflow_version_id: None,
            workflow_semantic_version: None,
            model: submission.model,
            lineage: usage_lineage(context, submission.output_port_ids),
            license_snapshot: submission.license_snapshot,
            output_measurement: submission.output_measurement.clone(),
            guarantee_level: guarantee_level(context, &submission.output_measurement),
            status: submission.status,
            retention_class: submission.retention_class,
            started_at_ms: submission.started_at_ms,
            completed_at_ms: submission.completed_at_ms,
            correlation_id: submission.correlation_id,
        })
    }

    fn validate_for_context(
        &self,
        context: &NodeExecutionContext,
    ) -> Result<(), RuntimeLedgerSubmissionError> {
        if self.route.kind != ManagedCapabilityKind::ModelExecution {
            return Err(RuntimeLedgerSubmissionError::ContextMismatch);
        }
        if !self.route.available {
            return Err(RuntimeLedgerSubmissionError::CapabilityUnavailable);
        }
        if self.route.workflow_id != *context.workflow_id()
            || self.route.attribution != *context.attribution()
            || self.route.node_id != *context.node_id()
            || self.route.node_type != *context.node_type()
        {
            return Err(RuntimeLedgerSubmissionError::ContextMismatch);
        }
        Ok(())
    }
}

fn usage_lineage(context: &NodeExecutionContext, output_port_ids: Vec<String>) -> UsageLineage {
    let port_ids = if output_port_ids.is_empty() {
        context
            .effective_contract()
            .outputs
            .iter()
            .map(|port| port.base.id.as_str().to_string())
            .collect()
    } else {
        output_port_ids
    };

    UsageLineage {
        node_id: context.node_id().as_str().to_string(),
        node_type: context.node_type().as_str().to_string(),
        port_ids,
        composed_parent_chain: context
            .lineage()
            .composed_node_stack
            .iter()
            .map(|node_id| node_id.as_str().to_string())
            .collect(),
        effective_contract_version: context
            .effective_contract()
            .static_contract
            .contract_version
            .clone(),
        effective_contract_digest: context
            .effective_contract()
            .static_contract
            .contract_digest
            .clone(),
        metadata_json: context
            .lineage()
            .lineage_segment_id
            .as_ref()
            .map(|segment_id| serde_json::json!({ "lineageSegmentId": segment_id }).to_string()),
    }
}

fn guarantee_level(
    context: &NodeExecutionContext,
    output_measurement: &ModelOutputMeasurement,
) -> ExecutionGuaranteeLevel {
    match context.guarantee() {
        NodeExecutionGuarantee::ManagedFull
            if !output_measurement.unavailable_reasons.is_empty() =>
        {
            ExecutionGuaranteeLevel::ManagedPartial
        }
        NodeExecutionGuarantee::ManagedFull => ExecutionGuaranteeLevel::ManagedFull,
        NodeExecutionGuarantee::ManagedPartial => ExecutionGuaranteeLevel::ManagedPartial,
        NodeExecutionGuarantee::EscapeHatchDetected => ExecutionGuaranteeLevel::EscapeHatchDetected,
        NodeExecutionGuarantee::UnsafeOrUnobserved => ExecutionGuaranteeLevel::UnsafeOrUnobserved,
    }
}

#[cfg(test)]
#[path = "node_execution_ledger_tests.rs"]
mod tests;

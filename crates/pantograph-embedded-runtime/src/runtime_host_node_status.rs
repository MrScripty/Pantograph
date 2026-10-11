//! Node-owner terminal status from the actual single-request host boundary.
//! This is independent of scheduler attempt projection. Accepted responses and
//! dropped futures do not prove completion or backend cancellation.

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, NodeExecutionProjectionStatus,
    NodeExecutionStatusPayload, RuntimeHostObservationOutcome, RuntimeHostRequestObservation,
};
use pantograph_runtime_host_contracts::RuntimeHostExecutionRequest;

pub(super) fn terminal_node_status(
    request: &RuntimeHostExecutionRequest,
    observation: &RuntimeHostRequestObservation,
    started_at_ms: i64,
) -> Option<DiagnosticEventAppendRequest> {
    let (status, error) = match observation.outcome {
        RuntimeHostObservationOutcome::Completed => {
            (NodeExecutionProjectionStatus::Completed, None)
        }
        RuntimeHostObservationOutcome::Failed => (
            NodeExecutionProjectionStatus::Failed,
            Some("runtime-host execution failed or returned an invalid response"),
        ),
        RuntimeHostObservationOutcome::Rejected => (
            NodeExecutionProjectionStatus::Failed,
            Some("runtime-host execution rejected"),
        ),
        RuntimeHostObservationOutcome::CancellationAcknowledged
        | RuntimeHostObservationOutcome::ShutdownAcknowledged => (
            NodeExecutionProjectionStatus::Cancelled,
            Some("runtime-host cancellation or shutdown acknowledged"),
        ),
        RuntimeHostObservationOutcome::Abandoned | RuntimeHostObservationOutcome::Nonterminal => {
            return None;
        }
    };
    let selected = request.handoff.dispatch_decision.as_ref()?;
    Some(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: Some("embedded-runtime-host".into()),
        occurred_at_ms: observation.recorded_at_ms,
        workflow_run_id: Some(
            request
                .handoff
                .workflow_run_id
                .as_str()
                .to_owned()
                .try_into()
                .ok()?,
        ),
        workflow_id: Some(
            request
                .handoff
                .workflow_id
                .as_str()
                .to_owned()
                .try_into()
                .ok()?,
        ),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(request.handoff.node_id.to_string()),
        node_type: None,
        node_version: None,
        runtime_id: Some(selected.selected_runtime_id.to_string()),
        runtime_version: None,
        model_id: Some(selected.selected_model_ref.model_id.clone()),
        model_version: selected.selected_model_ref.revision.clone(),
        client_id: None,
        client_session_id: None,
        bucket_id: None,
        scheduler_policy_id: None,
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::NodeExecutionStatus(NodeExecutionStatusPayload {
            status,
            started_at_ms: Some(started_at_ms),
            completed_at_ms: Some(observation.recorded_at_ms),
            // Host elapsed includes resolution, gateway waiting, loading and
            // output publication. This does not claim backend compute timing.
            duration_ms: Some(observation.host_elapsed_ms),
            error: error.map(str::to_owned),
            canonical_error_event_id: None,
            task_id: Some(request.handoff.task_intent.task_type.to_string()),
            // The handoff declares runtime identity, not a gateway backend key.
            selected_backend_key: None,
            execution_cache_status: None,
        }),
    })
}

use std::sync::Arc;

use crate::workflow::diagnostics::{
    DiagnosticsRuntimeLifecycleSnapshot, WorkflowDiagnosticsSnapshotRequest,
    WorkflowDiagnosticsStore,
};
use crate::workflow::headless_diagnostics::{
    HeadlessRuntimeSnapshotRecordInput, WorkflowDiagnosticsSnapshotProjectionInput,
    record_headless_runtime_snapshot, record_headless_scheduler_snapshot,
    stored_runtime_model_targets, stored_runtime_snapshots, stored_runtime_trace_metrics,
    workflow_clear_diagnostics_history_response, workflow_diagnostics_snapshot_projection,
    workflow_scheduler_snapshot_response, workflow_trace_snapshot_response,
};
use pantograph_workflow_service::graph::WorkflowExecutionSessionKind;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowCapabilityModel, WorkflowErrorCode, WorkflowErrorDetails,
    WorkflowErrorEnvelope, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionState,
    WorkflowExecutionSessionSummary, WorkflowGraph, WorkflowGraphEditSessionCreateRequest,
    WorkflowRuntimeRequirements, WorkflowSchedulerErrorDetails,
    WorkflowSchedulerRuntimeRegistryDiagnostics, WorkflowSchedulerRuntimeWarmupDecision,
    WorkflowSchedulerRuntimeWarmupReason, WorkflowSchedulerSnapshotDiagnostics,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse, WorkflowService,
    WorkflowServiceError, WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest,
};

macro_rules! workflow_projection {
    ($store:expr, $input:expr $(,)?) => {
        workflow_diagnostics_snapshot_projection($store, $input)
    };
    (
            $store:expr,
            $session_id:expr,
            $workflow_id:expr,
            $workflow_name:expr,
            $scheduler_snapshot_result:expr,
            $capabilities_result:expr,
            $current_session_state:expr,
            $runtime_trace_metrics:expr,
            $active_model_target:expr,
            $embedding_model_target:expr,
            $active_runtime_snapshot:expr,
            $embedding_runtime_snapshot:expr,
            $managed_runtimes:expr,
            $captured_at_ms:expr $(,)?
        ) => {
        workflow_diagnostics_snapshot_projection(
            $store,
            WorkflowDiagnosticsSnapshotProjectionInput {
                workflow_run_id: None,
                session_id: $session_id,
                workflow_id: $workflow_id,
                scheduler_snapshot_result: $scheduler_snapshot_result,
                capabilities_result: $capabilities_result,
                current_session_state: $current_session_state,
                workflow_graph: None,
                active_model_target: $active_model_target,
                embedding_model_target: $embedding_model_target,
                active_runtime_snapshot: $active_runtime_snapshot,
                embedding_runtime_snapshot: $embedding_runtime_snapshot,
                managed_runtimes: $managed_runtimes,
                captured_at_ms: $captured_at_ms,
            },
        )
    };
}

fn workflow_error_json(error: WorkflowServiceError) -> String {
    super::workflow_error_json(error)
}

fn running_session_summary() -> WorkflowExecutionSessionSummary {
    WorkflowExecutionSessionSummary {
        session_id: "session-1".to_string(),
        workflow_id: "wf-1".to_string(),
        session_kind: WorkflowExecutionSessionKind::Workflow,
        usage_profile: Some("interactive".to_string()),
        keep_alive: true,
        state: WorkflowExecutionSessionState::Running,
        queued_runs: 1,
        run_count: 2,
    }
}

fn capability_response() -> WorkflowCapabilitiesResponse {
    WorkflowCapabilitiesResponse {
        max_input_bindings: 4,
        max_output_targets: 2,
        max_value_bytes: 2_048,
        runtime_requirements: WorkflowRuntimeRequirements {
            estimated_peak_vram_mb: None,
            estimated_peak_ram_mb: None,
            estimated_min_vram_mb: None,
            estimated_min_ram_mb: None,
            estimation_confidence: "high".to_string(),
            required_models: vec!["model-a".to_string()],
            required_backends: vec!["llama_cpp".to_string()],
            required_extensions: vec!["kv_cache".to_string()],
        },
        models: vec![WorkflowCapabilityModel {
            model_id: "model-a".to_string(),
            model_revision_or_hash: None,
            model_type: Some("embedding".to_string()),
            node_ids: vec!["node-a".to_string()],
            roles: vec!["embedding".to_string()],
        }],
        runtime_capabilities: Vec::new(),
    }
}

#[path = "headless_workflow_commands_tests/diagnostics_helpers.rs"]
mod diagnostics_helpers;
#[path = "headless_workflow_commands_tests/diagnostics_projection.rs"]
mod diagnostics_projection;
#[path = "headless_workflow_commands_tests/transport_responses.rs"]
mod transport_responses;

use crate::HostRuntimeModeSnapshot;
use crate::runtime_health::RuntimeHealthAssessmentSnapshot;
use async_trait::async_trait;
use pantograph_runtime_registry::RuntimeRegistry;
use pantograph_workflow_service::graph::WorkflowSessionKind;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowSchedulerSnapshotResponse, WorkflowSessionQueueItem,
    WorkflowSessionState, WorkflowSessionSummary, WorkflowTraceRuntimeMetrics,
};

use super::{
    WorkflowExecutionDiagnosticsController, WorkflowExecutionDiagnosticsInput,
    WorkflowExecutionDiagnosticsSyncInput, build_runtime_diagnostics_projection,
    build_runtime_event_projection, build_runtime_event_projection_with_registry_override,
    build_runtime_event_projection_with_registry_reconciliation,
    build_runtime_event_projection_with_registry_sync,
    build_workflow_execution_diagnostics_snapshot,
    build_workflow_execution_diagnostics_snapshot_with_registry_sync,
    normalized_runtime_lifecycle_snapshot, reconcile_runtime_registry_stored_projection_overrides,
    resolve_runtime_model_target, trace_runtime_metrics,
    trace_runtime_metrics_with_observed_runtime_ids,
};

struct MockRuntimeRegistryController {
    mode_info: HostRuntimeModeSnapshot,
    active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
}

#[async_trait]
impl crate::runtime_registry::HostRuntimeRegistryController for MockRuntimeRegistryController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        self.mode_info.clone()
    }

    async fn stop_runtime_producer(&self, _producer: crate::runtime_registry::HostRuntimeProducer) {
    }

    async fn runtime_health_assessment_snapshot(&self) -> RuntimeHealthAssessmentSnapshot {
        RuntimeHealthAssessmentSnapshot::default()
    }
}

#[async_trait]
impl WorkflowExecutionDiagnosticsController for MockRuntimeRegistryController {
    async fn active_runtime_lifecycle_snapshot(&self) -> inference::RuntimeLifecycleSnapshot {
        self.active_runtime_snapshot.clone()
    }

    async fn embedding_runtime_lifecycle_snapshot(
        &self,
    ) -> Option<inference::RuntimeLifecycleSnapshot> {
        self.embedding_runtime_snapshot.clone()
    }
}

#[path = "workflow_runtime_tests/diagnostics_snapshot.rs"]
mod diagnostics_snapshot;
#[path = "workflow_runtime_tests/event_projection.rs"]
mod event_projection;
#[path = "workflow_runtime_tests/metrics.rs"]
mod metrics;
#[path = "workflow_runtime_tests/registry_reconciliation.rs"]
mod registry_reconciliation;

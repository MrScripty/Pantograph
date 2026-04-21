use std::sync::Arc;

use async_trait::async_trait;
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::{
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerRuntimeRegistryDiagnostics, WorkflowServiceError,
};

use crate::host_runtime::HostRuntimeModeSnapshot;
use crate::runtime_registry;
use crate::runtime_registry_errors::workflow_service_error_from_runtime_registry;

#[derive(Clone)]
pub(crate) struct EmbeddedWorkflowSchedulerDiagnosticsProvider {
    gateway: Arc<inference::InferenceGateway>,
    runtime_registry: SharedRuntimeRegistry,
}

impl EmbeddedWorkflowSchedulerDiagnosticsProvider {
    pub(crate) fn new(
        gateway: Arc<inference::InferenceGateway>,
        runtime_registry: SharedRuntimeRegistry,
    ) -> Self {
        Self {
            gateway,
            runtime_registry,
        }
    }
}

#[async_trait]
impl WorkflowSchedulerDiagnosticsProvider for EmbeddedWorkflowSchedulerDiagnosticsProvider {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        let mode_info = self.gateway.mode_info().await;
        let host_runtime_mode_info = HostRuntimeModeSnapshot::from_mode_info(&mode_info);
        Ok(Some(
            runtime_registry::scheduler_runtime_registry_diagnostics(
                &self.runtime_registry,
                &host_runtime_mode_info,
                request,
            )
            .map_err(workflow_service_error_from_runtime_registry)?,
        ))
    }
}

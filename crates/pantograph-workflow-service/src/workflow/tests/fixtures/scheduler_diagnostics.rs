use super::*;

#[derive(Clone)]
pub(in crate::workflow::tests) struct MockSchedulerDiagnosticsProvider {
    pub(in crate::workflow::tests) diagnostics: WorkflowSchedulerRuntimeRegistryDiagnostics,
    pub(in crate::workflow::tests) requests:
        Arc<Mutex<Vec<WorkflowSchedulerRuntimeDiagnosticsRequest>>>,
}

#[async_trait]
impl WorkflowSchedulerDiagnosticsProvider for MockSchedulerDiagnosticsProvider {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        self.requests
            .lock()
            .expect("scheduler diagnostics requests lock poisoned")
            .push(request.clone());
        Ok(Some(self.diagnostics.clone()))
    }
}

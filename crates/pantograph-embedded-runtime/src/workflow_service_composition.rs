use std::sync::Arc;

use pantograph_workflow_service::{
    WorkflowDependencyReadinessComponents, WorkflowService, WorkflowServiceError,
};

use crate::SharedWorkflowService;

/// Builds embedded-runtime workflow services before sharing them across hosts.
///
/// This composition boundary keeps infrastructure wiring at the embedded
/// runtime edge. Workflow-service remains responsible for orchestration, while
/// this module owns the moment concrete dependency-readiness components are
/// attached before the service is wrapped in `Arc`.
#[derive(Debug, Default)]
pub(crate) struct EmbeddedWorkflowServiceComposition {
    dependency_readiness: WorkflowDependencyReadinessComponents,
}

impl EmbeddedWorkflowServiceComposition {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn dependency_readiness(&self) -> &WorkflowDependencyReadinessComponents {
        &self.dependency_readiness
    }

    pub(crate) fn into_shared_workflow_service(
        self,
        max_loaded_sessions: Option<usize>,
    ) -> Result<SharedWorkflowService, WorkflowServiceError> {
        self.into_shared_configured_workflow_service(WorkflowService::new(), max_loaded_sessions)
    }

    pub(crate) fn into_shared_configured_workflow_service(
        self,
        service: WorkflowService,
        max_loaded_sessions: Option<usize>,
    ) -> Result<SharedWorkflowService, WorkflowServiceError> {
        let service = self
            .dependency_readiness
            .configure_workflow_service(service);
        service.set_loaded_runtime_capacity_limit(max_loaded_sessions)?;
        Ok(Arc::new(service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_shared_service_after_dependency_readiness_wiring() {
        let composition = EmbeddedWorkflowServiceComposition::new();
        let snapshot_provider = composition.dependency_readiness().snapshot_provider();
        let work_queue = composition.dependency_readiness().work_queue();
        let requirements_registry = composition.dependency_readiness().requirements_registry();

        let service = composition
            .into_shared_workflow_service(Some(1))
            .expect("composition should build workflow service");

        assert!(Arc::strong_count(&snapshot_provider) > 1);
        assert!(Arc::strong_count(&work_queue) > 1);
        assert!(Arc::strong_count(&requirements_registry) > 1);
        drop(service);
        assert_eq!(Arc::strong_count(&snapshot_provider), 1);
        assert_eq!(Arc::strong_count(&work_queue), 1);
        assert_eq!(Arc::strong_count(&requirements_registry), 1);
    }

    #[test]
    fn rejects_invalid_capacity_before_service_is_shared() {
        let error =
            match EmbeddedWorkflowServiceComposition::new().into_shared_workflow_service(Some(0)) {
                Ok(_) => panic!("zero capacity should be rejected"),
                Err(error) => error,
            };

        assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
    }

    #[test]
    fn builds_from_host_customized_unshared_service() {
        let composition = EmbeddedWorkflowServiceComposition::new();
        let snapshot_provider = composition.dependency_readiness().snapshot_provider();
        let service = WorkflowService::with_capacity_limits(4, 2);

        let shared = composition
            .into_shared_configured_workflow_service(service, Some(3))
            .expect("composition should accept host-customized service");

        assert!(Arc::strong_count(&snapshot_provider) > 1);
        drop(shared);
        assert_eq!(Arc::strong_count(&snapshot_provider), 1);
    }
}

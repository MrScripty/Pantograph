use std::sync::Arc;

use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshotProvider, DependencyReadinessWorkQueue,
    InMemoryDependencyRequirementsRegistry,
};

use super::{WorkflowService, WorkflowServiceError};

/// Backend composition helper for dependency-readiness snapshot wiring.
///
/// This type creates the shared synchronous snapshot provider that workflow
/// services consume and the shared producer work queue that scheduler/session
/// orchestration will fill. Async probing and snapshot production remain owned
/// by an outer embedded-runtime or infrastructure lifecycle.
#[derive(Debug, Clone)]
pub struct WorkflowDependencyReadinessComponents {
    snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
    work_queue: Arc<DependencyReadinessWorkQueue>,
    requirements_registry: Arc<InMemoryDependencyRequirementsRegistry>,
}

impl Default for WorkflowDependencyReadinessComponents {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowDependencyReadinessComponents {
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshot_provider: Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new()),
            work_queue: Arc::new(DependencyReadinessWorkQueue::new()),
            requirements_registry: Arc::new(InMemoryDependencyRequirementsRegistry::new()),
        }
    }

    #[must_use]
    pub fn with_parts(
        snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
        work_queue: Arc<DependencyReadinessWorkQueue>,
        requirements_registry: Arc<InMemoryDependencyRequirementsRegistry>,
    ) -> Self {
        Self {
            snapshot_provider,
            work_queue,
            requirements_registry,
        }
    }

    #[must_use]
    pub fn with_snapshot_provider_and_work_queue(
        snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
        work_queue: Arc<DependencyReadinessWorkQueue>,
    ) -> Self {
        Self::with_parts(
            snapshot_provider,
            work_queue,
            Arc::new(InMemoryDependencyRequirementsRegistry::new()),
        )
    }

    #[must_use]
    pub fn snapshot_provider(&self) -> Arc<DependencyEnvironmentReadinessSnapshotProvider> {
        self.snapshot_provider.clone()
    }

    #[must_use]
    pub fn work_queue(&self) -> Arc<DependencyReadinessWorkQueue> {
        self.work_queue.clone()
    }

    #[must_use]
    pub fn requirements_registry(&self) -> Arc<InMemoryDependencyRequirementsRegistry> {
        self.requirements_registry.clone()
    }

    #[must_use]
    pub fn configure_workflow_service(&self, service: WorkflowService) -> WorkflowService {
        service
            .with_dependency_environment_provider(self.snapshot_provider.clone())
            .with_dependency_readiness_work_queue(self.work_queue.clone())
            .with_dependency_requirements_registry(self.requirements_registry.clone())
    }

    #[must_use]
    pub fn workflow_service(&self) -> WorkflowService {
        self.configure_workflow_service(WorkflowService::new())
    }

    pub fn ephemeral_attribution_workflow_service(
        &self,
    ) -> Result<WorkflowService, WorkflowServiceError> {
        Ok(self.configure_workflow_service(WorkflowService::with_ephemeral_attribution_store()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_create_empty_snapshot_provider_before_service_sharing() {
        let components = WorkflowDependencyReadinessComponents::new();
        let provider = components.snapshot_provider();
        let work_queue = components.work_queue();
        let requirements_registry = components.requirements_registry();
        let service = components.workflow_service();
        let shared = Arc::new(service);

        assert_eq!(provider.snapshot_count(), 0);
        assert!(work_queue.is_empty());
        assert!(requirements_registry.is_empty());
        assert_eq!(Arc::strong_count(&provider), 4);
        assert_eq!(Arc::strong_count(&work_queue), 3);
        assert_eq!(Arc::strong_count(&requirements_registry), 3);
        drop(shared);
        assert_eq!(Arc::strong_count(&provider), 2);
        assert_eq!(Arc::strong_count(&work_queue), 2);
        assert_eq!(Arc::strong_count(&requirements_registry), 2);
    }

    #[test]
    fn components_accept_injected_snapshot_provider_and_work_queue() {
        let provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
        let requirements_registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
        let components = WorkflowDependencyReadinessComponents::with_parts(
            provider.clone(),
            work_queue.clone(),
            requirements_registry.clone(),
        );

        assert!(Arc::ptr_eq(&provider, &components.snapshot_provider()));
        assert!(Arc::ptr_eq(&work_queue, &components.work_queue()));
        assert!(Arc::ptr_eq(
            &requirements_registry,
            &components.requirements_registry()
        ));
    }
}

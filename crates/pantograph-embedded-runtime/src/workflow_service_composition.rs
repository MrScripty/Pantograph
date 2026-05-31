use std::sync::Arc;

use pantograph_runtime_host_contracts::{ReservationLifecyclePort, RuntimeHostExecutionPort};
use pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateProvider;
use pantograph_workflow_service::{
    WorkflowDependencyReadinessComponents, WorkflowService, WorkflowServiceError,
};

use crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsSource;
use crate::runtime_dispatch_candidate_provider::EmbeddedRuntimeDispatchCandidateProvider;
use crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsSource;
use crate::runtime_dispatch_resource_facts::RuntimeDispatchResourceFactsSource;
use crate::runtime_dispatch_source_snapshot::EmbeddedRuntimeDispatchSourceFactSnapshotStore;
use crate::SharedWorkflowService;

/// Builds embedded-runtime workflow services before sharing them across hosts.
///
/// This composition boundary keeps infrastructure wiring at the embedded
/// runtime edge. Workflow-service remains responsible for orchestration, while
/// this module owns the moment concrete dependency-readiness components are
/// attached before the service is wrapped in `Arc`.
#[derive(Default)]
pub(crate) struct EmbeddedWorkflowServiceComposition {
    dependency_readiness: WorkflowDependencyReadinessComponents,
    dispatch_dependencies: Option<EmbeddedWorkflowServiceDispatchDependencies>,
}

#[derive(Clone)]
pub(crate) struct EmbeddedWorkflowServiceDispatchDependencies {
    runtime_dispatch_candidate_provider: Arc<dyn WorkflowRuntimeDispatchCandidateProvider>,
    runtime_host_execution_port: Arc<dyn RuntimeHostExecutionPort>,
    reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
}

impl EmbeddedWorkflowServiceDispatchDependencies {
    #[must_use]
    pub(crate) fn new(
        runtime_dispatch_candidate_provider: Arc<dyn WorkflowRuntimeDispatchCandidateProvider>,
        runtime_host_execution_port: Arc<dyn RuntimeHostExecutionPort>,
        reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
    ) -> Self {
        Self {
            runtime_dispatch_candidate_provider,
            runtime_host_execution_port,
            reservation_lifecycle_port,
        }
    }

    #[must_use]
    pub(crate) fn resource_backed(
        pumas_source: PumasDispatchPackageFactsSource,
        runtime_capability_source: RuntimeDispatchCapabilityFactsSource,
        resource_facts_source: RuntimeDispatchResourceFactsSource,
        max_snapshot_age_ms: u64,
        runtime_host_execution_port: Arc<dyn RuntimeHostExecutionPort>,
        reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
    ) -> Self {
        let snapshot_store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            pumas_source,
            runtime_capability_source,
            max_snapshot_age_ms,
        );
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot_store(snapshot_store)
                .with_resource_facts_source(resource_facts_source);
        Self::new(
            Arc::new(provider),
            runtime_host_execution_port,
            reservation_lifecycle_port,
        )
    }

    #[must_use]
    fn configure_workflow_service(self, service: WorkflowService) -> WorkflowService {
        service
            .with_runtime_dispatch_candidate_provider(self.runtime_dispatch_candidate_provider)
            .with_runtime_host_execution_port(self.runtime_host_execution_port)
            .with_reservation_lifecycle_port(self.reservation_lifecycle_port)
    }
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

    #[must_use]
    pub(crate) fn with_runtime_dispatch_dependencies(
        mut self,
        dependencies: EmbeddedWorkflowServiceDispatchDependencies,
    ) -> Self {
        self.dispatch_dependencies = Some(dependencies);
        self
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
        let service = match self.dispatch_dependencies {
            Some(dependencies) => dependencies.configure_workflow_service(service),
            None => service,
        };
        service.set_loaded_runtime_capacity_limit(max_loaded_sessions)?;
        Ok(Arc::new(service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use pantograph_runtime_host_contracts::{
        ReservationLifecycleApplication, ReservationLifecycleEvent, ReservationLifecyclePortError,
        RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    };
    use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};

    #[derive(Debug)]
    struct RejectingRuntimeHostPort;

    #[async_trait]
    impl RuntimeHostExecutionPort for RejectingRuntimeHostPort {
        async fn execute_runtime_host_request(
            &self,
            _request: RuntimeHostExecutionRequest,
        ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
            Err(RuntimeHostExecutionPortError::ExecutionFailed {
                message: "test runtime host port is not executable".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct RejectingReservationLifecyclePort;

    #[async_trait]
    impl ReservationLifecyclePort for RejectingReservationLifecyclePort {
        async fn apply_reservation_lifecycle(
            &self,
            _event: ReservationLifecycleEvent,
        ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
            Err(ReservationLifecyclePortError::Failed {
                message: "test reservation lifecycle port is not executable".to_string(),
            })
        }
    }

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

    #[test]
    fn builds_with_paired_runtime_dispatch_dependencies() {
        let dependencies = EmbeddedWorkflowServiceDispatchDependencies::new(
            Arc::new(EmbeddedRuntimeDispatchCandidateProvider::new()),
            Arc::new(RejectingRuntimeHostPort),
            Arc::new(RejectingReservationLifecyclePort),
        );

        let shared = EmbeddedWorkflowServiceComposition::new()
            .with_runtime_dispatch_dependencies(dependencies)
            .into_shared_workflow_service(Some(1))
            .expect("paired dependencies should build workflow service");

        drop(shared);
    }

    #[test]
    fn builds_resource_backed_dispatch_dependencies_as_pair() {
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let dependencies = EmbeddedWorkflowServiceDispatchDependencies::resource_backed(
            PumasDispatchPackageFactsSource::new(None),
            RuntimeDispatchCapabilityFactsSource::new(registry.clone()),
            RuntimeDispatchResourceFactsSource::new(registry),
            1_000,
            Arc::new(RejectingRuntimeHostPort),
            Arc::new(RejectingReservationLifecyclePort),
        );

        let shared = EmbeddedWorkflowServiceComposition::new()
            .with_runtime_dispatch_dependencies(dependencies)
            .into_shared_workflow_service(Some(1))
            .expect("resource-backed dependency bundle should build workflow service");

        drop(shared);
    }
}

use std::sync::Arc;

use pantograph_runtime_host_contracts::{ReservationLifecyclePort, RuntimeHostExecutionPort};
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchSourceRefresher,
};
use pantograph_workflow_service::{
    WorkflowDependencyReadinessComponents, WorkflowService, WorkflowServiceError,
};
use workflow_nodes::setup::PumasSelectorAccess;

use crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsSource;
use crate::reservation_lifecycle::EmbeddedReservationLifecyclePort;
use crate::runtime_dispatch_candidate_provider::EmbeddedRuntimeDispatchCandidateProvider;
use crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsSource;
use crate::runtime_dispatch_resource_facts::RuntimeDispatchResourceFactsSource;
use crate::runtime_dispatch_source_snapshot::{
    EmbeddedRuntimeDispatchSourceFactRefresher, EmbeddedRuntimeDispatchSourceFactSnapshotStore,
};
use crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort;
use crate::runtime_host_load_target::RuntimeHostPumasLoadTargetResolver;
use crate::workflow_scheduler_diagnostics::EmbeddedWorkflowSchedulerDiagnosticsProvider;
use crate::{
    runtime_registry::HostRuntimeRegistryController, EmbeddedDependencyReadinessSnapshotProducer,
    EmbeddedDependencyReadinessSnapshotProducerConfig,
    EmbeddedDependencyReadinessSnapshotProducerHandle, EmbeddedRuntimeError, SharedWorkflowService,
};

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
    scheduler_diagnostics_provider:
        Option<Arc<dyn pantograph_workflow_service::WorkflowSchedulerDiagnosticsProvider>>,
}

pub(crate) struct EmbeddedHostedWorkflowServiceFactoryInput<C> {
    pub(crate) workflow_service: WorkflowService,
    pub(crate) max_loaded_sessions: Option<usize>,
    pub(crate) runtime_registry: SharedRuntimeRegistry,
    pub(crate) runtime_registry_controller: Arc<C>,
    pub(crate) gateway: Arc<inference::InferenceGateway>,
    pub(crate) pumas_selector_access: Arc<PumasSelectorAccess>,
    pub(crate) max_dispatch_source_snapshot_age_ms: u64,
}

impl<C> EmbeddedHostedWorkflowServiceFactoryInput<C> {
    pub(crate) fn new(
        runtime_registry: SharedRuntimeRegistry,
        runtime_registry_controller: Arc<C>,
        gateway: Arc<inference::InferenceGateway>,
        pumas_selector_access: Arc<PumasSelectorAccess>,
        max_loaded_sessions: Option<usize>,
        max_dispatch_source_snapshot_age_ms: u64,
    ) -> Self {
        Self {
            workflow_service: WorkflowService::new(),
            max_loaded_sessions,
            runtime_registry,
            runtime_registry_controller,
            gateway,
            pumas_selector_access,
            max_dispatch_source_snapshot_age_ms,
        }
    }

    #[must_use]
    pub(crate) fn with_workflow_service(mut self, workflow_service: WorkflowService) -> Self {
        self.workflow_service = workflow_service;
        self
    }
}

pub(crate) struct EmbeddedHostedWorkflowServiceCompositionInput<C> {
    pub(crate) factory_input: EmbeddedHostedWorkflowServiceFactoryInput<C>,
    pub(crate) dependency_readiness_runtime_handle: tokio::runtime::Handle,
    pub(crate) dependency_readiness_producer_config:
        EmbeddedDependencyReadinessSnapshotProducerConfig,
}

impl<C> EmbeddedHostedWorkflowServiceCompositionInput<C> {
    pub(crate) fn new(
        factory_input: EmbeddedHostedWorkflowServiceFactoryInput<C>,
        dependency_readiness_runtime_handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            factory_input,
            dependency_readiness_runtime_handle,
            dependency_readiness_producer_config:
                EmbeddedDependencyReadinessSnapshotProducerConfig::default(),
        }
    }

    #[must_use]
    pub(crate) fn with_dependency_readiness_producer_config(
        mut self,
        config: EmbeddedDependencyReadinessSnapshotProducerConfig,
    ) -> Self {
        self.dependency_readiness_producer_config = config;
        self
    }
}

pub(crate) struct EmbeddedHostedWorkflowServiceCompositionOutput {
    pub(crate) workflow_service: SharedWorkflowService,
    pub(crate) dependency_readiness_snapshot_producer:
        EmbeddedDependencyReadinessSnapshotProducerHandle,
}

impl EmbeddedHostedWorkflowServiceCompositionOutput {
    #[must_use]
    pub(crate) fn workflow_service(&self) -> &SharedWorkflowService {
        &self.workflow_service
    }
}

#[derive(Clone)]
pub(crate) struct EmbeddedWorkflowServiceDispatchDependencies {
    runtime_dispatch_candidate_provider: Arc<dyn WorkflowRuntimeDispatchCandidateProvider>,
    runtime_dispatch_source_refresher: Arc<dyn WorkflowRuntimeDispatchSourceRefresher>,
    runtime_host_execution_port: Arc<dyn RuntimeHostExecutionPort>,
    reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
}

impl EmbeddedWorkflowServiceDispatchDependencies {
    #[must_use]
    pub(crate) fn new(
        runtime_dispatch_candidate_provider: Arc<dyn WorkflowRuntimeDispatchCandidateProvider>,
        runtime_dispatch_source_refresher: Arc<dyn WorkflowRuntimeDispatchSourceRefresher>,
        runtime_host_execution_port: Arc<dyn RuntimeHostExecutionPort>,
        reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
    ) -> Self {
        Self {
            runtime_dispatch_candidate_provider,
            runtime_dispatch_source_refresher,
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
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot_store(
            snapshot_store.clone(),
        )
        .with_resource_facts_source(resource_facts_source);
        let refresher = EmbeddedRuntimeDispatchSourceFactRefresher::new(snapshot_store);
        Self::new(
            Arc::new(provider),
            Arc::new(refresher),
            runtime_host_execution_port,
            reservation_lifecycle_port,
        )
    }

    #[must_use]
    fn configure_workflow_service(self, service: WorkflowService) -> WorkflowService {
        service
            .with_runtime_dispatch_source_refresher(self.runtime_dispatch_source_refresher)
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

    #[must_use]
    pub(crate) fn with_scheduler_diagnostics_provider(
        mut self,
        provider: Arc<dyn pantograph_workflow_service::WorkflowSchedulerDiagnosticsProvider>,
    ) -> Self {
        self.scheduler_diagnostics_provider = Some(provider);
        self
    }

    pub(crate) fn resource_backed_hosted<C>(
        input: EmbeddedHostedWorkflowServiceFactoryInput<C>,
    ) -> Result<SharedWorkflowService, WorkflowServiceError>
    where
        C: HostRuntimeRegistryController + Send + Sync + 'static,
    {
        let pumas_api = match input.pumas_selector_access.as_ref() {
            PumasSelectorAccess::Owner(api) => api.clone(),
            PumasSelectorAccess::LocalClient(_) | PumasSelectorAccess::ReadOnly(_) => {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "hosted resource-backed workflow-service construction requires Pumas owner selector access, got {} access",
                    input.pumas_selector_access.role_name()
                )));
            }
        };
        let runtime_host_execution_port =
            Arc::new(EmbeddedRuntimeHostExecutionPort::with_load_target_resolver(
                RuntimeHostPumasLoadTargetResolver::new(pumas_api),
            ));
        let reservation_lifecycle_port = Arc::new(EmbeddedReservationLifecyclePort::new(
            input.runtime_registry.clone(),
            input.runtime_registry_controller,
        ));
        let dispatch_dependencies = EmbeddedWorkflowServiceDispatchDependencies::resource_backed(
            PumasDispatchPackageFactsSource::new(Some(input.pumas_selector_access)),
            RuntimeDispatchCapabilityFactsSource::new(input.runtime_registry.clone()),
            RuntimeDispatchResourceFactsSource::new(input.runtime_registry.clone()),
            input.max_dispatch_source_snapshot_age_ms,
            runtime_host_execution_port,
            reservation_lifecycle_port,
        );
        let scheduler_diagnostics_provider =
            Arc::new(EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
                input.gateway,
                input.runtime_registry,
            ));
        Self::new()
            .with_runtime_dispatch_dependencies(dispatch_dependencies)
            .with_scheduler_diagnostics_provider(scheduler_diagnostics_provider)
            .into_shared_configured_workflow_service(
                input.workflow_service,
                input.max_loaded_sessions,
            )
    }

    pub(crate) fn resource_backed_hosted_bundle<C>(
        input: EmbeddedHostedWorkflowServiceCompositionInput<C>,
    ) -> Result<EmbeddedHostedWorkflowServiceCompositionOutput, EmbeddedRuntimeError>
    where
        C: HostRuntimeRegistryController + Send + Sync + 'static,
    {
        let pumas_api = match input.factory_input.pumas_selector_access.as_ref() {
            PumasSelectorAccess::Owner(api) => api.clone(),
            PumasSelectorAccess::LocalClient(_) | PumasSelectorAccess::ReadOnly(_) => {
                return Err(EmbeddedRuntimeError::Initialization {
                    message: format!(
                        "hosted resource-backed workflow-service composition requires Pumas owner selector access, got {} access",
                        input.factory_input.pumas_selector_access.role_name()
                    ),
                });
            }
        };
        let dependency_readiness_runtime_handle = input.dependency_readiness_runtime_handle;
        let dependency_readiness_producer_config = input.dependency_readiness_producer_config;
        let factory_input = input.factory_input;
        let runtime_host_execution_port =
            Arc::new(EmbeddedRuntimeHostExecutionPort::with_load_target_resolver(
                RuntimeHostPumasLoadTargetResolver::new(pumas_api),
            ));
        let reservation_lifecycle_port = Arc::new(EmbeddedReservationLifecyclePort::new(
            factory_input.runtime_registry.clone(),
            factory_input.runtime_registry_controller,
        ));
        let dispatch_dependencies = EmbeddedWorkflowServiceDispatchDependencies::resource_backed(
            PumasDispatchPackageFactsSource::new(Some(factory_input.pumas_selector_access)),
            RuntimeDispatchCapabilityFactsSource::new(factory_input.runtime_registry.clone()),
            RuntimeDispatchResourceFactsSource::new(factory_input.runtime_registry.clone()),
            factory_input.max_dispatch_source_snapshot_age_ms,
            runtime_host_execution_port,
            reservation_lifecycle_port,
        );
        let scheduler_diagnostics_provider =
            Arc::new(EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
                factory_input.gateway,
                factory_input.runtime_registry,
            ));
        let composition = Self::new()
            .with_runtime_dispatch_dependencies(dispatch_dependencies)
            .with_scheduler_diagnostics_provider(scheduler_diagnostics_provider);
        let dependency_readiness = composition.dependency_readiness().clone();
        let workflow_service = composition
            .into_shared_configured_workflow_service(
                factory_input.workflow_service,
                factory_input.max_loaded_sessions,
            )
            .map_err(|error| EmbeddedRuntimeError::Initialization {
                message: error.to_string(),
            })?;
        let dependency_readiness_snapshot_producer =
            EmbeddedDependencyReadinessSnapshotProducer::new(
                dependency_readiness.snapshot_provider(),
                dependency_readiness.work_queue(),
                dependency_readiness.requirements_registry(),
            )
            .with_config(dependency_readiness_producer_config)
            .spawn(dependency_readiness_runtime_handle)?;
        Ok(EmbeddedHostedWorkflowServiceCompositionOutput {
            workflow_service,
            dependency_readiness_snapshot_producer,
        })
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
        let service = match self.scheduler_diagnostics_provider {
            Some(provider) => service.with_scheduler_diagnostics_provider(provider),
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
    use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
    use pantograph_runtime_host_contracts::{
        ReservationLifecycleApplication, ReservationLifecycleEvent, ReservationLifecyclePortError,
        RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    };
    use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};
    use pantograph_scheduler::SchedulerTaskStateRecord;
    use pantograph_workflow_service::workflow::{
        WorkflowRuntimeDispatchSourceRefreshError, WorkflowSchedulerTask,
    };

    #[derive(Debug)]
    struct RejectingRuntimeHostPort;

    #[derive(Debug)]
    struct AcceptingDispatchSourceRefresher;

    #[async_trait]
    impl WorkflowRuntimeDispatchSourceRefresher for AcceptingDispatchSourceRefresher {
        async fn refresh_runtime_dispatch_sources(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
            Ok(())
        }
    }

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
            Arc::new(AcceptingDispatchSourceRefresher),
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

    #[tokio::test]
    async fn builds_resource_backed_hosted_workflow_service_before_sharing() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let gateway = Arc::new(inference::InferenceGateway::new());
        let input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            Some(1),
            1_000,
        )
        .with_workflow_service(WorkflowService::with_capacity_limits(2, 2));

        let shared = EmbeddedWorkflowServiceComposition::resource_backed_hosted(input)
            .expect("hosted resource-backed workflow service should build");

        drop(shared);
    }

    #[tokio::test]
    async fn hosted_resource_backed_factory_rejects_non_owner_pumas_access() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let pumas_api = pumas_library::PumasApi::builder(temp_dir.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("pumas api");
        pumas_api
            .rebuild_model_index()
            .await
            .expect("model index rebuild");
        let read_only = pumas_library::PumasReadOnlyLibrary::open(
            temp_dir.path().join("shared-resources/models"),
        )
        .expect("read-only pumas");
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let gateway = Arc::new(inference::InferenceGateway::new());
        let input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
            Some(1),
            1_000,
        );

        let error = match EmbeddedWorkflowServiceComposition::resource_backed_hosted(input) {
            Ok(_) => panic!("read-only access cannot build resource-backed hosted dispatch"),
            Err(error) => error,
        };

        assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
        assert!(error
            .to_string()
            .contains("requires Pumas owner selector access"));
    }

    #[tokio::test]
    async fn resource_backed_hosted_bundle_returns_service_and_lifecycle_handle() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let gateway = Arc::new(inference::InferenceGateway::new());
        let factory_input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            Some(1),
            1_000,
        )
        .with_workflow_service(WorkflowService::with_capacity_limits(2, 2));
        let input = EmbeddedHostedWorkflowServiceCompositionInput::new(
            factory_input,
            tokio::runtime::Handle::current(),
        );

        let output = EmbeddedWorkflowServiceComposition::resource_backed_hosted_bundle(input)
            .expect("hosted resource-backed bundle should build");

        assert!(Arc::strong_count(output.workflow_service()) >= 1);
        output
            .dependency_readiness_snapshot_producer
            .shutdown()
            .await;
    }

    #[tokio::test]
    async fn resource_backed_hosted_bundle_rejects_invalid_producer_config_before_sharing() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let gateway = Arc::new(inference::InferenceGateway::new());
        let factory_input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            Some(1),
            1_000,
        );
        let input = EmbeddedHostedWorkflowServiceCompositionInput::new(
            factory_input,
            tokio::runtime::Handle::current(),
        )
        .with_dependency_readiness_producer_config(
            EmbeddedDependencyReadinessSnapshotProducerConfig {
                poll_interval: std::time::Duration::ZERO,
            },
        );

        let error = match EmbeddedWorkflowServiceComposition::resource_backed_hosted_bundle(input) {
            Ok(_) => panic!("invalid producer config must reject hosted bundle"),
            Err(error) => error,
        };

        assert!(matches!(error, EmbeddedRuntimeError::Config { .. }));
        assert!(error
            .to_string()
            .contains("dependency-readiness snapshot producer poll interval"));
    }
}

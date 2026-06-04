use std::path::PathBuf;
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_runtime_host_contracts::{ReservationLifecyclePort, RuntimeHostExecutionPort};
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchSourceRefresher,
};
use pantograph_workflow_service::{
    WorkflowDependencyReadinessComponents, WorkflowService, WorkflowServiceError,
};
use workflow_nodes::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

use crate::inference_interface_facts_provider::EmbeddedInferenceInterfaceFactsProvider;
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
use crate::runtime_host_media_artifact_sink::WorkflowServiceRuntimeHostMediaArtifactSink;
use crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsResolver;
use crate::workflow_scheduler_diagnostics::EmbeddedWorkflowSchedulerDiagnosticsProvider;
use crate::SharedExtensions;
use crate::{
    runtime_registry::HostRuntimeRegistryController, DependencyActivityHub,
    EmbeddedDependencyReadinessSnapshotProducer, EmbeddedDependencyReadinessSnapshotProducerConfig,
    EmbeddedDependencyReadinessSnapshotProducerHandle, EmbeddedRuntimeError, SharedWorkflowService,
};

/// Builds embedded-runtime workflow services before sharing them across hosts.
///
/// This composition boundary keeps infrastructure wiring at the embedded
/// runtime edge. Workflow-service remains responsible for orchestration, while
/// this module owns the moment concrete dependency-readiness components are
/// attached before the service is wrapped in `Arc`.
#[derive(Default)]
pub struct EmbeddedWorkflowServiceComposition {
    dependency_readiness: WorkflowDependencyReadinessComponents,
    dispatch_dependencies: Option<EmbeddedWorkflowServiceDispatchDependencies>,
    scheduler_diagnostics_provider:
        Option<Arc<dyn pantograph_workflow_service::WorkflowSchedulerDiagnosticsProvider>>,
    inference_interface_facts_provider:
        Option<Arc<dyn pantograph_workflow_service::graph::InferenceInterfaceFactsProvider>>,
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

/// Source used by hosted startup composition to obtain Pumas selector access.
///
/// Hosts may provide an already-created selector access handle, or they may
/// delegate path-based setup to embedded-runtime so Pumas acquisition and owner
/// validation happen before workflow-service is shared.
pub enum EmbeddedHostedStartupPumasSelectorSource {
    Provided(Arc<PumasSelectorAccess>),
    SetupPath(Option<PathBuf>),
}

pub struct EmbeddedHostedStartupCompositionInput<C> {
    workflow_service: WorkflowService,
    max_loaded_sessions: Option<usize>,
    runtime_registry: SharedRuntimeRegistry,
    runtime_registry_controller: Arc<C>,
    gateway: Arc<inference::InferenceGateway>,
    pumas_selector_source: Option<EmbeddedHostedStartupPumasSelectorSource>,
    project_root: PathBuf,
    kv_cache_dir: PathBuf,
    dependency_readiness_runtime_handle: tokio::runtime::Handle,
    dependency_readiness_producer_config: EmbeddedDependencyReadinessSnapshotProducerConfig,
    max_dispatch_source_snapshot_age_ms: u64,
}

impl<C> EmbeddedHostedStartupCompositionInput<C> {
    #[must_use]
    pub fn new(
        runtime_registry: SharedRuntimeRegistry,
        runtime_registry_controller: Arc<C>,
        gateway: Arc<inference::InferenceGateway>,
        pumas_selector_source: Option<EmbeddedHostedStartupPumasSelectorSource>,
        project_root: PathBuf,
        kv_cache_dir: PathBuf,
        dependency_readiness_runtime_handle: tokio::runtime::Handle,
        max_loaded_sessions: Option<usize>,
        max_dispatch_source_snapshot_age_ms: u64,
    ) -> Self {
        Self {
            workflow_service: WorkflowService::new(),
            max_loaded_sessions,
            runtime_registry,
            runtime_registry_controller,
            gateway,
            pumas_selector_source,
            project_root,
            kv_cache_dir,
            dependency_readiness_runtime_handle,
            dependency_readiness_producer_config:
                EmbeddedDependencyReadinessSnapshotProducerConfig::default(),
            max_dispatch_source_snapshot_age_ms,
        }
    }

    #[must_use]
    pub fn with_workflow_service(mut self, workflow_service: WorkflowService) -> Self {
        self.workflow_service = workflow_service;
        self
    }

    #[must_use]
    pub fn with_dependency_readiness_producer_config(
        mut self,
        config: EmbeddedDependencyReadinessSnapshotProducerConfig,
    ) -> Self {
        self.dependency_readiness_producer_config = config;
        self
    }
}

pub struct EmbeddedHostedStartupCompositionOutput {
    pub workflow_service: SharedWorkflowService,
    pub shared_extensions: SharedExtensions,
    pub dependency_activity: Arc<DependencyActivityHub>,
    pub dependency_readiness_snapshot_producer: EmbeddedDependencyReadinessSnapshotProducerHandle,
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

    #[must_use]
    pub(crate) fn with_inference_interface_facts_provider(
        mut self,
        provider: Arc<dyn pantograph_workflow_service::graph::InferenceInterfaceFactsProvider>,
    ) -> Self {
        self.inference_interface_facts_provider = Some(provider);
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
        let artifact_writer = input.workflow_service.artifact_writer()?;
        let runtime_host_execution_port =
            Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(RuntimeHostPumasLoadTargetResolver::new(pumas_api.clone())),
                Arc::new(RuntimeHostPumasPackageFactsResolver::new(pumas_api)),
                Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                    artifact_writer,
                )),
                input.gateway.clone(),
            ));
        let reservation_lifecycle_port = Arc::new(EmbeddedReservationLifecyclePort::new(
            input.runtime_registry.clone(),
            input.runtime_registry_controller,
        ));
        let pumas_selector_access = input.pumas_selector_access;
        let dispatch_dependencies = EmbeddedWorkflowServiceDispatchDependencies::resource_backed(
            PumasDispatchPackageFactsSource::new(Some(pumas_selector_access.clone())),
            RuntimeDispatchCapabilityFactsSource::new(input.runtime_registry.clone()),
            RuntimeDispatchResourceFactsSource::new(input.runtime_registry.clone()),
            input.max_dispatch_source_snapshot_age_ms,
            runtime_host_execution_port,
            reservation_lifecycle_port,
        );
        let scheduler_diagnostics_provider =
            Arc::new(EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
                input.gateway.clone(),
                input.runtime_registry.clone(),
            ));
        let inference_interface_facts_provider =
            Arc::new(EmbeddedInferenceInterfaceFactsProvider::new(
                PumasDispatchPackageFactsSource::new(Some(pumas_selector_access)),
                RuntimeDispatchCapabilityFactsSource::new(input.runtime_registry),
            ));
        Self::new()
            .with_runtime_dispatch_dependencies(dispatch_dependencies)
            .with_scheduler_diagnostics_provider(scheduler_diagnostics_provider)
            .with_inference_interface_facts_provider(inference_interface_facts_provider)
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
        let artifact_writer =
            factory_input
                .workflow_service
                .artifact_writer()
                .map_err(|error| EmbeddedRuntimeError::Initialization {
                    message: error.to_string(),
                })?;
        let runtime_host_execution_port =
            Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(RuntimeHostPumasLoadTargetResolver::new(pumas_api.clone())),
                Arc::new(RuntimeHostPumasPackageFactsResolver::new(pumas_api)),
                Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                    artifact_writer,
                )),
                factory_input.gateway.clone(),
            ));
        let reservation_lifecycle_port = Arc::new(EmbeddedReservationLifecyclePort::new(
            factory_input.runtime_registry.clone(),
            factory_input.runtime_registry_controller,
        ));
        let pumas_selector_access = factory_input.pumas_selector_access;
        let dispatch_dependencies = EmbeddedWorkflowServiceDispatchDependencies::resource_backed(
            PumasDispatchPackageFactsSource::new(Some(pumas_selector_access.clone())),
            RuntimeDispatchCapabilityFactsSource::new(factory_input.runtime_registry.clone()),
            RuntimeDispatchResourceFactsSource::new(factory_input.runtime_registry.clone()),
            factory_input.max_dispatch_source_snapshot_age_ms,
            runtime_host_execution_port,
            reservation_lifecycle_port,
        );
        let scheduler_diagnostics_provider =
            Arc::new(EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
                factory_input.gateway,
                factory_input.runtime_registry.clone(),
            ));
        let inference_interface_facts_provider =
            Arc::new(EmbeddedInferenceInterfaceFactsProvider::new(
                PumasDispatchPackageFactsSource::new(Some(pumas_selector_access)),
                RuntimeDispatchCapabilityFactsSource::new(factory_input.runtime_registry),
            ));
        let composition = Self::new()
            .with_runtime_dispatch_dependencies(dispatch_dependencies)
            .with_scheduler_diagnostics_provider(scheduler_diagnostics_provider)
            .with_inference_interface_facts_provider(inference_interface_facts_provider);
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

    pub async fn resource_backed_hosted_startup<C>(
        input: EmbeddedHostedStartupCompositionInput<C>,
    ) -> Result<EmbeddedHostedStartupCompositionOutput, EmbeddedRuntimeError>
    where
        C: HostRuntimeRegistryController + Send + Sync + 'static,
    {
        let shared_extensions: SharedExtensions =
            Arc::new(tokio::sync::RwLock::new(ExecutorExtensions::new()));
        let pumas_selector_access = Self::initialize_hosted_startup_extensions(
            &shared_extensions,
            input.pumas_selector_source,
        )
        .await?;
        Self::require_owner_pumas_selector_access(pumas_selector_access.as_ref())?;

        let dependency_activity = Arc::new(DependencyActivityHub::default());
        {
            let kv_store = Arc::new(inference::kv_cache::KvCacheStore::new(
                input.kv_cache_dir,
                inference::kv_cache::StoragePolicy::MemoryAndDisk,
            ));
            let mut guard = shared_extensions.write().await;
            guard.set(node_engine::extension_keys::KV_CACHE_STORE, kv_store);
        }

        let factory_input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            input.runtime_registry,
            input.runtime_registry_controller,
            input.gateway,
            pumas_selector_access,
            input.max_loaded_sessions,
            input.max_dispatch_source_snapshot_age_ms,
        )
        .with_workflow_service(input.workflow_service);
        let composition_input = EmbeddedHostedWorkflowServiceCompositionInput::new(
            factory_input,
            input.dependency_readiness_runtime_handle,
        )
        .with_dependency_readiness_producer_config(input.dependency_readiness_producer_config);
        let output = Self::resource_backed_hosted_bundle(composition_input)?;

        Ok(EmbeddedHostedStartupCompositionOutput {
            workflow_service: output.workflow_service,
            shared_extensions,
            dependency_activity,
            dependency_readiness_snapshot_producer: output.dependency_readiness_snapshot_producer,
        })
    }

    async fn initialize_hosted_startup_extensions(
        shared_extensions: &SharedExtensions,
        source: Option<EmbeddedHostedStartupPumasSelectorSource>,
    ) -> Result<Arc<PumasSelectorAccess>, EmbeddedRuntimeError> {
        let Some(source) = source else {
            return Err(EmbeddedRuntimeError::Initialization {
                message: "hosted startup composition requires a Pumas selector source before workflow-service sharing".to_string(),
            });
        };
        {
            let mut guard = shared_extensions.write().await;
            match source {
                EmbeddedHostedStartupPumasSelectorSource::Provided(selector_access) => {
                    if let PumasSelectorAccess::Owner(api) = selector_access.as_ref() {
                        guard.set(node_engine::extension_keys::PUMAS_API, api.clone());
                    }
                    guard.set(PUMAS_SELECTOR_ACCESS, selector_access);
                }
                EmbeddedHostedStartupPumasSelectorSource::SetupPath(path) => {
                    workflow_nodes::setup_extensions_with_path(&mut guard, path.as_deref()).await;
                }
            }
        }

        let selector_access = {
            let guard = shared_extensions.read().await;
            guard
                .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
                .cloned()
        };
        match selector_access {
            Some(selector_access) => Ok(selector_access),
            None => Err(EmbeddedRuntimeError::Initialization {
                message: "hosted startup composition could not initialize Pumas selector access before workflow-service sharing".to_string(),
            }),
        }
    }

    fn require_owner_pumas_selector_access(
        selector_access: &PumasSelectorAccess,
    ) -> Result<(), EmbeddedRuntimeError> {
        match selector_access {
            PumasSelectorAccess::Owner(_) => Ok(()),
            PumasSelectorAccess::LocalClient(_) | PumasSelectorAccess::ReadOnly(_) => {
                Err(EmbeddedRuntimeError::Initialization {
                    message: format!(
                        "hosted startup composition requires Pumas owner selector access, got {} access",
                        selector_access.role_name()
                    ),
                })
            }
        }
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
        let service = match self.inference_interface_facts_provider {
            Some(provider) => service.with_graph_session_fact_providers(
                provider,
                self.dependency_readiness.snapshot_provider(),
            ),
            None => service,
        };
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
    use pantograph_inference_interface_contracts::{
        DependencyEnvironmentAction, DependencyEnvironmentActionIntent,
        DependencyEnvironmentActionIntentStatus, DraftGraphValidationStatus,
    };
    use pantograph_runtime_host_contracts::{
        ReservationLifecycleApplication, ReservationLifecycleEvent, ReservationLifecyclePortError,
        RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPortError,
        RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    };
    use pantograph_runtime_registry::{
        RuntimeRegistration, RuntimeRegistry, RuntimeTransition, SharedRuntimeRegistry,
    };
    use pantograph_scheduler::{SchedulerEstimateHintKind, SchedulerTaskStateRecord};
    use pantograph_workflow_service::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
    use pantograph_workflow_service::workflow::{
        WorkflowGraphSessionExecutableValidationSnapshotPublishRequest,
        WorkflowRuntimeDispatchSourceRefreshError, WorkflowSchedulerInferenceTaskProjection,
        WorkflowSchedulerTask,
    };
    use pantograph_workflow_service::{
        ArtifactPolicy, ArtifactStore, WorkflowGraphCurrentValidationRefreshRequest,
        WorkflowGraphEditSessionCreateRequest,
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
            _cancellation: RuntimeHostExecutionCancellationHandle,
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
        .with_workflow_service(workflow_service_with_artifact_store(&temp_dir));

        let shared = EmbeddedWorkflowServiceComposition::resource_backed_hosted(input)
            .expect("hosted resource-backed workflow service should build");

        drop(shared);
    }

    #[tokio::test]
    async fn resource_backed_hosted_service_refreshes_validation_from_production_facts() {
        let temp_dir = create_test_env();
        let model_id = "diffusion/imported/test-bundle";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_test_diffusers_bundle(&model_dir);
        write_imported_diffusion_metadata(&model_dir, model_id, &model_dir);
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("model index rebuild");
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string(), "diffusers".to_string()]),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-instance.001".to_string()),
                },
            )
            .expect("ready runtime transition");
        let gateway = Arc::new(inference::InferenceGateway::new());
        let input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            Some(1),
            1_000,
        )
        .with_workflow_service(workflow_service_with_artifact_store(&temp_dir));
        let service = EmbeddedWorkflowServiceComposition::resource_backed_hosted(input)
            .expect("hosted resource-backed workflow service should build");
        let session = service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: connected_inference_graph(model_id),
                workflow_id: None,
            })
            .await
            .expect("create graph edit session");
        let graph_revision = session
            .graph_revision
            .parse()
            .expect("valid session graph revision");

        let validation = service
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: session.session_id,
                    graph_revision,
                },
            )
            .await
            .expect("refresh validation summary");

        let summary = validation
            .summary
            .summary
            .expect("current validation summary");
        assert_eq!(summary.status, DraftGraphValidationStatus::Executable);
        let projection = validation
            .node_projections
            .first()
            .expect("inference node projection");
        assert_eq!(projection.node_id.as_str(), "infer");
        assert_eq!(projection.descriptor.task_kind.as_str(), "image_generation");
        assert_eq!(projection.descriptor.inputs[0].port_id.as_str(), "prompt");
        assert_eq!(projection.descriptor.outputs[0].port_id.as_str(), "image");
        assert!(projection
            .runtime_constraint
            .as_ref()
            .is_some_and(|runtime_id| runtime_id.as_str() == "pytorch"));
        assert!(projection.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakRamBytes && hint.value > 0
        }));
        assert!(projection.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakVramBytes && hint.value > 0
        }));
    }

    #[tokio::test]
    async fn resource_backed_hosted_service_publishes_executable_snapshot_from_production_facts() {
        let temp_dir = create_test_env();
        let model_id = "diffusion/imported/test-bundle";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_test_diffusers_bundle(&model_dir);
        write_imported_diffusion_metadata(&model_dir, model_id, &model_dir);
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("model index rebuild");
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string(), "diffusers".to_string()]),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-instance.001".to_string()),
                },
            )
            .expect("ready runtime transition");
        let gateway = Arc::new(inference::InferenceGateway::new());
        let input = EmbeddedHostedWorkflowServiceFactoryInput::new(
            registry,
            gateway.clone(),
            gateway,
            Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            Some(1),
            1_000,
        )
        .with_workflow_service(workflow_service_with_artifact_and_attribution_store(
            &temp_dir,
        ));
        let service = EmbeddedWorkflowServiceComposition::resource_backed_hosted(input)
            .expect("hosted resource-backed workflow service should build");
        let session = service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: connected_dependency_inference_graph(model_id),
                workflow_id: Some("resource-backed-inference".to_string()),
            })
            .await
            .expect("create graph edit session");
        let graph_revision = session
            .graph_revision
            .parse()
            .expect("valid session graph revision");

        let validation = service
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: session.session_id.clone(),
                    graph_revision,
                },
            )
            .await
            .expect("refresh validation summary");
        let validation_session_id = validation
            .summary
            .validation_session_id
            .clone()
            .expect("validation session id");
        let readiness = service
            .workflow_graph_resolve_dependency_environment_action_intent(
                DependencyEnvironmentActionIntent {
                    contract_version: 1,
                    graph_session_id: session.session_id.parse().expect("graph session id"),
                    graph_revision: session.graph_revision.parse().expect("graph revision"),
                    validation_session_id: Some(validation_session_id.clone()),
                    target_node_id: "dep-env".parse().expect("dependency node id"),
                    action: DependencyEnvironmentAction::Resolve,
                },
            )
            .await
            .expect("resolve dependency readiness");
        assert_eq!(
            readiness.status,
            DependencyEnvironmentActionIntentStatus::RequestReady,
            "dependency readiness resolution should be request-ready: {:?}",
            readiness.diagnostics
        );
        assert!(
            readiness.diagnostics.is_empty(),
            "dependency readiness diagnostics: {:?}",
            readiness.diagnostics
        );

        let snapshot = service
            .publish_graph_session_executable_validation_snapshot(
                WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                    workflow_id: "resource-backed-inference".to_string(),
                    workflow_semantic_version: "1.0.0".to_string(),
                    graph_session_id: session.session_id,
                    validation_session_id: Some(validation_session_id.clone()),
                    validation_snapshot_id: None,
                },
            )
            .await
            .expect("publish executable validation snapshot");

        let record = snapshot.as_record();
        assert_eq!(record.validation_session_id, validation_session_id);
        assert_eq!(
            record.validation_summary.status,
            DraftGraphValidationStatus::Executable
        );
        assert_eq!(record.nodes.len(), 1);
        let node = &record.nodes[0];
        assert_eq!(node.node_id.as_str(), "infer");
        assert_eq!(node.task_kind.as_str(), "image_generation");
        assert!(node
            .constraints
            .requested_runtime_id
            .as_ref()
            .is_some_and(|runtime_id| runtime_id.as_str() == "pytorch"));
        assert!(node.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakRamBytes && hint.value > 0
        }));
        assert!(node.estimate_hints.iter().any(|hint| {
            hint.kind == SchedulerEstimateHintKind::PeakVramBytes && hint.value > 0
        }));

        let projections = snapshot
            .scheduler_inference_task_projections()
            .expect("scheduler projections");
        let scheduler_node_id = "infer".parse().expect("scheduler node id");
        let projection = projections
            .get(&scheduler_node_id)
            .expect("scheduler inference projection");
        let WorkflowSchedulerInferenceTaskProjection::Ready(ready_projection) = projection else {
            panic!("published executable snapshot should project ready inference task");
        };
        assert_eq!(ready_projection.estimate_hints, node.estimate_hints);
        assert_eq!(
            ready_projection
                .dependency_readiness_source
                .validation_session_id
                .as_ref()
                .expect("projection validation session id")
                .as_str(),
            record.validation_session_id.as_str()
        );
        assert_eq!(
            ready_projection
                .dependency_readiness_source
                .validation_snapshot_id
                .as_ref()
                .expect("projection validation snapshot id")
                .as_str(),
            record.validation_snapshot_id.as_str()
        );
    }

    #[tokio::test]
    async fn hosted_resource_backed_factory_requires_artifact_writer_before_sharing() {
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

        let error = match EmbeddedWorkflowServiceComposition::resource_backed_hosted(input) {
            Ok(_) => panic!("missing artifact writer cannot build full runtime-host wiring"),
            Err(error) => error,
        };

        assert!(matches!(error, WorkflowServiceError::Internal(_)));
        assert!(error
            .to_string()
            .contains("artifact store is not configured"));
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
        .with_workflow_service(workflow_service_with_artifact_store(&temp_dir));
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
        )
        .with_workflow_service(workflow_service_with_artifact_store(&temp_dir));
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

    #[tokio::test]
    async fn hosted_startup_composition_returns_service_extensions_and_lifecycle_handle() {
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
        let input = EmbeddedHostedStartupCompositionInput::new(
            registry,
            gateway.clone(),
            gateway,
            Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(PumasSelectorAccess::Owner(pumas_api.clone())),
            )),
            temp_dir.path().to_path_buf(),
            temp_dir.path().join("kv-cache"),
            tokio::runtime::Handle::current(),
            Some(1),
            1_000,
        )
        .with_workflow_service(workflow_service_with_artifact_store(&temp_dir));

        let output = EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(input)
            .await
            .expect("hosted startup composition should build");

        assert!(Arc::strong_count(&output.workflow_service) >= 1);
        {
            let extensions = output.shared_extensions.read().await;
            let selector_access = extensions
                .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
                .expect("selector access should be installed");
            assert!(matches!(
                selector_access.as_ref(),
                PumasSelectorAccess::Owner(_)
            ));
            assert!(
                extensions
                    .get::<Arc<inference::kv_cache::KvCacheStore>>(
                        node_engine::extension_keys::KV_CACHE_STORE,
                    )
                    .is_some(),
                "kv cache store should be installed"
            );
        }
        let observed_activity = Arc::new(std::sync::Mutex::new(Vec::new()));
        output.dependency_activity.set_emitter(Arc::new({
            let observed_activity = observed_activity.clone();
            move |event| {
                observed_activity
                    .lock()
                    .expect("activity lock")
                    .push(event.phase);
            }
        }));
        output
            .dependency_activity
            .emit(crate::DependencyActivityEvent {
                timestamp: "2026-05-31T00:00:00Z".to_string(),
                node_type: "diagnostic".to_string(),
                target_node_id: Some("diagnostic-only".to_string()),
                phase: "observed".to_string(),
                message: "activity boundary".to_string(),
                binding_id: None,
                requirement_name: None,
                stream: None,
            });
        assert_eq!(
            observed_activity.lock().expect("activity lock").as_slice(),
            ["observed"]
        );
        output
            .dependency_readiness_snapshot_producer
            .shutdown()
            .await;
    }

    #[tokio::test]
    async fn hosted_startup_composition_rejects_missing_pumas_selector_source() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let gateway = Arc::new(inference::InferenceGateway::new());
        let input = EmbeddedHostedStartupCompositionInput::new(
            registry,
            gateway.clone(),
            gateway,
            None,
            temp_dir.path().to_path_buf(),
            temp_dir.path().join("kv-cache"),
            tokio::runtime::Handle::current(),
            Some(1),
            1_000,
        );

        let error =
            match EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(input).await {
                Ok(_) => panic!("missing selector source must reject hosted startup composition"),
                Err(error) => error,
            };

        assert!(matches!(error, EmbeddedRuntimeError::Initialization { .. }));
        assert!(error
            .to_string()
            .contains("requires a Pumas selector source"));
    }

    #[tokio::test]
    async fn hosted_startup_composition_rejects_non_owner_pumas_selector_access() {
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
        let input = EmbeddedHostedStartupCompositionInput::new(
            registry,
            gateway.clone(),
            gateway,
            Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
            )),
            temp_dir.path().to_path_buf(),
            temp_dir.path().join("kv-cache"),
            tokio::runtime::Handle::current(),
            Some(1),
            1_000,
        );

        let error =
            match EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(input).await {
                Ok(_) => panic!("read-only access cannot build hosted startup composition"),
                Err(error) => error,
            };

        assert!(matches!(error, EmbeddedRuntimeError::Initialization { .. }));
        assert!(error
            .to_string()
            .contains("requires Pumas owner selector access"));
    }

    #[tokio::test]
    async fn hosted_startup_composition_rejects_service_error_before_starting_sidecar() {
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
        let input = EmbeddedHostedStartupCompositionInput::new(
            registry,
            gateway.clone(),
            gateway,
            Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(PumasSelectorAccess::Owner(pumas_api)),
            )),
            temp_dir.path().to_path_buf(),
            temp_dir.path().join("kv-cache"),
            tokio::runtime::Handle::current(),
            Some(0),
            1_000,
        )
        .with_workflow_service(workflow_service_with_store(
            &temp_dir,
            WorkflowService::with_capacity_limits(2, 2),
        ));

        let error =
            match EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(input).await {
                Ok(_) => panic!("invalid workflow-service capacity must reject hosted startup"),
                Err(error) => error,
            };

        assert!(matches!(error, EmbeddedRuntimeError::Initialization { .. }));
        assert!(error.to_string().contains("invalid"));
    }

    fn workflow_service_with_artifact_store(temp_dir: &tempfile::TempDir) -> WorkflowService {
        workflow_service_with_store(temp_dir, WorkflowService::with_capacity_limits(2, 2))
    }

    fn workflow_service_with_artifact_and_attribution_store(
        temp_dir: &tempfile::TempDir,
    ) -> WorkflowService {
        workflow_service_with_store(
            temp_dir,
            WorkflowService::with_ephemeral_attribution_store().expect("workflow service"),
        )
    }

    fn create_test_env() -> tempfile::TempDir {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
        temp_dir
    }

    fn connected_inference_graph(model_id: &str) -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "model".to_string(),
                    node_type: "puma-lib".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({
                        "pumas_model_ref": {
                            "model_id": model_id,
                            "selected_artifact_id": "diffusers"
                        }
                    }),
                },
                GraphNode {
                    id: "infer".to_string(),
                    node_type: "llm-inference".to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: serde_json::json!({
                        "task_kind": "image_generation",
                        "runtime": "pytorch"
                    }),
                },
            ],
            edges: vec![GraphEdge {
                id: "model-to-infer".to_string(),
                source: "model".to_string(),
                source_handle: "pumas_model_ref".to_string(),
                target: "infer".to_string(),
                target_handle: "pumas_model_ref".to_string(),
            }],
            derived_graph: None,
        }
    }

    fn connected_dependency_inference_graph(model_id: &str) -> WorkflowGraph {
        let mut graph = connected_inference_graph(model_id);
        graph.nodes.push(GraphNode {
            id: "dep-env".to_string(),
            node_type: "dependency-environment".to_string(),
            position: Position { x: 400.0, y: 0.0 },
            data: serde_json::json!({
                "mode": "manual"
            }),
        });
        graph.edges.push(GraphEdge {
            id: "dep-env-to-infer".to_string(),
            source: "dep-env".to_string(),
            source_handle: "dependency_environment_sidecar".to_string(),
            target: "infer".to_string(),
            target_handle: "dependency_environment_sidecar".to_string(),
        });
        graph
    }

    fn write_test_diffusers_bundle(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("scheduler")).unwrap();
        std::fs::create_dir_all(root.join("text_encoder")).unwrap();
        std::fs::create_dir_all(root.join("tokenizer")).unwrap();
        std::fs::create_dir_all(root.join("unet")).unwrap();
        std::fs::create_dir_all(root.join("vae")).unwrap();
        std::fs::write(
            root.join("model_index.json"),
            serde_json::json!({
                "_class_name": "StableDiffusionPipeline",
                "_diffusers_version": "0.32.0",
                "_name_or_path": "synthetic/tiny-sd",
                "scheduler": ["diffusers", "EulerDiscreteScheduler"],
                "text_encoder": ["transformers", "CLIPTextModel"],
                "tokenizer": ["transformers", "CLIPTokenizer"],
                "unet": ["diffusers", "UNet2DConditionModel"],
                "vae": ["diffusers", "AutoencoderKL"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_imported_diffusion_metadata(
        model_dir: &std::path::Path,
        model_id: &str,
        entry_path: &std::path::Path,
    ) {
        std::fs::create_dir_all(model_dir).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": model_id,
                "family": "imported",
                "model_type": "diffusion",
                "official_name": "test-bundle",
                "cleaned_name": "test-bundle",
                "source_path": entry_path.display().to_string(),
                "entry_path": entry_path.display().to_string(),
                "storage_kind": "external_reference",
                "bundle_format": "diffusers_directory",
                "pipeline_class": "StableDiffusionPipeline",
                "selected_artifact_id": "diffusers",
                "import_state": "ready",
                "validation_state": "valid",
                "pipeline_tag": "text-to-image",
                "task_type_primary": "text-to-image",
                "input_modalities": ["text"],
                "output_modalities": ["image"],
                "task_classification_source": "external-diffusers-import",
                "task_classification_confidence": 1.0,
                "model_type_resolution_source": "external-diffusers-import",
                "model_type_resolution_confidence": 1.0,
                "recommended_backend": "diffusers",
                "runtime_engine_hints": ["diffusers", "pytorch"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn workflow_service_with_store(
        temp_dir: &tempfile::TempDir,
        workflow_service: WorkflowService,
    ) -> WorkflowService {
        let artifact_store = ArtifactStore::open(
            temp_dir.path().join("workflow-artifacts"),
            artifact_policy(),
        )
        .expect("open artifact store");
        workflow_service.with_artifact_store(artifact_store)
    }

    fn artifact_policy() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "workflow-service-composition-test".to_string(),
            policy_version: 1,
            ttl_seconds: None,
            max_disk_bytes: None,
            max_memory_bytes: None,
            max_single_artifact_bytes: None,
            spill_threshold_bytes: None,
            delete_on_consume: false,
        }
    }
}

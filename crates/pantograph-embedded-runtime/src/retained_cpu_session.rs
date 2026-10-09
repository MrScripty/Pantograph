//! Explicit already-warm CPU session composition. No preload, target repair or
//! physical peak-resource guarantee is supplied by this constructor.
use crate::{
    EmbeddedRetainedCpuSerialPort, EmbeddedRuntime, EmbeddedRuntimeConfig, SharedExtensions,
    SharedWorkflowService,
};
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeReservationRequest, RuntimeReservationResourceClaim,
    RuntimeRetentionHint, SharedRuntimeRegistry,
};
use pantograph_workflow_service::workflow::{
    WorkflowSerialReadyConfig, WorkflowSessionExecutionRuntime,
};
use pantograph_workflow_service::{
    WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCloseResponse,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionCreateResponse,
    WorkflowExecutionSessionRunRequest, WorkflowHost, WorkflowRunResponse, WorkflowService,
    WorkflowServiceError,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) type SessionReservations = Arc<Mutex<HashMap<String, u64>>>;

/// Exact already-loaded model and an explicit operator RAM declaration for each
/// retaining session. This is a ledger floor, not a measured physical envelope;
/// available ordinary estimates are preserved when they are larger.
pub struct EmbeddedRetainedCpuSessionConfig {
    model_ref: inference::PumasModelRef,
    retaining_ram_bytes: u64,
    serial_ready: WorkflowSerialReadyConfig,
}
impl EmbeddedRetainedCpuSessionConfig {
    pub fn new(
        model_ref: inference::PumasModelRef,
        retaining_ram_bytes: u64,
        serial_ready: WorkflowSerialReadyConfig,
    ) -> Result<Self, WorkflowServiceError> {
        if retaining_ram_bytes == 0
            || !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(&model_ref)
            || model_ref.model_id.trim().is_empty()
        {
            return Err(refused(
                "bounded exact model and positive declared retaining RAM required",
            ));
        }
        Ok(Self {
            model_ref,
            retaining_ram_bytes,
            serial_ready,
        })
    }
}
pub(crate) struct RetainedCpuSessionProfile {
    model_ref: inference::PumasModelRef,
    retaining_ram_bytes: u64,
}
impl RetainedCpuSessionProfile {
    fn check_owner(
        &self,
        gateway: &inference::InferenceGateway,
    ) -> Result<(), WorkflowServiceError> {
        if gateway.resident_cpu_serial_owner(&self.model_ref).is_none() {
            return Err(refused(
                "exact qualified warm Candle CPU owner unavailable; preload disabled",
            ));
        }
        Ok(())
    }
    pub(crate) async fn apply_request(
        &self,
        host: &crate::EmbeddedWorkflowHost,
        workflow_id: &str,
        request: &mut RuntimeReservationRequest,
    ) -> Result<(), WorkflowServiceError> {
        self.check_owner(&host.gateway)?;
        if request.runtime_id != "candle"
            || request.retention_hint != RuntimeRetentionHint::KeepAlive
        {
            return Err(refused(
                "retained CPU sessions require Candle and KeepAlive",
            ));
        }
        let graph = host.workflow_graph(workflow_id).await?;
        if graph.nodes.len() > 64
            || graph.edges.len() > 128
            || !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(&graph)
        {
            return Err(refused("retained workflow graph bounds refused"));
        }
        let contracts = workflow_nodes::builtin_node_contracts()
            .map_err(|_| refused("canonical node contracts unavailable"))?;
        let mut inference_nodes = 0;
        for node in &graph.nodes {
            let contract = contracts
                .iter()
                .find(|contract| contract.node_type.as_str() == node.node_type);
            use pantograph_workflow_service::workflow::WorkflowSchedulerTaskExecutionClass;
            match pantograph_workflow_service::workflow::classify_workflow_scheduler_task(
                &node.node_type,
                contract,
            ) {
                WorkflowSchedulerTaskExecutionClass::RuntimeInference => {}
                WorkflowSchedulerTaskExecutionClass::SourceInput
                | WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => continue,
                _ => {
                    return Err(refused(
                        "retained workflow contains unsupported node execution",
                    ))
                }
            }
            inference_nodes += 1;
            let data = &node.data;
            if data.get("task_kind").and_then(|v| v.as_str()) != Some("embedding")
                || data.get("runtime").and_then(|v| v.as_str()) != Some("candle")
                || data.get("device").and_then(|v| v.as_str()) != Some("cpu")
            {
                return Err(refused(
                    "retained CPU workflow requires explicit Candle CPU embedding nodes",
                ));
            }
            let reference = data
                .get("pumas_model_ref")
                .ok_or_else(|| refused("workflow model reference missing"))?;
            if !inference::gateway::CandleCpuWarmAttemptRequest::metadata_is_bounded(reference) {
                return Err(refused("workflow model metadata bounds refused"));
            }
            let reference: inference::PumasModelRef = serde_json::from_value(reference.clone())
                .map_err(|_| refused("workflow model reference invalid"))?;
            if reference != self.model_ref {
                return Err(refused("workflow does not match the exact warm CPU model"));
            }
        }
        if inference_nodes == 0 {
            return Err(refused("retained CPU workflow has no embedding node"));
        }
        // Recheck after awaited graph access. Logical identity is proven by the
        // exact opaque owner query; the physical resident target is unchanged.
        self.check_owner(&host.gateway)?;
        request.model_id = Some(self.model_ref.model_id.clone());
        let requirements = request.requirements.get_or_insert_with(Default::default);
        if requirements.claims.len() > 2 {
            return Err(refused("session claim bounds refused"));
        }
        if let Some(ram) = requirements
            .claims
            .iter_mut()
            .find(|claim| claim.kind == RuntimeAdmissionResourceKind::RamBytes)
        {
            ram.bytes = ram.bytes.max(self.retaining_ram_bytes);
        } else {
            requirements
                .claims
                .push(RuntimeReservationResourceClaim::ram_bytes(
                    self.retaining_ram_bytes,
                ));
        }
        Ok(())
    }
}

/// Dedicated serial construction. Ordinary hosted startup is not reused, since
/// it replaces the constructor-owned batch/lifecycle ports.
pub struct EmbeddedRetainedCpuSessionComposition {
    port: Arc<EmbeddedRetainedCpuSerialPort>,
    service: WorkflowService,
    reservations: SessionReservations,
    profile: Arc<RetainedCpuSessionProfile>,
    selector: Option<Arc<workflow_nodes::setup::PumasSelectorAccess>>,
}
impl EmbeddedRetainedCpuSessionComposition {
    pub fn new(
        gateway: Arc<inference::InferenceGateway>,
        registry: SharedRuntimeRegistry,
        selector: Arc<workflow_nodes::setup::PumasSelectorAccess>,
        config: EmbeddedRetainedCpuSessionConfig,
    ) -> Result<Self, WorkflowServiceError> {
        let mut composition = Self::from_port(
            EmbeddedRetainedCpuSerialPort::new(gateway, registry, selector.clone()),
            config,
        )?;
        composition.selector = Some(selector);
        Ok(composition)
    }
    pub(crate) fn from_port(
        mut port: EmbeddedRetainedCpuSerialPort,
        config: EmbeddedRetainedCpuSessionConfig,
    ) -> Result<Self, WorkflowServiceError> {
        #[cfg(feature = "native-task-release")]
        if port.release_observer.is_some() {
            return Err(refused(
                "legacy protected release observer unsupported for declared sessions",
            ));
        }
        let profile = Arc::new(RetainedCpuSessionProfile {
            model_ref: config.model_ref,
            retaining_ram_bytes: config.retaining_ram_bytes,
        });
        profile.check_owner(&port.gateway)?;
        let reservations = Arc::new(Mutex::new(HashMap::new()));
        port.session_reservations = Some(reservations.clone());
        let port = Arc::new(port);
        let service = WorkflowService::new_serial_ready_cpu(port.clone(), config.serial_ready);
        Ok(Self {
            port,
            service,
            reservations,
            profile,
            selector: None,
        })
    }
    /// Configure saved graph/readiness/candidates/artifacts before sharing. A
    /// callback replacing the actual serial service owner is rejected.
    pub fn configure_service(
        mut self,
        configure: impl FnOnce(WorkflowService) -> WorkflowService,
    ) -> Result<Self, WorkflowServiceError> {
        self.service = self.service.configure_serial_ready_cpu(configure)?;
        Ok(self)
    }
    /// Install the SAME constructor selector into host extensions. Owner API
    /// identity is kept with it; conflicting or busy extensions refuse. A
    /// selector lacking full Owner facts still cannot pass ordinary preflight.
    pub fn into_runtime(
        self,
        config: EmbeddedRuntimeConfig,
        extensions: SharedExtensions,
    ) -> Result<EmbeddedRetainedCpuSessionRuntime, WorkflowServiceError> {
        self.profile.check_owner(&self.port.gateway)?;
        self.service
            .set_loaded_runtime_capacity_limit(config.max_loaded_sessions)?;
        if let Some(selector) = &self.selector {
            let mut guard = extensions
                .try_write()
                .map_err(|_| refused("host extensions busy"))?;
            let key = workflow_nodes::setup::PUMAS_SELECTOR_ACCESS;
            if guard.has(key)
                && !guard
                    .get::<Arc<workflow_nodes::setup::PumasSelectorAccess>>(key)
                    .is_some_and(|existing| Arc::ptr_eq(existing, selector))
            {
                return Err(refused("host Pumas selector ownership conflict"));
            }
            let api_key = node_engine::extension_keys::PUMAS_API;
            if let workflow_nodes::setup::PumasSelectorAccess::Owner(api) = selector.as_ref() {
                if guard.has(api_key)
                    && !guard
                        .get::<Arc<pumas_library::PumasApi>>(api_key)
                        .is_some_and(|existing| Arc::ptr_eq(existing, api))
                {
                    return Err(refused("host Pumas API ownership conflict"));
                }
                guard.set(api_key, api.clone());
            } else if guard.has(api_key) {
                return Err(refused("non-Owner selector conflicts with host Pumas API"));
            }
            guard.set(key, selector.clone());
        }
        let mut runtime = EmbeddedRuntime::from_hosted_composition(
            config,
            self.port.gateway.clone(),
            extensions,
            Arc::new(self.service),
            None,
            Some(self.port.registry.clone()),
            None,
        );
        runtime.session_runtime_reservations = self.reservations;
        runtime.retained_cpu_session = Some(self.profile);
        let execution = WorkflowSessionExecutionRuntime::from_shared_service(
            runtime.workflow_service.clone(),
            Arc::new(runtime.host()),
        );
        Ok(EmbeddedRetainedCpuSessionRuntime { runtime, execution })
    }
}

/// Owns one persistent public submission facade and supervised Worker. Call
/// shutdown explicitly; actual Worker drain precedes producer stop.
pub struct EmbeddedRetainedCpuSessionRuntime {
    pub(crate) runtime: EmbeddedRuntime,
    execution: WorkflowSessionExecutionRuntime,
}
impl EmbeddedRetainedCpuSessionRuntime {
    pub fn workflow_service(&self) -> &SharedWorkflowService {
        &self.runtime.workflow_service
    }
    pub async fn create_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        if !request.keep_alive {
            return Err(refused(
                "explicit retained CPU composition requires KeepAlive",
            ));
        }
        self.runtime
            .create_workflow_execution_session(request)
            .await
    }
    pub async fn run_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if self.runtime.workflow_service.serial_ready_cpu_is_poisoned() {
            return Err(refused(
                "serial admission is poisoned; verified recovery required",
            ));
        }
        let profile = self
            .runtime
            .retained_cpu_session
            .as_ref()
            .ok_or_else(|| refused("retained profile missing"))?;
        profile.check_owner(&self.runtime.gateway)?;
        if request.session_id.len() > 128 || request.session_id.trim().is_empty() {
            return Err(refused("session identity bounds refused"));
        }
        let session = self
            .runtime
            .workflow_service
            .workflow_get_execution_session_status(
                pantograph_workflow_service::WorkflowExecutionSessionStatusRequest {
                    session_id: request.session_id.clone(),
                },
            )
            .await?
            .session;
        if !session.keep_alive
            || !matches!(
                session.state,
                pantograph_workflow_service::WorkflowExecutionSessionState::IdleLoaded
                    | pantograph_workflow_service::WorkflowExecutionSessionState::Running
            )
        {
            return Err(refused(
                "actual retained session is not loaded and KeepAlive",
            ));
        }
        let registry = self
            .runtime
            .runtime_registry
            .as_ref()
            .ok_or_else(|| refused("registry missing"))?;
        // Routing lookup only. The service validates the actual active run and
        // attempts; the native envelope rechecks the exact captured incarnation.
        let id = *self
            .runtime
            .session_runtime_reservations
            .lock()
            .map_err(|_| refused("session map poisoned"))?
            .get(&request.session_id)
            .ok_or_else(|| refused("owned retaining session lease missing"))?;
        let lease = registry
            .reservation_lease(id)
            .ok_or_else(|| refused("retaining session incarnation missing"))?;
        if lease.reservation_owner_id.as_deref() != Some(request.session_id.as_str())
            || lease.retention_hint != RuntimeRetentionHint::KeepAlive
            || lease.workflow_id != session.workflow_id
            || lease.runtime_id != "candle"
            || lease.model_id.as_deref() != Some(profile.model_ref.model_id.as_str())
        {
            return Err(refused("session retaining ownership changed"));
        }
        self.execution.run_workflow_execution_session(request).await
    }
    pub async fn close_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionCloseRequest,
    ) -> Result<WorkflowExecutionSessionCloseResponse, WorkflowServiceError> {
        self.runtime.close_workflow_execution_session(request).await
    }
    pub async fn shutdown(
        &self,
        drain_timeout: Duration,
        abort_timeout: Duration,
    ) -> Result<(), WorkflowServiceError> {
        self.execution
            .shutdown_workflow_execution_runtime(drain_timeout, abort_timeout)
            .await?;
        self.runtime
            .shutdown()
            .await
            .map_err(|error| WorkflowServiceError::Internal(error.to_string()))
    }
}
fn refused(message: &str) -> WorkflowServiceError {
    WorkflowServiceError::RuntimeNotReady(message.into())
}

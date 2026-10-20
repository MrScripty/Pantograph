//! Dedicated composition owner for opt-in serial CPU residency.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pantograph_diagnostics_ledger::SqliteDiagnosticsLedger;
use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeModelResidentEstimate,
    RuntimeRegistration, RuntimeRegistry, RuntimeRegistrySnapshot, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim,
};
use pantograph_workflow_service::workflow::{
    WorkflowSchedulerTaskStateReadModelQueryRequest,
    WorkflowSchedulerTaskStateReadModelQueryResponse, WorkflowSerialReadyConfig,
    WorkflowSessionExecutionRuntime,
};
use pantograph_workflow_service::*;

use crate::runtime_host_load_target::RuntimeHostPumasLoadTargetResolver;
use crate::serial_cpu_port::CpuResidencyDeclaration;
use crate::{
    EmbeddedDependencyReadinessSnapshotProducerConfig, EmbeddedHostedStartupCompositionInput,
    EmbeddedHostedStartupConfig, EmbeddedHostedStartupPumasSelectorSource,
    EmbeddedRetainedCpuSerialPort, EmbeddedRuntime, EmbeddedRuntimeConfig, EmbeddedRuntimeError,
    EmbeddedWorkflowServiceComposition, SerialCpuResidencyDiagnostics,
};

/// Operator-declared resource bounds for one Pumas model/artifact identity.
/// These are capacity declarations, not physical allocator measurements.
#[derive(Debug, Clone)]
pub struct EmbeddedSerialCpuModelDeclaration {
    pub model_ref: pantograph_dependency_planning::PumasModelRef,
    pub resident_ram_bytes: u64,
    pub load_peak_ram_bytes: u64,
    pub load_peak_vram_bytes: u64,
}

/// Explicit construction of a private calibrated gateway and registry.
/// The Pumas root must be available for a dedicated owner; no native bootstrap
/// or model download is performed during construction.
pub struct EmbeddedSerialCpuResidencyConfig {
    pub runtime: EmbeddedRuntimeConfig,
    pub pumas_library_root: PathBuf,
    pub models: Vec<EmbeddedSerialCpuModelDeclaration>,
    pub ram_capacity_bytes: u64,
    pub vram_capacity_bytes: u64,
    pub ram_safety_margin_bytes: u64,
    pub vram_safety_margin_bytes: u64,
    pub cpu_threads: usize,
    pub readiness_poll_interval: Duration,
}

impl EmbeddedSerialCpuResidencyConfig {
    fn validate(&self) -> Result<(), EmbeddedRuntimeError> {
        if self.models.is_empty()
            || self.models.len() > 8
            || self.ram_capacity_bytes == 0
            || self.ram_capacity_bytes > i64::MAX as u64
            || self.vram_capacity_bytes > i64::MAX as u64
            || self.ram_safety_margin_bytes > self.ram_capacity_bytes
            || self.vram_safety_margin_bytes > self.vram_capacity_bytes
            || !(1..=4).contains(&self.cpu_threads)
            || self.readiness_poll_interval.is_zero()
            || self
                .runtime
                .max_loaded_sessions
                .is_some_and(|count| !(1..=8).contains(&count))
        {
            return Err(config_error("bounded models, explicit resource budgets, 1..4 CPU threads and 1..8 loaded sessions required"));
        }
        for (index, model) in self.models.iter().enumerate() {
            model
                .model_ref
                .validate()
                .map_err(|error| config_error(&error.to_string()))?;
            if model.resident_ram_bytes == 0
                || model.resident_ram_bytes > i64::MAX as u64
                || model.load_peak_ram_bytes == 0
                || model.load_peak_ram_bytes > i64::MAX as u64
                || model.load_peak_vram_bytes > i64::MAX as u64
                || self.models[..index]
                    .iter()
                    .any(|other| other.model_ref == model.model_ref)
            {
                return Err(config_error("nonduplicate models with known bounded resident and load-peak declarations required"));
            }
        }
        Ok(())
    }
}

fn config_error(message: &str) -> EmbeddedRuntimeError {
    EmbeddedRuntimeError::Config {
        message: message.to_owned(),
    }
}
fn initialization_error(error: impl std::fmt::Display) -> EmbeddedRuntimeError {
    EmbeddedRuntimeError::Initialization {
        message: error.to_string(),
    }
}
fn closed_error() -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest("serial CPU residency admission is closed".into())
}

fn require_execution_inventory() -> Result<(), WorkflowServiceError> {
    if !cfg!(feature = "host-dependency-inventory") {
        return Err(WorkflowServiceError::InvalidRequest(
            "serial CPU residency execution requires host-dependency-inventory".into(),
        ));
    }
    Ok(())
}

/// The only public owner of this residency composition. Methods return values;
/// no gateway, host, registry, service, extensions or inner-runtime alias escapes.
/// All run/resume calls share one supervised execution owner through shutdown.
pub struct EmbeddedSerialCpuResidency {
    runtime: Arc<EmbeddedRuntime>,
    execution: Arc<WorkflowSessionExecutionRuntime>,
    registry: Arc<RuntimeRegistry>,
    port: Arc<EmbeddedRetainedCpuSerialPort>,
    closed: AtomicBool,
    operations: Arc<tokio::sync::RwLock<()>>,
    shutdown_state: tokio::sync::Mutex<ShutdownState>,
}

#[derive(Default)]
struct ShutdownState {
    task: Option<tokio::task::JoinHandle<Result<(), WorkflowServiceError>>>,
    result: Option<Result<(), String>>,
}

impl EmbeddedSerialCpuResidency {
    pub async fn new(
        mut config: EmbeddedSerialCpuResidencyConfig,
    ) -> Result<Self, EmbeddedRuntimeError> {
        config.validate()?;
        config.runtime.max_loaded_sessions = Some(config.runtime.max_loaded_sessions.unwrap_or(8));
        let calibration = inference::CandleCpuCalibrationConfig::new(
            config.cpu_threads,
            3,
            Duration::from_secs(60),
        )
        .map_err(initialization_error)?;
        let gateway = Arc::new(
            inference::InferenceGateway::new_calibrated_candle_cpu(calibration)
                .map_err(initialization_error)?,
        );
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("candle", "Serial Candle CPU")
                .with_backend_keys(vec!["candle".into()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(config.ram_capacity_bytes))
                        .with_safety_margin_bytes(config.ram_safety_margin_bytes),
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(config.vram_capacity_bytes))
                        .with_safety_margin_bytes(config.vram_safety_margin_bytes),
                ])),
        );
        let owner = Arc::new(
            pumas_library::PumasApi::builder(&config.pumas_library_root)
                .auto_create_dirs(true)
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .map_err(initialization_error)?,
        );
        owner
            .rebuild_model_index()
            .await
            .map_err(initialization_error)?;
        let selector = Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(owner));
        let targets = RuntimeHostPumasLoadTargetResolver::new(selector.clone());
        let mut declarations = Vec::with_capacity(config.models.len());
        for model in config.models {
            let target = targets
                .resolve_for_model(&model.model_ref, "embedding", "candle.cpu")
                .await
                .map_err(initialization_error)?;
            declarations.push(CpuResidencyDeclaration {
                target: crate::runtime_host_image_execution::project_pumas_artifact_load_target(
                    target,
                ),
                resident_ram_bytes: model.resident_ram_bytes,
                load_peak_ram_bytes: model.load_peak_ram_bytes,
                load_peak_vram_bytes: model.load_peak_vram_bytes,
            });
        }
        registry
            .configure_model_resident_estimates(
                declarations
                    .iter()
                    .map(|model| RuntimeModelResidentEstimate {
                        runtime_id: "candle".into(),
                        model_id: model.target.local_load_path.clone(),
                        requirements: RuntimeReservationRequirements::from_claims(vec![
                            RuntimeReservationResourceClaim::ram_bytes(model.resident_ram_bytes),
                            RuntimeReservationResourceClaim::vram_bytes(0),
                        ]),
                    })
                    .collect(),
            )
            .map_err(initialization_error)?;
        let port = Arc::new(
            EmbeddedRetainedCpuSerialPort::new_residency(
                gateway.clone(),
                registry.clone(),
                selector.clone(),
                declarations,
            )
            .map_err(initialization_error)?,
        );
        std::fs::create_dir_all(&config.runtime.app_data_dir).map_err(initialization_error)?;
        let service = WorkflowService::new_serial_ready_cpu(
            port.clone(),
            WorkflowSerialReadyConfig::default()
                .with_fair_service_opportunities()
                .with_serial_residency()
                .map_err(initialization_error)?,
        )
        .with_attribution_store(
            pantograph_runtime_attribution::SqliteAttributionStore::open_in_memory()
                .map_err(initialization_error)?,
        )
        .with_diagnostics_ledger(
            SqliteDiagnosticsLedger::open(
                config
                    .runtime
                    .app_data_dir
                    .join("serial-cpu-diagnostics.db"),
            )
            .map_err(initialization_error)?,
        )
        .with_artifact_store(
            ArtifactStore::open(
                config.runtime.app_data_dir.join("serial-cpu-artifacts"),
                ArtifactPolicy {
                    policy_id: "serial-cpu-residency".into(),
                    policy_version: 1,
                    ttl_seconds: None,
                    max_disk_bytes: Some(1024 * 1024),
                    max_memory_bytes: Some(1024 * 1024),
                    max_single_artifact_bytes: Some(65536),
                    spill_threshold_bytes: None,
                    delete_on_consume: false,
                },
            )
            .map_err(initialization_error)?,
        );
        let input = EmbeddedHostedStartupCompositionInput::new(EmbeddedHostedStartupConfig {
            runtime_registry: registry.clone(),
            runtime_registry_controller: gateway.clone(),
            gateway: gateway.clone(),
            pumas_selector_source: Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                selector,
            )),
            project_root: config.runtime.project_root.clone(),
            kv_cache_dir: config.runtime.app_data_dir.join("serial-cpu-kv-cache"),
            dependency_readiness_runtime_handle: tokio::runtime::Handle::current(),
            max_loaded_sessions: config.runtime.max_loaded_sessions,
            max_dispatch_source_snapshot_age_ms: 1000,
        })
        .with_workflow_service(service)
        .with_dependency_readiness_producer_config(
            EmbeddedDependencyReadinessSnapshotProducerConfig {
                poll_interval: config.readiness_poll_interval,
            },
        );
        #[cfg(feature = "host-dependency-inventory")]
        let input =
            input.with_dependency_inventory_app_data_dir(config.runtime.app_data_dir.clone());
        let output =
            EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(input).await?;
        let runtime = Arc::new(
            EmbeddedRuntime::from_hosted_composition(
                config.runtime,
                gateway,
                output.shared_extensions,
                output.workflow_service,
                None,
                Some(registry.clone()),
                None,
            )
            .with_dependency_readiness_snapshot_producer(
                output.dependency_readiness_snapshot_producer,
            ),
        );
        let execution = Arc::new(WorkflowSessionExecutionRuntime::from_shared_service(
            runtime.workflow_service.clone(),
            Arc::new(runtime.host()),
        ));
        Ok(Self {
            runtime,
            execution,
            registry,
            port,
            closed: AtomicBool::new(false),
            operations: Arc::new(tokio::sync::RwLock::new(())),
            shutdown_state: tokio::sync::Mutex::new(ShutdownState::default()),
        })
    }

    async fn admit(&self) -> Result<tokio::sync::RwLockReadGuard<'_, ()>, WorkflowServiceError> {
        let guard = self.operations.read().await;
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        Ok(guard)
    }
    fn admit_now(&self) -> Result<tokio::sync::RwLockReadGuard<'_, ()>, WorkflowServiceError> {
        let guard = self.operations.try_read().map_err(|_| closed_error())?;
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        Ok(guard)
    }

    pub fn register_attribution_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime.register_attribution_client(request)
    }
    pub fn open_client_session(
        &self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime.open_client_session(request)
    }
    pub fn create_client_bucket(
        &self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime.create_client_bucket(request)
    }
    pub async fn create_attributed_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionAttributedCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        if request.keep_alive {
            return Err(WorkflowServiceError::InvalidRequest(
                "serial CPU residency refuses KeepAlive before session creation".into(),
            ));
        }
        self.runtime
            .create_attributed_workflow_execution_session(request)
            .await
    }
    /// Physical execution requires host dependency inventory. Construction and
    /// metadata/projection operations remain available without that feature.
    pub async fn run_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        require_execution_inventory()?;
        self.execution.run_workflow_execution_session(request).await
    }
    pub async fn resume_workflow_execution_session_runtime_dependency_readiness(
        &self,
        request: WorkflowExecutionSessionResumeRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        require_execution_inventory()?;
        self.execution
            .resume_workflow_execution_session_runtime_dependency_readiness(request)
            .await
    }
    pub fn workflow_execution_session_runtime_dependency_readiness_resume_candidates(
        &self,
    ) -> Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime
            .workflow_service
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
    }
    pub async fn close_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionCloseRequest,
    ) -> Result<WorkflowExecutionSessionCloseResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime.close_workflow_execution_session(request).await
    }
    pub async fn workflow_get_execution_session_status(
        &self,
        request: WorkflowExecutionSessionStatusRequest,
    ) -> Result<WorkflowExecutionSessionStatusResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_get_execution_session_status(request)
            .await
    }
    pub async fn workflow_get_scheduler_snapshot(
        &self,
        request: WorkflowSchedulerSnapshotRequest,
    ) -> Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_service
            .workflow_get_scheduler_snapshot(request)
            .await
    }
    pub async fn workflow_cancel_active_execution_session_task(
        &self,
        request: WorkflowExecutionSessionActiveTaskCancelRequest,
    ) -> Result<WorkflowExecutionSessionActiveTaskCancelResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_service
            .workflow_cancel_active_execution_session_task(request)
            .await
    }
    pub fn workflow_graph_save(
        &self,
        request: WorkflowGraphSaveRequest,
    ) -> Result<WorkflowGraphSaveResponse, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime.workflow_graph_save(request)
    }
    pub async fn workflow_graph_create_edit_session(
        &self,
        request: WorkflowGraphEditSessionCreateRequest,
    ) -> Result<WorkflowGraphEditSessionCreateResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_graph_create_edit_session(request)
            .await
    }
    pub async fn workflow_graph_close_edit_session(
        &self,
        request: WorkflowGraphEditSessionCloseRequest,
    ) -> Result<WorkflowGraphEditSessionCloseResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_graph_close_edit_session(request)
            .await
    }
    pub async fn workflow_graph_refresh_current_validation_summary(
        &self,
        request: WorkflowGraphCurrentValidationRefreshRequest,
    ) -> Result<WorkflowGraphCurrentValidationRefreshResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_graph_refresh_current_validation_summary(request)
            .await
    }
    pub async fn workflow_graph_resolve_dependency_environment_action_intent(
        &self,
        request: pantograph_inference_interface_contracts::DependencyEnvironmentActionIntent,
    ) -> Result<
        pantograph_inference_interface_contracts::DependencyEnvironmentActionIntentResult,
        WorkflowServiceError,
    > {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_service
            .workflow_graph_resolve_dependency_environment_action_intent(request)
            .await
    }
    pub async fn publish_graph_session_executable_validation_snapshot(
        &self,
        request: WorkflowGraphSessionExecutableValidationSnapshotPublishRequest,
    ) -> Result<ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .publish_graph_session_executable_validation_snapshot(request)
            .await
    }
    pub fn workflow_read_artifact_body(
        &self,
        request: ArtifactReadRequest,
    ) -> Result<ArtifactBodyRead, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime.workflow_read_artifact_body(request)
    }
    pub fn workflow_run_inspection_query(
        &self,
        request: WorkflowRunInspectionQueryRequest,
    ) -> Result<WorkflowRunInspectionQueryResponse, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime
            .workflow_service
            .workflow_run_inspection_query(request)
    }
    /// Advance one bounded page for each requested diagnostic projection entry.
    ///
    /// The service validates the existing request bounds and returns the actual
    /// projection cursors and failures. This does not execute or resume a run.
    pub fn workflow_diagnostics_projection_refresh(
        &self,
        request: WorkflowDiagnosticsProjectionRefreshRequest,
    ) -> Result<WorkflowDiagnosticsProjectionRefreshResponse, WorkflowServiceError> {
        let _admission = self.admit_now()?;
        self.runtime
            .workflow_service
            .workflow_diagnostics_projection_refresh(request)
    }
    pub async fn workflow_get_scheduler_task_state_read_models(
        &self,
        request: WorkflowSchedulerTaskStateReadModelQueryRequest,
    ) -> Result<WorkflowSchedulerTaskStateReadModelQueryResponse, WorkflowServiceError> {
        let _admission = self.admit().await?;
        self.runtime
            .workflow_service
            .workflow_get_scheduler_task_state_read_models(request)
            .await
    }
    pub fn registry_snapshot(&self) -> RuntimeRegistrySnapshot {
        self.registry.snapshot()
    }
    pub fn residency_diagnostics(&self) -> Option<SerialCpuResidencyDiagnostics> {
        self.port.residency_diagnostics()
    }

    #[cfg(test)]
    pub(crate) fn set_residency_after_load_gate(
        &self,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        self.port.set_after_load_gate(entered, release);
    }

    #[cfg(test)]
    pub(crate) fn set_residency_after_drain_gate(
        &self,
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    ) {
        self.port.set_after_drain_gate(entered, release);
    }

    /// Close admission, drain the one supervised execution owner, consume native
    /// stop/ACK, then finish metadata teardown. Failure never invokes generic
    /// stop/reconciliation or retries a fenced capability. Repeated calls return
    /// the same terminal outcome. The retained supervisor continues when a caller
    /// drops its wait; a later call observes the same join and actual stop/ACK.
    pub async fn shutdown(&self) -> Result<(), WorkflowServiceError> {
        self.closed.store(true, Ordering::Release);
        self.port.close_residency_admission();
        let mut state = self.shutdown_state.lock().await;
        if let Some(result) = state.result.as_ref() {
            return result
                .clone()
                .map_err(WorkflowServiceError::RuntimeNotReady);
        }
        if state.task.is_none() {
            let execution = self.execution.clone();
            let runtime = self.runtime.clone();
            let port = self.port.clone();
            let operations = self.operations.clone();
            state.task = Some(tokio::spawn(async move {
                execution
                    .shutdown_workflow_execution_runtime(
                        Duration::from_secs(30),
                        Duration::from_secs(5),
                    )
                    .await?;
                let _operations = operations.write().await;
                port.shutdown_residency()
                    .await
                    .map_err(|error| WorkflowServiceError::RuntimeNotReady(error.to_string()))?;
                runtime.finish_serial_residency_teardown().await
            }));
        }
        // Borrow the retained JoinHandle: dropping this caller drops only its
        // wait/lock guard, never the supervisor or its native completion owner.
        let result = match state
            .task
            .as_mut()
            .expect("shutdown supervisor installed")
            .await
        {
            Ok(result) => result.map_err(|error| error.to_string()),
            Err(error) => Err(format!(
                "serial CPU residency shutdown supervisor failed: {error}"
            )),
        };
        state.task = None;
        state.result = Some(result.clone());
        result.map_err(WorkflowServiceError::RuntimeNotReady)
    }
}

#[cfg(test)]
#[path = "serial_cpu_residency_tests.rs"]
pub(crate) mod tests;

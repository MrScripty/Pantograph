//! Opt-in retained-only actual native singleton dispatch and linear cleanup.
use crate::runtime_host_load_target::{
    RuntimeHostLoadTargetResolver, RuntimeHostPumasLoadTargetResolver,
};
use crate::runtime_host_package_facts::{
    RuntimeHostPackageFactsResolver, RuntimeHostPumasPackageFactsResolver,
};
use async_trait::async_trait;
use inference::gateway::{
    CandleCpuDrainedAttempt, CandleCpuWarmAttemptIdentity, CandleCpuWarmAttemptRequest,
};
use pantograph_runtime_host_contracts::*;
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_scheduler::*;
use std::sync::Arc;

#[cfg(test)]
type NativePhaseGate = (Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>);

/// Explicit constructor only. No default host or selector activates this port.
/// Public construction requires a qualified resident owner and retaining successor.
/// Cold residency construction stays private to the dedicated owning facade.
pub struct EmbeddedRetainedCpuSerialPort {
    pub(crate) gateway: Arc<inference::InferenceGateway>,
    pub(crate) registry: SharedRuntimeRegistry,
    package: Arc<dyn RuntimeHostPackageFactsResolver>,
    target: Arc<dyn RuntimeHostLoadTargetResolver>,
    residency: Option<Arc<SerialCpuResidencyState>>,
    #[cfg(test)]
    after_load_gate: std::sync::Mutex<Option<NativePhaseGate>>,
    #[cfg(test)]
    after_drain_gate: std::sync::Mutex<Option<NativePhaseGate>>,
}
impl EmbeddedRetainedCpuSerialPort {
    pub fn new(
        gateway: Arc<inference::InferenceGateway>,
        registry: SharedRuntimeRegistry,
        selector: Arc<workflow_nodes::setup::PumasSelectorAccess>,
    ) -> Self {
        Self {
            gateway,
            registry,
            package: Arc::new(RuntimeHostPumasPackageFactsResolver::new(selector.clone())),
            target: Arc::new(RuntimeHostPumasLoadTargetResolver::new(selector)),
            residency: None,
            #[cfg(test)]
            after_load_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            after_drain_gate: std::sync::Mutex::new(None),
        }
    }
}
fn evidence(f: inference::CandleCpuSerialOwnerFacts) -> SerialRuntimeHostCpuOwnerEvidence {
    SerialRuntimeHostCpuOwnerEvidence {
        snapshot: SchedulerSerialOwnerSnapshot {
            loaded_instance: f.loaded_instance,
            loaded_profile: f.loaded_profile,
            effective_settings: f.effective_settings,
            generation: f.generation,
            cpu_threads: f.cpu_threads,
        },
        generation: f.generation_handle,
    }
}
fn execution_error(message: impl ToString) -> RuntimeHostExecutionPortError {
    RuntimeHostExecutionPortError::ExecutionFailed {
        message: message.to_string(),
    }
}
fn cleanup_error(message: impl ToString) -> ReservationLifecyclePortError {
    ReservationLifecyclePortError::Failed {
        message: message.to_string(),
    }
}
struct NativeCleanup {
    receipt: CandleCpuDrainedAttempt,
    registry: SharedRuntimeRegistry,
    lease_id: String,
    residency: Option<Arc<SerialCpuResidencyState>>,
}
impl SchedulerSerialVerifiedWarmDrain for NativeCleanup {
    fn identity(&self) -> SchedulerSerialAttemptIdentity<'_> {
        let id = self.receipt.identity();
        SchedulerSerialAttemptIdentity {
            workflow_id: id.workflow_id,
            workflow_run_id: id.workflow_run_id,
            node_id: id.node_id,
            task_id: id.task_id,
            attempt_id: id.attempt_id,
            execution_request_id: id.execution_request_id,
            candidate_id: id.candidate_id,
            reservation_lease_id: &self.lease_id,
        }
    }
    fn previous_owner(&self) -> SchedulerSerialOwnerSnapshot {
        evidence(self.receipt.previous_owner_facts()).snapshot
    }
    fn current_owner(&self) -> SchedulerSerialOwnerSnapshot {
        evidence(self.receipt.current_owner_facts()).snapshot
    }
}
#[async_trait]
impl SerialRuntimeHostCleanupReceipt for NativeCleanup {
    fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        evidence(self.receipt.previous_owner_facts())
    }
    fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        evidence(self.receipt.current_owner_facts())
    }
    async fn apply(
        self: Box<Self>,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        event.validate().map_err(cleanup_error)?;
        let id = self.receipt.identity();
        // Completed native results explicitly defer release to this receipt.
        // The legacy orchestrator labels that instruction RetryDeferred. The
        // sealed success/drain receipt, not that label, authorizes this release.
        if !matches!(
            event.outcome,
            ReservationLifecycleOutcome::RuntimeHostCompleted
                | ReservationLifecycleOutcome::RetryDeferred
        ) || event.workflow_id.as_str() != id.workflow_id
            || event.workflow_run_id.as_str() != id.workflow_run_id
            || event.node_id.as_str() != id.node_id
            || event.task_id.as_str() != id.task_id
            || event.candidate_id.as_ref().map(|id| id.as_str()) != Some(id.candidate_id)
            || event.reservation_lease_id.as_str() != self.lease_id
        {
            return Err(cleanup_error(
                "native linear cleanup identity/outcome refused",
            ));
        }
        let mut ack_record = serde_json::json!({"phase":"task_registry_ack",
            "workflow_id":id.workflow_id,"workflow_run_id":id.workflow_run_id,
            "node_id":id.node_id,"task_id":id.task_id,"attempt_id":id.attempt_id,
            "execution_request_id":id.execution_request_id,"candidate_id":id.candidate_id,
            "reservation_lease_id":id.reservation_lease_id,
            "lifecycle_event_id":event.lifecycle_event_id,
            "scheduler_cleanup_acknowledged":false,"registry_task_release_acknowledged":true});
        let residency_mode = self.residency.is_some();
        if let Some(state) = self.residency {
            // Keep the cap inaccessible until the actual exact task ACK. A lost
            // receipt or failed ACK leaves the taken/fenced state and charges.
            let (_, idle) = self
                .receipt
                .release_retained_with_residency()
                .await
                .map_err(cleanup_error)?;
            let owner = evidence(idle.owner_facts());
            let mut cell = state.cell.lock().await;
            if cell.idle.is_some() || !cell.fenced {
                return Err(cleanup_error(
                    "residency cleanup destination is not exclusively in flight",
                ));
            }
            cell.idle = Some(idle);
            cell.fenced = false;
            *state.owner.lock().expect("CPU residency owner lock") = Some(owner);
        } else {
            self.receipt
                .release_retained()
                .await
                .map_err(cleanup_error)?;
        }
        if residency_mode {
            ack_record["charged_after_registry_ack"] = charged_resources(&self.registry);
            log_residency(ack_record);
        }
        Ok(ReservationLifecycleApplication {
            contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: event.reservation_lease_id,
            state: ReservationLifecycleApplicationState::Applied,
            diagnostics: Vec::new(),
        })
    }
}
#[async_trait]
impl RuntimeHostBatchExecutionPort for EmbeddedRetainedCpuSerialPort {
    async fn execute_runtime_host_batch_request(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        Err(execution_error(
            "native serial dispatch requires owned admission and a linear cleanup handoff",
        ))
    }
}
#[async_trait]
impl ReservationLifecyclePort for EmbeddedRetainedCpuSerialPort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        if self.residency.is_some() {
            return self.apply_residency_task_lifecycle(event);
        }
        crate::reservation_lifecycle::EmbeddedReservationLifecyclePort::new(
            self.registry.clone(),
            self.gateway.clone(),
        )
        .apply_reservation_lifecycle(event)
        .await
    }
}
#[async_trait]
impl SerialRuntimeHostBatchExecutionPort for EmbeddedRetainedCpuSerialPort {
    async fn prepare_serial_cpu(
        &self,
        request: SerialRuntimeHostPreparationRequest,
    ) -> Result<SerialRuntimeHostCpuOwnerEvidence, RuntimeHostExecutionPortError> {
        self.prepare_residency(request).await
    }
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        if intent.task_type.as_str() != "embedding"
            || !intent.trait_settings.is_empty()
            || intent.constraints.requested_runtime_id.as_ref()?.as_str() != "candle"
            || intent.constraints.requested_device_id.as_ref()?.as_str() != "cpu"
        {
            return None;
        }
        if let Some(state) = &self.residency {
            let owner = state.owner.lock().ok()?.clone()?;
            let model = crate::runtime_host_text_execution::project_model_ref(&intent.model_ref);
            let native = self.gateway.resident_cpu_serial_owner(&model)?;
            return (owner.is_current()
                && evidence(native.serial_facts()).snapshot == owner.snapshot)
                .then_some(owner);
        }
        let model = crate::runtime_host_text_execution::project_model_ref(&intent.model_ref);
        let owner = self.gateway.resident_cpu_serial_owner(&model)?;
        let evidence = evidence(owner.serial_facts());
        evidence.is_current().then_some(evidence)
    }
    async fn execute_serial_singleton(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
        _: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    > {
        Err(execution_error(
            "native serial dispatch requires its consumed cleanup receipt",
        ))
    }
    async fn apply_serial_cleanup(
        &self,
        _: ReservationLifecycleEvent,
        _: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        Err(cleanup_error(
            "native serial cleanup requires its actual linear receipt; snapshot fallback refused",
        ))
    }
    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        if !CandleCpuWarmAttemptRequest::metadata_is_bounded(&request) || request.members.len() != 1
        {
            return Err(execution_error(
                "native serial singleton metadata bounds refused",
            ));
        }
        request.validate().map_err(execution_error)?;
        let member = &request.members[0];
        let id = ownership.identity();
        let selected = member
            .handoff
            .dispatch_decision
            .as_ref()
            .ok_or_else(|| execution_error("selected lease missing"))?;
        if id.workflow_id != member.handoff.workflow_id.as_str()
            || id.workflow_run_id != member.handoff.workflow_run_id.as_str()
            || id.node_id != member.handoff.node_id.as_str()
            || id.task_id != member.handoff.task_id.as_str()
            || id.execution_request_id != member.execution_request_id
            || id.reservation_lease_id != selected.reservation_lease_id.as_str()
        {
            return Err(execution_error("native serial bound handoff changed"));
        }
        let lease_id = selected.reservation_lease_id.as_str();
        let reservation_id = lease_id
            .strip_prefix("runtime-registry.")
            .and_then(|id| id.parse::<u64>().ok())
            .ok_or_else(|| execution_error("native serial lease is not registry-issued"))?;
        if lease_id != format!("runtime-registry.{reservation_id}") {
            return Err(execution_error("noncanonical lease identity"));
        }
        let lease = self
            .registry
            .reservation_lease(reservation_id)
            .ok_or_else(|| execution_error("selected live lease missing"))?;
        let single = RuntimeHostExecutionRequest {
            contract_version: request.contract_version,
            execution_request_id: member.execution_request_id.clone(),
            handoff: member.handoff.clone(),
            materialized_inputs: member.materialized_inputs.clone(),
            cancellation_context: request.cancellation_context.clone(),
        };
        let validated =
            ValidatedRuntimeHostExecutionRequest::try_from(single).map_err(execution_error)?;
        crate::runtime_host_embedding_execution::validate_runtime_host_embedding_request(
            validated.as_ref(),
        )
        .map_err(execution_error)?;
        let package = self
            .package
            .resolve(&validated)
            .await
            .map_err(execution_error)?;
        let target = self
            .target
            .resolve(&validated)
            .await
            .map_err(execution_error)?;
        if !CandleCpuWarmAttemptRequest::metadata_is_bounded(&(&request, &package, &target)) {
            return Err(execution_error(
                "native serial resolved metadata bounds refused",
            ));
        }
        let target =
            crate::runtime_host_image_execution::project_pumas_artifact_load_target(target);
        let projection = crate::runtime_host_embedding_execution::project_runtime_host_embedding(
            &validated, package, target,
        )
        .map_err(execution_error)?;
        let stamp = self
            .gateway
            .resident_cpu_serial_owner(&projection.target.model_ref)
            .ok_or_else(|| {
                execution_error("actual qualified native owner unknown; fallback withheld")
            })?;
        if ownership
            .expected_owner()
            .is_some_and(|expected| expected != evidence(stamp.serial_facts()).snapshot)
        {
            return Err(execution_error(
                "native ranked owner changed; fallback withheld",
            ));
        }
        let native_identity = CandleCpuWarmAttemptIdentity {
            workflow_id: id.workflow_id,
            workflow_run_id: id.workflow_run_id,
            node_id: id.node_id,
            task_id: id.task_id,
            attempt_id: id.attempt_id,
            execution_request_id: id.execution_request_id,
            candidate_id: id.candidate_id,
            reservation_lease_id: reservation_id,
        };
        let (result, receipt) = if let Some(state) = &self.residency {
            let mut cell = state.cell.lock().await;
            if cell.fenced || state.closed.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(execution_error("residency owner is fenced or closing"));
            }
            let idle = cell
                .idle
                .take()
                .ok_or_else(|| execution_error("native idle capability missing"))?;
            cell.fenced = true;
            *state.owner.lock().expect("CPU residency owner lock") = None;
            drop(cell);
            self.gateway
                .execute_resident_cpu_warm_attempt(
                    idle,
                    inference::gateway::CandleCpuResidencyWarmAttemptRequest {
                        request: projection.request,
                        target: projection.target,
                        decision: projection.decision,
                        identity: native_identity,
                        lease: &lease,
                    },
                    inference_cancellation(cancellation),
                )
                .await
                .map_err(execution_error)?
        } else {
            self.gateway
                .execute_custodied_cpu_warm_attempt(
                    self.registry.clone(),
                    CandleCpuWarmAttemptRequest {
                        request: projection.request,
                        target: projection.target,
                        decision: projection.decision,
                        identity: native_identity,
                        lease: &lease,
                        expected_owner: &stamp,
                    },
                    inference_cancellation(cancellation),
                )
                .await
                .map_err(execution_error)?
        };
        if let Some(state) = &self.residency {
            state
                .diagnostics
                .lock()
                .expect("CPU residency diagnostics lock")
                .forward_drains += 1;
        }
        #[cfg(test)]
        if self.residency.is_some() {
            let gate = self
                .after_drain_gate
                .lock()
                .expect("after-drain test gate")
                .take();
            if let Some((entered, proceed)) = gate {
                entered.notify_one();
                proceed.notified().await;
            }
        }
        if self.residency.is_some() {
            let native = receipt.identity();
            log_residency(serde_json::json!({
                "phase":"native_drained", "workflow_id":native.workflow_id,
                "workflow_run_id":native.workflow_run_id,"node_id":native.node_id,
                "task_id":native.task_id,"attempt_id":native.attempt_id,
                "execution_request_id":native.execution_request_id,"candidate_id":native.candidate_id,
                "reservation_lease_id":native.reservation_lease_id,
                "previous_owner":owner_record(evidence(receipt.previous_owner_facts()).snapshot),
                "current_owner":owner_record(evidence(receipt.current_owner_facts()).snapshot),
                "forward_and_drain_ns":receipt.forward_and_drain_duration().map(|duration|duration.as_nanos()),
                "registry_task_release_acknowledged":false,
                "charged_after_native_drain":charged_resources(&self.registry)
            }));
        }
        let outputs =
            crate::runtime_host_embedding_execution::embedding_outputs(validated.as_ref(), result)
                .map_err(execution_error)?;
        let cleanup = NativeCleanup {
            receipt,
            registry: self.registry.clone(),
            lease_id: lease_id.to_owned(),
            residency: self.residency.clone(),
        };
        let actual = SerialRuntimeHostCleanupReceipt::current_owner(&cleanup);
        let drained = ownership
            .record_verified_warm_drained_response(&cleanup)
            .map_err(|error| execution_error(format!("native warm receipt refused: {error:?}")))?;
        let response = RuntimeHostBatchExecutionResponse {
            contract_version: request.contract_version,
            batch_execution_request_id: request.batch_execution_request_id,
            state: RuntimeHostBatchExecutionState::Completed,
            members: vec![RuntimeHostBatchExecutionMemberResponse {
                execution_request_id: member.execution_request_id.clone(),
                assignment_id: member.assignment_id.clone(),
                workflow_id: member.handoff.workflow_id.clone(),
                workflow_run_id: member.handoff.workflow_run_id.clone(),
                node_id: member.handoff.node_id.clone(),
                task_id: member.handoff.task_id.clone(),
                state: RuntimeHostBatchExecutionMemberState::Completed,
                retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                reservation_disposition:
                    RuntimeHostBatchMemberReservationDisposition::DeferredToScheduler,
                outputs,
                diagnostics: Vec::new(),
                terminal_metadata: None,
            }],
            diagnostics: Vec::new(),
        };
        response.validate().map_err(execution_error)?;
        Ok(SerialRuntimeHostDrainedExecution {
            response,
            drained,
            owner: Some(actual),
            cleanup: Some(Box::new(cleanup)),
        })
    }
}
struct Cancellation(RuntimeHostExecutionCancellationHandle);
impl inference::InferenceExecutionCancellationSignal for Cancellation {
    fn snapshot(&self) -> inference::InferenceExecutionCancellationSnapshot {
        let snapshot = self.0.snapshot();
        if snapshot.state == RuntimeHostExecutionCancellationState::Running {
            inference::InferenceExecutionCancellationSnapshot::running()
        } else {
            inference::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "native serial host cancellation or unknown state".into(),
            ))
        }
    }
}
fn inference_cancellation(
    handle: RuntimeHostExecutionCancellationHandle,
) -> inference::InferenceExecutionCancellationHandle {
    inference::InferenceExecutionCancellationHandle::with_signal(Arc::new(Cancellation(handle)))
}

#[cfg(test)]
#[path = "serial_cpu_port_tests.rs"]
pub(crate) mod tests;

/// Scalar-only diagnostics; no writable native generation handle escapes.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SerialCpuResidencyDiagnostics {
    pub loads: u64,
    pub retains: u64,
    pub stops: u64,
    pub forward_drains: u64,
    pub total_load_ns: u128,
    pub total_stop_ns: u128,
    pub total_publication_ns: u128,
    pub total_acknowledgement_ns: u128,
    pub fenced: bool,
    pub has_idle_owner: bool,
}

pub(crate) struct CpuResidencyDeclaration {
    pub target: inference::PumasArtifactLoadTarget,
    pub resident_ram_bytes: u64,
    pub load_peak_ram_bytes: u64,
    pub load_peak_vram_bytes: u64,
}
struct SerialCpuResidencyCell {
    idle: Option<inference::gateway::CandleCpuIdleResidency>,
    fenced: bool,
}
struct SerialCpuResidencyState {
    cell: tokio::sync::Mutex<SerialCpuResidencyCell>,
    owner: std::sync::Mutex<Option<SerialRuntimeHostCpuOwnerEvidence>>,
    closed: std::sync::atomic::AtomicBool,
    declarations: Vec<CpuResidencyDeclaration>,
    selector: Arc<workflow_nodes::setup::PumasSelectorAccess>,
    diagnostics: std::sync::Mutex<SerialCpuResidencyDiagnostics>,
}
impl EmbeddedRetainedCpuSerialPort {
    pub(crate) fn new_residency(
        gateway: Arc<inference::InferenceGateway>,
        registry: SharedRuntimeRegistry,
        selector: Arc<workflow_nodes::setup::PumasSelectorAccess>,
        declarations: Vec<CpuResidencyDeclaration>,
    ) -> Result<Self, RuntimeHostExecutionPortError> {
        if declarations.is_empty()
            || declarations.len() > 8
            || declarations
                .iter()
                .any(|d| d.resident_ram_bytes == 0 || d.load_peak_ram_bytes == 0)
            || declarations
                .iter()
                .enumerate()
                .any(|(i, d)| declarations[..i].iter().any(|p| p.target == d.target))
        {
            return Err(execution_error(
                "bounded nonduplicate CPU residency declarations required",
            ));
        }
        let mut port = Self::new(gateway, registry, selector.clone());
        port.residency = Some(Arc::new(SerialCpuResidencyState {
            cell: tokio::sync::Mutex::new(SerialCpuResidencyCell {
                idle: None,
                fenced: false,
            }),
            owner: std::sync::Mutex::new(None),
            closed: std::sync::atomic::AtomicBool::new(false),
            declarations,
            selector,
            diagnostics: std::sync::Mutex::new(SerialCpuResidencyDiagnostics::default()),
        }));
        Ok(port)
    }
    fn apply_residency_task_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        event.validate().map_err(cleanup_error)?;
        let raw = event.reservation_lease_id.as_str();
        let id = raw
            .strip_prefix("runtime-registry.")
            .and_then(|id| id.parse::<u64>().ok())
            .filter(|id| raw == format!("runtime-registry.{id}"))
            .ok_or_else(|| cleanup_error("residency lifecycle lease is not registry-issued"))?;
        let lease = self
            .registry
            .reservation_lease(id)
            .ok_or_else(|| cleanup_error("exact residency task lease absent"))?;
        let owner = format!("{}:{}", event.workflow_run_id, event.task_id);
        if lease.runtime_id != "candle"
            || lease.workflow_id != event.workflow_id.as_str()
            || lease.reservation_owner_id.as_deref() != Some(owner.as_str())
            || lease.pin_runtime
            || lease.retention_hint != pantograph_runtime_registry::RuntimeRetentionHint::Ephemeral
            || event.candidate_id.is_none()
        {
            return Err(cleanup_error(
                "residency lifecycle task lease identity refused",
            ));
        }
        let application_state = match event.outcome {
            ReservationLifecycleOutcome::DispatchStarted => {
                ReservationLifecycleApplicationState::Applied
            }
            ReservationLifecycleOutcome::DuplicateReplay => {
                ReservationLifecycleApplicationState::AlreadyApplied
            }
            ReservationLifecycleOutcome::CandidateUnselected
            | ReservationLifecycleOutcome::CandidateRequestRejected
            | ReservationLifecycleOutcome::RuntimeHostDispatchRejected
            | ReservationLifecycleOutcome::WorkflowCancelled => {
                // Only an exact ordinary unstarted task lease can be released.
                // Registry custody rejects an executing/undrained native task or
                // the load-started mode guard. No generic stop/reconcile runs.
                self.registry
                    .release_reservation_with_disposition(id)
                    .map_err(cleanup_error)?;
                ReservationLifecycleApplicationState::Applied
            }
            _ => {
                return Err(cleanup_error(
                    "residency native completion requires its actual sealed cleanup receipt",
                ))
            }
        };
        Ok(ReservationLifecycleApplication {
            contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: event.reservation_lease_id,
            state: application_state,
            diagnostics: Vec::new(),
        })
    }
    #[cfg(test)]
    pub(crate) fn set_after_load_gate(
        &self,
        entered: Arc<tokio::sync::Notify>,
        proceed: Arc<tokio::sync::Notify>,
    ) {
        *self.after_load_gate.lock().expect("after-load test gate") = Some((entered, proceed));
    }
    #[cfg(test)]
    pub(crate) fn set_after_drain_gate(
        &self,
        entered: Arc<tokio::sync::Notify>,
        proceed: Arc<tokio::sync::Notify>,
    ) {
        *self.after_drain_gate.lock().expect("after-drain test gate") = Some((entered, proceed));
    }
    pub(crate) fn close_residency_admission(&self) {
        if let Some(state) = &self.residency {
            state
                .closed
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    pub(crate) fn residency_diagnostics(&self) -> Option<SerialCpuResidencyDiagnostics> {
        let state = self.residency.as_ref()?;
        let mut diagnostics = state.diagnostics.lock().ok()?.clone();
        if let Ok(cell) = state.cell.try_lock() {
            diagnostics.fenced = cell.fenced;
            diagnostics.has_idle_owner = cell.idle.is_some();
        } else {
            diagnostics.fenced = true;
            diagnostics.has_idle_owner = false;
        }
        Some(diagnostics)
    }
    pub(crate) async fn shutdown_residency(&self) -> Result<(), RuntimeHostExecutionPortError> {
        let state = self
            .residency
            .as_ref()
            .ok_or_else(|| execution_error("residency mode absent"))?;
        self.close_residency_admission();
        let mut record = None;
        let mut cell = state.cell.lock().await;
        if cell.fenced {
            return Err(execution_error("residency shutdown refused fenced owner"));
        }
        if let Some(idle) = cell.idle.take() {
            let old_owner = owner_record(evidence(idle.owner_facts()).snapshot);
            let before_stop = charged_resources(&self.registry);
            cell.fenced = true;
            *state.owner.lock().expect("CPU residency owner lock") = None;
            let outcome = idle
                .stop(inference::InferenceExecutionCancellationHandle::running())
                .await
                .map_err(execution_error)?;
            let mut diagnostics = state
                .diagnostics
                .lock()
                .expect("CPU residency diagnostics lock");
            diagnostics.stops += 1;
            diagnostics.total_stop_ns += outcome.stop_duration.as_nanos();
            diagnostics.total_acknowledgement_ns += outcome.acknowledgement_duration.as_nanos();
            record = Some(
                serde_json::json!({"phase":"shutdown_stop_ack","old_owner":old_owner,
                "charged_before_stop":before_stop,"charged_after_ack":charged_resources(&self.registry),
                "stop_ns":outcome.stop_duration.as_nanos(),"acknowledgement_ns":outcome.acknowledgement_duration.as_nanos()}),
            );
            cell.fenced = false;
        }
        drop(cell);
        if let Some(record) = record {
            log_residency(record);
        }
        Ok(())
    }
    async fn prepare_residency(
        &self,
        request: SerialRuntimeHostPreparationRequest,
    ) -> Result<SerialRuntimeHostCpuOwnerEvidence, RuntimeHostExecutionPortError> {
        request.validate()?;
        let state = self.residency.as_ref().ok_or_else(|| {
            execution_error("retained-only CPU port does not prepare cold residency")
        })?;
        if state.closed.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(execution_error("residency admission closed"));
        }
        let package = RuntimeHostPumasPackageFactsResolver::new(state.selector.clone())
            .resolve_for_model(&request.task_intent.model_ref)
            .await
            .map_err(execution_error)?;
        let target = RuntimeHostPumasLoadTargetResolver::new(state.selector.clone())
            .resolve_for_model(&request.task_intent.model_ref, "embedding", "candle.cpu")
            .await
            .map_err(execution_error)?;
        let target =
            crate::runtime_host_image_execution::project_pumas_artifact_load_target(target);
        let declaration = state
            .declarations
            .iter()
            .find(|d| d.target == target)
            .ok_or_else(|| {
                execution_error(
                    "owner-fresh CPU target lacks an exact immutable residency declaration",
                )
            })?;
        let task_peak = cpu_task_claims(&request.task_intent, &package)?;
        let capacity = self
            .registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "candle")
            .and_then(|runtime| runtime.admission_budget);
        log_residency(serde_json::json!({"phase":"preparation_plan",
            "preparation_request_id":request.preparation_request_id,
            "workflow_id":request.task_intent.workflow_id,"workflow_run_id":request.task_intent.workflow_run_id,
            "node_id":request.task_intent.node_id,"task_id":request.task_intent.task_id,
            "model_ref":{"model_id":request.task_intent.model_ref.model_id,"revision":request.task_intent.model_ref.revision,
                "selected_artifact_id":request.task_intent.model_ref.selected_artifact_id,
                "selected_artifact_path_digest":request.task_intent.model_ref.selected_artifact_path.as_ref().map(|path|blake3::hash(path.as_bytes()).to_hex().to_string())},
            "resolved_model_ref":{"model_id":target.model_ref.model_id,"revision":target.model_ref.revision,
                "selected_artifact_id":target.model_ref.selected_artifact_id,
                "selected_artifact_path_digest":target.model_ref.selected_artifact_path.as_ref().map(|path|blake3::hash(path.as_bytes()).to_hex().to_string())},
            "runtime_id":"candle","runtime_variant_id":"candle.cpu","device_id":"cpu",
            "actual_task_ram_bytes":task_peak.0,"actual_task_vram_bytes":task_peak.1,
            "package_conservative_floor_bytes":cpu_package_peak(&package)?,
            "declared_guard_ram_bytes":declaration.load_peak_ram_bytes,
            "declared_guard_vram_bytes":declaration.load_peak_vram_bytes,
            "declared_resident_ram_bytes":declaration.resident_ram_bytes,"declared_resident_vram_bytes":0,
            "declared_capacity":capacity,"guard_or_task_acquired_by_this_preparation":false,
            "required_ram_bytes":declaration.load_peak_ram_bytes.checked_add(declaration.resident_ram_bytes).and_then(|bytes|bytes.checked_add(task_peak.0)),
            "required_vram_bytes":declaration.load_peak_vram_bytes.checked_add(task_peak.1)
        }));
        let mut records = Vec::with_capacity(3);
        let mut preparation_stage = "idle_owner_validation";
        let outcome = async {
        let mut cell = state.cell.lock().await;
        if cell.fenced || state.closed.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(execution_error("residency is fenced or closing"));
        }
        if let Some(idle) = cell.idle.as_ref() {
            if idle.matches_target(&target) {
                let actual = evidence(idle.owner_facts());
                if !actual.is_current() {
                    cell.fenced = true;
                    return Err(execution_error("idle native owner changed"));
                }
                state
                    .diagnostics
                    .lock()
                    .expect("CPU residency diagnostics lock")
                    .retains += 1;
                records.push(serde_json::json!({"phase":"retained","preparation_request_id":request.preparation_request_id,"owner":owner_record(actual.snapshot)}));
                return Ok(actual);
            }
        }
        if let Some(idle) = cell.idle.take() {
            preparation_stage = "old_owner_stop";
            let old_owner = owner_record(evidence(idle.owner_facts()).snapshot);
            let before_stop = charged_resources(&self.registry);
            cell.fenced = true;
            *state.owner.lock().expect("CPU residency owner lock") = None;
            let outcome = idle
                .stop(inference::InferenceExecutionCancellationHandle::running())
                .await
                .map_err(execution_error)?;
            let mut diagnostics = state
                .diagnostics
                .lock()
                .expect("CPU residency diagnostics lock");
            diagnostics.stops += 1;
            diagnostics.total_stop_ns += outcome.stop_duration.as_nanos();
            diagnostics.total_acknowledgement_ns += outcome.acknowledgement_duration.as_nanos();
            records.push(serde_json::json!({"phase":"switch_stop_ack","preparation_request_id":request.preparation_request_id,
                "old_owner":old_owner,"charged_before_stop":before_stop,"charged_after_ack":charged_resources(&self.registry),
                "stop_ns":outcome.stop_duration.as_nanos(),"acknowledgement_ns":outcome.acknowledgement_duration.as_nanos()}));
            cell.fenced = false;
        }
        preparation_stage = "headroom_admission";
        validate_residency_headroom(&self.registry, declaration, task_peak)?;
        preparation_stage = "guard_admission";
        let lease = self
            .registry
            .acquire_reservation(pantograph_runtime_registry::RuntimeReservationRequest {
                runtime_id: "candle".into(),
                workflow_id: "cpu-serial-residency-owner".into(),
                reservation_owner_id: Some(request.preparation_request_id.clone()),
                usage_profile: None,
                model_id: Some(target.model_ref.model_id.clone()),
                pin_runtime: false,
                requirements: Some(
                    pantograph_runtime_registry::RuntimeReservationRequirements::from_claims(vec![
                        pantograph_runtime_registry::RuntimeReservationResourceClaim::ram_bytes(
                            declaration.load_peak_ram_bytes,
                        ),
                        pantograph_runtime_registry::RuntimeReservationResourceClaim::vram_bytes(
                            declaration.load_peak_vram_bytes,
                        ),
                    ]),
                ),
                retention_hint: pantograph_runtime_registry::RuntimeRetentionHint::Ephemeral,
            })
            .map_err(execution_error)?;
        // Every path after taking this gate is conservative: failed/drop load
        // never reconstructs authority or erases the uncertain guard charge.
        cell.fenced = true;
        let (native_request, decision) = preparation_projection(&request, package, &target)?;
        preparation_stage = "native_load";
        let native_preparation = inference::InferenceExecutionCancellationHandle::with_signal(Arc::new(
            ResidencyPreparationCancellation(state.clone()),
        ));
        let idle = self
            .gateway
            .load_cpu_idle_residency(
                self.registry.clone(),
                inference::gateway::CandleCpuColdResidencyRequest {
                    request: native_request,
                    target,
                    decision,
                    guard: &lease,
                },
                native_preparation.clone(),
            )
            .await
            .map_err(execution_error)?;
        records.push(serde_json::json!({"phase":"cold_loaded","preparation_request_id":request.preparation_request_id,
            "owner":owner_record(evidence(idle.owner_facts()).snapshot),"guard_lease_id":lease.reservation_id,
            "charged_after_publication":charged_resources(&self.registry),
            "load_ns":idle.load_duration().as_nanos(),"publication_ns":idle.publication_duration().as_nanos()}));
        let actual = evidence(idle.owner_facts());
        {
            let mut diagnostics = state
                .diagnostics
                .lock()
                .expect("CPU residency diagnostics lock");
            diagnostics.loads += 1;
            diagnostics.total_load_ns += idle.load_duration().as_nanos();
            diagnostics.total_publication_ns += idle.publication_duration().as_nanos();
        }
        #[cfg(test)]
        {
            let gate=self.after_load_gate.lock().expect("after-load test gate").take();
            if let Some((entered,proceed))=gate {
                entered.notify_one();proceed.notified().await;
            }
        }
        // Common production recheck: shutdown can arrive while the supervised
        // loader joins/publishes. A test gate only controls this real boundary.
        if let Some(message)=native_preparation.rejection_message("completed native load before Start") {
            preparation_stage = "pre_start_shutdown_signal";
            return Err(execution_error(message));
        }
        cell.idle = Some(idle);
        cell.fenced = false;
        *state.owner.lock().expect("CPU residency owner lock") = Some(actual.clone());
        Ok(actual)
        }.await;
        for record in records {
            log_residency(record);
        }
        if outcome.is_err() {
            log_residency(
                serde_json::json!({"phase":"preparation_refused","preparation_request_id":request.preparation_request_id,"no_task_start_authorized":true,"refusal_stage":preparation_stage}),
            );
        }
        outcome
    }
}
fn cpu_task_claims(
    intent: &SchedulableTaskIntent,
    package: &inference::ResolvedModelPackageFacts,
) -> Result<(u64, u64), RuntimeHostExecutionPortError> {
    let conservative_floor = cpu_package_peak(package)?;
    // Match reservation_requirements_from_estimate_hints: actual per-kind MAX,
    // not a fresh package estimate substituted for the selected task's claims.
    let ram = intent
        .estimate_hints
        .iter()
        .filter(|h| h.kind == SchedulerEstimateHintKind::PeakRamBytes)
        .map(|h| h.value)
        .max();
    let vram = intent
        .estimate_hints
        .iter()
        .filter(|h| h.kind == SchedulerEstimateHintKind::PeakVramBytes)
        .map(|h| h.value)
        .max();
    match (ram, vram) {
        (Some(ram), Some(vram)) if ram >= conservative_floor && vram >= conservative_floor => {
            Ok((ram, vram))
        }
        _ => Err(execution_error(
            "actual per-kind task claims missing or below existing conservative estimator",
        )),
    }
}
fn validate_residency_headroom(
    registry: &SharedRuntimeRegistry,
    declaration: &CpuResidencyDeclaration,
    task_peak: (u64, u64),
) -> Result<(), RuntimeHostExecutionPortError> {
    use pantograph_runtime_registry::RuntimeAdmissionResourceKind::{RamBytes, VramBytes};
    let snapshot = registry.snapshot();
    let runtime = snapshot
        .runtimes
        .iter()
        .find(|r| r.runtime_id == "candle")
        .ok_or_else(|| execution_error("CPU runtime registration absent"))?;
    if !runtime.active_reservation_ids.is_empty()
        || runtime.model_resource_residency.is_some()
        || runtime.resident_resources_uncertain
    {
        return Err(execution_error(
            "cold residency requires acknowledged empty runtime",
        ));
    }
    for (kind, guard, resident, task) in [
        (
            RamBytes,
            declaration.load_peak_ram_bytes,
            declaration.resident_ram_bytes,
            task_peak.0,
        ),
        (VramBytes, declaration.load_peak_vram_bytes, 0, task_peak.1),
    ] {
        let resource = runtime
            .admission_budget
            .as_ref()
            .and_then(|b| b.resources.iter().find(|r| r.kind == kind))
            .ok_or_else(|| execution_error("explicit per-kind CPU residency capacity required"))?;
        let needed = guard
            .checked_add(resident)
            .and_then(|n| n.checked_add(task))
            .and_then(|n| n.checked_add(resource.safety_margin_bytes));
        if needed.is_none() || resource.total_bytes.is_none() || needed > resource.total_bytes {
            return Err(execution_error(
                "additive guard/resident/task CPU residency headroom refused",
            ));
        }
    }
    Ok(())
}
struct ResidencyPreparationCancellation(Arc<SerialCpuResidencyState>);
impl inference::InferenceExecutionCancellationSignal for ResidencyPreparationCancellation {
    fn snapshot(&self) -> inference::InferenceExecutionCancellationSnapshot {
        if self.0.closed.load(std::sync::atomic::Ordering::SeqCst) {
            inference::InferenceExecutionCancellationSnapshot::shutdown_requested(Some(
                "CPU residency owner closing".into(),
            ))
        } else {
            inference::InferenceExecutionCancellationSnapshot::running()
        }
    }
}
fn preparation_projection(
    request: &SerialRuntimeHostPreparationRequest,
    package: inference::ResolvedModelPackageFacts,
    target: &inference::PumasArtifactLoadTarget,
) -> Result<
    (
        inference::InferenceExecutionRequest,
        inference::BackendExecutionDecision,
    ),
    RuntimeHostExecutionPortError,
> {
    use inference::*;
    let text = match &request.materialized_inputs[0].value {
        RuntimeHostExecutionInputValue::String(text)
        | RuntimeHostExecutionInputValue::TranscriptText(text) => text.clone(),
        _ => return Err(execution_error("CPU residency preparation text refused")),
    };
    let device = InferenceDeviceId::parse("cpu").map_err(execution_error)?;
    let variant = RuntimeVariantId::parse("candle.cpu").map_err(execution_error)?;
    let mut requested_model = target.model_ref.clone();
    requested_model.revision = request.task_intent.model_ref.revision.clone();
    Ok((
        InferenceExecutionRequest {
            request_id: Some(request.preparation_request_id.clone()),
            task_id: InferenceTaskId::Embedding,
            model_ref: Some(requested_model),
            model_name: None,
            resolved_model_package_facts: Some(package),
            input: InferenceExecutionInput::Embedding { texts: vec![text] },
            generation_options: None,
            extra_options: serde_json::Value::Null,
        },
        BackendExecutionDecision {
            selected_backend_id: BackendId::parse("candle").map_err(execution_error)?,
            selected_runtime_variant_id: variant.clone(),
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device.clone()),
            device_decision: DeviceResolutionDecision {
                policy: InferenceDevicePolicy::Auto,
                runtime_variant_id: variant,
                selected_device_class: InferenceDeviceClass::Cpu,
                selected_device_id: Some(device),
                diagnostics: vec![],
            },
            selected_task_id: Some(InferenceTaskId::Embedding),
            selected_model_ref: Some(target.model_ref.clone()),
            diagnostics: vec![],
            dependency_readiness: vec![],
            selection_policy_trace: None,
        },
    ))
}

fn cpu_package_peak(
    package: &inference::ResolvedModelPackageFacts,
) -> Result<u64, RuntimeHostExecutionPortError> {
    let size = package
        .artifact
        .logical_size
        .as_ref()
        .ok_or_else(|| execution_error("known full package logical size required"))?;
    crate::inference_resource_estimator::conservative_loaded_runtime_memory_estimate_bytes(size)
        .and_then(|model| model.checked_add(512 * 1024 * 1024))
        .ok_or_else(|| execution_error("known nonoverflowing conservative CPU task peak required"))
}
fn owner_record(owner: SchedulerSerialOwnerSnapshot) -> serde_json::Value {
    serde_json::json!({"loaded_instance":owner.loaded_instance,"loaded_profile":owner.loaded_profile,
        "effective_settings":owner.effective_settings,"generation":owner.generation,"cpu_threads":owner.cpu_threads})
}
fn charged_resources(registry: &SharedRuntimeRegistry) -> serde_json::Value {
    let snapshot = registry.snapshot();
    let Some(runtime) = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "candle")
    else {
        return serde_json::Value::Null;
    };
    serde_json::json!({"active_lease_ids":runtime.active_reservation_ids,
        "active_claims":runtime.active_reservation_claims,
        "resident_claims":runtime.model_resource_residency.as_ref().and_then(|resident|resident.requirements.as_ref()),
        "resident_uncertain":runtime.resident_resources_uncertain})
}
fn log_residency(record: serde_json::Value) {
    log::info!(target:"embedded::serial_cpu_residency","{}",record);
}

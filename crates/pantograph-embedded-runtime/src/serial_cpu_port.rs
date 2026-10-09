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

/// Explicit constructor only. No default host or selector activates this port.
/// Requires an already qualified resident CPU owner and retaining successor.
pub struct EmbeddedRetainedCpuSerialPort {
    pub(crate) gateway: Arc<inference::InferenceGateway>,
    pub(crate) registry: SharedRuntimeRegistry,
    package: Arc<dyn RuntimeHostPackageFactsResolver>,
    target: Arc<dyn RuntimeHostLoadTargetResolver>,
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
    lease_id: String,
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
        self.receipt
            .release_retained()
            .await
            .map_err(cleanup_error)?;
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
        let (result, receipt) = self
            .gateway
            .execute_custodied_cpu_warm_attempt(
                self.registry.clone(),
                CandleCpuWarmAttemptRequest {
                    request: projection.request,
                    target: projection.target,
                    decision: projection.decision,
                    identity: CandleCpuWarmAttemptIdentity {
                        workflow_id: id.workflow_id,
                        workflow_run_id: id.workflow_run_id,
                        node_id: id.node_id,
                        task_id: id.task_id,
                        attempt_id: id.attempt_id,
                        execution_request_id: id.execution_request_id,
                        candidate_id: id.candidate_id,
                        reservation_lease_id: reservation_id,
                    },
                    lease: &lease,
                    expected_owner: &stamp,
                },
                inference_cancellation(cancellation),
            )
            .await
            .map_err(execution_error)?;
        let outputs =
            crate::runtime_host_embedding_execution::embedding_outputs(validated.as_ref(), result)
                .map_err(execution_error)?;
        let cleanup = NativeCleanup {
            receipt,
            lease_id: lease_id.to_owned(),
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

//! Shared runtime-host execution boundary contracts.
//!
//! This crate owns the serialized request/response DTOs, validation wrappers,
//! typed errors, runtime-host execution port, and scheduler dispatch helper
//! used between workflow orchestration and host runtime execution. It also owns
//! the shared reservation lifecycle contract that lets workflow-service report
//! dispatch/session outcomes while embedded-runtime owns runtime-registry
//! release and retention side effects. It does not own scheduler policy,
//! workflow orchestration, runtime loading, Pumas load-target resolution,
//! node-engine execution, concrete I/O, or Tokio runtime lifecycle.

mod reservation_lifecycle;
mod runtime_host_dispatch;
mod runtime_host_execution;
mod runtime_session_load;

pub use reservation_lifecycle::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleContractError, ReservationLifecycleDiagnostic,
    ReservationLifecycleDiagnosticCode, ReservationLifecycleDiagnosticSeverity,
    ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
    ReservationLifecyclePortError, ValidatedReservationLifecycleApplication,
    ValidatedReservationLifecycleEvent, RESERVATION_LIFECYCLE_CONTRACT_VERSION,
};
pub use runtime_host_dispatch::{
    RuntimeHostBatchExecutionPort, RuntimeHostDispatchError,
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionCancellationSignal,
    RuntimeHostExecutionPort, RuntimeHostExecutionPortError, SchedulerRuntimeHostBatchDispatcher,
    SchedulerRuntimeHostDispatcher,
};
pub use runtime_host_execution::{
    RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostBatchExecutionState,
    RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationDisposition,
    RuntimeHostBatchMemberReservationPolicy, RuntimeHostBatchMemberRetryDisposition,
    RuntimeHostExecutionCancellationContext, RuntimeHostExecutionCancellationOwner,
    RuntimeHostExecutionCancellationSnapshot, RuntimeHostExecutionCancellationState,
    RuntimeHostExecutionContractError, RuntimeHostExecutionDiagnostic,
    RuntimeHostExecutionDiagnosticCode, RuntimeHostExecutionDiagnosticSeverity,
    RuntimeHostExecutionInput, RuntimeHostExecutionInputValue,
    RuntimeHostExecutionMediaArtifactRef, RuntimeHostExecutionOutput,
    RuntimeHostExecutionOutputValue, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, RuntimeHostExecutionTerminalMetadata,
    ValidatedRuntimeHostBatchExecutionRequest, ValidatedRuntimeHostBatchExecutionResponse,
    ValidatedRuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionResponse,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};
pub use runtime_session_load::{
    RuntimeSessionLoadProofContractError, ValidatedWorkflowSessionRuntimeLoadProof,
    WorkflowSessionRuntimeLoadProof, WorkflowSessionRuntimeLoadProofDiagnosticPhase,
    WorkflowSessionRuntimeLoadProofReadinessState, RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
};

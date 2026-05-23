//! Shared runtime-host execution boundary contracts.
//!
//! This crate owns the serialized request/response DTOs, validation wrappers,
//! typed errors, runtime-host execution port, and scheduler dispatch helper
//! used between workflow orchestration and host runtime execution. It does not
//! own scheduler policy, workflow orchestration, runtime loading, Pumas
//! load-target resolution, node-engine execution, concrete I/O, or Tokio
//! runtime lifecycle.

mod runtime_host_dispatch;
mod runtime_host_execution;

pub use runtime_host_dispatch::{
    RuntimeHostDispatchError, RuntimeHostExecutionPort, RuntimeHostExecutionPortError,
    SchedulerRuntimeHostDispatcher,
};
pub use runtime_host_execution::{
    RuntimeHostExecutionContractError, RuntimeHostExecutionDiagnostic,
    RuntimeHostExecutionDiagnosticCode, RuntimeHostExecutionDiagnosticSeverity,
    RuntimeHostExecutionRequest, RuntimeHostExecutionResponse, RuntimeHostExecutionState,
    ValidatedRuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionResponse,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

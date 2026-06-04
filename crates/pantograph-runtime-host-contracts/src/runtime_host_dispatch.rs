use std::sync::Arc;

use async_trait::async_trait;
use pantograph_scheduler::SchedulerRuntimeHandoff;
use thiserror::Error;

use crate::{
    RuntimeHostExecutionCancellationContext, RuntimeHostExecutionCancellationSnapshot,
    RuntimeHostExecutionCancellationState, RuntimeHostExecutionContractError,
    RuntimeHostExecutionInput, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    ValidatedRuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionResponse,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

/// Runtime-host execution port called by scheduler dispatch.
#[async_trait]
pub trait RuntimeHostExecutionPort: Send + Sync {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError>;
}

pub trait RuntimeHostExecutionCancellationSignal: Send + Sync {
    fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot;
}

#[derive(Clone)]
#[must_use]
pub struct RuntimeHostExecutionCancellationHandle {
    signal: Arc<dyn RuntimeHostExecutionCancellationSignal>,
}

impl RuntimeHostExecutionCancellationHandle {
    pub fn running(context: RuntimeHostExecutionCancellationContext) -> Self {
        let snapshot = RuntimeHostExecutionCancellationSnapshot {
            cancellation_context_id: context.cancellation_context_id,
            state: RuntimeHostExecutionCancellationState::Running,
            reason: None,
        };
        Self {
            signal: Arc::new(StaticRuntimeHostExecutionCancellationSignal { snapshot }),
        }
    }

    pub fn with_signal(signal: Arc<dyn RuntimeHostExecutionCancellationSignal>) -> Self {
        Self { signal }
    }

    pub fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
        self.signal.snapshot()
    }
}

#[derive(Debug)]
struct StaticRuntimeHostExecutionCancellationSignal {
    snapshot: RuntimeHostExecutionCancellationSnapshot,
}

impl RuntimeHostExecutionCancellationSignal for StaticRuntimeHostExecutionCancellationSignal {
    fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
        self.snapshot.clone()
    }
}

/// Scheduler-side dispatcher for runtime-host execution requests.
#[derive(Clone)]
#[must_use]
pub struct SchedulerRuntimeHostDispatcher {
    port: Arc<dyn RuntimeHostExecutionPort>,
}

impl SchedulerRuntimeHostDispatcher {
    pub fn new(port: Arc<dyn RuntimeHostExecutionPort>) -> Self {
        Self { port }
    }

    pub async fn dispatch(
        &self,
        execution_request_id: impl Into<String>,
        handoff: SchedulerRuntimeHandoff,
        materialized_inputs: Vec<RuntimeHostExecutionInput>,
    ) -> Result<ValidatedRuntimeHostExecutionResponse, RuntimeHostDispatchError> {
        let execution_request_id = execution_request_id.into();
        let cancellation_context =
            RuntimeHostExecutionCancellationContext::workflow_service(&execution_request_id);
        let cancellation =
            RuntimeHostExecutionCancellationHandle::running(cancellation_context.clone());
        self.dispatch_with_cancellation(
            execution_request_id,
            handoff,
            materialized_inputs,
            cancellation_context,
            cancellation,
        )
        .await
    }

    pub async fn dispatch_with_cancellation(
        &self,
        execution_request_id: impl Into<String>,
        handoff: SchedulerRuntimeHandoff,
        materialized_inputs: Vec<RuntimeHostExecutionInput>,
        cancellation_context: RuntimeHostExecutionCancellationContext,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<ValidatedRuntimeHostExecutionResponse, RuntimeHostDispatchError> {
        let request = RuntimeHostExecutionRequest {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            execution_request_id: execution_request_id.into(),
            cancellation_context,
            handoff,
            materialized_inputs,
        };
        let validated_request = ValidatedRuntimeHostExecutionRequest::try_from(request)?;
        let response = self
            .port
            .execute_runtime_host_request(validated_request.as_ref().clone(), cancellation)
            .await?;
        validate_response_matches_request(validated_request.as_ref(), &response)?;
        ValidatedRuntimeHostExecutionResponse::try_from(response)
            .map_err(RuntimeHostDispatchError::ResponseContract)
    }
}

fn validate_response_matches_request(
    request: &RuntimeHostExecutionRequest,
    response: &RuntimeHostExecutionResponse,
) -> Result<(), RuntimeHostDispatchError> {
    let handoff = &request.handoff;
    if response.execution_request_id != request.execution_request_id {
        return Err(RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "execution_request_id",
            reason: "runtime-host response must match scheduler dispatch request id",
        });
    }
    if response.workflow_id != handoff.workflow_id {
        return Err(RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "workflow_id",
            reason: "runtime-host response must match scheduler handoff workflow id",
        });
    }
    if response.workflow_run_id != handoff.workflow_run_id {
        return Err(RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "workflow_run_id",
            reason: "runtime-host response must match scheduler handoff workflow run id",
        });
    }
    if response.node_id != handoff.node_id {
        return Err(RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "node_id",
            reason: "runtime-host response must match scheduler handoff node id",
        });
    }
    if response.task_id != handoff.task_id {
        return Err(RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "task_id",
            reason: "runtime-host response must match scheduler handoff task id",
        });
    }
    Ok(())
}

/// Runtime-host execution port failure.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeHostExecutionPortError {
    #[error("runtime-host execution port failed: {message}")]
    ExecutionFailed { message: String },
}

/// Scheduler dispatch to runtime-host execution failure.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeHostDispatchError {
    #[error("invalid runtime-host execution request")]
    RequestContract(#[from] RuntimeHostExecutionContractError),
    #[error(transparent)]
    Port(#[from] RuntimeHostExecutionPortError),
    #[error("invalid runtime-host execution response")]
    ResponseContract(RuntimeHostExecutionContractError),
    #[error("invalid runtime-host response correlation `{field}`: {reason}")]
    InvalidResponseCorrelation {
        field: &'static str,
        reason: &'static str,
    },
}

#[cfg(test)]
#[path = "runtime_host_dispatch_tests.rs"]
mod tests;

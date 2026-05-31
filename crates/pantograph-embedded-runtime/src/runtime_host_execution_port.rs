use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, ValidatedRuntimeHostExecutionRequest,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

use crate::runtime_host_load_target::{
    RuntimeHostPumasLoadTargetError, RuntimeHostPumasLoadTargetResolver,
};

const MISSING_LOAD_TARGET_RESOLVER_HINT: &str =
    "embedded_runtime_host_execution_port.missing_load_target_resolver";
const LOAD_TARGET_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.pumas_load_target_unavailable";
const RUNTIME_EXECUTION_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.runtime_execution_unavailable";

pub(crate) struct EmbeddedRuntimeHostExecutionPort {
    load_target_resolver: Option<RuntimeHostPumasLoadTargetResolver>,
}

impl EmbeddedRuntimeHostExecutionPort {
    #[must_use]
    pub(crate) fn fail_closed() -> Self {
        Self {
            load_target_resolver: None,
        }
    }

    #[must_use]
    pub(crate) fn with_load_target_resolver(
        load_target_resolver: RuntimeHostPumasLoadTargetResolver,
    ) -> Self {
        Self {
            load_target_resolver: Some(load_target_resolver),
        }
    }
}

#[async_trait]
impl RuntimeHostExecutionPort for EmbeddedRuntimeHostExecutionPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        let validated_request =
            ValidatedRuntimeHostExecutionRequest::try_from(request).map_err(|error| {
                RuntimeHostExecutionPortError::ExecutionFailed {
                    message: format!("embedded runtime-host request failed validation: {error}"),
                }
            })?;

        let Some(load_target_resolver) = self.load_target_resolver.as_ref() else {
            return Ok(rejected_response(
                validated_request.as_ref(),
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired,
                "embedded runtime-host execution requires a Pumas load-target resolver",
                MISSING_LOAD_TARGET_RESOLVER_HINT,
            ));
        };

        match load_target_resolver.resolve(&validated_request).await {
            Ok(_load_target) => Ok(rejected_response(
                validated_request.as_ref(),
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host execution has resolved the Pumas load target, but runtime-specific execution is not wired yet",
                RUNTIME_EXECUTION_UNAVAILABLE_HINT,
            )),
            Err(error) => Ok(rejected_response(
                validated_request.as_ref(),
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                &load_target_error_message(error),
                LOAD_TARGET_UNAVAILABLE_HINT,
            )),
        }
    }
}

fn rejected_response(
    request: &RuntimeHostExecutionRequest,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostExecutionResponse {
    RuntimeHostExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        execution_request_id: request.execution_request_id.clone(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        state: RuntimeHostExecutionState::Rejected,
        outputs: Vec::new(),
        diagnostics: vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Error,
            code: diagnostic_code,
            message: message.to_string(),
            hint: Some(hint.to_string()),
        }],
        terminal_metadata: None,
    }
}

fn load_target_error_message(error: RuntimeHostPumasLoadTargetError) -> String {
    format!("embedded runtime-host Pumas load-target resolution failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_runtime_host_contracts::RuntimeHostExecutionContractError;

    #[tokio::test]
    async fn fail_closed_port_rejects_without_load_target_resolver() {
        let request = runtime_host_request_fixture();
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_request(request)
            .await
            .expect("missing resolver should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert_eq!(response.execution_request_id, "runtime-host.request.001");
        assert!(response.outputs.is_empty());
        assert_eq!(response.diagnostics.len(), 1);
        let diagnostic = &response.diagnostics[0];
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(MISSING_LOAD_TARGET_RESOLVER_HINT)
        );
    }

    #[tokio::test]
    async fn port_rejects_invalid_requests_as_port_errors() {
        let mut request = runtime_host_request_fixture();
        request.execution_request_id.clear();
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let error = port
            .execute_runtime_host_request(request)
            .await
            .expect_err("invalid request should fail the port");

        assert!(matches!(
            error,
            RuntimeHostExecutionPortError::ExecutionFailed { .. }
        ));
        assert!(error
            .to_string()
            .contains("embedded runtime-host request failed validation"));
        assert!(error.to_string().contains(
            &RuntimeHostExecutionContractError::InvalidIdentifier {
                field: "execution_request_id"
            }
            .to_string()
        ));
    }

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture should deserialize")
    }
}

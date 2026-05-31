use std::sync::Arc;

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, ValidatedRuntimeHostExecutionRequest,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

use crate::runtime_host_load_target::{
    RuntimeHostLoadTargetResolver, RuntimeHostPumasLoadTargetError,
    RuntimeHostPumasLoadTargetResolver,
};
use crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSink;

const MISSING_LOAD_TARGET_RESOLVER_HINT: &str =
    "embedded_runtime_host_execution_port.missing_load_target_resolver";
const LOAD_TARGET_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.pumas_load_target_unavailable";
const MISSING_MEDIA_ARTIFACT_SINK_HINT: &str =
    "embedded_runtime_host_execution_port.missing_media_artifact_sink";
const RUNTIME_EXECUTION_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.runtime_execution_unavailable";

pub(crate) struct EmbeddedRuntimeHostExecutionPort {
    load_target_resolver: Option<Arc<dyn RuntimeHostLoadTargetResolver>>,
    media_artifact_sink: Option<Arc<dyn RuntimeHostMediaArtifactSink>>,
}

impl EmbeddedRuntimeHostExecutionPort {
    #[must_use]
    pub(crate) fn fail_closed() -> Self {
        Self {
            load_target_resolver: None,
            media_artifact_sink: None,
        }
    }

    #[must_use]
    pub(crate) fn with_load_target_resolver(
        load_target_resolver: RuntimeHostPumasLoadTargetResolver,
    ) -> Self {
        Self {
            load_target_resolver: Some(Arc::new(load_target_resolver)),
            media_artifact_sink: None,
        }
    }

    #[must_use]
    pub(crate) fn with_runtime_dependencies(
        load_target_resolver: Arc<dyn RuntimeHostLoadTargetResolver>,
        media_artifact_sink: Arc<dyn RuntimeHostMediaArtifactSink>,
    ) -> Self {
        Self {
            load_target_resolver: Some(load_target_resolver),
            media_artifact_sink: Some(media_artifact_sink),
        }
    }

    #[cfg(test)]
    fn with_load_target_resolver_only_for_test(
        load_target_resolver: Arc<dyn RuntimeHostLoadTargetResolver>,
    ) -> Self {
        Self {
            load_target_resolver: Some(load_target_resolver),
            media_artifact_sink: None,
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
            Ok(_load_target) => {
                let Some(_media_artifact_sink) = self.media_artifact_sink.as_ref() else {
                    return Ok(rejected_response(
                        validated_request.as_ref(),
                        RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                        "embedded runtime-host execution requires a media artifact sink before generated media can be returned",
                        MISSING_MEDIA_ARTIFACT_SINK_HINT,
                    ));
                };
                Ok(rejected_response(
                    validated_request.as_ref(),
                    RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                    "embedded runtime-host execution has resolved the Pumas load target and media artifact sink, but runtime-specific execution is not wired yet",
                    RUNTIME_EXECUTION_UNAVAILABLE_HINT,
                ))
            }
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
    use pumas_library::models::{
        AssetValidationState, PackageArtifactKind, PumasArtifactLoadPathKind,
        PumasArtifactLoadTarget, StorageKind,
    };

    use crate::runtime_host_media_artifact_sink::{
        RuntimeHostImageArtifactWriteRequest, RuntimeHostMediaArtifactSinkError,
    };

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

    #[tokio::test]
    async fn port_rejects_after_load_target_when_media_sink_is_missing() {
        let request = runtime_host_request_fixture();
        let port = EmbeddedRuntimeHostExecutionPort::with_load_target_resolver_only_for_test(
            Arc::new(ReadyLoadTargetResolver),
        );

        let response = port
            .execute_runtime_host_request(request)
            .await
            .expect("missing media sink should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(MISSING_MEDIA_ARTIFACT_SINK_HINT)
        );
    }

    #[tokio::test]
    async fn port_rejects_runtime_unavailable_after_required_dependencies_exist() {
        let request = runtime_host_request_fixture();
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(ReadyLoadTargetResolver),
            Arc::new(UnusedMediaArtifactSink),
        );

        let response = port
            .execute_runtime_host_request(request)
            .await
            .expect("unwired runtime execution should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(RUNTIME_EXECUTION_UNAVAILABLE_HINT)
        );
    }

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture should deserialize")
    }

    struct ReadyLoadTargetResolver;

    #[async_trait]
    impl RuntimeHostLoadTargetResolver for ReadyLoadTargetResolver {
        async fn resolve(
            &self,
            _request: &ValidatedRuntimeHostExecutionRequest,
        ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
            Ok(PumasArtifactLoadTarget {
                model_ref: pumas_library::models::PumasModelRef {
                    model_id: "pumas://models/juggernaut-xl-v10".to_string(),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: Some("juggernaut-xl-v10/diffusers".to_string()),
                    ..Default::default()
                },
                artifact_kind: PackageArtifactKind::DiffusersBundle,
                local_load_path: "/host-only/pumas/juggernaut-xl-v10".to_string(),
                load_path_kind: PumasArtifactLoadPathKind::Directory,
                library_root_id: Some("default".to_string()),
                storage_kind: StorageKind::LibraryOwned,
                validation_state: AssetValidationState::Valid,
                content_fingerprint: Some("sha256:abc".to_string()),
                package_facts_contract_version: Some(2),
            })
        }
    }

    struct UnusedMediaArtifactSink;

    impl RuntimeHostMediaArtifactSink for UnusedMediaArtifactSink {
        fn write_image_output(
            &self,
            _request: RuntimeHostImageArtifactWriteRequest<'_>,
        ) -> Result<
            pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
            RuntimeHostMediaArtifactSinkError,
        > {
            panic!("media sink must not be called before runtime execution is wired")
        }
    }
}

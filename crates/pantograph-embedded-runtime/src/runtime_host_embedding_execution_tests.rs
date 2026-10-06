use super::*;
use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPort, RuntimeHostExecutionState,
    ValidatedRuntimeHostExecutionResponse,
};
use std::{path::Path, sync::Arc};

pub(crate) fn fixture(
    width: usize,
) -> (
    tempfile::TempDir,
    RuntimeHostExecutionRequest,
    ResolvedModelPackageFacts,
    inference::PumasArtifactLoadTarget,
) {
    fn copy(source: &Path, destination: &Path) {
        std::fs::create_dir_all(destination).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &destination.join(entry.file_name()));
            } else {
                std::fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
            }
        }
    }
    let directory = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../inference/tests/fixtures/candle_bert/bert-{width}"
        )),
        directory.path(),
    );
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../../inference/tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
    )).unwrap();
    package.model_ref.model_id = format!("embedding/test/Synthetic-BERT-{width}");
    package.model_ref.revision = Some(format!("untrained-seed-{}", 171 + width));
    package.model_ref.selected_artifact_path = None;
    let reference = serde_json::to_value(&package.model_ref).unwrap();
    let mut request: serde_json::Value = serde_json::from_str(include_str!(
        "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    )).unwrap();
    request["materialized_inputs"] = serde_json::json!([
        {"port_id":"text", "value":{"value_type":"string", "value":"hello world"}}
    ]);
    for pointer in [
        "/handoff/task_intent",
        "/handoff/dispatch_decision/task_intent",
    ] {
        let intent = request.pointer_mut(pointer).unwrap();
        intent["task_type"] = serde_json::json!("embedding");
        intent["model_ref"] = reference.clone();
        intent["constraints"] =
            serde_json::json!({"requested_runtime_id":"candle", "requested_device_id":"cpu"});
        intent["trait_settings"] = serde_json::json!([]);
    }
    for pointer in [
        "/handoff/readiness_proof/preflight_result/identity_key",
        "/handoff/dispatch_decision/readiness_proof/preflight_result/identity_key",
    ] {
        let key = request.pointer_mut(pointer).unwrap();
        key["task_id"] = serde_json::json!("embedding");
        key["model_ref"] = reference.clone();
        key["scheduler_intent"] =
            serde_json::json!({"requested_runtime_id":"candle", "requested_device_id":"cpu"});
    }
    let decision = &mut request["handoff"]["dispatch_decision"];
    decision["selected_model_ref"] = reference.clone();
    decision["selected_runtime_id"] = serde_json::json!("candle");
    decision["selected_runtime_variant_id"] = serde_json::json!("candle.cpu");
    decision["selected_device_ids"] = serde_json::json!(["cpu"]);
    for reservation in decision["reservations"].as_array_mut().unwrap() {
        reservation["device_id"] = serde_json::json!("cpu");
    }
    let target = inference::PumasArtifactLoadTarget {
        model_ref: serde_json::from_value(reference).unwrap(),
        artifact_kind: inference::ModelArtifactKind::HfCompatibleDirectory,
        local_load_path: directory.path().to_str().unwrap().into(),
        load_path_kind: inference::PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("synthetic-fixture-root".into()),
        storage_kind: inference::ModelStorageKind::LibraryOwned,
        validation_state: inference::ModelValidationState::Valid,
        content_fingerprint: None,
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    (
        directory,
        serde_json::from_value(request).unwrap(),
        package,
        target,
    )
}

pub(crate) struct Target(pub(crate) pumas_library::models::PumasArtifactLoadTarget);
#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for Target {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        Ok(self.0.clone())
    }
}
pub(crate) struct Package(pub(crate) ResolvedModelPackageFacts);
#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for Package {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        Ok(self.0.clone())
    }
}
pub(crate) struct UnusedMediaSink;
impl crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSink for UnusedMediaSink {
    fn write_image_output(
        &self,
        _: crate::runtime_host_media_artifact_sink::RuntimeHostImageArtifactWriteRequest<'_>,
    ) -> Result<
        pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
        crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSinkError,
    > {
        panic!("embedding output must not require a media artifact");
    }
}

#[tokio::test]
async fn host_embedding_returns_actual_candle_vectors_and_replaces_model_identity() {
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::candle::CandleBackend::new()),
        "Candle",
    ));
    for width in [8, 12] {
        let (_directory, request, package, target) = fixture(width);
        let expected_identity = request
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .selected_model_ref
            .clone();
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap())), Arc::new(Package(package)), Arc::new(UnusedMediaSink), gateway.clone());
        let cancellation =
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{:?}",
            response.diagnostics
        );
        let response = ValidatedRuntimeHostExecutionResponse::try_from(response)
            .unwrap()
            .into_inner();
        let RuntimeHostExecutionOutputValue::Json(vector) = &response.outputs[0].value else {
            panic!("structured vector");
        };
        let values = vector.as_array().unwrap();
        assert_eq!(values.len(), width);
        let golden: serde_json::Value = serde_json::from_slice(
            &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../inference/tests/fixtures/candle_bert/bert-{width}/golden.json"
            )))
            .unwrap(),
        )
        .unwrap();
        for (actual, expected) in values
            .iter()
            .zip(golden["single_vectors"][0].as_array().unwrap())
        {
            assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-5);
        }
        let RuntimeHostExecutionOutputValue::Json(metadata) = &response.outputs[1].value else {
            panic!("metadata");
        };
        assert_eq!(
            metadata["model_ref"],
            serde_json::to_value(expected_identity).unwrap()
        );
        assert_eq!(metadata["token_count"], 4);
        assert_eq!(metadata["index"], 0);
        assert_eq!(metadata["runtime_variant_id"], "candle.cpu");
    }
    gateway.stop().await.unwrap();
}

#[tokio::test]
async fn host_embedding_rejects_identity_recipe_and_precancellation_without_vectors() {
    for fault in ["identity", "recipe", "cancel"] {
        let (directory, request, package, mut target) = fixture(8);
        if fault == "identity" {
            target.model_ref.revision = Some("wrong-revision".into());
        }
        if fault == "recipe" {
            std::fs::remove_file(directory.path().join("modules.json")).unwrap();
        }
        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(inference::backend::candle::CandleBackend::new()),
            "Candle",
        ));
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap())), Arc::new(Package(package)), Arc::new(UnusedMediaSink), gateway.clone());
        let cancellation = if fault == "cancel" {
            RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Cancelled(
                request.cancellation_context.cancellation_context_id.clone(),
            )))
        } else {
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone())
        };
        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .unwrap();
        assert_ne!(response.state, RuntimeHostExecutionState::Completed);
        assert!(response.outputs.is_empty());
        assert!(!gateway.is_ready().await);
        response.validate().unwrap();
    }
}

struct Cancelled(String);
impl pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSignal for Cancelled {
    fn snapshot(
        &self,
    ) -> pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
        pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
            cancellation_context_id: self.0.clone(),
            state: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::CancellationRequested,
            reason: Some("fixture cancellation".into()),
        }
    }
}

fn embedding_batch_request(
    request: RuntimeHostExecutionRequest,
) -> pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionRequest,
        RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationPolicy,
    };
    let members = ["hello world", "hello"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let mut request = request.clone();
            request.execution_request_id = format!("embedding.execution.{index}");
            request.handoff.workflow_id = format!("workflow.embedding.{index}").parse().unwrap();
            request.handoff.workflow_run_id = format!("run.embedding.{index}").parse().unwrap();
            request.handoff.node_id = format!("node.embedding.{index}").parse().unwrap();
            request.handoff.task_id = format!("task.embedding.{index}").parse().unwrap();
            request.handoff.task_intent.workflow_id = request.handoff.workflow_id.clone();
            request.handoff.task_intent.workflow_run_id = request.handoff.workflow_run_id.clone();
            request.handoff.task_intent.node_id = request.handoff.node_id.clone();
            request.handoff.task_intent.task_id = request.handoff.task_id.clone();
            let context = &mut request.handoff.readiness_proof.execution_context;
            context.workflow_id = request.handoff.workflow_id.as_str().parse().unwrap();
            context.workflow_run_id = request.handoff.workflow_run_id.as_str().parse().unwrap();
            context.node_id = request.handoff.node_id.as_str().parse().unwrap();
            context.scheduler_task_id = request.handoff.task_id.as_str().parse().unwrap();
            let decision = request.handoff.dispatch_decision.as_mut().unwrap();
            decision.workflow_id = request.handoff.workflow_id.clone();
            decision.workflow_run_id = request.handoff.workflow_run_id.clone();
            decision.node_id = request.handoff.node_id.clone();
            decision.task_id = request.handoff.task_id.clone();
            decision.task_intent = request.handoff.task_intent.clone();
            decision.readiness_proof = request.handoff.readiness_proof.clone();
            for reservation in &mut decision.reservations {
                reservation.workflow_run_id = decision.workflow_run_id.clone();
                reservation.task_id = decision.task_id.clone();
            }
            request.materialized_inputs[0].value =
                RuntimeHostExecutionInputValue::String(text.into());
            RuntimeHostBatchExecutionMemberRequest {
                execution_request_id: request.execution_request_id,
                assignment_id: format!("assignment.embedding.{index}"),
                handoff: request.handoff,
                materialized_inputs: request.materialized_inputs,
                timeout_ms: None,
                failure_policy: RuntimeHostBatchMemberFailurePolicy::TerminalOnly,
                reservation_policy: RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal,
            }
        })
        .collect();
    let batch = RuntimeHostBatchExecutionRequest {
        contract_version: pantograph_runtime_host_contracts::RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: "embedding.batch.001".into(), anchor_execution_request_id: "embedding.execution.0".into(),
        cancellation_context: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationContext::workflow_service("embedding.batch.001"),
        members,
    };
    batch.validate().unwrap();
    batch
}

#[tokio::test]
async fn embedding_host_envelope_returns_per_member_candle_cpu_goldens_and_identity() {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
        RuntimeHostBatchExecutionState,
    };
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::candle::CandleBackend::new()),
        "Candle",
    ));
    for width in [8, 12] {
        let (_directory, request, package, target) = fixture(width);
        let batch = embedding_batch_request(request);
        let expected = batch.members.clone();
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap())),
            Arc::new(Package(package)), Arc::new(UnusedMediaSink), gateway.clone());
        let cancellation =
            RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone());
        let response = port
            .execute_runtime_host_batch_request(batch, cancellation)
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostBatchExecutionState::Completed,
            "{:?}",
            response.diagnostics
        );
        response.validate().unwrap();
        assert_eq!(response.batch_execution_request_id, "embedding.batch.001");
        assert_eq!(response.members.len(), 2);
        let golden: serde_json::Value = serde_json::from_slice(
            &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../inference/tests/fixtures/candle_bert/bert-{width}/golden.json"
            )))
            .unwrap(),
        )
        .unwrap();
        for (index, (member, expected)) in response.members.iter().zip(&expected).enumerate() {
            assert_eq!(
                member.state,
                RuntimeHostBatchExecutionMemberState::Completed,
                "{:?}",
                member.diagnostics
            );
            assert_eq!(member.execution_request_id, expected.execution_request_id);
            assert_eq!(member.assignment_id, expected.assignment_id);
            assert_eq!(member.workflow_id, expected.handoff.workflow_id);
            assert_eq!(member.workflow_run_id, expected.handoff.workflow_run_id);
            assert_eq!(member.node_id, expected.handoff.node_id);
            assert_eq!(member.task_id, expected.handoff.task_id);
            assert_eq!(
                member
                    .outputs
                    .iter()
                    .map(|output| output.port_id.as_str())
                    .collect::<Vec<_>>(),
                ["embedding", "metadata", "usage"]
            );
            let RuntimeHostExecutionOutputValue::Json(vector) = &member.outputs[0].value else {
                panic!("embedding JSON vector")
            };
            let values = vector.as_array().unwrap();
            assert_eq!(values.len(), width);
            for (actual, reference) in values
                .iter()
                .zip(golden["single_vectors"][index].as_array().unwrap())
            {
                assert!((actual.as_f64().unwrap() - reference.as_f64().unwrap()).abs() < 1e-5);
            }
            let token_count = if index == 0 { 4 } else { 3 };
            let RuntimeHostExecutionOutputValue::Json(metadata) = &member.outputs[1].value else {
                panic!("embedding metadata JSON")
            };
            assert_eq!(
                metadata["model_ref"],
                serde_json::to_value(
                    &expected
                        .handoff
                        .dispatch_decision
                        .as_ref()
                        .unwrap()
                        .selected_model_ref
                )
                .unwrap()
            );
            assert_eq!(metadata["vector_length"], width);
            assert_eq!(metadata["index"], 0);
            assert_eq!(metadata["token_count"], token_count);
            assert_eq!(metadata["runtime_variant_id"], "candle.cpu");
            assert_eq!(metadata["device_ids"], serde_json::json!(["cpu"]));
            let RuntimeHostExecutionOutputValue::Json(usage) = &member.outputs[2].value else {
                panic!("embedding usage JSON")
            };
            assert_eq!(
                *usage,
                serde_json::json!({"prompt_tokens": token_count, "total_tokens": token_count})
            );
        }
    }
    gateway.stop().await.unwrap();
}

struct NeverResolve;
#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for NeverResolve {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        panic!("invalid or pre-cancelled embedding must reject before target resolution")
    }
}
#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for NeverResolve {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        panic!("invalid or pre-cancelled embedding must reject before package resolution")
    }
}

#[tokio::test]
async fn embedding_host_envelope_rejects_invalid_shape_and_precancellation_before_resolvers() {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
        RuntimeHostBatchExecutionState, RuntimeHostExecutionInput,
    };
    for fault in ["cancel", "blank", "wrong_port", "wrong_type", "extra_input"] {
        let (_directory, request, _package, _target) = fixture(8);
        let mut batch = embedding_batch_request(request);
        for member in &mut batch.members {
            match fault {
                "blank" => {
                    member.materialized_inputs[0].value =
                        RuntimeHostExecutionInputValue::String(" \n".into())
                }
                "wrong_port" => member.materialized_inputs[0].port_id = "prompt".into(),
                "wrong_type" => {
                    member.materialized_inputs[0].value = RuntimeHostExecutionInputValue::U64(8)
                }
                "extra_input" => member.materialized_inputs.push(RuntimeHostExecutionInput {
                    port_id: "seed".into(),
                    value: RuntimeHostExecutionInputValue::U64(42),
                }),
                _ => {}
            }
        }
        let cancellation = if fault == "cancel" {
            RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Cancelled(
                batch.cancellation_context.cancellation_context_id.clone(),
            )))
        } else {
            RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone())
        };
        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(inference::backend::candle::CandleBackend::new()),
            "Candle",
        ));
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(Arc::new(NeverResolve), Arc::new(NeverResolve), Arc::new(UnusedMediaSink), gateway.clone());
        let response = port
            .execute_runtime_host_batch_request(batch, cancellation)
            .await
            .unwrap();
        let (batch_state, member_state) = if fault == "cancel" {
            (
                RuntimeHostBatchExecutionState::Cancelled,
                RuntimeHostBatchExecutionMemberState::Cancelled,
            )
        } else {
            (
                RuntimeHostBatchExecutionState::Rejected,
                RuntimeHostBatchExecutionMemberState::Rejected,
            )
        };
        assert_eq!(
            response.state, batch_state,
            "{fault}: {:?}",
            response.diagnostics
        );
        response.validate().unwrap();
        assert_eq!(response.members.len(), 2);
        assert!(response
            .members
            .iter()
            .all(|member| member.state == member_state && member.outputs.is_empty()));
        assert!(!gateway.is_ready().await);
    }
}

#[test]
fn embedding_output_projector_rejects_invalid_result_shape_without_outputs() {
    use inference::InferenceEmbeddingResult;
    let (_directory, request, _package, _target) = fixture(8);
    let valid = InferenceEmbeddingResult {
        vector: vec![0.25, -0.5],
        token_count: Some(4),
        index: Some(0),
    };
    for fault in [
        "missing",
        "multiple",
        "missing_index",
        "wrong_index",
        "missing_tokens",
        "empty",
        "oversized",
        "nan",
        "infinity",
        "negative_infinity",
    ] {
        let mut embedding = valid.clone();
        match fault {
            "missing_index" => embedding.index = None,
            "wrong_index" => embedding.index = Some(1),
            "missing_tokens" => embedding.token_count = None,
            "empty" => embedding.vector.clear(),
            "oversized" => embedding.vector = vec![0.0; 4097],
            "nan" => embedding.vector[0] = f32::NAN,
            "infinity" => embedding.vector[0] = f32::INFINITY,
            "negative_infinity" => embedding.vector[0] = f32::NEG_INFINITY,
            _ => {}
        }
        let embeddings = match fault {
            "missing" => vec![],
            "multiple" => vec![embedding.clone(), embedding],
            _ => vec![embedding],
        };
        let result = InferenceExecutionResult::Embedding {
            embeddings,
            usage: None,
            option_diagnostics: Vec::new(),
        };
        assert!(
            matches!(
                embedding_outputs(&request, result),
                Err(RuntimeHostEmbeddingProjectionError::InvalidResult)
            ),
            "{fault}"
        );
    }
    let wrong_kind = InferenceExecutionResult::TextGeneration {
        text: "not an embedding".into(),
        usage: None,
        cache_handle_id: None,
        option_diagnostics: Vec::new(),
    };
    assert!(matches!(
        embedding_outputs(&request, wrong_kind),
        Err(RuntimeHostEmbeddingProjectionError::InvalidResult)
    ));
}

#[test]
fn embedding_output_projector_preserves_one_finite_vector_metadata_and_optional_usage() {
    use inference::{InferenceEmbeddingResult, InferenceUsage};
    let (_directory, request, _package, _target) = fixture(8);
    for usage in [
        None,
        Some(InferenceUsage {
            prompt_tokens: Some(4),
            completion_tokens: None,
            total_tokens: Some(4),
        }),
    ] {
        let result = InferenceExecutionResult::Embedding {
            embeddings: vec![InferenceEmbeddingResult {
                vector: vec![0.25, -0.5],
                token_count: Some(4),
                index: Some(0),
            }],
            usage: usage.clone(),
            option_diagnostics: Vec::new(),
        };
        let outputs = embedding_outputs(&request, result).unwrap();
        assert_eq!(outputs.len(), if usage.is_some() { 3 } else { 2 });
        assert_eq!(outputs[0].port_id, "embedding");
        assert_eq!(
            outputs[0].value,
            RuntimeHostExecutionOutputValue::Json(serde_json::json!([0.25, -0.5]))
        );
        let RuntimeHostExecutionOutputValue::Json(metadata) = &outputs[1].value else {
            panic!("structured metadata")
        };
        assert_eq!(
            metadata["model_ref"],
            serde_json::to_value(
                &request
                    .handoff
                    .dispatch_decision
                    .as_ref()
                    .unwrap()
                    .selected_model_ref
            )
            .unwrap()
        );
        assert_eq!(metadata["vector_length"], 2);
        assert_eq!(metadata["index"], 0);
        assert_eq!(metadata["token_count"], 4);
        assert_eq!(metadata["runtime_variant_id"], "candle.cpu");
        assert_eq!(metadata["device_ids"], serde_json::json!(["cpu"]));
        if let Some(usage) = usage {
            assert_eq!(outputs[2].port_id, "usage");
            assert_eq!(
                outputs[2].value,
                RuntimeHostExecutionOutputValue::Json(serde_json::to_value(usage).unwrap())
            );
        }
    }
}

use super::*;
use crate::runtime_host_embedding_execution::tests::{Package, Target, UnusedMediaSink};
use async_trait::async_trait;
use inference::{
    backend::BackendStartOutcome, AudioTranscriptionRequest, AudioTranscriptionResult,
    BackendCapabilities, BackendConfig, BackendError, ChatChunk, EmbeddingResult, InferenceBackend,
    InferenceGateway, ProcessSpawner, RerankRequest, RerankResponse,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionCancellationSignal,
    RuntimeHostExecutionCancellationSnapshot, RuntimeHostExecutionCancellationState,
    RuntimeHostExecutionPort, RuntimeHostExecutionState,
};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub(crate) fn fixture() -> (
    tempfile::TempDir,
    RuntimeHostExecutionRequest,
    ResolvedModelPackageFacts,
    PumasArtifactLoadTarget,
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("synthetic-asr");
    // No model payload: this file exercises target validation only.
    std::fs::create_dir(&path).unwrap();
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../../inference/tests/fixtures/inference_package_facts/hf_audio_transcription_package_facts.json"
    ))
    .unwrap();
    package.model_ref.revision = Some("synthetic-r1".into());
    package.model_ref.selected_artifact_path = None;
    package.model_ref.model_id = "synthetic/asr-fixture".into();
    package.transformers = None;
    let reference = serde_json::to_value(&package.model_ref).unwrap();
    let mut request: serde_json::Value = serde_json::from_str(include_str!("../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json")).unwrap();
    request["materialized_inputs"] = serde_json::json!([
        {"port_id":"audio","value":{"value_type":"json","value":{"data_base64":include_str!("../../inference/tests/fixtures/selected_audio/tiny_pcm16.base64"),"mime_type":"audio/wav","sample_rate_hz":16000}}},
        {"port_id":"language","value":{"value_type":"string","value":"en"}},
        {"port_id":"prompt","value":{"value_type":"string","value":"fixture context"}},
        {"port_id":"asr_task","value":{"value_type":"string","value":"transcribe"}},
        {"port_id":"chunk_length_s","value":{"value_type":"f64","value":0.5}}
    ]);
    for pointer in [
        "/handoff/task_intent",
        "/handoff/dispatch_decision/task_intent",
    ] {
        let intent = request.pointer_mut(pointer).unwrap();
        intent["task_type"] = serde_json::json!("audio_transcription");
        intent["model_ref"] = reference.clone();
        intent["constraints"] =
            serde_json::json!({"requested_runtime_id":"pytorch","requested_device_id":"cpu"});
        intent["trait_settings"] = serde_json::json!([]);
    }
    for pointer in [
        "/handoff/readiness_proof/preflight_result/identity_key",
        "/handoff/dispatch_decision/readiness_proof/preflight_result/identity_key",
    ] {
        let key = request.pointer_mut(pointer).unwrap();
        key["task_id"] = serde_json::json!("audio_transcription");
        key["model_ref"] = reference.clone();
        key["scheduler_intent"] =
            serde_json::json!({"requested_runtime_id":"pytorch","requested_device_id":"cpu"});
    }
    let selected = &mut request["handoff"]["dispatch_decision"];
    selected["selected_model_ref"] = reference.clone();
    selected["runtime_trait_settings"] = serde_json::json!([]);
    selected["selected_runtime_id"] = serde_json::json!("pytorch");
    selected["selected_runtime_variant_id"] = serde_json::json!("pytorch.cpu");
    selected["selected_device_ids"] = serde_json::json!(["cpu"]);
    for reservation in selected["reservations"].as_array_mut().unwrap() {
        reservation["device_id"] = serde_json::json!("cpu");
    }
    let target = PumasArtifactLoadTarget {
        model_ref: package.model_ref.clone(),
        artifact_kind: inference::ModelArtifactKind::HfCompatibleDirectory,
        local_load_path: path.to_str().unwrap().into(),
        load_path_kind: inference::PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("synthetic-root".into()),
        storage_kind: inference::ModelStorageKind::LibraryOwned,
        validation_state: inference::ModelValidationState::Valid,
        content_fingerprint: Some("synthetic-content-r1".into()),
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    let request: RuntimeHostExecutionRequest = serde_json::from_value(request).unwrap();
    request.validate().unwrap();
    (directory, request, package, target)
}

#[derive(Default)]
pub(crate) struct Capture {
    pub loads: Mutex<Vec<(String, PumasArtifactLoadTarget, BackendExecutionDecision)>>,
    pub requests: Mutex<Vec<AudioTranscriptionRequest>>,
    pub ids: Mutex<Vec<String>>,
    pub entered: tokio::sync::Notify,
    pub release: tokio::sync::Notify,
    pub block: AtomicBool,
    pub block_load: AtomicBool,
    pub completed: AtomicBool,
    pub unsupported_load: AtomicBool,
    pub bad_result: AtomicBool,
}
struct Synthetic(Arc<Capture>);
#[async_trait]
impl InferenceBackend for Synthetic {
    fn name(&self) -> &'static str {
        "PyTorch"
    }
    fn description(&self) -> &'static str {
        "Controlled ASR contract owner; no model"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
        panic!("generic startup forbidden")
    }
    async fn stop(&mut self) -> std::result::Result<(), BackendError> {
        Ok(())
    }
    fn is_ready(&self) -> bool {
        true
    }
    async fn health_check(&self) -> bool {
        true
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn chat_completion_stream(
        &self,
        _: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        panic!("audio cannot use prompt fallback")
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
        panic!("audio cannot use embeddings")
    }
    async fn rerank(&self, _: RerankRequest) -> std::result::Result<RerankResponse, BackendError> {
        panic!("audio cannot use rerank")
    }
    async fn load_selected_audio(
        &mut self,
        request: &InferenceExecutionRequest,
        target: &PumasArtifactLoadTarget,
        decision: &BackendExecutionDecision,
        _: Option<Arc<dyn ProcessSpawner>>,
        _: inference::InferenceExecutionCancellationHandle,
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
        if self.0.unsupported_load.load(Ordering::SeqCst) {
            return Err(BackendError::Config("selected audio unsupported".into()));
        }
        self.0.loads.lock().unwrap().push((
            request.request_id.clone().unwrap(),
            target.clone(),
            decision.clone(),
        ));
        if self.0.block_load.load(Ordering::SeqCst) {
            self.0.entered.notify_one();
            self.0.release.notified().await;
            self.0.completed.store(true, Ordering::SeqCst);
        }
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            ..Default::default()
        })
    }
    async fn selected_audio(
        &self,
        request: AudioTranscriptionRequest,
        id: &str,
        _: &PumasArtifactLoadTarget,
        _: &BackendExecutionDecision,
        cancellation: inference::InferenceExecutionCancellationHandle,
    ) -> std::result::Result<AudioTranscriptionResult, BackendError> {
        self.0.ids.lock().unwrap().push(id.into());
        let result = self.transcribe_audio(request).await;
        if let Some(reason) = cancellation.rejection_message("selected audio") {
            return Err(BackendError::Cancelled(reason));
        }
        result
    }
    async fn transcribe_audio(
        &self,
        request: AudioTranscriptionRequest,
    ) -> std::result::Result<AudioTranscriptionResult, BackendError> {
        self.0.requests.lock().unwrap().push(request);
        if self.0.block.load(Ordering::SeqCst) {
            self.0.entered.notify_one();
            self.0.release.notified().await;
        }
        self.0.completed.store(true, Ordering::SeqCst);
        Ok(AudioTranscriptionResult {
            text: "controlled fixture transcript".into(),
            language: Some("en".into()),
            duration_seconds: Some(if self.0.bad_result.load(Ordering::SeqCst) {
                f32::NAN
            } else {
                0.00025
            }),
            segments: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}
pub(crate) fn gateway(capture: Arc<Capture>) -> Arc<InferenceGateway> {
    Arc::new(InferenceGateway::with_backend(
        Box::new(Synthetic(capture)),
        "PyTorch",
    ))
}
fn port(
    package: ResolvedModelPackageFacts,
    target: PumasArtifactLoadTarget,
    gateway: Arc<InferenceGateway>,
) -> Arc<crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort> {
    Arc::new(crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(Arc::new(Target(serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap())), Arc::new(Package(package)), Arc::new(UnusedMediaSink), gateway))
}
fn running(request: &RuntimeHostExecutionRequest) -> RuntimeHostExecutionCancellationHandle {
    RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone())
}
pub(crate) async fn parent_outputs(
    gateway: Arc<InferenceGateway>,
    reference: &inference::PumasModelRef,
    values: HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    use node_engine::TaskExecutor;
    let mut values = values;
    // The retained parent reads extension controls from its settings schema.
    if let Some(options) = values
        .get("extra_options")
        .and_then(|value| value.as_object())
        .cloned()
    {
        values.insert(
            "inference_settings".into(),
            serde_json::Value::Array(
                options
                    .iter()
                    .map(|(key, value)| serde_json::json!({"key":key,"default":value}))
                    .collect(),
            ),
        );
    }
    values.insert(
        "_data".into(),
        serde_json::json!({"node_type":"llm-inference"}),
    );
    values.insert("task_kind".into(), serde_json::json!("audio_transcription"));
    values.insert(
        "pumas_model_ref".into(),
        serde_json::to_value(reference).unwrap(),
    );
    node_engine::CoreTaskExecutor::new()
        .with_gateway(gateway)
        .execute_task(
            "llm-inference-1",
            values,
            &node_engine::Context::new(),
            &node_engine::ExecutorExtensions::new(),
        )
        .await
        .unwrap()
}
pub(crate) fn input_values(
    request: &RuntimeHostExecutionRequest,
) -> HashMap<String, serde_json::Value> {
    inputs(request).unwrap()
}
fn output_values(
    response: &pantograph_runtime_host_contracts::RuntimeHostExecutionResponse,
) -> HashMap<String, serde_json::Value> {
    response
        .outputs
        .iter()
        .map(|output| {
            (
                output.port_id.clone(),
                match &output.value {
                    RuntimeHostExecutionOutputValue::Json(value) => value.clone(),
                    RuntimeHostExecutionOutputValue::String(value) => serde_json::json!(value),
                    value => panic!("unexpected rerank output {value:?}"),
                },
            )
        })
        .collect()
}

#[tokio::test]
async fn selected_audio_matches_all_eight_parent_outputs_and_normalizes_only_empty_options() {
    for object in [true, false] {
        let (_directory, mut request, package, target) = fixture();
        if !object {
            request.materialized_inputs[0].value = RuntimeHostExecutionInputValue::String(
                include_str!("../../inference/tests/fixtures/selected_audio/tiny_pcm16.base64")
                    .into(),
            );
        }
        request.materialized_inputs.push(
            pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                port_id: "extra_options".into(),
                value: RuntimeHostExecutionInputValue::Json(serde_json::json!({})),
            },
        );
        let capture = Arc::new(Capture::default());
        let gw = gateway(capture.clone());
        let parent = parent_outputs(gw.clone(), &package.model_ref, input_values(&request)).await;
        let response = port(package.clone(), target.clone(), gw)
            .execute_runtime_host_request(request.clone(), running(&request))
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{:?}",
            response.diagnostics
        );
        assert_eq!(output_values(&response), parent);
        assert_eq!(response.outputs.len(), 8);
        let loads = capture.loads.lock().unwrap();
        assert_eq!(loads[0].0, request.execution_request_id);
        assert_eq!(loads[0].1, target);
        assert_eq!(
            loads[0].2.selected_model_ref.as_ref(),
            Some(&package.model_ref)
        );
        let requests = capture.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        let mut original = requests[0].clone();
        assert_eq!(original.extra_options, serde_json::json!({}));
        original.extra_options = serde_json::Value::Null;
        assert_eq!(requests[1], original);
        assert_eq!(capture.ids.lock().unwrap()[0], request.execution_request_id);
    }
}

#[tokio::test]
async fn selected_audio_rejects_unsupported_inputs_before_owner_effects() {
    for (port_id, value) in [
        ("audio", serde_json::json!("UklGRg==")),
        ("audio", serde_json::json!("artifact://sample.wav")),
        (
            "audio",
            serde_json::json!({"data_base64":"invalid","mime_type":"audio/flac"}),
        ),
        (
            "audio",
            serde_json::json!({"data_base64":include_str!("../../inference/tests/fixtures/selected_audio/tiny_pcm16.base64"),"sample_rate_hz":8000}),
        ),
        ("extra_options", serde_json::json!({"stream":true})),
        (
            "extra_options",
            serde_json::json!({"return_timestamps":true}),
        ),
        ("chunk_length_s", serde_json::json!(0.0)),
        ("stream", serde_json::json!(true)),
        ("unknown", serde_json::json!("control")),
    ] {
        let (_directory, mut request, package, target) = fixture();
        request.materialized_inputs.retain(|i| i.port_id != port_id);
        request.materialized_inputs.push(
            pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                port_id: port_id.into(),
                value: match port_id {
                    "stream" => RuntimeHostExecutionInputValue::Bool(true),
                    "chunk_length_s" => {
                        RuntimeHostExecutionInputValue::F64(value.as_number().unwrap().clone())
                    }
                    _ => RuntimeHostExecutionInputValue::Json(value),
                },
            },
        );
        let capture = Arc::new(Capture::default());
        let response = port(package, target, gateway(capture.clone()))
            .execute_runtime_host_request(request.clone(), running(&request))
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Rejected,
            "{port_id}: {:?}",
            response.diagnostics
        );
        assert!(response.outputs.is_empty());
        assert!(capture.loads.lock().unwrap().is_empty());
        assert!(capture.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn selected_audio_requires_exact_identity_known_content_and_supported_owner() {
    for case in [
        "model",
        "revision",
        "artifact",
        "fingerprint",
        "directory",
        "task",
        "custom_code",
        "unsupported_owner",
        "bad_result",
    ] {
        let (_directory, request, mut package, mut target) = fixture();
        let capture = Arc::new(Capture::default());
        match case {
            "model" => target.model_ref.model_id = "wrong-owner".into(),
            "revision" => target.model_ref.revision = Some("wrong".into()),
            "artifact" => target.model_ref.selected_artifact_id = Some("wrong".into()),
            "fingerprint" => target.content_fingerprint = None,
            "directory" => target.local_load_path.push_str("missing"),
            "task" => package.task.task_type_primary = Some("embedding".into()),
            "custom_code" => package.custom_code.requires_custom_code = true,
            "unsupported_owner" => capture.unsupported_load.store(true, Ordering::SeqCst),
            "bad_result" => capture.bad_result.store(true, Ordering::SeqCst),
            _ => unreachable!(),
        }
        let response = port(package, target, gateway(capture.clone()))
            .execute_runtime_host_request(request.clone(), running(&request))
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Failed,
            "{case}: {:?}",
            response.diagnostics
        );
        assert!(response.outputs.is_empty());
        assert_eq!(
            capture.requests.lock().unwrap().len(),
            usize::from(case == "bad_result")
        );
    }
}
struct Cancellation {
    context: String,
    cancel: AtomicBool,
}
impl RuntimeHostExecutionCancellationSignal for Cancellation {
    fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
        RuntimeHostExecutionCancellationSnapshot {
            cancellation_context_id: self.context.clone(),
            state: if self.cancel.load(Ordering::SeqCst) {
                RuntimeHostExecutionCancellationState::CancellationRequested
            } else {
                RuntimeHostExecutionCancellationState::Running
            },
            reason: None,
        }
    }
}
#[tokio::test]
async fn selected_audio_cancellation_waits_for_owner_and_suppresses_outputs() {
    for loading in [false, true] {
        let (_directory, request, package, target) = fixture();
        let capture = Arc::new(Capture::default());
        if loading {
            capture.block_load.store(true, Ordering::SeqCst);
        } else {
            capture.block.store(true, Ordering::SeqCst);
        }
        let gateway = gateway(capture.clone());
        let port = port(package, target, gateway.clone());
        let signal = Arc::new(Cancellation {
            context: request.cancellation_context.cancellation_context_id.clone(),
            cancel: AtomicBool::new(false),
        });
        let cancellation = RuntimeHostExecutionCancellationHandle::with_signal(signal.clone());
        let task = tokio::spawn(async move {
            port.execute_runtime_host_request(request, cancellation)
                .await
                .unwrap()
        });
        tokio::time::timeout(Duration::from_secs(2), capture.entered.notified())
            .await
            .unwrap();
        signal.cancel.store(true, Ordering::SeqCst);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), gateway.is_ready())
                .await
                .is_err(),
            "custody must hold while owner is still running"
        );
        assert!(!task.is_finished());
        capture.release.notify_one();
        let response = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        assert!(capture.completed.load(Ordering::SeqCst));
        assert!(gateway.is_ready().await);
    }
}
#[tokio::test]
async fn selected_audio_caller_abort_keeps_owner_custody_until_completion() {
    for loading in [false, true] {
        let (_directory, request, package, target) = fixture();
        let capture = Arc::new(Capture::default());
        if loading {
            capture.block_load.store(true, Ordering::SeqCst);
        } else {
            capture.block.store(true, Ordering::SeqCst);
        }
        let gateway = gateway(capture.clone());
        let port = port(package, target, gateway.clone());
        let cancellation = running(&request);
        let task = tokio::spawn(async move {
            port.execute_runtime_host_request(request, cancellation)
                .await
        });
        tokio::time::timeout(Duration::from_secs(2), capture.entered.notified())
            .await
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(
            tokio::time::timeout(Duration::from_millis(30), gateway.is_ready())
                .await
                .is_err()
        );
        capture.release.notify_one();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), gateway.is_ready())
                .await
                .unwrap()
        );
        assert!(capture.completed.load(Ordering::SeqCst));
    }
}

fn audio_batch_request(
    request: RuntimeHostExecutionRequest,
) -> pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionRequest,
        RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationPolicy,
    };
    let members = [0, 1]
        .into_iter()
        .enumerate()
        .map(|(index, _)| {
            let mut request = request.clone();
            request.execution_request_id = format!("audio.execution.{index}");
            request.handoff.workflow_id = format!("workflow.audio.{index}").parse().unwrap();
            request.handoff.workflow_run_id = format!("run.audio.{index}").parse().unwrap();
            request.handoff.node_id = format!("node.audio.{index}").parse().unwrap();
            request.handoff.task_id = format!("task.audio.{index}").parse().unwrap();
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
            RuntimeHostBatchExecutionMemberRequest {
                execution_request_id: request.execution_request_id,
                assignment_id: format!("assignment.audio.{index}"),
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
        batch_execution_request_id: "audio.batch.001".into(), anchor_execution_request_id: "audio.execution.0".into(),
        cancellation_context: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationContext::workflow_service("audio.batch.001"),
        members,
    };
    batch.validate().unwrap();
    batch
}

#[tokio::test]
async fn selected_audio_sequential_envelope_preserves_member_identity_and_outputs() {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionPort, RuntimeHostBatchExecutionState,
        RuntimeHostExecutionCancellationHandle,
    };
    let (_directory, request, package, target) = fixture();
    let capture = Arc::new(Capture::default());
    let gw = gateway(capture.clone());
    let parent = parent_outputs(gw.clone(), &package.model_ref, input_values(&request)).await;
    let batch = audio_batch_request(request);
    let port = port(package, target, gw);
    let response = port
        .execute_runtime_host_batch_request(
            batch.clone(),
            RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        response.state,
        RuntimeHostBatchExecutionState::Completed,
        "{:?}",
        response.diagnostics
    );
    assert_eq!(response.members.len(), 2);
    for (actual, expected) in response.members.iter().zip(&batch.members) {
        assert_eq!(actual.execution_request_id, expected.execution_request_id);
        assert_eq!(actual.assignment_id, expected.assignment_id);
        let values:HashMap<_,_>=actual.outputs.iter().map(|o|(o.port_id.clone(),match &o.value {pantograph_runtime_host_contracts::RuntimeHostExecutionOutputValue::String(s)=>serde_json::json!(s),pantograph_runtime_host_contracts::RuntimeHostExecutionOutputValue::Json(v)=>v.clone(),other=>panic!("{other:?}")})).collect();
        assert_eq!(values, parent);
    }
    let ids = capture.ids.lock().unwrap();
    assert_eq!(*ids, vec!["audio.execution.0", "audio.execution.1"]);
}

use super::*;
use crate::runtime_host_embedding_execution::tests::{Package, Target, UnusedMediaSink};
use async_trait::async_trait;
use inference::{
    backend::BackendStartOutcome, BackendCapabilities, BackendConfig, BackendError, ChatChunk,
    EmbeddingResult, InferenceBackend, InferenceGateway, ProcessSpawner, RerankRequest,
    RerankResponse, RerankResult,
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
    let path = directory.path().join("synthetic-reranker.gguf");
    // No model payload: this file exercises target validation only.
    std::fs::write(&path, b"synthetic selected-owner target").unwrap();
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../../inference/tests/fixtures/inference_package_facts/rerank_package_facts.json"
    ))
    .unwrap();
    package.model_ref.revision = Some("synthetic-r1".into());
    package.model_ref.selected_artifact_path = None;
    let reference = serde_json::to_value(&package.model_ref).unwrap();
    let mut request: serde_json::Value = serde_json::from_str(include_str!("../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json")).unwrap();
    request["materialized_inputs"] = serde_json::json!([
        {"port_id":"query","value":{"value_type":"string","value":"search"}},
        {"port_id":"documents","value":{"value_type":"json","value":["a",{"text":"b"}," ",{"content":"c"},{"document":"d"}]}},
        {"port_id":"task_options","value":{"value_type":"json","value":{"top_k":2,"return_documents":true}}}
    ]);
    for pointer in [
        "/handoff/task_intent",
        "/handoff/dispatch_decision/task_intent",
    ] {
        let intent = request.pointer_mut(pointer).unwrap();
        intent["task_type"] = serde_json::json!("rerank");
        intent["model_ref"] = reference.clone();
        intent["constraints"] =
            serde_json::json!({"requested_runtime_id":"llamacpp","requested_device_id":"cpu"});
        intent["trait_settings"] = serde_json::json!([]);
    }
    for pointer in [
        "/handoff/readiness_proof/preflight_result/identity_key",
        "/handoff/dispatch_decision/readiness_proof/preflight_result/identity_key",
    ] {
        let key = request.pointer_mut(pointer).unwrap();
        key["task_id"] = serde_json::json!("rerank");
        key["model_ref"] = reference.clone();
        key["scheduler_intent"] =
            serde_json::json!({"requested_runtime_id":"llamacpp","requested_device_id":"cpu"});
    }
    let selected = &mut request["handoff"]["dispatch_decision"];
    selected["selected_model_ref"] = reference.clone();
    selected["runtime_trait_settings"] = serde_json::json!([]);
    selected["selected_runtime_id"] = serde_json::json!("llamacpp");
    selected["selected_runtime_variant_id"] = serde_json::json!("llama_cpp.cpu");
    selected["selected_device_ids"] = serde_json::json!(["cpu"]);
    for reservation in selected["reservations"].as_array_mut().unwrap() {
        reservation["device_id"] = serde_json::json!("cpu");
    }
    let target = PumasArtifactLoadTarget {
        model_ref: package.model_ref.clone(),
        artifact_kind: inference::ModelArtifactKind::Gguf,
        local_load_path: path.to_str().unwrap().into(),
        load_path_kind: inference::PumasArtifactLoadPathKind::File,
        library_root_id: Some("synthetic-root".into()),
        storage_kind: inference::ModelStorageKind::LibraryOwned,
        validation_state: inference::ModelValidationState::Valid,
        content_fingerprint: None,
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    let request: RuntimeHostExecutionRequest = serde_json::from_value(request).unwrap();
    request.validate().unwrap();
    (directory, request, package, target)
}

#[derive(Default)]
pub(crate) struct Capture {
    pub loads: Mutex<Vec<(String, PumasArtifactLoadTarget, BackendExecutionDecision)>>,
    pub requests: Mutex<Vec<RerankRequest>>,
    pub entered: tokio::sync::Notify,
    pub release: tokio::sync::Notify,
    pub block: AtomicBool,
    pub completed: AtomicBool,
    pub bad_result: AtomicBool,
    pub nonfinite_score: AtomicBool,
    pub duplicate_index: AtomicBool,
    pub unsupported_load: AtomicBool,
}
pub(crate) struct Synthetic(pub Arc<Capture>);
#[async_trait]
impl InferenceBackend for Synthetic {
    fn name(&self) -> &'static str {
        "llama.cpp"
    }
    fn description(&self) -> &'static str {
        "Synthetic parity owner, no runtime or model"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            reranking: true,
            ..Default::default()
        }
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
        panic!("generic startup forbidden")
    }
    async fn load_selected_rerank(
        &mut self,
        request: &InferenceExecutionRequest,
        target: &PumasArtifactLoadTarget,
        decision: &BackendExecutionDecision,
        _: Option<Arc<dyn ProcessSpawner>>,
        _: inference::InferenceExecutionCancellationHandle,
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
        if self.0.unsupported_load.load(Ordering::SeqCst) {
            return Err(BackendError::Config(
                "synthetic selected load unsupported".into(),
            ));
        }
        self.0.loads.lock().unwrap().push((
            request.request_id.clone().unwrap(),
            target.clone(),
            decision.clone(),
        ));
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            ..Default::default()
        })
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
        panic!("rerank must not use chat fallback")
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
        panic!("rerank must not use embeddings")
    }
    async fn rerank(
        &self,
        request: RerankRequest,
    ) -> std::result::Result<RerankResponse, BackendError> {
        self.0.requests.lock().unwrap().push(request.clone());
        if self.0.block.load(Ordering::SeqCst) {
            self.0.entered.notify_one();
            self.0.release.notified().await;
        }
        self.0.completed.store(true, Ordering::SeqCst);
        let mut results: Vec<_> = request
            .documents
            .iter()
            .enumerate()
            .rev()
            .map(|(index, document)| RerankResult {
                index,
                score: (index + 1) as f32 / 10.0,
                document: request.return_documents.then(|| document.clone()),
            })
            .take(request.top_n.unwrap_or(usize::MAX))
            .collect();
        if self.0.bad_result.load(Ordering::SeqCst) {
            results[0].index = request.documents.len();
        }
        if self.0.nonfinite_score.load(Ordering::SeqCst) {
            results[0].score = f32::NAN;
        }
        if self.0.duplicate_index.load(Ordering::SeqCst) {
            results[1].index = results[0].index;
        }
        Ok(RerankResponse {
            results,
            metadata: serde_json::Value::Null,
        })
    }
}
pub(crate) fn gateway(capture: Arc<Capture>) -> Arc<InferenceGateway> {
    Arc::new(InferenceGateway::with_backend(
        Box::new(Synthetic(capture)),
        "llama.cpp",
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
    values.insert("task_kind".into(), serde_json::json!("rerank"));
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
async fn selected_rerank_matches_retained_parent_route_and_loads_exact_identity() {
    for (alias, return_documents) in [(false, true), (true, false)] {
        let (_directory, mut request, package, target) = fixture();
        if alias {
            let documents = input_values(&request)["documents"].clone();
            request.materialized_inputs[1].port_id = "documents_json".into();
            request.materialized_inputs[1].value =
                RuntimeHostExecutionInputValue::String(serde_json::to_string(&documents).unwrap());
            request.materialized_inputs.push(
                pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                    port_id: "returnDocuments".into(),
                    value: RuntimeHostExecutionInputValue::Bool(return_documents),
                },
            );
            request.materialized_inputs.push(
                pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                    port_id: "topN".into(),
                    value: RuntimeHostExecutionInputValue::U64(1),
                },
            );
        }
        request.materialized_inputs.push(
            pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                port_id: "extra_options".into(),
                value: RuntimeHostExecutionInputValue::Json(
                    serde_json::json!({"synthetic_extension":7}),
                ),
            },
        );
        let capture = Arc::new(Capture::default());
        let gateway = gateway(capture.clone());
        let parent =
            parent_outputs(gateway.clone(), &package.model_ref, input_values(&request)).await;
        let response = port(package.clone(), target.clone(), gateway)
            .execute_runtime_host_request(request.clone(), running(&request))
            .await
            .unwrap();
        response.validate().unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{:?}",
            response.diagnostics
        );
        assert_eq!(output_values(&response), parent);
        let requests = capture.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            serde_json::to_value(&requests[0]).unwrap(),
            serde_json::to_value(&requests[1]).unwrap()
        );
        let loads = capture.loads.lock().unwrap();
        assert_eq!(loads.len(), 1);
        assert_eq!(loads[0].0, request.execution_request_id);
        assert_eq!(loads[0].1, target);
        assert_eq!(
            loads[0].2.selected_model_ref.as_ref(),
            Some(&package.model_ref)
        );
        assert_eq!(
            loads[0].2.selected_runtime_variant_id.as_str(),
            "llama_cpp.cpu"
        );
        assert_eq!(
            loads[0].2.selected_device_id.as_ref().unwrap().as_str(),
            "cpu"
        );
    }
}

#[tokio::test]
async fn selected_rerank_rejects_streaming_and_invalid_inputs_before_owner_effects() {
    for (port_id, value) in [
        ("stream", serde_json::json!(true)),
        ("query", serde_json::json!(" ")),
        ("documents", serde_json::json!([3])),
        ("top_n", serde_json::json!(0)),
        ("extra_options", serde_json::json!([])),
        ("prompt", serde_json::json!("wrong route")),
    ] {
        let (_directory, mut request, package, target) = fixture();
        request
            .materialized_inputs
            .retain(|input| input.port_id != port_id);
        request.materialized_inputs.push(
            pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                port_id: port_id.into(),
                value: match port_id {
                    "query" | "prompt" => {
                        RuntimeHostExecutionInputValue::String(value.as_str().unwrap().into())
                    }
                    "stream" => RuntimeHostExecutionInputValue::Bool(value.as_bool().unwrap()),
                    "top_n" => RuntimeHostExecutionInputValue::U64(value.as_u64().unwrap()),
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
async fn selected_rerank_checks_package_target_revision_and_owner_support() {
    for case in [
        "model",
        "revision",
        "requested_revision",
        "artifact",
        "missing_file",
        "unknown_storage",
        "reserved_options",
        "task",
        "custom_code",
        "unsupported_load",
        "bad_result",
        "nonfinite_score",
        "duplicate_index",
    ] {
        let (_directory, mut request, mut package, mut target) = fixture();
        let capture = Arc::new(Capture::default());
        match case {
            "model" => target.model_ref.model_id = "rerank/wrong-owner".into(),
            "revision" => target.model_ref.revision = Some("wrong".into()),
            "requested_revision" => {
                let mut value = serde_json::to_value(&request).unwrap();
                for pointer in ["/handoff/task_intent/model_ref", "/handoff/dispatch_decision/task_intent/model_ref", "/handoff/readiness_proof/preflight_result/identity_key/model_ref", "/handoff/dispatch_decision/readiness_proof/preflight_result/identity_key/model_ref"] {value.pointer_mut(pointer).unwrap()["revision"] = serde_json::json!("requested-other");}
                request = serde_json::from_value(value).unwrap();
            }
            "artifact" => target.model_ref.selected_artifact_id = Some("wrong".into()),
            "missing_file" => target.local_load_path.push_str("-missing"),
            "unknown_storage" => {
                package.artifact.storage_kind = inference::ModelStorageKind::Unknown
            }
            "reserved_options" => request.materialized_inputs.push(
                pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                    port_id: "extra_options".into(),
                    value: RuntimeHostExecutionInputValue::Json(
                        serde_json::json!({"model":"other-owner","documents":["foreign"]}),
                    ),
                },
            ),
            "task" => package.task.task_type_primary = Some("embedding".into()),
            "custom_code" => package.custom_code.requires_custom_code = true,
            "unsupported_load" => capture.unsupported_load.store(true, Ordering::SeqCst),
            "bad_result" => capture.bad_result.store(true, Ordering::SeqCst),
            "nonfinite_score" => capture.nonfinite_score.store(true, Ordering::SeqCst),
            "duplicate_index" => capture.duplicate_index.store(true, Ordering::SeqCst),
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
            usize::from(matches!(
                case,
                "bad_result" | "nonfinite_score" | "duplicate_index"
            ))
        );
        assert_eq!(
            capture.loads.lock().unwrap().len(),
            usize::from(matches!(
                case,
                "bad_result" | "nonfinite_score" | "duplicate_index"
            ))
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
async fn selected_rerank_cancellation_waits_for_owner_and_suppresses_outputs() {
    let (_directory, request, package, target) = fixture();
    let capture = Arc::new(Capture::default());
    capture.block.store(true, Ordering::SeqCst);
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
#[tokio::test]
async fn selected_rerank_caller_abort_keeps_owner_custody_until_completion() {
    let (_directory, request, package, target) = fixture();
    let capture = Arc::new(Capture::default());
    capture.block.store(true, Ordering::SeqCst);
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

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn production_rerank_loader_launches_exact_file_in_cpu_rerank_mode_without_fallback() {
    struct Spawner {
        root: std::path::PathBuf,
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }
    #[async_trait]
    impl ProcessSpawner for Spawner {
        async fn spawn_sidecar(
            &self,
            name: &str,
            args: &[&str],
        ) -> std::result::Result<
            (
                tokio::sync::mpsc::Receiver<inference::process::ProcessEvent>,
                Box<dyn inference::process::ProcessHandle>,
            ),
            String,
        > {
            self.calls.lock().unwrap().push((
                name.into(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            Err("synthetic launch refusal; no executable or runtime fetched".into())
        }
        fn app_data_dir(&self) -> std::result::Result<std::path::PathBuf, String> {
            Ok(self.root.clone())
        }
        fn binaries_dir(&self) -> std::result::Result<std::path::PathBuf, String> {
            Ok(self.root.clone())
        }
    }
    let (directory, request, package, target) = fixture();
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(inference::backend::llamacpp::LlamaCppBackend::new()),
        "llama.cpp",
    ));
    let spawner = Arc::new(Spawner {
        root: directory.path().to_owned(),
        calls: Mutex::new(vec![]),
    });
    gateway.set_spawner(spawner.clone()).await;
    let response = port(package, target.clone(), gateway.clone())
        .execute_runtime_host_request(request.clone(), running(&request))
        .await
        .unwrap();
    assert_eq!(response.state, RuntimeHostExecutionState::Failed);
    assert!(response.outputs.is_empty());
    {
        let calls = spawner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "{:?}", response.diagnostics);
        assert_eq!(calls[0].0, "llama-server-wrapper");
        let args = &calls[0].1;
        let argument =
            |flag: &str| args[args.iter().position(|arg| arg == flag).unwrap() + 1].as_str();
        assert_eq!(argument("-m"), target.local_load_path);
        assert_eq!(argument("-ngl"), "0");
        assert_eq!(argument("--device"), "none");
        assert!(args.iter().any(|arg| arg == "--reranking"));
        assert!(!args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--embedding" | "--hf-repo" | "--mmproj")));
    }
    assert!(!gateway.is_ready().await);
}

fn rerank_batch_request(
    request: RuntimeHostExecutionRequest,
) -> pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionRequest,
        RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationPolicy,
    };
    let members = ["first query", "second query"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let mut request = request.clone();
            request.execution_request_id = format!("rerank.execution.{index}");
            request.handoff.workflow_id = format!("workflow.rerank.{index}").parse().unwrap();
            request.handoff.workflow_run_id = format!("run.rerank.{index}").parse().unwrap();
            request.handoff.node_id = format!("node.rerank.{index}").parse().unwrap();
            request.handoff.task_id = format!("task.rerank.{index}").parse().unwrap();
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
                assignment_id: format!("assignment.rerank.{index}"),
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
        batch_execution_request_id: "rerank.batch.001".into(), anchor_execution_request_id: "rerank.execution.0".into(),
        cancellation_context: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationContext::workflow_service("rerank.batch.001"),
        members,
    };
    batch.validate().unwrap();
    batch
}

#[tokio::test]
async fn selected_rerank_envelope_preserves_each_member_identity_and_parent_outputs() {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
        RuntimeHostBatchExecutionState,
    };
    let (_directory, request, package, target) = fixture();
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    let port = port(package.clone(), target.clone(), gateway.clone());
    let batch = rerank_batch_request(request);
    let mut parents = Vec::new();
    for member in &batch.members {
        let request = RuntimeHostExecutionRequest {
            contract_version: batch.contract_version,
            execution_request_id: member.execution_request_id.clone(),
            cancellation_context: batch.cancellation_context.clone(),
            handoff: member.handoff.clone(),
            materialized_inputs: member.materialized_inputs.clone(),
        };
        parents.push(
            parent_outputs(gateway.clone(), &package.model_ref, input_values(&request)).await,
        );
    }
    let response = port
        .execute_runtime_host_batch_request(
            batch.clone(),
            RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone()),
        )
        .await
        .unwrap();
    response.validate().unwrap();
    assert_eq!(
        response.state,
        RuntimeHostBatchExecutionState::Completed,
        "{:?}",
        response.diagnostics
    );
    for ((actual, expected), parent) in response.members.iter().zip(&batch.members).zip(parents) {
        assert_eq!(
            actual.state,
            RuntimeHostBatchExecutionMemberState::Completed
        );
        assert_eq!(actual.execution_request_id, expected.execution_request_id);
        assert_eq!(actual.assignment_id, expected.assignment_id);
        assert_eq!(actual.workflow_id, expected.handoff.workflow_id);
        assert_eq!(actual.workflow_run_id, expected.handoff.workflow_run_id);
        assert_eq!(actual.node_id, expected.handoff.node_id);
        assert_eq!(actual.task_id, expected.handoff.task_id);
        let outputs: HashMap<_, _> = actual
            .outputs
            .iter()
            .map(|output| {
                (
                    output.port_id.clone(),
                    match &output.value {
                        RuntimeHostExecutionOutputValue::Json(value) => value.clone(),
                        RuntimeHostExecutionOutputValue::String(value) => serde_json::json!(value),
                        _ => panic!("rerank output"),
                    },
                )
            })
            .collect();
        assert_eq!(outputs, parent);
    }
    let loads = capture.loads.lock().unwrap();
    assert_eq!(loads.len(), 2);
    for (load, member) in loads.iter().zip(batch.members) {
        assert_eq!(load.0, member.execution_request_id);
        assert_eq!(load.1, target);
    }
}

#[tokio::test]
async fn selected_rerank_rejects_runtime_traits_gpu_variant_and_generation_controls() {
    for case in ["runtime_traits", "gpu_variant", "generation_controls"] {
        let (_directory, mut request, package, target) = fixture();
        let capture = Arc::new(Capture::default());
        let gateway = gateway(capture.clone());
        if case == "generation_controls" {
            let mut projection = project_runtime_host_rerank(
                &ValidatedRuntimeHostExecutionRequest::try_from(request).unwrap(),
                package,
                target,
            )
            .unwrap();
            projection.request.generation_options = Some(inference::GenerationOptions::default());
            assert!(gateway
                .execute_selected_rerank_with_cancellation(
                    projection.request,
                    projection.target,
                    projection.decision,
                    inference::InferenceExecutionCancellationHandle::running()
                )
                .await
                .is_err());
        } else {
            let selected = request.handoff.dispatch_decision.as_mut().unwrap();
            if case == "gpu_variant" {
                selected.selected_runtime_variant_id = Some("llama_cpp.cuda".parse().unwrap());
            } else {
                selected.runtime_trait_settings = serde_json::from_value(serde_json::json!([{"trait_id":"unsupported","value":{"kind":"string","value":"trait"}}])).unwrap();
            }
            request.validate().unwrap();
            let response = port(package, target, gateway)
                .execute_runtime_host_request(request.clone(), running(&request))
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
        }
        assert!(capture.loads.lock().unwrap().is_empty());
        assert!(capture.requests.lock().unwrap().is_empty());
    }
}

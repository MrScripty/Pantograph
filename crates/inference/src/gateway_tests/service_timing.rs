use super::*;
use crate::service_timing::{ServiceTimingClock, ServiceTimingInstrumentation};
use pantograph_timing_contracts::{
    RuntimeServiceTimingAttempt, RuntimeServiceTimingIdentity,
    RuntimeServiceTimingOutcome as Outcome, RuntimeServiceTimingPhase as Phase,
    RuntimeServiceTimingProfile, RuntimeServiceTimingUnavailableReason as Unknown,
    RuntimeServiceTimingValue as Value,
};
use std::sync::atomic::AtomicU64;

#[derive(Default)]
struct Clock {
    value: AtomicU64,
    reads: AtomicU64,
}
impl Clock {
    fn advance(&self, ns: u64) {
        self.value.fetch_add(ns, Ordering::SeqCst);
    }
}
impl ServiceTimingClock for Clock {
    fn now_ns(&self) -> u64 {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.value.load(Ordering::SeqCst)
    }
}
struct Recorder {
    rows: Mutex<Vec<RuntimeServiceTimingAttempt>>,
    accept: bool,
}
impl Default for Recorder {
    fn default() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            accept: true,
        }
    }
}
impl crate::RuntimeServiceTimingRecorder for Recorder {
    fn try_record(&self, attempt: RuntimeServiceTimingAttempt) -> bool {
        if !self.accept {
            return false;
        }
        let Ok(mut rows) = self.rows.try_lock() else {
            return false;
        };
        if rows.len() == 8 {
            return false;
        }
        rows.push(attempt);
        true
    }
}
fn owner_facts() -> crate::RuntimeServiceTimingOwnerFacts {
    crate::RuntimeServiceTimingOwnerFacts {
        implementation_fingerprint: "controlled-implementation-v1".into(),
        effective_configuration_fingerprint: "controlled-effective-config-v1".into(),
        physical_device_fingerprint: "controlled-device-v1".into(),
        device_id: "cpu".parse().unwrap(),
    }
}
struct Backend {
    inner: SelectedTextBackend,
    clock: Arc<Clock>,
    owner_reads: Arc<AtomicU64>,
    facts: Option<crate::RuntimeServiceTimingOwnerFacts>,
    shared_facts: Option<Arc<Mutex<Option<crate::RuntimeServiceTimingOwnerFacts>>>>,
    fail_load: bool,
    rewind_load_clock: bool,
    fail_cleanup: bool,
    load_started: Option<Arc<tokio::sync::Notify>>,
}
impl Backend {
    fn new(clock: Arc<Clock>) -> Self {
        Self {
            inner: SelectedTextBackend::default(),
            clock,
            owner_reads: Arc::new(AtomicU64::new(0)),
            facts: Some(owner_facts()),
            shared_facts: None,
            fail_load: false,
            rewind_load_clock: false,
            fail_cleanup: false,
            load_started: None,
        }
    }
}
#[async_trait]
impl InferenceBackend for Backend {
    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        self.inner.embeddings(texts, model).await
    }
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, BackendError> {
        self.inner.rerank(request).await
    }
    fn name(&self) -> &'static str {
        "PyTorch"
    }
    fn description(&self) -> &'static str {
        "controlled timing owner"
    }
    fn capabilities(&self) -> BackendCapabilities {
        self.inner.capabilities()
    }
    fn is_ready(&self) -> bool {
        true
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn health_check(&self) -> bool {
        true
    }
    async fn start(
        &mut self,
        config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.inner.start(config, spawner).await
    }
    async fn stop(&mut self) -> Result<(), BackendError> {
        self.inner.stop().await
    }
    fn runtime_service_timing_owner_facts(&self) -> Option<crate::RuntimeServiceTimingOwnerFacts> {
        self.owner_reads.fetch_add(1, Ordering::SeqCst);
        match &self.shared_facts {
            Some(facts) => facts.lock().unwrap().clone(),
            None => self.facts.clone(),
        }
    }
    async fn load_selected_text(
        &mut self,
        request: &InferenceExecutionRequest,
        target: &PumasArtifactLoadTarget,
        decision: &BackendExecutionDecision,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.clock.advance(11);
        if self.rewind_load_clock {
            self.clock.value.store(0, Ordering::SeqCst);
        }
        if let Some(started) = &self.load_started {
            started.notify_one();
            std::future::pending::<()>().await;
        }
        if self.fail_load {
            return Err(BackendError::Inference("controlled load failure".into()));
        }
        self.inner
            .load_selected_text(request, target, decision)
            .await
    }
    async fn finish_selected_text(&self, cancel: bool) -> Result<(), BackendError> {
        self.clock.advance(7);
        if self.fail_cleanup {
            return Err(BackendError::Inference("controlled cleanup failure".into()));
        }
        self.inner.finish_selected_text(cancel).await
    }
    async fn chat_completion_stream(
        &self,
        request: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        self.clock.advance(23);
        self.inner.chat_completion_stream(request).await
    }
}
fn instrument(backend: Backend, recorder: Arc<Recorder>, clock: Arc<Clock>) -> InferenceGateway {
    let mut gateway = InferenceGateway::with_backend(Box::new(backend), "PyTorch");
    gateway.service_timing = Some(ServiceTimingInstrumentation::with_clock(recorder, clock));
    gateway
}
fn exact_profile(attempt: &RuntimeServiceTimingAttempt) -> RuntimeServiceTimingProfile {
    let RuntimeServiceTimingIdentity::Exact { profile } = &attempt.identity else {
        panic!("{:?}", attempt.identity);
    };
    profile.clone()
}
fn value(attempt: &RuntimeServiceTimingAttempt, phase: Phase) -> &Value {
    &attempt
        .phases
        .iter()
        .find(|item| item.phase == phase)
        .unwrap()
        .value
}
async fn execute(
    gateway: &InferenceGateway,
    request: InferenceExecutionRequest,
    target: PumasArtifactLoadTarget,
    decision: BackendExecutionDecision,
) -> Result<InferenceExecutionResult, GatewayError> {
    gateway
        .execute_selected_text_with_cancellation(
            request,
            target,
            decision,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
}

#[tokio::test]
async fn service_timing_measures_distinct_owner_bound_phases_and_runtime_generation() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let gateway = instrument(Backend::new(clock.clone()), recorder.clone(), clock.clone());
    execute(&gateway, request.clone(), target.clone(), decision.clone())
        .await
        .unwrap();
    execute(&gateway, request, target.clone(), decision)
        .await
        .unwrap();
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(rows.len(), 2);
    let first = exact_profile(&rows[0]);
    let second = exact_profile(&rows[1]);
    assert_eq!(first.identity_fingerprint(), second.identity_fingerprint());
    assert_ne!(first.runtime_instance_id(), second.runtime_instance_id());
    for (phase, elapsed) in [
        (Phase::GatewayCustodyWait, 0),
        (Phase::SelectedModelLoad, 11),
        (Phase::TextExecution, 23),
        (Phase::WorkerCleanup, 7),
    ] {
        assert_eq!(rows[0].observed_completed_ns(&first, phase), Some(elapsed));
        assert_eq!(rows[1].observed_completed_ns(&first, phase), None);
    }
    let wire = serde_json::to_string(&rows[0]).unwrap();
    assert!(!wire.contains("exact prompt"));
    assert!(!wire.contains(&target.local_load_path));
    assert!(!wire.contains("controlled-effective-config"));
    assert_eq!(clock.reads.load(Ordering::SeqCst), 16);
}

#[tokio::test]
async fn service_timing_disabled_defaults_do_not_read_phase_clock_or_owner_facts() {
    let (_directory, request, target, decision) = crate::selected_text_execution::fixture();
    let clock = Arc::new(Clock::default());
    let backend = Backend::new(clock.clone());
    let reads = backend.owner_reads.clone();
    let gateway = InferenceGateway::with_backend(Box::new(backend), "PyTorch");
    execute(&gateway, request, target, decision).await.unwrap();
    assert_eq!(clock.reads.load(Ordering::SeqCst), 0);
    assert_eq!(reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn service_timing_missing_content_or_owner_facts_never_becomes_exact() {
    for missing in ["content", "owner", "device", "config"] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        if missing != "content" {
            target.content_fingerprint = Some("controlled-content-v1".into());
        }
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        match missing {
            "owner" => backend.facts = None,
            "device" => backend.facts.as_mut().unwrap().device_id = "cuda:0".parse().unwrap(),
            "config" => backend
                .facts
                .as_mut()
                .unwrap()
                .effective_configuration_fingerprint
                .clear(),
            _ => {}
        }
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        let rows = recorder.rows.lock().unwrap();
        assert_eq!(
            rows[0].identity,
            RuntimeServiceTimingIdentity::Unknown {
                reason: if missing == "content" {
                    Unknown::ModelContentIdentityUnavailable
                } else {
                    Unknown::RuntimeOwnerFactsUnavailable
                }
            }
        );
        assert_eq!(
            value(&rows[0], Phase::SelectedModelLoad),
            &Value::Observed {
                elapsed_ns: 11,
                outcome: Outcome::Completed
            }
        );
    }
}

#[tokio::test]
async fn service_timing_load_and_cleanup_failure_keep_partial_evidence_unusable() {
    for failure in ["load", "cleanup"] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("controlled-content-v1".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.fail_load = failure == "load";
        backend.fail_cleanup = failure == "cleanup";
        let gateway = instrument(backend, recorder.clone(), clock);
        assert!(execute(&gateway, request, target, decision).await.is_err());
        let rows = recorder.rows.lock().unwrap();
        assert_eq!(rows[0].outcome, Outcome::Failed);
        if failure == "load" {
            assert_eq!(
                value(&rows[0], Phase::SelectedModelLoad),
                &Value::Observed {
                    elapsed_ns: 11,
                    outcome: Outcome::Failed
                }
            );
            assert_eq!(
                value(&rows[0], Phase::TextExecution),
                &Value::Unknown {
                    reason: Unknown::PhaseNotReached
                }
            );
        } else {
            assert_eq!(
                value(&rows[0], Phase::WorkerCleanup),
                &Value::Observed {
                    elapsed_ns: 7,
                    outcome: Outcome::Failed
                }
            );
            assert_eq!(
                rows[0].observed_completed_ns(&exact_profile(&rows[0]), Phase::SelectedModelLoad),
                None
            );
        }
    }
}

#[tokio::test]
async fn service_timing_dropped_call_records_abandoned_without_inventing_cleanup() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let started = Arc::new(tokio::sync::Notify::new());
    let mut backend = Backend::new(clock.clone());
    backend.load_started = Some(started.clone());
    let gateway = instrument(backend, recorder.clone(), clock.clone());
    let mut future = Box::pin(execute(&gateway, request, target, decision));
    tokio::select! { _ = started.notified() => {}, _ = &mut future => panic!("load should remain pending") }
    clock.advance(19);
    drop(future);
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, Outcome::Abandoned);
    assert_eq!(
        value(&rows[0], Phase::SelectedModelLoad),
        &Value::Observed {
            elapsed_ns: 30,
            outcome: Outcome::Abandoned
        }
    );
    assert_eq!(
        value(&rows[0], Phase::WorkerCleanup),
        &Value::Unknown {
            reason: Unknown::PhaseNotReached
        }
    );
}

#[tokio::test]
async fn service_timing_recorder_saturation_does_not_change_execution() {
    let (_directory, request, target, decision) = crate::selected_text_execution::fixture();
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder {
        rows: Mutex::new(Vec::new()),
        accept: false,
    });
    let gateway = instrument(Backend::new(clock.clone()), recorder.clone(), clock);
    execute(&gateway, request, target, decision).await.unwrap();
    assert!(recorder.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn service_timing_observed_keys_invalidate_changed_content_config_and_owner_facts() {
    let (_directory, base_request, mut base_target, base_decision) =
        crate::selected_text_execution::fixture();
    base_target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let facts = Arc::new(Mutex::new(Some(owner_facts())));
    let mut backend = Backend::new(clock.clone());
    backend.shared_facts = Some(facts.clone());
    let gateway = instrument(backend, recorder.clone(), clock);
    execute(
        &gateway,
        base_request.clone(),
        base_target.clone(),
        base_decision.clone(),
    )
    .await
    .unwrap();
    let baseline = exact_profile(&recorder.rows.lock().unwrap()[0]);
    for changed in [
        "content",
        "revision",
        "artifact",
        "generation",
        "implementation",
        "configuration",
        "physical-device",
    ] {
        let mut request = base_request.clone();
        let mut target = base_target.clone();
        let mut decision = base_decision.clone();
        let mut owner = owner_facts();
        match changed {
            "content" => target.content_fingerprint = Some("controlled-content-v2".into()),
            "revision" | "artifact" => {
                let mut model = target.model_ref.clone();
                if changed == "revision" {
                    model.revision = Some("controlled-revision-v2".into());
                } else {
                    model.selected_artifact_id = Some("controlled-artifact-v2".into());
                }
                target.model_ref = model.clone();
                request.model_ref = Some(model.clone());
                request
                    .resolved_model_package_facts
                    .as_mut()
                    .unwrap()
                    .model_ref = model.clone();
                decision.selected_model_ref = Some(model);
            }
            "generation" => {
                request.generation_options = Some(crate::GenerationOptions {
                    length: crate::LengthGenerationOptions {
                        max_new_tokens: Some(128),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            }
            "implementation" => {
                owner.implementation_fingerprint = "controlled-implementation-v2".into()
            }
            "configuration" => {
                owner.effective_configuration_fingerprint = "controlled-config-v2".into()
            }
            "physical-device" => owner.physical_device_fingerprint = "controlled-device-v2".into(),
            _ => unreachable!(),
        }
        *facts.lock().unwrap() = Some(owner);
        execute(&gateway, request, target, decision).await.unwrap();
        let rows = recorder.rows.lock().unwrap();
        let current = exact_profile(rows.last().unwrap());
        assert_ne!(
            current.identity_fingerprint(),
            baseline.identity_fingerprint(),
            "{changed}"
        );
        assert_eq!(
            rows[0].observed_completed_ns(&current, Phase::SelectedModelLoad),
            None,
            "{changed}"
        );
    }
    assert_eq!(recorder.rows.lock().unwrap().len(), 8);
}

#[tokio::test]
async fn service_timing_clock_discontinuity_is_unknown_instead_of_observed_zero() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    clock.value.store(100, Ordering::SeqCst);
    let recorder = Arc::new(Recorder::default());
    let mut backend = Backend::new(clock.clone());
    backend.rewind_load_clock = true;
    let gateway = instrument(backend, recorder.clone(), clock);
    execute(&gateway, request, target, decision).await.unwrap();
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(
        value(&rows[0], Phase::SelectedModelLoad),
        &Value::Unknown {
            reason: Unknown::ClockDiscontinuity
        }
    );
    assert_eq!(
        rows[0].observed_completed_ns(&exact_profile(&rows[0]), Phase::SelectedModelLoad),
        None
    );
}

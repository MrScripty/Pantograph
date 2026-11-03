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
    rewind_owner_clock: bool,
    load_reuse: Option<bool>,
    fail_cleanup: bool,
    cancel_at_cleanup: Option<Arc<AtomicBool>>,
    load_started: Option<Arc<tokio::sync::Notify>>,
    load_request_ids: Arc<Mutex<Vec<Option<String>>>>,
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
            rewind_owner_clock: false,
            load_reuse: Some(false),
            fail_cleanup: false,
            cancel_at_cleanup: None,
            load_started: None,
            load_request_ids: Arc::new(Mutex::new(Vec::new())),
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
        if self.rewind_owner_clock {
            self.clock.value.store(0, Ordering::SeqCst);
        }
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
        self.load_request_ids
            .lock()
            .unwrap()
            .push(request.request_id.clone());
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
        let mut outcome = self
            .inner
            .load_selected_text(request, target, decision)
            .await?;
        outcome.runtime_reused = self.load_reuse;
        Ok(outcome)
    }
    async fn finish_selected_text(&self, cancel: bool) -> Result<(), BackendError> {
        self.clock.advance(7);
        if let Some(cancelled) = &self.cancel_at_cleanup {
            cancelled.store(true, Ordering::SeqCst);
        }
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
    assert_eq!(clock.reads.load(Ordering::SeqCst), 18);
}

#[tokio::test]
async fn service_timing_disabled_defaults_do_not_read_phase_clock_or_owner_facts() {
    let (_directory, mut request, target, decision) = crate::selected_text_execution::fixture();
    request.request_id = Some(format!(
        "{}disabled-id{}",
        " ".repeat(512 * 1024),
        " ".repeat(512 * 1024)
    ));
    let clock = Arc::new(Clock::default());
    let backend = Backend::new(clock.clone());
    let reads = backend.owner_reads.clone();
    let gateway = InferenceGateway::with_backend(Box::new(backend), "PyTorch");
    assert!(gateway.runtime_service_timing_clock_snapshot().is_none());
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

#[tokio::test]
async fn service_timing_bounds_retained_request_correlation_without_changing_execution_identity() {
    for original_id in [
        "small-id".to_string(),
        format!(
            "{}padded-id{}",
            " ".repeat(512 * 1024),
            " ".repeat(512 * 1024)
        ),
        "oversized-id-payload".repeat(64 * 1024),
    ] {
        let (_directory, mut request, target, decision) = crate::selected_text_execution::fixture();
        request.request_id = Some(original_id.clone());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let backend = Backend::new(clock.clone());
        let observed_ids = backend.load_request_ids.clone();
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        assert_eq!(
            observed_ids.lock().unwrap()[0].as_deref(),
            Some(original_id.as_str())
        );
        let rows = recorder.rows.lock().unwrap();
        let serialized = serde_json::to_string(&rows[0]).unwrap();
        assert!(
            serialized.len() < 4096,
            "retained timing row was {} bytes",
            serialized.len()
        );
        assert_eq!(
            rows[0].execution_request_id_digest.as_deref(),
            (original_id.len() <= 64 * 1024)
                .then(|| blake3::hash(original_id.as_bytes()).to_hex().to_string())
                .as_deref()
        );
        assert!(!serialized.contains(original_id.trim()));
    }
}

#[tokio::test]
async fn service_timing_rejected_calls_bound_correlation_before_validation() {
    for original_id in [
        None,
        Some(" ".repeat(1024 * 1024)),
        Some("rejected-caller-payload".repeat(64 * 1024)),
        Some(format!(
            "{}rejected-padded-id{}",
            " ".repeat(512 * 1024),
            " ".repeat(512 * 1024)
        )),
    ] {
        let (_directory, mut request, target, decision) = crate::selected_text_execution::fixture();
        request.request_id = original_id.clone();
        request.resolved_model_package_facts = None;
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let backend = Backend::new(clock.clone());
        let observed_ids = backend.load_request_ids.clone();
        let gateway = instrument(backend, recorder.clone(), clock.clone());
        assert!(execute(&gateway, request, target, decision).await.is_err());
        assert!(observed_ids.lock().unwrap().is_empty());
        assert_eq!(clock.reads.load(Ordering::SeqCst), 0);
        let rows = recorder.rows.lock().unwrap();
        assert_eq!(rows[0].outcome, Outcome::Failed);
        assert!(rows[0].phases.iter().all(|phase| matches!(
            phase.value,
            Value::Unknown {
                reason: Unknown::PhaseNotReached
            }
        )));
        assert_eq!(
            rows[0].execution_request_id_digest,
            original_id
                .as_ref()
                .filter(|id| id.len() <= 64 * 1024)
                .map(|id| blake3::hash(id.as_bytes()).to_hex().to_string())
        );
        let serialized = serde_json::to_string(&rows[0]).unwrap();
        assert!(
            serialized.len() < 4096,
            "retained rejected timing row was {} bytes",
            serialized.len()
        );
        assert!(!serialized.contains("rejected-caller-payload"));
        assert!(!serialized.contains("rejected-padded-id"));
    }
}

#[tokio::test]
async fn service_timing_capture_uses_owner_clock_and_actual_load_disposition() {
    use crate::{
        RuntimeServiceTimingLoadDisposition as Load, RuntimeServiceTimingOwnerProvenance as Source,
    };
    for (reuse, expected) in [
        (Some(false), Load::Reloaded),
        (Some(true), Load::Reused),
        (None, Load::Unknown),
    ] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("controlled-content-v1".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.load_reuse = reuse;
        let mut gateway = instrument(backend, recorder.clone(), clock.clone());
        // Even an internal built-in marker cannot promote a controlled clock.
        gateway.service_timing_owner_provenance = Source::BuiltIn;
        execute(&gateway, request, target, decision).await.unwrap();
        clock.advance(9);
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let rows = recorder.rows.lock().unwrap();
        let capture = rows[0].capture.as_ref().unwrap();
        assert_eq!(capture.clock_epoch, snapshot.clock_epoch);
        assert_eq!(capture.observed_at_ns, 41);
        assert_eq!(snapshot.now_ns, 50);
        assert_eq!(capture.owner_provenance, Source::Injected);
        assert_eq!(capture.load_disposition, expected);
        assert!(rows[0]
            .fresh_production_observation(&exact_profile(&rows[0]), &snapshot, 10)
            .is_none());
    }
}

#[tokio::test]
async fn service_timing_clock_regression_between_phases_invalidates_capture() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let mut backend = Backend::new(clock.clone());
    backend.rewind_owner_clock = true;
    let gateway = instrument(backend, recorder.clone(), clock);
    execute(&gateway, request, target, decision).await.unwrap();
    let rows = recorder.rows.lock().unwrap();
    assert!(rows[0].capture.is_none());
    assert!(matches!(
        value(&rows[0], Phase::TextExecution),
        Value::Unknown {
            reason: Unknown::ClockDiscontinuity
        }
    ));
}

#[tokio::test]
async fn service_timing_cancelled_before_load_never_qualifies_observation() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let gateway = instrument(Backend::new(clock.clone()), recorder.clone(), clock.clone());
    assert!(gateway
        .execute_selected_text_with_cancellation(
            request,
            target,
            decision,
            InferenceExecutionCancellationHandle::cancellation_requested("controlled cancellation")
        )
        .await
        .is_err());
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(rows[0].outcome, Outcome::Failed);
    assert!(rows[0].capture.is_none());
    assert_eq!(clock.reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn service_timing_oversized_workload_omits_identity_without_changing_execution() {
    let (_directory, mut request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let crate::InferenceExecutionInput::TextGeneration { system_prompt, .. } = &mut request.input
    else {
        unreachable!()
    };
    let oversized = "x".repeat(64 * 1024 + 1);
    *system_prompt = Some(oversized.clone());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let mut backend = Backend::new(clock.clone());
    backend.inner.expected_system_prompt = Some(oversized);
    let gateway = instrument(backend, recorder.clone(), clock);
    execute(&gateway, request, target, decision).await.unwrap();
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(rows[0].outcome, Outcome::Completed);
    assert!(matches!(
        rows[0].identity,
        RuntimeServiceTimingIdentity::Unknown {
            reason: Unknown::IdentityBudgetExceeded
        }
    ));
}

#[test]
fn service_timing_injected_owner_never_promotes_by_backend_label() {
    use crate::RuntimeServiceTimingOwnerProvenance as Source;
    let gateway =
        InferenceGateway::with_backend(Box::new(SelectedTextBackend::default()), "PyTorch");
    assert_eq!(gateway.service_timing_owner_provenance, Source::Injected);
    #[cfg(feature = "backend-llamacpp")]
    assert_eq!(
        InferenceGateway::new().service_timing_owner_provenance,
        Source::BuiltIn
    );
}

#[tokio::test]
async fn service_timing_cancellation_during_successful_cleanup_excludes_sample() {
    struct Signal(Arc<AtomicBool>);
    impl crate::InferenceExecutionCancellationSignal for Signal {
        fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
            if self.0.load(Ordering::SeqCst) {
                crate::InferenceExecutionCancellationSnapshot::cancellation_requested(None)
            } else {
                crate::InferenceExecutionCancellationSnapshot::running()
            }
        }
    }
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("controlled-content-v1".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut backend = Backend::new(clock.clone());
    backend.cancel_at_cleanup = Some(cancelled.clone());
    let gateway = instrument(backend, recorder.clone(), clock);
    let result = gateway
        .execute_selected_text_with_cancellation(
            request,
            target,
            decision,
            InferenceExecutionCancellationHandle::with_signal(Arc::new(Signal(cancelled))),
        )
        .await;
    assert!(matches!(
        result,
        Err(GatewayError::Backend(BackendError::Cancelled(_)))
    ));
    let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
    let rows = recorder.rows.lock().unwrap();
    assert_eq!(rows[0].outcome, Outcome::Failed);
    assert!(rows[0]
        .fresh_production_observation(&exact_profile(&rows[0]), &snapshot, 10)
        .is_none());
    // Distinct completed drain evidence remains available for diagnostics.
    assert!(matches!(
        value(&rows[0], Phase::WorkerCleanup),
        Value::Observed {
            outcome: Outcome::Completed,
            ..
        }
    ));
}

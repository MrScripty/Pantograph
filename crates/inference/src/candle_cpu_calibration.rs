//! Opt-in, process-local observations of the actual resident CPU BERT service.
//! This does not supply missing cold-load, transfer or release costs.
use crate::{BackendError, InferenceExecutionCancellationHandle, PumasModelRef};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX_KEYS: usize = 64;
const MAX_SAMPLES: usize = 8;
const MAX_TEXT_BYTES: usize = 64 * 1024;

/// Immutable configuration for a private, bounded CPU worker pool and local ring.
#[derive(Debug, Clone)]
pub struct CandleCpuCalibrationConfig {
    threads: usize,
    minimum_samples: usize,
    maximum_age: Duration,
}
impl CandleCpuCalibrationConfig {
    pub fn new(
        threads: usize,
        minimum_samples: usize,
        maximum_age: Duration,
    ) -> Result<Self, BackendError> {
        if !(1..=4).contains(&threads)
            || !(3..=MAX_SAMPLES).contains(&minimum_samples)
            || maximum_age.is_zero()
            || maximum_age > Duration::from_secs(3600)
        {
            return Err(BackendError::Config(
                "CPU calibration requires 1..4 workers, 3..8 samples and an age in (0, 1 hour]"
                    .into(),
            ));
        }
        Ok(Self {
            threads,
            minimum_samples,
            maximum_age,
        })
    }
}

/// Both estimates come from the same held CPU model and exact unmodified texts.
/// They are maxima of fresh observations, not worst-case execution guarantees.
#[derive(Clone)]
pub struct CandleCpuWarmComparison {
    pub execution_and_drain_ns: [u64; 2],
    pub sample_counts: [usize; 2],
    pub selected_position: usize,
    generation: u64,
    owner: Arc<CalibrationOwner>,
}

/// Opaque actual loaded-owner stamp. Only a held native owner may issue it;
/// it carries no reservation authority and cannot enable serial scheduling.
#[derive(Clone)]
pub struct CandleCpuCleanupOwner {
    owner: Arc<CalibrationOwner>,
    profile: LoadedCpuProfile,
    generation: u64,
}
impl CandleCpuCleanupOwner {
    #[cfg(test)]
    pub(crate) fn epoch_for_test(&self) -> u64 {
        self.generation
    }
    pub(crate) fn model_ref(&self) -> &PumasModelRef {
        &self.profile.model_ref
    }
}

/// Private-contents linear receipt minted only by successful actual warm-load
/// publication. It is not a drain, reservation or scheduler capability by itself.
pub struct CandleCpuVerifiedWarmLoad {
    previous: CandleCpuCleanupOwner,
    current: CandleCpuCleanupOwner,
}
impl CandleCpuVerifiedWarmLoad {
    pub(crate) fn adopt(self, expected: &CandleCpuCleanupOwner) -> Option<CandleCpuCleanupOwner> {
        (Arc::ptr_eq(&self.previous.owner, &expected.owner)
            && self.previous.profile == expected.profile
            && self.previous.generation == expected.generation
            && expected.generation.checked_add(2) == Some(self.current.generation))
        .then_some(self.current)
    }
}
impl CandleCpuWarmComparison {
    /// Bounded atomic refusal after reload, stop or another service load begins.
    /// Callers must still acquire/revalidate dispatch ownership before execution.
    pub fn is_current(&self) -> bool {
        self.owner.generation.load(Ordering::Acquire) == self.generation
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ExecutionSettings {
    gemm_partitions: usize,
    tokenizer_parallelism: Option<String>,
    tokenizer_effective_parallelism: bool,
}
impl ExecutionSettings {
    fn current() -> Option<Self> {
        let tokenizer_parallelism = std::env::var("TOKENIZERS_PARALLELISM").ok();
        if tokenizer_parallelism
            .as_ref()
            .is_some_and(|s| s.len() > 128)
        {
            return None;
        }
        Some(Self {
            gemm_partitions: candle_core::utils::get_num_threads(),
            tokenizer_parallelism,
            tokenizer_effective_parallelism: tokenizers::utils::parallelism::get_parallelism(),
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LoadedCpuProfile {
    pub(crate) instance: uuid::Uuid,
    model_ref: PumasModelRef,
    digest: blake3::Hash,
    settings: ExecutionSettings,
}
impl LoadedCpuProfile {
    pub(crate) fn from_loaded_bytes(
        model_ref: &PumasModelRef,
        weights: &[u8],
        inputs: &[(String, Vec<u8>)],
        width: usize,
    ) -> Option<Self> {
        if !bounded_model(model_ref) {
            return None;
        }
        let settings = ExecutionSettings::current()?;
        let mut hash = blake3::Hasher::new();
        // The process owner also binds the actual private pool. Nothing crosses owners.
        hash.update(b"candle.cpu/f32/bert/masked-mean/l2/v1");
        hash.update(include_bytes!("../../../Cargo.lock"));
        hash.update(include_bytes!("backend/candle_embedding.rs"));
        hash.update(&width.to_le_bytes());
        hash.update(&(weights.len() as u64).to_le_bytes());
        hash.update(weights);
        for (path, bytes) in inputs {
            hash.update(&(path.len() as u64).to_le_bytes());
            hash.update(path.as_bytes());
            hash.update(&(bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
        Some(Self {
            instance: uuid::Uuid::new_v4(),
            model_ref: model_ref.clone(),
            digest: hash.finalize(),
            settings,
        })
    }
}

struct Sample {
    generation: u64,
    at: u64,
    execution: u64,
    drain: u64,
}
struct Entry {
    key: blake3::Hash,
    samples: VecDeque<Sample>,
}
#[derive(Default)]
struct Store {
    accepted_generation: u64,
    latest_timestamp: Option<u64>,
    profile: Option<LoadedCpuProfile>,
    entries: VecDeque<Entry>,
    warm: bool,
}
#[cfg(test)]
type AttemptHook = Arc<dyn Fn(&str) + Send + Sync>;
pub(crate) struct CalibrationOwner {
    #[cfg(test)]
    pub(crate) attempt_hook: Mutex<Option<AttemptHook>>,
    pub(crate) pool: rayon::ThreadPool,
    config: CandleCpuCalibrationConfig,
    clock: Arc<dyn Fn() -> Option<u64> + Send + Sync>,
    synthetic: bool,
    // Odd generations are invalid. Publication is the only transition to even.
    generation: AtomicU64,
    store: Mutex<Store>,
}
impl CalibrationOwner {
    #[cfg(test)]
    pub(crate) fn test_attempt_phase(&self, phase: &str) {
        let hook = self.attempt_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(phase);
        }
    }
    pub(crate) fn cleanup_owner(
        self: &Arc<Self>,
        instance: uuid::Uuid,
    ) -> Option<CandleCpuCleanupOwner> {
        if self.synthetic {
            return None;
        }
        let generation = self.generation.load(Ordering::Acquire);
        if generation == 0 || !generation.is_multiple_of(2) {
            return None;
        }
        let store = self.store.try_lock().ok()?;
        let profile = store.profile.as_ref()?;
        if profile.instance != instance
            || ExecutionSettings::current().as_ref() != Some(&profile.settings)
            || self.generation.load(Ordering::Acquire) != generation
        {
            return None;
        }
        Some(CandleCpuCleanupOwner {
            owner: self.clone(),
            profile: profile.clone(),
            generation,
        })
    }

    pub(crate) fn matches_cleanup_owner(
        self: &Arc<Self>,
        expected: &CandleCpuCleanupOwner,
        instance: uuid::Uuid,
    ) -> bool {
        Arc::ptr_eq(self, &expected.owner)
            && self.cleanup_owner(instance).is_some_and(|actual| {
                actual.generation == expected.generation && actual.profile == expected.profile
            })
    }

    pub(crate) fn new(config: CandleCpuCalibrationConfig) -> Result<Arc<Self>, BackendError> {
        let start = Instant::now();
        Self::with_clock(
            config,
            Arc::new(move || start.elapsed().as_nanos().try_into().ok()),
            false,
        )
    }
    fn with_clock(
        config: CandleCpuCalibrationConfig,
        clock: Arc<dyn Fn() -> Option<u64> + Send + Sync>,
        synthetic: bool,
    ) -> Result<Arc<Self>, BackendError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.threads)
            .build()
            .map_err(|error| {
                BackendError::Config(format!("CPU calibration worker pool: {error}"))
            })?;
        Ok(Arc::new(Self {
            #[cfg(test)]
            attempt_hook: Mutex::new(None),
            pool,
            config,
            clock,
            synthetic,
            generation: AtomicU64::new(1),
            store: Mutex::new(Store::default()),
        }))
    }
    fn invalidate_generation(&self) -> u64 {
        self.generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                Some(if n.is_multiple_of(2) {
                    n.saturating_add(1)
                } else {
                    n.saturating_add(2)
                })
            })
            .unwrap()
    }
    pub(crate) fn invalidate(&self) {
        self.invalidate_generation();
        if let Ok(mut store) = self.store.try_lock() {
            *store = Store::default();
        }
    }
    pub(crate) fn check_worker_settings(&self, profile: &LoadedCpuProfile) {
        // GEMM's effective budget may depend on the current thread's affinity.
        // Probe on the actual pool worker as well as the gateway boundary.
        if ExecutionSettings::current().as_ref() != Some(&profile.settings) {
            self.invalidate();
        }
    }
    pub(crate) fn begin_load(self: &Arc<Self>) -> LoadPublication {
        let previous = self
            .store
            .try_lock()
            .ok()
            .and_then(|store| store.profile.as_ref().map(|profile| profile.instance))
            .and_then(|instance| self.cleanup_owner(instance));
        self.invalidate_generation();
        LoadPublication {
            owner: self.clone(),
            epoch: self.generation.load(Ordering::Acquire),
            published: false,
            previous,
        }
    }
    pub(crate) fn begin_service(
        self: &Arc<Self>,
        instance: uuid::Uuid,
        texts: &[String],
        cancellation: InferenceExecutionCancellationHandle,
    ) -> Option<ServiceObservation> {
        let key = single_text_key(texts)?;
        let generation = self.generation.load(Ordering::Acquire);
        if !generation.is_multiple_of(2) {
            return None;
        }
        let store = self.store.try_lock().ok()?;
        let profile = store.profile.as_ref()?;
        if profile.instance != instance {
            return None;
        }
        if ExecutionSettings::current().as_ref() != Some(&profile.settings) {
            drop(store);
            self.invalidate();
            return None;
        }
        if !store.warm {
            return None;
        }
        let settings = profile.settings.clone();
        drop(store);
        let Some(start) = (self.clock)() else {
            self.invalidate();
            return None;
        };
        let store = self.store.try_lock().ok()?;
        if store.latest_timestamp.is_some_and(|last| start < last) {
            drop(store);
            self.invalidate();
            return None;
        }
        drop(store);
        Some(ServiceObservation {
            owner: self.clone(),
            generation,
            key,
            settings,
            cancellation,
            start,
            execution_end: None,
            drain_end: None,
            accepted: false,
        })
    }
    pub(crate) fn compare(
        self: &Arc<Self>,
        instance: uuid::Uuid,
        model_ref: &PumasModelRef,
        texts: [&str; 2],
    ) -> Option<CandleCpuWarmComparison> {
        self.compare_bound(instance, model_ref, texts, false)
    }
    #[cfg(test)]
    fn compare_inner(
        self: &Arc<Self>,
        model_ref: &PumasModelRef,
        texts: [&str; 2],
        permit_synthetic: bool,
    ) -> Option<CandleCpuWarmComparison> {
        let instance = self.store.try_lock().ok()?.profile.as_ref()?.instance;
        self.compare_bound(instance, model_ref, texts, permit_synthetic)
    }
    fn compare_bound(
        self: &Arc<Self>,
        instance: uuid::Uuid,
        model_ref: &PumasModelRef,
        texts: [&str; 2],
        permit_synthetic: bool,
    ) -> Option<CandleCpuWarmComparison> {
        if !bounded_model(model_ref) {
            return None;
        }
        if self.synthetic && !permit_synthetic {
            return None;
        }
        let keys = [text_key(texts[0])?, text_key(texts[1])?];
        let generation = self.generation.load(Ordering::Acquire);
        if !generation.is_multiple_of(2) {
            return None;
        }
        let Some(now) = (self.clock)() else {
            self.invalidate();
            return None;
        };
        let age: u64 = self.config.maximum_age.as_nanos().try_into().ok()?;
        let store = self.store.try_lock().ok()?;
        if store.latest_timestamp.is_some_and(|last| now < last) {
            drop(store);
            self.invalidate();
            return None;
        }
        let profile = store.profile.as_ref()?;
        if profile.instance != instance {
            return None;
        }
        if &profile.model_ref != model_ref {
            return None;
        }
        if ExecutionSettings::current().as_ref() != Some(&profile.settings) {
            drop(store);
            self.invalidate();
            return None;
        }
        let mut costs = [0; 2];
        let mut counts = [0; 2];
        for i in 0..2 {
            let entry = store.entries.iter().find(|e| e.key == keys[i])?;
            for sample in &entry.samples {
                // Samples retain their original generation; verified reuse can span generations.
                if sample.generation > generation {
                    return None;
                }
                let elapsed = now.checked_sub(sample.at)?;
                if elapsed > age {
                    continue;
                }
                costs[i] = costs[i].max(sample.execution.checked_add(sample.drain)?);
                counts[i] += 1;
            }
            if counts[i] < self.config.minimum_samples {
                return None;
            }
        }
        if self.generation.load(Ordering::Acquire) != generation {
            return None;
        }
        Some(CandleCpuWarmComparison {
            execution_and_drain_ns: costs,
            sample_counts: counts,
            selected_position: usize::from(costs[1] < costs[0]),
            generation,
            owner: self.clone(),
        })
    }
}
fn text_key(text: &str) -> Option<blake3::Hash> {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES {
        return None;
    }
    Some(blake3::hash(text.as_bytes()))
}
fn bounded_model(model: &PumasModelRef) -> bool {
    model.model_id.len() <= 1024
        && model.revision.as_ref().is_none_or(|s| s.len() <= 1024)
        && model
            .selected_artifact_id
            .as_ref()
            .is_none_or(|s| s.len() <= 1024)
        && model
            .selected_artifact_path
            .as_ref()
            .is_none_or(|s| s.len() <= 4096)
        && model.migration_diagnostics.is_empty()
}
fn single_text_key(texts: &[String]) -> Option<blake3::Hash> {
    match texts {
        [text] => text_key(text),
        _ => None,
    }
}

pub(crate) struct LoadPublication {
    owner: Arc<CalibrationOwner>,
    epoch: u64,
    published: bool,
    previous: Option<CandleCpuCleanupOwner>,
}
impl LoadPublication {
    pub(crate) fn publish(
        mut self,
        profile: Option<LoadedCpuProfile>,
        reused: bool,
    ) -> Option<CandleCpuVerifiedWarmLoad> {
        let profile = profile?;
        let mut verified = None;
        if let Ok(mut store) = self.owner.store.try_lock() {
            if self.owner.generation.load(Ordering::Acquire) != self.epoch {
                return None;
            }
            if !reused
                || store.profile.as_ref() != Some(&profile)
                || store.accepted_generation.checked_add(1) != Some(self.epoch)
            {
                store.entries.clear();
                store.latest_timestamp = None;
            }
            store.profile = Some(profile.clone());
            store.warm = reused;
            if let Some(next) = self.epoch.checked_add(1) {
                store.accepted_generation = next;
                self.published = self
                    .owner
                    .generation
                    .compare_exchange(self.epoch, next, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
                if self.published && reused {
                    if let Some(previous) = self.previous.take().filter(|previous| {
                        previous.profile == profile
                            && previous.generation.checked_add(1) == Some(self.epoch)
                            && previous.generation.checked_add(2) == Some(next)
                    }) {
                        verified = Some(CandleCpuVerifiedWarmLoad {
                            previous,
                            current: CandleCpuCleanupOwner {
                                owner: self.owner.clone(),
                                profile,
                                generation: next,
                            },
                        });
                    }
                }
            }
        }
        verified
    }
}
impl Drop for LoadPublication {
    fn drop(&mut self) {
        if !self.published {
            self.owner.invalidate();
        }
    }
}

/// Only the gateway can finish this guard, after actual worker drain and validation.
pub(crate) struct ServiceObservation {
    owner: Arc<CalibrationOwner>,
    generation: u64,
    key: blake3::Hash,
    settings: ExecutionSettings,
    cancellation: InferenceExecutionCancellationHandle,
    start: u64,
    execution_end: Option<u64>,
    drain_end: Option<u64>,
    accepted: bool,
}
impl ServiceObservation {
    pub(crate) fn execution_finished(&mut self) {
        self.execution_end = (self.owner.clock)();
    }
    pub(crate) fn drain_finished(&mut self) {
        self.drain_end = (self.owner.clock)();
    }
    pub(crate) fn completed(mut self) {
        let Some(end) = self.drain_end else { return };
        let Some(execution_end) = self.execution_end else {
            return;
        };
        let Some(execution) = execution_end.checked_sub(self.start) else {
            return;
        };
        let Some(drain) = end.checked_sub(execution_end) else {
            return;
        };
        if execution == 0
            || self
                .cancellation
                .rejection_message("CPU observation completion")
                .is_some()
            || ExecutionSettings::current().as_ref() != Some(&self.settings)
        {
            return;
        }
        let Ok(mut store) = self.owner.store.try_lock() else {
            return;
        };
        if self.owner.generation.load(Ordering::Acquire) != self.generation {
            return;
        }
        if !store.entries.iter().any(|entry| entry.key == self.key) {
            if store.entries.len() == MAX_KEYS {
                store.entries.pop_front();
            }
            store.entries.push_back(Entry {
                key: self.key,
                samples: VecDeque::new(),
            });
        }
        let entry = store
            .entries
            .iter_mut()
            .find(|entry| entry.key == self.key)
            .unwrap();
        if entry.samples.len() == MAX_SAMPLES {
            entry.samples.pop_front();
        }
        entry.samples.push_back(Sample {
            generation: self.generation,
            at: end,
            execution,
            drain,
        });
        store.latest_timestamp = Some(end);
        self.accepted = true;
    }
}
impl Drop for ServiceObservation {
    fn drop(&mut self) {
        if !self.accepted {
            self.owner.invalidate();
        }
    }
}

pub(crate) struct CalibratedCandleFactory(pub(crate) Arc<CalibrationOwner>);
impl crate::BackendFactory for CalibratedCandleFactory {
    fn create(&self) -> Result<Box<dyn crate::InferenceBackend>, BackendError> {
        Ok(Box::new(crate::CandleBackend::with_calibration(
            self.0.clone(),
        )))
    }
    fn info(&self) -> crate::BackendInfo {
        crate::backend::registry::CandleFactory.info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InferenceExecutionInput, InferenceExecutionRequest, InferenceGateway};

    #[derive(Default)]
    struct Clock {
        script: Mutex<VecDeque<Option<u64>>>,
        now: AtomicU64,
    }
    impl Clock {
        fn read(&self) -> Option<u64> {
            self.script
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Some(self.now.load(Ordering::Relaxed)))
        }
        fn phase_times(&self, times: [Option<u64>; 3]) {
            assert!(self.script.lock().unwrap().is_empty());
            *self.script.lock().unwrap() = times.into();
            if let Some(end) = times[2] {
                self.now.store(end, Ordering::Relaxed);
            }
        }
    }
    fn calibrated() -> (InferenceGateway, Arc<CalibrationOwner>, Arc<Clock>) {
        let clock = Arc::new(Clock::default());
        let reader = clock.clone();
        let owner = CalibrationOwner::with_clock(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_nanos(100_000)).unwrap(),
            Arc::new(move || reader.read()),
            true,
        )
        .unwrap();
        let gateway =
            InferenceGateway::new_with_calibrated_candle_cpu_owner(owner.clone()).unwrap();
        (gateway, owner, clock)
    }
    async fn call(
        gateway: &InferenceGateway,
        request: &InferenceExecutionRequest,
        target: &crate::PumasArtifactLoadTarget,
        decision: &crate::BackendExecutionDecision,
        text: &str,
    ) -> Result<crate::InferenceExecutionResult, crate::GatewayError> {
        let mut request = request.clone();
        request.input = InferenceExecutionInput::Embedding {
            texts: vec![text.into()],
        };
        gateway
            .execute_selected_embedding_with_cancellation(
                request,
                target.clone(),
                decision.clone(),
                InferenceExecutionCancellationHandle::running(),
            )
            .await
    }
    async fn populate(
        gateway: &InferenceGateway,
        owner: &Arc<CalibrationOwner>,
        clock: &Clock,
        request: &InferenceExecutionRequest,
        target: &crate::PumasArtifactLoadTarget,
        decision: &crate::BackendExecutionDecision,
    ) {
        call(gateway, request, target, decision, "hello")
            .await
            .unwrap(); // cold: no sample
        assert!(owner.store.lock().unwrap().entries.is_empty());
        for i in 0..3 {
            let base = 1000 + i * 1000;
            clock.phase_times([Some(base), Some(base + 90), Some(base + 95)]);
            call(gateway, request, target, decision, "hello world")
                .await
                .unwrap();
            clock.phase_times([Some(base + 200), Some(base + 210), Some(base + 215)]);
            call(gateway, request, target, decision, "hello")
                .await
                .unwrap();
        }
    }
    #[tokio::test]
    async fn actual_service_populates_warm_phase_costs_and_preserves_exact_identity() {
        let (_dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let (gateway, owner, clock) = calibrated();
        populate(&gateway, &owner, &clock, &request, &target, &decision).await;
        assert_eq!(owner.pool.current_num_threads(), 1);
        let comparison = owner
            .compare_inner(&target.model_ref, ["hello world", "hello"], true)
            .unwrap();
        assert_eq!(comparison.execution_and_drain_ns, [95, 15]);
        assert_eq!(comparison.sample_counts, [3, 3]);
        assert_eq!(comparison.selected_position, 1);
        assert!(comparison.is_current());
        let tied = owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .unwrap();
        assert_eq!(tied.selected_position, 0);
        assert!(owner
            .compare_inner(&target.model_ref, [" hello", "hello"], true)
            .is_none());
        assert!(owner
            .compare_inner(
                &target.model_ref,
                ["hello", &"x".repeat(MAX_TEXT_BYTES + 1)],
                true
            )
            .is_none());
        let mut wrong = target.model_ref.clone();
        wrong.revision = Some("other".into());
        assert!(owner
            .compare_inner(&wrong, ["hello", "hello"], true)
            .is_none());
        // A controlled clock can exercise collection, never production qualification.
        assert!(gateway
            .compare_resident_cpu_embeddings(&target.model_ref, ["hello world", "hello"])
            .is_none());
        clock.now.store(200_000, Ordering::Relaxed);
        assert!(owner
            .compare_inner(&target.model_ref, ["hello world", "hello"], true)
            .is_none());
    }

    #[tokio::test]
    async fn public_real_clock_owners_compare_actual_warm_service_then_refuse_after_stop() {
        let (_dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let config = CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(3600)).unwrap();
        let gateways = [
            InferenceGateway::new_calibrated_candle_cpu(config.clone()).unwrap(),
            #[cfg(feature = "backend-llamacpp")]
            InferenceGateway::new()
                .with_candle_cpu_calibration(config)
                .unwrap(),
        ];
        for gateway in gateways {
            assert!(gateway
                .compare_resident_cpu_embeddings(&target.model_ref, ["hello", "hello world"])
                .is_none());
            call(&gateway, &request, &target, &decision, "hello")
                .await
                .unwrap(); // actual cold load
            for _ in 0..3 {
                for text in ["hello", "hello world"] {
                    call(&gateway, &request, &target, &decision, text)
                        .await
                        .unwrap();
                }
            }
            let comparison = gateway
                .compare_resident_cpu_embeddings(&target.model_ref, ["hello", "hello world"])
                .unwrap();
            assert_eq!(comparison.sample_counts, [3, 3]);
            assert!(comparison
                .execution_and_drain_ns
                .iter()
                .all(|cost| *cost > 0));
            assert!(comparison.is_current());
            // No winner or wall-time bound is asserted for this untrained fixture.
            gateway.stop().await.unwrap();
            assert!(!comparison.is_current());
            assert!(gateway
                .compare_resident_cpu_embeddings(&target.model_ref, ["hello", "hello world"])
                .is_none());
        }
    }
    #[tokio::test]
    async fn reload_stop_and_failed_load_invalidate_real_owner_generations() {
        let (dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let (gateway, owner, clock) = calibrated();
        populate(&gateway, &owner, &clock, &request, &target, &decision).await;
        let before = owner
            .compare_inner(&target.model_ref, ["hello world", "hello"], true)
            .unwrap();
        let profile = owner.store.lock().unwrap().profile.clone().unwrap();
        let weights_path = dir.path().join("model.safetensors");
        let mut bytes = std::fs::read(&weights_path).unwrap();
        let offset = bytes.len() - 4;
        let last = f32::from_le_bytes(bytes[offset..].try_into().unwrap());
        bytes[offset..].copy_from_slice(&(last + 0.125).to_le_bytes());
        std::fs::write(&weights_path, bytes).unwrap();
        call(&gateway, &request, &target, &decision, "hello")
            .await
            .unwrap();
        assert!(!before.is_current());
        {
            let store = owner.store.lock().unwrap();
            assert!(store.entries.is_empty());
            assert_ne!(profile.digest, store.profile.as_ref().unwrap().digest);
        }
        clock.phase_times([Some(4000), Some(4010), Some(4015)]);
        call(&gateway, &request, &target, &decision, "hello")
            .await
            .unwrap();
        assert_eq!(owner.store.lock().unwrap().entries.len(), 1);
        std::fs::write(dir.path().join("tokenizer.json"), b"invalid").unwrap();
        assert!(call(&gateway, &request, &target, &decision, "hello")
            .await
            .is_err());
        assert!(owner.store.lock().unwrap().entries.is_empty());
        assert_eq!(owner.generation.load(Ordering::Acquire) % 2, 1);
        gateway.stop().await.unwrap();
        assert!(owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .is_none());
    }
    #[tokio::test]
    async fn failed_cancelled_and_regressing_service_phases_are_not_samples() {
        let (_dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let (gateway, owner, clock) = calibrated();
        call(&gateway, &request, &target, &decision, "hello")
            .await
            .unwrap();
        for times in [
            [Some(10), Some(9), Some(11)],
            [Some(20), Some(22), Some(21)],
            [Some(30), None, Some(31)],
            [Some(40), Some(40), Some(41)],
        ] {
            clock.phase_times(times);
            call(&gateway, &request, &target, &decision, "hello")
                .await
                .unwrap();
            assert!(owner.store.lock().unwrap().entries.is_empty());
        }
        let mut request = request;
        request.input = InferenceExecutionInput::Embedding {
            texts: vec!["hello".into()],
        };
        let cancelled =
            InferenceExecutionCancellationHandle::cancellation_requested("fixture cancellation");
        assert!(gateway
            .execute_selected_embedding_with_cancellation(
                request.clone(),
                target.clone(),
                decision.clone(),
                cancelled
            )
            .await
            .is_err());
        assert!(owner.store.lock().unwrap().entries.is_empty());
        // Dropping a guard, or omitting the real drain, cannot commit an observation.
        clock.phase_times([Some(100), Some(110), Some(115)]);
        call(&gateway, &request, &target, &decision, "hello")
            .await
            .unwrap();
        let instance = owner
            .store
            .lock()
            .unwrap()
            .profile
            .as_ref()
            .unwrap()
            .instance;
        let mut guard = owner
            .begin_service(
                instance,
                &["hello".into()],
                InferenceExecutionCancellationHandle::running(),
            )
            .unwrap();
        guard.execution_finished();
        drop(guard);
        assert!(owner.store.lock().unwrap().entries.is_empty());
    }
    #[tokio::test]
    async fn bounded_rings_evict_and_busy_queries_refuse_without_io() {
        let (_dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let (gateway, owner, clock) = calibrated();
        call(&gateway, &request, &target, &decision, "hello")
            .await
            .unwrap();
        let mut now = 100;
        for _ in 0..10 {
            clock.phase_times([Some(now), Some(now + 1), Some(now + 2)]);
            now += 10;
            call(&gateway, &request, &target, &decision, "hello")
                .await
                .unwrap();
        }
        assert_eq!(
            owner.store.lock().unwrap().entries[0].samples.len(),
            MAX_SAMPLES
        );
        let comparison = owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .unwrap();
        let load = owner.begin_load();
        assert!(!comparison.is_current());
        assert!(owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .is_none());
        drop(load); // unpublished generation remains unusable
        for i in 0..=MAX_KEYS {
            clock.phase_times([Some(now), Some(now + 1), Some(now + 2)]);
            now += 10;
            call(
                &gateway,
                &request,
                &target,
                &decision,
                &format!("hello {i}"),
            )
            .await
            .unwrap();
        }
        let store = owner.store.lock().unwrap();
        assert_eq!(store.entries.len(), MAX_KEYS);
        assert!(!store
            .entries
            .iter()
            .any(|e| e.key == text_key("hello 0").unwrap()));
        drop(store);
        let guard = owner.store.lock().unwrap();
        assert!(owner
            .compare_inner(&target.model_ref, ["hello 1", "hello 1"], true)
            .is_none());
        drop(guard);
    }
    #[test]
    fn injected_owner_and_invalid_pool_configuration_cannot_enable_calibration() {
        assert!(CandleCpuCalibrationConfig::new(0, 3, Duration::from_secs(1)).is_err());
        assert!(CandleCpuCalibrationConfig::new(5, 3, Duration::from_secs(1)).is_err());
        assert!(CandleCpuCalibrationConfig::new(1, 2, Duration::from_secs(1)).is_err());
        let gateway =
            InferenceGateway::with_backend(Box::new(crate::CandleBackend::new()), "Candle");
        assert!(gateway
            .with_candle_cpu_calibration(
                CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(1)).unwrap()
            )
            .is_err());
    }

    #[tokio::test]
    async fn populated_history_is_invalidated_by_clock_faults_and_foreign_instances() {
        for fault in 0..3 {
            let (_dir, request, target, decision) = crate::selected_embedding_execution::fixture(8);
            let (gateway, owner, clock) = calibrated();
            populate(&gateway, &owner, &clock, &request, &target, &decision).await;
            let before = owner
                .compare_inner(&target.model_ref, ["hello", "hello"], true)
                .unwrap();
            let wrong_instance = uuid::Uuid::new_v4();
            assert!(owner
                .compare_bound(wrong_instance, &target.model_ref, ["hello", "hello"], true)
                .is_none());
            match fault {
                0 => {
                    clock.phase_times([Some(4000), Some(3999), Some(4001)]);
                    call(&gateway, &request, &target, &decision, "hello")
                        .await
                        .unwrap();
                }
                1 => {
                    clock.phase_times([Some(4000), None, Some(4001)]);
                    call(&gateway, &request, &target, &decision, "hello")
                        .await
                        .unwrap();
                }
                _ => {
                    clock.now.store(1, Ordering::Relaxed);
                    assert!(owner
                        .compare_inner(&target.model_ref, ["hello", "hello"], true)
                        .is_none());
                }
            }
            assert!(!before.is_current());
            assert!(owner.store.lock().unwrap().entries.is_empty());
        }
    }

    #[tokio::test]
    async fn dropped_service_keeps_native_worker_custody_busy_and_excludes_history() {
        use crate::{InferenceExecutionCancellationSignal, InferenceExecutionCancellationSnapshot};
        use std::sync::atomic::AtomicBool;
        struct BlockForward {
            block: Arc<AtomicBool>,
            entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
            release: Mutex<std::sync::mpsc::Receiver<()>>,
        }
        impl InferenceExecutionCancellationSignal for BlockForward {
            fn snapshot(&self) -> InferenceExecutionCancellationSnapshot {
                if self.block.swap(false, Ordering::AcqRel) {
                    self.entered
                        .lock()
                        .unwrap()
                        .take()
                        .unwrap()
                        .send(())
                        .unwrap();
                    self.release
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(2))
                        .unwrap();
                }
                InferenceExecutionCancellationSnapshot::running()
            }
        }
        let (_dir, mut request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let clock = Arc::new(Clock::default());
        let reader = clock.clone();
        let block = Arc::new(AtomicBool::new(false));
        let clock_block = block.clone();
        let owner = CalibrationOwner::with_clock(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(1)).unwrap(),
            Arc::new(move || {
                let time = reader.read();
                if time == Some(10_000) {
                    clock_block.store(true, Ordering::Release);
                }
                time
            }),
            true,
        )
        .unwrap();
        let gateway = Arc::new(
            InferenceGateway::new_with_calibrated_candle_cpu_owner(owner.clone()).unwrap(),
        );
        populate(&gateway, &owner, &clock, &request, &target, &decision).await;
        let before = owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .unwrap();
        let model_ref = target.model_ref.clone();
        assert!(gateway.cpu_calibration_instance_for_test().is_some());
        request.input = InferenceExecutionInput::Embedding {
            texts: vec!["hello".into()],
        };
        clock.phase_times([Some(10_000), Some(10_010), Some(10_015)]);
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let cancellation =
            InferenceExecutionCancellationHandle::with_signal(Arc::new(BlockForward {
                block,
                entered: Mutex::new(Some(entered_tx)),
                release: Mutex::new(release_rx),
            }));
        let worker_gateway = gateway.clone();
        let task = tokio::spawn(async move {
            worker_gateway
                .execute_selected_embedding_with_cancellation(
                    request,
                    target,
                    decision,
                    cancellation,
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), entered_rx)
            .await
            .unwrap()
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!before.is_current());
        assert!(owner.store.lock().unwrap().entries.is_empty());
        // The actual worker is blocked. Caller loss cannot release its custody.
        assert!(gateway.cpu_calibration_instance_for_test().is_none());
        assert!(gateway
            .compare_resident_cpu_embeddings(&model_ref, ["hello", "hello"])
            .is_none());
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), gateway.stop())
            .await
            .unwrap()
            .unwrap();
    }

    #[test]
    fn effective_tokenizer_mode_is_bound_in_an_isolated_process() {
        const MARKER: &str = "PANTOGRAPH_CALIBRATION_TOKENIZER_TEST_CHILD";
        if std::env::var_os(MARKER).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "candle_cpu_calibration::tests::effective_tokenizer_mode_is_bound_in_an_isolated_process", "--test-threads=1"])
                .env(MARKER, "1").status().unwrap();
            assert!(status.success());
            return;
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (_dir, request, target, decision) =
                    crate::selected_embedding_execution::fixture(8);
                let (gateway, owner, clock) = calibrated();
                populate(&gateway, &owner, &clock, &request, &target, &decision).await;
                let before = owner
                    .compare_inner(&target.model_ref, ["hello", "hello"], true)
                    .unwrap();
                let was_parallel = tokenizers::utils::parallelism::get_parallelism();
                tokenizers::utils::parallelism::set_parallelism(!was_parallel);
                assert!(owner
                    .compare_inner(&target.model_ref, ["hello", "hello"], true)
                    .is_none());
                assert!(!before.is_current());

                // Caller-side probes alone cannot see a mode changed only during
                // actual execution. Restore it at execution-finished; the pool
                // worker probes must still invalidate the already populated ring.
                let worker_mode = tokenizers::utils::parallelism::get_parallelism();
                let clock = Arc::new(Clock::default());
                let reader = clock.clone();
                let owner = CalibrationOwner::with_clock(
                    CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(1)).unwrap(),
                    Arc::new(move || {
                        let time = reader.read();
                        if time == Some(4000) {
                            tokenizers::utils::parallelism::set_parallelism(!worker_mode);
                        } else if time == Some(4010) {
                            tokenizers::utils::parallelism::set_parallelism(worker_mode);
                        }
                        time
                    }),
                    true,
                )
                .unwrap();
                let gateway =
                    InferenceGateway::new_with_calibrated_candle_cpu_owner(owner.clone()).unwrap();
                populate(&gateway, &owner, &clock, &request, &target, &decision).await;
                clock.phase_times([Some(4000), Some(4010), Some(4015)]);
                call(&gateway, &request, &target, &decision, "hello")
                    .await
                    .unwrap();
                assert!(clock.script.lock().unwrap().is_empty());
                assert_eq!(
                    tokenizers::utils::parallelism::get_parallelism(),
                    worker_mode
                );
                assert!(owner.store.lock().unwrap().entries.is_empty());
            });
    }

    #[tokio::test]
    async fn cancellation_after_real_execution_before_service_completion_invalidates_evidence() {
        use crate::{InferenceExecutionCancellationSignal, InferenceExecutionCancellationSnapshot};
        use std::sync::atomic::AtomicBool;
        struct CancelSignal(Arc<AtomicBool>);
        impl InferenceExecutionCancellationSignal for CancelSignal {
            fn snapshot(&self) -> InferenceExecutionCancellationSnapshot {
                if self.0.load(Ordering::Acquire) {
                    InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                        "fixture phase cancellation".into(),
                    ))
                } else {
                    InferenceExecutionCancellationSnapshot::running()
                }
            }
        }
        let (_dir, mut request, target, decision) = crate::selected_embedding_execution::fixture(8);
        let clock = Arc::new(Clock::default());
        let reader = clock.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let clock_cancelled = cancelled.clone();
        let owner = CalibrationOwner::with_clock(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(1)).unwrap(),
            Arc::new(move || {
                let time = reader.read();
                if time == Some(4010) {
                    clock_cancelled.store(true, Ordering::Release);
                }
                time
            }),
            true,
        )
        .unwrap();
        let gateway =
            InferenceGateway::new_with_calibrated_candle_cpu_owner(owner.clone()).unwrap();
        populate(&gateway, &owner, &clock, &request, &target, &decision).await;
        let before = owner
            .compare_inner(&target.model_ref, ["hello", "hello"], true)
            .unwrap();
        request.input = InferenceExecutionInput::Embedding {
            texts: vec!["hello".into()],
        };
        clock.phase_times([Some(4000), Some(4010), Some(4015)]);
        let result = gateway
            .execute_selected_embedding_with_cancellation(
                request,
                target,
                decision,
                InferenceExecutionCancellationHandle::with_signal(Arc::new(CancelSignal(
                    cancelled,
                ))),
            )
            .await;
        assert!(result.is_err());
        assert!(clock.script.lock().unwrap().is_empty()); // all actual phases reached
        assert!(!before.is_current());
        assert!(owner.store.lock().unwrap().entries.is_empty());
        assert!(gateway.cpu_calibration_instance_for_test().is_some()); // actual drain retired custody
    }
}

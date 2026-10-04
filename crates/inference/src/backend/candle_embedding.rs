//! The admitted CPU F32 BERT recipe, using the locked Candle BERT implementation.
use super::candle::{CandleEmbeddingLoadPlan, CandleLoadDType, CandleLoadDevice};
use super::{BackendError, EmbeddingResult};
use crate::{InferenceExecutionCancellationHandle, PumasArtifactLoadTarget};
use candle_core::{DType, Tensor};
use candle_transformers::models::bert::{BertModel, Config};
use futures_util::{future::Shared, FutureExt};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Component, Path};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

fn invalid(message: impl std::fmt::Display) -> BackendError {
    BackendError::Config(format!("Candle BERT embedding: {message}"))
}
fn inference(message: impl std::fmt::Display) -> BackendError {
    BackendError::Inference(format!("Candle BERT embedding: {message}"))
}

#[derive(Deserialize)]
struct Module {
    idx: usize,
    path: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Retained with the model: exact selected target and the actual parsed input bytes.
/// This is an in-memory load snapshot, not a new package/residency authority.
struct LoadIdentity {
    target: PumasArtifactLoadTarget,
    plan: CandleEmbeddingLoadPlan,
    inputs: Vec<(String, Vec<u8>)>,
    weights: FileIdentity,
}

#[derive(Debug, PartialEq, Eq)]
struct FileIdentity {
    path: std::path::PathBuf,
    len: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    inode: (u64, u64, i64, i64),
}

impl FileIdentity {
    fn read(path: &Path) -> Result<Self, BackendError> {
        let path = path.canonicalize().map_err(invalid)?;
        let metadata = std::fs::metadata(&path).map_err(invalid)?;
        if !metadata.is_file() {
            return Err(invalid("checkpoint must be a regular file"));
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            path,
            len: metadata.len(),
            modified: metadata.modified().map_err(invalid)?,
            #[cfg(unix)]
            inode: (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            ),
        })
    }
}

#[derive(Clone)]
pub(super) struct LoadedEmbedding {
    pub(super) model: Arc<EmbeddingModel>,
    pub(super) reused: bool,
}

pub(super) fn check_load_stop(
    stop: &AtomicBool,
    cancellation: &InferenceExecutionCancellationHandle,
) -> Result<(), BackendError> {
    if stop.load(Ordering::Acquire) {
        Err(BackendError::Cancelled(
            "Candle model load cancelled".into(),
        ))
    } else if let Some(reason) = cancellation.rejection_message("Candle model load") {
        Err(BackendError::Cancelled(reason))
    } else {
        Ok(())
    }
}

pub(super) struct EmbeddingModel {
    bert: BertModel,
    tokenizer: tokenizers::Tokenizer,
    width: usize,
    identity: LoadIdentity,
}

fn read_input(
    root: &Path,
    relative: &Path,
    inputs: &mut Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, BackendError> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(invalid("unsafe recipe/component path"));
    }
    let path = root.join(relative).canonicalize().map_err(invalid)?;
    if !path.starts_with(root.canonicalize().map_err(invalid)?) {
        return Err(invalid("recipe/component resolves outside selected target"));
    }
    let bytes = std::fs::read(&path).map_err(invalid)?;
    inputs.push((relative.to_string_lossy().into_owned(), bytes.clone()));
    Ok(bytes)
}

impl EmbeddingModel {
    pub(super) fn accepts_model_name(&self, name: &str) -> bool {
        fn identity(name: &str) -> &str {
            name.strip_prefix("pumas://models/").unwrap_or(name)
        }
        name.is_empty() || identity(name) == identity(&self.identity.target.model_ref.model_id)
    }

    pub(super) fn load(
        plan: CandleEmbeddingLoadPlan,
        target: PumasArtifactLoadTarget,
        stop: &AtomicBool,
        cancellation: &InferenceExecutionCancellationHandle,
    ) -> Result<Self, BackendError> {
        check_load_stop(stop, cancellation)?;
        if plan.dtype != CandleLoadDType::F32
            || !matches!(plan.device, CandleLoadDevice::Auto | CandleLoadDevice::Cpu)
        {
            return Err(invalid("executable profile requires CPU F32"));
        }
        let root = &plan.model_dir;
        let mut inputs = Vec::new();
        let config_bytes = read_input(
            root,
            plan.config_path.strip_prefix(root).map_err(invalid)?,
            &mut inputs,
        )?;
        let config_json: Value = serde_json::from_slice(&config_bytes).map_err(invalid)?;
        if config_json["model_type"] != "bert"
            || config_json["architectures"] != serde_json::json!(["BertModel"])
            || plan.architecture.as_deref() != Some("BertModel")
            || config_json["is_decoder"].as_bool() == Some(true)
            || config_json["add_cross_attention"].as_bool() == Some(true)
            || config_json["position_embedding_type"]
                .as_str()
                .is_some_and(|kind| kind != "absolute")
            || config_json["torch_dtype"]
                .as_str()
                .is_some_and(|kind| !matches!(kind, "float32" | "f32"))
        {
            return Err(invalid(
                "only an absolute-position BertModel encoder with F32 weights is supported",
            ));
        }
        let config: Config = serde_json::from_slice(&config_bytes).map_err(invalid)?;
        if config.hidden_size == 0
            || config.num_attention_heads == 0
            || !config
                .hidden_size
                .is_multiple_of(config.num_attention_heads)
            || config.num_hidden_layers == 0
            || config.intermediate_size == 0
            || config.vocab_size == 0
            || config.type_vocab_size == 0
            || config.max_position_embeddings == 0
            || !config.layer_norm_eps.is_finite()
            || config.layer_norm_eps <= 0.0
        {
            return Err(invalid(
                "invalid BERT dimensions or layer normalization epsilon",
            ));
        }
        let modules: Vec<Module> =
            serde_json::from_slice(&read_input(root, Path::new("modules.json"), &mut inputs)?)
                .map_err(invalid)?;
        let expected = ["Transformer", "Pooling", "Normalize"];
        if modules.len() != expected.len()
            || modules
                .iter()
                .zip(expected)
                .enumerate()
                .any(|(index, (module, kind))| {
                    module.idx != index
                        || module.kind != format!("sentence_transformers.models.{kind}")
                })
            || !modules[0].path.is_empty()
            || modules[1].path.is_empty()
            || modules[2].path.is_empty()
        {
            return Err(invalid(
                "explicit Transformer -> Pooling -> Normalize modules are required",
            ));
        }
        // Normalize has no configuration in the supported sentence-transformers recipe.
        let normalize_relative = Path::new(&modules[2].path);
        if normalize_relative.is_absolute()
            || normalize_relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(invalid("unsafe Normalize module path"));
        }
        let normalize_path = root.join(normalize_relative);
        // Standard Normalize.save writes no files; Git packages can omit its directory.
        // If a payload exists, reject unsupported configuration rather than ignoring it.
        if normalize_path.exists() {
            let canonical = normalize_path.canonicalize().map_err(invalid)?;
            if !canonical.starts_with(root.canonicalize().map_err(invalid)?)
                || !canonical.is_dir()
                || std::fs::read_dir(canonical)
                    .map_err(invalid)?
                    .next()
                    .is_some()
            {
                return Err(invalid(
                    "Normalize has an unsupported or out-of-target payload",
                ));
            }
        }
        let pooling: Value = serde_json::from_slice(&read_input(
            root,
            &Path::new(&modules[1].path).join("config.json"),
            &mut inputs,
        )?)
        .map_err(invalid)?;
        if pooling["word_embedding_dimension"].as_u64() != Some(config.hidden_size as u64)
            || pooling["pooling_mode_mean_tokens"] != true
            || pooling["include_prompt"] != true
            || pooling.as_object().is_none_or(|fields| {
                fields.iter().any(|(key, value)| {
                    key.starts_with("pooling_mode_")
                        && key != "pooling_mode_mean_tokens"
                        && value != false
                })
            })
        {
            return Err(invalid(
                "only declared masked mean pooling including special tokens is supported",
            ));
        }
        let sentence: Value = serde_json::from_slice(&read_input(
            root,
            Path::new("sentence_bert_config.json"),
            &mut inputs,
        )?)
        .map_err(invalid)?;
        let max_length = sentence["max_seq_length"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| invalid("explicit max_seq_length is required"))?;
        if max_length == 0 || max_length > config.max_position_embeddings {
            return Err(invalid(
                "recipe sequence length exceeds BERT position capacity",
            ));
        }
        let tokenizer_bytes = read_input(
            root,
            plan.tokenizer_path.strip_prefix(root).map_err(invalid)?,
            &mut inputs,
        )?;
        let mut tokenizer = tokenizers::Tokenizer::from_bytes(&tokenizer_bytes).map_err(invalid)?;
        if tokenizer
            .get_vocab(true)
            .values()
            .any(|id| *id as usize >= config.vocab_size)
        {
            return Err(invalid("tokenizer IDs exceed the checkpoint vocabulary"));
        }
        let tokenizer_json: Value = serde_json::from_slice(&tokenizer_bytes).map_err(invalid)?;
        let lowercase = sentence["do_lower_case"]
            .as_bool()
            .ok_or_else(|| invalid("explicit do_lower_case is required"))?;
        if Some(lowercase) != tokenizer_json["normalizer"]["lowercase"].as_bool()
            || tokenizer_json["normalizer"]["type"] != "BertNormalizer"
        {
            return Err(invalid(
                "declared lowercasing must match the BERT tokenizer",
            ));
        }
        let padding = tokenizer
            .get_padding()
            .cloned()
            .ok_or_else(|| invalid("explicit tokenizer padding is required"))?;
        if !matches!(padding.direction, tokenizers::PaddingDirection::Right)
            || padding.pad_id != config.pad_token_id as u32
            || padding.pad_type_id != 0
            || padding.pad_to_multiple_of.is_some()
            || !matches!(padding.strategy, tokenizers::PaddingStrategy::BatchLongest)
        {
            return Err(invalid(
                "only declared right BatchLongest padding with matching pad ID is supported",
            ));
        }
        let truncation = tokenizer
            .get_truncation()
            .cloned()
            .ok_or_else(|| invalid("explicit tokenizer truncation is required"))?;
        if truncation.direction != tokenizers::TruncationDirection::Right
            || truncation.strategy != tokenizers::TruncationStrategy::LongestFirst
            || truncation.stride != 0
        {
            return Err(invalid(
                "only right LongestFirst truncation without stride is supported",
            ));
        }
        if truncation.max_length != max_length {
            return Err(invalid("tokenizer and recipe truncation lengths disagree"));
        }
        tokenizer
            .with_truncation(Some(truncation))
            .map_err(invalid)?;
        // Resolve the actual weight file inside the same target before loading it.
        let weights = plan.safetensors_path.canonicalize().map_err(invalid)?;
        if !weights.starts_with(root.canonicalize().map_err(invalid)?) {
            return Err(invalid("weights resolve outside selected target"));
        }
        check_load_stop(stop, cancellation)?;
        let weights_identity = FileIdentity::read(&weights)?;
        let tensors =
            candle_core::safetensors::load(&weights, &candle_core::Device::Cpu).map_err(invalid)?;
        check_load_stop(stop, cancellation)?;
        if FileIdentity::read(&weights)? != weights_identity {
            return Err(invalid("checkpoint changed during model loading"));
        }
        if tensors.is_empty() || tensors.values().any(|tensor| tensor.dtype() != DType::F32) {
            return Err(invalid("all checkpoint tensors must be F32"));
        }
        let vb =
            candle_nn::VarBuilder::from_tensors(tensors, DType::F32, &candle_core::Device::Cpu);
        let bert = BertModel::load(vb, &config).map_err(invalid)?;
        check_load_stop(stop, cancellation)?;
        Ok(Self {
            bert,
            tokenizer,
            width: config.hidden_size,
            identity: LoadIdentity {
                target,
                plan,
                inputs,
                weights: weights_identity,
            },
        })
    }

    pub(super) fn matches_load(
        &self,
        plan: &CandleEmbeddingLoadPlan,
        target: &PumasArtifactLoadTarget,
    ) -> bool {
        if self.identity.target != *target || self.identity.plan != *plan {
            return false;
        }
        let Ok(root) = plan.model_dir.canonicalize() else {
            return false;
        };
        // Recheck small recipe/tokenizer bytes and the physical checkpoint
        // identity, without deserializing tensors or rebuilding the BERT model.
        let mut inputs = Vec::new();
        for (relative, bytes) in &self.identity.inputs {
            if read_input(&plan.model_dir, Path::new(relative), &mut inputs)
                .map_or(true, |current| current != *bytes)
            {
                return false;
            }
        }
        let normalize: Result<Vec<Module>, _> = serde_json::from_slice(
            &self
                .identity
                .inputs
                .iter()
                .find(|(path, _)| path == "modules.json")
                .unwrap()
                .1,
        );
        let Ok(modules) = normalize else { return false };
        let path = plan.model_dir.join(&modules[2].path);
        if path.exists()
            && (path
                .canonicalize()
                .map_or(true, |path| !path.starts_with(&root))
                || std::fs::read_dir(path).map_or(true, |mut entries| entries.next().is_some()))
        {
            return false;
        }
        FileIdentity::read(&plan.safetensors_path)
            .is_ok_and(|current| current == self.identity.weights)
    }

    pub(super) fn forward(
        &self,
        texts: Vec<String>,
        cancellation: &InferenceExecutionCancellationHandle,
        stop: &AtomicBool,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        let check = || {
            if stop.load(Ordering::Acquire) {
                return Err(BackendError::Cancelled("Candle embedding stopped".into()));
            }
            cancellation
                .rejection_message("Candle embedding")
                .map_or(Ok(()), |reason| Err(BackendError::Cancelled(reason)))
        };
        check()?;
        if texts.is_empty() || texts.iter().any(|text| text.trim().is_empty()) {
            return Err(invalid("a nonempty batch of nonblank texts is required"));
        }
        let encodings = self
            .tokenizer
            .encode_batch(texts, true)
            .map_err(inference)?;
        let batch = encodings.len();
        let length = encodings[0].len();
        if length == 0
            || encodings.iter().any(|encoding| {
                encoding.len() != length || !encoding.get_attention_mask().contains(&1)
            })
        {
            return Err(inference("tokenizer produced inconsistent batch shapes"));
        }
        let ids: Vec<u32> = encodings
            .iter()
            .flat_map(|encoding| encoding.get_ids().iter().copied())
            .collect();
        let types: Vec<u32> = encodings
            .iter()
            .flat_map(|encoding| encoding.get_type_ids().iter().copied())
            .collect();
        let masks: Vec<u32> = encodings
            .iter()
            .flat_map(|encoding| encoding.get_attention_mask().iter().copied())
            .collect();
        let device = &self.bert.device;
        let ids = Tensor::from_vec(ids, (batch, length), device).map_err(inference)?;
        let types = Tensor::from_vec(types, (batch, length), device).map_err(inference)?;
        let mask = Tensor::from_vec(masks, (batch, length), device).map_err(inference)?;
        check()?;
        // A running native forward is not preemptible; completion remains backend-owned.
        let hidden = self
            .bert
            .forward(&ids, &types, Some(&mask))
            .map_err(inference)?;
        check()?;
        let mask = mask.to_dtype(DType::F32).map_err(inference)?;
        let counts = mask.sum_keepdim(1).map_err(inference)?;
        let mean = hidden
            .broadcast_mul(&mask.unsqueeze(2).map_err(inference)?)
            .and_then(|tensor| tensor.sum(1))
            .and_then(|tensor| tensor.broadcast_div(&counts))
            .map_err(inference)?;
        let norms = mean
            .sqr()
            .and_then(|tensor| tensor.sum_keepdim(1))
            .and_then(|tensor| tensor.sqrt())
            .and_then(|tensor| tensor.clamp(1e-12f32, f32::MAX))
            .map_err(inference)?;
        let vectors = mean
            .broadcast_div(&norms)
            .and_then(|tensor| tensor.to_vec2::<f32>())
            .map_err(inference)?;
        if vectors.len() != batch
            || vectors.iter().any(|vector| {
                vector.len() != self.width || vector.iter().any(|value| !value.is_finite())
            })
        {
            return Err(inference(
                "nonfinite or incorrectly shaped embedding result",
            ));
        }
        check()?;
        Ok(vectors
            .into_iter()
            .zip(encodings)
            .map(|(vector, encoding)| EmbeddingResult {
                vector,
                token_count: encoding
                    .get_attention_mask()
                    .iter()
                    .map(|value| *value as usize)
                    .sum(),
            })
            .collect())
    }
}

type Completion<T> = Shared<futures_util::future::BoxFuture<'static, Arc<Result<T, BackendError>>>>;
#[derive(Clone)]
struct Job<T> {
    stop: Arc<AtomicBool>,
    completion: Completion<T>,
}
pub(super) struct EmbeddingJobs<T = Vec<EmbeddingResult>>(Mutex<Option<Job<T>>>);

impl<T> Default for EmbeddingJobs<T> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

struct CancelLoadOnDrop(Arc<AtomicBool>);
impl Drop for CancelLoadOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn copy_result<T: Clone>(result: &Result<T, BackendError>) -> Result<T, BackendError> {
    match result {
        Ok(embeddings) => Ok(embeddings.clone()),
        Err(BackendError::Cancelled(reason)) => Err(BackendError::Cancelled(reason.clone())),
        Err(BackendError::Config(reason)) => Err(BackendError::Config(reason.clone())),
        Err(error) => Err(inference(error)),
    }
}

impl EmbeddingJobs {
    pub(super) async fn execute(
        &self,
        model: Arc<EmbeddingModel>,
        texts: Vec<String>,
        cancellation: InferenceExecutionCancellationHandle,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        let completion = self.spawn(move |stop| model.forward(texts, &cancellation, &stop))?;
        copy_result(completion.await.as_ref())
    }
}

impl<T: Clone + Send + Sync + 'static> EmbeddingJobs<T> {
    pub(super) async fn load(
        &self,
        run: impl FnOnce(Arc<AtomicBool>) -> Result<T, BackendError> + Send + 'static,
    ) -> Result<T, BackendError> {
        let job = self.start(run)?;
        // Caller loss signals cancellation; the slot retains the actual join.
        // Replacement and stop must drain that join before releasing residency.
        let _cancel = CancelLoadOnDrop(job.stop);
        copy_result(job.completion.await.as_ref())
    }

    fn spawn(
        &self,
        run: impl FnOnce(Arc<AtomicBool>) -> Result<T, BackendError> + Send + 'static,
    ) -> Result<Completion<T>, BackendError> {
        Ok(self.start(run)?.completion)
    }

    fn start(
        &self,
        run: impl FnOnce(Arc<AtomicBool>) -> Result<T, BackendError> + Send + 'static,
    ) -> Result<Job<T>, BackendError> {
        let mut slot = self.0.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|job| job.completion.peek().is_none())
        {
            return Err(inference("embedding worker is still owned"));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let handle = tokio::task::spawn_blocking(move || run(worker_stop));
        let completion = async move {
            Arc::new(
                handle
                    .await
                    .unwrap_or_else(|error| Err(inference(format!("worker join failed: {error}")))),
            )
        }
        .boxed()
        .shared();
        let job = Job { stop, completion };
        *slot = Some(job.clone());
        Ok(job)
    }
    pub(super) async fn drain(&self, cancel: bool) -> Result<(), BackendError> {
        let job = self.0.lock().unwrap().clone();
        if let Some(job) = job {
            if cancel {
                job.stop.store(true, Ordering::Release);
            }
            job.completion.await;
            // Execution owns the request outcome. Lifecycle cleanup observes the
            // join, then retires this completed job even when inference failed.
            // A concurrent request may already have installed a different job.
            let mut slot = self.0.lock().unwrap();
            if slot
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&current.stop, &job.stop))
            {
                *slot = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InferenceBackend;

    #[tokio::test]
    async fn real_candle_forward_matches_two_independent_transformers_references() {
        for width in [8, 12] {
            let (directory, request, target, decision) =
                crate::selected_embedding_execution::fixture(width);
            if width == 8 {
                std::fs::remove_dir(directory.path().join("2_Normalize")).unwrap();
            }
            let golden: Value = serde_json::from_slice(
                &std::fs::read(directory.path().join("golden.json")).unwrap(),
            )
            .unwrap();
            let mut backend = super::super::candle::CandleBackend::new();
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            assert!(backend.is_ready());
            let texts: Vec<String> = serde_json::from_value(golden["texts"].clone()).unwrap();
            let batch = backend
                .embeddings(texts.clone(), &target.model_ref.model_id)
                .await
                .unwrap();
            let encodings = backend
                .model
                .as_ref()
                .unwrap()
                .tokenizer
                .encode_batch(texts.clone(), true)
                .unwrap();
            for (index, encoding) in encodings.iter().enumerate() {
                assert_eq!(
                    serde_json::to_value(encoding.get_ids()).unwrap(),
                    golden["input_ids"][index]
                );
                assert_eq!(
                    serde_json::to_value(encoding.get_attention_mask()).unwrap(),
                    golden["attention_mask"][index]
                );
                assert_eq!(
                    serde_json::to_value(encoding.get_type_ids()).unwrap(),
                    golden["token_type_ids"][index]
                );
            }
            let mut max_error = 0.0f32;
            for (index, (embedding, text)) in batch.iter().zip(texts).enumerate() {
                let single = backend.embeddings(vec![text], "").await.unwrap();
                assert_eq!(embedding.vector.len(), width);
                assert_eq!(embedding.token_count, if index == 1 { 3 } else { 4 });
                let norm = embedding
                    .vector
                    .iter()
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt();
                assert!((norm - 1.0).abs() < 1e-5);
                for (column, (&actual, &one)) in
                    embedding.vector.iter().zip(&single[0].vector).enumerate()
                {
                    let expected = golden["batch_vectors"][index][column].as_f64().unwrap() as f32;
                    let single_expected =
                        golden["single_vectors"][index][column].as_f64().unwrap() as f32;
                    let tolerance = 1e-5 + 1e-4 * expected.abs();
                    max_error = max_error.max((actual - expected).abs());
                    assert!(
                        (actual - expected).abs() <= tolerance,
                        "width={width} item={index} column={column}: {actual} != {expected}"
                    );
                    assert!((one - single_expected).abs() <= 1e-5 + 1e-4 * single_expected.abs());
                    assert!(
                        (actual - one).abs() <= tolerance,
                        "padding changed an embedding"
                    );
                }
            }
            println!(
                "actual CPU F32 Candle BERT width={width}, max_abs_reference_error={max_error:e}"
            );
            let long_texts: Vec<String> =
                serde_json::from_value(golden["truncation_texts"].clone()).unwrap();
            let truncated = backend.embeddings(long_texts, "").await.unwrap();
            assert_eq!(truncated[0].token_count, 16);
            assert_eq!(truncated[1].token_count, 3);
            for (index, embedding) in truncated.iter().enumerate() {
                for (column, actual) in embedding.vector.iter().enumerate() {
                    let expected = golden["truncation_vectors"][index][column]
                        .as_f64()
                        .unwrap() as f32;
                    assert!((actual - expected).abs() <= 1e-5 + 1e-4 * expected.abs());
                }
            }
            backend.stop().await.unwrap();
            assert!(!backend.is_ready());
        }
    }

    #[tokio::test]
    async fn repeated_load_reuses_only_the_matching_target_and_input_snapshot() {
        let (directory, mut request, mut target, mut decision) =
            crate::selected_embedding_execution::fixture(8);
        let mut backend = super::super::candle::CandleBackend::new();
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(false)
        );
        let original = backend.model.clone().unwrap();
        let baseline = backend.embeddings(vec!["hello".into()], "").await.unwrap();
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(true)
        );
        assert!(Arc::ptr_eq(&original, backend.model.as_ref().unwrap()));
        assert_eq!(
            backend.embeddings(vec!["hello".into()], "").await.unwrap()[0].vector,
            baseline[0].vector
        );

        // A known revision change is a distinct selected identity even when
        // the physical checkpoint and resulting vectors happen to be equal.
        for reference in [
            request.model_ref.as_mut().unwrap(),
            decision.selected_model_ref.as_mut().unwrap(),
            &mut request
                .resolved_model_package_facts
                .as_mut()
                .unwrap()
                .model_ref,
            &mut target.model_ref,
        ] {
            reference.revision = Some("new-known-fixture-revision".into());
        }
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(false)
        );
        assert!(!Arc::ptr_eq(&original, backend.model.as_ref().unwrap()));
        let revised = backend.model.clone().unwrap();
        target.content_fingerprint = Some("fixture-fingerprint-v2".into());
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(false)
        );
        assert!(!Arc::ptr_eq(&revised, backend.model.as_ref().unwrap()));

        let before = backend.model.clone().unwrap();
        let config_path = directory.path().join("config.json");
        let mut config: Value =
            serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
        config["layer_norm_eps"] = serde_json::json!(1e-10);
        std::fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(false)
        );
        assert!(!Arc::ptr_eq(&before, backend.model.as_ref().unwrap()));

        let before = backend.model.clone().unwrap();
        let weights = directory.path().join("model.safetensors");
        let replacement = directory.path().join("replacement.safetensors");
        std::fs::copy(&weights, &replacement).unwrap();
        std::fs::rename(replacement, weights).unwrap();
        assert_eq!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap()
                .runtime_reused,
            Some(false)
        );
        assert!(!Arc::ptr_eq(&before, backend.model.as_ref().unwrap()));
        backend.stop().await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn repeated_load_reuses_symlink_and_noncanonical_roots_without_accepting_escape() {
        let (directory, request, mut target, decision) =
            crate::selected_embedding_execution::fixture(8);
        let aliases = tempfile::tempdir().unwrap();
        let alias = aliases.path().join("selected-model");
        std::os::unix::fs::symlink(directory.path(), &alias).unwrap();
        for root in [alias, directory.path().join("2_Normalize/..")] {
            target.local_load_path = root.to_str().unwrap().into();
            let mut backend = super::super::candle::CandleBackend::new();
            assert_eq!(
                backend
                    .load_selected_embedding(&request, &target, &decision)
                    .await
                    .unwrap()
                    .runtime_reused,
                Some(false)
            );
            let original = backend.model.clone().unwrap();
            let baseline = backend.embeddings(vec!["hello".into()], "").await.unwrap();
            assert_eq!(
                backend
                    .load_selected_embedding(&request, &target, &decision)
                    .await
                    .unwrap()
                    .runtime_reused,
                Some(true)
            );
            assert!(Arc::ptr_eq(&original, backend.model.as_ref().unwrap()));
            assert_eq!(
                backend.embeddings(vec!["hello".into()], "").await.unwrap()[0].vector,
                baseline[0].vector
            );
            backend.stop().await.unwrap();
        }

        // Canonical containment still rejects an empty Normalize directory
        // redirected outside the selected root, preserving the resident model.
        target.local_load_path = directory.path().to_str().unwrap().into();
        let mut backend = super::super::candle::CandleBackend::new();
        backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .unwrap();
        let original = backend.model.clone().unwrap();
        let normalize = directory.path().join("2_Normalize");
        let outside = tempfile::tempdir().unwrap();
        std::fs::remove_dir(&normalize).unwrap();
        std::os::unix::fs::symlink(outside.path(), &normalize).unwrap();
        assert!(matches!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await,
            Err(BackendError::Config(_))
        ));
        assert!(Arc::ptr_eq(&original, backend.model.as_ref().unwrap()));
        assert_eq!(
            backend.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                .vector
                .len(),
            8
        );
        backend.stop().await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_real_load_retains_join_and_previous_model_until_replacement_or_stop() {
        for stop_backend in [false, true] {
            let (_original_directory, request, target, decision) =
                crate::selected_embedding_execution::fixture(8);
            let (_candidate_directory, candidate_request, candidate_target, candidate_decision) =
                crate::selected_embedding_execution::fixture(12);
            let backend = Arc::new(tokio::sync::Mutex::new(
                super::super::candle::CandleBackend::new(),
            ));
            let mut owner = backend.lock().await;
            owner
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            let original = owner.model.clone().unwrap();
            let (built_tx, built_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let built_tx = Mutex::new(Some(built_tx));
            let release_rx = Mutex::new(release_rx);
            owner.load_hook = Some(Arc::new(move || {
                built_tx.lock().unwrap().take().unwrap().send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
            }));
            drop(owner);
            let loading_backend = backend.clone();
            let loading = tokio::spawn(async move {
                loading_backend
                    .lock()
                    .await
                    .load_selected_embedding(
                        &candidate_request,
                        &candidate_target,
                        &candidate_decision,
                    )
                    .await
            });
            // This signal follows actual checkpoint parsing and BERT construction
            // on a blocking worker. The sole Tokio worker remains responsive.
            built_rx.await.unwrap();
            loading.abort();
            assert!(loading.await.unwrap_err().is_cancelled());
            assert!(Arc::ptr_eq(
                &original,
                backend.lock().await.model.as_ref().unwrap()
            ));
            let draining_backend = backend.clone();
            let (draining_tx, draining_rx) = tokio::sync::oneshot::channel();
            let draining = tokio::spawn(async move {
                let mut owner = draining_backend.lock().await;
                owner.load_hook = None;
                draining_tx.send(()).unwrap();
                if stop_backend {
                    owner.stop().await.map(|_| None)
                } else {
                    owner
                        .load_selected_embedding(&request, &target, &decision)
                        .await
                        .map(|outcome| outcome.runtime_reused)
                }
            });
            draining_rx.await.unwrap();
            assert!(
                !draining.is_finished(),
                "custody must wait for the real worker's exit"
            );
            release_tx.send(()).unwrap();
            let outcome = draining.await.unwrap().unwrap();
            let owner = backend.lock().await;
            if stop_backend {
                assert!(!owner.is_ready());
            } else {
                assert_eq!(outcome, Some(true));
                assert!(Arc::ptr_eq(&original, owner.model.as_ref().unwrap()));
                assert_eq!(
                    owner.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                        .vector
                        .len(),
                    8
                );
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn host_cancelled_real_load_preserves_the_previous_model() {
        use crate::{InferenceExecutionCancellationSignal, InferenceExecutionCancellationSnapshot};
        struct HostCancellation(AtomicBool);
        impl InferenceExecutionCancellationSignal for HostCancellation {
            fn snapshot(&self) -> InferenceExecutionCancellationSnapshot {
                if self.0.load(Ordering::Acquire) {
                    InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                        "host cancelled candidate load".into(),
                    ))
                } else {
                    InferenceExecutionCancellationSnapshot::running()
                }
            }
        }
        let (_original_directory, request, target, decision) =
            crate::selected_embedding_execution::fixture(8);
        let (_candidate_directory, candidate_request, candidate_target, candidate_decision) =
            crate::selected_embedding_execution::fixture(12);
        let backend = Arc::new(tokio::sync::Mutex::new(
            super::super::candle::CandleBackend::new(),
        ));
        let mut owner = backend.lock().await;
        owner
            .load_selected_embedding(&request, &target, &decision)
            .await
            .unwrap();
        let original = owner.model.clone().unwrap();
        let (built_tx, built_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let built_tx = Mutex::new(Some(built_tx));
        let release_rx = Mutex::new(release_rx);
        owner.load_hook = Some(Arc::new(move || {
            built_tx.lock().unwrap().take().unwrap().send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
        }));
        drop(owner);
        let signal = Arc::new(HostCancellation(AtomicBool::new(false)));
        let cancellation = InferenceExecutionCancellationHandle::with_signal(signal.clone());
        let loading_backend = backend.clone();
        let loading = tokio::spawn(async move {
            loading_backend
                .lock()
                .await
                .load_selected_embedding_with_cancellation(
                    &candidate_request,
                    &candidate_target,
                    &candidate_decision,
                    cancellation,
                )
                .await
        });
        built_rx.await.unwrap();
        signal.0.store(true, Ordering::Release);
        assert!(!loading.is_finished());
        release_tx.send(()).unwrap();
        assert!(matches!(
            loading.await.unwrap(),
            Err(BackendError::Cancelled(_))
        ));
        let mut owner = backend.lock().await;
        assert!(Arc::ptr_eq(&original, owner.model.as_ref().unwrap()));
        assert_eq!(
            owner.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                .vector
                .len(),
            8
        );
        owner.stop().await.unwrap();
    }

    #[tokio::test]
    async fn invalid_candidates_preserve_the_loaded_model() {
        let (directory, mut request, target, decision) =
            crate::selected_embedding_execution::fixture(8);
        let mut backend = super::super::candle::CandleBackend::new();
        backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .unwrap();
        let baseline = backend.embeddings(vec!["hello".into()], "").await.unwrap();
        for (file, mutation) in [
            ("config.json", ("model_type", serde_json::json!("roberta"))),
            ("config.json", ("torch_dtype", serde_json::json!("float16"))),
            ("config.json", ("is_decoder", serde_json::json!(true))),
            ("config.json", ("hidden_size", serde_json::json!(10))),
            (
                "1_Pooling/config.json",
                ("pooling_mode_cls_token", serde_json::json!(true)),
            ),
            (
                "sentence_bert_config.json",
                ("max_seq_length", serde_json::json!(32)),
            ),
            (
                "sentence_bert_config.json",
                ("do_lower_case", serde_json::Value::Null),
            ),
        ] {
            let path = directory.path().join(file);
            let original = std::fs::read(&path).unwrap();
            let mut json: Value = serde_json::from_slice(&original).unwrap();
            json[mutation.0] = mutation.1;
            std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
            assert!(matches!(
                backend
                    .load_selected_embedding(&request, &target, &decision)
                    .await,
                Err(BackendError::Config(_))
            ));
            assert!(backend.is_ready());
            assert_eq!(
                backend.embeddings(vec!["hello".into()], "").await.unwrap()[0].vector,
                baseline[0].vector
            );
            std::fs::write(path, original).unwrap();
        }
        request
            .resolved_model_package_facts
            .as_mut()
            .unwrap()
            .transformers
            .as_mut()
            .unwrap()
            .torch_dtype = Some("float16".into());
        assert!(backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .is_err());
        assert!(backend.is_ready());
    }

    #[tokio::test]
    async fn drain_observes_real_worker_exit_after_caller_loss() {
        let jobs: Arc<EmbeddingJobs> = Arc::new(EmbeddingJobs::default());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let exited = Arc::new(AtomicBool::new(false));
        let worker_exited = exited.clone();
        let completion = jobs
            .spawn(move |stop| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                assert!(stop.load(Ordering::Acquire));
                worker_exited.store(true, Ordering::Release);
                Err(BackendError::Cancelled("observed stop".into()))
            })
            .unwrap();
        started_rx.await.unwrap();
        drop(completion); // The backend still owns the worker join.
        assert!(jobs.spawn(|_| Ok(vec![])).is_err());
        let draining_jobs = jobs.clone();
        let drain = tokio::spawn(async move { draining_jobs.drain(true).await });
        // Synchronize on the actual stop signal, not timing of a mock inference result.
        loop {
            if jobs
                .0
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .stop
                .load(Ordering::Acquire)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(!drain.is_finished());
        assert!(!exited.load(Ordering::Acquire));
        release_tx.send(()).unwrap();
        drain.await.unwrap().unwrap();
        assert!(exited.load(Ordering::Acquire));
        assert!(jobs.0.lock().unwrap().is_none());
        jobs.spawn(|_| Ok(vec![]))
            .unwrap()
            .await
            .as_ref()
            .as_ref()
            .unwrap();
    }

    #[tokio::test]
    async fn config_failure_allows_stop_and_direct_reload_recovery() {
        for stop_first in [true, false] {
            let (_directory, request, target, decision) =
                crate::selected_embedding_execution::fixture(8);
            let mut backend = super::super::candle::CandleBackend::new();
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            assert!(matches!(
                backend.embeddings(vec![" ".into()], "").await,
                Err(BackendError::Config(_))
            ));
            assert!(backend.is_ready());
            if stop_first {
                backend.stop().await.unwrap();
                assert!(!backend.is_ready());
                backend.stop().await.unwrap();
            }
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            assert_eq!(
                backend.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                    .vector
                    .len(),
                8
            );
            backend.stop().await.unwrap();
        }
    }

    #[tokio::test]
    async fn inference_failure_allows_stop_and_direct_reload_recovery() {
        for stop_first in [true, false] {
            let (_directory, request, target, decision) =
                crate::selected_embedding_execution::fixture(8);
            let mut backend = super::super::candle::CandleBackend::new();
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            // Controlled tokenizer fault creates a genuine inconsistent-shape
            // execution error, without replacing inference with a mock result.
            Arc::get_mut(backend.model.as_mut().unwrap())
                .unwrap()
                .tokenizer
                .with_padding(None);
            assert!(matches!(
                backend
                    .embeddings(vec!["hello world".into(), "hello".into()], "")
                    .await,
                Err(BackendError::Inference(_))
            ));
            assert!(backend.is_ready());
            if stop_first {
                backend.stop().await.unwrap();
                assert!(!backend.is_ready());
                backend.stop().await.unwrap();
            }
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .unwrap();
            assert_eq!(
                backend.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                    .vector
                    .len(),
                8
            );
            backend.stop().await.unwrap();
        }
    }

    #[tokio::test]
    async fn request_failure_remains_observable_after_lifecycle_retirement() {
        let jobs: EmbeddingJobs = EmbeddingJobs::default();
        let request_completion = jobs
            .spawn(|_| Err(BackendError::Inference("controlled failure".into())))
            .unwrap();
        jobs.drain(false).await.unwrap();
        assert!(jobs.0.lock().unwrap().is_none());
        assert!(matches!(
            copy_result(request_completion.await.as_ref()),
            Err(BackendError::Inference(_))
        ));
        jobs.drain(true).await.unwrap();
    }

    #[tokio::test]
    async fn actual_forward_observes_cancellation_at_native_stage_boundaries() {
        use crate::{InferenceExecutionCancellationSignal, InferenceExecutionCancellationSnapshot};
        use std::sync::atomic::AtomicUsize;
        struct CancelAt {
            snapshots: AtomicUsize,
            at: usize,
        }
        impl InferenceExecutionCancellationSignal for CancelAt {
            fn snapshot(&self) -> InferenceExecutionCancellationSnapshot {
                if self.snapshots.fetch_add(1, Ordering::AcqRel) + 1 >= self.at {
                    InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                        "stage boundary".into(),
                    ))
                } else {
                    InferenceExecutionCancellationSnapshot::running()
                }
            }
        }
        let (_directory, request, target, decision) =
            crate::selected_embedding_execution::fixture(8);
        let mut backend = super::super::candle::CandleBackend::new();
        backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .unwrap();
        for at in [1, 3] {
            let signal = Arc::new(CancelAt {
                snapshots: AtomicUsize::new(0),
                at,
            });
            let cancellation = InferenceExecutionCancellationHandle::with_signal(signal.clone());
            assert!(matches!(
                backend
                    .selected_embeddings(vec!["hello".into()], cancellation)
                    .await,
                Err(BackendError::Cancelled(_))
            ));
            assert_eq!(signal.snapshots.load(Ordering::Acquire), at);
            backend.finish_selected_embedding(true).await.unwrap();
            assert!(backend.is_ready());
        }
        // The same loaded native model remains executable after cancelled jobs are drained.
        assert_eq!(
            backend.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                .vector
                .len(),
            8
        );
        backend.stop().await.unwrap();
    }

    #[tokio::test]
    async fn missing_recipe_wrong_weights_and_non_f32_checkpoint_fail_before_ready() {
        let (directory, request, target, decision) =
            crate::selected_embedding_execution::fixture(8);
        let modules_path = directory.path().join("modules.json");
        let original_modules = std::fs::read(&modules_path).unwrap();
        std::fs::remove_file(&modules_path).unwrap();
        let mut backend = super::super::candle::CandleBackend::new();
        assert!(matches!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await,
            Err(BackendError::Config(_))
        ));
        assert!(!backend.is_ready());
        let mut modules: Value = serde_json::from_slice(&original_modules).unwrap();
        modules[2]["type"] = serde_json::json!("sentence_transformers.models.Dense");
        std::fs::write(&modules_path, serde_json::to_vec(&modules).unwrap()).unwrap();
        assert!(backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .is_err());
        std::fs::write(modules_path, original_modules).unwrap();
        let weights = directory.path().join("model.safetensors");
        let original_weights = std::fs::read(&weights).unwrap();
        let (other, _, _, _) = crate::selected_embedding_execution::fixture(12);
        std::fs::copy(other.path().join("model.safetensors"), &weights).unwrap();
        assert!(matches!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await,
            Err(BackendError::Config(_))
        ));
        std::fs::write(&weights, original_weights).unwrap();
        let tensors = candle_core::safetensors::load(&weights, &candle_core::Device::Cpu).unwrap();
        let tensors = tensors
            .into_iter()
            .map(|(name, tensor)| (name, tensor.to_dtype(DType::F16).unwrap()))
            .collect::<std::collections::HashMap<_, _>>();
        candle_core::safetensors::save(&tensors, &weights).unwrap();
        assert!(matches!(
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await,
            Err(BackendError::Config(_))
        ));
        assert!(!backend.is_ready());
    }
}

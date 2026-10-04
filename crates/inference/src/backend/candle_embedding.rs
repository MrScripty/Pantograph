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
    _target: PumasArtifactLoadTarget,
    _plan: CandleEmbeddingLoadPlan,
    _inputs: Vec<(String, Vec<u8>)>,
}

pub(super) struct EmbeddingModel {
    bert: BertModel,
    tokenizer: tokenizers::Tokenizer,
    width: usize,
    _identity: LoadIdentity,
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
    pub(super) fn load(
        plan: CandleEmbeddingLoadPlan,
        target: PumasArtifactLoadTarget,
    ) -> Result<Self, BackendError> {
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
        // This exact Pooling class includes all attention-masked tokens in v2.0.0;
        // v2.6.0 loads omitted include_prompt as true. Pinned source provenance:
        // tests/fixtures/candle_bert/README.md. Explicit non-true values stay rejected.
        if pooling["word_embedding_dimension"].as_u64() != Some(config.hidden_size as u64)
            || pooling["pooling_mode_mean_tokens"] != true
            || pooling
                .get("include_prompt")
                .is_some_and(|value| value != true)
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
        let tensors =
            candle_core::safetensors::load(weights, &candle_core::Device::Cpu).map_err(invalid)?;
        if tensors.is_empty() || tensors.values().any(|tensor| tensor.dtype() != DType::F32) {
            return Err(invalid("all checkpoint tensors must be F32"));
        }
        let vb =
            candle_nn::VarBuilder::from_tensors(tensors, DType::F32, &candle_core::Device::Cpu);
        let bert = BertModel::load(vb, &config).map_err(invalid)?;
        Ok(Self {
            bert,
            tokenizer,
            width: config.hidden_size,
            _identity: LoadIdentity {
                _target: target,
                _plan: plan,
                _inputs: inputs,
            },
        })
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

type Completion = Shared<
    futures_util::future::BoxFuture<'static, Arc<Result<Vec<EmbeddingResult>, BackendError>>>,
>;
#[derive(Clone)]
struct Job {
    stop: Arc<AtomicBool>,
    completion: Completion,
}
#[derive(Default)]
pub(super) struct EmbeddingJobs(Mutex<Option<Job>>);

fn copy_result(
    result: &Result<Vec<EmbeddingResult>, BackendError>,
) -> Result<Vec<EmbeddingResult>, BackendError> {
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
    fn spawn(
        &self,
        run: impl FnOnce(Arc<AtomicBool>) -> Result<Vec<EmbeddingResult>, BackendError> + Send + 'static,
    ) -> Result<Completion, BackendError> {
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
        *slot = Some(Job {
            stop,
            completion: completion.clone(),
        });
        Ok(completion)
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
                .embeddings(texts.clone(), "not-a-model-path")
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
                let single = backend
                    .embeddings(vec![text], "ignored-legacy-name")
                    .await
                    .unwrap();
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
    async fn include_prompt_default_matches_true_and_rejects_false_without_replacement() {
        for width in [8, 12] {
            let (directory, request, target, decision) =
                crate::selected_embedding_execution::fixture(width);
            let path = directory.path().join("1_Pooling/config.json");
            let mut pooling: Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(pooling["include_prompt"], true);
            let golden: Value = serde_json::from_slice(
                &std::fs::read(directory.path().join("golden.json")).unwrap(),
            )
            .unwrap();
            let texts: Vec<String> = serde_json::from_value(golden["texts"].clone()).unwrap();
            let mut backend = super::super::candle::CandleBackend::new();
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .expect("explicit include_prompt=true should load");
            let explicit = backend.embeddings(texts.clone(), "").await.unwrap();

            pooling.as_object_mut().unwrap().remove("include_prompt");
            std::fs::write(&path, serde_json::to_vec(&pooling).unwrap()).unwrap();
            backend
                .load_selected_embedding(&request, &target, &decision)
                .await
                .expect("omitted include_prompt should use the defined inclusion default");
            let defaulted = backend.embeddings(texts.clone(), "").await.unwrap();
            assert_eq!(defaulted.len(), explicit.len());
            let mut max_error = 0.0_f32;
            for (index, (actual, expected)) in defaulted.iter().zip(&explicit).enumerate() {
                assert_eq!(actual.vector, expected.vector);
                assert_eq!(actual.vector.len(), width);
                assert_eq!(actual.token_count, expected.token_count);
                assert_eq!(actual.token_count, if index == 1 { 3 } else { 4 });
                let norm = actual
                    .vector
                    .iter()
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt();
                assert!((norm - 1.0).abs() < 1e-5);
                for (column, value) in actual.vector.iter().enumerate() {
                    assert!(value.is_finite());
                    let reference = golden["batch_vectors"][index][column].as_f64().unwrap() as f32;
                    max_error = max_error.max((value - reference).abs());
                    assert!((value - reference).abs() <= 1e-5 + 1e-4 * reference.abs());
                }
            }
            println!("actual Candle BERT width={width}: absent=true, max_abs_reference_error={max_error:e}");

            for rejected in [
                serde_json::json!(false),
                Value::Null,
                serde_json::json!("true"),
                serde_json::json!(1),
                serde_json::json!({}),
            ] {
                pooling["include_prompt"] = rejected.clone();
                std::fs::write(&path, serde_json::to_vec(&pooling).unwrap()).unwrap();
                assert!(
                    matches!(
                        backend
                            .load_selected_embedding(&request, &target, &decision)
                            .await,
                        Err(BackendError::Config(_))
                    ),
                    "unsupported include_prompt={rejected} must be rejected"
                );
                assert!(backend.is_ready());
                let retained = backend.embeddings(texts.clone(), "").await.unwrap();
                for (actual, expected) in retained.iter().zip(&explicit) {
                    assert_eq!(actual.vector, expected.vector);
                    assert_eq!(actual.token_count, expected.token_count);
                }
            }
            backend.stop().await.unwrap();
        }
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
        let jobs = Arc::new(EmbeddingJobs::default());
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
        let jobs = EmbeddingJobs::default();
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

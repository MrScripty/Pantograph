//! Candle backend implementation with embedded Axum HTTP server
//!
//! This backend provides in-process inference using the Candle ML framework,
//! exposed via an OpenAI-compatible HTTP API using Axum.
//!
//! **Architecture:**
//! - CandleEngine: Model loading and inference logic
//! - Axum HTTP Server: OpenAI-compatible endpoints (/health, /v1/models, /v1/embeddings)
//! - CandleBackend: InferenceBackend trait implementation
//!
//! **Supported model architectures:**
//! - BERT-based encoders (BGE, GTE, etc.) - mean pooling
//! - Qwen3 decoders (Qwen3-Embedding) - last-token extraction + L2 norm
//!
//! **Important limitations:**
//! - CUDA-only (no Vulkan/Metal support)
//! - Uses SafeTensors format (not GGUF)
//! - Models must be downloaded manually from HuggingFace
//! - Higher memory usage (full precision models)

use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use super::{
    BackendCapabilities, BackendConfig, BackendError, ChatChunk, EmbeddingResult,
    InferenceBackend,
};

// Candle imports - these are behind the feature flag
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig, DTYPE as BERT_DTYPE};
use candle_transformers::models::qwen3::{Config as Qwen3Config, Model as Qwen3Model};
use tokenizers::Tokenizer;

// ============================================================================
// CandleEngine - Model loading and inference
// ============================================================================

/// Model type enum to handle different architectures
enum EmbeddingModel {
    /// BERT-based encoder models (BGE, GTE, etc.)
    Bert(BertModel),
    /// Qwen3 decoder models (Qwen3-Embedding)
    Qwen3(Qwen3Model),
}

/// The inference engine supporting multiple model architectures
struct CandleEngine {
    device: Device,
    model: EmbeddingModel,
    tokenizer: Tokenizer,
    model_id: String,
}

impl CandleEngine {
    /// Load model from a LOCAL SafeTensors directory (no auto-download)
    ///
    /// Auto-detects the model architecture from config.json and loads accordingly.
    /// Supports BERT-based encoders and Qwen3 decoders.
    ///
    /// The directory must contain:
    /// - config.json
    /// - tokenizer.json
    /// - model.safetensors
    fn load_from_path(model_dir: &Path, device: &Device) -> Result<Self, BackendError> {
        let config_path = model_dir.join("config.json");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");

        // Validate files exist with helpful error messages
        if !config_path.exists() {
            return Err(BackendError::Config(format!(
                "config.json not found in {}. Download the model manually from HuggingFace \
                 (e.g., git clone https://huggingface.co/BAAI/bge-small-en-v1.5).",
                model_dir.display()
            )));
        }
        if !tokenizer_path.exists() {
            return Err(BackendError::Config(format!(
                "tokenizer.json not found in {}. Ensure you downloaded the complete model.",
                model_dir.display()
            )));
        }
        if !weights_path.exists() {
            return Err(BackendError::Config(format!(
                "model.safetensors not found in {}. Ensure you downloaded the complete model \
                 (use git lfs pull if needed).",
                model_dir.display()
            )));
        }

        log::info!(
            "Loading Candle embedding model from: {}",
            model_dir.display()
        );

        // Load config as raw JSON first to detect architecture
        let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
            BackendError::StartupFailed(format!("Failed to read config: {}", e))
        })?;

        let config_json: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
            BackendError::StartupFailed(format!("Failed to parse config JSON: {}", e))
        })?;

        // Detect architecture from config.json
        let architectures = config_json
            .get("architectures")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        log::info!("Detected architectures: {:?}", architectures);

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            BackendError::StartupFailed(format!("Failed to load tokenizer: {}", e))
        })?;

        // Load model based on architecture
        let model = if architectures
            .iter()
            .any(|a| a.contains("Qwen3") || a.contains("Qwen2"))
        {
            // Load as Qwen3 decoder model
            log::info!("Detected Qwen architecture, loading decoder model...");

            let config: Qwen3Config = serde_json::from_str(&config_str).map_err(|e| {
                BackendError::StartupFailed(format!("Failed to parse Qwen3 config: {}", e))
            })?;

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F16, device).map_err(
                    |e| BackendError::StartupFailed(format!("Failed to load model weights: {}", e)),
                )?
            };

            // Candle's Qwen3 model expects tensors with "model." prefix (e.g., "model.embed_tokens.weight")
            // but some HuggingFace exports (especially sentence-transformers) have no prefix.
            // Detect which format we have and add a renamer if needed.
            let has_model_prefix = vb.contains_tensor("model.embed_tokens.weight");
            let vb = if has_model_prefix {
                log::info!("Model has 'model.' prefix, using tensors directly");
                vb
            } else {
                log::info!("Model lacks 'model.' prefix, adding prefix mapper");
                // Rename "model.X" -> "X" (strip the prefix that Candle adds)
                vb.rename_f(|name: &str| {
                    if let Some(stripped) = name.strip_prefix("model.") {
                        stripped.to_string()
                    } else {
                        name.to_string()
                    }
                })
            };

            let model = Qwen3Model::new(&config, vb).map_err(|e| {
                BackendError::StartupFailed(format!("Failed to create Qwen3 model: {}", e))
            })?;

            EmbeddingModel::Qwen3(model)
        } else {
            // Default to BERT encoder model
            log::info!("Loading BERT/encoder architecture...");

            let config: BertConfig = serde_json::from_str(&config_str).map_err(|e| {
                BackendError::StartupFailed(format!("Failed to parse BERT config: {}", e))
            })?;

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[&weights_path], BERT_DTYPE, device).map_err(
                    |e| BackendError::StartupFailed(format!("Failed to load model weights: {}", e)),
                )?
            };

            let model = BertModel::load(vb, &config).map_err(|e| {
                BackendError::StartupFailed(format!("Failed to create BERT model: {}", e))
            })?;

            EmbeddingModel::Bert(model)
        };

        // Extract model name from directory
        let model_id = model_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        log::info!("Successfully loaded Candle embedding model: {}", model_id);

        Ok(Self {
            device: device.clone(),
            model,
            tokenizer,
            model_id,
        })
    }

    /// Generate embeddings for a batch of texts
    /// Uses different strategies based on model architecture:
    /// - BERT: mean pooling over all tokens
    /// - Qwen3: last-token extraction + L2 normalization
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, BackendError> {
        let mut all_embeddings = Vec::new();

        for text in texts {
            // Tokenize
            let encoding = self
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| BackendError::Inference(format!("Tokenization failed: {}", e)))?;

            let token_ids = encoding.get_ids();

            let embedding_vec = match &mut self.model {
                EmbeddingModel::Bert(model) => {
                    // BERT: mean pooling over sequence
                    let attention_mask = encoding.get_attention_mask();

                    let token_ids_tensor = Tensor::new(token_ids, &self.device)
                        .map_err(|e| {
                            BackendError::Inference(format!("Tensor creation failed: {}", e))
                        })?
                        .unsqueeze(0)
                        .map_err(|e| BackendError::Inference(format!("Unsqueeze failed: {}", e)))?;

                    let attention_mask_tensor = Tensor::new(attention_mask, &self.device)
                        .map_err(|e| {
                            BackendError::Inference(format!("Tensor creation failed: {}", e))
                        })?
                        .unsqueeze(0)
                        .map_err(|e| BackendError::Inference(format!("Unsqueeze failed: {}", e)))?;

                    let token_type_ids = token_ids_tensor.zeros_like().map_err(|e| {
                        BackendError::Inference(format!("Zeros like failed: {}", e))
                    })?;

                    // Forward pass
                    let embeddings = model
                        .forward(&token_ids_tensor, &token_type_ids, Some(&attention_mask_tensor))
                        .map_err(|e| {
                            BackendError::Inference(format!("Forward pass failed: {}", e))
                        })?;

                    // Mean pooling over sequence length (dim 1)
                    let (_batch, seq_len, _hidden) = embeddings
                        .dims3()
                        .map_err(|e| BackendError::Inference(format!("Dims failed: {}", e)))?;

                    let sum = embeddings
                        .sum(1)
                        .map_err(|e| BackendError::Inference(format!("Sum failed: {}", e)))?;
                    let pooled = (sum / (seq_len as f64))
                        .map_err(|e| BackendError::Inference(format!("Division failed: {}", e)))?;

                    // Convert to Vec<f32>
                    pooled
                        .squeeze(0)
                        .map_err(|e| BackendError::Inference(format!("Squeeze failed: {}", e)))?
                        .to_vec1()
                        .map_err(|e| BackendError::Inference(format!("To vec failed: {}", e)))?
                }
                EmbeddingModel::Qwen3(model) => {
                    // NOTE: KV cache accumulates across forward() calls. For large documents,
                    // ensure texts are chunked appropriately before embedding to avoid OOM.
                    // The clear_kv_cache() method exists but is private in candle-transformers.

                    // Qwen3: last-token extraction + L2 normalization
                    let tokens = Tensor::new(token_ids, &self.device)
                        .map_err(|e| {
                            BackendError::Inference(format!("Tensor creation failed: {}", e))
                        })?
                        .unsqueeze(0)
                        .map_err(|e| BackendError::Inference(format!("Unsqueeze failed: {}", e)))?;

                    // Forward pass through decoder (offset=0 for fresh inference)
                    let hidden_states = model.forward(&tokens, 0).map_err(|e| {
                        BackendError::Inference(format!("Forward pass failed: {}", e))
                    })?;

                    // Extract last token's hidden state
                    let (_, seq_len, _) = hidden_states
                        .dims3()
                        .map_err(|e| BackendError::Inference(format!("Dims failed: {}", e)))?;

                    let last_hidden = hidden_states
                        .narrow(1, seq_len - 1, 1)
                        .map_err(|e| BackendError::Inference(format!("Narrow failed: {}", e)))?
                        .squeeze(1)
                        .map_err(|e| BackendError::Inference(format!("Squeeze failed: {}", e)))?;

                    // L2 normalization
                    let norm = last_hidden
                        .sqr()
                        .map_err(|e| BackendError::Inference(format!("Sqr failed: {}", e)))?
                        .sum_keepdim(1)
                        .map_err(|e| {
                            BackendError::Inference(format!("Sum keepdim failed: {}", e))
                        })?
                        .sqrt()
                        .map_err(|e| BackendError::Inference(format!("Sqrt failed: {}", e)))?;

                    let normalized = last_hidden.broadcast_div(&norm).map_err(|e| {
                        BackendError::Inference(format!("Broadcast div failed: {}", e))
                    })?;

                    // Convert to Vec<f32>
                    normalized
                        .to_dtype(DType::F32)
                        .map_err(|e| BackendError::Inference(format!("To dtype failed: {}", e)))?
                        .squeeze(0)
                        .map_err(|e| {
                            BackendError::Inference(format!("Final squeeze failed: {}", e))
                        })?
                        .to_vec1()
                        .map_err(|e| BackendError::Inference(format!("To vec failed: {}", e)))?
                }
            };

            all_embeddings.push(embedding_vec);
        }

        Ok(all_embeddings)
    }
}

// ============================================================================
// Axum HTTP Server
// ============================================================================

/// Shared state for Axum handlers
type ServerState = Arc<RwLock<CandleEngine>>;

/// Start the Axum HTTP server on a free port
async fn start_server(
    engine: Arc<RwLock<CandleEngine>>,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>), BackendError> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/embeddings", post(embeddings_handler))
        .layer(cors)
        .with_state(engine);

    // Bind to a free port on localhost
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| BackendError::StartupFailed(format!("Failed to bind TCP listener: {}", e)))?;

    let addr = listener
        .local_addr()
        .map_err(|e| BackendError::StartupFailed(format!("Failed to get local address: {}", e)))?;

    log::info!("Candle HTTP server listening on http://{}", addr);

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("Candle HTTP server error: {}", e);
        }
    });

    Ok((addr, handle))
}

// --- Axum Handlers ---

async fn health_handler() -> &'static str {
    "ok"
}

async fn models_handler(State(engine): State<ServerState>) -> Json<serde_json::Value> {
    let guard = engine.read().await;
    Json(serde_json::json!({
        "object": "list",
        "data": [{
            "id": guard.model_id,
            "object": "model",
            "owned_by": "candle"
        }]
    }))
}

#[derive(Debug, Deserialize)]
struct EmbeddingRequest {
    input: EmbeddingInput,
    #[allow(dead_code)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

impl EmbeddingInput {
    fn into_vec(self) -> Vec<String> {
        match self {
            EmbeddingInput::Single(s) => vec![s],
            EmbeddingInput::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingResponse {
    object: &'static str,
    data: Vec<EmbeddingData>,
    model: String,
    usage: Usage,
}

#[derive(Debug, Serialize)]
struct EmbeddingData {
    object: &'static str,
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Serialize)]
struct Usage {
    prompt_tokens: usize,
    total_tokens: usize,
}

async fn embeddings_handler(
    State(engine): State<ServerState>,
    Json(req): Json<EmbeddingRequest>,
) -> Result<Json<EmbeddingResponse>, (StatusCode, String)> {
    let texts = req.input.into_vec();
    // Use write lock because Qwen3's forward() takes &mut self for KV cache
    let mut guard = engine.write().await;

    let vectors = guard.embed(&texts).map_err(|e| {
        log::error!("Embedding error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let data: Vec<EmbeddingData> = vectors
        .into_iter()
        .enumerate()
        .map(|(i, embedding)| EmbeddingData {
            object: "embedding",
            embedding,
            index: i,
        })
        .collect();

    Ok(Json(EmbeddingResponse {
        object: "list",
        data,
        model: guard.model_id.clone(),
        usage: Usage {
            prompt_tokens: 0,
            total_tokens: 0,
        },
    }))
}

// ============================================================================
// CandleBackend - InferenceBackend implementation
// ============================================================================

/// Candle backend for in-process inference with HTTP API
///
/// This backend uses Candle's pure-Rust ML framework for inference,
/// exposed via an embedded Axum HTTP server with OpenAI-compatible endpoints.
pub struct CandleBackend {
    /// The inference engine (None until started)
    engine: Option<Arc<RwLock<CandleEngine>>>,
    /// HTTP server address (None until started)
    server_addr: Option<SocketAddr>,
    /// HTTP server task handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Whether the backend is ready
    ready: bool,
}

impl CandleBackend {
    /// Create a new Candle backend
    pub fn new() -> Self {
        Self {
            engine: None,
            server_addr: None,
            server_handle: None,
            ready: false,
        }
    }

    /// Check if CUDA is available for this backend
    ///
    /// Returns (available, reason_if_not) tuple.
    /// This is called by the registry to populate BackendInfo.available.
    pub fn check_availability() -> (bool, Option<String>) {
        // Try to create a CUDA device (device 0)
        match Device::new_cuda(0) {
            Ok(_) => (true, None),
            Err(e) => {
                let err_str = e.to_string();
                let reason = if err_str.contains("not been built with cuda") {
                    "Candle was not compiled with CUDA support".to_string()
                } else {
                    format!("CUDA not available: {}", err_str)
                };
                (false, Some(reason))
            }
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: false,          // Vision requires separate model loading - not implemented yet
            embeddings: true,       // BERT/BGE embedding models supported
            gpu: true,              // CUDA required
            device_selection: true, // Supports CUDA device selection (e.g., "CUDA0", "CUDA1")
            streaming: true,        // Token streaming supported (for future chat models)
            tool_calling: false,    // Not implemented
        }
    }

    /// Get CUDA device from device string
    ///
    /// Parses device strings like "CUDA0", "CUDA1", or "auto" and returns the appropriate Device.
    /// For hybrid GPU systems (Intel iGPU + NVIDIA dGPU), this allows selecting the correct GPU.
    fn get_cuda_device(device_str: Option<&str>) -> Result<Device, BackendError> {
        let device_idx = match device_str {
            Some(s) if s.starts_with("CUDA") => {
                // Parse "CUDA0", "CUDA1", etc.
                s.strip_prefix("CUDA")
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0)
            }
            Some("auto") | None => 0, // Default to first CUDA device
            Some(other) => {
                log::warn!(
                    "Unknown device '{}' for Candle backend, defaulting to CUDA0",
                    other
                );
                0
            }
        };

        log::info!("Candle: Requesting CUDA device {}", device_idx);

        Device::new_cuda(device_idx).map_err(|e| {
            BackendError::StartupFailed(format!(
                "CUDA device {} not available: {}. \
                 Install CUDA toolkit and ensure the specified GPU is available.",
                device_idx, e
            ))
        })
    }
}

impl Default for CandleBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for CandleBackend {
    fn name(&self) -> &'static str {
        "Candle"
    }

    fn description(&self) -> &'static str {
        "In-process Candle inference (CUDA required). Uses local SafeTensors models."
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(&mut self, config: &BackendConfig, _app: &AppHandle) -> Result<(), BackendError> {
        // 1. Get CUDA device from config (supports device selection like "CUDA0", "CUDA1")
        let device = Self::get_cuda_device(config.device.as_deref())?;
        log::info!("Candle: CUDA device initialized successfully");

        // 2. Get model path from config (NOT model_id for HuggingFace download)
        let model_path = config.model_path.as_ref().ok_or_else(|| {
            BackendError::Config(
                "model_path required for Candle (path to directory with SafeTensors model). \
                 Download a model from HuggingFace (e.g., BAAI/bge-small-en-v1.5) and set the path."
                    .to_string(),
            )
        })?;

        // 3. Load the embedding model from local path (blocking operation)
        let model_path_clone = model_path.clone();
        let device_clone = device.clone();
        let engine = tokio::task::spawn_blocking(move || {
            CandleEngine::load_from_path(&model_path_clone, &device_clone)
        })
        .await
        .map_err(|e| BackendError::StartupFailed(format!("Task join error: {}", e)))??;

        let engine = Arc::new(RwLock::new(engine));

        // 4. Start HTTP server
        let (addr, handle) = start_server(engine.clone()).await?;

        // 5. Store state
        self.engine = Some(engine);
        self.server_addr = Some(addr);
        self.server_handle = Some(handle);
        self.ready = true;

        log::info!(
            "Candle backend started with model from: {}",
            model_path.display()
        );
        Ok(())
    }

    fn stop(&mut self) {
        // Abort the HTTP server task
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            log::info!("Candle HTTP server stopped");
        }

        self.engine = None;
        self.server_addr = None;
        self.ready = false;
        log::info!("Candle backend stopped");
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        // Check if model is loaded and server is running
        self.ready && self.engine.is_some() && self.server_addr.is_some()
    }

    fn base_url(&self) -> Option<String> {
        // Return the HTTP server URL (now we have one!)
        self.server_addr.map(|addr| format!("http://{}", addr))
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        // Chat completion requires a generative model (LLaVA, etc.)
        // Currently only embedding models are supported
        Err(BackendError::Inference(
            "Chat completion not yet implemented for Candle backend. \
             Currently only embedding models are supported."
                .to_string(),
        ))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        let engine = self.engine.as_ref().ok_or(BackendError::NotReady)?;
        // Use write lock because Qwen3's forward() takes &mut self
        let mut guard = engine.write().await;

        let vectors = guard.embed(&texts)?;

        let embeddings = vectors
            .into_iter()
            .map(|vector| EmbeddingResult {
                vector,
                token_count: 0,
            })
            .collect();

        Ok(embeddings)
    }
}

// CandleBackend is Send + Sync because engine is protected by Arc<RwLock>
unsafe impl Send for CandleBackend {}
unsafe impl Sync for CandleBackend {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_name() {
        let backend = CandleBackend::new();
        assert_eq!(backend.name(), "Candle");
    }

    #[test]
    fn test_capabilities() {
        let caps = CandleBackend::static_capabilities();
        assert!(!caps.vision); // Vision not implemented yet
        assert!(caps.embeddings);
        assert!(caps.gpu);
        assert!(caps.device_selection); // Supports CUDA device selection (e.g., "CUDA0", "CUDA1")
        assert!(caps.streaming);
        assert!(!caps.tool_calling);
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = CandleBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none()); // No URL until started
    }

    #[test]
    fn test_description_updated() {
        let backend = CandleBackend::new();
        assert!(backend.description().contains("local SafeTensors"));
    }
}

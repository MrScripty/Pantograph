//! Application configuration storage
//!
//! Handles persistent storage of model paths and connection settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use crate::constants::defaults;

/// Model configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    /// Path to the VLM model file (e.g., Qwen3-VL-4B)
    pub vlm_model_path: Option<String>,
    /// Path to the mmproj file for vision models
    pub vlm_mmproj_path: Option<String>,
    /// Path to the embedding model file (GGUF format for llama.cpp, e.g., Qwen3-Embedding-0.6B)
    pub embedding_model_path: Option<String>,
    /// Path to the Candle embedding model directory (SafeTensors format, e.g., bge-small-en-v1.5/)
    /// This is separate from embedding_model_path because Candle uses a different model format.
    pub candle_embedding_model_path: Option<String>,
    /// Ollama model name for VLM inference (e.g., "llava:13b", "qwen2-vl:7b")
    /// Used when Ollama is the selected backend instead of file paths.
    pub ollama_vlm_model: Option<String>,
}

/// Device configuration for inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device identifier (e.g., "Vulkan0", "Vulkan1", "none" for CPU-only)
    pub device: String,
    /// Number of layers to offload to GPU (-1 = all layers)
    pub gpu_layers: i32,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            device: defaults::DEVICE.to_string(),
            gpu_layers: defaults::GPU_LAYERS,
        }
    }
}

/// Information about an available compute device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device identifier used with --device flag (e.g., "Vulkan0", "none")
    pub id: String,
    /// Human-readable device name (e.g., "NVIDIA GeForce RTX 4060 Laptop GPU")
    pub name: String,
    /// Total VRAM in MB (0 for CPU)
    pub total_vram_mb: u64,
    /// Free VRAM in MB (0 for CPU)
    pub free_vram_mb: u64,
}

/// Connection mode preference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ConnectionMode {
    /// No connection configured
    None,
    /// Connect to external server (remote API or local server like LM Studio)
    External { url: String },
    /// Use built-in llama.cpp sidecar
    Sidecar,
}

impl Default for ConnectionMode {
    fn default() -> Self {
        ConnectionMode::None
    }
}

/// Memory management mode for embedding model
/// Controls how the embedding model is loaded relative to the main LLM
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingMemoryMode {
    /// Embedding model runs on CPU (RAM), LLM on GPU (VRAM)
    /// Best for machines with limited VRAM but plenty of RAM
    /// This is the recommended default for most users
    #[default]
    CpuParallel,
    /// Both models run on GPU (VRAM) simultaneously
    /// Requires ~800MB+ additional VRAM for embedding model
    /// Fastest option but needs sufficient VRAM
    GpuParallel,
    /// Only one model in memory at a time, swap as needed
    /// Lowest memory usage but adds ~2-5s latency per search
    /// Best for very limited memory systems
    Sequential,
}

/// Import validation mode for generated Svelte components
/// Controls how imports are validated before the component is loaded
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportValidationMode {
    /// No import validation (current behavior, fastest)
    /// Errors only surface when Vite tries to bundle at runtime
    #[default]
    None,
    /// Parse imports and check against package.json dependencies
    /// Fast and catches most errors (typos in package names)
    ImportResolve,
    /// Use Vite's module resolution to validate imports
    /// Most accurate but requires Vite dev server
    ViteIntegration,
    /// Use esbuild to attempt bundling the script block
    /// Catches all bundler errors but slowest option
    EsbuildBundle,
}

/// Sandbox configuration for component validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// How to validate imports in generated components
    #[serde(default)]
    pub import_validation_mode: ImportValidationMode,
    /// Timeout for validation scripts in milliseconds
    #[serde(default = "default_validation_timeout")]
    pub validation_timeout_ms: u64,
    /// Additional packages to allow beyond package.json dependencies
    #[serde(default)]
    pub allowed_packages: Vec<String>,
}

fn default_validation_timeout() -> u64 {
    5000
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            import_validation_mode: ImportValidationMode::default(),
            validation_timeout_ms: default_validation_timeout(),
            allowed_packages: Vec::new(),
        }
    }
}

/// Full application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Model paths for sidecar mode
    pub models: ModelConfig,
    /// Device configuration for inference
    #[serde(default)]
    pub device: DeviceConfig,
    /// Last used connection mode
    pub connection_mode: ConnectionMode,
    /// External server URL (if using external mode)
    pub external_url: Option<String>,
    /// API key for external providers (OpenAI, Anthropic, etc.)
    pub api_key: Option<String>,
    /// Memory management mode for embedding model
    #[serde(default)]
    pub embedding_memory_mode: EmbeddingMemoryMode,
    /// Sandbox configuration for component validation
    #[serde(default)]
    pub sandbox: SandboxConfig,
}

impl AppConfig {
    /// Load configuration from disk
    pub async fn load(app_data_dir: &PathBuf) -> Result<Self, ConfigError> {
        let config_path = app_data_dir.join("config.json");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path).await
            .map_err(ConfigError::Io)?;

        serde_json::from_str(&contents)
            .map_err(ConfigError::Parse)
    }

    /// Save configuration to disk
    pub async fn save(&self, app_data_dir: &PathBuf) -> Result<(), ConfigError> {
        // Ensure directory exists
        fs::create_dir_all(app_data_dir).await
            .map_err(ConfigError::Io)?;

        let config_path = app_data_dir.join("config.json");
        let contents = serde_json::to_string_pretty(self)
            .map_err(ConfigError::Serialize)?;

        fs::write(&config_path, contents).await
            .map_err(ConfigError::Io)?;

        log::info!("Configuration saved to {:?}", config_path);
        Ok(())
    }

    /// Check if sidecar models are configured
    pub fn has_sidecar_models(&self) -> bool {
        self.models.vlm_model_path.is_some() && self.models.vlm_mmproj_path.is_some()
    }

    /// Check if embedding model is configured
    pub fn has_embedding_model(&self) -> bool {
        self.models.embedding_model_path.is_some()
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    Parse(serde_json::Error),
    #[error("Failed to serialize config: {0}")]
    Serialize(serde_json::Error),
}

/// Information about current server mode for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ServerModeInfo {
    /// Current mode type
    pub mode: String,
    /// Whether the server is ready
    pub ready: bool,
    /// URL if connected to external server
    pub url: Option<String>,
    /// Model path if using sidecar
    pub model_path: Option<String>,
    /// Whether in embedding mode (sidecar only)
    pub is_embedding_mode: bool,
}

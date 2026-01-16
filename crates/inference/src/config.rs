//! Configuration types for the inference library

use serde::{Deserialize, Serialize};

use crate::constants::defaults;

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

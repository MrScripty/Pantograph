//! Configuration types for the inference library

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::constants::defaults;
use crate::device::DeviceBackend;

/// Device configuration for inference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceConfig {
    /// Backend-local llama.cpp device selector.
    #[serde(
        serialize_with = "serialize_llamacpp_device_backend",
        deserialize_with = "deserialize_llamacpp_device_backend"
    )]
    pub device: DeviceBackend,
    /// Number of layers to offload to GPU (-1 = all layers)
    pub gpu_layers: i32,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            device: DeviceBackend::Auto,
            gpu_layers: defaults::GPU_LAYERS,
        }
    }
}

fn serialize_llamacpp_device_backend<S>(
    device: &DeviceBackend,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&device.to_id())
}

fn deserialize_llamacpp_device_backend<'de, D>(deserializer: D) -> Result<DeviceBackend, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    DeviceBackend::try_from_id(&value).map_err(serde::de::Error::custom)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_config_serde_uses_llamacpp_selector_strings() {
        let config = DeviceConfig {
            device: DeviceBackend::Cuda(0),
            gpu_layers: 40,
        };

        let encoded = serde_json::to_value(&config).expect("encode device config");
        assert_eq!(
            encoded,
            serde_json::json!({
                "device": "CUDA0",
                "gpu_layers": 40
            })
        );

        let decoded: DeviceConfig = serde_json::from_value(encoded).expect("decode device config");
        assert_eq!(decoded.device, DeviceBackend::Cuda(0));
        assert_eq!(decoded.gpu_layers, 40);
    }

    #[test]
    fn device_config_serde_rejects_invalid_llamacpp_selector() {
        let error = serde_json::from_value::<DeviceConfig>(serde_json::json!({
            "device": "CUDAx",
            "gpu_layers": 40
        }))
        .expect_err("invalid device config selector should fail decode");

        assert!(error.to_string().contains("invalid ordinal"));
    }

    #[test]
    fn device_config_serde_rejects_canonical_device_id_as_llamacpp_selector() {
        let error = serde_json::from_value::<DeviceConfig>(serde_json::json!({
            "device": "cuda:0",
            "gpu_layers": 40
        }))
        .expect_err("canonical device id should not decode as llama.cpp selector");

        assert!(error
            .to_string()
            .contains("unknown llama.cpp device selector"));
    }
}

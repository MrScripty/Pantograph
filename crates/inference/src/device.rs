//! Device management for GPU/CPU inference
//!
//! Single source of truth for device detection, type parsing, and backend selection.

use std::path::Path;

use thiserror::Error;
use tokio::process::Command;

use crate::config::DeviceInfo;
use crate::constants::device_types;
use crate::device_contracts::{InferenceDeviceClass, InferenceDeviceId};
use crate::managed_runtime::{resolve_binary_command, ManagedBinaryId};

/// Represents a compute backend for inference
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DeviceBackend {
    /// CPU-only inference (no GPU acceleration)
    Cpu,
    /// NVIDIA CUDA device with index
    Cuda(u8),
    /// Vulkan device with index
    Vulkan(u8),
    /// Apple Metal device with index
    Metal(u8),
    /// Let llama-server auto-select the best device
    #[default]
    Auto,
}

/// Error produced while parsing a backend-local llama.cpp device selector.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceBackendParseError {
    /// The selector was empty after trimming.
    #[error("llama.cpp device selector must not be empty")]
    Empty,
    /// The selector does not match a known llama.cpp device family.
    #[error("unknown llama.cpp device selector '{0}'")]
    Unknown(String),
    /// A prefixed device selector was missing its numeric ordinal.
    #[error("llama.cpp device selector '{0}' is missing a device ordinal")]
    MissingOrdinal(String),
    /// A prefixed device selector carried an invalid numeric ordinal.
    #[error("llama.cpp device selector '{selector}' has invalid ordinal '{ordinal}'")]
    InvalidOrdinal {
        /// Original selector.
        selector: String,
        /// Invalid ordinal fragment.
        ordinal: String,
    },
}

/// Error produced while projecting backend-local selectors into canonical facts.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceBackendContractError {
    /// Auto mode must be resolved by scheduler policy before a selected fact exists.
    #[error("auto device policy must be resolved before selected device facts are emitted")]
    AutoRequiresResolution,
    /// The backend-local selector has no canonical scheduler device class yet.
    #[error("llama.cpp device selector '{0}' is not supported by canonical device contracts")]
    UnsupportedBackendDevice(String),
    /// The projected canonical device id failed validation.
    #[error("llama.cpp device selector '{selector}' projected invalid device id: {message}")]
    InvalidProjectedDeviceId {
        /// Original backend-local selector.
        selector: String,
        /// Validation failure message.
        message: String,
    },
}

impl DeviceBackend {
    /// Parse a llama.cpp device ID string into a `DeviceBackend`.
    ///
    /// # Examples
    /// ```
    /// use inference::DeviceBackend;
    ///
    /// assert_eq!(DeviceBackend::try_from_id("none").unwrap(), DeviceBackend::Cpu);
    /// assert_eq!(DeviceBackend::try_from_id("auto").unwrap(), DeviceBackend::Auto);
    /// assert_eq!(DeviceBackend::try_from_id("CUDA0").unwrap(), DeviceBackend::Cuda(0));
    /// assert_eq!(DeviceBackend::try_from_id("Vulkan1").unwrap(), DeviceBackend::Vulkan(1));
    /// ```
    pub fn try_from_id(id: &str) -> Result<Self, DeviceBackendParseError> {
        let id = id.trim();
        if id.is_empty() {
            return Err(DeviceBackendParseError::Empty);
        }

        Ok(match id {
            s if s == device_types::CPU => Self::Cpu,
            s if s == device_types::AUTO => Self::Auto,
            s if s.starts_with(device_types::CUDA_PREFIX) => {
                parse_backend_ordinal(s, device_types::CUDA_PREFIX, DeviceBackend::Cuda)?
            }
            s if s.starts_with(device_types::VULKAN_PREFIX) => {
                parse_backend_ordinal(s, device_types::VULKAN_PREFIX, DeviceBackend::Vulkan)?
            }
            s if s.starts_with(device_types::METAL_PREFIX) => {
                parse_backend_ordinal(s, device_types::METAL_PREFIX, DeviceBackend::Metal)?
            }
            _ => return Err(DeviceBackendParseError::Unknown(id.to_string())),
        })
    }

    /// Check if this device requires the CUDA binary
    pub fn requires_cuda_binary(&self) -> bool {
        matches!(self, Self::Cuda(_))
    }

    /// Check if this device requires the Vulkan binary
    pub fn requires_vulkan_binary(&self) -> bool {
        matches!(self, Self::Vulkan(_) | Self::Auto)
    }

    /// Convert to the command-line argument format for llama-server
    ///
    /// Returns None for Auto mode (let llama-server choose).
    pub fn to_arg(&self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::Cpu => Some(device_types::CPU.to_string()),
            Self::Cuda(i) => Some(format!("{}{}", device_types::CUDA_PREFIX, i)),
            Self::Vulkan(i) => Some(format!("{}{}", device_types::VULKAN_PREFIX, i)),
            Self::Metal(i) => Some(format!("{}{}", device_types::METAL_PREFIX, i)),
        }
    }

    /// Get the device ID string
    pub fn to_id(&self) -> String {
        match self {
            Self::Auto => device_types::AUTO.to_string(),
            Self::Cpu => device_types::CPU.to_string(),
            Self::Cuda(i) => format!("{}{}", device_types::CUDA_PREFIX, i),
            Self::Vulkan(i) => format!("{}{}", device_types::VULKAN_PREFIX, i),
            Self::Metal(i) => format!("{}{}", device_types::METAL_PREFIX, i),
        }
    }

    /// Check if this is GPU-accelerated
    pub fn is_gpu(&self) -> bool {
        !matches!(self, Self::Cpu)
    }

    /// Project a resolved backend-local selector into canonical scheduler facts.
    pub fn to_contract_device(
        &self,
    ) -> Result<(InferenceDeviceClass, InferenceDeviceId), DeviceBackendContractError> {
        let (device_class, device_id) = match self {
            Self::Cpu => (InferenceDeviceClass::Cpu, "cpu".to_string()),
            Self::Cuda(index) => (InferenceDeviceClass::Cuda, format!("cuda:{index}")),
            Self::Metal(index) => (InferenceDeviceClass::Metal, format!("metal:{index}")),
            Self::Vulkan(index) => {
                return Err(DeviceBackendContractError::UnsupportedBackendDevice(
                    format!("{}{}", device_types::VULKAN_PREFIX, index),
                ));
            }
            Self::Auto => return Err(DeviceBackendContractError::AutoRequiresResolution),
        };

        let device_id = InferenceDeviceId::parse(&device_id).map_err(|error| {
            DeviceBackendContractError::InvalidProjectedDeviceId {
                selector: self.to_id(),
                message: error.to_string(),
            }
        })?;
        Ok((device_class, device_id))
    }
}

impl std::str::FromStr for DeviceBackend {
    type Err = DeviceBackendParseError;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        Self::try_from_id(id)
    }
}

impl std::fmt::Display for DeviceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::Cuda(i) => write!(f, "CUDA {}", i),
            Self::Vulkan(i) => write!(f, "Vulkan {}", i),
            Self::Metal(i) => write!(f, "Metal {}", i),
            Self::Auto => write!(f, "Auto"),
        }
    }
}

fn parse_backend_ordinal(
    selector: &str,
    prefix: &str,
    build: impl FnOnce(u8) -> DeviceBackend,
) -> Result<DeviceBackend, DeviceBackendParseError> {
    let Some(ordinal) = selector.strip_prefix(prefix) else {
        return Err(DeviceBackendParseError::Unknown(selector.to_string()));
    };
    if ordinal.is_empty() {
        return Err(DeviceBackendParseError::MissingOrdinal(
            selector.to_string(),
        ));
    }
    let parsed = ordinal
        .parse::<u8>()
        .map_err(|_| DeviceBackendParseError::InvalidOrdinal {
            selector: selector.to_string(),
            ordinal: ordinal.to_string(),
        })?;
    Ok(build(parsed))
}

fn parse_device_vram(vram_info: &str) -> (u64, u64) {
    let parts: Vec<&str> = vram_info.split(',').collect();
    let total = parts
        .first()
        .and_then(|s| s.trim().strip_suffix(" MiB"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let free = parts
        .get(1)
        .and_then(|s| s.trim().strip_suffix(" MiB free"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    (total, free)
}

pub fn parse_llamacpp_device_listing(output: &str) -> Vec<DeviceInfo> {
    let mut devices = vec![DeviceInfo {
        id: device_types::CPU.to_string(),
        name: "CPU Only".to_string(),
        total_vram_mb: 0,
        free_vram_mb: 0,
    }];

    for line in output.lines() {
        let line = line.trim();
        let Some(colon_pos) = line.find(':') else {
            continue;
        };

        let id = line[..colon_pos].trim();
        let Ok(device_backend) = DeviceBackend::try_from_id(id) else {
            continue;
        };
        if matches!(device_backend, DeviceBackend::Auto | DeviceBackend::Cpu) {
            continue;
        }

        let rest = line[colon_pos + 1..].trim();
        let (name, total_vram_mb, free_vram_mb) = if let Some(paren_pos) = rest.rfind('(') {
            let name = rest[..paren_pos].trim().to_string();
            let vram_info = rest[paren_pos + 1..].trim_end_matches(')');
            let (total, free) = parse_device_vram(vram_info);
            (name, total, free)
        } else {
            (rest.to_string(), 0, 0)
        };

        devices.push(DeviceInfo {
            id: id.to_string(),
            name,
            total_vram_mb,
            free_vram_mb,
        });
    }

    devices
}

pub async fn list_llamacpp_devices(app_data_dir: &Path) -> Result<Vec<DeviceInfo>, String> {
    let resolved = resolve_binary_command(
        app_data_dir,
        ManagedBinaryId::LlamaCpp,
        &["--device", "CUDA0", "--list-devices"],
    )?;

    let mut command = Command::new(&resolved.executable_path);
    command
        .current_dir(&resolved.working_directory)
        .args(&resolved.args);
    for (key, value) in resolved.env_overrides {
        command.env(key, value);
    }

    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;
    let output = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    Ok(parse_llamacpp_device_listing(&output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_id_parses_known_llamacpp_devices() {
        assert_eq!(DeviceBackend::try_from_id("none"), Ok(DeviceBackend::Cpu));
        assert_eq!(DeviceBackend::try_from_id("auto"), Ok(DeviceBackend::Auto));
        assert_eq!(
            DeviceBackend::try_from_id("CUDA0"),
            Ok(DeviceBackend::Cuda(0))
        );
        assert_eq!(
            DeviceBackend::try_from_id("CUDA1"),
            Ok(DeviceBackend::Cuda(1))
        );
        assert_eq!(
            DeviceBackend::try_from_id("Vulkan0"),
            Ok(DeviceBackend::Vulkan(0))
        );
        assert_eq!(
            DeviceBackend::try_from_id("Vulkan1"),
            Ok(DeviceBackend::Vulkan(1))
        );
        assert_eq!(
            DeviceBackend::try_from_id("Metal0"),
            Ok(DeviceBackend::Metal(0))
        );
    }

    #[test]
    fn try_from_id_rejects_unknown_and_malformed_llamacpp_devices() {
        assert_eq!(
            DeviceBackend::try_from_id(""),
            Err(DeviceBackendParseError::Empty)
        );
        assert_eq!(
            DeviceBackend::try_from_id("unknown"),
            Err(DeviceBackendParseError::Unknown("unknown".to_string()))
        );
        assert_eq!(
            DeviceBackend::try_from_id("CUDA"),
            Err(DeviceBackendParseError::MissingOrdinal("CUDA".to_string()))
        );
        assert_eq!(
            DeviceBackend::try_from_id("CUDAx"),
            Err(DeviceBackendParseError::InvalidOrdinal {
                selector: "CUDAx".to_string(),
                ordinal: "x".to_string(),
            })
        );
        assert_eq!(
            DeviceBackend::try_from_id("CUDA300"),
            Err(DeviceBackendParseError::InvalidOrdinal {
                selector: "CUDA300".to_string(),
                ordinal: "300".to_string(),
            })
        );
    }

    #[test]
    fn test_requires_cuda() {
        assert!(DeviceBackend::Cuda(0).requires_cuda_binary());
        assert!(!DeviceBackend::Vulkan(0).requires_cuda_binary());
        assert!(!DeviceBackend::Cpu.requires_cuda_binary());
        assert!(!DeviceBackend::Auto.requires_cuda_binary());
    }

    #[test]
    fn test_to_arg() {
        assert_eq!(DeviceBackend::Cpu.to_arg(), Some("none".to_string()));
        assert_eq!(DeviceBackend::Auto.to_arg(), None);
        assert_eq!(DeviceBackend::Cuda(0).to_arg(), Some("CUDA0".to_string()));
        assert_eq!(
            DeviceBackend::Vulkan(1).to_arg(),
            Some("Vulkan1".to_string())
        );
    }

    #[test]
    fn test_roundtrip() {
        let devices = vec![
            DeviceBackend::Cpu,
            DeviceBackend::Auto,
            DeviceBackend::Cuda(0),
            DeviceBackend::Cuda(1),
            DeviceBackend::Vulkan(0),
            DeviceBackend::Metal(0),
        ];

        for device in devices {
            let id = device.to_id();
            let parsed = DeviceBackend::try_from_id(&id).expect("roundtrip device id should parse");
            assert_eq!(device, parsed);
        }
    }

    #[test]
    fn parse_llamacpp_listing_keeps_cpu_and_gpu_devices() {
        let devices = parse_llamacpp_device_listing(
            "
Available devices:
  Vulkan0: Intel(R) Graphics (RPL-P) (32003 MiB, 28803 MiB free)
  CUDA0: NVIDIA GeForce RTX 4060 Laptop GPU (8188 MiB, 547 MiB free)
",
        );

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].id, "none");
        assert_eq!(devices[0].name, "CPU Only");
        assert_eq!(devices[1].id, "Vulkan0");
        assert_eq!(devices[1].total_vram_mb, 32_003);
        assert_eq!(devices[1].free_vram_mb, 28_803);
        assert_eq!(devices[2].id, "CUDA0");
        assert_eq!(devices[2].total_vram_mb, 8_188);
        assert_eq!(devices[2].free_vram_mb, 547);
    }

    #[test]
    fn parse_llamacpp_listing_ignores_non_device_lines() {
        let devices = parse_llamacpp_device_listing(
            "
llama_model_loader: loaded meta data with 37 key-value pairs and 339 tensors from /models/demo.gguf
Metal backend initialized
",
        );

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "none");
    }

    #[test]
    fn parse_llamacpp_listing_ignores_malformed_device_ids() {
        let devices = parse_llamacpp_device_listing(
            "
Available devices:
  CUDAx: malformed
  Metal: missing ordinal
  CUDA1: NVIDIA GPU (8192 MiB, 4096 MiB free)
",
        );

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "none");
        assert_eq!(devices[1].id, "CUDA1");
    }

    #[test]
    fn device_backend_projects_canonical_contract_facts() {
        for (backend, expected_class, expected_id) in [
            (DeviceBackend::Cpu, InferenceDeviceClass::Cpu, "cpu"),
            (DeviceBackend::Cuda(1), InferenceDeviceClass::Cuda, "cuda:1"),
            (
                DeviceBackend::Metal(0),
                InferenceDeviceClass::Metal,
                "metal:0",
            ),
        ] {
            let (device_class, device_id) = backend
                .to_contract_device()
                .expect("resolved backend should project to contract facts");
            assert_eq!(device_class, expected_class);
            assert_eq!(device_id.as_str(), expected_id);
        }
    }

    #[test]
    fn device_backend_contract_projection_rejects_unresolved_or_unsupported_devices() {
        assert_eq!(
            DeviceBackend::Auto.to_contract_device(),
            Err(DeviceBackendContractError::AutoRequiresResolution)
        );
        assert_eq!(
            DeviceBackend::Vulkan(0).to_contract_device(),
            Err(DeviceBackendContractError::UnsupportedBackendDevice(
                "Vulkan0".to_string()
            ))
        );
    }
}

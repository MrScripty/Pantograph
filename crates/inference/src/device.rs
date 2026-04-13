//! Device management for GPU/CPU inference
//!
//! Single source of truth for device detection, type parsing, and backend selection.

use std::path::Path;

use tokio::process::Command;

use crate::config::DeviceInfo;
use crate::managed_runtime::{ManagedBinaryId, resolve_binary_command};
use crate::constants::device_types;

/// Represents a compute backend for inference
#[derive(Debug, Clone, PartialEq, Eq)]
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
    Auto,
}

impl DeviceBackend {
    /// Parse a device ID string into a DeviceBackend
    ///
    /// # Examples
    /// ```
    /// use inference::DeviceBackend;
    ///
    /// assert_eq!(DeviceBackend::from_id("none"), DeviceBackend::Cpu);
    /// assert_eq!(DeviceBackend::from_id("auto"), DeviceBackend::Auto);
    /// assert_eq!(DeviceBackend::from_id("CUDA0"), DeviceBackend::Cuda(0));
    /// assert_eq!(DeviceBackend::from_id("Vulkan1"), DeviceBackend::Vulkan(1));
    /// ```
    pub fn from_id(id: &str) -> Self {
        match id {
            s if s == device_types::CPU => Self::Cpu,
            s if s == device_types::AUTO => Self::Auto,
            s if s.starts_with(device_types::CUDA_PREFIX) => {
                let idx = s[device_types::CUDA_PREFIX.len()..].parse().unwrap_or(0);
                Self::Cuda(idx)
            }
            s if s.starts_with(device_types::VULKAN_PREFIX) => {
                let idx = s[device_types::VULKAN_PREFIX.len()..].parse().unwrap_or(0);
                Self::Vulkan(idx)
            }
            s if s.starts_with(device_types::METAL_PREFIX) => {
                let idx = s[device_types::METAL_PREFIX.len()..].parse().unwrap_or(0);
                Self::Metal(idx)
            }
            _ => Self::Auto,
        }
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
}

impl Default for DeviceBackend {
    fn default() -> Self {
        Self::Auto
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
        if id.contains(' ')
            || !(id.starts_with(device_types::VULKAN_PREFIX)
                || id.starts_with(device_types::CUDA_PREFIX)
                || id.starts_with(device_types::METAL_PREFIX))
        {
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
    fn test_from_id() {
        assert_eq!(DeviceBackend::from_id("none"), DeviceBackend::Cpu);
        assert_eq!(DeviceBackend::from_id("auto"), DeviceBackend::Auto);
        assert_eq!(DeviceBackend::from_id("CUDA0"), DeviceBackend::Cuda(0));
        assert_eq!(DeviceBackend::from_id("CUDA1"), DeviceBackend::Cuda(1));
        assert_eq!(DeviceBackend::from_id("Vulkan0"), DeviceBackend::Vulkan(0));
        assert_eq!(DeviceBackend::from_id("Vulkan1"), DeviceBackend::Vulkan(1));
        assert_eq!(DeviceBackend::from_id("Metal0"), DeviceBackend::Metal(0));
        assert_eq!(DeviceBackend::from_id("unknown"), DeviceBackend::Auto);
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
            let parsed = DeviceBackend::from_id(&id);
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
}

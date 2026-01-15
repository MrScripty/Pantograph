//! Device management for GPU/CPU inference
//!
//! Single source of truth for device detection, type parsing, and backend selection.
//! This consolidates device logic that was previously scattered across the wrapper script,
//! Rust commands, and TypeScript UI.

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
    /// use crate::llm::device::DeviceBackend;
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
    ///
    /// This is used by the wrapper script to select the correct llama-server binary.
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
        assert_eq!(DeviceBackend::Vulkan(1).to_arg(), Some("Vulkan1".to_string()));
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
}

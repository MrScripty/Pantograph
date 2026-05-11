use thiserror::Error;

use crate::device::{DeviceBackend, DeviceBackendParseError};
use crate::device_contracts::{DeviceContractError, InferenceDeviceId, InferenceDevicePolicy};

/// Typed startup device intent before backend-specific translation.
///
/// This deliberately separates scheduler-facing device policy and canonical
/// selected ids from llama.cpp-local selectors such as `CUDA0`. Callers must
/// choose the constructor that matches the source contract; this type must not
/// infer one device namespace from another.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
#[non_exhaustive]
pub enum BackendStartupDeviceIntent {
    /// Scheduler-facing policy such as automatic placement or explicit class.
    SchedulerPolicy(InferenceDevicePolicy),
    /// Concrete canonical device id selected by scheduler/backend facts.
    CanonicalDevice(InferenceDeviceId),
    /// Backend-local llama.cpp selector used only at the llama.cpp adapter.
    LlamaCppSelector(DeviceBackend),
}

/// Error produced while validating startup device intent.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendStartupDeviceIntentError {
    /// Canonical device-id validation failed.
    #[error("invalid canonical startup device id: {0}")]
    CanonicalDeviceId(DeviceContractError),
    /// llama.cpp selector validation failed.
    #[error("invalid llama.cpp startup device selector: {0}")]
    LlamaCppSelector(DeviceBackendParseError),
}

impl BackendStartupDeviceIntent {
    /// Build scheduler-facing startup policy intent.
    pub fn scheduler_policy(policy: InferenceDevicePolicy) -> Self {
        Self::SchedulerPolicy(policy)
    }

    /// Parse a canonical device id. This rejects policy keywords such as
    /// `auto` and backend-local selectors such as `CUDA0`.
    pub fn canonical_device_id(
        value: impl AsRef<str>,
    ) -> Result<Self, BackendStartupDeviceIntentError> {
        InferenceDeviceId::parse(value)
            .map(Self::CanonicalDevice)
            .map_err(BackendStartupDeviceIntentError::CanonicalDeviceId)
    }

    /// Parse a backend-local llama.cpp selector. This accepts selectors like
    /// `auto`, `none`, and `CUDA0`, but does not accept canonical ids like
    /// `cuda:0`.
    pub fn llama_cpp_selector(
        value: impl AsRef<str>,
    ) -> Result<Self, BackendStartupDeviceIntentError> {
        DeviceBackend::try_from_id(value.as_ref())
            .map(Self::LlamaCppSelector)
            .map_err(BackendStartupDeviceIntentError::LlamaCppSelector)
    }

    /// Borrow scheduler policy intent when this value carries one.
    #[must_use]
    pub fn as_scheduler_policy(&self) -> Option<&InferenceDevicePolicy> {
        match self {
            Self::SchedulerPolicy(policy) => Some(policy),
            Self::CanonicalDevice(_) | Self::LlamaCppSelector(_) => None,
        }
    }

    /// Borrow canonical device id when this value carries one.
    #[must_use]
    pub fn as_canonical_device_id(&self) -> Option<&InferenceDeviceId> {
        match self {
            Self::CanonicalDevice(device_id) => Some(device_id),
            Self::SchedulerPolicy(_) | Self::LlamaCppSelector(_) => None,
        }
    }

    /// Borrow backend-local llama.cpp selector when this value carries one.
    #[must_use]
    pub fn as_llama_cpp_selector(&self) -> Option<&DeviceBackend> {
        match self {
            Self::LlamaCppSelector(selector) => Some(selector),
            Self::SchedulerPolicy(_) | Self::CanonicalDevice(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_contracts::InferenceDeviceClass;

    #[test]
    fn startup_device_intent_keeps_scheduler_policy_separate() {
        let intent =
            BackendStartupDeviceIntent::scheduler_policy(InferenceDevicePolicy::Explicit {
                device_class: InferenceDeviceClass::Cuda,
                device_id: Some(InferenceDeviceId::parse("cuda:0").expect("valid device id")),
            });

        assert!(matches!(
            intent.as_scheduler_policy(),
            Some(InferenceDevicePolicy::Explicit {
                device_class: InferenceDeviceClass::Cuda,
                ..
            })
        ));
        assert!(intent.as_canonical_device_id().is_none());
        assert!(intent.as_llama_cpp_selector().is_none());
    }

    #[test]
    fn startup_device_intent_parses_canonical_device_ids_only() {
        let intent = BackendStartupDeviceIntent::canonical_device_id("cuda:0")
            .expect("canonical cuda id should parse");

        assert_eq!(
            intent
                .as_canonical_device_id()
                .expect("canonical device id")
                .as_str(),
            "cuda:0"
        );
        assert!(matches!(
            BackendStartupDeviceIntent::canonical_device_id("auto"),
            Err(BackendStartupDeviceIntentError::CanonicalDeviceId(
                DeviceContractError::ReservedIdentifier { .. }
            ))
        ));
        assert!(matches!(
            BackendStartupDeviceIntent::canonical_device_id("CUDA0"),
            Err(BackendStartupDeviceIntentError::CanonicalDeviceId(
                DeviceContractError::InvalidIdentifier { .. }
            ))
        ));
    }

    #[test]
    fn startup_device_intent_parses_llamacpp_selectors_only() {
        let intent = BackendStartupDeviceIntent::llama_cpp_selector("CUDA0")
            .expect("llama.cpp cuda selector should parse");

        assert_eq!(
            intent.as_llama_cpp_selector(),
            Some(&DeviceBackend::Cuda(0))
        );
        assert!(matches!(
            BackendStartupDeviceIntent::llama_cpp_selector("auto"),
            Ok(BackendStartupDeviceIntent::LlamaCppSelector(
                DeviceBackend::Auto
            ))
        ));
        assert!(matches!(
            BackendStartupDeviceIntent::llama_cpp_selector("cuda:0"),
            Err(BackendStartupDeviceIntentError::LlamaCppSelector(
                DeviceBackendParseError::Unknown(_)
            ))
        ));
    }
}

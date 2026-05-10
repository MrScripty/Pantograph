use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model_contracts::{InferenceTaskId, PumasModelRef};

use super::{BackendId, DeviceContractError, InferenceDeviceId, RuntimeVariantId};

/// Canonical device classes known to the execution planner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceDeviceClass {
    /// CPU execution.
    Cpu,
    /// NVIDIA CUDA execution.
    Cuda,
    /// Apple Metal execution.
    Metal,
    /// PyTorch MPS execution on Apple platforms.
    Mps,
}

impl InferenceDeviceClass {
    /// Stable snake_case label for diagnostics and wire payloads.
    #[must_use]
    pub fn canonical_label(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::Metal => "metal",
            Self::Mps => "mps",
        }
    }
}

impl fmt::Display for InferenceDeviceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_label())
    }
}

/// User or workflow device intent submitted to the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "policy", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceDevicePolicy {
    /// Scheduler-owned automatic selection.
    Auto,
    /// Explicit device-class intent that must fail when unavailable.
    Explicit {
        /// Requested canonical device class.
        device_class: InferenceDeviceClass,
        /// Optional concrete device id when the caller asks for one device.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_id: Option<InferenceDeviceId>,
    },
}

/// Severity for device/runtime selection diagnostics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeviceResolutionDiagnosticSeverity {
    /// Informational scheduler or adapter fact.
    Advisory,
    /// Non-blocking degraded or partial capability fact.
    Warning,
    /// Blocking validation or selection failure.
    Error,
}

/// Stable machine-readable device/runtime diagnostic codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeviceResolutionDiagnosticCode {
    /// Device policy could not be validated.
    InvalidDevicePolicy,
    /// Device id could not be validated.
    InvalidDeviceId,
    /// Runtime variant id could not be validated.
    InvalidRuntimeVariantId,
    /// Backend id could not be validated.
    InvalidBackendId,
    /// A candidate is unavailable.
    CandidateUnavailable,
    /// An explicit device request cannot be satisfied.
    ExplicitDeviceUnavailable,
    /// No valid candidate exists.
    NoValidCandidate,
    /// Auto mode found more than one valid candidate and needs scheduler policy.
    AmbiguousAutoResolution,
    /// Backend cannot execute the requested task or model.
    BackendIncompatible,
    /// Backend does not support the requested device class.
    UnsupportedDeviceClass,
    /// Runtime variant state is missing.
    MissingRuntimeVariant,
    /// A legacy raw device value was rejected instead of normalized.
    LegacyDeviceRejected,
}

/// Bounded diagnostic fact emitted while resolving devices/runtime variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DeviceResolutionDiagnostic {
    /// Stable diagnostic code.
    pub code: DeviceResolutionDiagnosticCode,
    /// Diagnostic severity.
    pub severity: DeviceResolutionDiagnosticSeverity,
    /// Human-readable bounded message.
    pub message: String,
    /// Related device class, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<InferenceDeviceClass>,
    /// Related device id, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<InferenceDeviceId>,
    /// Related runtime variant, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantId>,
    /// Related backend, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<BackendId>,
}

/// Runtime-variant capability fact reported by a backend adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeVariantCapability {
    /// Runtime variant this fact describes.
    pub runtime_variant_id: RuntimeVariantId,
    /// Device class the variant can use.
    pub device_class: InferenceDeviceClass,
    /// Whether the variant is currently usable.
    pub available: bool,
    /// Bounded diagnostics explaining unavailable/degraded state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DeviceResolutionDiagnostic>,
}

/// Device-resolution request consumed by canonical scheduler admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DeviceResolutionRequest {
    /// Caller or workflow device intent.
    pub policy: InferenceDevicePolicy,
    /// Runtime variant being considered.
    pub runtime_variant_id: RuntimeVariantId,
    /// Device classes exposed by candidate backend facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_device_classes: Vec<InferenceDeviceClass>,
}

/// Concrete resolved runtime/device decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[must_use]
pub struct DeviceResolutionDecision {
    /// Original policy that produced the decision.
    pub policy: InferenceDevicePolicy,
    /// Selected runtime variant.
    pub runtime_variant_id: RuntimeVariantId,
    /// Selected canonical device class.
    pub selected_device_class: InferenceDeviceClass,
    /// Selected concrete device id when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<InferenceDeviceId>,
    /// Non-fallback diagnostics retained with the decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DeviceResolutionDiagnostic>,
}

/// Static resource estimates reported by backend adapters when known.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendResourceEstimate {
    /// Estimated system RAM required in MiB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_mb: Option<u64>,
    /// Estimated VRAM required in MiB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_mb: Option<u64>,
    /// Maximum context tokens represented by this candidate, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u32>,
}

/// Optional observed throughput hint retained as scheduler evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BackendObservedThroughputHint {
    /// Tokens per second observed for text-like workloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f32>,
    /// Images per minute observed for image-generation workloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images_per_minute: Option<f32>,
}

/// Scheduler-facing backend candidate facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BackendExecutionCandidate {
    /// Backend adapter identity.
    pub backend_id: BackendId,
    /// Whether the candidate can execute the requested model.
    pub model_compatible: bool,
    /// Optional resolved model ref that this candidate was evaluated against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<PumasModelRef>,
    /// Supported canonical tasks for this candidate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_task_ids: Vec<InferenceTaskId>,
    /// Runtime variant exposed by the adapter.
    pub runtime_variant_id: RuntimeVariantId,
    /// Device class exposed by the adapter.
    pub device_class: InferenceDeviceClass,
    /// Concrete device id when the adapter knows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<InferenceDeviceId>,
    /// Static resource estimate when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_estimate: Option<BackendResourceEstimate>,
    /// Observed-throughput hint retained as evidence, not ranking policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput: Option<BackendObservedThroughputHint>,
    /// Bounded candidate diagnostics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DeviceResolutionDiagnostic>,
}

/// Scheduler-selected execution choice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[must_use]
pub struct BackendExecutionDecision {
    /// Selected backend identity.
    pub selected_backend_id: BackendId,
    /// Selected runtime variant.
    pub selected_runtime_variant_id: RuntimeVariantId,
    /// Selected device class.
    pub selected_device_class: InferenceDeviceClass,
    /// Selected concrete device id when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<InferenceDeviceId>,
    /// Device decision consumed by runtime load.
    pub device_decision: DeviceResolutionDecision,
    /// Selected task id when the scheduler is selecting for a known task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_task_id: Option<InferenceTaskId>,
    /// Selected model ref when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_ref: Option<PumasModelRef>,
    /// Diagnostics retained with the decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DeviceResolutionDiagnostic>,
}

impl BackendExecutionDecision {
    /// Build a selected decision from exactly one scheduler-selected candidate.
    pub fn try_from_selected_candidate(
        mut candidates: Vec<BackendExecutionCandidate>,
        policy: InferenceDevicePolicy,
        selected_task_id: Option<InferenceTaskId>,
    ) -> Result<Self, DeviceContractError> {
        if candidates.is_empty() {
            return Err(DeviceContractError::EmptyBackendCandidates);
        }
        if candidates.len() > 1 {
            return Err(DeviceContractError::AmbiguousBackendCandidates {
                count: candidates.len(),
            });
        }

        let candidate = candidates.remove(0);
        let device_decision = DeviceResolutionDecision {
            policy,
            runtime_variant_id: candidate.runtime_variant_id.clone(),
            selected_device_class: candidate.device_class,
            selected_device_id: candidate.device_id.clone(),
            diagnostics: candidate.diagnostics.clone(),
        };

        Ok(Self {
            selected_backend_id: candidate.backend_id,
            selected_runtime_variant_id: candidate.runtime_variant_id,
            selected_device_class: candidate.device_class,
            selected_device_id: candidate.device_id,
            device_decision,
            selected_task_id,
            selected_model_ref: candidate.model_ref,
            diagnostics: candidate.diagnostics,
        })
    }
}

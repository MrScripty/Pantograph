use std::cmp::Ordering;
use std::collections::BTreeSet;

use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};
use serde::{Deserialize, Serialize};

use crate::runtime_selection_policy::{
    candidate_is_eligible, candidate_matches_device_policy,
    select_runtime_technical_fit_automatically, RuntimeSelectionDecisionInput,
};
use crate::snapshot::RuntimeRegistrySnapshot;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitFactor {
    RequiredContextLength,
    RuntimeRequirements,
    ResidencyReuse,
    WarmupCost,
    BudgetPressure,
    QueuePressure,
}

impl RuntimeTechnicalFitFactor {
    pub const ALL: [Self; 6] = [
        Self::RequiredContextLength,
        Self::RuntimeRequirements,
        Self::ResidencyReuse,
        Self::WarmupCost,
        Self::BudgetPressure,
        Self::QueuePressure,
    ];

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitCandidateSourceKind {
    PumasPackageFacts,
    RuntimeCapabilityFacts,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitResidencyState {
    Unloaded,
    Loaded,
    Reserved,
    Active,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitWarmupState {
    Cold,
    Warm,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl RuntimeTechnicalFitOverride {
    pub fn normalized(&self) -> Option<Self> {
        let runtime_id = normalize_runtime_id(self.runtime_id.as_deref());
        let runtime_variant_id = normalize_trimmed_string(self.runtime_variant_id.as_deref());
        let model_id = normalize_trimmed_string(self.model_id.as_deref());
        let backend_key = normalize_backend_key(self.backend_key.as_deref());
        if runtime_id.is_none()
            && runtime_variant_id.is_none()
            && model_id.is_none()
            && backend_key.is_none()
        {
            None
        } else {
            Some(Self {
                runtime_id,
                runtime_variant_id,
                model_id,
                backend_key,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitDeviceClass {
    Cpu,
    Cuda,
    Metal,
    Mps,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum RuntimeTechnicalFitDevicePolicy {
    Auto,
    Explicit {
        device_class: RuntimeTechnicalFitDeviceClass,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
    },
}

impl RuntimeTechnicalFitDevicePolicy {
    pub fn normalized(&self) -> Self {
        match self {
            Self::Auto => Self::Auto,
            Self::Explicit {
                device_class,
                device_id,
            } => Self::Explicit {
                device_class: *device_class,
                device_id: normalize_trimmed_string(device_id.as_deref()),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitResourceEstimateKind {
    OutputRgbaBytes,
    VaeWorkingMemoryBytes,
    ModelResidencyBytes,
    RuntimeOverheadBytes,
    PeakVramBytes,
    PeakRamBytes,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitResourceEstimateState {
    Available,
    NotAvailable,
    NotImplemented,
    InsufficientFacts,
    Overflow,
    UnsupportedFamily,
    UnsupportedRuntime,
}

impl RuntimeTechnicalFitResourceEstimateState {
    pub fn is_available(self) -> bool {
        self == Self::Available
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitUnavailableResourceEstimateState {
    NotAvailable,
    NotImplemented,
    InsufficientFacts,
    Overflow,
    UnsupportedFamily,
    UnsupportedRuntime,
}

impl From<RuntimeTechnicalFitUnavailableResourceEstimateState>
    for RuntimeTechnicalFitResourceEstimateState
{
    fn from(state: RuntimeTechnicalFitUnavailableResourceEstimateState) -> Self {
        match state {
            RuntimeTechnicalFitUnavailableResourceEstimateState::NotAvailable => Self::NotAvailable,
            RuntimeTechnicalFitUnavailableResourceEstimateState::NotImplemented => {
                Self::NotImplemented
            }
            RuntimeTechnicalFitUnavailableResourceEstimateState::InsufficientFacts => {
                Self::InsufficientFacts
            }
            RuntimeTechnicalFitUnavailableResourceEstimateState::Overflow => Self::Overflow,
            RuntimeTechnicalFitUnavailableResourceEstimateState::UnsupportedFamily => {
                Self::UnsupportedFamily
            }
            RuntimeTechnicalFitUnavailableResourceEstimateState::UnsupportedRuntime => {
                Self::UnsupportedRuntime
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitResourceEstimateDiagnosticCode {
    ArithmeticOverflow,
    InvalidInput,
    InsufficientFacts,
    NotAvailable,
    NotImplemented,
    UnsupportedFamily,
    UnsupportedRuntime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitResourceEstimateDiagnosticSeverity {
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitResourceEstimateDiagnostic {
    pub code: RuntimeTechnicalFitResourceEstimateDiagnosticCode,
    pub severity: RuntimeTechnicalFitResourceEstimateDiagnosticSeverity,
    pub field_path: String,
    pub message: String,
}

impl RuntimeTechnicalFitResourceEstimateDiagnostic {
    pub fn error(
        code: RuntimeTechnicalFitResourceEstimateDiagnosticCode,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: RuntimeTechnicalFitResourceEstimateDiagnosticSeverity::Error,
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitResourceEstimate {
    kind: RuntimeTechnicalFitResourceEstimateKind,
    state: RuntimeTechnicalFitResourceEstimateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<RuntimeTechnicalFitResourceEstimateDiagnostic>,
}

impl RuntimeTechnicalFitResourceEstimate {
    pub fn available(kind: RuntimeTechnicalFitResourceEstimateKind, value_bytes: u64) -> Self {
        Self {
            kind,
            state: RuntimeTechnicalFitResourceEstimateState::Available,
            value_bytes: Some(value_bytes),
            diagnostics: Vec::new(),
        }
    }

    pub fn unavailable(
        kind: RuntimeTechnicalFitResourceEstimateKind,
        state: RuntimeTechnicalFitUnavailableResourceEstimateState,
        diagnostics: Vec<RuntimeTechnicalFitResourceEstimateDiagnostic>,
    ) -> Self {
        Self {
            kind,
            state: state.into(),
            value_bytes: None,
            diagnostics,
        }
    }

    pub fn kind(&self) -> RuntimeTechnicalFitResourceEstimateKind {
        self.kind
    }

    pub fn state(&self) -> RuntimeTechnicalFitResourceEstimateState {
        self.state
    }

    pub fn value_bytes(&self) -> Option<u64> {
        self.value_bytes
    }

    pub fn diagnostics(&self) -> &[RuntimeTechnicalFitResourceEstimateDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitObservedThroughputHint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_second_milli: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images_per_second_milli: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<u64>,
}

impl RuntimeTechnicalFitObservedThroughputHint {
    pub fn normalized(&self) -> Option<Self> {
        if self.tokens_per_second_milli.is_none()
            && self.images_per_second_milli.is_none()
            && self.sample_count.is_none()
        {
            None
        } else {
            Some(self.clone())
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitDeviceDiagnosticSeverity {
    Advisory,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitDeviceDiagnosticCode {
    InvalidDevicePolicy,
    InvalidDeviceId,
    InvalidRuntimeVariantId,
    InvalidBackendId,
    CandidateUnavailable,
    ExplicitDeviceUnavailable,
    NoValidCandidate,
    AmbiguousAutoResolution,
    BackendIncompatible,
    UnsupportedDeviceClass,
    MissingRuntimeVariant,
    MissingModelPackageFacts,
    CandidateSetOverflow,
    LegacyDeviceRejected,
    EvidenceUnsupportedTask,
    EvidenceBackendUnavailable,
    EvidenceMissingRuntimeCapability,
    EvidenceRequiredPackageUnavailable,
    EvidenceBackendCompatibilityRejected,
    EvidenceGraphRuntimeUnsatisfied,
    EvidenceNoAcceptedCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitDeviceDiagnostic {
    pub code: RuntimeTechnicalFitDeviceDiagnosticCode,
    pub severity: RuntimeTechnicalFitDeviceDiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<RuntimeTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_key: Option<String>,
}

impl RuntimeTechnicalFitDeviceDiagnostic {
    pub fn normalized(&self) -> Self {
        Self {
            code: self.code,
            severity: self.severity,
            message: normalize_trimmed_string(Some(self.message.as_str())).unwrap_or_default(),
            task_id: normalize_trimmed_string(self.task_id.as_deref()),
            runtime_id: normalize_runtime_id(self.runtime_id.as_deref()),
            device_class: self.device_class,
            device_id: normalize_trimmed_string(self.device_id.as_deref()),
            runtime_variant_id: normalize_trimmed_string(self.runtime_variant_id.as_deref()),
            backend_key: normalize_backend_key(self.backend_key.as_deref()),
            model_id: normalize_trimmed_string(self.model_id.as_deref()),
            evidence_key: normalize_trimmed_string(self.evidence_key.as_deref()),
            requested_runtime_key: normalize_backend_key(self.requested_runtime_key.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitDependencyReadinessSubjectKind {
    Package,
    Dependency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitDependencyReadinessState {
    Available,
    NotInstalled,
    NotImplemented,
    UnsupportedPlatform,
    MissingDependency,
    DisabledByPolicy,
    MissingModelFacts,
    RequiresRuntimeCapability,
    RequiresModelCapability,
}

impl RuntimeTechnicalFitDependencyReadinessState {
    #[must_use]
    pub fn is_ready(self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeTechnicalFitDependencyReadinessResolverOwner {
    Inference,
    EmbeddedRuntime,
    ManagedRuntime,
    RuntimeBridge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitDependencyReadinessFact {
    pub subject_kind: RuntimeTechnicalFitDependencyReadinessSubjectKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_family_id: Option<String>,
    pub dependency_id: String,
    pub state: RuntimeTechnicalFitDependencyReadinessState,
    pub resolver_owner: RuntimeTechnicalFitDependencyReadinessResolverOwner,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl RuntimeTechnicalFitDependencyReadinessFact {
    pub fn normalized(&self) -> Option<Self> {
        let dependency_id = normalize_trimmed_string(Some(self.dependency_id.as_str()))?;

        Some(Self {
            subject_kind: self.subject_kind,
            runtime_id: normalize_runtime_id(self.runtime_id.as_deref()),
            backend_key: normalize_backend_key(self.backend_key.as_deref()),
            runtime_variant_id: normalize_trimmed_string(self.runtime_variant_id.as_deref()),
            task_id: normalize_trimmed_string(self.task_id.as_deref()),
            model_family_id: normalize_trimmed_string(self.model_family_id.as_deref()),
            dependency_id,
            state: self.state,
            resolver_owner: self.resolver_owner,
            reason_code: normalize_trimmed_string(self.reason_code.as_deref()),
            reason: normalize_trimmed_string(self.reason.as_deref()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitResourcePressure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_run_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_runtime_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_runtime_capacity: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitCandidate {
    pub candidate_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<RuntimeTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_estimates: Vec<RuntimeTechnicalFitResourceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput_hint: Option<RuntimeTechnicalFitObservedThroughputHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_readiness: Vec<RuntimeTechnicalFitDependencyReadinessFact>,
    #[serde(default)]
    pub source_kind: RuntimeTechnicalFitCandidateSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residency_state: Option<RuntimeTechnicalFitResidencyState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warmup_state: Option<RuntimeTechnicalFitWarmupState>,
    #[serde(default)]
    pub supports_runtime_requirements: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_report: Option<RuntimeTechnicalFitCompatibilityReport>,
    #[serde(default)]
    pub compatibility_issue_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_issues: Vec<RuntimeTechnicalFitCompatibilityIssue>,
}

impl RuntimeTechnicalFitCandidate {
    pub fn normalized(&self) -> Self {
        let runtime_id = normalize_runtime_id(self.runtime_id.as_deref());
        let runtime_variant_id = normalize_trimmed_string(self.runtime_variant_id.as_deref());
        let backend_key = normalize_backend_key(self.backend_key.as_deref());
        let model_id = normalize_trimmed_string(self.model_id.as_deref());
        let candidate_id = normalize_trimmed_string(Some(self.candidate_id.as_str()))
            .unwrap_or_else(|| {
                derive_candidate_id(
                    runtime_id.as_deref(),
                    backend_key.as_deref(),
                    model_id.as_deref(),
                )
            });

        Self {
            candidate_id,
            runtime_id,
            runtime_variant_id,
            backend_key,
            model_id,
            device_class: self.device_class,
            selected_device_id: normalize_trimmed_string(self.selected_device_id.as_deref()),
            resource_estimates: self.resource_estimates.clone(),
            observed_throughput_hint: self
                .observed_throughput_hint
                .as_ref()
                .and_then(RuntimeTechnicalFitObservedThroughputHint::normalized),
            device_diagnostics: self
                .device_diagnostics
                .iter()
                .map(RuntimeTechnicalFitDeviceDiagnostic::normalized)
                .collect(),
            dependency_readiness: self
                .dependency_readiness
                .iter()
                .filter_map(RuntimeTechnicalFitDependencyReadinessFact::normalized)
                .collect(),
            source_kind: self.source_kind,
            context_window_tokens: self.context_window_tokens,
            residency_state: self.residency_state,
            warmup_state: self.warmup_state,
            supports_runtime_requirements: self.supports_runtime_requirements,
            compatibility_report: self
                .compatibility_report
                .as_ref()
                .map(RuntimeTechnicalFitCompatibilityReport::normalized),
            compatibility_issue_count: self.compatibility_issue_count,
            compatibility_issues: self
                .compatibility_issues
                .iter()
                .map(RuntimeTechnicalFitCompatibilityIssue::normalized)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitCompatibilityReport {
    pub status: String,
    pub compatible: bool,
    pub task: String,
    pub model_source: String,
    pub preprocessing: String,
    pub postprocessing: String,
}

impl RuntimeTechnicalFitCompatibilityReport {
    pub fn normalized(&self) -> Self {
        Self {
            status: normalize_trimmed_string(Some(self.status.as_str())).unwrap_or_default(),
            compatible: self.compatible,
            task: normalize_trimmed_string(Some(self.task.as_str())).unwrap_or_default(),
            model_source: normalize_trimmed_string(Some(self.model_source.as_str()))
                .unwrap_or_default(),
            preprocessing: normalize_trimmed_string(Some(self.preprocessing.as_str()))
                .unwrap_or_default(),
            postprocessing: normalize_trimmed_string(Some(self.postprocessing.as_str()))
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitCompatibilityIssue {
    pub kind: String,
    pub phase: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl RuntimeTechnicalFitCompatibilityIssue {
    pub fn normalized(&self) -> Self {
        Self {
            kind: normalize_trimmed_string(Some(self.kind.as_str())).unwrap_or_default(),
            phase: normalize_trimmed_string(Some(self.phase.as_str())).unwrap_or_default(),
            message: normalize_trimmed_string(Some(self.message.as_str())).unwrap_or_default(),
            model_id: normalize_trimmed_string(self.model_id.as_deref()),
            path: normalize_trimmed_string(self.path.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitRequest {
    pub runtime_snapshot: RuntimeRegistrySnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub required_model_ids: Vec<String>,
    #[serde(default)]
    pub required_backend_keys: Vec<String>,
    #[serde(default)]
    pub required_extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_context_window_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<RuntimeTechnicalFitOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_policy: Option<RuntimeTechnicalFitDevicePolicy>,
    #[serde(default)]
    pub legal_factors: Vec<RuntimeTechnicalFitFactor>,
    #[serde(default)]
    pub candidates: Vec<RuntimeTechnicalFitCandidate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_history_summaries: Vec<RuntimeTechnicalFitCandidateHistorySummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_pressure: Option<RuntimeTechnicalFitResourcePressure>,
}

impl RuntimeTechnicalFitRequest {
    pub fn normalized(&self) -> Self {
        let legal_factors = if self.legal_factors.is_empty() {
            RuntimeTechnicalFitFactor::all().to_vec()
        } else {
            BTreeSet::from_iter(self.legal_factors.iter().copied())
                .into_iter()
                .collect()
        };

        Self {
            runtime_snapshot: self.runtime_snapshot.clone(),
            workflow_id: normalize_trimmed_string(self.workflow_id.as_deref()),
            required_model_ids: normalize_string_list(&self.required_model_ids),
            required_backend_keys: normalize_backend_key_list(&self.required_backend_keys),
            required_extensions: normalize_string_list(&self.required_extensions),
            required_context_window_tokens: self.required_context_window_tokens,
            override_selection: self
                .override_selection
                .as_ref()
                .and_then(RuntimeTechnicalFitOverride::normalized),
            device_policy: self
                .device_policy
                .as_ref()
                .map(RuntimeTechnicalFitDevicePolicy::normalized),
            legal_factors,
            candidates: self
                .candidates
                .iter()
                .map(RuntimeTechnicalFitCandidate::normalized)
                .collect(),
            candidate_history_summaries: self
                .candidate_history_summaries
                .iter()
                .map(RuntimeTechnicalFitCandidateHistorySummary::normalized)
                .filter(|summary| !summary.candidate_id.is_empty())
                .collect(),
            resource_pressure: self.resource_pressure.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitSelectionMode {
    #[default]
    Automatic,
    ExplicitOverride,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitReasonCode {
    ExplicitRuntimeOverride,
    ExplicitRuntimeVariantOverride,
    ExplicitModelOverride,
    ExplicitBackendOverride,
    AutomaticRanking,
    ControlledExploration,
    RequiredContextLength,
    RuntimeRequirements,
    ResidencyReuse,
    WarmupCost,
    BudgetPressure,
    QueuePressure,
    MissingCandidateData,
    MissingRuntimeState,
    HistoricalPerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitReason {
    pub code: RuntimeTechnicalFitReasonCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<String>,
}

impl RuntimeTechnicalFitReason {
    pub fn new(code: RuntimeTechnicalFitReasonCode, candidate_id: Option<&str>) -> Self {
        Self {
            code,
            candidate_id: normalize_trimmed_string(candidate_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitCandidateSetSummary {
    #[serde(default)]
    pub total_candidate_count: u32,
    #[serde(default)]
    pub eligible_candidate_count: u32,
    #[serde(default)]
    pub rejected_candidate_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eligible_candidate_ids: Vec<String>,
}

impl RuntimeTechnicalFitCandidateSetSummary {
    pub fn normalized(&self) -> Self {
        Self {
            total_candidate_count: self.total_candidate_count,
            eligible_candidate_count: self.eligible_candidate_count,
            rejected_candidate_count: self.rejected_candidate_count,
            eligible_candidate_ids: normalize_string_list(&self.eligible_candidate_ids),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitCandidateHistorySummary {
    pub candidate_id: String,
    #[serde(default)]
    pub sample_count: u32,
    #[serde(default)]
    pub min_sample_count: u32,
    #[serde(default)]
    pub threshold_met: bool,
    #[serde(default)]
    pub completed_count: u32,
    #[serde(default)]
    pub failed_count: u32,
    #[serde(default)]
    pub cancelled_count: u32,
    #[serde(default)]
    pub duration_sample_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typical_min_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typical_max_duration_ms: Option<u64>,
    #[serde(default)]
    pub queue_wait_sample_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_queue_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_queue_wait_ms: Option<u64>,
}

impl RuntimeTechnicalFitCandidateHistorySummary {
    pub fn normalized(&self) -> Self {
        Self {
            candidate_id: normalize_trimmed_string(Some(self.candidate_id.as_str()))
                .unwrap_or_default(),
            sample_count: self.sample_count,
            min_sample_count: self.min_sample_count,
            threshold_met: self.threshold_met,
            completed_count: self.completed_count,
            failed_count: self.failed_count,
            cancelled_count: self.cancelled_count,
            duration_sample_count: self.duration_sample_count,
            average_duration_ms: self.average_duration_ms,
            median_duration_ms: self.median_duration_ms,
            typical_min_duration_ms: self.typical_min_duration_ms,
            typical_max_duration_ms: self.typical_max_duration_ms,
            queue_wait_sample_count: self.queue_wait_sample_count,
            average_queue_wait_ms: self.average_queue_wait_ms,
            median_queue_wait_ms: self.median_queue_wait_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitPolicyPhase {
    CandidateRanking,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitDecisionCode {
    SelectedCandidate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitHistoryThresholdState {
    NotEvaluated,
    InsufficientSamples,
    Evaluated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitSelectionPolicyTrace {
    pub policy_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_phase: Option<RuntimeTechnicalFitPolicyPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_code: Option<RuntimeTechnicalFitDecisionCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_threshold_state: Option<RuntimeTechnicalFitHistoryThresholdState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_set_summary: Option<RuntimeTechnicalFitCandidateSetSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranking_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploration_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_basis: Option<String>,
}

impl RuntimeTechnicalFitSelectionPolicyTrace {
    pub fn normalized(&self) -> Self {
        Self {
            policy_version: self.policy_version,
            policy_phase: self.policy_phase,
            decision_code: self.decision_code,
            history_threshold_state: self.history_threshold_state,
            candidate_set_summary: self
                .candidate_set_summary
                .as_ref()
                .map(RuntimeTechnicalFitCandidateSetSummary::normalized),
            ranking_reason: normalize_trimmed_string(self.ranking_reason.as_deref()),
            exploration_reason: normalize_trimmed_string(self.exploration_reason.as_deref()),
            seed_basis: normalize_trimmed_string(self.seed_basis.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitDecision {
    #[serde(default)]
    pub selection_mode: RuntimeTechnicalFitSelectionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_class: Option<RuntimeTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_estimates: Vec<RuntimeTechnicalFitResourceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput_hint: Option<RuntimeTechnicalFitObservedThroughputHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_readiness: Vec<RuntimeTechnicalFitDependencyReadinessFact>,
    #[serde(default)]
    pub reasons: Vec<RuntimeTechnicalFitReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_policy_trace: Option<RuntimeTechnicalFitSelectionPolicyTrace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_report: Option<RuntimeTechnicalFitCompatibilityReport>,
    #[serde(default)]
    pub compatibility_issue_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_issues: Vec<RuntimeTechnicalFitCompatibilityIssue>,
}

impl RuntimeTechnicalFitDecision {
    pub fn normalized(&self) -> Self {
        Self {
            selection_mode: self.selection_mode,
            selected_candidate_id: normalize_trimmed_string(self.selected_candidate_id.as_deref()),
            selected_runtime_id: normalize_runtime_id(self.selected_runtime_id.as_deref()),
            selected_runtime_variant_id: normalize_trimmed_string(
                self.selected_runtime_variant_id.as_deref(),
            ),
            selected_backend_key: normalize_backend_key(self.selected_backend_key.as_deref()),
            selected_model_id: normalize_trimmed_string(self.selected_model_id.as_deref()),
            selected_device_class: self.selected_device_class,
            selected_device_id: normalize_trimmed_string(self.selected_device_id.as_deref()),
            resource_estimates: self.resource_estimates.clone(),
            observed_throughput_hint: self
                .observed_throughput_hint
                .as_ref()
                .and_then(RuntimeTechnicalFitObservedThroughputHint::normalized),
            device_diagnostics: self
                .device_diagnostics
                .iter()
                .map(RuntimeTechnicalFitDeviceDiagnostic::normalized)
                .collect(),
            dependency_readiness: self
                .dependency_readiness
                .iter()
                .filter_map(RuntimeTechnicalFitDependencyReadinessFact::normalized)
                .collect(),
            reasons: self.reasons.clone(),
            selection_policy_trace: self
                .selection_policy_trace
                .as_ref()
                .map(RuntimeTechnicalFitSelectionPolicyTrace::normalized),
            compatibility_report: self
                .compatibility_report
                .as_ref()
                .map(RuntimeTechnicalFitCompatibilityReport::normalized),
            compatibility_issue_count: self.compatibility_issue_count,
            compatibility_issues: self
                .compatibility_issues
                .iter()
                .map(RuntimeTechnicalFitCompatibilityIssue::normalized)
                .collect(),
        }
    }
}

pub fn select_runtime_technical_fit(
    request: &RuntimeTechnicalFitRequest,
) -> RuntimeTechnicalFitDecision {
    let normalized = request.normalized();
    let candidates = normalized.candidates.clone();
    let mut reasons = Vec::new();

    if let Some(override_selection) = normalized.override_selection.as_ref() {
        if let Some(candidate) = candidates
            .iter()
            .filter(|candidate| candidate_matches_override(candidate, override_selection))
            .filter(|candidate| candidate_is_eligible(candidate, &normalized))
            .min_by(|left, right| compare_candidate_ids(left, right))
        {
            if override_selection.runtime_id.is_some() {
                reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::ExplicitRuntimeOverride,
                    Some(candidate.candidate_id.as_str()),
                ));
            }
            if override_selection.runtime_variant_id.is_some() {
                reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::ExplicitRuntimeVariantOverride,
                    Some(candidate.candidate_id.as_str()),
                ));
            }
            if override_selection.model_id.is_some() {
                reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::ExplicitModelOverride,
                    Some(candidate.candidate_id.as_str()),
                ));
            }
            if override_selection.backend_key.is_some() {
                reasons.push(RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                    Some(candidate.candidate_id.as_str()),
                ));
            }
            return decision_from_candidate(
                RuntimeTechnicalFitSelectionMode::ExplicitOverride,
                candidate,
                reasons,
            );
        }

        if override_selection.runtime_id.is_some() {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ExplicitRuntimeOverride,
                None,
            ));
        }
        if override_selection.runtime_variant_id.is_some() {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ExplicitRuntimeVariantOverride,
                None,
            ));
        }
        if override_selection.model_id.is_some() {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ExplicitModelOverride,
                None,
            ));
        }
        if override_selection.backend_key.is_some() {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                None,
            ));
        }
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            None,
        ));
        return unselected_decision_with_device_diagnostics(
            RuntimeTechnicalFitSelectionMode::ExplicitOverride,
            reasons,
            explicit_override_rejection_diagnostics(&normalized, &candidates, override_selection),
        );
    }

    let input = match RuntimeSelectionDecisionInput::try_from_normalized_request(&normalized) {
        Ok(input) => input,
        Err(error) => {
            return unselected_decision_with_device_diagnostics(
                RuntimeTechnicalFitSelectionMode::Automatic,
                vec![RuntimeTechnicalFitReason::new(
                    RuntimeTechnicalFitReasonCode::MissingCandidateData,
                    None,
                )],
                vec![error.into_diagnostic()],
            );
        }
    };

    select_runtime_technical_fit_automatically(input).into_technical_fit_decision()
}

fn normalize_runtime_id(value: Option<&str>) -> Option<String> {
    let value = normalize_trimmed_string(value)?;
    let normalized = canonical_runtime_id(&value);
    if normalized.trim().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_backend_key(value: Option<&str>) -> Option<String> {
    let value = normalize_trimmed_string(value)?;
    let normalized = canonical_runtime_backend_key(&value);
    if normalized.trim().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_trimmed_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_string_list(values: &[String]) -> Vec<String> {
    BTreeSet::from_iter(
        values
            .iter()
            .filter_map(|value| normalize_trimmed_string(Some(value))),
    )
    .into_iter()
    .collect()
}

fn normalize_backend_key_list(values: &[String]) -> Vec<String> {
    BTreeSet::from_iter(
        values
            .iter()
            .filter_map(|value| normalize_backend_key(Some(value))),
    )
    .into_iter()
    .collect()
}

fn derive_candidate_id(
    runtime_id: Option<&str>,
    backend_key: Option<&str>,
    model_id: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(runtime_id) = runtime_id {
        parts.push(runtime_id.to_string());
    }
    if let Some(backend_key) = backend_key {
        parts.push(backend_key.to_string());
    }
    if let Some(model_id) = model_id {
        parts.push(model_id.to_string());
    }

    if parts.is_empty() {
        "unknown_candidate".to_string()
    } else {
        parts.join("|")
    }
}

fn decision_from_candidate(
    selection_mode: RuntimeTechnicalFitSelectionMode,
    candidate: &RuntimeTechnicalFitCandidate,
    reasons: Vec<RuntimeTechnicalFitReason>,
) -> RuntimeTechnicalFitDecision {
    decision_from_candidate_with_trace(selection_mode, candidate, reasons, None)
}

pub(crate) fn decision_from_candidate_with_trace(
    selection_mode: RuntimeTechnicalFitSelectionMode,
    candidate: &RuntimeTechnicalFitCandidate,
    reasons: Vec<RuntimeTechnicalFitReason>,
    selection_policy_trace: Option<RuntimeTechnicalFitSelectionPolicyTrace>,
) -> RuntimeTechnicalFitDecision {
    RuntimeTechnicalFitDecision {
        selection_mode,
        selected_candidate_id: Some(candidate.candidate_id.clone()),
        selected_runtime_id: candidate.runtime_id.clone(),
        selected_runtime_variant_id: candidate.runtime_variant_id.clone(),
        selected_backend_key: candidate.backend_key.clone(),
        selected_model_id: candidate.model_id.clone(),
        selected_device_class: candidate.device_class,
        selected_device_id: candidate.selected_device_id.clone(),
        resource_estimates: candidate.resource_estimates.clone(),
        observed_throughput_hint: candidate.observed_throughput_hint.clone(),
        device_diagnostics: candidate.device_diagnostics.clone(),
        dependency_readiness: candidate.dependency_readiness.clone(),
        reasons,
        selection_policy_trace,
        compatibility_report: candidate.compatibility_report.clone(),
        compatibility_issue_count: candidate.compatibility_issue_count,
        compatibility_issues: candidate.compatibility_issues.clone(),
    }
    .normalized()
}

pub(crate) fn unselected_decision_with_device_diagnostics(
    selection_mode: RuntimeTechnicalFitSelectionMode,
    reasons: Vec<RuntimeTechnicalFitReason>,
    device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
) -> RuntimeTechnicalFitDecision {
    RuntimeTechnicalFitDecision {
        selection_mode,
        selected_candidate_id: None,
        selected_runtime_id: None,
        selected_runtime_variant_id: None,
        selected_backend_key: None,
        selected_model_id: None,
        selected_device_class: None,
        selected_device_id: None,
        resource_estimates: Vec::new(),
        observed_throughput_hint: None,
        device_diagnostics,
        dependency_readiness: Vec::new(),
        reasons,
        selection_policy_trace: None,
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    }
    .normalized()
}

pub(crate) fn explicit_device_unavailable_diagnostics(
    request: &RuntimeTechnicalFitRequest,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    let Some(RuntimeTechnicalFitDevicePolicy::Explicit {
        device_class,
        device_id,
    }) = request.device_policy.as_ref()
    else {
        return Vec::new();
    };

    if request
        .candidates
        .iter()
        .any(|candidate| candidate_matches_device_policy(candidate, request))
    {
        return Vec::new();
    }

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit could not satisfy the explicit device policy".to_string(),
        task_id: None,
        runtime_id: None,
        device_class: Some(*device_class),
        device_id: device_id.clone(),
        runtime_variant_id: None,
        backend_key: None,
        model_id: None,
        evidence_key: None,
        requested_runtime_key: None,
    }]
}

fn explicit_override_rejection_diagnostics(
    request: &RuntimeTechnicalFitRequest,
    candidates: &[RuntimeTechnicalFitCandidate],
    override_selection: &RuntimeTechnicalFitOverride,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    let explicit_device_diagnostics = explicit_device_unavailable_diagnostics(request);
    if !explicit_device_diagnostics.is_empty() {
        return explicit_device_diagnostics;
    }

    candidates
        .iter()
        .filter(|candidate| candidate_matches_override(candidate, override_selection))
        .min_by(|left, right| compare_candidate_ids(left, right))
        .map(candidate_rejection_diagnostics)
        .filter(|diagnostics| !diagnostics.is_empty())
        .unwrap_or_else(|| synthetic_explicit_override_diagnostic(override_selection))
}

fn candidate_rejection_diagnostics(
    candidate: &RuntimeTechnicalFitCandidate,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    if !candidate.device_diagnostics.is_empty() {
        return candidate.device_diagnostics.clone();
    }
    let dependency_diagnostics = candidate_dependency_readiness_diagnostics(candidate);
    if !dependency_diagnostics.is_empty() {
        return dependency_diagnostics;
    }

    let compatibility_rejected = candidate
        .compatibility_report
        .as_ref()
        .map(|report| !report.compatible)
        .unwrap_or(false)
        || candidate.compatibility_issue_count > 0
        || !candidate.compatibility_issues.is_empty();
    if !compatibility_rejected {
        return Vec::new();
    }

    let message = candidate
        .compatibility_issues
        .first()
        .map(|issue| issue.message.clone())
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| {
            "technical-fit explicit override candidate is incompatible with the requested model or task"
                .to_string()
        });

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message,
        task_id: None,
        runtime_id: candidate.runtime_id.clone(),
        device_class: candidate.device_class,
        device_id: candidate.selected_device_id.clone(),
        runtime_variant_id: candidate.runtime_variant_id.clone(),
        backend_key: candidate.backend_key.clone(),
        model_id: candidate.model_id.clone(),
        evidence_key: None,
        requested_runtime_key: None,
    }]
}

pub(crate) fn candidate_dependency_readiness_is_ready(
    candidate: &RuntimeTechnicalFitCandidate,
) -> bool {
    candidate
        .dependency_readiness
        .iter()
        .all(|fact| fact.state.is_ready())
}

pub(crate) fn candidate_dependency_readiness_diagnostics(
    candidate: &RuntimeTechnicalFitCandidate,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    candidate
        .dependency_readiness
        .iter()
        .filter(|fact| !fact.state.is_ready())
        .map(|fact| RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceRequiredPackageUnavailable,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: fact.reason.clone().unwrap_or_else(|| {
                format!(
                    "required dependency '{}' is not ready for technical-fit candidate '{}'",
                    fact.dependency_id, candidate.candidate_id
                )
            }),
            task_id: fact.task_id.clone(),
            runtime_id: fact
                .runtime_id
                .clone()
                .or_else(|| candidate.runtime_id.clone()),
            device_class: candidate.device_class,
            device_id: candidate.selected_device_id.clone(),
            runtime_variant_id: fact
                .runtime_variant_id
                .clone()
                .or_else(|| candidate.runtime_variant_id.clone()),
            backend_key: fact
                .backend_key
                .clone()
                .or_else(|| candidate.backend_key.clone()),
            model_id: candidate.model_id.clone(),
            evidence_key: Some(fact.dependency_id.clone()),
            requested_runtime_key: None,
        })
        .collect()
}

fn synthetic_explicit_override_diagnostic(
    override_selection: &RuntimeTechnicalFitOverride,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    if override_selection.runtime_variant_id.is_some() {
        return vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit could not satisfy the explicit runtime variant override"
                .to_string(),
            task_id: None,
            runtime_id: override_selection.runtime_id.clone(),
            device_class: None,
            device_id: None,
            runtime_variant_id: override_selection.runtime_variant_id.clone(),
            backend_key: override_selection.backend_key.clone(),
            model_id: override_selection.model_id.clone(),
            evidence_key: None,
            requested_runtime_key: override_selection.backend_key.clone(),
        }];
    }

    if override_selection.backend_key.is_some() {
        return vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit could not satisfy the explicit backend override".to_string(),
            task_id: None,
            runtime_id: override_selection.runtime_id.clone(),
            device_class: None,
            device_id: None,
            runtime_variant_id: None,
            backend_key: override_selection.backend_key.clone(),
            model_id: override_selection.model_id.clone(),
            evidence_key: None,
            requested_runtime_key: override_selection.backend_key.clone(),
        }];
    }

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit could not satisfy the explicit runtime or model override"
            .to_string(),
        task_id: None,
        runtime_id: override_selection.runtime_id.clone(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
        model_id: override_selection.model_id.clone(),
        evidence_key: None,
        requested_runtime_key: override_selection.backend_key.clone(),
    }]
}

fn candidate_matches_override(
    candidate: &RuntimeTechnicalFitCandidate,
    override_selection: &RuntimeTechnicalFitOverride,
) -> bool {
    let runtime_matches = override_selection.runtime_id.is_none()
        || candidate.runtime_id == override_selection.runtime_id;
    let runtime_variant_matches = override_selection.runtime_variant_id.is_none()
        || candidate.runtime_variant_id == override_selection.runtime_variant_id;
    let model_matches =
        override_selection.model_id.is_none() || candidate.model_id == override_selection.model_id;
    let backend_matches = override_selection.backend_key.is_none()
        || candidate.backend_key == override_selection.backend_key;
    runtime_matches && runtime_variant_matches && model_matches && backend_matches
}

pub(crate) fn compare_candidate_ids(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
) -> Ordering {
    left.candidate_id.cmp(&right.candidate_id)
}

#[cfg(test)]
#[path = "technical_fit_tests.rs"]
mod tests;

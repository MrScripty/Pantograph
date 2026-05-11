use std::cmp::Ordering;
use std::collections::BTreeSet;

use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};
use serde::{Deserialize, Serialize};

use crate::snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
use crate::state::RuntimeRegistryStatus;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitResourceEstimate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_ram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_min_vram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_min_ram_mb: Option<u64>,
}

impl RuntimeTechnicalFitResourceEstimate {
    pub fn normalized(&self) -> Option<Self> {
        if self.estimated_peak_vram_mb.is_none()
            && self.estimated_peak_ram_mb.is_none()
            && self.estimated_min_vram_mb.is_none()
            && self.estimated_min_ram_mb.is_none()
        {
            None
        } else {
            Some(self.clone())
        }
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
    LegacyDeviceRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeTechnicalFitDeviceDiagnostic {
    pub code: RuntimeTechnicalFitDeviceDiagnosticCode,
    pub severity: RuntimeTechnicalFitDeviceDiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<RuntimeTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl RuntimeTechnicalFitDeviceDiagnostic {
    pub fn normalized(&self) -> Self {
        Self {
            code: self.code,
            severity: self.severity,
            message: normalize_trimmed_string(Some(self.message.as_str())).unwrap_or_default(),
            device_class: self.device_class,
            device_id: normalize_trimmed_string(self.device_id.as_deref()),
            runtime_variant_id: normalize_trimmed_string(self.runtime_variant_id.as_deref()),
            backend_key: normalize_backend_key(self.backend_key.as_deref()),
        }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_ram_mb: Option<u64>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput_hint: Option<RuntimeTechnicalFitObservedThroughputHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
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
            resource_estimate: self
                .resource_estimate
                .as_ref()
                .and_then(RuntimeTechnicalFitResourceEstimate::normalized),
            observed_throughput_hint: self
                .observed_throughput_hint
                .as_ref()
                .and_then(RuntimeTechnicalFitObservedThroughputHint::normalized),
            device_diagnostics: self
                .device_diagnostics
                .iter()
                .map(RuntimeTechnicalFitDeviceDiagnostic::normalized)
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
    RequiredContextLength,
    RuntimeRequirements,
    ResidencyReuse,
    WarmupCost,
    BudgetPressure,
    QueuePressure,
    MissingCandidateData,
    MissingRuntimeState,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput_hint: Option<RuntimeTechnicalFitObservedThroughputHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
    #[serde(default)]
    pub reasons: Vec<RuntimeTechnicalFitReason>,
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
            resource_estimate: self
                .resource_estimate
                .as_ref()
                .and_then(RuntimeTechnicalFitResourceEstimate::normalized),
            observed_throughput_hint: self
                .observed_throughput_hint
                .as_ref()
                .and_then(RuntimeTechnicalFitObservedThroughputHint::normalized),
            device_diagnostics: self
                .device_diagnostics
                .iter()
                .map(RuntimeTechnicalFitDeviceDiagnostic::normalized)
                .collect(),
            reasons: self.reasons.clone(),
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

    let mut eligible_candidates = candidates
        .iter()
        .filter(|candidate| candidate_is_eligible(candidate, &normalized))
        .collect::<Vec<_>>();
    eligible_candidates.sort_by(|left, right| compare_candidates(left, right, &normalized));

    if let Some(selected_candidate) = eligible_candidates.first().copied() {
        if eligible_candidates.iter().skip(1).any(|candidate| {
            compare_candidate_priority(selected_candidate, candidate, &normalized).is_eq()
        }) {
            return unselected_decision_with_device_diagnostics(
                RuntimeTechnicalFitSelectionMode::Automatic,
                Vec::new(),
                vec![ambiguous_auto_resolution_diagnostic()],
            );
        }

        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::RuntimeRequirements,
            Some(selected_candidate.candidate_id.as_str()),
        ));

        if uses_factor(&normalized, RuntimeTechnicalFitFactor::ResidencyReuse)
            && candidate_residency_rank(selected_candidate, &normalized) > 0
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ResidencyReuse,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if uses_factor(&normalized, RuntimeTechnicalFitFactor::WarmupCost)
            && candidate_warmup_rank(selected_candidate, &normalized) > 0
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::WarmupCost,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if queue_pressure_applies(&normalized)
            && eligible_candidates.iter().skip(1).any(|candidate| {
                candidate_queue_pressure_rank(selected_candidate, &normalized)
                    > candidate_queue_pressure_rank(candidate, &normalized)
            })
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::QueuePressure,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        if budget_pressure_applies(&normalized)
            && eligible_candidates.iter().skip(1).any(|candidate| {
                candidate_budget_pressure_rank(selected_candidate, &normalized)
                    > candidate_budget_pressure_rank(candidate, &normalized)
            })
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::BudgetPressure,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        return decision_from_candidate(
            RuntimeTechnicalFitSelectionMode::Automatic,
            selected_candidate,
            reasons,
        );
    }

    let scoped_diagnostic_candidate = diagnostic_candidate(&candidates, &normalized);
    if candidates.is_empty() {
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            None,
        ));
    } else {
        if candidates
            .iter()
            .any(|candidate| candidate_has_missing_state(candidate, &normalized))
        {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::MissingRuntimeState,
                scoped_diagnostic_candidate.map(|candidate| candidate.candidate_id.as_str()),
            ));
        }
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            scoped_diagnostic_candidate.map(|candidate| candidate.candidate_id.as_str()),
        ));
    }

    unselected_decision_with_device_diagnostics(
        RuntimeTechnicalFitSelectionMode::Automatic,
        reasons,
        automatic_no_valid_candidate_diagnostics(&normalized),
    )
}

fn diagnostic_candidate<'a>(
    candidates: &'a [RuntimeTechnicalFitCandidate],
    normalized: &RuntimeTechnicalFitRequest,
) -> Option<&'a RuntimeTechnicalFitCandidate> {
    candidates
        .iter()
        .filter(|candidate| {
            let runtime_snapshot = candidate_runtime_snapshot(candidate, normalized);
            candidate_matches_required_models(candidate, runtime_snapshot, normalized)
                && candidate_matches_required_backends(candidate, runtime_snapshot, normalized)
        })
        .min_by(|left, right| compare_candidate_ids(left, right))
        .or_else(|| {
            candidates
                .iter()
                .min_by(|left, right| compare_candidate_ids(left, right))
        })
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
    RuntimeTechnicalFitDecision {
        selection_mode,
        selected_candidate_id: Some(candidate.candidate_id.clone()),
        selected_runtime_id: candidate.runtime_id.clone(),
        selected_runtime_variant_id: candidate.runtime_variant_id.clone(),
        selected_backend_key: candidate.backend_key.clone(),
        selected_model_id: candidate.model_id.clone(),
        selected_device_class: candidate.device_class,
        selected_device_id: candidate.selected_device_id.clone(),
        resource_estimate: candidate.resource_estimate.clone(),
        observed_throughput_hint: candidate.observed_throughput_hint.clone(),
        device_diagnostics: candidate.device_diagnostics.clone(),
        reasons,
        compatibility_report: candidate.compatibility_report.clone(),
        compatibility_issue_count: candidate.compatibility_issue_count,
        compatibility_issues: candidate.compatibility_issues.clone(),
    }
    .normalized()
}

fn unselected_decision_with_device_diagnostics(
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
        resource_estimate: None,
        observed_throughput_hint: None,
        device_diagnostics,
        reasons,
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    }
    .normalized()
}

fn explicit_device_unavailable_diagnostics(
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
        device_class: Some(*device_class),
        device_id: device_id.clone(),
        runtime_variant_id: None,
        backend_key: None,
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
        .map(|candidate| candidate.device_diagnostics.clone())
        .filter(|diagnostics| !diagnostics.is_empty())
        .unwrap_or_else(|| synthetic_explicit_override_diagnostic(override_selection))
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
            device_class: None,
            device_id: None,
            runtime_variant_id: override_selection.runtime_variant_id.clone(),
            backend_key: override_selection.backend_key.clone(),
        }];
    }

    if override_selection.backend_key.is_some() {
        return vec![RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "technical-fit could not satisfy the explicit backend override".to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: None,
            backend_key: override_selection.backend_key.clone(),
        }];
    }

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit could not satisfy the explicit runtime or model override"
            .to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }]
}

fn automatic_no_valid_candidate_diagnostics(
    request: &RuntimeTechnicalFitRequest,
) -> Vec<RuntimeTechnicalFitDeviceDiagnostic> {
    let explicit_device_diagnostics = explicit_device_unavailable_diagnostics(request);
    if !explicit_device_diagnostics.is_empty() {
        return explicit_device_diagnostics;
    }

    vec![RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit auto policy found no valid candidate".to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }]
}

fn ambiguous_auto_resolution_diagnostic() -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "technical-fit auto policy matched multiple equally ranked candidates".to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }
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

fn compare_candidates(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> Ordering {
    compare_candidate_priority(left, right, request)
        .then_with(|| compare_candidate_ids(left, right))
}

fn compare_candidate_priority(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> Ordering {
    candidate_residency_rank(left, request)
        .cmp(&candidate_residency_rank(right, request))
        .reverse()
        .then_with(|| {
            candidate_warmup_rank(left, request)
                .cmp(&candidate_warmup_rank(right, request))
                .reverse()
        })
        .then_with(|| {
            candidate_queue_pressure_rank(left, request)
                .cmp(&candidate_queue_pressure_rank(right, request))
                .reverse()
        })
        .then_with(|| {
            candidate_budget_pressure_rank(left, request)
                .cmp(&candidate_budget_pressure_rank(right, request))
                .reverse()
        })
}

fn compare_candidate_ids(
    left: &RuntimeTechnicalFitCandidate,
    right: &RuntimeTechnicalFitCandidate,
) -> Ordering {
    left.candidate_id.cmp(&right.candidate_id)
}

fn candidate_is_eligible(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let runtime_snapshot = candidate_runtime_snapshot(candidate, request);

    (!uses_factor(request, RuntimeTechnicalFitFactor::RuntimeRequirements)
        || candidate.supports_runtime_requirements)
        && candidate_matches_required_models(candidate, runtime_snapshot, request)
        && candidate_matches_required_backends(candidate, runtime_snapshot, request)
        && candidate_matches_device_policy(candidate, request)
        && candidate_meets_context_length(candidate, request)
}

fn candidate_matches_device_policy(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let Some(RuntimeTechnicalFitDevicePolicy::Explicit {
        device_class,
        device_id,
    }) = request.device_policy.as_ref()
    else {
        return true;
    };

    if candidate.device_class != Some(*device_class) {
        return false;
    }

    let Some(device_id) = device_id.as_deref() else {
        return true;
    };

    candidate.selected_device_id.as_deref() == Some(device_id)
}

fn candidate_matches_required_models(
    candidate: &RuntimeTechnicalFitCandidate,
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    if request.required_model_ids.is_empty() {
        return true;
    }

    if let Some(model_id) = candidate.model_id.as_deref() {
        return request
            .required_model_ids
            .iter()
            .any(|required| required == model_id);
    }

    let Some(runtime_snapshot) = runtime_snapshot else {
        return false;
    };

    request.required_model_ids.iter().all(|required| {
        runtime_snapshot
            .models
            .iter()
            .any(|model| model.model_id == *required)
    })
}

fn candidate_matches_required_backends(
    candidate: &RuntimeTechnicalFitCandidate,
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    if request.required_backend_keys.is_empty() {
        return true;
    }

    let candidate_backend_matches = candidate.backend_key.as_deref().map(|backend_key| {
        request
            .required_backend_keys
            .iter()
            .any(|required| required == backend_key)
    });

    if candidate_backend_matches == Some(true) {
        return true;
    }

    let Some(runtime_snapshot) = runtime_snapshot else {
        return false;
    };

    request.required_backend_keys.iter().all(|required| {
        runtime_snapshot
            .backend_keys
            .iter()
            .any(|backend_key| backend_key == required)
    })
}

fn candidate_meets_context_length(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    let Some(required_context_window_tokens) = request.required_context_window_tokens else {
        return true;
    };

    let Some(context_window_tokens) = candidate.context_window_tokens else {
        return false;
    };

    context_window_tokens >= required_context_window_tokens
}

fn candidate_has_missing_state(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> bool {
    candidate_runtime_snapshot(candidate, request).is_none()
        && candidate.runtime_id.is_some()
        && (candidate.residency_state.is_none() || candidate.warmup_state.is_none())
}

fn candidate_residency_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u8 {
    if !uses_factor(request, RuntimeTechnicalFitFactor::ResidencyReuse) {
        return 0;
    }

    match candidate
        .residency_state
        .or_else(|| snapshot_residency_state(candidate_runtime_snapshot(candidate, request)))
    {
        Some(RuntimeTechnicalFitResidencyState::Active) => 3,
        Some(RuntimeTechnicalFitResidencyState::Reserved) => 2,
        Some(RuntimeTechnicalFitResidencyState::Loaded) => 1,
        Some(RuntimeTechnicalFitResidencyState::Unloaded) | None => 0,
    }
}

fn candidate_warmup_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u8 {
    if !uses_factor(request, RuntimeTechnicalFitFactor::WarmupCost) {
        return 0;
    }

    match candidate
        .warmup_state
        .or_else(|| snapshot_warmup_state(candidate_runtime_snapshot(candidate, request)))
    {
        Some(RuntimeTechnicalFitWarmupState::Ready) => 2,
        Some(RuntimeTechnicalFitWarmupState::Warm) => 1,
        Some(RuntimeTechnicalFitWarmupState::Cold) | None => 0,
    }
}

fn candidate_queue_pressure_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    if !queue_pressure_applies(request) {
        return 0;
    }

    runtime_headroom_rank(candidate, request)
}

fn candidate_budget_pressure_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    if !budget_pressure_applies(request) {
        return 0;
    }

    runtime_headroom_rank(candidate, request)
}

fn runtime_headroom_rank(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &RuntimeTechnicalFitRequest,
) -> u16 {
    let active_reservation_count = candidate_runtime_snapshot(candidate, request)
        .map(|runtime| runtime.active_reservation_ids.len())
        .unwrap_or(usize::MAX);
    u16::MAX.saturating_sub(active_reservation_count.min(u16::MAX as usize) as u16)
}

fn candidate_runtime_snapshot<'a>(
    candidate: &RuntimeTechnicalFitCandidate,
    request: &'a RuntimeTechnicalFitRequest,
) -> Option<&'a RuntimeRegistryRuntimeSnapshot> {
    let runtime_id = candidate.runtime_id.as_deref()?;
    request
        .runtime_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == runtime_id)
}

fn snapshot_residency_state(
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
) -> Option<RuntimeTechnicalFitResidencyState> {
    let runtime_snapshot = runtime_snapshot?;
    match runtime_snapshot.status {
        RuntimeRegistryStatus::Busy => Some(RuntimeTechnicalFitResidencyState::Active),
        RuntimeRegistryStatus::Ready => {
            if runtime_snapshot.active_reservation_ids.is_empty() {
                Some(RuntimeTechnicalFitResidencyState::Loaded)
            } else {
                Some(RuntimeTechnicalFitResidencyState::Reserved)
            }
        }
        RuntimeRegistryStatus::Warming => Some(RuntimeTechnicalFitResidencyState::Reserved),
        RuntimeRegistryStatus::Stopped
        | RuntimeRegistryStatus::Stopping
        | RuntimeRegistryStatus::Unhealthy
        | RuntimeRegistryStatus::Failed => Some(RuntimeTechnicalFitResidencyState::Unloaded),
    }
}

fn snapshot_warmup_state(
    runtime_snapshot: Option<&RuntimeRegistryRuntimeSnapshot>,
) -> Option<RuntimeTechnicalFitWarmupState> {
    let runtime_snapshot = runtime_snapshot?;
    match runtime_snapshot.status {
        RuntimeRegistryStatus::Busy | RuntimeRegistryStatus::Ready => {
            Some(RuntimeTechnicalFitWarmupState::Ready)
        }
        RuntimeRegistryStatus::Warming => Some(RuntimeTechnicalFitWarmupState::Warm),
        RuntimeRegistryStatus::Stopped
        | RuntimeRegistryStatus::Stopping
        | RuntimeRegistryStatus::Unhealthy
        | RuntimeRegistryStatus::Failed => Some(RuntimeTechnicalFitWarmupState::Cold),
    }
}

fn uses_factor(request: &RuntimeTechnicalFitRequest, factor: RuntimeTechnicalFitFactor) -> bool {
    request.legal_factors.contains(&factor)
}

fn queue_pressure_applies(request: &RuntimeTechnicalFitRequest) -> bool {
    uses_factor(request, RuntimeTechnicalFitFactor::QueuePressure)
        && request
            .resource_pressure
            .as_ref()
            .and_then(|pressure| pressure.queued_run_count)
            .unwrap_or(0)
            > 0
}

fn budget_pressure_applies(request: &RuntimeTechnicalFitRequest) -> bool {
    uses_factor(request, RuntimeTechnicalFitFactor::BudgetPressure)
        && request.resource_pressure.as_ref().is_some_and(|pressure| {
            pressure.estimated_peak_vram_mb.is_some()
                || pressure.estimated_peak_ram_mb.is_some()
                || pressure
                    .loaded_runtime_count
                    .zip(pressure.loaded_runtime_capacity)
                    .is_some_and(|(count, capacity)| count >= capacity)
        })
}

#[cfg(test)]
#[path = "technical_fit_tests.rs"]
mod tests;

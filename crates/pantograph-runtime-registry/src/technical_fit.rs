use std::collections::BTreeSet;

use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};
use serde::{Deserialize, Serialize};

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
    PumasFeasible,
    RuntimeCapabilityFallback,
    OverrideFallback,
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
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl RuntimeTechnicalFitOverride {
    pub fn normalized(&self) -> Option<Self> {
        let model_id = normalize_trimmed_string(self.model_id.as_deref());
        let backend_key = normalize_backend_key(self.backend_key.as_deref());
        if model_id.is_none() && backend_key.is_none() {
            None
        } else {
            Some(Self {
                model_id,
                backend_key,
            })
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
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
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
}

impl RuntimeTechnicalFitCandidate {
    pub fn normalized(&self) -> Self {
        let runtime_id = normalize_runtime_id(self.runtime_id.as_deref());
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
            backend_key,
            model_id,
            source_kind: self.source_kind,
            context_window_tokens: self.context_window_tokens,
            residency_state: self.residency_state,
            warmup_state: self.warmup_state,
            supports_runtime_requirements: self.supports_runtime_requirements,
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
    ConservativeFallback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTechnicalFitReasonCode {
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
    DeterministicTieBreak,
    ConservativeFallback,
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
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_id: Option<String>,
    #[serde(default)]
    pub reasons: Vec<RuntimeTechnicalFitReason>,
}

impl RuntimeTechnicalFitDecision {
    pub fn normalized(&self) -> Self {
        Self {
            selection_mode: self.selection_mode,
            selected_candidate_id: normalize_trimmed_string(self.selected_candidate_id.as_deref()),
            selected_runtime_id: normalize_runtime_id(self.selected_runtime_id.as_deref()),
            selected_backend_key: normalize_backend_key(self.selected_backend_key.as_deref()),
            selected_model_id: normalize_trimmed_string(self.selected_model_id.as_deref()),
            reasons: self.reasons.clone(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::RuntimeRegistrySnapshot;

    fn empty_snapshot() -> RuntimeRegistrySnapshot {
        RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: Vec::new(),
            reservations: Vec::new(),
        }
    }

    #[test]
    fn technical_fit_request_normalizes_inputs_and_defaults_legal_factors() {
        let request = RuntimeTechnicalFitRequest {
            runtime_snapshot: empty_snapshot(),
            workflow_id: Some("  workflow-a  ".to_string()),
            required_model_ids: vec![" model-a ".to_string(), "model-a".to_string()],
            required_backend_keys: vec!["llama.cpp".to_string(), "llama_cpp".to_string()],
            required_extensions: vec![" kv_cache ".to_string(), "kv_cache".to_string()],
            required_context_window_tokens: Some(8192),
            override_selection: Some(RuntimeTechnicalFitOverride {
                model_id: Some(" model-a ".to_string()),
                backend_key: Some("llama.cpp".to_string()),
            }),
            legal_factors: Vec::new(),
            candidates: vec![RuntimeTechnicalFitCandidate {
                candidate_id: " ".to_string(),
                runtime_id: Some("llama.cpp".to_string()),
                backend_key: Some("llama.cpp".to_string()),
                model_id: Some(" model-a ".to_string()),
                source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
                context_window_tokens: Some(8192),
                residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                supports_runtime_requirements: true,
            }],
            resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
                queued_run_count: Some(2),
                loaded_runtime_count: Some(1),
                loaded_runtime_capacity: Some(2),
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
            }),
        };

        let normalized = request.normalized();

        assert_eq!(normalized.workflow_id.as_deref(), Some("workflow-a"));
        assert_eq!(normalized.required_model_ids, vec!["model-a".to_string()]);
        assert_eq!(
            normalized.required_backend_keys,
            vec!["llama_cpp".to_string()]
        );
        assert_eq!(normalized.required_extensions, vec!["kv_cache".to_string()]);
        assert_eq!(
            normalized.override_selection,
            Some(RuntimeTechnicalFitOverride {
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            })
        );
        assert_eq!(normalized.legal_factors, RuntimeTechnicalFitFactor::all());
        assert_eq!(
            normalized.candidates[0].candidate_id,
            "llama_cpp|llama_cpp|model-a"
        );
        assert_eq!(
            normalized.candidates[0].runtime_id.as_deref(),
            Some("llama_cpp")
        );
        assert_eq!(
            normalized.candidates[0].backend_key.as_deref(),
            Some("llama_cpp")
        );
    }

    #[test]
    fn technical_fit_override_drops_empty_fields() {
        let override_selection = RuntimeTechnicalFitOverride {
            model_id: Some("  ".to_string()),
            backend_key: Some(" ".to_string()),
        };

        assert_eq!(override_selection.normalized(), None);
    }

    #[test]
    fn technical_fit_decision_normalizes_selected_identifiers() {
        let decision = RuntimeTechnicalFitDecision {
            selection_mode: RuntimeTechnicalFitSelectionMode::ExplicitOverride,
            selected_candidate_id: Some(" candidate-1 ".to_string()),
            selected_runtime_id: Some("llama.cpp".to_string()),
            selected_backend_key: Some("llama.cpp".to_string()),
            selected_model_id: Some(" model-a ".to_string()),
            reasons: vec![RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                Some(" candidate-1 "),
            )],
        };

        let normalized = decision.normalized();

        assert_eq!(
            normalized.selected_candidate_id.as_deref(),
            Some("candidate-1")
        );
        assert_eq!(normalized.selected_runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            normalized.selected_backend_key.as_deref(),
            Some("llama_cpp")
        );
        assert_eq!(normalized.selected_model_id.as_deref(), Some("model-a"));
        assert_eq!(
            normalized.reasons,
            vec![RuntimeTechnicalFitReason {
                code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                candidate_id: Some("candidate-1".to_string()),
            }]
        );
    }
}

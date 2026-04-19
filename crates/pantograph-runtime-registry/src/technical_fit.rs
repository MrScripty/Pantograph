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

pub fn select_runtime_technical_fit(
    request: &RuntimeTechnicalFitRequest,
) -> RuntimeTechnicalFitDecision {
    let normalized = request.normalized();
    let mut candidates = normalized.candidates.clone();
    let mut reasons = Vec::new();

    if let Some(override_selection) = normalized.override_selection.as_ref() {
        if !candidates
            .iter()
            .any(|candidate| candidate_matches_override(candidate, override_selection))
        {
            candidates.push(override_fallback_candidate(override_selection));
        }

        if let Some(candidate) = candidates
            .iter()
            .filter(|candidate| candidate_matches_override(candidate, override_selection))
            .min_by(|left, right| compare_candidate_ids(left, right))
        {
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
    }

    let mut eligible_candidates = candidates
        .iter()
        .filter(|candidate| candidate_is_eligible(candidate, &normalized))
        .collect::<Vec<_>>();
    eligible_candidates.sort_by(|left, right| compare_candidates(left, right, &normalized));

    if let Some(selected_candidate) = eligible_candidates.first().copied() {
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

        if eligible_candidates.iter().skip(1).any(|candidate| {
            compare_candidate_priority(selected_candidate, candidate, &normalized).is_eq()
        }) {
            reasons.push(RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::DeterministicTieBreak,
                Some(selected_candidate.candidate_id.as_str()),
            ));
        }

        return decision_from_candidate(
            RuntimeTechnicalFitSelectionMode::Automatic,
            selected_candidate,
            reasons,
        );
    }

    let scoped_fallback_candidates = candidates
        .iter()
        .filter(|candidate| {
            let runtime_snapshot = candidate_runtime_snapshot(candidate, &normalized);
            candidate_matches_required_models(candidate, runtime_snapshot, &normalized)
                && candidate_matches_required_backends(candidate, runtime_snapshot, &normalized)
        })
        .collect::<Vec<_>>();
    let fallback_candidate = scoped_fallback_candidates
        .into_iter()
        .min_by(|left, right| compare_candidate_ids(left, right))
        .or_else(|| {
            candidates
                .iter()
                .min_by(|left, right| compare_candidate_ids(left, right))
        });
    if fallback_candidate.is_none() {
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
                fallback_candidate.map(|candidate| candidate.candidate_id.as_str()),
            ));
        }
        reasons.push(RuntimeTechnicalFitReason::new(
            RuntimeTechnicalFitReasonCode::MissingCandidateData,
            fallback_candidate.map(|candidate| candidate.candidate_id.as_str()),
        ));
    }
    reasons.push(RuntimeTechnicalFitReason::new(
        RuntimeTechnicalFitReasonCode::ConservativeFallback,
        fallback_candidate.map(|candidate| candidate.candidate_id.as_str()),
    ));

    if let Some(candidate) = fallback_candidate {
        decision_from_candidate(
            RuntimeTechnicalFitSelectionMode::ConservativeFallback,
            candidate,
            reasons,
        )
    } else {
        RuntimeTechnicalFitDecision {
            selection_mode: RuntimeTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: None,
            selected_model_id: None,
            reasons,
        }
        .normalized()
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

fn override_fallback_candidate(
    override_selection: &RuntimeTechnicalFitOverride,
) -> RuntimeTechnicalFitCandidate {
    RuntimeTechnicalFitCandidate {
        candidate_id: derive_candidate_id(
            None,
            override_selection.backend_key.as_deref(),
            override_selection.model_id.as_deref(),
        ),
        runtime_id: None,
        backend_key: override_selection.backend_key.clone(),
        model_id: override_selection.model_id.clone(),
        source_kind: RuntimeTechnicalFitCandidateSourceKind::OverrideFallback,
        context_window_tokens: None,
        residency_state: None,
        warmup_state: None,
        supports_runtime_requirements: true,
    }
    .normalized()
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
        selected_backend_key: candidate.backend_key.clone(),
        selected_model_id: candidate.model_id.clone(),
        reasons,
    }
    .normalized()
}

fn candidate_matches_override(
    candidate: &RuntimeTechnicalFitCandidate,
    override_selection: &RuntimeTechnicalFitOverride,
) -> bool {
    let model_matches =
        override_selection.model_id.is_none() || candidate.model_id == override_selection.model_id;
    let backend_matches = override_selection.backend_key.is_none()
        || candidate.backend_key == override_selection.backend_key;
    model_matches && backend_matches
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
        && candidate_meets_context_length(candidate, request)
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
mod tests {
    use super::*;
    use crate::snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
    use crate::state::RuntimeRegistryStatus;

    fn empty_snapshot() -> RuntimeRegistrySnapshot {
        RuntimeRegistrySnapshot {
            generated_at_ms: 123,
            runtimes: Vec::new(),
            reservations: Vec::new(),
        }
    }

    fn runtime_snapshot(
        runtime_id: &str,
        backend_keys: Vec<&str>,
        status: RuntimeRegistryStatus,
        active_reservation_count: usize,
    ) -> RuntimeRegistryRuntimeSnapshot {
        RuntimeRegistryRuntimeSnapshot {
            runtime_id: runtime_id.to_string(),
            display_name: runtime_id.to_string(),
            backend_keys: backend_keys.into_iter().map(ToOwned::to_owned).collect(),
            status,
            runtime_instance_id: Some(format!("{runtime_id}-instance")),
            last_error: None,
            last_transition_at_ms: 123,
            active_reservation_ids: (0..active_reservation_count as u64).collect(),
            models: Vec::new(),
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

    #[test]
    fn selector_prefers_explicit_override_over_hotter_candidate() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: RuntimeRegistrySnapshot {
                generated_at_ms: 123,
                runtimes: vec![
                    runtime_snapshot(
                        "runtime-a",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Busy,
                        1,
                    ),
                    runtime_snapshot("runtime-b", vec!["ollama"], RuntimeRegistryStatus::Ready, 0),
                ],
                reservations: Vec::new(),
            },
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: Vec::new(),
            required_extensions: Vec::new(),
            required_context_window_tokens: None,
            override_selection: Some(RuntimeTechnicalFitOverride {
                model_id: None,
                backend_key: Some("ollama".to_string()),
            }),
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-a".to_string(),
                    runtime_id: Some("runtime-a".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
                    context_window_tokens: Some(8192),
                    residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                    warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                    supports_runtime_requirements: true,
                },
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-b".to_string(),
                    runtime_id: Some("runtime-b".to_string()),
                    backend_key: Some("ollama".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasFeasible,
                    context_window_tokens: Some(8192),
                    residency_state: Some(RuntimeTechnicalFitResidencyState::Loaded),
                    warmup_state: Some(RuntimeTechnicalFitWarmupState::Warm),
                    supports_runtime_requirements: true,
                },
            ],
            resource_pressure: None,
        });

        assert_eq!(
            decision,
            RuntimeTechnicalFitDecision {
                selection_mode: RuntimeTechnicalFitSelectionMode::ExplicitOverride,
                selected_candidate_id: Some("runtime-b".to_string()),
                selected_runtime_id: Some("runtime-b".to_string()),
                selected_backend_key: Some("ollama".to_string()),
                selected_model_id: None,
                reasons: vec![RuntimeTechnicalFitReason {
                    code: RuntimeTechnicalFitReasonCode::ExplicitBackendOverride,
                    candidate_id: Some("runtime-b".to_string()),
                }],
            }
        );
    }

    #[test]
    fn selector_uses_snapshot_residency_and_deterministic_tie_break() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: RuntimeRegistrySnapshot {
                generated_at_ms: 123,
                runtimes: vec![
                    runtime_snapshot(
                        "runtime-b",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Ready,
                        0,
                    ),
                    runtime_snapshot(
                        "runtime-a",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Ready,
                        0,
                    ),
                ],
                reservations: Vec::new(),
            },
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
            required_context_window_tokens: None,
            override_selection: None,
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-b".to_string(),
                    runtime_id: Some("runtime-b".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-a".to_string(),
                    runtime_id: Some("runtime-a".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
            ],
            resource_pressure: None,
        });

        assert_eq!(
            decision.selection_mode,
            RuntimeTechnicalFitSelectionMode::Automatic
        );
        assert_eq!(decision.selected_candidate_id.as_deref(), Some("runtime-a"));
        assert_eq!(decision.selected_runtime_id.as_deref(), Some("runtime-a"));
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::DeterministicTieBreak
                && reason.candidate_id.as_deref() == Some("runtime-a")
        }));
    }

    #[test]
    fn selector_falls_back_conservatively_when_required_context_is_missing() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: empty_snapshot(),
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
            required_context_window_tokens: Some(8192),
            override_selection: None,
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![RuntimeTechnicalFitCandidate {
                candidate_id: "runtime-a".to_string(),
                runtime_id: Some("runtime-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                model_id: None,
                source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                context_window_tokens: None,
                residency_state: None,
                warmup_state: None,
                supports_runtime_requirements: true,
            }],
            resource_pressure: None,
        });

        assert_eq!(
            decision.selection_mode,
            RuntimeTechnicalFitSelectionMode::ConservativeFallback
        );
        assert_eq!(decision.selected_candidate_id.as_deref(), Some("runtime-a"));
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::MissingRuntimeState
                && reason.candidate_id.as_deref() == Some("runtime-a")
        }));
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::ConservativeFallback
                && reason.candidate_id.as_deref() == Some("runtime-a")
        }));
    }

    #[test]
    fn selector_conservative_fallback_stays_with_required_backend_candidate() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: empty_snapshot(),
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
            required_context_window_tokens: None,
            override_selection: None,
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![
                RuntimeTechnicalFitCandidate {
                    candidate_id: "candle".to_string(),
                    runtime_id: Some("candle".to_string()),
                    backend_key: Some("candle".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: Some(RuntimeTechnicalFitResidencyState::Active),
                    warmup_state: Some(RuntimeTechnicalFitWarmupState::Ready),
                    supports_runtime_requirements: true,
                },
                RuntimeTechnicalFitCandidate {
                    candidate_id: "llama_cpp".to_string(),
                    runtime_id: Some("llama_cpp".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: Some(RuntimeTechnicalFitResidencyState::Unloaded),
                    warmup_state: None,
                    supports_runtime_requirements: false,
                },
            ],
            resource_pressure: None,
        });

        assert_eq!(
            decision.selection_mode,
            RuntimeTechnicalFitSelectionMode::ConservativeFallback
        );
        assert_eq!(decision.selected_candidate_id.as_deref(), Some("llama_cpp"));
        assert_eq!(decision.selected_runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::ConservativeFallback
                && reason.candidate_id.as_deref() == Some("llama_cpp")
        }));
    }

    #[test]
    fn selector_prefers_more_headroom_under_queue_pressure() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: RuntimeRegistrySnapshot {
                generated_at_ms: 123,
                runtimes: vec![
                    runtime_snapshot(
                        "runtime-hot",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Ready,
                        3,
                    ),
                    runtime_snapshot(
                        "runtime-cool",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Ready,
                        1,
                    ),
                ],
                reservations: Vec::new(),
            },
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
            required_context_window_tokens: None,
            override_selection: None,
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-hot".to_string(),
                    runtime_id: Some("runtime-hot".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-cool".to_string(),
                    runtime_id: Some("runtime-cool".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
            ],
            resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
                queued_run_count: Some(4),
                loaded_runtime_count: Some(2),
                loaded_runtime_capacity: Some(4),
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
            }),
        });

        assert_eq!(
            decision.selected_candidate_id.as_deref(),
            Some("runtime-cool")
        );
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::QueuePressure
                && reason.candidate_id.as_deref() == Some("runtime-cool")
        }));
    }

    #[test]
    fn selector_prefers_more_headroom_under_budget_pressure() {
        let decision = select_runtime_technical_fit(&RuntimeTechnicalFitRequest {
            runtime_snapshot: RuntimeRegistrySnapshot {
                generated_at_ms: 123,
                runtimes: vec![
                    runtime_snapshot(
                        "runtime-tight",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Busy,
                        2,
                    ),
                    runtime_snapshot(
                        "runtime-roomy",
                        vec!["llama_cpp"],
                        RuntimeRegistryStatus::Busy,
                        0,
                    ),
                ],
                reservations: Vec::new(),
            },
            workflow_id: Some("workflow-a".to_string()),
            required_model_ids: Vec::new(),
            required_backend_keys: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
            required_context_window_tokens: None,
            override_selection: None,
            legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
            candidates: vec![
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-tight".to_string(),
                    runtime_id: Some("runtime-tight".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
                RuntimeTechnicalFitCandidate {
                    candidate_id: "runtime-roomy".to_string(),
                    runtime_id: Some("runtime-roomy".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                    model_id: None,
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
                    context_window_tokens: Some(8192),
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: true,
                },
            ],
            resource_pressure: Some(RuntimeTechnicalFitResourcePressure {
                queued_run_count: Some(0),
                loaded_runtime_count: Some(2),
                loaded_runtime_capacity: Some(2),
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
            }),
        });

        assert_eq!(
            decision.selected_candidate_id.as_deref(),
            Some("runtime-roomy")
        );
        assert!(decision.reasons.iter().any(|reason| {
            reason.code == RuntimeTechnicalFitReasonCode::BudgetPressure
                && reason.candidate_id.as_deref() == Some("runtime-roomy")
        }));
    }
}

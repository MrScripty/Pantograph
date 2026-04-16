use pantograph_runtime_identity::canonical_runtime_backend_key;
use serde::{Deserialize, Serialize};

use crate::workflow::WorkflowRuntimeRequirements;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl WorkflowTechnicalFitOverride {
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
pub struct WorkflowTechnicalFitQueuePressure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_session_queue_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_queued_run_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_runtime_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_runtime_capacity: Option<u64>,
}

impl WorkflowTechnicalFitQueuePressure {
    pub fn normalized(&self) -> Option<Self> {
        if self.current_session_queue_depth.is_none()
            && self.total_queued_run_count.is_none()
            && self.loaded_runtime_count.is_none()
            && self.loaded_runtime_capacity.is_none()
        {
            None
        } else {
            Some(self.clone())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitRequest {
    pub workflow_id: String,
    pub runtime_requirements: WorkflowRuntimeRequirements,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_pressure: Option<WorkflowTechnicalFitQueuePressure>,
}

impl WorkflowTechnicalFitRequest {
    pub fn normalized(&self) -> Self {
        Self {
            workflow_id: self.workflow_id.trim().to_string(),
            runtime_requirements: normalize_runtime_requirements(&self.runtime_requirements),
            override_selection: self
                .override_selection
                .as_ref()
                .and_then(WorkflowTechnicalFitOverride::normalized),
            session_id: normalize_trimmed_string(self.session_id.as_deref()),
            usage_profile: normalize_trimmed_string(self.usage_profile.as_deref()),
            queue_pressure: self
                .queue_pressure
                .as_ref()
                .and_then(WorkflowTechnicalFitQueuePressure::normalized),
        }
    }
}

pub fn build_workflow_technical_fit_request(
    workflow_id: &str,
    runtime_requirements: &WorkflowRuntimeRequirements,
    override_selection: Option<WorkflowTechnicalFitOverride>,
    session_id: Option<&str>,
    usage_profile: Option<&str>,
    queue_pressure: Option<WorkflowTechnicalFitQueuePressure>,
) -> WorkflowTechnicalFitRequest {
    WorkflowTechnicalFitRequest {
        workflow_id: workflow_id.trim().to_string(),
        runtime_requirements: normalize_runtime_requirements(runtime_requirements),
        override_selection: override_selection.and_then(|value| value.normalized()),
        session_id: normalize_trimmed_string(session_id),
        usage_profile: normalize_trimmed_string(usage_profile),
        queue_pressure: queue_pressure.and_then(|value| value.normalized()),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitSelectionMode {
    #[default]
    Automatic,
    ExplicitOverride,
    ConservativeFallback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitReasonCode {
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
pub struct WorkflowTechnicalFitReason {
    pub code: WorkflowTechnicalFitReasonCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<String>,
}

impl WorkflowTechnicalFitReason {
    pub fn new(code: WorkflowTechnicalFitReasonCode, candidate_id: Option<&str>) -> Self {
        Self {
            code,
            candidate_id: normalize_trimmed_string(candidate_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitDecision {
    #[serde(default)]
    pub selection_mode: WorkflowTechnicalFitSelectionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_id: Option<String>,
    #[serde(default)]
    pub reasons: Vec<WorkflowTechnicalFitReason>,
}

impl WorkflowTechnicalFitDecision {
    pub fn normalized(&self) -> Self {
        Self {
            selection_mode: self.selection_mode,
            selected_candidate_id: normalize_trimmed_string(self.selected_candidate_id.as_deref()),
            selected_runtime_id: normalize_trimmed_string(self.selected_runtime_id.as_deref()),
            selected_backend_key: normalize_backend_key(self.selected_backend_key.as_deref()),
            selected_model_id: normalize_trimmed_string(self.selected_model_id.as_deref()),
            reasons: self.reasons.clone(),
        }
    }
}

fn normalize_runtime_requirements(
    runtime_requirements: &WorkflowRuntimeRequirements,
) -> WorkflowRuntimeRequirements {
    let mut required_models = runtime_requirements.required_models.clone();
    required_models.sort();
    required_models.dedup();
    required_models.retain(|value| !value.trim().is_empty());

    let mut required_backends = runtime_requirements
        .required_backends
        .iter()
        .filter_map(|value| normalize_backend_key(Some(value)))
        .collect::<Vec<_>>();
    required_backends.sort();
    required_backends.dedup();

    let mut required_extensions = runtime_requirements
        .required_extensions
        .iter()
        .filter_map(|value| normalize_trimmed_string(Some(value)))
        .collect::<Vec<_>>();
    required_extensions.sort();
    required_extensions.dedup();

    WorkflowRuntimeRequirements {
        estimated_peak_vram_mb: runtime_requirements.estimated_peak_vram_mb,
        estimated_peak_ram_mb: runtime_requirements.estimated_peak_ram_mb,
        estimated_min_vram_mb: runtime_requirements.estimated_min_vram_mb,
        estimated_min_ram_mb: runtime_requirements.estimated_min_ram_mb,
        estimation_confidence: normalize_trimmed_string(Some(
            runtime_requirements.estimation_confidence.as_str(),
        ))
        .unwrap_or_else(|| "unknown".to_string()),
        required_models,
        required_backends,
        required_extensions,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_requirements() -> WorkflowRuntimeRequirements {
        WorkflowRuntimeRequirements {
            estimated_peak_vram_mb: Some(4096),
            estimated_peak_ram_mb: Some(8192),
            estimated_min_vram_mb: Some(2048),
            estimated_min_ram_mb: Some(4096),
            estimation_confidence: " medium ".to_string(),
            required_models: vec!["model-a".to_string(), "model-a".to_string()],
            required_backends: vec!["llama.cpp".to_string(), "llama_cpp".to_string()],
            required_extensions: vec!["kv_cache".to_string(), " kv_cache ".to_string()],
        }
    }

    #[test]
    fn build_workflow_technical_fit_request_normalizes_inputs() {
        let request = build_workflow_technical_fit_request(
            " workflow-a ",
            &runtime_requirements(),
            Some(WorkflowTechnicalFitOverride {
                model_id: Some(" model-a ".to_string()),
                backend_key: Some("llama.cpp".to_string()),
            }),
            Some(" session-a "),
            Some(" interactive "),
            Some(WorkflowTechnicalFitQueuePressure {
                current_session_queue_depth: Some(1),
                total_queued_run_count: Some(2),
                loaded_runtime_count: Some(1),
                loaded_runtime_capacity: Some(4),
            }),
        );

        assert_eq!(request.workflow_id, "workflow-a");
        assert_eq!(
            request.runtime_requirements.required_models,
            vec!["model-a"]
        );
        assert_eq!(
            request.runtime_requirements.required_backends,
            vec!["llama_cpp"]
        );
        assert_eq!(
            request.runtime_requirements.required_extensions,
            vec!["kv_cache"]
        );
        assert_eq!(request.session_id.as_deref(), Some("session-a"));
        assert_eq!(request.usage_profile.as_deref(), Some("interactive"));
        assert_eq!(
            request.override_selection,
            Some(WorkflowTechnicalFitOverride {
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            })
        );
    }

    #[test]
    fn workflow_technical_fit_decision_normalizes_selected_backend() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ExplicitOverride,
            selected_candidate_id: Some(" candidate-a ".to_string()),
            selected_runtime_id: Some("runtime-a".to_string()),
            selected_backend_key: Some("llama.cpp".to_string()),
            selected_model_id: Some(" model-a ".to_string()),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ExplicitBackendOverride,
                Some(" candidate-a "),
            )],
        };

        let normalized = decision.normalized();

        assert_eq!(
            normalized.selected_candidate_id.as_deref(),
            Some("candidate-a")
        );
        assert_eq!(normalized.selected_runtime_id.as_deref(), Some("runtime-a"));
        assert_eq!(
            normalized.selected_backend_key.as_deref(),
            Some("llama_cpp")
        );
        assert_eq!(normalized.selected_model_id.as_deref(), Some("model-a"));
        assert_eq!(
            normalized.reasons,
            vec![WorkflowTechnicalFitReason {
                code: WorkflowTechnicalFitReasonCode::ExplicitBackendOverride,
                candidate_id: Some("candidate-a".to_string()),
            }]
        );
    }
}

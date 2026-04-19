use pantograph_runtime_identity::canonical_runtime_backend_key;
use serde::{Deserialize, Serialize};

use crate::workflow::{
    evaluate_runtime_preflight, validate_workflow_id, WorkflowHost, WorkflowHostCapabilities,
    WorkflowRuntimeCapability, WorkflowRuntimeIssue, WorkflowRuntimeRequirements, WorkflowService,
    WorkflowServiceError,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowTechnicalFitSessionContext {
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub queue_pressure: WorkflowTechnicalFitQueuePressure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowRuntimePreflightAssessment {
    pub technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
    pub runtime_warnings: Vec<WorkflowRuntimeIssue>,
    pub blocking_runtime_issues: Vec<WorkflowRuntimeIssue>,
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

impl WorkflowService {
    pub(crate) async fn workflow_runtime_preflight_assessment<H: WorkflowHost>(
        &self,
        host: &H,
        workflow_id: &str,
        capabilities: &WorkflowHostCapabilities,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowRuntimePreflightAssessment, WorkflowServiceError> {
        let request = build_workflow_technical_fit_request(
            workflow_id,
            &capabilities.runtime_requirements,
            override_selection,
            None,
            None,
            None,
        );
        self.runtime_preflight_assessment(host, &request, capabilities)
            .await
    }

    pub(crate) async fn workflow_session_runtime_preflight_assessment<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        capabilities: &WorkflowHostCapabilities,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowRuntimePreflightAssessment, WorkflowServiceError> {
        let session_context = self.technical_fit_session_context(session_id)?;
        let request = build_workflow_technical_fit_request(
            &session_context.workflow_id,
            &capabilities.runtime_requirements,
            override_selection,
            Some(session_id.trim()),
            session_context.usage_profile.as_deref(),
            Some(session_context.queue_pressure),
        );
        self.runtime_preflight_assessment(host, &request, capabilities)
            .await
    }

    async fn runtime_preflight_assessment<H: WorkflowHost>(
        &self,
        host: &H,
        request: &WorkflowTechnicalFitRequest,
        capabilities: &WorkflowHostCapabilities,
    ) -> Result<WorkflowRuntimePreflightAssessment, WorkflowServiceError> {
        let technical_fit_decision = host.workflow_technical_fit_decision(request).await?;
        Ok(match technical_fit_decision.as_ref() {
            Some(decision) => workflow_runtime_preflight_from_decision(
                decision,
                &capabilities.runtime_requirements.required_backends,
                &capabilities.runtime_capabilities,
            ),
            None => {
                let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
                    &capabilities.runtime_requirements.required_backends,
                    &capabilities.runtime_capabilities,
                );
                WorkflowRuntimePreflightAssessment {
                    technical_fit_decision: None,
                    runtime_warnings,
                    blocking_runtime_issues,
                }
            }
        })
    }

    pub(crate) fn technical_fit_session_context(
        &self,
        session_id: &str,
    ) -> Result<WorkflowTechnicalFitSessionContext, WorkflowServiceError> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let store = self.session_store_guard()?;
        let session = store.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let total_queued_run_count = store
            .active
            .values()
            .map(|state| state.queue_len() as u64)
            .sum::<u64>();
        Ok(WorkflowTechnicalFitSessionContext {
            workflow_id: session.workflow_id.clone(),
            usage_profile: session.usage_profile.clone(),
            queue_pressure: WorkflowTechnicalFitQueuePressure {
                current_session_queue_depth: Some(session.queue_len() as u64),
                total_queued_run_count: Some(total_queued_run_count),
                loaded_runtime_count: Some(store.loaded_session_count() as u64),
                loaded_runtime_capacity: Some(store.max_loaded_sessions as u64),
            },
        })
    }

    pub async fn workflow_technical_fit_request<H: WorkflowHost>(
        &self,
        host: &H,
        workflow_id: &str,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowTechnicalFitRequest, WorkflowServiceError> {
        validate_workflow_id(workflow_id)?;
        host.validate_workflow(workflow_id).await?;
        let capabilities = host.workflow_capabilities(workflow_id).await?;
        Ok(build_workflow_technical_fit_request(
            workflow_id,
            &capabilities.runtime_requirements,
            override_selection,
            None,
            None,
            None,
        ))
    }

    pub async fn workflow_session_technical_fit_request<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowTechnicalFitRequest, WorkflowServiceError> {
        let session_context = self.technical_fit_session_context(session_id)?;
        let capabilities = host
            .workflow_capabilities(&session_context.workflow_id)
            .await?;
        Ok(build_workflow_technical_fit_request(
            &session_context.workflow_id,
            &capabilities.runtime_requirements,
            override_selection,
            Some(session_id.trim()),
            session_context.usage_profile.as_deref(),
            Some(session_context.queue_pressure),
        ))
    }
}

fn workflow_runtime_preflight_from_decision(
    decision: &WorkflowTechnicalFitDecision,
    required_backends: &[String],
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> WorkflowRuntimePreflightAssessment {
    let decision = decision.normalized();
    let required_backend_key = decision
        .selected_backend_key
        .clone()
        .or_else(|| {
            required_backends
                .iter()
                .find_map(|backend_key| normalize_backend_key(Some(backend_key)))
        })
        .unwrap_or_else(|| "runtime".to_string());

    let runtime = find_runtime_capability_for_decision(
        &decision,
        &required_backend_key,
        runtime_capabilities,
    );
    let runtime_id = decision
        .selected_runtime_id
        .clone()
        .or_else(|| runtime.as_ref().map(|runtime| runtime.runtime_id.clone()))
        .or_else(|| decision.selected_candidate_id.clone())
        .unwrap_or_else(|| required_backend_key.clone());
    let display_name = runtime
        .as_ref()
        .map(|runtime| runtime.display_name.clone())
        .or_else(|| decision.selected_backend_key.clone())
        .unwrap_or_else(|| required_backend_key.clone());

    let mut runtime_warnings = Vec::new();
    let mut blocking_runtime_issues = Vec::new();

    if decision.selected_runtime_id.is_some() {
        if decision.selection_mode == WorkflowTechnicalFitSelectionMode::ConservativeFallback
            || decision.reasons.iter().any(|reason| {
                matches!(
                    reason.code,
                    WorkflowTechnicalFitReasonCode::MissingCandidateData
                        | WorkflowTechnicalFitReasonCode::MissingRuntimeState
                        | WorkflowTechnicalFitReasonCode::ConservativeFallback
                )
            })
        {
            runtime_warnings.push(WorkflowRuntimeIssue {
                runtime_id,
                display_name,
                required_backend_key,
                message: describe_technical_fit_warning(&decision),
            });
        }
    } else {
        let issue = WorkflowRuntimeIssue {
            runtime_id,
            display_name,
            required_backend_key,
            message: describe_technical_fit_blocking_issue(&decision),
        };
        runtime_warnings.push(issue.clone());
        blocking_runtime_issues.push(issue);
    }

    WorkflowRuntimePreflightAssessment {
        technical_fit_decision: Some(decision),
        runtime_warnings,
        blocking_runtime_issues,
    }
}

fn find_runtime_capability_for_decision(
    decision: &WorkflowTechnicalFitDecision,
    required_backend_key: &str,
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> Option<WorkflowRuntimeCapability> {
    let selected_runtime_id = decision.selected_runtime_id.as_deref();
    let selected_backend_key = decision
        .selected_backend_key
        .as_deref()
        .unwrap_or(required_backend_key);
    let normalized_backend_key = canonical_runtime_backend_key(selected_backend_key);

    runtime_capabilities
        .iter()
        .find(|runtime| {
            selected_runtime_id == Some(runtime.runtime_id.as_str())
                || canonical_runtime_backend_key(&runtime.runtime_id) == normalized_backend_key
                || runtime.backend_keys.iter().any(|backend_key| {
                    canonical_runtime_backend_key(backend_key) == normalized_backend_key
                })
        })
        .cloned()
}

fn describe_technical_fit_warning(decision: &WorkflowTechnicalFitDecision) -> String {
    let target = decision
        .selected_backend_key
        .as_deref()
        .or(decision.selected_runtime_id.as_deref())
        .or(decision.selected_candidate_id.as_deref())
        .unwrap_or("runtime");
    if decision.selection_mode == WorkflowTechnicalFitSelectionMode::ConservativeFallback {
        format!(
            "technical-fit selected '{}' conservatively because candidate or runtime state is partial",
            target
        )
    } else {
        format!(
            "technical-fit selected '{}' with backend-owned runtime facts",
            target
        )
    }
}

fn describe_technical_fit_blocking_issue(decision: &WorkflowTechnicalFitDecision) -> String {
    let target = decision
        .selected_backend_key
        .as_deref()
        .or(decision.selected_runtime_id.as_deref())
        .or(decision.selected_candidate_id.as_deref())
        .unwrap_or("runtime");

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
        )
    }) {
        return format!(
            "technical-fit could not satisfy the explicit backend override for '{}'",
            target
        );
    }

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::ExplicitModelOverride
        )
    }) {
        return format!(
            "technical-fit could not satisfy the explicit model override for '{}'",
            target
        );
    }

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::RequiredContextLength
        )
    }) {
        return format!(
            "technical-fit found no candidate for '{}' with sufficient context length",
            target
        );
    }

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::MissingRuntimeState
                | WorkflowTechnicalFitReasonCode::MissingCandidateData
                | WorkflowTechnicalFitReasonCode::ConservativeFallback
        )
    }) {
        return format!(
            "technical-fit could not select a ready runtime for '{}' because runtime or candidate state is incomplete",
            target
        );
    }

    format!(
        "technical-fit could not select a ready runtime for '{}'",
        target
    )
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

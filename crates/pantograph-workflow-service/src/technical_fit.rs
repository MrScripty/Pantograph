use pantograph_runtime_identity::canonical_runtime_backend_key;
use serde::{Deserialize, Serialize};

use crate::workflow::{
    evaluate_runtime_preflight, runtime_issue_for_capability, validate_workflow_id, WorkflowHost,
    WorkflowHostCapabilities, WorkflowRuntimeCapability, WorkflowRuntimeIssue,
    WorkflowRuntimeRequirements, WorkflowService, WorkflowServiceError,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl WorkflowTechnicalFitOverride {
    pub fn normalized(&self) -> Option<Self> {
        let runtime_id = normalize_trimmed_string(self.runtime_id.as_deref());
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitDeviceClass {
    Cpu,
    Cuda,
    Metal,
    Mps,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum WorkflowTechnicalFitDevicePolicy {
    Auto,
    Explicit {
        device_class: WorkflowTechnicalFitDeviceClass,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
    },
}

impl WorkflowTechnicalFitDevicePolicy {
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
pub struct WorkflowTechnicalFitResourceEstimate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_peak_ram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_min_vram_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_min_ram_mb: Option<u64>,
}

impl WorkflowTechnicalFitResourceEstimate {
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
pub struct WorkflowTechnicalFitObservedThroughputHint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_second_milli: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images_per_second_milli: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<u64>,
}

impl WorkflowTechnicalFitObservedThroughputHint {
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
pub enum WorkflowTechnicalFitDeviceDiagnosticSeverity {
    Advisory,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitDeviceDiagnosticCode {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitDeviceDiagnostic {
    pub code: WorkflowTechnicalFitDeviceDiagnosticCode,
    pub severity: WorkflowTechnicalFitDeviceDiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<WorkflowTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

impl WorkflowTechnicalFitDeviceDiagnostic {
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
    pub device_policy: Option<WorkflowTechnicalFitDevicePolicy>,
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
            device_policy: self
                .device_policy
                .as_ref()
                .map(WorkflowTechnicalFitDevicePolicy::normalized),
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
        device_policy: None,
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitReasonCode {
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
pub struct WorkflowTechnicalFitCandidateSetSummary {
    #[serde(default)]
    pub total_candidate_count: u32,
    #[serde(default)]
    pub eligible_candidate_count: u32,
    #[serde(default)]
    pub rejected_candidate_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eligible_candidate_ids: Vec<String>,
}

impl WorkflowTechnicalFitCandidateSetSummary {
    pub fn normalized(&self) -> Self {
        Self {
            total_candidate_count: self.total_candidate_count,
            eligible_candidate_count: self.eligible_candidate_count,
            rejected_candidate_count: self.rejected_candidate_count,
            eligible_candidate_ids: normalize_string_list(&self.eligible_candidate_ids),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitPolicyPhase {
    CandidateRanking,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitDecisionCode {
    SelectedCandidate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTechnicalFitHistoryThresholdState {
    NotEvaluated,
    InsufficientSamples,
    Evaluated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitSelectionPolicyTrace {
    pub policy_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_phase: Option<WorkflowTechnicalFitPolicyPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_code: Option<WorkflowTechnicalFitDecisionCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_threshold_state: Option<WorkflowTechnicalFitHistoryThresholdState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_set_summary: Option<WorkflowTechnicalFitCandidateSetSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranking_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploration_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_basis: Option<String>,
}

impl WorkflowTechnicalFitSelectionPolicyTrace {
    pub fn normalized(&self) -> Self {
        Self {
            policy_version: self.policy_version,
            policy_phase: self.policy_phase,
            decision_code: self.decision_code,
            history_threshold_state: self.history_threshold_state,
            candidate_set_summary: self
                .candidate_set_summary
                .as_ref()
                .map(WorkflowTechnicalFitCandidateSetSummary::normalized),
            ranking_reason: normalize_trimmed_string(self.ranking_reason.as_deref()),
            exploration_reason: normalize_trimmed_string(self.exploration_reason.as_deref()),
            seed_basis: normalize_trimmed_string(self.seed_basis.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTechnicalFitCompatibilityReport {
    pub status: String,
    pub compatible: bool,
    pub task: String,
    pub model_source: String,
    pub preprocessing: String,
    pub postprocessing: String,
}

impl WorkflowTechnicalFitCompatibilityReport {
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
pub struct WorkflowTechnicalFitCompatibilityIssue {
    pub kind: String,
    pub phase: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl WorkflowTechnicalFitCompatibilityIssue {
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
    pub selected_runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_class: Option<WorkflowTechnicalFitDeviceClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_estimate: Option<WorkflowTechnicalFitResourceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_throughput_hint: Option<WorkflowTechnicalFitObservedThroughputHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_diagnostics: Vec<WorkflowTechnicalFitDeviceDiagnostic>,
    #[serde(default)]
    pub reasons: Vec<WorkflowTechnicalFitReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_policy_trace: Option<WorkflowTechnicalFitSelectionPolicyTrace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_report: Option<WorkflowTechnicalFitCompatibilityReport>,
    #[serde(default)]
    pub compatibility_issue_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_issues: Vec<WorkflowTechnicalFitCompatibilityIssue>,
}

impl WorkflowTechnicalFitDecision {
    pub fn normalized(&self) -> Self {
        Self {
            selection_mode: self.selection_mode,
            selected_candidate_id: normalize_trimmed_string(self.selected_candidate_id.as_deref()),
            selected_runtime_id: normalize_trimmed_string(self.selected_runtime_id.as_deref()),
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
                .and_then(WorkflowTechnicalFitResourceEstimate::normalized),
            observed_throughput_hint: self
                .observed_throughput_hint
                .as_ref()
                .and_then(WorkflowTechnicalFitObservedThroughputHint::normalized),
            device_diagnostics: self
                .device_diagnostics
                .iter()
                .map(WorkflowTechnicalFitDeviceDiagnostic::normalized)
                .collect(),
            reasons: self.reasons.clone(),
            selection_policy_trace: self
                .selection_policy_trace
                .as_ref()
                .map(WorkflowTechnicalFitSelectionPolicyTrace::normalized),
            compatibility_report: self
                .compatibility_report
                .as_ref()
                .map(WorkflowTechnicalFitCompatibilityReport::normalized),
            compatibility_issue_count: self.compatibility_issue_count,
            compatibility_issues: self
                .compatibility_issues
                .iter()
                .map(WorkflowTechnicalFitCompatibilityIssue::normalized)
                .collect(),
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

    pub(crate) async fn workflow_execution_session_runtime_preflight_assessment<H: WorkflowHost>(
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
                &capabilities.runtime_requirements.required_models,
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

    pub async fn workflow_execution_session_technical_fit_request<H: WorkflowHost>(
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
    required_models: &[String],
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> WorkflowRuntimePreflightAssessment {
    let decision = decision.normalized();
    if decision_conflicts_with_required_backends(&decision, required_backends) {
        let (runtime_warnings, blocking_runtime_issues) =
            evaluate_runtime_preflight(required_backends, runtime_capabilities);
        return WorkflowRuntimePreflightAssessment {
            technical_fit_decision: Some(decision),
            runtime_warnings,
            blocking_runtime_issues,
        };
    }

    let enforce_runtime_readiness =
        decision_enforces_runtime_readiness(&decision, required_backends, required_models);
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

    if decision_has_blocking_device_diagnostic(&decision)
        || decision_has_incomplete_runtime_state(&decision)
    {
        let issue = WorkflowRuntimeIssue {
            runtime_id,
            display_name,
            required_backend_key,
            message: describe_technical_fit_blocking_issue(&decision),
        };
        runtime_warnings.push(issue.clone());
        blocking_runtime_issues.push(issue);
        return WorkflowRuntimePreflightAssessment {
            technical_fit_decision: Some(decision),
            runtime_warnings,
            blocking_runtime_issues,
        };
    }

    if !enforce_runtime_readiness {
        return WorkflowRuntimePreflightAssessment {
            technical_fit_decision: Some(decision),
            runtime_warnings,
            blocking_runtime_issues,
        };
    }

    if let Some(runtime) = runtime.as_ref() {
        if !(runtime.available && runtime.configured) {
            let issue = runtime_issue_for_capability(runtime, &required_backend_key);
            runtime_warnings.push(issue.clone());
            blocking_runtime_issues.push(issue);
            return WorkflowRuntimePreflightAssessment {
                technical_fit_decision: Some(decision),
                runtime_warnings,
                blocking_runtime_issues,
            };
        }
    }

    if decision.selected_runtime_id.is_none() {
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

fn decision_has_incomplete_runtime_state(decision: &WorkflowTechnicalFitDecision) -> bool {
    decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::MissingCandidateData
                | WorkflowTechnicalFitReasonCode::MissingRuntimeState
        )
    })
}

fn decision_has_blocking_device_diagnostic(decision: &WorkflowTechnicalFitDecision) -> bool {
    blocking_device_diagnostic(decision).is_some()
}

fn blocking_device_diagnostic(
    decision: &WorkflowTechnicalFitDecision,
) -> Option<&WorkflowTechnicalFitDeviceDiagnostic> {
    decision.device_diagnostics.iter().find(|diagnostic| {
        diagnostic.severity == WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
    })
}

fn decision_conflicts_with_required_backends(
    decision: &WorkflowTechnicalFitDecision,
    required_backends: &[String],
) -> bool {
    let Some(selected_backend_key) = decision
        .selected_backend_key
        .as_deref()
        .and_then(|backend| normalize_backend_key(Some(backend)))
    else {
        return false;
    };

    let required_backend_keys = required_backends
        .iter()
        .filter_map(|backend| normalize_backend_key(Some(backend)))
        .collect::<Vec<_>>();
    !required_backend_keys.is_empty()
        && !required_backend_keys
            .iter()
            .any(|required| required == &selected_backend_key)
}

fn decision_enforces_runtime_readiness(
    decision: &WorkflowTechnicalFitDecision,
    required_backends: &[String],
    _required_models: &[String],
) -> bool {
    if required_backends
        .iter()
        .any(|backend| !backend.trim().is_empty())
    {
        return true;
    }

    if decision.selected_model_id.is_some() {
        return true;
    }

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
                | WorkflowTechnicalFitReasonCode::ExplicitModelOverride
                | WorkflowTechnicalFitReasonCode::ExplicitRuntimeOverride
                | WorkflowTechnicalFitReasonCode::ExplicitRuntimeVariantOverride
        )
    }) {
        return true;
    }

    false
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

fn describe_technical_fit_blocking_issue(decision: &WorkflowTechnicalFitDecision) -> String {
    let target = decision
        .selected_backend_key
        .as_deref()
        .or(decision.selected_runtime_id.as_deref())
        .or(decision.selected_candidate_id.as_deref())
        .unwrap_or("runtime");

    if let Some(diagnostic) = blocking_device_diagnostic(decision) {
        let diagnostic_target = diagnostic
            .device_id
            .as_deref()
            .or(diagnostic.runtime_variant_id.as_deref())
            .or(diagnostic.backend_key.as_deref())
            .or(decision.selected_device_id.as_deref())
            .unwrap_or(target);
        if diagnostic.code == WorkflowTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable {
            return format!(
                "technical-fit could not satisfy the explicit device policy for '{}'",
                diagnostic_target
            );
        }

        if diagnostic.message.is_empty() {
            return format!(
                "technical-fit reported a blocking device diagnostic for '{}'",
                diagnostic_target
            );
        }

        return format!(
            "technical-fit reported a blocking device diagnostic for '{}': {}",
            diagnostic_target, diagnostic.message
        );
    }

    if decision.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WorkflowTechnicalFitReasonCode::ExplicitRuntimeOverride
                | WorkflowTechnicalFitReasonCode::ExplicitRuntimeVariantOverride
        )
    }) {
        return format!(
            "technical-fit could not satisfy the explicit runtime override for '{}'",
            target
        );
    }

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

fn normalize_string_list(values: &[String]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .filter_map(|value| normalize_trimmed_string(Some(value)))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
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
    use crate::workflow::{
        WorkflowRuntimeInstallState, WorkflowRuntimeReadinessState, WorkflowRuntimeSourceKind,
    };

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

    fn unavailable_candle_runtime() -> WorkflowRuntimeCapability {
        WorkflowRuntimeCapability {
            runtime_id: "candle".to_string(),
            display_name: "Candle".to_string(),
            install_state: WorkflowRuntimeInstallState::SystemProvided,
            available: true,
            configured: false,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Host,
            selected: true,
            readiness_state: Some(WorkflowRuntimeReadinessState::Failed),
            selected_version: None,
            supports_external_connection: false,
            backend_capability_facts: None,
            backend_keys: vec!["candle".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: Some(
                "Candle backend has a staged embedding load planner but executable model loading is not implemented"
                    .to_string(),
            ),
        }
    }

    fn ready_llama_runtime() -> WorkflowRuntimeCapability {
        WorkflowRuntimeCapability {
            runtime_id: "llama_cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Managed,
            selected: false,
            readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
            selected_version: Some("test".to_string()),
            supports_external_connection: false,
            backend_capability_facts: None,
            backend_keys: vec!["llama_cpp".to_string(), "llamacpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }
    }

    #[test]
    fn build_workflow_technical_fit_request_normalizes_inputs() {
        let request = build_workflow_technical_fit_request(
            " workflow-a ",
            &runtime_requirements(),
            Some(WorkflowTechnicalFitOverride {
                runtime_id: Some(" runtime-a ".to_string()),
                runtime_variant_id: Some(" pytorch.cuda ".to_string()),
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
                runtime_id: Some("runtime-a".to_string()),
                runtime_variant_id: Some("pytorch.cuda".to_string()),
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            })
        );
    }

    #[test]
    fn workflow_technical_fit_request_normalizes_device_policy_intent() {
        let request = WorkflowTechnicalFitRequest {
            workflow_id: " workflow-a ".to_string(),
            runtime_requirements: runtime_requirements(),
            override_selection: None,
            device_policy: Some(WorkflowTechnicalFitDevicePolicy::Explicit {
                device_class: WorkflowTechnicalFitDeviceClass::Cuda,
                device_id: Some(" cuda:0 ".to_string()),
            }),
            session_id: None,
            usage_profile: None,
            queue_pressure: None,
        };

        let normalized = request.normalized();

        assert_eq!(normalized.workflow_id, "workflow-a");
        assert_eq!(
            normalized.device_policy,
            Some(WorkflowTechnicalFitDevicePolicy::Explicit {
                device_class: WorkflowTechnicalFitDeviceClass::Cuda,
                device_id: Some("cuda:0".to_string()),
            })
        );
    }

    #[test]
    fn workflow_technical_fit_decision_normalizes_selected_backend() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ExplicitOverride,
            selected_candidate_id: Some(" candidate-a ".to_string()),
            selected_runtime_id: Some("runtime-a".to_string()),
            selected_runtime_variant_id: None,
            selected_backend_key: Some("llama.cpp".to_string()),
            selected_model_id: Some(" model-a ".to_string()),
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ExplicitBackendOverride,
                Some(" candidate-a "),
            )],
            selection_policy_trace: Some(WorkflowTechnicalFitSelectionPolicyTrace {
                policy_version: 1,
                policy_phase: Some(WorkflowTechnicalFitPolicyPhase::CandidateRanking),
                decision_code: Some(WorkflowTechnicalFitDecisionCode::SelectedCandidate),
                history_threshold_state: Some(
                    WorkflowTechnicalFitHistoryThresholdState::NotEvaluated,
                ),
                candidate_set_summary: Some(WorkflowTechnicalFitCandidateSetSummary {
                    total_candidate_count: 2,
                    eligible_candidate_count: 1,
                    rejected_candidate_count: 1,
                    eligible_candidate_ids: vec![" candidate-a ".to_string()],
                }),
                ranking_reason: Some(" explicit_backend_override ".to_string()),
                exploration_reason: None,
                seed_basis: Some(" workflow-a:node-a ".to_string()),
            }),
            compatibility_report: Some(WorkflowTechnicalFitCompatibilityReport {
                status: " rejected ".to_string(),
                compatible: false,
                task: " supported ".to_string(),
                model_source: " unsupported ".to_string(),
                preprocessing: " supported ".to_string(),
                postprocessing: " supported ".to_string(),
            }),
            compatibility_issue_count: 1,
            compatibility_issues: vec![WorkflowTechnicalFitCompatibilityIssue {
                kind: " unsupported_model_artifact ".to_string(),
                phase: " model_package_resolution ".to_string(),
                message: " backend cannot load artifact ".to_string(),
                model_id: Some(" model-a ".to_string()),
                path: Some(" model.gguf ".to_string()),
            }],
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
        let trace = normalized
            .selection_policy_trace
            .as_ref()
            .expect("selection policy trace should normalize");
        assert_eq!(trace.policy_version, 1);
        assert_eq!(
            trace.policy_phase,
            Some(WorkflowTechnicalFitPolicyPhase::CandidateRanking)
        );
        assert_eq!(
            trace.decision_code,
            Some(WorkflowTechnicalFitDecisionCode::SelectedCandidate)
        );
        assert_eq!(
            trace.history_threshold_state,
            Some(WorkflowTechnicalFitHistoryThresholdState::NotEvaluated)
        );
        assert_eq!(
            trace.ranking_reason.as_deref(),
            Some("explicit_backend_override")
        );
        assert_eq!(trace.seed_basis.as_deref(), Some("workflow-a:node-a"));
        assert_eq!(
            trace
                .candidate_set_summary
                .as_ref()
                .map(|summary| summary.eligible_candidate_ids.as_slice()),
            Some(["candidate-a".to_string()].as_slice())
        );
        assert_eq!(
            normalized
                .compatibility_report
                .as_ref()
                .map(|report| (report.status.as_str(), report.model_source.as_str())),
            Some(("rejected", "unsupported"))
        );
        assert_eq!(normalized.compatibility_issue_count, 1);
        assert_eq!(
            normalized.compatibility_issues[0].kind,
            "unsupported_model_artifact"
        );
        assert_eq!(
            normalized.compatibility_issues[0].model_id.as_deref(),
            Some("model-a")
        );
    }

    #[test]
    fn technical_fit_preflight_blocks_missing_candidate_selected_backend() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candle".to_string()),
            selected_runtime_id: Some("candle".to_string()),
            selected_runtime_variant_id: None,
            selected_backend_key: Some("candle".to_string()),
            selected_model_id: None,
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::MissingCandidateData,
                Some("candle"),
            )],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        };

        let assessment = workflow_runtime_preflight_from_decision(
            &decision,
            &[],
            &["llm/gen-verse/trado-8b-instruct".to_string()],
            &[unavailable_candle_runtime()],
        );

        assert_eq!(assessment.runtime_warnings.len(), 1);
        assert_eq!(assessment.blocking_runtime_issues.len(), 1);
        assert!(assessment.blocking_runtime_issues[0]
            .message
            .contains("candidate state is incomplete"));
    }

    #[test]
    fn technical_fit_preflight_blocks_explicit_device_unavailable_diagnostic() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_runtime_variant_id: None,
            selected_backend_key: Some("pytorch".to_string()),
            selected_model_id: Some("image/model".to_string()),
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: vec![WorkflowTechnicalFitDeviceDiagnostic {
                code: WorkflowTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable,
                severity: WorkflowTechnicalFitDeviceDiagnosticSeverity::Error,
                message: "CUDA device is not available".to_string(),
                device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
                device_id: Some(" cuda:0 ".to_string()),
                runtime_variant_id: Some("pytorch.cuda".to_string()),
                backend_key: Some("pytorch".to_string()),
            }],
            reasons: Vec::new(),
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        };

        let assessment = workflow_runtime_preflight_from_decision(
            &decision,
            &[],
            &["image/model".to_string()],
            &[],
        );

        assert_eq!(assessment.runtime_warnings.len(), 1);
        assert_eq!(assessment.blocking_runtime_issues.len(), 1);
        assert!(assessment.blocking_runtime_issues[0]
            .message
            .contains("explicit device policy"));
        assert!(assessment.blocking_runtime_issues[0]
            .message
            .contains("cuda:0"));
    }

    #[test]
    fn technical_fit_preflight_blocks_model_grounded_unready_backend() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candle|llm/model".to_string()),
            selected_runtime_id: Some("candle".to_string()),
            selected_runtime_variant_id: None,
            selected_backend_key: Some("candle".to_string()),
            selected_model_id: Some("llm/model".to_string()),
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::RuntimeRequirements,
                Some("candle|llm/model"),
            )],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        };

        let assessment = workflow_runtime_preflight_from_decision(
            &decision,
            &[],
            &["llm/model".to_string()],
            &[unavailable_candle_runtime()],
        );

        assert_eq!(assessment.blocking_runtime_issues.len(), 1);
        assert!(assessment.blocking_runtime_issues[0]
            .message
            .contains("workflow requires backend 'candle'"));
    }

    #[test]
    fn technical_fit_preflight_uses_required_backend_when_decision_conflicts() {
        let decision = WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candle|vlm/qwen".to_string()),
            selected_runtime_id: Some("candle".to_string()),
            selected_runtime_variant_id: None,
            selected_backend_key: Some("candle".to_string()),
            selected_model_id: Some("vlm/qwen".to_string()),
            selected_device_class: None,
            selected_device_id: None,
            resource_estimate: None,
            observed_throughput_hint: None,
            device_diagnostics: Vec::new(),
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::RuntimeRequirements,
                Some("candle|vlm/qwen"),
            )],
            selection_policy_trace: None,
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
        };

        let assessment = workflow_runtime_preflight_from_decision(
            &decision,
            &["llama_cpp".to_string()],
            &["vlm/qwen".to_string()],
            &[unavailable_candle_runtime(), ready_llama_runtime()],
        );

        assert!(assessment.runtime_warnings.is_empty());
        assert!(assessment.blocking_runtime_issues.is_empty());
    }
}

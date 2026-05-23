use std::collections::BTreeSet;

use pantograph_dependency_planning::{
    DependencyPlanningContractError, DependencyTaskId, DeviceIntentId, PumasModelRef,
    RuntimeIntentId,
};
use serde::{Deserialize, Serialize};

use crate::dispatch::SchedulerBatchingGroupId;
use crate::error::SchedulerContractError;
use crate::intent::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};
use crate::resource_types::SchedulerModelResidencyState;

const MAX_TEXT_LEN: usize = 1024;
const MAX_INPUT_SHAPE_SIGNATURE_LEN: usize = 128;
const MAX_TIME_MS: u64 = i64::MAX as u64;

/// Current contract version for scheduler batching decisions.
pub const SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerBatchPolicyState {
    Compatible,
    WaitingForMore,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerBatchDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerBatchDiagnosticCode {
    CompatibleBatch,
    WaitingForMoreCandidates,
    IncompatibleTaskFamily,
    IncompatibleModelRef,
    IncompatibleRuntime,
    IncompatibleDeviceSet,
    IncompatibleInputShape,
    MemoryImpactOverflow,
    FairnessPolicyTrace,
    SchedulerBatchPolicyError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerBatchDiagnostic {
    pub severity: SchedulerBatchDiagnosticSeverity,
    pub code: SchedulerBatchDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerBatchDiagnostic {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("batch_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("batch_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerBatchMemoryImpact {
    pub incremental_bytes: u64,
    pub peak_bytes: u64,
}

impl SchedulerBatchMemoryImpact {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_positive(
            "batch_memory_impact.incremental_bytes",
            self.incremental_bytes,
        )?;
        validate_positive("batch_memory_impact.peak_bytes", self.peak_bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerBatchCandidate {
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub task_intent: SchedulableTaskIntent,
    pub task_family: DependencyTaskId,
    pub selected_runtime_id: RuntimeIntentId,
    pub selected_device_ids: Vec<DeviceIntentId>,
    pub selected_model_ref: PumasModelRef,
    pub model_residency_state: SchedulerModelResidencyState,
    pub input_shape_signature: String,
    pub estimated_latency_ms: u64,
    pub memory_impact: SchedulerBatchMemoryImpact,
}

impl SchedulerBatchCandidate {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        self.task_intent.validate()?;
        validate_candidate_correlation(self)?;
        validate_candidate_selection(self)?;
        validate_input_shape_signature(&self.input_shape_signature)?;
        validate_time_ms(
            "batch_candidate.estimated_latency_ms",
            self.estimated_latency_ms,
        )?;
        self.memory_impact.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerBatchPolicyDecision {
    #[serde(default = "default_scheduler_batching_policy_contract_version")]
    pub contract_version: u16,
    pub batching_group_id: SchedulerBatchingGroupId,
    pub state: SchedulerBatchPolicyState,
    pub max_batch_size: u64,
    pub selected_batch_size: u64,
    pub total_incremental_memory_bytes: u64,
    pub candidates: Vec<SchedulerBatchCandidate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerBatchDiagnostic>,
}

impl SchedulerBatchPolicyDecision {
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.contract_version != SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION {
            return Err(SchedulerContractError::InvalidField {
                field: "contract_version",
                reason: "unsupported scheduler batching policy contract version",
            });
        }
        validate_positive("max_batch_size", self.max_batch_size)?;
        validate_positive("selected_batch_size", self.selected_batch_size)?;
        if self.selected_batch_size > self.max_batch_size {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_batch_size",
                reason: "selected batch size must not exceed max batch size",
            });
        }
        validate_candidates(&self.candidates)?;
        if self.selected_batch_size as usize > self.candidates.len() {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_batch_size",
                reason: "selected batch size must not exceed candidate count",
            });
        }
        validate_compatible_candidates(&self.candidates)?;
        validate_memory_total(self)?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        if self.state == SchedulerBatchPolicyState::Rejected && self.diagnostics.is_empty() {
            return Err(SchedulerContractError::MissingField {
                field: "batch_policy.diagnostics",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerBatchPolicyDecision(SchedulerBatchPolicyDecision);

impl ValidatedSchedulerBatchPolicyDecision {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerBatchPolicyDecision {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerBatchPolicyDecision {
        self.0
    }
}

impl TryFrom<SchedulerBatchPolicyDecision> for ValidatedSchedulerBatchPolicyDecision {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerBatchPolicyDecision) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn validate_candidates(
    candidates: &[SchedulerBatchCandidate],
) -> Result<(), SchedulerContractError> {
    if candidates.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "batch_policy.candidates",
        });
    }
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        candidate.validate()?;
        if !seen.insert((
            candidate.workflow_run_id.as_str(),
            candidate.task_id.as_str(),
        )) {
            return Err(SchedulerContractError::InvalidField {
                field: "batch_policy.candidates",
                reason: "batch candidates must not contain duplicate workflow-run task ids",
            });
        }
    }
    Ok(())
}

fn validate_compatible_candidates(
    candidates: &[SchedulerBatchCandidate],
) -> Result<(), SchedulerContractError> {
    let Some(first) = candidates.first() else {
        return Ok(());
    };
    for candidate in &candidates[1..] {
        if candidate.task_family != first.task_family {
            return incompatible("task_family", "batch candidates must share task family");
        }
        if candidate.selected_model_ref != first.selected_model_ref {
            return incompatible(
                "selected_model_ref",
                "batch candidates must share model ref",
            );
        }
        if candidate.selected_runtime_id != first.selected_runtime_id {
            return incompatible("selected_runtime_id", "batch candidates must share runtime");
        }
        if candidate.selected_device_ids != first.selected_device_ids {
            return incompatible(
                "selected_device_ids",
                "batch candidates must share device set",
            );
        }
        if candidate.input_shape_signature != first.input_shape_signature {
            return incompatible(
                "input_shape_signature",
                "batch candidates must share input shape",
            );
        }
    }
    Ok(())
}

fn validate_memory_total(
    decision: &SchedulerBatchPolicyDecision,
) -> Result<(), SchedulerContractError> {
    let mut total = 0_u64;
    for candidate in &decision.candidates {
        total = total
            .checked_add(candidate.memory_impact.incremental_bytes)
            .ok_or(SchedulerContractError::InvalidField {
                field: "total_incremental_memory_bytes",
                reason: "batch memory impact must not overflow",
            })?;
    }
    if total != decision.total_incremental_memory_bytes {
        return Err(SchedulerContractError::InvalidField {
            field: "total_incremental_memory_bytes",
            reason: "batch memory total must equal candidate incremental bytes",
        });
    }
    Ok(())
}

fn validate_candidate_correlation(
    candidate: &SchedulerBatchCandidate,
) -> Result<(), SchedulerContractError> {
    if candidate.workflow_id != candidate.task_intent.workflow_id
        || candidate.workflow_run_id != candidate.task_intent.workflow_run_id
        || candidate.node_id != candidate.task_intent.node_id
        || candidate.task_id != candidate.task_intent.task_id
    {
        return Err(SchedulerContractError::InvalidField {
            field: "batch_candidate.task_intent",
            reason: "batch candidate correlation must match task intent",
        });
    }
    if candidate.task_family != candidate.task_intent.task_type {
        return Err(SchedulerContractError::InvalidField {
            field: "batch_candidate.task_family",
            reason: "batch candidate family must match task intent task type",
        });
    }
    Ok(())
}

fn validate_candidate_selection(
    candidate: &SchedulerBatchCandidate,
) -> Result<(), SchedulerContractError> {
    candidate
        .selected_model_ref
        .validate()
        .map_err(map_dependency_error)?;
    if candidate.selected_model_ref.model_id != candidate.task_intent.model_ref.model_id {
        return Err(SchedulerContractError::InvalidField {
            field: "batch_candidate.selected_model_ref",
            reason: "selected model ref must match task intent model id",
        });
    }
    if let Some(requested_artifact_id) = &candidate.task_intent.model_ref.selected_artifact_id {
        if Some(requested_artifact_id) != candidate.selected_model_ref.selected_artifact_id.as_ref()
        {
            return Err(SchedulerContractError::InvalidField {
                field: "batch_candidate.selected_model_ref",
                reason: "selected model artifact must satisfy task intent artifact requirement",
            });
        }
    }
    if let Some(requested_runtime_id) = &candidate.task_intent.constraints.requested_runtime_id {
        if requested_runtime_id != &candidate.selected_runtime_id {
            return Err(SchedulerContractError::InvalidField {
                field: "batch_candidate.selected_runtime_id",
                reason: "selected runtime must satisfy task intent runtime requirement",
            });
        }
    }
    if candidate.selected_device_ids.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "batch_candidate.selected_device_ids",
        });
    }
    let mut seen = BTreeSet::new();
    for device_id in &candidate.selected_device_ids {
        if !seen.insert(device_id) {
            return Err(SchedulerContractError::InvalidField {
                field: "batch_candidate.selected_device_ids",
                reason: "selected device ids must not contain duplicates",
            });
        }
    }
    if let Some(requested_device_id) = &candidate.task_intent.constraints.requested_device_id {
        if !seen.contains(requested_device_id) {
            return Err(SchedulerContractError::InvalidField {
                field: "batch_candidate.selected_device_ids",
                reason: "selected device set must satisfy task intent device requirement",
            });
        }
    }
    Ok(())
}

fn incompatible(field: &'static str, reason: &'static str) -> Result<(), SchedulerContractError> {
    Err(SchedulerContractError::InvalidField { field, reason })
}

fn validate_input_shape_signature(value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "input_shape_signature",
        });
    }
    if trimmed.len() > MAX_INPUT_SHAPE_SIGNATURE_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field: "input_shape_signature",
            max_len: MAX_INPUT_SHAPE_SIGNATURE_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(SchedulerContractError::InvalidIdentifier {
            field: "input_shape_signature",
        });
    }
    Ok(())
}

fn validate_positive(field: &'static str, value: u64) -> Result<(), SchedulerContractError> {
    if value == 0 {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "value must be greater than zero",
        });
    }
    Ok(())
}

fn validate_time_ms(field: &'static str, value: u64) -> Result<(), SchedulerContractError> {
    validate_positive(field, value)?;
    if value > MAX_TIME_MS {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "time values must fit signed ledger/resource arithmetic",
        });
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}

fn map_dependency_error(error: DependencyPlanningContractError) -> SchedulerContractError {
    match error {
        DependencyPlanningContractError::MissingField { field } => {
            SchedulerContractError::MissingField { field }
        }
        DependencyPlanningContractError::FieldTooLong { field, max_len } => {
            SchedulerContractError::FieldTooLong { field, max_len }
        }
        DependencyPlanningContractError::InvalidIdentifier { field } => {
            SchedulerContractError::InvalidIdentifier { field }
        }
        DependencyPlanningContractError::InvalidText { field } => {
            SchedulerContractError::InvalidText { field }
        }
        DependencyPlanningContractError::InvalidField { field, reason } => {
            SchedulerContractError::InvalidField { field, reason }
        }
        _ => SchedulerContractError::InvalidField {
            field: "dependency_planning",
            reason: "dependency planning contract value is invalid",
        },
    }
}

fn default_scheduler_batching_policy_contract_version() -> u16 {
    SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION
}

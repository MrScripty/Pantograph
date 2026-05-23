use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, DependencyPlanningContractError,
    DependencyPreflightResult, DependencyReadinessPolicy, ValidatedDependencyPreflightResult,
};
use serde::{Deserialize, Serialize};

use crate::error::SchedulerContractError;
use crate::intent::SchedulableTaskIntent;

const MAX_TEXT_LEN: usize = 1024;

/// Current contract version for scheduler-owned readiness admission.
pub const SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION: u16 = 1;

/// Scheduler admission state after dependency readiness policy is applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerReadinessAdmissionState {
    Ready,
    Deferred,
    TerminalFailed,
}

/// Scheduler action selected for dependency readiness admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerReadinessAdmissionAction {
    AdmitForDispatch,
    CheckDependencies,
    InstallMissingDependencies,
    Defer,
    Fail,
}

/// Diagnostic severity for scheduler readiness admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerReadinessAdmissionSeverity {
    Info,
    Warning,
    Error,
}

/// Stable diagnostic code for scheduler readiness admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerReadinessAdmissionDiagnosticCode {
    DependencyNotReady,
    DependencyUnavailable,
    DependencyPolicyRejected,
    MissingReadinessProof,
    InvalidReadinessProof,
    StaleReadinessProof,
    SchedulerPolicyError,
}

/// Bounded scheduler readiness admission diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerReadinessAdmissionDiagnostic {
    pub severity: SchedulerReadinessAdmissionSeverity,
    pub code: SchedulerReadinessAdmissionDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerReadinessAdmissionDiagnostic {
    pub(crate) fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("readiness_admission_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("readiness_admission_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Host-produced dependency readiness proof admitted by scheduler policy.
///
/// This wrapper intentionally carries the path-free dependency preflight
/// result, not `ModelRefV2`, executable load targets, or local model paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDependencyReadinessProof {
    pub preflight_result: DependencyPreflightResult,
}

impl SchedulerDependencyReadinessProof {
    pub(crate) fn validate_for_intent(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Result<(), SchedulerContractError> {
        let _validated =
            ValidatedDependencyPreflightResult::try_from(self.preflight_result.clone())
                .map_err(map_dependency_error)?;
        if self.preflight_result.readiness_state != DependencyEnvironmentReadinessState::Ready {
            return Err(SchedulerContractError::InvalidField {
                field: "readiness_proof.preflight_result.readiness_state",
                reason: "ready scheduler admission requires ready dependency preflight proof",
            });
        }
        if self.preflight_result.identity_key.model_ref != intent.model_ref {
            return Err(SchedulerContractError::InvalidField {
                field: "readiness_proof.preflight_result.identity_key.model_ref",
                reason: "readiness proof model ref must match scheduler task intent",
            });
        }
        if self.preflight_result.identity_key.task_id != intent.task_type {
            return Err(SchedulerContractError::InvalidField {
                field: "readiness_proof.preflight_result.identity_key.task_id",
                reason: "readiness proof task id must match scheduler task type",
            });
        }
        Ok(())
    }
}

/// Scheduler input for dependency readiness admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerReadinessAdmissionRequest {
    #[serde(default = "default_scheduler_readiness_admission_contract_version")]
    pub contract_version: u16,
    pub task_intent: SchedulableTaskIntent,
    pub policy: DependencyReadinessPolicy,
}

impl SchedulerReadinessAdmissionRequest {
    /// Validates this raw admission request before scheduler policy consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()
    }
}

/// Scheduler output after dependency readiness admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerReadinessAdmissionDecision {
    #[serde(default = "default_scheduler_readiness_admission_contract_version")]
    pub contract_version: u16,
    pub task_intent: SchedulableTaskIntent,
    pub policy: DependencyReadinessPolicy,
    pub action: SchedulerReadinessAdmissionAction,
    pub state: SchedulerReadinessAdmissionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_proof: Option<SchedulerDependencyReadinessProof>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerReadinessAdmissionDiagnostic>,
}

impl SchedulerReadinessAdmissionDecision {
    /// Validates this raw admission decision before host handoff consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        match self.state {
            SchedulerReadinessAdmissionState::Ready => {
                if self.action != SchedulerReadinessAdmissionAction::AdmitForDispatch {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_admission.action",
                        reason: "ready scheduler admission must admit for dispatch",
                    });
                }
                let Some(proof) = &self.readiness_proof else {
                    return Err(SchedulerContractError::MissingField {
                        field: "readiness_proof",
                    });
                };
                proof.validate_for_intent(&self.task_intent)
            }
            SchedulerReadinessAdmissionState::Deferred => {
                if self.readiness_proof.is_some() {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_proof",
                        reason: "deferred scheduler admission must not carry ready proof",
                    });
                }
                if self.diagnostics.is_empty() {
                    return Err(SchedulerContractError::MissingField {
                        field: "readiness_admission.diagnostics",
                    });
                }
                if !matches!(
                    self.action,
                    SchedulerReadinessAdmissionAction::CheckDependencies
                        | SchedulerReadinessAdmissionAction::InstallMissingDependencies
                        | SchedulerReadinessAdmissionAction::Defer
                ) {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_admission.action",
                        reason: "deferred scheduler admission requires a check, install, or defer action",
                    });
                }
                Ok(())
            }
            SchedulerReadinessAdmissionState::TerminalFailed => {
                if self.readiness_proof.is_some() {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_proof",
                        reason: "terminal scheduler admission failure must not carry ready proof",
                    });
                }
                if self.action != SchedulerReadinessAdmissionAction::Fail {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_admission.action",
                        reason: "terminal scheduler admission failure must use fail action",
                    });
                }
                if self.diagnostics.is_empty() {
                    return Err(SchedulerContractError::MissingField {
                        field: "readiness_admission.diagnostics",
                    });
                }
                Ok(())
            }
        }
    }
}

/// Validated scheduler readiness admission request for internal policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerReadinessAdmissionRequest(SchedulerReadinessAdmissionRequest);

impl ValidatedSchedulerReadinessAdmissionRequest {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerReadinessAdmissionRequest {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerReadinessAdmissionRequest {
        self.0
    }
}

impl TryFrom<SchedulerReadinessAdmissionRequest> for ValidatedSchedulerReadinessAdmissionRequest {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerReadinessAdmissionRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

/// Validated scheduler readiness admission decision for host handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerReadinessAdmissionDecision(SchedulerReadinessAdmissionDecision);

impl ValidatedSchedulerReadinessAdmissionDecision {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerReadinessAdmissionDecision {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerReadinessAdmissionDecision {
        self.0
    }
}

impl TryFrom<SchedulerReadinessAdmissionDecision> for ValidatedSchedulerReadinessAdmissionDecision {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerReadinessAdmissionDecision) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn default_scheduler_readiness_admission_contract_version() -> u16 {
    SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler readiness admission contract version",
        })
    }
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

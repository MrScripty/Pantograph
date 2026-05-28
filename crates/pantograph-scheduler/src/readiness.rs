use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, DependencyPlanningContractError,
    DependencyReadinessPolicy, DependencyReadinessProofEnvelope,
    ValidatedDependencyReadinessProofEnvelope,
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
    RetryableFailed,
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
    RetryDependencyReadiness,
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
    pub readiness_proof: Option<DependencyReadinessProofEnvelope>,
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
                validate_ready_proof_for_intent(proof, &self.task_intent)
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
            SchedulerReadinessAdmissionState::RetryableFailed => {
                if self.readiness_proof.is_some() {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_proof",
                        reason: "retryable scheduler admission failure must not carry ready proof",
                    });
                }
                if self.action != SchedulerReadinessAdmissionAction::RetryDependencyReadiness {
                    return Err(SchedulerContractError::InvalidField {
                        field: "readiness_admission.action",
                        reason: "retryable scheduler admission failure must use retry action",
                    });
                }
                if self.diagnostics.is_empty() {
                    return Err(SchedulerContractError::MissingField {
                        field: "readiness_admission.diagnostics",
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

/// Applies scheduler-owned dependency readiness policy to one task.
pub fn plan_scheduler_readiness_admission(
    request: ValidatedSchedulerReadinessAdmissionRequest,
    readiness_proof: Option<DependencyReadinessProofEnvelope>,
) -> Result<ValidatedSchedulerReadinessAdmissionDecision, SchedulerContractError> {
    let request = request.into_inner();
    let decision = match readiness_proof {
        None => build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::Deferred,
            SchedulerReadinessAdmissionAction::CheckDependencies,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyNotReady,
            "Dependency readiness has not been checked for this task.",
        ),
        Some(proof) => plan_from_readiness_proof(request, proof)?,
    };
    ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
}

pub(crate) fn validate_ready_proof_for_intent(
    proof: &DependencyReadinessProofEnvelope,
    intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    let _validated = ValidatedDependencyReadinessProofEnvelope::try_from(proof.clone())
        .map_err(map_dependency_error)?;
    if proof.preflight_result.readiness_state != DependencyEnvironmentReadinessState::Ready {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.preflight_result.readiness_state",
            reason: "ready scheduler admission requires ready dependency preflight proof",
        });
    }
    validate_proof_identity_for_intent(proof, intent)?;
    validate_proof_execution_context_for_intent(proof, intent)
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

fn plan_from_readiness_proof(
    request: SchedulerReadinessAdmissionRequest,
    proof: DependencyReadinessProofEnvelope,
) -> Result<SchedulerReadinessAdmissionDecision, SchedulerContractError> {
    let _validated = ValidatedDependencyReadinessProofEnvelope::try_from(proof.clone())
        .map_err(map_dependency_error)?;
    if validate_proof_identity_for_intent(&proof, &request.task_intent).is_err() {
        return Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::TerminalFailed,
            SchedulerReadinessAdmissionAction::Fail,
            SchedulerReadinessAdmissionDiagnosticCode::InvalidReadinessProof,
            "Dependency readiness proof does not match the scheduler task intent.",
        ));
    }
    if validate_proof_execution_context_for_intent(&proof, &request.task_intent).is_err() {
        return Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::TerminalFailed,
            SchedulerReadinessAdmissionAction::Fail,
            SchedulerReadinessAdmissionDiagnosticCode::StaleReadinessProof,
            "Dependency readiness proof execution context does not match the active scheduler task.",
        ));
    }

    match proof.preflight_result.readiness_state {
        DependencyEnvironmentReadinessState::Ready => Ok(SchedulerReadinessAdmissionDecision {
            contract_version: SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
            task_intent: request.task_intent,
            policy: request.policy,
            action: SchedulerReadinessAdmissionAction::AdmitForDispatch,
            state: SchedulerReadinessAdmissionState::Ready,
            readiness_proof: Some(proof),
            diagnostics: Vec::new(),
        }),
        DependencyEnvironmentReadinessState::Missing
            if request.policy == DependencyReadinessPolicy::AutoInstallMissing =>
        {
            Ok(build_non_ready_decision(
                request,
                SchedulerReadinessAdmissionState::Deferred,
                SchedulerReadinessAdmissionAction::InstallMissingDependencies,
                SchedulerReadinessAdmissionDiagnosticCode::DependencyNotReady,
                "Dependency readiness is missing; scheduler policy selected install.",
            ))
        }
        DependencyEnvironmentReadinessState::Unknown
        | DependencyEnvironmentReadinessState::Resolved => Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::Deferred,
            SchedulerReadinessAdmissionAction::CheckDependencies,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyNotReady,
            "Dependency readiness requires another scheduler-owned check.",
        )),
        DependencyEnvironmentReadinessState::Missing => Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::Deferred,
            SchedulerReadinessAdmissionAction::Defer,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyPolicyRejected,
            "Dependency readiness is missing and scheduler policy does not allow install.",
        )),
        DependencyEnvironmentReadinessState::Failed => Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::RetryableFailed,
            SchedulerReadinessAdmissionAction::RetryDependencyReadiness,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyUnavailable,
            "Dependency readiness check failed and may be retried by scheduler policy.",
        )),
        DependencyEnvironmentReadinessState::Unavailable
        | DependencyEnvironmentReadinessState::Invalid
        | DependencyEnvironmentReadinessState::NotImplemented => Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::TerminalFailed,
            SchedulerReadinessAdmissionAction::Fail,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyUnavailable,
            "Dependency readiness cannot be satisfied for this task.",
        )),
        _ => Ok(build_non_ready_decision(
            request,
            SchedulerReadinessAdmissionState::TerminalFailed,
            SchedulerReadinessAdmissionAction::Fail,
            SchedulerReadinessAdmissionDiagnosticCode::DependencyUnavailable,
            "Dependency readiness state is not supported by scheduler policy.",
        )),
    }
}

fn validate_proof_identity_for_intent(
    proof: &DependencyReadinessProofEnvelope,
    intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    if proof.preflight_result.identity_key.model_ref != intent.model_ref {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.preflight_result.identity_key.model_ref",
            reason: "readiness proof model ref must match scheduler task intent",
        });
    }
    if proof.preflight_result.identity_key.task_id != intent.task_type {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.preflight_result.identity_key.task_id",
            reason: "readiness proof task id must match scheduler task type",
        });
    }
    Ok(())
}

fn validate_proof_execution_context_for_intent(
    proof: &DependencyReadinessProofEnvelope,
    intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    let context = &proof.execution_context;
    if context.workflow_id.as_str() != intent.workflow_id.as_str() {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.execution_context.workflow_id",
            reason: "readiness proof workflow id must match scheduler task intent",
        });
    }
    if context.workflow_run_id.as_str() != intent.workflow_run_id.as_str() {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.execution_context.workflow_run_id",
            reason: "readiness proof workflow run id must match scheduler task intent",
        });
    }
    if context.node_id.as_str() != intent.node_id.as_str() {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.execution_context.node_id",
            reason: "readiness proof node id must match scheduler task intent",
        });
    }
    if context.scheduler_task_id.as_str() != intent.task_id.as_str() {
        return Err(SchedulerContractError::InvalidField {
            field: "readiness_proof.execution_context.scheduler_task_id",
            reason: "readiness proof scheduler task id must match scheduler task intent",
        });
    }
    Ok(())
}

fn build_non_ready_decision(
    request: SchedulerReadinessAdmissionRequest,
    state: SchedulerReadinessAdmissionState,
    action: SchedulerReadinessAdmissionAction,
    code: SchedulerReadinessAdmissionDiagnosticCode,
    message: &'static str,
) -> SchedulerReadinessAdmissionDecision {
    SchedulerReadinessAdmissionDecision {
        contract_version: SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
        task_intent: request.task_intent,
        policy: request.policy,
        action,
        state,
        readiness_proof: None,
        diagnostics: vec![SchedulerReadinessAdmissionDiagnostic {
            severity: SchedulerReadinessAdmissionSeverity::Warning,
            code,
            message: message.to_string(),
            hint: None,
        }],
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

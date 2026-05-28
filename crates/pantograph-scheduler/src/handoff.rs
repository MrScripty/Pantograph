use pantograph_dependency_planning::{
    DependencyEnvironmentRef, DependencyPlanningContractError, DependencyReadinessProofEnvelope,
};
use serde::{Deserialize, Serialize};

use crate::dispatch::SchedulerDispatchDecision;
use crate::error::SchedulerContractError;
use crate::intent::SchedulableTaskIntent;
use crate::readiness::{validate_ready_proof_for_intent, SchedulerReadinessAdmissionDiagnostic};

/// Current contract version for scheduler-owned runtime handoff.
pub const SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION: u16 = 1;

/// Runtime handoff state before or after scheduler dispatch decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerRuntimeHandoffState {
    ReadinessAdmitted,
    DispatchSelected,
}

/// Non-legacy runtime handoff envelope produced after readiness admission.
///
/// This envelope is path-free. It carries scheduler-owned readiness proof and
/// optional scheduler dispatch decision facts, but never `ModelRefV2`,
/// executable Pumas load targets, local paths, or worker launch data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerRuntimeHandoff {
    #[serde(default = "default_scheduler_runtime_handoff_contract_version")]
    pub contract_version: u16,
    pub workflow_id: crate::SchedulerWorkflowId,
    pub workflow_run_id: crate::SchedulerWorkflowRunId,
    pub node_id: crate::SchedulerNodeId,
    pub task_id: crate::SchedulerTaskId,
    pub task_intent: SchedulableTaskIntent,
    pub state: SchedulerRuntimeHandoffState,
    pub readiness_proof: DependencyReadinessProofEnvelope,
    pub environment_ref: DependencyEnvironmentRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_decision: Option<SchedulerDispatchDecision>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerReadinessAdmissionDiagnostic>,
}

impl SchedulerRuntimeHandoff {
    /// Validates this raw runtime handoff before host/runtime code consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        validate_ready_proof_for_intent(&self.readiness_proof, &self.task_intent)?;
        self.environment_ref
            .validate()
            .map_err(map_dependency_error)?;
        self.validate_correlation()?;
        self.validate_environment_ref()?;
        self.validate_dispatch_state()?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }

    fn validate_correlation(&self) -> Result<(), SchedulerContractError> {
        if self.workflow_id != self.task_intent.workflow_id {
            return Err(SchedulerContractError::InvalidField {
                field: "workflow_id",
                reason: "runtime handoff workflow id must match task intent",
            });
        }
        if self.workflow_run_id != self.task_intent.workflow_run_id {
            return Err(SchedulerContractError::InvalidField {
                field: "workflow_run_id",
                reason: "runtime handoff workflow run id must match task intent",
            });
        }
        if self.node_id != self.task_intent.node_id {
            return Err(SchedulerContractError::InvalidField {
                field: "node_id",
                reason: "runtime handoff node id must match task intent",
            });
        }
        if self.task_id != self.task_intent.task_id {
            return Err(SchedulerContractError::InvalidField {
                field: "task_id",
                reason: "runtime handoff task id must match task intent",
            });
        }
        Ok(())
    }

    fn validate_environment_ref(&self) -> Result<(), SchedulerContractError> {
        let Some(proof_environment_ref) = &self.readiness_proof.preflight_result.environment_ref
        else {
            return Err(SchedulerContractError::MissingField {
                field: "readiness_proof.preflight_result.environment_ref",
            });
        };
        if proof_environment_ref != &self.environment_ref {
            return Err(SchedulerContractError::InvalidField {
                field: "environment_ref",
                reason: "runtime handoff environment ref must match readiness proof",
            });
        }
        Ok(())
    }

    fn validate_dispatch_state(&self) -> Result<(), SchedulerContractError> {
        match self.state {
            SchedulerRuntimeHandoffState::ReadinessAdmitted => {
                if self.dispatch_decision.is_some() {
                    return Err(SchedulerContractError::InvalidField {
                        field: "dispatch_decision",
                        reason: "readiness-admitted handoff must not carry dispatch decision",
                    });
                }
                Ok(())
            }
            SchedulerRuntimeHandoffState::DispatchSelected => {
                let Some(decision) = &self.dispatch_decision else {
                    return Err(SchedulerContractError::MissingField {
                        field: "dispatch_decision",
                    });
                };
                self.validate_dispatch_decision(decision)
            }
        }
    }

    fn validate_dispatch_decision(
        &self,
        decision: &SchedulerDispatchDecision,
    ) -> Result<(), SchedulerContractError> {
        decision.validate()?;
        if decision.task_intent != self.task_intent {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_decision.task_intent",
                reason: "dispatch decision task intent must match runtime handoff task intent",
            });
        }
        if decision.readiness_proof != self.readiness_proof {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_decision.readiness_proof",
                reason: "dispatch decision readiness proof must match runtime handoff proof",
            });
        }
        if decision.environment_ref != self.environment_ref {
            return Err(SchedulerContractError::InvalidField {
                field: "dispatch_decision.environment_ref",
                reason: "dispatch decision environment ref must match runtime handoff environment",
            });
        }
        Ok(())
    }
}

/// Validated scheduler runtime handoff for host/runtime consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerRuntimeHandoff(SchedulerRuntimeHandoff);

impl ValidatedSchedulerRuntimeHandoff {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerRuntimeHandoff {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerRuntimeHandoff {
        self.0
    }
}

impl TryFrom<SchedulerRuntimeHandoff> for ValidatedSchedulerRuntimeHandoff {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerRuntimeHandoff) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn default_scheduler_runtime_handoff_contract_version() -> u16 {
    SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler runtime handoff contract version",
        })
    }
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

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;

/// Current contract version for runtime session load proofs.
pub const RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION: u16 = 1;

/// Backend-owned proof that a runtime session has the requested model active.
///
/// This contract intentionally carries path-free model/artifact identity. The
/// runtime host may know executable load targets internally, but graph,
/// workflow-service, scheduler, and Tauri consumers only receive this proof.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSessionRuntimeLoadProof {
    #[serde(default = "default_runtime_session_load_proof_contract_version")]
    pub contract_version: u16,
    pub workflow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub backend_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_target_id: Option<String>,
    pub readiness_state: WorkflowSessionRuntimeLoadProofReadinessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_phase: Option<WorkflowSessionRuntimeLoadProofDiagnosticPhase>,
    #[serde(default)]
    pub requested_model_active: bool,
}

impl WorkflowSessionRuntimeLoadProof {
    pub fn validate(&self) -> Result<(), RuntimeSessionLoadProofContractError> {
        validate_contract_version(self.contract_version)?;
        validate_identifier("workflow_id", &self.workflow_id)?;
        if let Some(task_id) = self.task_id.as_deref() {
            validate_identifier("task_id", task_id)?;
        }
        validate_identifier("backend_key", &self.backend_key)?;
        if let Some(runtime_id) = self.runtime_id.as_deref() {
            validate_identifier("runtime_id", runtime_id)?;
        }
        if let Some(model_id) = self.model_id.as_deref() {
            validate_reference_identifier("model_id", model_id)?;
        }
        if let Some(artifact_id) = self.artifact_id.as_deref() {
            validate_reference_identifier("artifact_id", artifact_id)?;
        }
        if let Some(load_target_id) = self.load_target_id.as_deref() {
            validate_identifier("load_target_id", load_target_id)?;
        }
        if matches!(
            self.readiness_state,
            WorkflowSessionRuntimeLoadProofReadinessState::Ready
        ) && !self.requested_model_active
        {
            return Err(RuntimeSessionLoadProofContractError::InvalidField {
                field: "requested_model_active",
                reason: "ready runtime session load proofs must mark the requested model active",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSessionRuntimeLoadProofReadinessState {
    Ready,
    NotReady,
    Stale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSessionRuntimeLoadProofDiagnosticPhase {
    RuntimeModelLoad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedWorkflowSessionRuntimeLoadProof(WorkflowSessionRuntimeLoadProof);

impl ValidatedWorkflowSessionRuntimeLoadProof {
    #[must_use]
    pub fn as_ref(&self) -> &WorkflowSessionRuntimeLoadProof {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> WorkflowSessionRuntimeLoadProof {
        self.0
    }
}

impl TryFrom<WorkflowSessionRuntimeLoadProof> for ValidatedWorkflowSessionRuntimeLoadProof {
    type Error = RuntimeSessionLoadProofContractError;

    fn try_from(value: WorkflowSessionRuntimeLoadProof) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeSessionLoadProofContractError {
    #[error("invalid field `{field}`: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("invalid identifier field `{field}`")]
    InvalidIdentifier { field: &'static str },
}

fn default_runtime_session_load_proof_contract_version() -> u16 {
    RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), RuntimeSessionLoadProofContractError> {
    if value == RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(RuntimeSessionLoadProofContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported runtime session load proof contract version",
        })
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeSessionLoadProofContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_ID_LEN
        || trimmed
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':')))
    {
        return Err(RuntimeSessionLoadProofContractError::InvalidIdentifier { field });
    }
    Ok(())
}

fn validate_reference_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeSessionLoadProofContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_ID_LEN
        || trimmed.chars().any(|ch| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':' | '/' | '@'))
        })
    {
        return Err(RuntimeSessionLoadProofContractError::InvalidIdentifier { field });
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime_session_load_tests.rs"]
mod tests;

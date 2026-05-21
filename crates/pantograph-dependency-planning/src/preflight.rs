use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{ModelArtifactKind, PumasModelRef};
use crate::request::{
    DependencyBindingId, DependencyPlanningPlatformContext, DependencyRequirementsId,
    DependencyTaskId, DeviceIntentId, RuntimeIntentId,
};
use crate::result::DependencyPlanningDiagnostic;

/// Shared cache/activity/preflight identity for dependency planning.
///
/// This key intentionally excludes Pumas-approved local load targets. It can be
/// used by graph execution, dependency cache, activity correlation, and
/// frontend status matching without leaking executable paths back into graph or
/// node-engine identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPlanningIdentityKey {
    pub model_ref: PumasModelRef,
    pub task_id: DependencyTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<DependencyTaskId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_artifact_kind: Option<ModelArtifactKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_id: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<DeviceIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_context: Option<DependencyPlanningPlatformContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
}

impl DependencyPlanningIdentityKey {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_path_free_model_ref(&self.model_ref)?;
        Ok(())
    }
}

/// Path-free model reference produced after dependency preflight.
///
/// This is the successor for graph/node-engine dependency identity. Host
/// planning may use `DependencyPlanningResult` for Pumas-approved load-target
/// handoff, but this contract must remain path-free.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPreflightModelRef {
    #[serde(default = "default_dependency_preflight_contract_version")]
    pub contract_version: u32,
    pub identity_key: DependencyPlanningIdentityKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyPreflightModelRef {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.contract_version != 1 {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_preflight_model_ref.contract_version",
                reason: "only dependency preflight model ref contract version 1 is supported",
            });
        }
        self.identity_key.validate()
    }
}

fn default_dependency_preflight_contract_version() -> u32 {
    1
}

fn validate_path_free_model_ref(
    model_ref: &PumasModelRef,
) -> Result<(), DependencyPlanningContractError> {
    model_ref.validate()?;
    if model_ref.selected_artifact_path.is_some() {
        return Err(DependencyPlanningContractError::InvalidField {
            field: "pumas_model_ref.selected_artifact_path",
            reason: "path-free dependency identity must not carry selected artifact paths",
        });
    }
    Ok(())
}

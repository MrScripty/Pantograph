use serde::{Deserialize, Serialize};

use crate::environment::{DependencyEnvironmentReadinessState, DependencyEnvironmentRef};
use crate::error::DependencyPlanningContractError;
use crate::model_ref::{ModelArtifactKind, PumasModelRef};
use crate::request::{
    DependencyBindingId, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyRequirementsId, DependencyTaskId, SchedulerIntent,
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
    #[serde(default, skip_serializing_if = "SchedulerIntent::is_empty")]
    pub scheduler_intent: SchedulerIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_context: Option<DependencyPlanningPlatformContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
}

impl DependencyPlanningIdentityKey {
    /// Build the canonical path-free identity key for a planning request.
    ///
    /// # Errors
    ///
    /// Returns `DependencyPlanningContractError` when the planning request is
    /// invalid or contains dependency identity that cannot be used as a
    /// path-free planning key.
    pub fn from_planning_request(
        request: &DependencyPlanningRequest,
    ) -> Result<Self, DependencyPlanningContractError> {
        request.validate()?;
        let identity_key = Self {
            model_ref: request.model_ref.clone(),
            task_id: request.task_id.clone(),
            task_type: request.task_type.clone(),
            expected_artifact_kind: request.expected_artifact_kind.clone(),
            scheduler_intent: request.scheduler_intent.clone(),
            platform_context: request.platform_context.clone(),
            selected_binding_ids: request.selected_binding_ids.clone(),
        };
        identity_key.validate()?;
        Ok(identity_key)
    }

    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_path_free_model_ref(&self.model_ref)?;
        validate_unique_binding_ids(&self.selected_binding_ids)?;
        Ok(())
    }

    pub(crate) fn validate_matches_planning_request(
        &self,
        request: &DependencyPlanningRequest,
    ) -> Result<(), DependencyPlanningContractError> {
        if self.model_ref != request.model_ref {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.model_ref",
                reason: "identity key model ref must match planning request model ref",
            });
        }
        if self.task_id != request.task_id {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.task_id",
                reason: "identity key task id must match planning request task id",
            });
        }
        if self.task_type != request.task_type {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.task_type",
                reason: "identity key task type must match planning request task type",
            });
        }
        if self.expected_artifact_kind != request.expected_artifact_kind {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.expected_artifact_kind",
                reason: "identity key artifact kind must match planning request artifact kind",
            });
        }
        if self.scheduler_intent != request.scheduler_intent {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.scheduler_intent",
                reason:
                    "identity key scheduler intent must match planning request scheduler intent",
            });
        }
        if self.platform_context != request.platform_context {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.platform_context",
                reason:
                    "identity key platform context must match planning request platform context",
            });
        }
        if self.selected_binding_ids != request.selected_binding_ids {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.selected_binding_ids",
                reason:
                    "identity key selected bindings must match planning request selected bindings",
            });
        }
        Ok(())
    }
}

/// Path-free preflight request for graph/node-engine dependency identity.
///
/// This request carries graph/caller scheduler intent and dependency
/// environment identity only. It does not select executable runtime/device
/// facts and does not carry Pumas load targets, package facts, or local paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPreflightRequest {
    #[serde(default = "default_dependency_preflight_contract_version")]
    pub contract_version: u32,
    pub identity_key: DependencyPlanningIdentityKey,
    pub planning_request: DependencyPlanningRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_ref: Option<DependencyEnvironmentRef>,
}

impl DependencyPreflightRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_preflight_request.contract_version",
            "only dependency preflight request contract version 1 is supported",
        )?;
        self.identity_key.validate()?;
        validate_path_free_model_ref(&self.planning_request.model_ref)?;
        self.planning_request.validate()?;
        self.identity_key
            .validate_matches_planning_request(&self.planning_request)?;
        if self.dependency_requirements_id.is_none() {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_requirements_id",
            });
        }
        let Some(environment_ref) = &self.environment_ref else {
            return Err(DependencyPlanningContractError::MissingField {
                field: "environment_ref",
            });
        };
        environment_ref.validate()
    }
}

/// Path-free preflight result produced after dependency-environment readiness.
///
/// Ready results carry only environment identity/readiness proof and the
/// dependency requirements id. Scheduler/host execution planning remains the
/// only source of executable load targets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPreflightResult {
    #[serde(default = "default_dependency_preflight_contract_version")]
    pub contract_version: u32,
    pub identity_key: DependencyPlanningIdentityKey,
    pub readiness_state: DependencyEnvironmentReadinessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_ref: Option<DependencyEnvironmentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyPreflightResult {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_preflight_result.contract_version",
            "only dependency preflight result contract version 1 is supported",
        )?;
        self.identity_key.validate()?;
        if let Some(environment_ref) = &self.environment_ref {
            environment_ref.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        if self.readiness_state == DependencyEnvironmentReadinessState::Ready {
            if self.dependency_requirements_id.is_none() {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "dependency_requirements_id",
                });
            }
            if self.environment_ref.is_none() {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "environment_ref",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyPreflightRequest(DependencyPreflightRequest);

impl ValidatedDependencyPreflightRequest {
    pub fn into_inner(self) -> DependencyPreflightRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyPreflightRequest {
        &self.0
    }
}

impl TryFrom<DependencyPreflightRequest> for ValidatedDependencyPreflightRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyPreflightRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyPreflightRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_path_shaped_preflight_fields(&value)?;
        reject_executable_preflight_payload_fields(&value)?;
        let request: DependencyPreflightRequest = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_preflight_request",
                reason: "request JSON did not match dependency preflight contract",
            }
        })?;
        Self::try_from(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyPreflightResult(DependencyPreflightResult);

impl ValidatedDependencyPreflightResult {
    pub fn into_inner(self) -> DependencyPreflightResult {
        self.0
    }

    pub fn as_result(&self) -> &DependencyPreflightResult {
        &self.0
    }
}

impl TryFrom<DependencyPreflightResult> for ValidatedDependencyPreflightResult {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyPreflightResult) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyPreflightResult {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_path_shaped_preflight_fields(&value)?;
        reject_executable_preflight_payload_fields(&value)?;
        let result: DependencyPreflightResult = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_preflight_result",
                reason: "result JSON did not match dependency preflight contract",
            }
        })?;
        Self::try_from(result)
    }
}

fn default_dependency_preflight_contract_version() -> u32 {
    1
}

pub(crate) fn validate_contract_version(
    value: u32,
    field: &'static str,
    reason: &'static str,
) -> Result<(), DependencyPlanningContractError> {
    if value == 1 {
        Ok(())
    } else {
        Err(DependencyPlanningContractError::InvalidField { field, reason })
    }
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

fn validate_unique_binding_ids(
    selected_binding_ids: &[DependencyBindingId],
) -> Result<(), DependencyPlanningContractError> {
    let mut seen = std::collections::BTreeSet::new();
    for id in selected_binding_ids {
        if !seen.insert(id.as_str()) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.selected_binding_ids",
                reason: "selected binding ids must be unique",
            });
        }
    }
    Ok(())
}

fn reject_path_shaped_preflight_fields(
    value: &serde_json::Value,
) -> Result<(), DependencyPlanningContractError> {
    reject_path_shaped_dependency_fields(
        value,
        "dependency_preflight",
        "preflight payload must not contain path-shaped dependency identity fields",
    )
}

fn reject_executable_preflight_payload_fields(
    value: &serde_json::Value,
) -> Result<(), DependencyPlanningContractError> {
    reject_executable_dependency_payload_fields(
        value,
        "dependency_preflight",
        "preflight payload must not contain executable dependency handoff fields",
    )
}

pub(crate) fn reject_path_shaped_dependency_fields(
    value: &serde_json::Value,
    field: &'static str,
    reason: &'static str,
) -> Result<(), DependencyPlanningContractError> {
    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(key, child)| {
                matches!(
                    key.as_str(),
                    "model_path"
                        | "modelPath"
                        | "entry_path"
                        | "entryPath"
                        | "selected_artifact_path"
                        | "selectedArtifactPath"
                        | "local_load_path"
                        | "localLoadPath"
                        | "manifest_path"
                        | "manifestPath"
                ) || visit(child)
            }),
            serde_json::Value::Array(items) => items.iter().any(visit),
            _ => false,
        }
    }

    if visit(value) {
        Err(DependencyPlanningContractError::InvalidField { field, reason })
    } else {
        Ok(())
    }
}

pub(crate) fn reject_executable_dependency_payload_fields(
    value: &serde_json::Value,
    field: &'static str,
    reason: &'static str,
) -> Result<(), DependencyPlanningContractError> {
    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(key, child)| {
                matches!(
                    key.as_str(),
                    "load_target"
                        | "loadTarget"
                        | "resolved_model_package_facts"
                        | "resolvedModelPackageFacts"
                        | "model_package_facts"
                        | "modelPackageFacts"
                        | "python_executable"
                        | "pythonExecutable"
                        | "wheel_source_path"
                        | "wheelSourcePath"
                        | "package_source_path"
                        | "packageSourcePath"
                        | "package_source_override"
                        | "packageSourceOverride"
                ) || visit(child)
            }),
            serde_json::Value::Array(items) => items.iter().any(visit),
            _ => false,
        }
    }

    if visit(value) {
        Err(DependencyPlanningContractError::InvalidField { field, reason })
    } else {
        Ok(())
    }
}

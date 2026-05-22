use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::preflight::{
    reject_executable_dependency_payload_fields, reject_path_shaped_dependency_fields,
    validate_contract_version, DependencyPlanningIdentityKey,
};
use crate::request::DependencyPlanningRequest;

/// Host-owned dependency readiness policy for the current run.
///
/// This policy tells the host whether it may prepare missing dependencies. It
/// does not select runtimes, devices, Pumas artifacts, or executable paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyReadinessPolicy {
    CheckOnly,
    AutoInstallMissing,
}

/// Path-free host input for producing dependency preflight readiness proof.
///
/// The host consumes this request, resolves/checks/install dependencies through
/// implementation-owned services, and returns `DependencyPreflightResult`.
/// Readiness proof fields are intentionally absent from this input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyReadinessRequest {
    #[serde(default = "default_dependency_readiness_contract_version")]
    pub contract_version: u32,
    pub identity_key: DependencyPlanningIdentityKey,
    pub planning_request: DependencyPlanningRequest,
    pub policy: DependencyReadinessPolicy,
}

impl DependencyReadinessRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_readiness_request.contract_version",
            "only dependency readiness request contract version 1 is supported",
        )?;
        self.identity_key.validate()?;
        self.planning_request.validate()?;
        self.identity_key
            .validate_matches_planning_request(&self.planning_request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyReadinessRequest(DependencyReadinessRequest);

impl ValidatedDependencyReadinessRequest {
    pub fn into_inner(self) -> DependencyReadinessRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyReadinessRequest {
        &self.0
    }
}

impl TryFrom<DependencyReadinessRequest> for ValidatedDependencyReadinessRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyReadinessRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyReadinessRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_path_shaped_dependency_fields(
            &value,
            "dependency_readiness",
            "readiness payload must not contain path-shaped dependency identity fields",
        )?;
        reject_executable_dependency_payload_fields(
            &value,
            "dependency_readiness",
            "readiness payload must not contain executable dependency handoff fields",
        )?;
        let request: DependencyReadinessRequest = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_readiness_request",
                reason: "request JSON did not match dependency readiness contract",
            }
        })?;
        Self::try_from(request)
    }
}

fn default_dependency_readiness_contract_version() -> u32 {
    1
}

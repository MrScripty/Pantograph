use std::fmt;
use std::str::FromStr;

use pantograph_dependency_planning::{
    DependencyEnvironmentRef, DependencyReadinessProofEnvelope, DeviceIntentId, PumasModelRef,
    RuntimeIntentId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::dispatch::{
    SchedulerBatchingGroupId, SchedulerDispatchDecision, SchedulerRuntimeVariantId,
};
use crate::dispatch_selection_validation::{
    default_scheduler_dispatch_selection_contract_version, map_dependency_error,
    validate_candidate_selected_model_ref, validate_contract_version, validate_environment_ref,
    validate_identifier, validate_reservation, validate_resource_fit, validate_selected_device_ids,
    validate_text,
};
use crate::error::SchedulerContractError;
use crate::intent::{SchedulableTaskIntent, SchedulerTraitSetting};
use crate::readiness::validate_ready_proof_for_intent;
use crate::resource::{SchedulerResourceFitAssessment, SchedulerResourceReservation};

/// Current contract version for scheduler dispatch selection.
pub const SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct SchedulerDispatchCandidateId(String);

impl SchedulerDispatchCandidateId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, SchedulerContractError> {
        validate_identifier("candidate_id", value.as_ref()).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SchedulerDispatchCandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SchedulerDispatchCandidateId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for SchedulerDispatchCandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for SchedulerDispatchCandidateId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for SchedulerDispatchCandidateId {
    type Err = SchedulerContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for SchedulerDispatchCandidateId {
    type Error = SchedulerContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for SchedulerDispatchCandidateId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SchedulerDispatchCandidateId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerDispatchSelectionState {
    Selected,
    NoSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerDispatchSelectionDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerDispatchSelectionDiagnosticCode {
    CandidateSelected,
    NoCandidates,
    IncompatibleRuntimeRequirement,
    IncompatibleDeviceRequirement,
    MissingReservation,
    MissingResourceFit,
    ResourceFitRejected,
    InvalidCandidateEvidence,
    DuplicateCandidateId,
    AmbiguousRanking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchSelectionDiagnostic {
    pub severity: SchedulerDispatchSelectionDiagnosticSeverity,
    pub code: SchedulerDispatchSelectionDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<SchedulerDispatchCandidateId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerDispatchSelectionDiagnostic {
    pub(crate) fn error(
        code: SchedulerDispatchSelectionDiagnosticCode,
        candidate_id: Option<&SchedulerDispatchCandidateId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: SchedulerDispatchSelectionDiagnosticSeverity::Error,
            code,
            message: message.into(),
            candidate_id: candidate_id.cloned(),
            hint: None,
        }
    }

    pub(crate) fn info(
        code: SchedulerDispatchSelectionDiagnosticCode,
        candidate_id: Option<&SchedulerDispatchCandidateId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
            code,
            message: message.into(),
            candidate_id: candidate_id.cloned(),
            hint: None,
        }
    }

    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("dispatch_selection_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("dispatch_selection_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchCandidate {
    pub candidate_id: SchedulerDispatchCandidateId,
    pub selected_runtime_id: RuntimeIntentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_variant_id: Option<SchedulerRuntimeVariantId>,
    pub selected_device_ids: Vec<DeviceIntentId>,
    pub selected_model_ref: PumasModelRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_trait_settings: Vec<SchedulerTraitSetting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservation: Option<SchedulerResourceReservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_fit_assessment: Option<SchedulerResourceFitAssessment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batching_group_id: Option<SchedulerBatchingGroupId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_source_diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
}

impl SchedulerDispatchCandidate {
    fn validate(&self, intent: &SchedulableTaskIntent) -> Result<(), SchedulerContractError> {
        self.selected_model_ref
            .validate()
            .map_err(map_dependency_error)?;
        validate_selected_device_ids(&self.selected_device_ids)?;
        validate_candidate_selected_model_ref(self, intent)?;
        for trait_setting in &self.runtime_trait_settings {
            trait_setting
                .value
                .validate_for_capability_hint()
                .map_err(|_| SchedulerContractError::InvalidField {
                    field: "dispatch_candidate.runtime_trait_settings",
                    reason: "runtime trait setting is invalid",
                })?;
        }
        if let Some(reservation) = &self.reservation {
            validate_reservation(self, intent, reservation)?;
        }
        if let Some(resource_fit_assessment) = &self.resource_fit_assessment {
            validate_resource_fit(intent, resource_fit_assessment)?;
        }
        for diagnostic in &self.candidate_source_diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchSelectionRequest {
    #[serde(default = "default_scheduler_dispatch_selection_contract_version")]
    pub contract_version: u16,
    pub task_intent: SchedulableTaskIntent,
    pub readiness_proof: DependencyReadinessProofEnvelope,
    pub environment_ref: DependencyEnvironmentRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<SchedulerDispatchCandidate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
}

impl SchedulerDispatchSelectionRequest {
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        validate_ready_proof_for_intent(&self.readiness_proof, &self.task_intent)?;
        self.environment_ref
            .validate()
            .map_err(map_dependency_error)?;
        validate_environment_ref(&self.readiness_proof, &self.environment_ref)?;
        for candidate in &self.candidates {
            candidate.validate(&self.task_intent)?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerDispatchSelectionRequest(SchedulerDispatchSelectionRequest);

impl ValidatedSchedulerDispatchSelectionRequest {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerDispatchSelectionRequest {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerDispatchSelectionRequest {
        self.0
    }
}

impl TryFrom<SchedulerDispatchSelectionRequest> for ValidatedSchedulerDispatchSelectionRequest {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerDispatchSelectionRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchSelectionDecision {
    #[serde(default = "default_scheduler_dispatch_selection_contract_version")]
    pub contract_version: u16,
    pub task_intent: SchedulableTaskIntent,
    pub state: SchedulerDispatchSelectionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_decision: Option<SchedulerDispatchDecision>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
}

impl SchedulerDispatchSelectionDecision {
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        match self.state {
            SchedulerDispatchSelectionState::Selected => {
                let Some(dispatch_decision) = &self.dispatch_decision else {
                    return Err(SchedulerContractError::MissingField {
                        field: "dispatch_decision",
                    });
                };
                dispatch_decision.validate()?;
                if dispatch_decision.task_intent != self.task_intent {
                    return Err(SchedulerContractError::InvalidField {
                        field: "dispatch_decision.task_intent",
                        reason: "dispatch decision task intent must match selection task intent",
                    });
                }
                Ok(())
            }
            SchedulerDispatchSelectionState::NoSelection => {
                if self.dispatch_decision.is_some() {
                    return Err(SchedulerContractError::InvalidField {
                        field: "dispatch_decision",
                        reason: "no-selection dispatch result must not carry a dispatch decision",
                    });
                }
                if self.diagnostics.is_empty() {
                    return Err(SchedulerContractError::MissingField {
                        field: "dispatch_selection.diagnostics",
                    });
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerDispatchSelectionDecision(SchedulerDispatchSelectionDecision);

impl ValidatedSchedulerDispatchSelectionDecision {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerDispatchSelectionDecision {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerDispatchSelectionDecision {
        self.0
    }
}

impl TryFrom<SchedulerDispatchSelectionDecision> for ValidatedSchedulerDispatchSelectionDecision {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerDispatchSelectionDecision) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

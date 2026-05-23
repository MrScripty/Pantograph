use pantograph_dependency_planning::{
    DependencyPlanningContractError, DependencyTaskId, DeviceIntentId, PumasModelRef,
    RuntimeIntentId,
};
use serde::{Deserialize, Serialize};

use crate::error::SchedulerContractError;
use crate::intent::{SchedulerTraitId, SchedulerTraitValue};

const MAX_TEXT_LEN: usize = 1024;

/// Current contract version for backend-owned scheduler capability hints.
pub const SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION: u16 = 1;

/// Availability states shown to graph editor and option-provider consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityAvailabilityState {
    Available,
    Unavailable,
    NotInstalled,
    NotImplemented,
    Stale,
    Invalid,
    Ambiguous,
}

/// Diagnostic severity for capability hints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerCapabilitySeverity {
    Info,
    Warning,
    Error,
}

/// Stable diagnostic code for capability hint consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerCapabilityDiagnosticCode {
    RuntimeUnavailable,
    DeviceUnavailable,
    TraitUnavailable,
    DependencyUnavailable,
    NotInstalled,
    NotImplemented,
    StaleFacts,
    InvalidFacts,
    AmbiguousFacts,
}

/// Backend-owned capability diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerCapabilityDiagnostic {
    pub severity: SchedulerCapabilitySeverity,
    pub code: SchedulerCapabilityDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerCapabilityDiagnostic {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("capability_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("capability_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Runtime capability exposed as a hint, not as a dispatch decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerRuntimeCapabilityHint {
    pub runtime_id: RuntimeIntentId,
    pub state: CapabilityAvailabilityState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerCapabilityDiagnostic>,
}

impl SchedulerRuntimeCapabilityHint {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_diagnostics(&self.diagnostics)
    }
}

/// Device capability exposed as a hint, not as a dispatch decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDeviceCapabilityHint {
    pub device_id: DeviceIntentId,
    pub state: CapabilityAvailabilityState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerCapabilityDiagnostic>,
}

impl SchedulerDeviceCapabilityHint {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_diagnostics(&self.diagnostics)
    }
}

/// One selectable value for a typed scheduler trait option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTraitOptionValue {
    pub value: SchedulerTraitValue,
    pub state: CapabilityAvailabilityState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerCapabilityDiagnostic>,
}

impl SchedulerTraitOptionValue {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        self.value.validate_for_capability_hint()?;
        if let Some(label) = &self.label {
            validate_text("trait_option_value.label", label)?;
        }
        validate_diagnostics(&self.diagnostics)
    }
}

/// Backend-owned trait option hint for graph editor controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTraitOptionHint {
    pub trait_id: SchedulerTraitId,
    pub state: CapabilityAvailabilityState,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<SchedulerTraitOptionValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerCapabilityDiagnostic>,
}

impl SchedulerTraitOptionHint {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        for value in &self.values {
            value.validate()?;
        }
        validate_diagnostics(&self.diagnostics)
    }
}

/// Backend-owned capability hint snapshot for graph/editor consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerCapabilityHintSnapshot {
    #[serde(default = "default_scheduler_capability_hint_contract_version")]
    pub contract_version: u16,
    pub task_type: DependencyTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<PumasModelRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtimes: Vec<SchedulerRuntimeCapabilityHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<SchedulerDeviceCapabilityHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_options: Vec<SchedulerTraitOptionHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerCapabilityDiagnostic>,
}

impl SchedulerCapabilityHintSnapshot {
    /// Validates this raw capability snapshot before editor/option providers consume it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.contract_version != SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION {
            return Err(SchedulerContractError::InvalidField {
                field: "contract_version",
                reason: "unsupported scheduler capability hint contract version",
            });
        }
        if let Some(model_ref) = &self.model_ref {
            model_ref.validate().map_err(map_dependency_error)?;
        }
        for runtime in &self.runtimes {
            runtime.validate()?;
        }
        for device in &self.devices {
            device.validate()?;
        }
        for option in &self.trait_options {
            option.validate()?;
        }
        validate_diagnostics(&self.diagnostics)
    }
}

fn default_scheduler_capability_hint_contract_version() -> u16 {
    SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION
}

/// Validated scheduler capability hints for graph/editor consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerCapabilityHintSnapshot(SchedulerCapabilityHintSnapshot);

impl ValidatedSchedulerCapabilityHintSnapshot {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerCapabilityHintSnapshot {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerCapabilityHintSnapshot {
        self.0
    }
}

impl TryFrom<SchedulerCapabilityHintSnapshot> for ValidatedSchedulerCapabilityHintSnapshot {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerCapabilityHintSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn validate_diagnostics(
    diagnostics: &[SchedulerCapabilityDiagnostic],
) -> Result<(), SchedulerContractError> {
    for diagnostic in diagnostics {
        diagnostic.validate()?;
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

//! Shared inference interface descriptor and validation contracts.
//!
//! This crate owns path-free DTOs for resolving model-specific inference-node
//! ports, authored graph snapshots, drift reports, and validation summaries. It
//! does not own live validation event streams, Pumas lookup, scheduler policy,
//! runtime-host execution, workflow mutation, node-engine execution, or frontend
//! rendering behavior.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

pub use pantograph_dependency_planning::{DeviceIntentId, PumasModelRef, RuntimeIntentId};

pub const INFERENCE_INTERFACE_CONTRACT_VERSION: u32 = 1;

const MAX_ID_LEN: usize = 128;
const MAX_LABEL_LEN: usize = 256;
const MAX_MESSAGE_LEN: usize = 1024;
const MAX_PORTS: usize = 128;
const MAX_OPTIONS: usize = 512;
const MAX_DIAGNOSTICS: usize = 128;
const MAX_REASONS: usize = 32;
const MAX_CHANGES: usize = 256;
const MAX_BINDINGS: usize = 256;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceContractError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("{field} contains {actual_len} items; maximum is {max_len}")]
    TooManyItems {
        field: &'static str,
        actual_len: usize,
        max_len: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("unsupported inference interface contract version {actual}; expected {expected}")]
    UnsupportedContractVersion { actual: u32, expected: u32 },
}

macro_rules! validated_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, InferenceInterfaceContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $name {
            type Err = InferenceInterfaceContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InferenceInterfaceContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_id!(InferenceTaskKind, "task_kind");
validated_id!(InferenceInterfaceFingerprint, "descriptor_fingerprint");
validated_id!(InferencePortId, "port_id");
validated_id!(InferenceOptionId, "option_id");
validated_id!(WorkflowNodeId, "node_id");
validated_id!(DraftGraphValidationSessionId, "validation_session_id");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolveInferenceInterfaceRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub model_ref: PumasModelRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_kind: Option<InferenceTaskKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_constraint: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_constraint: Option<DeviceIntentId>,
}

impl ResolveInferenceInterfaceRequest {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_contract_version(self.contract_version)?;
        validate_model_ref("request.model_ref", &self.model_ref)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceDescriptor {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub model_ref: PumasModelRef,
    pub task_kind: InferenceTaskKind,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_conditions: Vec<InferenceRuntimeCondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<InferencePortDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<InferencePortDescriptor>,
    pub availability: InferenceAvailability,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

impl InferenceInterfaceDescriptor {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_contract_version(self.contract_version)?;
        validate_model_ref("descriptor.model_ref", &self.model_ref)?;
        validate_collection_len("descriptor.inputs", self.inputs.len(), MAX_PORTS)?;
        validate_collection_len("descriptor.outputs", self.outputs.len(), MAX_PORTS)?;
        validate_collection_len(
            "descriptor.runtime_conditions",
            self.runtime_conditions.len(),
            MAX_REASONS,
        )?;
        validate_collection_len(
            "descriptor.diagnostics",
            self.diagnostics.len(),
            MAX_DIAGNOSTICS,
        )?;
        for port in self.inputs.iter().chain(self.outputs.iter()) {
            port.validate()?;
        }
        for condition in &self.runtime_conditions {
            condition.validate()?;
        }
        self.availability.validate()?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferencePortDescriptor {
    pub port_id: InferencePortId,
    pub label: String,
    pub direction: InferencePortDirection,
    pub requirement: InferencePortRequirement,
    pub value_type: InferenceValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<InferenceDefaultValue>,
    pub options: InferencePortOptions,
    pub availability: InferenceAvailability,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_conditions: Vec<InferenceRuntimeCondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

impl InferencePortDescriptor {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("port.label", &self.label, MAX_LABEL_LEN)?;
        validate_collection_len(
            "port.runtime_conditions",
            self.runtime_conditions.len(),
            MAX_REASONS,
        )?;
        validate_collection_len("port.diagnostics", self.diagnostics.len(), MAX_DIAGNOSTICS)?;
        self.options.validate()?;
        self.availability.validate()?;
        for condition in &self.runtime_conditions {
            condition.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferencePortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferencePortRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "category", content = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceValueType {
    Scalar(InferenceScalarType),
    Artifact(InferenceArtifactType),
    Reference(InferenceReferenceType),
    Constraint(InferenceConstraintType),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceScalarType {
    String,
    Bool,
    I64,
    U64,
    F64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceArtifactType {
    Image,
    Audio,
    Video,
    Tensor,
    Document,
    Media,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceReferenceType {
    PumasModel,
    MediaArtifact,
    RuntimeArtifact,
    SchedulerTaskResult,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceConstraintType {
    Runtime,
    Device,
    DenoisingScheduler,
    SamplingMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InferenceDefaultValue {
    UseBackendDefault,
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InferencePortOptions {
    None,
    Any,
    Enum { values: Vec<InferenceOptionValue> },
    NumericRange { range: InferenceNumericRange },
}

impl InferencePortOptions {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        match self {
            Self::None | Self::Any => Ok(()),
            Self::Enum { values } => {
                validate_collection_len("port.options.values", values.len(), MAX_OPTIONS)?;
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            Self::NumericRange { range } => range.validate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceOptionValue {
    pub option_id: InferenceOptionId,
    pub label: String,
    pub value: InferenceOptionScalar,
    pub availability: InferenceAvailability,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

impl InferenceOptionValue {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("option.label", &self.label, MAX_LABEL_LEN)?;
        validate_collection_len(
            "option.diagnostics",
            self.diagnostics.len(),
            MAX_DIAGNOSTICS,
        )?;
        self.availability.validate()?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InferenceOptionScalar {
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceNumericRange {
    pub min: f64,
    pub max: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
}

impl InferenceNumericRange {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        if !self.min.is_finite() || !self.max.is_finite() {
            return Err(InferenceInterfaceContractError::InvalidField {
                field: "numeric_range",
                reason: "range bounds must be finite",
            });
        }
        if self.min > self.max {
            return Err(InferenceInterfaceContractError::InvalidField {
                field: "numeric_range",
                reason: "min must not exceed max",
            });
        }
        if matches!(self.step, Some(step) if !step.is_finite() || step <= 0.0) {
            return Err(InferenceInterfaceContractError::InvalidField {
                field: "numeric_range.step",
                reason: "step must be finite and greater than zero",
            });
        }
        if matches!(self.default, Some(default) if !default.is_finite() || default < self.min || default > self.max)
        {
            return Err(InferenceInterfaceContractError::InvalidField {
                field: "numeric_range.default",
                reason: "default must be finite and within range",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceAvailability {
    pub status: InferenceAvailabilityStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<InferenceAvailabilityReason>,
}

impl InferenceAvailability {
    pub fn available() -> Self {
        Self {
            status: InferenceAvailabilityStatus::Available,
            reasons: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_collection_len("availability.reasons", self.reasons.len(), MAX_REASONS)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceAvailabilityStatus {
    Available,
    Unavailable,
    NotImplemented,
    Unsupported,
    Pending,
    Stale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceAvailabilityReason {
    MissingModelFacts,
    MissingSelectedArtifact,
    MissingRuntimeCapability,
    RuntimeNotInstalled,
    FeatureNotImplemented,
    UnsupportedModelFamily,
    UnsupportedTaskKind,
    ExplicitRuntimeInvalid,
    ExplicitDeviceInvalid,
    StaleFacts,
    DriftDetected,
    MissingRequiredInput,
    InvalidOption,
    BackendValidationPending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceRuntimeCondition {
    pub condition: InferenceRuntimeConditionKind,
    pub value: String,
}

impl InferenceRuntimeCondition {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("runtime_condition.value", &self.value, MAX_LABEL_LEN)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceRuntimeConditionKind {
    Runtime,
    Device,
    RuntimeFeature,
    ModelFamily,
    ArtifactKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceDiagnostic {
    pub severity: InferenceDiagnosticSeverity,
    pub code: InferenceDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<InferencePortId>,
}

impl InferenceInterfaceDiagnostic {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("diagnostic.message", &self.message, MAX_MESSAGE_LEN)?;
        validate_optional_text("diagnostic.hint", self.hint.as_deref(), MAX_MESSAGE_LEN)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceDiagnosticCode {
    DescriptorResolved,
    DescriptorUnavailable,
    DescriptorStale,
    UnsupportedTaskKind,
    NotImplemented,
    MissingRequiredInput,
    InvalidOption,
    InvalidRuntimeConstraint,
    InvalidDeviceConstraint,
    AlternativeAvailable,
    DriftDetected,
    GraphValidationPending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AuthoredInferenceInterfaceSnapshot {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub task_kind: InferenceTaskKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<AuthoredInferencePortSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<AuthoredInferencePortSnapshot>,
}

impl AuthoredInferenceInterfaceSnapshot {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_contract_version(self.contract_version)?;
        validate_collection_len("authored_snapshot.inputs", self.inputs.len(), MAX_PORTS)?;
        validate_collection_len("authored_snapshot.outputs", self.outputs.len(), MAX_PORTS)?;
        for port in self.inputs.iter().chain(self.outputs.iter()) {
            port.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AuthoredInferencePortSnapshot {
    pub port_id: InferencePortId,
    pub label: String,
    pub direction: InferencePortDirection,
    pub requirement: InferencePortRequirement,
    pub value_type: InferenceValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<InferenceDefaultValue>,
    pub availability: InferenceAvailability,
}

impl AuthoredInferencePortSnapshot {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("authored_port.label", &self.label, MAX_LABEL_LEN)?;
        self.availability.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceDriftReport {
    pub authored_fingerprint: InferenceInterfaceFingerprint,
    pub current_fingerprint: InferenceInterfaceFingerprint,
    pub severity: InferenceDriftSeverity,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<InferenceInterfaceDriftChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

impl InferenceInterfaceDriftReport {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_collection_len("drift_report.changes", self.changes.len(), MAX_CHANGES)?;
        validate_collection_len(
            "drift_report.diagnostics",
            self.diagnostics.len(),
            MAX_DIAGNOSTICS,
        )?;
        for change in &self.changes {
            change.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceDriftSeverity {
    None,
    Informational,
    NonBlocking,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceDriftChange {
    pub kind: InferenceInterfaceDriftChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<InferencePortId>,
    pub message: String,
}

impl InferenceInterfaceDriftChange {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_text("drift_change.message", &self.message, MAX_MESSAGE_LEN)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceInterfaceDriftChangeKind {
    PortAdded,
    PortRemoved,
    PortTypeChanged,
    RequirementChanged,
    DefaultChanged,
    OptionSetChanged,
    AvailabilityChanged,
    TaskKindChanged,
    RuntimeConditionChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ValidateInferenceNodeRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub node_id: WorkflowNodeId,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_bindings: Vec<InferenceInputBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub literal_values: Vec<InferenceLiteralValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_options: Vec<InferenceSelectedOption>,
}

impl ValidateInferenceNodeRequest {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_contract_version(self.contract_version)?;
        validate_collection_len(
            "validate_request.input_bindings",
            self.input_bindings.len(),
            MAX_BINDINGS,
        )?;
        validate_collection_len(
            "validate_request.literal_values",
            self.literal_values.len(),
            MAX_BINDINGS,
        )?;
        validate_collection_len(
            "validate_request.selected_options",
            self.selected_options.len(),
            MAX_BINDINGS,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInputBinding {
    pub port_id: InferencePortId,
    pub upstream_node_id: WorkflowNodeId,
    pub upstream_port_id: InferencePortId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceLiteralValue {
    pub port_id: InferencePortId,
    pub value: InferenceOptionScalar,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceSelectedOption {
    pub port_id: InferencePortId,
    pub option_id: InferenceOptionId,
    pub value: InferenceOptionScalar,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DraftGraphValidationSummary {
    pub status: DraftGraphValidationStatus,
    pub executable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enqueue_disabled_reasons: Vec<DraftGraphEnqueueDisabledReason>,
    pub diagnostics_count: u32,
    pub blocking_diagnostics_count: u32,
}

impl DraftGraphValidationSummary {
    pub fn validate(&self) -> Result<(), InferenceInterfaceContractError> {
        validate_collection_len(
            "validation_summary.enqueue_disabled_reasons",
            self.enqueue_disabled_reasons.len(),
            MAX_REASONS,
        )?;
        if self.executable && self.status != DraftGraphValidationStatus::Executable {
            return Err(InferenceInterfaceContractError::InvalidField {
                field: "validation_summary.executable",
                reason: "only executable summaries may set executable true",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DraftGraphValidationStatus {
    Pending,
    Stale,
    Unresolved,
    Unavailable,
    Blocked,
    Executable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DraftGraphEnqueueDisabledReason {
    ValidationPending,
    ValidationStale,
    DescriptorUnresolved,
    DescriptorUnavailable,
    BlockingDiagnostics,
    MissingRequiredInput,
    InvalidPortBinding,
    InvalidRuntimeConstraint,
    InvalidDeviceConstraint,
    DriftRequiresReview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedInferenceInterfaceDescriptor(InferenceInterfaceDescriptor);

impl ValidatedInferenceInterfaceDescriptor {
    #[must_use]
    pub fn as_descriptor(&self) -> &InferenceInterfaceDescriptor {
        &self.0
    }

    pub fn into_inner(self) -> InferenceInterfaceDescriptor {
        self.0
    }
}

impl TryFrom<InferenceInterfaceDescriptor> for ValidatedInferenceInterfaceDescriptor {
    type Error = InferenceInterfaceContractError;

    fn try_from(value: InferenceInterfaceDescriptor) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAuthoredInferenceInterfaceSnapshot(AuthoredInferenceInterfaceSnapshot);

impl ValidatedAuthoredInferenceInterfaceSnapshot {
    #[must_use]
    pub fn as_snapshot(&self) -> &AuthoredInferenceInterfaceSnapshot {
        &self.0
    }

    pub fn into_inner(self) -> AuthoredInferenceInterfaceSnapshot {
        self.0
    }
}

impl TryFrom<AuthoredInferenceInterfaceSnapshot> for ValidatedAuthoredInferenceInterfaceSnapshot {
    type Error = InferenceInterfaceContractError;

    fn try_from(value: AuthoredInferenceInterfaceSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDraftGraphValidationSummary(DraftGraphValidationSummary);

impl ValidatedDraftGraphValidationSummary {
    #[must_use]
    pub fn as_summary(&self) -> &DraftGraphValidationSummary {
        &self.0
    }

    pub fn into_inner(self) -> DraftGraphValidationSummary {
        self.0
    }
}

impl TryFrom<DraftGraphValidationSummary> for ValidatedDraftGraphValidationSummary {
    type Error = InferenceInterfaceContractError;

    fn try_from(value: DraftGraphValidationSummary) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn default_contract_version() -> u32 {
    INFERENCE_INTERFACE_CONTRACT_VERSION
}

fn validate_contract_version(version: u32) -> Result<(), InferenceInterfaceContractError> {
    if version == INFERENCE_INTERFACE_CONTRACT_VERSION {
        return Ok(());
    }
    Err(
        InferenceInterfaceContractError::UnsupportedContractVersion {
            actual: version,
            expected: INFERENCE_INTERFACE_CONTRACT_VERSION,
        },
    )
}

fn validate_model_ref(
    field: &'static str,
    model_ref: &PumasModelRef,
) -> Result<(), InferenceInterfaceContractError> {
    model_ref
        .validate()
        .map_err(|_| InferenceInterfaceContractError::InvalidField {
            field,
            reason: "model reference failed dependency-planning validation",
        })
}

fn validate_collection_len(
    field: &'static str,
    actual_len: usize,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    if actual_len > max_len {
        return Err(InferenceInterfaceContractError::TooManyItems {
            field,
            actual_len,
            max_len,
        });
    }
    Ok(())
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, InferenceInterfaceContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(InferenceInterfaceContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(InferenceInterfaceContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(InferenceInterfaceContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(InferenceInterfaceContractError::MissingField { field });
    }
    if trimmed.len() > max_len {
        return Err(InferenceInterfaceContractError::FieldTooLong { field, max_len });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(InferenceInterfaceContractError::InvalidText { field });
    }
    Ok(())
}

fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    match value {
        Some(value) => validate_text(field, value, max_len),
        None => Ok(()),
    }
}

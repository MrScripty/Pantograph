use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{ModelArtifactKind, PumasModelRef};

const MAX_ID_LEN: usize = 128;
const MAX_CONTEXT_LEN: usize = 256;

macro_rules! validated_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

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
            type Err = DependencyPlanningContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = DependencyPlanningContractError;

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

validated_id!(DependencyTaskId, "task_id");
validated_id!(RuntimeIntentId, "runtime_id");
validated_id!(DeviceIntentId, "device_id");
validated_id!(DependencyBindingId, "dependency_binding_id");
validated_id!(DependencyRequirementsId, "dependency_requirements_id");
validated_id!(
    DependencyOverrideFingerprint,
    "dependency_override_fingerprint"
);
validated_id!(DependencyNodeTypeId, "node_type");
validated_id!(DependencyPlatformKey, "platform_key");
validated_id!(DependencyTraitIntentId, "dependency_trait_intent_id");

/// Scheduler-facing intent supplied by a graph or caller.
///
/// These fields influence scheduler selection. They do not bypass scheduler
/// policy and do not authorize node-engine or frontend code to choose an
/// executable runtime/device directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerIntent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<DeviceIntentId>,
}

/// Bounded caller context used for diagnostics and traceability.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPlanningCallerContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_node_type: Option<DependencyNodeTypeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

impl DependencyPlanningCallerContext {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_context("caller_context.workflow_id", self.workflow_id.as_deref())?;
        validate_optional_context("caller_context.node_id", self.node_id.as_deref())?;
        validate_optional_context("caller_context.port_id", self.port_id.as_deref())?;
        validate_optional_context("caller_context.run_id", self.run_id.as_deref())?;
        Ok(())
    }
}

/// Canonical platform identity for dependency planning.
///
/// The planner and host resolver may derive this from graph input, a dependency
/// requirement result, or host facts. The shared contract carries the stable key
/// only, not arbitrary platform JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPlanningPlatformContext {
    pub platform_key: DependencyPlatformKey,
}

impl DependencyPlanningPlatformContext {
    pub fn parse_platform_key(
        value: impl AsRef<str>,
    ) -> Result<Self, DependencyPlanningContractError> {
        Ok(Self {
            platform_key: DependencyPlatformKey::parse(value)?,
        })
    }

    pub fn from_os_arch(
        os: impl AsRef<str>,
        arch: impl AsRef<str>,
    ) -> Result<Self, DependencyPlanningContractError> {
        let os = validate_identifier("platform_context.os", os.as_ref())?;
        let arch = validate_identifier("platform_context.arch", arch.as_ref())?;
        Self::parse_platform_key(format!(
            "{}-{}",
            os.to_ascii_lowercase(),
            arch.to_ascii_lowercase()
        ))
    }
}

/// Override scope for Pantograph-managed dependency patches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyOverrideScope {
    Binding,
    Requirement,
}

/// Supported override fields for dependency patch contract v1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyOverrideFieldsV1 {
    #[serde(default)]
    pub python_executable: Option<String>,
    #[serde(default)]
    pub index_url: Option<String>,
    #[serde(default)]
    pub extra_index_urls: Option<Vec<String>>,
    #[serde(default)]
    pub wheel_source_path: Option<String>,
    #[serde(default)]
    pub package_source_override: Option<String>,
}

impl DependencyOverrideFieldsV1 {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_context(
            "dependency_override.fields.python_executable",
            self.python_executable.as_deref(),
        )?;
        validate_optional_context(
            "dependency_override.fields.index_url",
            self.index_url.as_deref(),
        )?;
        validate_optional_context(
            "dependency_override.fields.wheel_source_path",
            self.wheel_source_path.as_deref(),
        )?;
        validate_optional_context(
            "dependency_override.fields.package_source_override",
            self.package_source_override.as_deref(),
        )?;
        if let Some(extra_index_urls) = &self.extra_index_urls {
            for extra_index_url in extra_index_urls {
                validate_optional_context(
                    "dependency_override.fields.extra_index_urls",
                    Some(extra_index_url),
                )?;
            }
        }
        Ok(())
    }
}

/// Manual override patch contract v1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyOverridePatchV1 {
    #[serde(default = "default_dependency_override_contract_version")]
    pub contract_version: u32,
    pub binding_id: String,
    pub scope: DependencyOverrideScope,
    #[serde(default)]
    pub requirement_name: Option<String>,
    #[serde(default)]
    pub fields: DependencyOverrideFieldsV1,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl DependencyOverridePatchV1 {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.contract_version != 1 {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_override.contract_version",
                reason: "only dependency override contract version 1 is supported",
            });
        }
        validate_identifier("dependency_override.binding_id", &self.binding_id)?;
        validate_optional_context(
            "dependency_override.requirement_name",
            self.requirement_name.as_deref(),
        )?;
        validate_optional_context("dependency_override.source", self.source.as_deref())?;
        validate_optional_context("dependency_override.updated_at", self.updated_at.as_deref())?;
        self.fields.validate()
    }
}

fn default_dependency_override_contract_version() -> u32 {
    1
}

/// Typed dependency-planning trait intent supplied by graph or host callers.
///
/// These intents are local to dependency planning identity. Scheduler runtime
/// policy stays in scheduler-owned contracts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyTraitIntent {
    pub trait_id: DependencyTraitIntentId,
    pub value: DependencyTraitIntentValue,
}

impl DependencyTraitIntent {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        self.value.validate()
    }
}

/// Bounded value for dependency-planning-local trait intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
#[non_exhaustive]
pub enum DependencyTraitIntentValue {
    Boolean(bool),
    Integer(i64),
    Text(String),
}

impl DependencyTraitIntentValue {
    fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if let Self::Text(value) = self {
            validate_optional_context("dependency_trait_intent.value", Some(value))?;
        }
        Ok(())
    }
}

/// Typed dependency planning request crossing graph, host, and scheduler seams.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyPlanningRequest {
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_intents: Vec<DependencyTraitIntent>,
    #[serde(
        default,
        skip_serializing_if = "DependencyPlanningCallerContext::is_empty"
    )]
    pub caller_context: DependencyPlanningCallerContext,
}

impl DependencyPlanningRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        self.model_ref.validate()?;
        self.caller_context.validate()?;
        for patch in &self.dependency_override_patches {
            patch.validate()?;
        }
        for intent in &self.trait_intents {
            intent.validate()?;
        }
        Ok(())
    }
}

impl SchedulerIntent {
    pub(crate) fn is_empty(&self) -> bool {
        self.requested_runtime_id.is_none() && self.requested_device_id.is_none()
    }
}

impl DependencyPlanningCallerContext {
    fn is_empty(&self) -> bool {
        self.source_node_type.is_none()
            && self.workflow_id.is_none()
            && self.node_id.is_none()
            && self.port_id.is_none()
            && self.run_id.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyPlanningRequest(DependencyPlanningRequest);

impl ValidatedDependencyPlanningRequest {
    pub fn into_inner(self) -> DependencyPlanningRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyPlanningRequest {
        &self.0
    }
}

impl TryFrom<DependencyPlanningRequest> for ValidatedDependencyPlanningRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyPlanningRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyPlanningRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let request: DependencyPlanningRequest = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_planning_request",
                reason: "request JSON did not match dependency planning contract",
            }
        })?;
        Self::try_from(request)
    }
}

pub(crate) fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_optional_context(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), DependencyPlanningContractError> {
    let Some(value) = value else {
        return Ok(());
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_CONTEXT_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_CONTEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DependencyPlanningContractError::InvalidText { field });
    }
    Ok(())
}

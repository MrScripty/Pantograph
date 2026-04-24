//! Canonical node, port, effective contract, and discovery semantics.
//!
//! This crate owns backend node-contract facts before they are projected into
//! workflow-service graph authoring, GUI, Tauri, UniFFI, Rustler, or other
//! host-language surfaces.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_ID_LEN: usize = 128;
const MAX_LABEL_LEN: usize = 256;
const MAX_DESCRIPTION_LEN: usize = 2048;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NodeContractError {
    #[error("{kind} is required")]
    MissingIdentifier { kind: &'static str },
    #[error("{kind} is longer than {max_len} characters")]
    IdentifierTooLong { kind: &'static str, max_len: usize },
    #[error("{kind} contains unsupported characters")]
    InvalidIdentifier { kind: &'static str },
    #[error("{field} is longer than {max_len} characters")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("node contract must define at least one port")]
    MissingPorts,
    #[error("port '{port_id}' has kind {actual:?}; expected {expected:?}")]
    WrongPortKind {
        port_id: PortId,
        expected: PortKind,
        actual: PortKind,
    },
}

macro_rules! contract_id {
    ($name:ident, $kind:literal, $prefix:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Self {
                Self(format!("{}{}", $prefix, Uuid::new_v4()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = NodeContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                validate_identifier($kind, value).map(Self)
            }
        }

        impl FromStr for $name {
            type Err = NodeContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(value.to_string())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

contract_id!(NodeTypeId, "node_type_id", "node_type_");
contract_id!(NodeInstanceId, "node_instance_id", "node_");
contract_id!(PortId, "port_id", "port_");

fn validate_identifier(kind: &'static str, value: String) -> Result<String, NodeContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(NodeContractError::MissingIdentifier { kind });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(NodeContractError::IdentifierTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(NodeContractError::InvalidIdentifier { kind });
    }
    Ok(trimmed.to_string())
}

pub fn validate_display_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), NodeContractError> {
    if value.len() > max_len {
        return Err(NodeContractError::FieldTooLong { field, max_len });
    }
    if value.chars().any(char::is_control) {
        return Err(NodeContractError::InvalidText { field });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    Input,
    Output,
    Processing,
    Control,
    Tool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortCardinality {
    Single,
    Multiple,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortVisibility {
    Public,
    Advanced,
    Hidden,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortValueType {
    Any,
    String,
    Image,
    Audio,
    AudioStream,
    Component,
    Stream,
    Prompt,
    Tools,
    Embedding,
    Document,
    Json,
    KvCache,
    Boolean,
    Number,
    VectorDb,
    ModelHandle,
    EmbeddingHandle,
    DatabaseHandle,
    Vector,
    Tensor,
    AudioSamples,
}

impl PortValueType {
    pub fn is_compatible_with(self, target: Self) -> bool {
        self.compatibility_with(target).is_compatible()
    }

    pub fn compatibility_with(self, target: Self) -> PortTypeCompatibility {
        if matches!(self, Self::Any) || matches!(target, Self::Any) {
            return PortTypeCompatibility::compatible(CompatibilityRule::Any);
        }

        if self == target {
            return PortTypeCompatibility::compatible(CompatibilityRule::Exact);
        }

        if matches!(
            (self, target),
            (Self::Prompt, Self::String) | (Self::String, Self::Prompt)
        ) {
            return PortTypeCompatibility::compatible(CompatibilityRule::PromptString);
        }

        if matches!(
            (self, target),
            (Self::AudioStream, Self::Stream) | (Self::Stream, Self::AudioStream)
        ) {
            return PortTypeCompatibility::compatible(CompatibilityRule::AudioStream);
        }

        if matches!(target, Self::String)
            && matches!(self, Self::Json | Self::Number | Self::Boolean)
        {
            return PortTypeCompatibility::compatible(CompatibilityRule::StringCoercion);
        }

        PortTypeCompatibility {
            compatible: false,
            rule: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityRule {
    Any,
    Exact,
    PromptString,
    AudioStream,
    StringCoercion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub struct PortTypeCompatibility {
    pub compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<CompatibilityRule>,
}

impl PortTypeCompatibility {
    pub fn compatible(rule: CompatibilityRule) -> Self {
        Self {
            compatible: true,
            rule: Some(rule),
        }
    }

    pub fn is_compatible(self) -> bool {
        self.compatible
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PortConstraint {
    AllowedValues { values: Vec<String> },
    MinNumber { value: String },
    MaxNumber { value: String },
    MimeTypes { values: Vec<String> },
    SchemaRef { schema_id: String },
    RuntimeCapability { capability_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorHint {
    Text,
    TextArea,
    Number,
    Boolean,
    Select,
    File,
    Image,
    Audio,
    Code,
    Model,
    Hidden,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionSemantics {
    Batch,
    Stream,
    Reactive,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeCapabilityRequirement {
    pub capability_id: String,
    #[serde(default)]
    pub required: bool,
}

impl NodeCapabilityRequirement {
    pub fn required(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            required: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeAuthoringMetadata {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PortContract {
    pub id: PortId,
    pub kind: PortKind,
    pub label: String,
    pub value_type: PortValueType,
    pub requirement: PortRequirement,
    pub cardinality: PortCardinality,
    pub visibility: PortVisibility,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<PortConstraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub editor_hints: Vec<EditorHint>,
}

impl PortContract {
    pub fn input(
        id: PortId,
        label: impl Into<String>,
        value_type: PortValueType,
        requirement: PortRequirement,
    ) -> Self {
        Self {
            id,
            kind: PortKind::Input,
            label: label.into(),
            value_type,
            requirement,
            cardinality: PortCardinality::Single,
            visibility: PortVisibility::Public,
            constraints: Vec::new(),
            editor_hints: Vec::new(),
        }
    }

    pub fn output(id: PortId, label: impl Into<String>, value_type: PortValueType) -> Self {
        Self {
            id,
            kind: PortKind::Output,
            label: label.into(),
            value_type,
            requirement: PortRequirement::Optional,
            cardinality: PortCardinality::Single,
            visibility: PortVisibility::Public,
            constraints: Vec::new(),
            editor_hints: Vec::new(),
        }
    }

    pub fn multiple(mut self) -> Self {
        self.cardinality = PortCardinality::Multiple;
        self
    }

    pub fn validate(&self) -> Result<(), NodeContractError> {
        validate_display_text("port.label", &self.label, MAX_LABEL_LEN)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeTypeContract {
    pub node_type: NodeTypeId,
    pub category: NodeCategory,
    pub label: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<PortContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<PortContract>,
    pub execution_semantics: NodeExecutionSemantics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_requirements: Vec<NodeCapabilityRequirement>,
    #[serde(default)]
    pub authoring: NodeAuthoringMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
}

impl NodeTypeContract {
    pub fn validate(&self) -> Result<(), NodeContractError> {
        validate_display_text("node.label", &self.label, MAX_LABEL_LEN)?;
        validate_display_text("node.description", &self.description, MAX_DESCRIPTION_LEN)?;
        if self.inputs.is_empty() && self.outputs.is_empty() {
            return Err(NodeContractError::MissingPorts);
        }
        for port in self.inputs.iter().chain(self.outputs.iter()) {
            port.validate()?;
        }
        for port in &self.inputs {
            if port.kind != PortKind::Input {
                return Err(NodeContractError::WrongPortKind {
                    port_id: port.id.clone(),
                    expected: PortKind::Input,
                    actual: port.kind,
                });
            }
        }
        for port in &self.outputs {
            if port.kind != PortKind::Output {
                return Err(NodeContractError::WrongPortKind {
                    port_id: port.id.clone(),
                    expected: PortKind::Output,
                    actual: port.kind,
                });
            }
        }
        Ok(())
    }

    pub fn input(&self, port_id: &PortId) -> Option<&PortContract> {
        self.inputs.iter().find(|port| &port.id == port_id)
    }

    pub fn output(&self, port_id: &PortId) -> Option<&PortContract> {
        self.outputs.iter().find(|port| &port.id == port_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NodeInstanceContext {
    pub node_instance_id: NodeInstanceId,
    pub node_type: NodeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct EffectivePortContract {
    pub base: PortContract,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_reasons: Vec<ContractExpansionReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EffectiveNodeContract {
    pub context: NodeInstanceContext,
    pub static_contract: NodeTypeContract,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<EffectivePortContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<EffectivePortContract>,
    #[serde(default)]
    pub diagnostics: ContractResolutionDiagnostics,
}

impl EffectiveNodeContract {
    pub fn from_static(context: NodeInstanceContext, static_contract: NodeTypeContract) -> Self {
        let inputs = static_contract
            .inputs
            .iter()
            .cloned()
            .map(|base| EffectivePortContract {
                base,
                expansion_reasons: vec![ContractExpansionReason::StaticContract],
            })
            .collect();
        let outputs = static_contract
            .outputs
            .iter()
            .cloned()
            .map(|base| EffectivePortContract {
                base,
                expansion_reasons: vec![ContractExpansionReason::StaticContract],
            })
            .collect();
        Self {
            context,
            static_contract,
            inputs,
            outputs,
            diagnostics: ContractResolutionDiagnostics::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractExpansionReason {
    StaticContract,
    DynamicConfiguration,
    BackendCapability,
    PortOptionSelection,
    RuntimeState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ContractResolutionDiagnostics {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_reasons: Vec<ContractExpansionReason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ContractResolutionWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ContractResolutionWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CompatibilityCheck {
    pub source_node_id: NodeInstanceId,
    pub source_port_id: PortId,
    pub source_value_type: PortValueType,
    pub target_node_id: NodeInstanceId,
    pub target_port_id: PortId,
    pub target_value_type: PortValueType,
}

impl CompatibilityCheck {
    pub fn new(
        source_node_id: NodeInstanceId,
        source_port: &PortContract,
        target_node_id: NodeInstanceId,
        target_port: &PortContract,
    ) -> Result<Self, NodeContractError> {
        if source_port.kind != PortKind::Output {
            return Err(NodeContractError::WrongPortKind {
                port_id: source_port.id.clone(),
                expected: PortKind::Output,
                actual: source_port.kind,
            });
        }
        if target_port.kind != PortKind::Input {
            return Err(NodeContractError::WrongPortKind {
                port_id: target_port.id.clone(),
                expected: PortKind::Input,
                actual: target_port.kind,
            });
        }
        Ok(Self {
            source_node_id,
            source_port_id: source_port.id.clone(),
            source_value_type: source_port.value_type,
            target_node_id,
            target_port_id: target_port.id.clone(),
            target_value_type: target_port.value_type,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CompatibilityResult {
    pub compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<CompatibilityRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejectionDiagnostic>,
}

impl CompatibilityResult {
    pub fn is_compatible(&self) -> bool {
        self.compatible
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRejectionReason {
    UnknownSourcePort,
    UnknownTargetPort,
    SourcePortNotOutput,
    TargetPortNotInput,
    IncompatibleTypes,
    TargetCapacityReached,
    DuplicateConnection,
    SelfConnection,
    CycleDetected,
    ConstraintViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ConnectionRejectionDiagnostic {
    pub reason: ConnectionRejectionReason,
    pub source_node_id: NodeInstanceId,
    pub source_port_id: PortId,
    pub source_value_type: PortValueType,
    pub target_node_id: NodeInstanceId,
    pub target_port_id: PortId,
    pub target_value_type: PortValueType,
    pub message: String,
}

pub fn check_compatibility(check: CompatibilityCheck) -> CompatibilityResult {
    let compatibility = check
        .source_value_type
        .compatibility_with(check.target_value_type);
    if compatibility.compatible {
        return CompatibilityResult {
            compatible: true,
            rule: compatibility.rule,
            rejection: None,
        };
    }

    CompatibilityResult {
        compatible: false,
        rule: None,
        rejection: Some(ConnectionRejectionDiagnostic {
            reason: ConnectionRejectionReason::IncompatibleTypes,
            source_node_id: check.source_node_id,
            source_port_id: check.source_port_id,
            source_value_type: check.source_value_type,
            target_node_id: check.target_node_id,
            target_port_id: check.target_port_id,
            target_value_type: check.target_value_type,
            message: format!(
                "source type '{:?}' is not compatible with target type '{:?}'",
                check.source_value_type, check.target_value_type
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T: FromStr<Err = NodeContractError>>(value: &str) -> T {
        value.parse().expect("valid id")
    }

    fn test_contract() -> NodeTypeContract {
        NodeTypeContract {
            node_type: id("llm-inference"),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Runs an LLM request".to_string(),
            inputs: vec![PortContract::input(
                id("prompt"),
                "Prompt",
                PortValueType::Prompt,
                PortRequirement::Required,
            )],
            outputs: vec![PortContract::output(
                id("response"),
                "Response",
                PortValueType::String,
            )],
            execution_semantics: NodeExecutionSemantics::Reactive,
            capability_requirements: vec![NodeCapabilityRequirement::required("llm")],
            authoring: NodeAuthoringMetadata::default(),
            contract_version: Some("1".to_string()),
            contract_digest: None,
        }
    }

    #[test]
    fn ids_trim_and_reject_invalid_values() {
        let parsed: NodeTypeId = "  llm-inference  ".parse().expect("valid node type");
        assert_eq!(parsed.as_str(), "llm-inference");

        assert_eq!(
            "".parse::<NodeTypeId>().expect_err("blank id"),
            NodeContractError::MissingIdentifier {
                kind: "node_type_id"
            }
        );
        assert_eq!(
            "bad id".parse::<PortId>().expect_err("space rejected"),
            NodeContractError::InvalidIdentifier { kind: "port_id" }
        );
    }

    #[test]
    fn generated_node_instance_ids_are_backend_owned_and_valid() {
        let generated = NodeInstanceId::generate();
        assert!(generated.as_str().starts_with("node_"));
        assert!(generated.as_str().parse::<NodeInstanceId>().is_ok());
    }

    #[test]
    fn port_value_type_compatibility_matches_backend_rules() {
        assert_eq!(
            PortValueType::Any
                .compatibility_with(PortValueType::KvCache)
                .rule,
            Some(CompatibilityRule::Any)
        );
        assert_eq!(
            PortValueType::Prompt
                .compatibility_with(PortValueType::String)
                .rule,
            Some(CompatibilityRule::PromptString)
        );
        assert_eq!(
            PortValueType::AudioStream
                .compatibility_with(PortValueType::Stream)
                .rule,
            Some(CompatibilityRule::AudioStream)
        );
        assert_eq!(
            PortValueType::Number
                .compatibility_with(PortValueType::String)
                .rule,
            Some(CompatibilityRule::StringCoercion)
        );
        assert!(!PortValueType::KvCache.is_compatible_with(PortValueType::Json));
    }

    #[test]
    fn compatibility_result_carries_structured_rejection() {
        let source = PortContract::output(id("cache"), "Cache", PortValueType::KvCache);
        let target = PortContract::input(
            id("json"),
            "JSON",
            PortValueType::Json,
            PortRequirement::Required,
        );
        let check = CompatibilityCheck::new(id("source"), &source, id("target"), &target)
            .expect("valid check");

        let result = check_compatibility(check);

        assert!(!result.is_compatible());
        let rejection = result.rejection.expect("rejection");
        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::IncompatibleTypes
        );
        assert_eq!(rejection.source_port_id.as_str(), "cache");
        assert_eq!(rejection.target_port_id.as_str(), "json");
    }

    #[test]
    fn compatibility_check_rejects_wrong_port_directions() {
        let input = PortContract::input(
            id("prompt"),
            "Prompt",
            PortValueType::Prompt,
            PortRequirement::Required,
        );
        let output = PortContract::output(id("response"), "Response", PortValueType::String);

        let err = CompatibilityCheck::new(id("source"), &input, id("target"), &output)
            .expect_err("input cannot be source");
        assert_eq!(
            err,
            NodeContractError::WrongPortKind {
                port_id: id("prompt"),
                expected: PortKind::Output,
                actual: PortKind::Input,
            }
        );
    }

    #[test]
    fn node_type_contract_validates_port_directions_and_text() {
        let contract = test_contract();
        contract.validate().expect("valid contract");

        let mut invalid = contract;
        invalid.inputs[0].kind = PortKind::Output;

        assert_eq!(
            invalid.validate().expect_err("wrong direction"),
            NodeContractError::WrongPortKind {
                port_id: id("prompt"),
                expected: PortKind::Input,
                actual: PortKind::Output,
            }
        );
    }

    #[test]
    fn effective_contract_preserves_static_ports_with_diagnostics() {
        let static_contract = test_contract();
        let context = NodeInstanceContext {
            node_instance_id: id("llm-1"),
            node_type: id("llm-inference"),
            graph_revision: Some("rev-1".to_string()),
            configuration: Some(serde_json::json!({"model": "example"})),
        };

        let effective = EffectiveNodeContract::from_static(context, static_contract);

        assert_eq!(effective.inputs.len(), 1);
        assert_eq!(effective.outputs.len(), 1);
        assert_eq!(
            effective.inputs[0].expansion_reasons,
            vec![ContractExpansionReason::StaticContract]
        );
        assert!(effective.diagnostics.warnings.is_empty());
    }

    #[test]
    fn contracts_round_trip_as_snake_case_json() {
        let contract = test_contract();
        let value = serde_json::to_value(&contract).expect("serialize");

        assert_eq!(value["node_type"], "llm-inference");
        assert_eq!(value["execution_semantics"], "reactive");
        assert_eq!(value["inputs"][0]["value_type"], "prompt");

        let parsed: NodeTypeContract = serde_json::from_value(value).expect("deserialize");
        assert_eq!(parsed.node_type.as_str(), "llm-inference");
    }
}

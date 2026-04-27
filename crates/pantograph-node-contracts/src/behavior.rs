use serde::{Deserialize, Serialize};

use super::{NodeContractError, NodeTypeContract, NodeTypeId};

const BEHAVIOR_DIGEST_PREFIX: &str = "contract-blake3:";
const MAX_BEHAVIOR_DIGEST_LEN: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeBehaviorVersion {
    pub node_type: NodeTypeId,
    pub contract_version: String,
    pub behavior_digest: String,
}

impl NodeBehaviorVersion {
    pub fn from_contract(contract: &NodeTypeContract) -> Result<Self, NodeContractError> {
        let contract_version = validated_contract_version(contract.contract_version.as_deref())?;
        let behavior_digest = match contract.contract_digest.as_deref() {
            Some(digest) if !digest.trim().is_empty() => validated_behavior_digest(digest)?,
            _ => compute_node_behavior_digest(contract)?,
        };

        Ok(Self {
            node_type: contract.node_type.clone(),
            contract_version: contract_version.to_string(),
            behavior_digest,
        })
    }
}

pub fn compute_node_behavior_digest(
    contract: &NodeTypeContract,
) -> Result<String, NodeContractError> {
    let mut digest_input = contract.clone();
    digest_input.contract_digest = None;
    let bytes = serde_json::to_vec(&digest_input).map_err(|error| {
        NodeContractError::BehaviorDigestEncoding {
            reason: error.to_string(),
        }
    })?;
    Ok(format!("{BEHAVIOR_DIGEST_PREFIX}{}", blake3::hash(&bytes)))
}

fn validated_contract_version(value: Option<&str>) -> Result<&str, NodeContractError> {
    let Some(value) = value else {
        return Err(NodeContractError::MissingBehaviorIdentityField {
            field: "contract_version",
        });
    };
    if !is_semantic_version(value) {
        return Err(NodeContractError::InvalidBehaviorIdentityField {
            field: "contract_version",
            reason: "must use major.minor.patch numeric semantic version",
        });
    }
    Ok(value)
}

fn validated_behavior_digest(value: &str) -> Result<String, NodeContractError> {
    if value.len() > MAX_BEHAVIOR_DIGEST_LEN {
        return Err(NodeContractError::InvalidBehaviorIdentityField {
            field: "contract_digest",
            reason: "must be at most 256 bytes",
        });
    }
    if value.trim() != value || value.is_empty() {
        return Err(NodeContractError::InvalidBehaviorIdentityField {
            field: "contract_digest",
            reason: "must be non-empty without surrounding whitespace",
        });
    }
    if !value.is_ascii() || value.chars().any(char::is_whitespace) {
        return Err(NodeContractError::InvalidBehaviorIdentityField {
            field: "contract_digest",
            reason: "must be ASCII without whitespace",
        });
    }
    Ok(value.to_string())
}

fn is_semantic_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && is_numeric_semver_part(major)
        && is_numeric_semver_part(minor)
        && is_numeric_semver_part(patch)
}

fn is_numeric_semver_part(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        NodeAuthoringMetadata, NodeCapabilityRequirement, NodeCategory, NodeExecutionSemantics,
        PortContract, PortRequirement, PortValueType,
    };

    fn contract(version: Option<&str>, digest: Option<&str>) -> NodeTypeContract {
        NodeTypeContract {
            node_type: "llm-inference".parse().expect("node type"),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Runs an LLM request".to_string(),
            inputs: vec![PortContract::input(
                "prompt".parse().expect("port id"),
                "Prompt",
                PortValueType::Prompt,
                PortRequirement::Required,
            )],
            outputs: vec![PortContract::output(
                "response".parse().expect("port id"),
                "Response",
                PortValueType::String,
            )],
            execution_semantics: NodeExecutionSemantics::Reactive,
            capability_requirements: vec![NodeCapabilityRequirement::required("llm")],
            authoring: NodeAuthoringMetadata::default(),
            contract_version: version.map(ToOwned::to_owned),
            contract_digest: digest.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn behavior_version_uses_provided_digest() {
        let version =
            NodeBehaviorVersion::from_contract(&contract(Some("1.2.3"), Some("sha256:abc123")))
                .expect("behavior version");

        assert_eq!(version.contract_version, "1.2.3");
        assert_eq!(version.behavior_digest, "sha256:abc123");
    }

    #[test]
    fn behavior_version_computes_stable_digest_when_missing() {
        let left = NodeBehaviorVersion::from_contract(&contract(Some("1.0.0"), None))
            .expect("left behavior version");
        let right = NodeBehaviorVersion::from_contract(&contract(Some("1.0.0"), None))
            .expect("right behavior version");

        assert!(left.behavior_digest.starts_with(BEHAVIOR_DIGEST_PREFIX));
        assert_eq!(left.behavior_digest, right.behavior_digest);
    }

    #[test]
    fn behavior_version_rejects_missing_or_non_semantic_versions() {
        assert!(matches!(
            NodeBehaviorVersion::from_contract(&contract(None, None)),
            Err(NodeContractError::MissingBehaviorIdentityField {
                field: "contract_version"
            })
        ));
        assert!(matches!(
            NodeBehaviorVersion::from_contract(&contract(Some("1"), None)),
            Err(NodeContractError::InvalidBehaviorIdentityField {
                field: "contract_version",
                ..
            })
        ));
    }
}

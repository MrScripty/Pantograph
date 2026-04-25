//! Canonical contract projection for concrete workflow node registrations.
//!
//! Concrete node implementations still expose execution descriptors through
//! `node-engine`, while this module projects those descriptors into
//! `pantograph-node-contracts` for graph-authoring and binding surfaces.

use pantograph_node_contracts::{
    ComposedInternalEdge, ComposedInternalGraph, ComposedInternalNode, ComposedNodeContract,
    ComposedPortMapping, ComposedPortMappings, ComposedTracePolicy, NodeAuthoringMetadata,
    NodeCapabilityRequirement, NodeCategory, NodeContractError, NodeExecutionSemantics,
    NodeInstanceId, NodeTypeContract, NodeTypeId, PortCardinality, PortContract, PortId, PortKind,
    PortRequirement, PortValueType, PortVisibility,
};

pub fn builtin_node_contracts() -> Result<Vec<NodeTypeContract>, NodeContractError> {
    let registry = node_engine::NodeRegistry::with_builtins();
    let mut contracts = registry
        .all_metadata()
        .into_iter()
        .map(task_metadata_to_contract)
        .collect::<Result<Vec<_>, _>>()?;
    contracts.sort_by(|left, right| left.node_type.as_str().cmp(right.node_type.as_str()));
    Ok(contracts)
}

pub fn builtin_composed_node_contracts() -> Result<Vec<ComposedNodeContract>, NodeContractError> {
    let contracts = vec![tool_loop_composed_contract()?];
    for contract in &contracts {
        contract.validate()?;
    }
    Ok(contracts)
}

pub fn task_metadata_to_contract(
    metadata: &node_engine::TaskMetadata,
) -> Result<NodeTypeContract, NodeContractError> {
    let node_type = NodeTypeId::try_from(metadata.node_type.clone())?;
    let inputs = metadata
        .inputs
        .iter()
        .map(|port| port_metadata_to_contract(PortKind::Input, port))
        .collect::<Result<Vec<_>, _>>()?;
    let outputs = metadata
        .outputs
        .iter()
        .map(|port| port_metadata_to_contract(PortKind::Output, port))
        .collect::<Result<Vec<_>, _>>()?;

    let contract = NodeTypeContract {
        node_type,
        category: convert_category(metadata.category),
        label: metadata.label.clone(),
        description: metadata.description.clone(),
        inputs,
        outputs,
        execution_semantics: convert_execution_semantics(metadata.execution_mode),
        capability_requirements: capability_requirements(metadata),
        authoring: authoring_metadata(metadata),
        contract_version: Some("1".to_string()),
        contract_digest: None,
    };
    contract.validate()?;
    Ok(contract)
}

fn tool_loop_composed_contract() -> Result<ComposedNodeContract, NodeContractError> {
    let metadata = <crate::control::ToolLoopTask as node_engine::TaskDescriptor>::descriptor();
    let external_contract = task_metadata_to_contract(&metadata)?;
    Ok(ComposedNodeContract {
        external_contract,
        internal_graph: ComposedInternalGraph {
            graph_id: "tool-loop-internal-v1".to_string(),
            nodes: vec![
                internal_node("tool-loop.llm", "llm-inference", "Tool Loop LLM")?,
                internal_node("tool-loop.tool-executor", "tool-executor", "Tool Executor")?,
                internal_node("tool-loop.turn-state", "merge", "Turn State")?,
            ],
            edges: vec![
                ComposedInternalEdge {
                    source_node_id: node_id("tool-loop.llm")?,
                    source_port_id: port_id("tool_calls")?,
                    target_node_id: node_id("tool-loop.tool-executor")?,
                    target_port_id: port_id("tool_calls")?,
                },
                ComposedInternalEdge {
                    source_node_id: node_id("tool-loop.tool-executor")?,
                    source_port_id: port_id("results")?,
                    target_node_id: node_id("tool-loop.turn-state")?,
                    target_port_id: port_id("inputs")?,
                },
            ],
        },
        port_mappings: ComposedPortMappings {
            inputs: vec![
                map_port("prompt", "tool-loop.llm", "prompt")?,
                map_port("system_prompt", "tool-loop.llm", "system_prompt")?,
                map_port("context", "tool-loop.llm", "context")?,
                map_port("tools", "tool-loop.llm", "tools")?,
                map_port("max_turns", "tool-loop.turn-state", "inputs")?,
            ],
            outputs: vec![
                map_port("response", "tool-loop.llm", "response")?,
                map_port("tool_calls", "tool-loop.llm", "tool_calls")?,
                map_port("turns", "tool-loop.turn-state", "count")?,
            ],
        },
        trace_policy: ComposedTracePolicy::PreservePrimitiveFacts,
        upgrade_metadata: None,
    })
}

fn internal_node(
    node_id_value: &str,
    node_type_value: &str,
    label: &str,
) -> Result<ComposedInternalNode, NodeContractError> {
    Ok(ComposedInternalNode {
        node_id: node_id(node_id_value)?,
        node_type: node_type_id(node_type_value)?,
        label: label.to_string(),
        contract_version: Some("1".to_string()),
        contract_digest: None,
    })
}

fn map_port(
    external_port_id: &str,
    internal_node_id: &str,
    internal_port_id: &str,
) -> Result<ComposedPortMapping, NodeContractError> {
    Ok(ComposedPortMapping {
        external_port_id: port_id(external_port_id)?,
        internal_node_id: node_id(internal_node_id)?,
        internal_port_id: port_id(internal_port_id)?,
    })
}

fn node_type_id(value: &str) -> Result<NodeTypeId, NodeContractError> {
    value.parse()
}

fn node_id(value: &str) -> Result<NodeInstanceId, NodeContractError> {
    value.parse()
}

fn port_id(value: &str) -> Result<PortId, NodeContractError> {
    value.parse()
}

fn port_metadata_to_contract(
    kind: PortKind,
    metadata: &node_engine::PortMetadata,
) -> Result<PortContract, NodeContractError> {
    let contract = PortContract {
        id: PortId::try_from(metadata.id.clone())?,
        kind,
        label: metadata.label.clone(),
        value_type: convert_value_type(metadata.data_type),
        requirement: if metadata.required {
            PortRequirement::Required
        } else {
            PortRequirement::Optional
        },
        cardinality: if metadata.multiple {
            PortCardinality::Multiple
        } else {
            PortCardinality::Single
        },
        visibility: PortVisibility::Public,
        constraints: Vec::new(),
        editor_hints: Vec::new(),
    };
    contract.validate()?;
    Ok(contract)
}

fn convert_category(category: node_engine::NodeCategory) -> NodeCategory {
    match category {
        node_engine::NodeCategory::Input => NodeCategory::Input,
        node_engine::NodeCategory::Output => NodeCategory::Output,
        node_engine::NodeCategory::Processing => NodeCategory::Processing,
        node_engine::NodeCategory::Control => NodeCategory::Control,
        node_engine::NodeCategory::Tool => NodeCategory::Tool,
    }
}

fn convert_execution_semantics(mode: node_engine::ExecutionMode) -> NodeExecutionSemantics {
    match mode {
        node_engine::ExecutionMode::Batch => NodeExecutionSemantics::Batch,
        node_engine::ExecutionMode::Stream => NodeExecutionSemantics::Stream,
        node_engine::ExecutionMode::Reactive => NodeExecutionSemantics::Reactive,
        node_engine::ExecutionMode::Manual => NodeExecutionSemantics::Manual,
    }
}

fn convert_value_type(value_type: node_engine::PortDataType) -> PortValueType {
    match value_type {
        node_engine::PortDataType::Any => PortValueType::Any,
        node_engine::PortDataType::String => PortValueType::String,
        node_engine::PortDataType::Image => PortValueType::Image,
        node_engine::PortDataType::Audio => PortValueType::Audio,
        node_engine::PortDataType::AudioStream => PortValueType::AudioStream,
        node_engine::PortDataType::Component => PortValueType::Component,
        node_engine::PortDataType::Stream => PortValueType::Stream,
        node_engine::PortDataType::Prompt => PortValueType::Prompt,
        node_engine::PortDataType::Tools => PortValueType::Tools,
        node_engine::PortDataType::Embedding => PortValueType::Embedding,
        node_engine::PortDataType::Document => PortValueType::Document,
        node_engine::PortDataType::Json => PortValueType::Json,
        node_engine::PortDataType::KvCache => PortValueType::KvCache,
        node_engine::PortDataType::Boolean => PortValueType::Boolean,
        node_engine::PortDataType::Number => PortValueType::Number,
        node_engine::PortDataType::VectorDb => PortValueType::VectorDb,
        node_engine::PortDataType::ModelHandle => PortValueType::ModelHandle,
        node_engine::PortDataType::EmbeddingHandle => PortValueType::EmbeddingHandle,
        node_engine::PortDataType::DatabaseHandle => PortValueType::DatabaseHandle,
        node_engine::PortDataType::Vector => PortValueType::Vector,
        node_engine::PortDataType::Tensor => PortValueType::Tensor,
        node_engine::PortDataType::AudioSamples => PortValueType::AudioSamples,
    }
}

fn capability_requirements(metadata: &node_engine::TaskMetadata) -> Vec<NodeCapabilityRequirement> {
    match metadata.node_type.as_str() {
        "llm-inference" | "llamacpp-inference" | "ollama-inference" | "pytorch-inference"
        | "onnx-inference" => vec![NodeCapabilityRequirement::required("llm")],
        "diffusion-inference" => vec![NodeCapabilityRequirement::required("image_generation")],
        "audio-generation" => vec![NodeCapabilityRequirement::required("audio_generation")],
        "embedding" => vec![NodeCapabilityRequirement::required("embedding")],
        "puma-lib" | "model-provider" => {
            vec![NodeCapabilityRequirement::required("model_library")]
        }
        _ => Vec::new(),
    }
}

fn authoring_metadata(metadata: &node_engine::TaskMetadata) -> NodeAuthoringMetadata {
    NodeAuthoringMetadata {
        tags: vec![format!("{:?}", metadata.category).to_lowercase()],
        icon: None,
        color: None,
        documentation_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_node_contracts_cover_all_registered_descriptors() {
        let engine_registry = node_engine::NodeRegistry::with_builtins();
        let contracts = builtin_node_contracts().expect("canonical contracts");

        assert_eq!(contracts.len(), engine_registry.all_metadata().len());
        assert!(contracts.iter().all(|contract| contract.validate().is_ok()));
        assert!(contracts
            .iter()
            .any(|contract| contract.node_type.as_str() == "llm-inference"));
        assert!(contracts
            .iter()
            .any(|contract| contract.node_type.as_str() == "text-output"));
    }

    #[test]
    fn builtin_composed_contracts_register_tool_loop_authoring_contract() {
        let contracts = builtin_composed_node_contracts().expect("composed contracts");
        let tool_loop = contracts
            .iter()
            .find(|contract| contract.external_contract.node_type.as_str() == "tool-loop")
            .expect("tool-loop composed contract");

        assert_eq!(
            tool_loop.trace_policy,
            ComposedTracePolicy::PreservePrimitiveFacts
        );
        assert!(tool_loop
            .internal_graph
            .nodes
            .iter()
            .any(|node| node.node_type.as_str() == "llm-inference"));
        assert!(tool_loop
            .internal_graph
            .nodes
            .iter()
            .any(|node| node.node_type.as_str() == "tool-executor"));
        assert_eq!(
            tool_loop
                .port_mappings
                .inputs
                .iter()
                .map(|mapping| mapping.external_port_id.as_str())
                .collect::<Vec<_>>(),
            vec!["prompt", "system_prompt", "context", "tools", "max_turns"]
        );
        assert_eq!(
            tool_loop
                .port_mappings
                .outputs
                .iter()
                .map(|mapping| mapping.external_port_id.as_str())
                .collect::<Vec<_>>(),
            vec!["response", "tool_calls", "turns"]
        );
        tool_loop.validate().expect("valid composed contract");
    }

    #[test]
    fn contract_projection_preserves_port_directions_and_value_types() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let llm = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "llm-inference")
            .expect("llm contract");

        let prompt = llm
            .inputs
            .iter()
            .find(|port| port.id.as_str() == "prompt")
            .expect("prompt port");
        assert_eq!(prompt.kind, PortKind::Input);
        assert_eq!(prompt.value_type, PortValueType::Prompt);
        assert_eq!(prompt.requirement, PortRequirement::Required);

        let response = llm
            .outputs
            .iter()
            .find(|port| port.id.as_str() == "response")
            .expect("response port");
        assert_eq!(response.kind, PortKind::Output);
        assert_eq!(response.value_type, PortValueType::String);
    }

    #[test]
    fn projection_preserves_extended_engine_value_types() {
        let metadata = node_engine::TaskMetadata {
            node_type: "extended-types".to_string(),
            category: node_engine::NodeCategory::Processing,
            label: "Extended Types".to_string(),
            description: "Preserves engine-only value types".to_string(),
            inputs: vec![node_engine::PortMetadata::required(
                "model",
                "Model",
                node_engine::PortDataType::ModelHandle,
            )],
            outputs: vec![node_engine::PortMetadata::optional(
                "tensor",
                "Tensor",
                node_engine::PortDataType::Tensor,
            )],
            execution_mode: node_engine::ExecutionMode::Batch,
        };

        let contract = task_metadata_to_contract(&metadata).expect("contract");

        assert_eq!(contract.inputs[0].value_type, PortValueType::ModelHandle);
        assert_eq!(contract.outputs[0].value_type, PortValueType::Tensor);
        assert_eq!(contract.execution_semantics, NodeExecutionSemantics::Batch);
    }

    #[test]
    fn projection_rejects_invalid_descriptor_ids() {
        let metadata = node_engine::TaskMetadata {
            node_type: "bad node".to_string(),
            category: node_engine::NodeCategory::Processing,
            label: "Bad Node".to_string(),
            description: "Invalid id".to_string(),
            inputs: Vec::new(),
            outputs: vec![node_engine::PortMetadata::optional(
                "out",
                "Out",
                node_engine::PortDataType::String,
            )],
            execution_mode: node_engine::ExecutionMode::Reactive,
        };

        assert_eq!(
            task_metadata_to_contract(&metadata).expect_err("invalid id"),
            NodeContractError::InvalidIdentifier {
                kind: "node_type_id"
            }
        );
    }
}

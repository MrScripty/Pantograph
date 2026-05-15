//! Canonical contract projection for concrete workflow node registrations.
//!
//! Concrete node implementations still expose execution descriptors through
//! `node-engine`, while this module projects those descriptors into
//! `pantograph-node-contracts` for graph-authoring and binding surfaces.

use inference::model_contracts::{
    default_task_registry_entries, InferenceExecutionInputKind, InferenceExecutionResultKind,
    InferenceModality, InferenceTaskId, TaskRequestContract, TaskStreamingSupport,
};
use pantograph_node_contracts::{
    ComposedInternalEdge, ComposedInternalGraph, ComposedInternalNode, ComposedNodeContract,
    ComposedPortMapping, ComposedPortMappings, ComposedTracePolicy,
    ContractInferenceExecutionInputKind, ContractInferenceExecutionResultKind,
    ContractInferenceModality, ContractInferenceStreamingSupport, ContractInferenceTaskId,
    InferencePortPayloadContract, InferencePortPayloadRole, NodeAuthoringMetadata,
    NodeCapabilityRequirement, NodeCategory, NodeContractError, NodeExecutionSemantics,
    NodeInferenceTaskContract, NodeInstanceId, NodeTypeContract, NodeTypeId, PortCardinality,
    PortContract, PortId, PortKind, PortRequirement, PortValueType, PortVisibility,
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

    let mut contract = NodeTypeContract {
        node_type,
        category: convert_category(metadata.category),
        label: metadata.label.clone(),
        description: metadata.description.clone(),
        inputs,
        outputs,
        execution_semantics: convert_execution_semantics(metadata.execution_mode),
        capability_requirements: capability_requirements(metadata),
        inference_tasks: inference_task_contracts(metadata),
        authoring: authoring_metadata(metadata),
        contract_version: Some("1.0.0".to_string()),
        contract_digest: None,
    };
    apply_inference_port_payloads(metadata, &mut contract)?;
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
        contract_version: Some("1.0.0".to_string()),
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
        inference_payloads: Vec::new(),
    };
    contract.validate()?;
    Ok(contract)
}

fn inference_task_contracts(
    metadata: &node_engine::TaskMetadata,
) -> Vec<NodeInferenceTaskContract> {
    if metadata.node_type != "llm-inference" {
        return Vec::new();
    }

    default_task_registry_entries()
        .into_iter()
        .filter(|entry| llm_supported_registry_task_ids().contains(&entry.task_id))
        .filter_map(|entry| entry.request_contract())
        .map(inference_task_contract_from_registry)
        .collect()
}

fn inference_task_contract_from_registry(
    contract: TaskRequestContract,
) -> NodeInferenceTaskContract {
    NodeInferenceTaskContract {
        task_id: contract_task_id(&contract.task_id),
        input_kind: contract_input_kind(contract.input_kind),
        result_kind: contract_result_kind(contract.result_kind),
        execution_supported: contract.execution_supported,
        streaming_support: contract_streaming_support(contract.streaming_support),
        required_input_modalities: contract
            .required_input_modalities
            .iter()
            .map(contract_modality)
            .collect(),
        output_modalities: contract
            .output_modalities
            .iter()
            .map(contract_modality)
            .collect(),
    }
}

fn contract_task_id(task_id: &InferenceTaskId) -> ContractInferenceTaskId {
    match task_id {
        InferenceTaskId::TextGeneration => ContractInferenceTaskId::TextGeneration,
        InferenceTaskId::ChatCompletion => ContractInferenceTaskId::ChatCompletion,
        InferenceTaskId::Embedding => ContractInferenceTaskId::Embedding,
        InferenceTaskId::Rerank => ContractInferenceTaskId::Rerank,
        InferenceTaskId::ImageGeneration => ContractInferenceTaskId::ImageGeneration,
        InferenceTaskId::ImageUnderstanding => ContractInferenceTaskId::ImageUnderstanding,
        InferenceTaskId::DepthEstimation => ContractInferenceTaskId::DepthEstimation,
        InferenceTaskId::AudioTranscription => ContractInferenceTaskId::AudioTranscription,
        InferenceTaskId::VideoUnderstanding => ContractInferenceTaskId::VideoUnderstanding,
        InferenceTaskId::MultimodalGeneration => ContractInferenceTaskId::MultimodalGeneration,
        InferenceTaskId::Unknown => ContractInferenceTaskId::Unknown,
    }
}

fn contract_input_kind(
    input_kind: InferenceExecutionInputKind,
) -> ContractInferenceExecutionInputKind {
    match input_kind {
        InferenceExecutionInputKind::TextGeneration => {
            ContractInferenceExecutionInputKind::TextGeneration
        }
        InferenceExecutionInputKind::Embedding => ContractInferenceExecutionInputKind::Embedding,
        InferenceExecutionInputKind::Rerank => ContractInferenceExecutionInputKind::Rerank,
        InferenceExecutionInputKind::ImageGeneration => {
            ContractInferenceExecutionInputKind::ImageGeneration
        }
        InferenceExecutionInputKind::ImageUnderstanding => {
            ContractInferenceExecutionInputKind::ImageUnderstanding
        }
        InferenceExecutionInputKind::DepthEstimation => {
            ContractInferenceExecutionInputKind::DepthEstimation
        }
        InferenceExecutionInputKind::AudioTranscription => {
            ContractInferenceExecutionInputKind::AudioTranscription
        }
        InferenceExecutionInputKind::VideoUnderstanding => {
            ContractInferenceExecutionInputKind::VideoUnderstanding
        }
        InferenceExecutionInputKind::MultimodalGeneration => {
            ContractInferenceExecutionInputKind::MultimodalGeneration
        }
    }
}

fn contract_result_kind(
    result_kind: InferenceExecutionResultKind,
) -> ContractInferenceExecutionResultKind {
    match result_kind {
        InferenceExecutionResultKind::TextGeneration => {
            ContractInferenceExecutionResultKind::TextGeneration
        }
        InferenceExecutionResultKind::Embedding => ContractInferenceExecutionResultKind::Embedding,
        InferenceExecutionResultKind::Rerank => ContractInferenceExecutionResultKind::Rerank,
        InferenceExecutionResultKind::ImageGeneration => {
            ContractInferenceExecutionResultKind::ImageGeneration
        }
        InferenceExecutionResultKind::ImageUnderstanding => {
            ContractInferenceExecutionResultKind::ImageUnderstanding
        }
        InferenceExecutionResultKind::DepthEstimation => {
            ContractInferenceExecutionResultKind::DepthEstimation
        }
        InferenceExecutionResultKind::AudioTranscription => {
            ContractInferenceExecutionResultKind::AudioTranscription
        }
        InferenceExecutionResultKind::VideoUnderstanding => {
            ContractInferenceExecutionResultKind::VideoUnderstanding
        }
        InferenceExecutionResultKind::MultimodalGeneration => {
            ContractInferenceExecutionResultKind::MultimodalGeneration
        }
    }
}

fn contract_streaming_support(
    streaming_support: TaskStreamingSupport,
) -> ContractInferenceStreamingSupport {
    match streaming_support {
        TaskStreamingSupport::Supported => ContractInferenceStreamingSupport::Supported,
        TaskStreamingSupport::Unsupported => ContractInferenceStreamingSupport::Unsupported,
        TaskStreamingSupport::BackendDependent => {
            ContractInferenceStreamingSupport::BackendDependent
        }
        TaskStreamingSupport::Unknown => ContractInferenceStreamingSupport::Unknown,
    }
}

fn contract_modality(modality: &InferenceModality) -> ContractInferenceModality {
    match modality {
        InferenceModality::Text => ContractInferenceModality::Text,
        InferenceModality::Image => ContractInferenceModality::Image,
        InferenceModality::Audio => ContractInferenceModality::Audio,
        InferenceModality::Video => ContractInferenceModality::Video,
        InferenceModality::Embedding => ContractInferenceModality::Embedding,
        InferenceModality::Tokens => ContractInferenceModality::Tokens,
        InferenceModality::Json => ContractInferenceModality::Json,
        InferenceModality::PointCloud => ContractInferenceModality::PointCloud,
        InferenceModality::Mesh => ContractInferenceModality::Mesh,
        InferenceModality::Other => ContractInferenceModality::Other,
    }
}

fn apply_inference_port_payloads(
    metadata: &node_engine::TaskMetadata,
    contract: &mut NodeTypeContract,
) -> Result<(), NodeContractError> {
    if metadata.node_type != "llm-inference" {
        return Ok(());
    }

    for port in &mut contract.inputs {
        port.inference_payloads = llm_input_payloads(port.id.as_str());
    }
    for port in &mut contract.outputs {
        port.inference_payloads = llm_output_payloads(port.id.as_str());
    }
    Ok(())
}

fn llm_input_payloads(port_id: &str) -> Vec<InferencePortPayloadContract> {
    match port_id {
        "pumas_model_ref" => task_role_payloads(
            &llm_supported_task_ids(),
            InferencePortPayloadRole::ModelReference,
        ),
        "generation_options" => task_role_payloads(
            &[
                ContractInferenceTaskId::TextGeneration,
                ContractInferenceTaskId::ChatCompletion,
            ],
            InferencePortPayloadRole::Options,
        ),
        "task_kind" | "backend_key" | "task_options" | "inference_settings" => {
            task_role_payloads(&llm_supported_task_ids(), InferencePortPayloadRole::Options)
        }
        "denoising_scheduler" => task_role_payloads(
            &[ContractInferenceTaskId::ImageGeneration],
            InferencePortPayloadRole::Options,
        ),
        "prompt" | "system_prompt" | "context" | "tools" => vec![
            InferencePortPayloadContract::task_input(
                ContractInferenceTaskId::TextGeneration,
                ContractInferenceExecutionInputKind::TextGeneration,
            ),
            InferencePortPayloadContract::task_input(
                ContractInferenceTaskId::ChatCompletion,
                ContractInferenceExecutionInputKind::TextGeneration,
            ),
            InferencePortPayloadContract::task_input(
                ContractInferenceTaskId::ImageGeneration,
                ContractInferenceExecutionInputKind::ImageGeneration,
            ),
        ],
        "audio" => vec![InferencePortPayloadContract::task_input(
            ContractInferenceTaskId::AudioTranscription,
            ContractInferenceExecutionInputKind::AudioTranscription,
        )],
        "text" => vec![InferencePortPayloadContract::task_input(
            ContractInferenceTaskId::Embedding,
            ContractInferenceExecutionInputKind::Embedding,
        )],
        "query" | "documents" | "documents_json" => {
            vec![InferencePortPayloadContract::task_input(
                ContractInferenceTaskId::Rerank,
                ContractInferenceExecutionInputKind::Rerank,
            )]
        }
        _ => Vec::new(),
    }
}

fn llm_output_payloads(port_id: &str) -> Vec<InferencePortPayloadContract> {
    match port_id {
        "model_ref" => task_role_payloads(
            &llm_supported_task_ids(),
            InferencePortPayloadRole::ModelReference,
        ),
        "metadata" | "diagnostics" => task_role_payloads(
            &llm_supported_task_ids(),
            InferencePortPayloadRole::Diagnostics,
        ),
        "usage" => task_role_payloads(
            &[
                ContractInferenceTaskId::TextGeneration,
                ContractInferenceTaskId::ChatCompletion,
            ],
            InferencePortPayloadRole::Usage,
        ),
        "response" => vec![
            InferencePortPayloadContract::task_output(
                ContractInferenceTaskId::TextGeneration,
                ContractInferenceExecutionResultKind::TextGeneration,
            ),
            InferencePortPayloadContract::task_output(
                ContractInferenceTaskId::ChatCompletion,
                ContractInferenceExecutionResultKind::TextGeneration,
            ),
            InferencePortPayloadContract::task_output(
                ContractInferenceTaskId::AudioTranscription,
                ContractInferenceExecutionResultKind::AudioTranscription,
            ),
        ],
        "tool_calls" | "has_tool_calls" | "stream" => {
            vec![
                InferencePortPayloadContract::task_output(
                    ContractInferenceTaskId::TextGeneration,
                    ContractInferenceExecutionResultKind::TextGeneration,
                ),
                InferencePortPayloadContract::task_output(
                    ContractInferenceTaskId::ChatCompletion,
                    ContractInferenceExecutionResultKind::TextGeneration,
                ),
            ]
        }
        "kv_cache_out" => task_role_payloads(
            &[
                ContractInferenceTaskId::TextGeneration,
                ContractInferenceTaskId::ChatCompletion,
            ],
            InferencePortPayloadRole::CacheHandle,
        ),
        "embedding" => vec![InferencePortPayloadContract::task_output(
            ContractInferenceTaskId::Embedding,
            ContractInferenceExecutionResultKind::Embedding,
        )],
        "results" => vec![
            InferencePortPayloadContract::task_output(
                ContractInferenceTaskId::Rerank,
                ContractInferenceExecutionResultKind::Rerank,
            ),
            InferencePortPayloadContract::task_output(
                ContractInferenceTaskId::ImageGeneration,
                ContractInferenceExecutionResultKind::ImageGeneration,
            ),
        ],
        "scores" | "top_document" | "top_score" => vec![InferencePortPayloadContract::task_output(
            ContractInferenceTaskId::Rerank,
            ContractInferenceExecutionResultKind::Rerank,
        )],
        _ => Vec::new(),
    }
}

fn task_role_payloads(
    task_ids: &[ContractInferenceTaskId],
    role: InferencePortPayloadRole,
) -> Vec<InferencePortPayloadContract> {
    task_ids
        .iter()
        .copied()
        .map(|task_id| InferencePortPayloadContract::task_role(task_id, role))
        .collect()
}

fn llm_supported_task_ids() -> [ContractInferenceTaskId; 6] {
    [
        ContractInferenceTaskId::TextGeneration,
        ContractInferenceTaskId::ChatCompletion,
        ContractInferenceTaskId::Embedding,
        ContractInferenceTaskId::Rerank,
        ContractInferenceTaskId::ImageGeneration,
        ContractInferenceTaskId::AudioTranscription,
    ]
}

fn llm_supported_registry_task_ids() -> [InferenceTaskId; 6] {
    [
        InferenceTaskId::TextGeneration,
        InferenceTaskId::ChatCompletion,
        InferenceTaskId::Embedding,
        InferenceTaskId::Rerank,
        InferenceTaskId::ImageGeneration,
        InferenceTaskId::AudioTranscription,
    ]
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
        "llm-inference" | "onnx-inference" => vec![NodeCapabilityRequirement::required("llm")],
        "audio-generation" => vec![NodeCapabilityRequirement::required("audio_generation")],
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
        assert_eq!(prompt.requirement, PortRequirement::Optional);

        let text = llm
            .inputs
            .iter()
            .find(|port| port.id.as_str() == "text")
            .expect("text port");
        assert_eq!(text.kind, PortKind::Input);
        assert_eq!(text.value_type, PortValueType::String);
        assert_eq!(text.requirement, PortRequirement::Optional);

        let response = llm
            .outputs
            .iter()
            .find(|port| port.id.as_str() == "response")
            .expect("response port");
        assert_eq!(response.kind, PortKind::Output);
        assert_eq!(response.value_type, PortValueType::String);
    }

    #[test]
    fn llm_inference_contract_exposes_inference_task_payload_metadata() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let llm = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "llm-inference")
            .expect("llm contract");

        assert!(llm.inference_tasks.iter().any(|task| {
            task.task_id == ContractInferenceTaskId::TextGeneration
                && task.input_kind == ContractInferenceExecutionInputKind::TextGeneration
                && task.result_kind == ContractInferenceExecutionResultKind::TextGeneration
        }));
        assert!(llm.inference_tasks.iter().any(|task| {
            task.task_id == ContractInferenceTaskId::Embedding
                && task.input_kind == ContractInferenceExecutionInputKind::Embedding
                && task.result_kind == ContractInferenceExecutionResultKind::Embedding
        }));
        assert!(llm.inference_tasks.iter().any(|task| {
            task.task_id == ContractInferenceTaskId::Rerank
                && task.input_kind == ContractInferenceExecutionInputKind::Rerank
                && task.result_kind == ContractInferenceExecutionResultKind::Rerank
        }));
        assert!(llm.inference_tasks.iter().any(|task| {
            task.task_id == ContractInferenceTaskId::ImageGeneration
                && task.input_kind == ContractInferenceExecutionInputKind::ImageGeneration
                && task.result_kind == ContractInferenceExecutionResultKind::ImageGeneration
                && task.execution_supported
        }));
        assert!(llm.inference_tasks.iter().any(|task| {
            task.task_id == ContractInferenceTaskId::AudioTranscription
                && task.input_kind == ContractInferenceExecutionInputKind::AudioTranscription
                && task.result_kind == ContractInferenceExecutionResultKind::AudioTranscription
                && task.execution_supported
        }));
        let registry_entries = default_task_registry_entries();
        let registry_embedding = registry_entries
            .iter()
            .find(|entry| entry.task_id == InferenceTaskId::Embedding)
            .and_then(|entry| entry.request_contract())
            .expect("embedding registry contract");
        let projected_embedding = llm
            .inference_tasks
            .iter()
            .find(|task| task.task_id == ContractInferenceTaskId::Embedding)
            .expect("projected embedding contract");
        assert_eq!(
            projected_embedding.execution_supported,
            registry_embedding.execution_supported
        );
        assert_eq!(
            projected_embedding.streaming_support,
            contract_streaming_support(registry_embedding.streaming_support)
        );
        let registry_audio = registry_entries
            .iter()
            .find(|entry| entry.task_id == InferenceTaskId::AudioTranscription)
            .and_then(|entry| entry.request_contract())
            .expect("audio transcription registry contract");
        let projected_audio = llm
            .inference_tasks
            .iter()
            .find(|task| task.task_id == ContractInferenceTaskId::AudioTranscription)
            .expect("projected audio transcription contract");
        assert_eq!(
            projected_audio.execution_supported,
            registry_audio.execution_supported
        );
        assert_eq!(
            projected_audio.streaming_support,
            contract_streaming_support(registry_audio.streaming_support)
        );

        let prompt = llm
            .input(&port_id("prompt").expect("prompt port id"))
            .unwrap();
        assert!(prompt.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::TextGeneration
                && payload.input_kind == Some(ContractInferenceExecutionInputKind::TextGeneration)
        }));
        assert!(prompt.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::ImageGeneration
                && payload.input_kind == Some(ContractInferenceExecutionInputKind::ImageGeneration)
        }));

        let task_kind = llm
            .input(&port_id("task_kind").expect("task kind port id"))
            .unwrap();
        assert!(task_kind.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Rerank
                && payload.role == InferencePortPayloadRole::Options
        }));

        let audio = llm
            .input(&port_id("audio").expect("audio port id"))
            .unwrap();
        assert!(audio.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::AudioTranscription
                && payload.input_kind
                    == Some(ContractInferenceExecutionInputKind::AudioTranscription)
        }));

        let text = llm.input(&port_id("text").expect("text port id")).unwrap();
        assert!(text.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Embedding
                && payload.input_kind == Some(ContractInferenceExecutionInputKind::Embedding)
        }));

        let pumas_model_ref = llm
            .input(&port_id("pumas_model_ref").expect("pumas model ref port id"))
            .unwrap();
        assert!(pumas_model_ref.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Embedding
                && payload.role == InferencePortPayloadRole::ModelReference
        }));

        let denoising_scheduler = llm
            .input(&port_id("denoising_scheduler").expect("denoising scheduler port id"))
            .unwrap();
        assert_eq!(denoising_scheduler.inference_payloads.len(), 1);
        assert!(denoising_scheduler
            .inference_payloads
            .iter()
            .any(|payload| {
                payload.task_id == ContractInferenceTaskId::ImageGeneration
                    && payload.role == InferencePortPayloadRole::Options
            }));

        assert!(llm
            .input(&port_id("resolved_model_source").expect("resolved model source port id"))
            .is_none());
        assert!(llm
            .input(
                &port_id("resolved_model_package_facts")
                    .expect("resolved model package facts port id"),
            )
            .is_none());

        let results = llm
            .output(&port_id("results").expect("results port id"))
            .unwrap();
        assert!(results.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Rerank
                && payload.result_kind == Some(ContractInferenceExecutionResultKind::Rerank)
        }));
        assert!(results.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::ImageGeneration
                && payload.result_kind
                    == Some(ContractInferenceExecutionResultKind::ImageGeneration)
        }));

        let usage = llm
            .output(&port_id("usage").expect("usage port id"))
            .unwrap();
        assert!(usage.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::TextGeneration
                && payload.role == InferencePortPayloadRole::Usage
                && payload.result_kind.is_none()
        }));

        let kv_cache_out = llm
            .output(&port_id("kv_cache_out").expect("kv cache out port id"))
            .unwrap();
        assert!(kv_cache_out.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::TextGeneration
                && payload.role == InferencePortPayloadRole::CacheHandle
                && payload.result_kind.is_none()
        }));
        assert!(kv_cache_out
            .inference_payloads
            .iter()
            .all(|payload| { payload.role == InferencePortPayloadRole::CacheHandle }));

        let response = llm
            .output(&port_id("response").expect("response port id"))
            .unwrap();
        assert!(response.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::AudioTranscription
                && payload.result_kind
                    == Some(ContractInferenceExecutionResultKind::AudioTranscription)
        }));
    }

    #[test]
    fn llm_inference_contract_exposes_backend_neutral_diagnostics_payloads() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let llm = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "llm-inference")
            .expect("llm contract");
        let expected_tasks = llm_supported_task_ids().to_vec();

        for output_id in ["diagnostics", "metadata"] {
            let port = llm.output(&port_id(output_id).expect("diagnostics port id"));
            let port = port.expect("diagnostics output port");
            let projected_tasks = port
                .inference_payloads
                .iter()
                .map(|payload| payload.task_id)
                .collect::<Vec<_>>();

            assert_eq!(projected_tasks, expected_tasks);
            assert!(port.inference_payloads.iter().all(|payload| {
                payload.role == InferencePortPayloadRole::Diagnostics
                    && payload.input_kind.is_none()
                    && payload.result_kind.is_none()
            }));
            let serialized =
                serde_json::to_string(&port.inference_payloads).expect("payload serialization");
            assert!(!serialized.contains("backend_key"));
            assert!(!serialized.contains("runtime_id"));
            assert!(!serialized.contains("scheduler"));
            assert!(!serialized.contains("reservation"));
        }
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

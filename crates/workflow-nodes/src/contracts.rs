//! Canonical contract projection for concrete workflow node registrations.
//!
//! Concrete node implementations still expose execution descriptors through
//! `node-engine`, while this module projects those descriptors into
//! `pantograph-node-contracts` for graph-authoring and binding surfaces.

use inference::model_contracts::{
    default_task_registry_entries, InferenceExecutionInputKind, InferenceExecutionResultKind,
    InferenceModality, InferenceTaskId, TaskRequestContract, TaskStreamingSupport,
};
use std::collections::HashSet;

use pantograph_node_contracts::{
    ContractInferenceExecutionInputKind, ContractInferenceExecutionResultKind,
    ContractInferenceModality, ContractInferenceStreamingSupport, ContractInferenceTaskId,
    InferencePortPayloadContract, InferencePortPayloadRole, NodeAuthoringMetadata,
    NodeCapabilityRequirement, NodeCategory, NodeContractError, NodeExecutionSemantics,
    NodeInferenceTaskContract, NodeTypeContract, NodeTypeId, PortCardinality, PortContract, PortId,
    PortKind, PortOptionsProviderRef, PortRequirement, PortValueType, PortVisibility,
};

pub fn builtin_node_contracts() -> Result<Vec<NodeTypeContract>, NodeContractError> {
    let registry = node_engine::NodeRegistry::with_builtins();
    let queryable_ports = registry
        .queryable_ports()
        .into_iter()
        .map(|(node_type, port_id)| (node_type.to_string(), port_id.to_string()))
        .collect::<HashSet<_>>();
    let mut contracts = registry
        .all_metadata()
        .into_iter()
        .map(|metadata| task_metadata_to_contract_with_options(metadata, &queryable_ports))
        .collect::<Result<Vec<_>, _>>()?;
    contracts.sort_by(|left, right| left.node_type.as_str().cmp(right.node_type.as_str()));
    Ok(contracts)
}

pub fn task_metadata_to_contract(
    metadata: &node_engine::TaskMetadata,
) -> Result<NodeTypeContract, NodeContractError> {
    task_metadata_to_contract_with_options(metadata, &HashSet::new())
}

fn task_metadata_to_contract_with_options(
    metadata: &node_engine::TaskMetadata,
    queryable_ports: &HashSet<(String, String)>,
) -> Result<NodeTypeContract, NodeContractError> {
    let node_type = NodeTypeId::try_from(metadata.node_type.clone())?;
    let inputs = metadata
        .inputs
        .iter()
        .map(|port| port_metadata_to_contract(&node_type, PortKind::Input, port, queryable_ports))
        .collect::<Result<Vec<_>, _>>()?;
    let outputs = metadata
        .outputs
        .iter()
        .map(|port| port_metadata_to_contract(&node_type, PortKind::Output, port, queryable_ports))
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

#[cfg(test)]
fn port_id(value: &str) -> Result<PortId, NodeContractError> {
    value.parse()
}

fn port_metadata_to_contract(
    node_type: &NodeTypeId,
    kind: PortKind,
    metadata: &node_engine::PortMetadata,
    queryable_ports: &HashSet<(String, String)>,
) -> Result<PortContract, NodeContractError> {
    let port_id = PortId::try_from(metadata.id.clone())?;
    let options_provider = if queryable_ports
        .contains(&(node_type.as_str().to_string(), port_id.as_str().to_string()))
    {
        Some(PortOptionsProviderRef::new(
            node_type.clone(),
            port_id.clone(),
        ))
    } else {
        None
    };
    let contract = PortContract {
        id: port_id,
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
        options_provider,
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
        "task_kind" | "runtime" | "device" => {
            task_role_payloads(&llm_supported_task_ids(), InferencePortPayloadRole::Options)
        }
        _ => Vec::new(),
    }
}

fn llm_output_payloads(port_id: &str) -> Vec<InferencePortPayloadContract> {
    match port_id {
        "diagnostics" => task_role_payloads(
            &llm_supported_task_ids(),
            InferencePortPayloadRole::Diagnostics,
        ),
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
        node_engine::PortDataType::DependencyEnvironmentSidecar => {
            PortValueType::DependencyEnvironmentSidecar
        }
    }
}

fn capability_requirements(metadata: &node_engine::TaskMetadata) -> Vec<NodeCapabilityRequirement> {
    match metadata.node_type.as_str() {
        "llm-inference" => vec![NodeCapabilityRequirement::required("llm")],
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
    fn contract_projection_preserves_port_directions_and_value_types() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let llm = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "llm-inference")
            .expect("llm contract");

        let pumas_model_ref = llm
            .inputs
            .iter()
            .find(|port| port.id.as_str() == "pumas_model_ref")
            .expect("pumas_model_ref port");
        assert_eq!(pumas_model_ref.kind, PortKind::Input);
        assert_eq!(pumas_model_ref.value_type, PortValueType::Json);
        assert_eq!(pumas_model_ref.requirement, PortRequirement::Optional);

        let dependency_sidecar = llm
            .inputs
            .iter()
            .find(|port| port.id.as_str() == "dependency_environment_sidecar")
            .expect("dependency sidecar port");
        assert_eq!(dependency_sidecar.kind, PortKind::Input);
        assert_eq!(
            dependency_sidecar.value_type,
            PortValueType::DependencyEnvironmentSidecar
        );
        assert_eq!(dependency_sidecar.requirement, PortRequirement::Optional);

        let diagnostics = llm
            .outputs
            .iter()
            .find(|port| port.id.as_str() == "diagnostics")
            .expect("diagnostics port");
        assert_eq!(diagnostics.kind, PortKind::Output);
        assert_eq!(diagnostics.value_type, PortValueType::Json);

        for retired_port in [
            "prompt",
            "text",
            "query",
            "documents",
            "documents_json",
            "audio",
            "system_prompt",
            "context",
            "tools",
            "kv_cache_in",
            "generation_options",
            "task_options",
            "denoising_scheduler",
            "inference_settings",
            "response",
            "results",
            "scores",
            "top_document",
            "top_score",
            "embedding",
            "image",
            "metadata",
            "model_ref",
            "tool_calls",
            "has_tool_calls",
            "kv_cache_out",
            "stream",
            "usage",
        ] {
            assert!(
                llm.input(&port_id(retired_port).expect("retired input port id"))
                    .is_none(),
                "retired static input port {retired_port} must come from descriptors"
            );
            assert!(
                llm.output(&port_id(retired_port).expect("retired output port id"))
                    .is_none(),
                "retired static output port {retired_port} must come from descriptors"
            );
        }
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

        let task_kind = llm
            .input(&port_id("task_kind").expect("task kind port id"))
            .unwrap();
        assert!(task_kind.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Rerank
                && payload.role == InferencePortPayloadRole::Options
        }));

        let pumas_model_ref = llm
            .input(&port_id("pumas_model_ref").expect("pumas model ref port id"))
            .unwrap();
        assert!(pumas_model_ref.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::Embedding
                && payload.role == InferencePortPayloadRole::ModelReference
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

        assert!(llm
            .output(&port_id("results").expect("results port id"))
            .is_none());
        assert!(llm
            .output(&port_id("response").expect("response port id"))
            .is_none());
    }

    #[test]
    fn llm_inference_contract_exposes_backend_neutral_diagnostics_payloads() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let llm = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "llm-inference")
            .expect("llm contract");
        let expected_tasks = llm_supported_task_ids().to_vec();

        let port = llm.output(&port_id("diagnostics").expect("diagnostics port id"));
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
        assert!(llm
            .output(&port_id("metadata").expect("metadata port id"))
            .is_none());
    }

    #[test]
    fn builtin_contracts_preserve_registered_port_options_provider_refs() {
        let contracts = builtin_node_contracts().expect("canonical contracts");
        let puma_lib = contracts
            .iter()
            .find(|contract| contract.node_type.as_str() == "puma-lib")
            .expect("puma-lib contract");
        assert!(
            puma_lib
                .output(&port_id("model_path").expect("model path port id"))
                .is_none(),
            "puma-lib must not expose model_path as an executable output"
        );
        assert!(
            puma_lib
                .output(&port_id("backend_key").expect("backend key port id"))
                .is_none(),
            "puma-lib must not expose backend_key as an executable output"
        );
        let pumas_model_ref = puma_lib
            .output(&port_id("pumas_model_ref").expect("pumas model ref port id"))
            .expect("pumas model ref output");

        #[cfg(feature = "model-library")]
        {
            let provider = pumas_model_ref
                .options_provider
                .as_ref()
                .expect("registered options provider");
            assert_eq!(provider.node_type.as_str(), "puma-lib");
            assert_eq!(provider.port_id.as_str(), "pumas_model_ref");

            let serialized = serde_json::to_value(pumas_model_ref).expect("provider serialization");
            assert_eq!(serialized["options_provider"]["node_type"], "puma-lib");
            assert_eq!(serialized["options_provider"]["port_id"], "pumas_model_ref");

            let llm_inference = contracts
                .iter()
                .find(|contract| contract.node_type.as_str() == "llm-inference")
                .expect("llm-inference contract");
            assert!(
                llm_inference
                    .input(&port_id("denoising_scheduler").expect("denoising scheduler port id"))
                    .is_none(),
                "denoising scheduler options must come from descriptor-backed option sets"
            );
        }

        #[cfg(not(feature = "model-library"))]
        {
            assert!(pumas_model_ref.options_provider.is_none());
        }
    }

    #[test]
    fn projection_preserves_extended_engine_value_types() {
        let metadata = node_engine::TaskMetadata {
            node_type: "extended-types".to_string(),
            category: node_engine::NodeCategory::Processing,
            label: "Extended Types".to_string(),
            description: "Preserves engine-only value types".to_string(),
            inputs: vec![
                node_engine::PortMetadata::required(
                    "model",
                    "Model",
                    node_engine::PortDataType::ModelHandle,
                ),
                node_engine::PortMetadata::optional(
                    "dependency_environment_sidecar",
                    "Dependency Environment",
                    node_engine::PortDataType::DependencyEnvironmentSidecar,
                ),
            ],
            outputs: vec![node_engine::PortMetadata::optional(
                "tensor",
                "Tensor",
                node_engine::PortDataType::Tensor,
            )],
            execution_mode: node_engine::ExecutionMode::Batch,
        };

        let contract = task_metadata_to_contract(&metadata).expect("contract");

        assert_eq!(contract.inputs[0].value_type, PortValueType::ModelHandle);
        assert_eq!(
            contract.inputs[1].value_type,
            PortValueType::DependencyEnvironmentSidecar
        );
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

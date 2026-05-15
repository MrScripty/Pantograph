use std::collections::HashMap;

use super::types::{
    ExecutionMode, IoBindingOrigin, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

fn convert_contract(contract: &pantograph_node_contracts::NodeTypeContract) -> NodeDefinition {
    let category = convert_category(contract.category);
    NodeDefinition {
        node_type: contract.node_type.as_str().to_string(),
        category: category.clone(),
        label: contract.label.clone(),
        description: contract.description.clone(),
        io_binding_origin: determine_io_binding_origin(contract.node_type.as_str(), &category),
        inputs: contract.inputs.iter().map(convert_port).collect(),
        outputs: contract.outputs.iter().map(convert_port).collect(),
        execution_mode: convert_execution_mode(contract.execution_semantics),
    }
}

fn determine_io_binding_origin(node_type: &str, category: &NodeCategory) -> IoBindingOrigin {
    if !matches!(category, NodeCategory::Input | NodeCategory::Output) {
        return IoBindingOrigin::Integrated;
    }

    match node_type {
        "puma-lib" | "linked-input" | "model-provider" | "component-preview"
        | "point-cloud-output" => IoBindingOrigin::Integrated,
        "audio-input" | "boolean-input" | "human-input" | "image-input" | "masked-text-input"
        | "number-input" | "selection-input" | "text-input" | "vector-input" | "audio-output"
        | "image-output" | "text-output" | "vector-output" => IoBindingOrigin::ClientSession,
        _ => panic!(
            "input/output node type '{}' is missing explicit io_binding_origin mapping",
            node_type
        ),
    }
}

fn convert_category(cat: pantograph_node_contracts::NodeCategory) -> NodeCategory {
    match cat {
        pantograph_node_contracts::NodeCategory::Input => NodeCategory::Input,
        pantograph_node_contracts::NodeCategory::Output => NodeCategory::Output,
        pantograph_node_contracts::NodeCategory::Processing => NodeCategory::Processing,
        pantograph_node_contracts::NodeCategory::Control => NodeCategory::Control,
        pantograph_node_contracts::NodeCategory::Tool => NodeCategory::Tool,
    }
}

fn convert_execution_mode(
    mode: pantograph_node_contracts::NodeExecutionSemantics,
) -> ExecutionMode {
    match mode {
        pantograph_node_contracts::NodeExecutionSemantics::Batch => ExecutionMode::Reactive,
        pantograph_node_contracts::NodeExecutionSemantics::Stream => ExecutionMode::Stream,
        pantograph_node_contracts::NodeExecutionSemantics::Reactive => ExecutionMode::Reactive,
        pantograph_node_contracts::NodeExecutionSemantics::Manual => ExecutionMode::Manual,
    }
}

pub(super) fn convert_port(port: &pantograph_node_contracts::PortContract) -> PortDefinition {
    PortDefinition {
        id: port.id.as_str().to_string(),
        label: port.label.clone(),
        data_type: PortDataType::from_contract_value_type(port.value_type),
        required: matches!(
            port.requirement,
            pantograph_node_contracts::PortRequirement::Required
        ),
        multiple: matches!(
            port.cardinality,
            pantograph_node_contracts::PortCardinality::Multiple
        ),
        options_provider: port.options_provider.clone(),
        inference_payloads: port.inference_payloads.clone(),
    }
}

pub fn validate_workflow_connection(
    source_type: &PortDataType,
    target_type: &PortDataType,
) -> bool {
    source_type.is_compatible_with(target_type)
}

pub struct NodeRegistry {
    definitions: HashMap<String, NodeDefinition>,
    contracts: HashMap<String, pantograph_node_contracts::NodeTypeContract>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut definitions = HashMap::new();
        let mut contracts_by_type = HashMap::new();
        let contracts = workflow_nodes::builtin_node_contracts()
            .expect("built-in workflow node descriptors must project to canonical contracts");
        for contract in contracts {
            let def = convert_contract(&contract);
            contracts_by_type.insert(def.node_type.clone(), contract);
            definitions.insert(def.node_type.clone(), def);
        }
        Self {
            definitions,
            contracts: contracts_by_type,
        }
    }

    pub fn get_definition(&self, node_type: &str) -> Option<&NodeDefinition> {
        self.definitions.get(node_type)
    }

    pub fn get_contract(
        &self,
        node_type: &str,
    ) -> Option<&pantograph_node_contracts::NodeTypeContract> {
        self.contracts.get(node_type)
    }

    pub fn all_definitions(&self) -> Vec<NodeDefinition> {
        self.definitions.values().cloned().collect()
    }

    pub fn definitions_by_category(&self) -> HashMap<String, Vec<NodeDefinition>> {
        let mut grouped = HashMap::new();
        for def in self.definitions.values() {
            let category = format!("{:?}", def.category).to_lowercase();
            grouped
                .entry(category)
                .or_insert_with(Vec::new)
                .push(def.clone());
        }
        grouped
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_node_contracts::{
        ContractInferenceExecutionInputKind, ContractInferenceExecutionResultKind,
        ContractInferenceTaskId, InferencePortPayloadRole,
    };

    #[test]
    fn node_definition_preserves_inference_payload_contracts_for_llm_diagnostics() {
        let registry = NodeRegistry::new();
        let definition = registry
            .get_definition("llm-inference")
            .expect("llm-inference definition");
        let diagnostics = definition
            .outputs
            .iter()
            .find(|port| port.id == "diagnostics")
            .expect("diagnostics output");

        assert!(diagnostics.inference_payloads.iter().any(|payload| {
            payload.role == InferencePortPayloadRole::Diagnostics
                && payload.task_id == ContractInferenceTaskId::TextGeneration
                && payload.input_kind.is_none()
                && payload.result_kind.is_none()
        }));

        let encoded = serde_json::to_value(diagnostics).expect("encode diagnostics port");
        let payload = &encoded["inference_payloads"][0];
        assert_eq!(payload["role"], serde_json::json!("diagnostics"));
        assert_eq!(payload["task_id"], serde_json::json!("text_generation"));
        assert_llm_inference_payloads_do_not_expose_runtime_policy(&definition);
    }

    #[test]
    fn node_definition_preserves_image_generation_payload_contracts() {
        let registry = NodeRegistry::new();
        let definition = registry
            .get_definition("llm-inference")
            .expect("llm-inference definition");
        let prompt = definition
            .inputs
            .iter()
            .find(|port| port.id == "prompt")
            .expect("prompt input");
        let results = definition
            .outputs
            .iter()
            .find(|port| port.id == "results")
            .expect("results output");

        assert!(prompt.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::ImageGeneration
                && payload.input_kind == Some(ContractInferenceExecutionInputKind::ImageGeneration)
        }));
        assert!(results.inference_payloads.iter().any(|payload| {
            payload.task_id == ContractInferenceTaskId::ImageGeneration
                && payload.result_kind
                    == Some(ContractInferenceExecutionResultKind::ImageGeneration)
        }));

        let encoded = serde_json::to_value(results).expect("encode results port");
        assert!(encoded["inference_payloads"]
            .as_array()
            .is_some_and(|payloads| {
                payloads.iter().any(|payload| {
                    payload["task_id"] == serde_json::json!("image_generation")
                        && payload["result_kind"] == serde_json::json!("image_generation")
                })
            }));
    }

    #[test]
    fn node_definition_preserves_registered_port_options_provider_refs() {
        let registry = NodeRegistry::new();
        let definition = registry
            .get_definition("puma-lib")
            .expect("puma-lib definition");
        let model_path = definition
            .outputs
            .iter()
            .find(|port| port.id == "model_path")
            .expect("model path output");

        let provider = model_path
            .options_provider
            .as_ref()
            .expect("registered options provider");
        assert_eq!(provider.node_type.as_str(), "puma-lib");
        assert_eq!(provider.port_id.as_str(), "model_path");

        let encoded = serde_json::to_value(model_path).expect("encode model path port");
        assert_eq!(
            encoded["options_provider"],
            serde_json::json!({
                "node_type": "puma-lib",
                "port_id": "model_path"
            })
        );
    }

    fn assert_llm_inference_payloads_do_not_expose_runtime_policy(definition: &NodeDefinition) {
        let policy_fields = [
            "backend_key",
            "runtime_id",
            "runtime_instance_id",
            "selected_backend_key",
            "selected_runtime_id",
            "scheduler_policy",
            "scheduler_policy_id",
            "admission",
            "reservation",
            "eviction",
            "priority",
        ];

        for port in definition.inputs.iter().chain(definition.outputs.iter()) {
            let encoded =
                serde_json::to_value(&port.inference_payloads).expect("encode inference payloads");
            let payloads = encoded.as_array().expect("payloads encode as array");
            for payload in payloads {
                for field in policy_fields {
                    assert!(
                        payload.get(field).is_none(),
                        "port '{}' payload unexpectedly exposes policy field '{}': {}",
                        port.id,
                        field,
                        payload
                    );
                }
            }
        }
    }
}

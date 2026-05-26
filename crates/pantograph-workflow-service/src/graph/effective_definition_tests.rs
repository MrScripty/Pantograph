use serde_json::json;

use pantograph_node_contracts::{
    ContractExpansionReason, ContractInferenceExecutionResultKind, ContractInferenceTaskId,
    InferencePortPayloadRole,
};

use super::effective_definition::{
    effective_node_contract, effective_node_definition, EffectiveDefinitionError,
};
use super::{GraphNode, NodeRegistry, PortDataType, Position};

#[test]
fn effective_node_definition_merges_dynamic_ports_without_dropping_static_ports() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "text".to_string(),
        node_type: "text-input".to_string(),
        position: Position::default(),
        data: json!({
            "definition": {
                "node_type": "text-input",
                "inputs": [
                    {
                        "id": "temperature",
                        "label": "Temperature",
                        "data_type": "number",
                        "required": false,
                        "multiple": false
                    }
                ]
            }
        }),
    };

    let definition = effective_node_definition(&node, &registry).expect("definition");

    assert!(
        definition.inputs.iter().any(|port| port.id == "text"),
        "static text input must remain available"
    );
    assert_eq!(
        definition
            .inputs
            .iter()
            .find(|port| port.id == "temperature")
            .map(|port| &port.data_type),
        Some(&PortDataType::Number)
    );
}

#[test]
fn effective_node_definition_preserves_dynamic_inference_payload_contracts() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "text".to_string(),
        node_type: "text-output".to_string(),
        position: Position::default(),
        data: json!({
            "definition": {
                "node_type": "text-output",
                "outputs": [
                    {
                        "id": "rerank_debug_results",
                        "label": "Rerank Debug Results",
                        "data_type": "json",
                        "required": false,
                        "multiple": false,
                        "inference_payloads": [
                            {
                                "task_id": "rerank",
                                "role": "task_output",
                                "result_kind": "rerank"
                            }
                        ]
                    }
                ]
            }
        }),
    };

    let definition = effective_node_definition(&node, &registry).expect("definition");
    let dynamic_output = definition
        .outputs
        .iter()
        .find(|port| port.id == "rerank_debug_results")
        .expect("dynamic output");
    assert_eq!(dynamic_output.inference_payloads.len(), 1);
    let payload = &dynamic_output.inference_payloads[0];
    assert_eq!(payload.task_id, ContractInferenceTaskId::Rerank);
    assert_eq!(payload.role, InferencePortPayloadRole::TaskOutput);
    assert_eq!(
        payload.result_kind,
        Some(ContractInferenceExecutionResultKind::Rerank)
    );

    let effective = effective_node_contract(&node, &registry).expect("contract");
    let contract_output = effective
        .outputs
        .iter()
        .find(|port| port.base.id.as_str() == "rerank_debug_results")
        .expect("dynamic contract output");
    assert_eq!(
        contract_output.base.inference_payloads,
        dynamic_output.inference_payloads
    );

    let encoded = serde_json::to_value(dynamic_output).expect("dynamic output encodes");
    assert_eq!(
        encoded["inference_payloads"][0]["task_id"],
        serde_json::json!("rerank")
    );
    assert_eq!(
        encoded["inference_payloads"][0]["role"],
        serde_json::json!("task_output")
    );
    assert_eq!(
        encoded["inference_payloads"][0]["result_kind"],
        serde_json::json!("rerank")
    );
}

#[test]
fn effective_node_contract_reports_mismatched_dynamic_definition() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "text".to_string(),
        node_type: "text-input".to_string(),
        position: Position::default(),
        data: json!({
            "definition": {
                "node_type": "text-output",
                "inputs": [
                    {
                        "id": "foreign_dynamic_input",
                        "label": "Foreign Dynamic Input",
                        "data_type": "number"
                    }
                ]
            }
        }),
    };

    let effective = effective_node_contract(&node, &registry).expect("contract");

    assert!(
        effective
            .inputs
            .iter()
            .all(|port| port.base.id.as_str() != "foreign_dynamic_input"),
        "mismatched dynamic definition must not add ports"
    );
    assert_eq!(
        effective.diagnostics.warnings[0].code,
        "dynamic_node_type_mismatch"
    );
}

#[test]
fn effective_node_contract_records_dynamic_expansion_reason() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "text".to_string(),
        node_type: "text-input".to_string(),
        position: Position::default(),
        data: json!({
            "definition": {
                "node_type": "text-input",
                "inputs": [
                    {
                        "id": "temperature",
                        "label": "Temperature",
                        "data_type": "number"
                    }
                ]
            }
        }),
    };

    let effective = effective_node_contract(&node, &registry).expect("contract");

    assert_eq!(
        effective.diagnostics.expansion_reasons,
        vec![ContractExpansionReason::DynamicConfiguration]
    );
    assert_eq!(
        effective
            .inputs
            .last()
            .expect("dynamic port")
            .expansion_reasons,
        vec![ContractExpansionReason::DynamicConfiguration]
    );
}

#[test]
fn effective_node_contract_rejects_inference_node_data_definition() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "llm".to_string(),
        node_type: "llm-inference".to_string(),
        position: Position::default(),
        data: json!({
            "definition": {
                "node_type": "llm-inference",
                "inputs": [
                    {
                        "id": "temperature",
                        "label": "Temperature",
                        "data_type": "number"
                    }
                ]
            }
        }),
    };

    let error = effective_node_contract(&node, &registry)
        .expect_err("inference node definitions must not be semantic fallbacks");

    assert!(matches!(
        error,
        EffectiveDefinitionError::InvalidDynamicDefinition { .. }
    ));
}

#[test]
fn effective_node_definition_projects_authored_inference_snapshot_ports() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "llm".to_string(),
        node_type: "llm-inference".to_string(),
        position: Position::default(),
        data: json!({
            "inference_interface_snapshot": authored_snapshot_json(),
        }),
    };

    let definition = effective_node_definition(&node, &registry).expect("definition");

    assert_eq!(
        definition
            .inputs
            .iter()
            .find(|port| port.id == "descriptor_prompt")
            .map(|port| (&port.label, &port.data_type, port.required)),
        Some((
            &"Descriptor Prompt".to_string(),
            &PortDataType::String,
            true
        ))
    );
    assert_eq!(
        definition
            .outputs
            .iter()
            .find(|port| port.id == "descriptor_image")
            .map(|port| (&port.label, &port.data_type, port.required)),
        Some((&"Descriptor Image".to_string(), &PortDataType::Image, true))
    );
}

#[test]
fn effective_node_definition_ignores_inference_definition_when_snapshot_exists() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "llm".to_string(),
        node_type: "llm-inference".to_string(),
        position: Position::default(),
        data: json!({
            "inference_interface_snapshot": authored_snapshot_json(),
            "definition": {
                "node_type": "llm-inference",
                "inputs": [
                    {
                        "id": "legacy_temperature",
                        "label": "Legacy Temperature",
                        "data_type": "number"
                    }
                ]
            }
        }),
    };

    let definition = effective_node_definition(&node, &registry).expect("definition");

    assert!(definition
        .inputs
        .iter()
        .any(|port| port.id == "descriptor_prompt"));
    assert!(definition
        .inputs
        .iter()
        .all(|port| port.id != "legacy_temperature"));
}

#[test]
fn effective_node_contract_rejects_invalid_inference_snapshot() {
    let registry = NodeRegistry::new();
    let node = GraphNode {
        id: "llm".to_string(),
        node_type: "llm-inference".to_string(),
        position: Position::default(),
        data: json!({
            "inference_interface_snapshot": {
                "contract_version": 1,
                "descriptor_fingerprint": "iface.invalid",
                "task_kind": "text_to_image",
                "unexpected_field": true
            },
        }),
    };

    let error = effective_node_contract(&node, &registry)
        .expect_err("invalid snapshot must block interface projection");

    assert!(matches!(
        error,
        EffectiveDefinitionError::InvalidDynamicDefinition { .. }
    ));
}

fn authored_snapshot_json() -> serde_json::Value {
    json!({
        "contract_version": 1,
        "descriptor_fingerprint": "iface.test.text_to_image.v1",
        "task_kind": "text_to_image",
        "inputs": [
            {
                "port_id": "descriptor_prompt",
                "label": "Descriptor Prompt",
                "direction": "input",
                "requirement": "required",
                "value_type": {
                    "category": "scalar",
                    "kind": "string"
                },
                "availability": {
                    "status": "available"
                }
            }
        ],
        "outputs": [
            {
                "port_id": "descriptor_image",
                "label": "Descriptor Image",
                "direction": "output",
                "requirement": "required",
                "value_type": {
                    "category": "artifact",
                    "kind": "image"
                },
                "availability": {
                    "status": "available"
                }
            }
        ]
    })
}

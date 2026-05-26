use std::collections::BTreeMap;

use pantograph_dependency_planning::{DeviceIntentId, PumasModelRef, RuntimeIntentId};
use pantograph_inference_interface_contracts::{
    InferenceTaskKind, ResolveInferenceInterfaceRequest, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::WorkflowGraph;

const NODE_TYPE_GENERIC_INFERENCE: &str = "llm-inference";
const PORT_PUMAS_MODEL_REF: &str = "pumas_model_ref";
const PORT_TASK_KIND: &str = "task_kind";
const PORT_RUNTIME: &str = "runtime";
const PORT_DEVICE: &str = "device";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceGraphResolutionInput {
    pub node_id: String,
    pub request: ResolveInferenceInterfaceRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceGraphResolutionInputs {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requests: Vec<InferenceInterfaceGraphResolutionInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceGraphResolutionDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceGraphResolutionDiagnostic {
    pub node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    pub code: InferenceInterfaceGraphResolutionDiagnosticCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceInterfaceGraphResolutionDiagnosticCode {
    MissingPumasModelRef,
    InvalidPumasModelRef,
    InvalidTaskKind,
    InvalidRuntimeConstraint,
    InvalidDeviceConstraint,
}

pub fn inference_interface_resolution_inputs_from_graph(
    graph: &WorkflowGraph,
) -> InferenceInterfaceGraphResolutionInputs {
    let nodes_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), &node.data))
        .collect::<BTreeMap<_, _>>();
    let incoming_model_refs = incoming_model_ref_sources(graph);
    let mut requests = Vec::new();
    let mut diagnostics = Vec::new();

    for node in graph
        .nodes
        .iter()
        .filter(|node| node.node_type == NODE_TYPE_GENERIC_INFERENCE)
    {
        let Some(model_ref) = model_ref_for_inference_node(
            &node.id,
            &node.data,
            &incoming_model_refs,
            &nodes_by_id,
            &mut diagnostics,
        ) else {
            continue;
        };
        let Some(task_kind) = optional_task_kind(&node.id, &node.data, &mut diagnostics) else {
            continue;
        };
        let Some(runtime_constraint) =
            optional_runtime_constraint(&node.id, &node.data, &mut diagnostics)
        else {
            continue;
        };
        let Some(device_constraint) =
            optional_device_constraint(&node.id, &node.data, &mut diagnostics)
        else {
            continue;
        };
        let request = ResolveInferenceInterfaceRequest {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            model_ref,
            task_kind,
            runtime_constraint,
            device_constraint,
        };
        if let Err(error) = request.validate() {
            diagnostics.push(diagnostic(
                &node.id,
                None,
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRef,
                format!("inference interface request is invalid: {error}"),
            ));
            continue;
        }
        requests.push(InferenceInterfaceGraphResolutionInput {
            node_id: node.id.clone(),
            request,
        });
    }

    InferenceInterfaceGraphResolutionInputs {
        requests,
        diagnostics,
    }
}

fn incoming_model_ref_sources(graph: &WorkflowGraph) -> BTreeMap<&str, &str> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.target_handle == PORT_PUMAS_MODEL_REF)
        .map(|edge| (edge.target.as_str(), edge.source.as_str()))
        .collect()
}

fn model_ref_for_inference_node(
    node_id: &str,
    node_data: &Value,
    incoming_model_refs: &BTreeMap<&str, &str>,
    nodes_by_id: &BTreeMap<&str, &Value>,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<PumasModelRef> {
    if let Some(source_node_id) = incoming_model_refs.get(node_id) {
        let Some(source_data) = nodes_by_id.get(source_node_id) else {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::MissingPumasModelRef,
                "connected pumas_model_ref source node is missing from the graph",
            ));
            return None;
        };
        return parse_model_ref(node_id, source_data.get(PORT_PUMAS_MODEL_REF), diagnostics);
    }

    parse_model_ref(node_id, node_data.get(PORT_PUMAS_MODEL_REF), diagnostics)
}

fn parse_model_ref(
    node_id: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<PumasModelRef> {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            node_id,
            Some(PORT_PUMAS_MODEL_REF),
            InferenceInterfaceGraphResolutionDiagnosticCode::MissingPumasModelRef,
            "inference interface resolution requires a canonical pumas_model_ref input",
        ));
        return None;
    };

    match serde_json::from_value::<PumasModelRef>(value.clone()) {
        Ok(model_ref) => match model_ref.validate() {
            Ok(()) => Some(model_ref),
            Err(error) => {
                diagnostics.push(diagnostic(
                    node_id,
                    Some(PORT_PUMAS_MODEL_REF),
                    InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRef,
                    format!("pumas_model_ref is invalid: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRef,
                format!("pumas_model_ref must match the canonical contract: {error}"),
            ));
            None
        }
    }
}

fn optional_task_kind(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<InferenceTaskKind>> {
    let Some(value) = optional_string(data, PORT_TASK_KIND) else {
        return Some(None);
    };
    match InferenceTaskKind::parse(value) {
        Ok(task_kind) => Some(Some(task_kind)),
        Err(error) => {
            push_contract_diagnostic(
                node_id,
                PORT_TASK_KIND,
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidTaskKind,
                error,
                diagnostics,
            );
            None
        }
    }
}

fn optional_runtime_constraint(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<RuntimeIntentId>> {
    let Some(value) = optional_string(data, PORT_RUNTIME) else {
        return Some(None);
    };
    match RuntimeIntentId::parse(value) {
        Ok(runtime_id) => Some(Some(runtime_id)),
        Err(error) => {
            push_contract_diagnostic(
                node_id,
                PORT_RUNTIME,
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeConstraint,
                error,
                diagnostics,
            );
            None
        }
    }
}

fn optional_device_constraint(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<DeviceIntentId>> {
    let Some(value) = optional_string(data, PORT_DEVICE) else {
        return Some(None);
    };
    match DeviceIntentId::parse(value) {
        Ok(device_id) => Some(Some(device_id)),
        Err(error) => {
            push_contract_diagnostic(
                node_id,
                PORT_DEVICE,
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidDeviceConstraint,
                error,
                diagnostics,
            );
            None
        }
    }
}

fn optional_string<'a>(data: &'a Value, field: &str) -> Option<&'a str> {
    match data.get(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn push_contract_diagnostic(
    node_id: &str,
    port_id: &str,
    code: InferenceInterfaceGraphResolutionDiagnosticCode,
    error: impl std::fmt::Display,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) {
    diagnostics.push(diagnostic(
        node_id,
        Some(port_id),
        code,
        format!("{port_id} is invalid: {error}"),
    ));
}

fn diagnostic(
    node_id: &str,
    port_id: Option<&str>,
    code: InferenceInterfaceGraphResolutionDiagnosticCode,
    message: impl Into<String>,
) -> InferenceInterfaceGraphResolutionDiagnostic {
    InferenceInterfaceGraphResolutionDiagnostic {
        node_id: node_id.to_string(),
        port_id: port_id.map(str::to_string),
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::graph::{GraphEdge, GraphNode, Position};

    #[test]
    fn graph_resolution_inputs_use_connected_pumas_model_ref() {
        let result =
            inference_interface_resolution_inputs_from_graph(&graph_with_connected_model());

        assert!(result.diagnostics.is_empty());
        assert_eq!(result.requests.len(), 1);
        let request = &result.requests[0];
        assert_eq!(request.node_id, "infer");
        assert_eq!(request.request.model_ref.model_id, "image/example/tiny");
        assert_eq!(
            request.request.task_kind.as_ref().map(|task| task.as_str()),
            Some("image_generation")
        );
        assert_eq!(
            request
                .request
                .runtime_constraint
                .as_ref()
                .map(|runtime| runtime.as_str()),
            Some("pytorch")
        );
        assert_eq!(
            request
                .request
                .device_constraint
                .as_ref()
                .map(|device| device.as_str()),
            Some("cuda.0")
        );
    }

    #[test]
    fn graph_resolution_inputs_reject_missing_canonical_model_ref() {
        let mut graph = graph_with_connected_model();
        graph.nodes[0].data = json!({
            "model_path": "/tmp/retired-model"
        });

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == InferenceInterfaceGraphResolutionDiagnosticCode::MissingPumasModelRef
                && diagnostic.port_id.as_deref() == Some(PORT_PUMAS_MODEL_REF)
        }));
        let encoded = serde_json::to_string(&result).expect("result json");
        assert!(!encoded.contains("model_path"));
        assert!(!encoded.contains("/tmp/retired-model"));
    }

    #[test]
    fn graph_resolution_inputs_reject_invalid_runtime_constraint() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data["runtime"] = json!("not valid");

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeConstraint
                && diagnostic.port_id.as_deref() == Some(PORT_RUNTIME)
        }));
    }

    fn graph_with_connected_model() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "model".to_string(),
                    node_type: "puma-lib".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: json!({
                        "pumas_model_ref": {
                            "model_id": "image/example/tiny",
                            "revision": "main",
                            "selected_artifact_id": "diffusers"
                        }
                    }),
                },
                GraphNode {
                    id: "infer".to_string(),
                    node_type: NODE_TYPE_GENERIC_INFERENCE.to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: json!({
                        "task_kind": "image_generation",
                        "runtime": "pytorch",
                        "device": "cuda.0"
                    }),
                },
            ],
            edges: vec![GraphEdge {
                id: "model-to-infer".to_string(),
                source: "model".to_string(),
                source_handle: PORT_PUMAS_MODEL_REF.to_string(),
                target: "infer".to_string(),
                target_handle: PORT_PUMAS_MODEL_REF.to_string(),
            }],
            derived_graph: None,
        }
    }
}

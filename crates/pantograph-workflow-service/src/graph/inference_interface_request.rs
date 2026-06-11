use std::collections::BTreeMap;

use pantograph_dependency_planning::{DeviceIntentId, PumasModelRef, RuntimeIntentId};
use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, InferenceTaskKind, ResolveInferenceInterfaceRequest,
    INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{GraphNode, WorkflowGraph};

const NODE_TYPE_GENERIC_INFERENCE: &str = "llm-inference";
const PORT_PUMAS_MODEL_REF: &str = "pumas_model_ref";
const PORT_TASK_KIND: &str = "task_kind";
const PORT_RUNTIME: &str = "runtime";
const PORT_DEVICE: &str = "device";
const PORT_RUNTIME_SOURCE_CONTEXT: &str = "runtime_source_context";
const NODE_TYPE_PUMA_LIB: &str = "puma-lib";
const INFERENCE_INTERFACE_SNAPSHOT_FIELD: &str = "inference_interface_snapshot";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceGraphResolutionInput {
    pub node_id: String,
    pub request: ResolveInferenceInterfaceRequest,
    pub runtime_source_context: WorkflowRuntimeSourceContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_snapshot: Option<AuthoredInferenceInterfaceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowRuntimeSourceContext {
    pub operation_type: String,
    pub context_shape_key: String,
    pub cancellation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    DuplicatePumasModelRefBinding,
    InvalidPumasModelRefBindingSource,
    PumasModelRefBindingDisagreement,
    InvalidTaskKind,
    InvalidRuntimeConstraint,
    InvalidDeviceConstraint,
    InvalidAuthoredSnapshot,
    MissingRuntimeSourceContext,
    InvalidRuntimeSourceContext,
}

pub fn inference_interface_resolution_inputs_from_graph(
    graph: &WorkflowGraph,
) -> InferenceInterfaceGraphResolutionInputs {
    let nodes_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let incoming_model_refs = incoming_model_ref_bindings(graph);
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
        let Some(authored_snapshot) =
            optional_authored_snapshot(&node.id, &node.data, &mut diagnostics)
        else {
            continue;
        };
        let Some(runtime_source_context) =
            runtime_source_context(&node.id, &node.data, &mut diagnostics)
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
            runtime_source_context,
            authored_snapshot,
        });
    }

    InferenceInterfaceGraphResolutionInputs {
        requests,
        diagnostics,
    }
}

fn incoming_model_ref_bindings(
    graph: &WorkflowGraph,
) -> BTreeMap<&str, Vec<ModelRefBindingSource<'_>>> {
    let mut incoming = BTreeMap::<&str, Vec<ModelRefBindingSource<'_>>>::new();
    for edge in graph
        .edges
        .iter()
        .filter(|edge| edge.target_handle == PORT_PUMAS_MODEL_REF)
    {
        incoming
            .entry(edge.target.as_str())
            .or_default()
            .push(ModelRefBindingSource {
                source_node_id: edge.source.as_str(),
                source_handle: edge.source_handle.as_str(),
            });
    }
    incoming
}

#[derive(Debug, Clone, Copy)]
struct ModelRefBindingSource<'a> {
    source_node_id: &'a str,
    source_handle: &'a str,
}

fn model_ref_for_inference_node(
    node_id: &str,
    node_data: &Value,
    incoming_model_refs: &BTreeMap<&str, Vec<ModelRefBindingSource<'_>>>,
    nodes_by_id: &BTreeMap<&str, &GraphNode>,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<PumasModelRef> {
    let inline_model_ref = if node_data.get(PORT_PUMAS_MODEL_REF).is_some() {
        parse_model_ref(node_id, node_data.get(PORT_PUMAS_MODEL_REF), diagnostics)
    } else {
        None
    };

    if let Some(sources) = incoming_model_refs.get(node_id) {
        if sources.len() > 1 {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::DuplicatePumasModelRefBinding,
                "inference nodes accept exactly one incoming pumas_model_ref binding",
            ));
            return None;
        }
        let source = sources[0];
        if source.source_handle != PORT_PUMAS_MODEL_REF {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRefBindingSource,
                "incoming pumas_model_ref binding must originate from a pumas_model_ref output handle",
            ));
            return None;
        }
        let Some(source_node) = nodes_by_id.get(source.source_node_id) else {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRefBindingSource,
                "connected pumas_model_ref source node is missing from the graph",
            ));
            return None;
        };
        if source_node.node_type != NODE_TYPE_PUMA_LIB {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRefBindingSource,
                "incoming pumas_model_ref binding must originate from a puma-lib node",
            ));
            return None;
        }
        if source_node.data.get(PORT_PUMAS_MODEL_REF).is_none() {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_PUMAS_MODEL_REF),
                InferenceInterfaceGraphResolutionDiagnosticCode::MissingPumasModelRef,
                "connected puma-lib node does not provide a canonical pumas_model_ref output",
            ));
            return None;
        }
        let connected_model_ref = parse_model_ref(
            node_id,
            source_node.data.get(PORT_PUMAS_MODEL_REF),
            diagnostics,
        )?;
        if let Some(inline_model_ref) = inline_model_ref {
            if inline_model_ref != connected_model_ref {
                diagnostics.push(diagnostic(
                    node_id,
                    Some(PORT_PUMAS_MODEL_REF),
                    InferenceInterfaceGraphResolutionDiagnosticCode::PumasModelRefBindingDisagreement,
                    "inline pumas_model_ref disagrees with the connected pumas_model_ref binding",
                ));
                return None;
            }
        }
        return Some(connected_model_ref);
    }

    match inline_model_ref {
        Some(model_ref) => Some(model_ref),
        None => {
            if node_data.get(PORT_PUMAS_MODEL_REF).is_none() {
                diagnostics.push(diagnostic(
                    node_id,
                    Some(PORT_PUMAS_MODEL_REF),
                    InferenceInterfaceGraphResolutionDiagnosticCode::MissingPumasModelRef,
                    "inference interface resolution requires a canonical pumas_model_ref input",
                ));
            }
            None
        }
    }
}

fn parse_model_ref(
    node_id: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<PumasModelRef> {
    let Some(value) = value else {
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

fn runtime_source_context(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<WorkflowRuntimeSourceContext> {
    let Some(value) = data.get(PORT_RUNTIME_SOURCE_CONTEXT) else {
        diagnostics.push(diagnostic(
            node_id,
            Some(PORT_RUNTIME_SOURCE_CONTEXT),
            InferenceInterfaceGraphResolutionDiagnosticCode::MissingRuntimeSourceContext,
            "inference nodes require explicit runtime_source_context",
        ));
        return None;
    };
    if value.is_null() {
        diagnostics.push(diagnostic(
            node_id,
            Some(PORT_RUNTIME_SOURCE_CONTEXT),
            InferenceInterfaceGraphResolutionDiagnosticCode::MissingRuntimeSourceContext,
            "inference nodes require explicit runtime_source_context",
        ));
        return None;
    }
    let Ok(context) = serde_json::from_value::<WorkflowRuntimeSourceContext>(value.clone()) else {
        diagnostics.push(diagnostic(
            node_id,
            Some(PORT_RUNTIME_SOURCE_CONTEXT),
            InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeSourceContext,
            "runtime_source_context must match the canonical workflow-service contract",
        ));
        return None;
    };
    validate_runtime_source_context(node_id, &context, diagnostics).then_some(context)
}

fn validate_runtime_source_context(
    node_id: &str,
    context: &WorkflowRuntimeSourceContext,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> bool {
    let mut valid = true;
    for (field, value) in [
        ("operation_type", context.operation_type.as_str()),
        ("context_shape_key", context.context_shape_key.as_str()),
        ("cancellation_mode", context.cancellation_mode.as_str()),
    ] {
        if value.trim().is_empty() {
            diagnostics.push(diagnostic(
                node_id,
                Some(PORT_RUNTIME_SOURCE_CONTEXT),
                InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeSourceContext,
                format!("runtime_source_context.{field} must be non-empty"),
            ));
            valid = false;
        }
    }
    valid
}

fn optional_authored_snapshot(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<AuthoredInferenceInterfaceSnapshot>> {
    match data.get(INFERENCE_INTERFACE_SNAPSHOT_FIELD) {
        None | Some(Value::Null) => Some(None),
        Some(value) => {
            match serde_json::from_value::<AuthoredInferenceInterfaceSnapshot>(value.clone()) {
                Ok(snapshot) => match snapshot.validate() {
                    Ok(()) => Some(Some(snapshot)),
                    Err(error) => {
                        diagnostics.push(diagnostic(
                        node_id,
                        Some(INFERENCE_INTERFACE_SNAPSHOT_FIELD),
                        InferenceInterfaceGraphResolutionDiagnosticCode::InvalidAuthoredSnapshot,
                        format!("authored inference interface snapshot is invalid: {error}"),
                    ));
                        None
                    }
                },
                Err(error) => {
                    diagnostics.push(diagnostic(
                    node_id,
                    Some(INFERENCE_INTERFACE_SNAPSHOT_FIELD),
                    InferenceInterfaceGraphResolutionDiagnosticCode::InvalidAuthoredSnapshot,
                    format!(
                        "authored inference interface snapshot must match the canonical contract: {error}",
                    ),
                ));
                    None
                }
            }
        }
    }
}

fn optional_task_kind(
    node_id: &str,
    data: &Value,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<InferenceTaskKind>> {
    let value = optional_string(
        node_id,
        data,
        PORT_TASK_KIND,
        InferenceInterfaceGraphResolutionDiagnosticCode::InvalidTaskKind,
        diagnostics,
    )?;
    let Some(value) = value else {
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
    let value = optional_string(
        node_id,
        data,
        PORT_RUNTIME,
        InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeConstraint,
        diagnostics,
    )?;
    let Some(value) = value else {
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
    let value = optional_string(
        node_id,
        data,
        PORT_DEVICE,
        InferenceInterfaceGraphResolutionDiagnosticCode::InvalidDeviceConstraint,
        diagnostics,
    )?;
    let Some(value) = value else {
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

fn optional_string<'a>(
    node_id: &str,
    data: &'a Value,
    field: &str,
    code: InferenceInterfaceGraphResolutionDiagnosticCode,
    diagnostics: &mut Vec<InferenceInterfaceGraphResolutionDiagnostic>,
) -> Option<Option<&'a str>> {
    match data.get(field) {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(value)) if value.trim().is_empty() => Some(None),
        Some(Value::String(value)) => Some(Some(value)),
        Some(_) => {
            diagnostics.push(diagnostic(
                node_id,
                Some(field),
                code,
                format!("{field} must be a string when provided"),
            ));
            None
        }
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
    use serde_json::{json, Value};

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
        assert!(request.authored_snapshot.is_none());
    }

    #[test]
    fn graph_resolution_inputs_preserve_authored_inference_snapshot() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data[INFERENCE_INTERFACE_SNAPSHOT_FIELD] = authored_snapshot_json();

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.diagnostics.is_empty());
        assert_eq!(result.requests.len(), 1);
        let snapshot = result.requests[0]
            .authored_snapshot
            .as_ref()
            .expect("authored snapshot");
        assert_eq!(
            snapshot.descriptor_fingerprint.as_str(),
            "descriptor.previous"
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_eq!(snapshot.inputs[0].port_id.as_str(), "prompt");
    }

    #[test]
    fn graph_resolution_inputs_reject_invalid_authored_inference_snapshot() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data[INFERENCE_INTERFACE_SNAPSHOT_FIELD] = json!({
            "descriptor_fingerprint": "",
            "task_kind": "image_generation",
            "inputs": [],
            "outputs": []
        });

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidAuthoredSnapshot
                && diagnostic.port_id.as_deref() == Some(INFERENCE_INTERFACE_SNAPSHOT_FIELD)
        }));
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

    #[test]
    fn graph_resolution_inputs_reject_duplicate_model_ref_bindings() {
        let mut graph = graph_with_connected_model();
        graph.nodes.push(GraphNode {
            id: "model-2".to_string(),
            node_type: NODE_TYPE_PUMA_LIB.to_string(),
            position: Position { x: 0.0, y: 100.0 },
            data: json!({
                "pumas_model_ref": {
                    "model_id": "image/example/other",
                    "selected_artifact_id": "diffusers"
                }
            }),
        });
        graph.edges.push(GraphEdge {
            id: "model-2-to-infer".to_string(),
            source: "model-2".to_string(),
            source_handle: PORT_PUMAS_MODEL_REF.to_string(),
            target: "infer".to_string(),
            target_handle: PORT_PUMAS_MODEL_REF.to_string(),
        });

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::DuplicatePumasModelRefBinding
                && diagnostic.port_id.as_deref() == Some(PORT_PUMAS_MODEL_REF)
        }));
    }

    #[test]
    fn graph_resolution_inputs_reject_wrong_model_ref_source_handle() {
        let mut graph = graph_with_connected_model();
        graph.edges[0].source_handle = "model_id".to_string();

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRefBindingSource
                && diagnostic.port_id.as_deref() == Some(PORT_PUMAS_MODEL_REF)
        }));
    }

    #[test]
    fn graph_resolution_inputs_reject_wrong_model_ref_source_node_type() {
        let mut graph = graph_with_connected_model();
        graph.nodes[0].node_type = "text-input".to_string();

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidPumasModelRefBindingSource
                && diagnostic.port_id.as_deref() == Some(PORT_PUMAS_MODEL_REF)
        }));
    }

    #[test]
    fn graph_resolution_inputs_reject_inline_connected_model_ref_disagreement() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data["pumas_model_ref"] = json!({
            "model_id": "image/example/different",
            "selected_artifact_id": "diffusers"
        });

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::PumasModelRefBindingDisagreement
                && diagnostic.port_id.as_deref() == Some(PORT_PUMAS_MODEL_REF)
        }));
    }

    #[test]
    fn graph_resolution_inputs_reject_wrong_type_optional_constraints() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data["runtime"] = json!({"id": "pytorch"});

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeConstraint
                && diagnostic.port_id.as_deref() == Some(PORT_RUNTIME)
        }));
    }

    #[test]
    fn graph_resolution_inputs_treat_null_and_blank_optional_constraints_as_absent() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data["task_kind"] = Value::Null;
        graph.nodes[1].data["runtime"] = json!("   ");
        graph.nodes[1].data["device"] = Value::Null;

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.diagnostics.is_empty());
        assert_eq!(result.requests.len(), 1);
        let request = &result.requests[0].request;
        assert!(request.task_kind.is_none());
        assert!(request.runtime_constraint.is_none());
        assert!(request.device_constraint.is_none());
    }

    #[test]
    fn graph_resolution_inputs_preserve_runtime_source_context() {
        let result =
            inference_interface_resolution_inputs_from_graph(&graph_with_connected_model());

        assert!(result.diagnostics.is_empty());
        let source_context = &result.requests[0].runtime_source_context;
        assert_eq!(source_context.operation_type.as_str(), "image_generation");
        assert_eq!(
            source_context.context_shape_key.as_str(),
            "image.1024.square"
        );
        assert_eq!(source_context.cancellation_mode.as_str(), "run_scoped");
    }

    #[test]
    fn graph_resolution_inputs_reject_missing_runtime_source_context() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1]
            .data
            .as_object_mut()
            .expect("node data object")
            .remove(PORT_RUNTIME_SOURCE_CONTEXT);

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::MissingRuntimeSourceContext
                && diagnostic.port_id.as_deref() == Some(PORT_RUNTIME_SOURCE_CONTEXT)
        }));
    }

    #[test]
    fn graph_resolution_inputs_reject_invalid_runtime_source_context() {
        let mut graph = graph_with_connected_model();
        graph.nodes[1].data[PORT_RUNTIME_SOURCE_CONTEXT] = json!({
            "operation_type": "image_generation",
            "context_shape_key": "",
            "cancellation_mode": "run_scoped"
        });

        let result = inference_interface_resolution_inputs_from_graph(&graph);

        assert!(result.requests.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == InferenceInterfaceGraphResolutionDiagnosticCode::InvalidRuntimeSourceContext
                && diagnostic.port_id.as_deref() == Some(PORT_RUNTIME_SOURCE_CONTEXT)
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
                        "device": "cuda.0",
                        "runtime_source_context": {
                            "operation_type": "image_generation",
                            "context_shape_key": "image.1024.square",
                            "cancellation_mode": "run_scoped"
                        }
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

    fn authored_snapshot_json() -> Value {
        json!({
            "contract_version": INFERENCE_INTERFACE_CONTRACT_VERSION,
            "descriptor_fingerprint": "descriptor.previous",
            "task_kind": "image_generation",
            "inputs": [
                {
                    "port_id": "prompt",
                    "label": "Prompt",
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
            "outputs": []
        })
    }
}

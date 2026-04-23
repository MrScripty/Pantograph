use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::registry::NodeRegistry;
use super::types::{
    GraphEdge, GraphNode, NodeDefinition, PortDataType, PortDefinition, WorkflowGraph,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct InferenceParamConstraints {
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    allowed_values: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct InferenceParamSchema {
    key: String,
    label: String,
    param_type: String,
    #[serde(default)]
    default: Value,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    constraints: Option<InferenceParamConstraints>,
    #[serde(default)]
    pantograph_origin: Option<String>,
    #[serde(default)]
    pantograph_owner_node_type: Option<String>,
}

pub fn canonicalize_workflow_graph(graph: WorkflowGraph, registry: &NodeRegistry) -> WorkflowGraph {
    let (graph, _) = canonicalize_legacy_node_types(graph);
    let mut nodes = graph.nodes;
    let mut edges = graph.edges;
    let node_indices = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect::<HashMap<_, _>>();
    let source_node_ids = nodes
        .iter()
        .filter(|node| node.node_type != "expand-settings")
        .filter_map(|node| {
            parse_inference_settings(node.data.get("inference_settings")).map(|_| node.id.clone())
        })
        .collect::<Vec<_>>();

    for source_node_id in source_node_ids {
        let Some(source_index) = node_indices.get(&source_node_id).copied() else {
            continue;
        };
        let Some(inference_settings) =
            parse_inference_settings(nodes[source_index].data.get("inference_settings"))
        else {
            continue;
        };

        for target_node_id in find_connected_targets(&edges, &source_node_id, "inference_settings")
        {
            let Some(target_index) = node_indices.get(&target_node_id).copied() else {
                continue;
            };
            if nodes[target_index].node_type != "expand-settings" {
                reconcile_inference_node(&mut nodes[target_index], registry, &inference_settings);
                continue;
            }

            let Some(base_expand_definition) = registry.get_definition("expand-settings") else {
                continue;
            };
            let current_expand_definition =
                resolved_definition_json(&nodes[target_index], base_expand_definition);
            let downstream_node_ids =
                find_connected_targets(&edges, &target_node_id, "inference_settings");
            let downstream_base_definitions = downstream_node_ids
                .iter()
                .filter_map(|node_id| {
                    let node_index = node_indices.get(node_id).copied()?;
                    registry
                        .get_definition(nodes[node_index].node_type.as_str())
                        .cloned()
                })
                .collect::<Vec<_>>();
            let merged_expand_settings =
                build_expand_settings_schema(&downstream_base_definitions, &inference_settings);
            let expand_definition = build_dynamic_expand_definition_json(
                current_expand_definition,
                base_expand_definition,
                &merged_expand_settings,
            );
            set_node_definition(&mut nodes[target_index], expand_definition);
            set_node_inference_settings(&mut nodes[target_index], &merged_expand_settings);

            for downstream_node_id in downstream_node_ids {
                let Some(downstream_index) = node_indices.get(&downstream_node_id).copied() else {
                    continue;
                };
                let target_settings = reconcile_inference_node(
                    &mut nodes[downstream_index],
                    registry,
                    &inference_settings,
                );

                for param in target_settings {
                    if has_edge(
                        &edges,
                        &target_node_id,
                        &param.key,
                        &downstream_node_id,
                        &param.key,
                    ) {
                        continue;
                    }
                    edges.push(GraphEdge {
                        id: format!(
                            "{}-{}-{}-{}",
                            target_node_id, param.key, downstream_node_id, param.key
                        ),
                        source: target_node_id.clone(),
                        source_handle: param.key.clone(),
                        target: downstream_node_id.clone(),
                        target_handle: param.key,
                    });
                }
            }
        }
    }

    WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    }
}

fn canonicalize_legacy_node_types(graph: WorkflowGraph) -> (WorkflowGraph, HashSet<String>) {
    let mut migrated_node_ids = HashSet::new();
    let nodes = graph
        .nodes
        .into_iter()
        .map(|mut node| {
            if node.node_type != "system-prompt" {
                return node;
            }
            migrated_node_ids.insert(node.id.clone());
            node.node_type = "text-input".to_string();
            if let Some(data) = node.data.as_object_mut() {
                if let Some(prompt) = data.remove("prompt") {
                    data.entry("text".to_string()).or_insert(prompt);
                }
            }
            node
        })
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .into_iter()
        .map(|mut edge| {
            if migrated_node_ids.contains(&edge.source) && edge.source_handle == "prompt" {
                edge.source_handle = "text".to_string();
            }
            if migrated_node_ids.contains(&edge.target) && edge.target_handle == "prompt" {
                edge.target_handle = "text".to_string();
            }
            edge
        })
        .collect::<Vec<_>>();
    (
        WorkflowGraph {
            nodes,
            edges,
            derived_graph: None,
        },
        migrated_node_ids,
    )
}

fn parse_inference_settings(value: Option<&Value>) -> Option<Vec<InferenceParamSchema>> {
    let array = value?.as_array()?;
    let parsed = array
        .iter()
        .map(|entry| serde_json::from_value::<InferenceParamSchema>(entry.clone()).ok())
        .collect::<Option<Vec<_>>>()?;
    if parsed.iter().all(|entry| !entry.key.trim().is_empty()) {
        Some(parsed)
    } else {
        None
    }
}

fn find_connected_targets(
    edges: &[GraphEdge],
    source_id: &str,
    source_handle: &str,
) -> Vec<String> {
    edges
        .iter()
        .filter(|edge| edge.source == source_id && edge.source_handle == source_handle)
        .map(|edge| edge.target.clone())
        .collect()
}

fn has_edge(
    edges: &[GraphEdge],
    source: &str,
    source_handle: &str,
    target: &str,
    target_handle: &str,
) -> bool {
    edges.iter().any(|edge| {
        edge.source == source
            && edge.source_handle == source_handle
            && edge.target == target
            && edge.target_handle == target_handle
    })
}

fn reconcile_inference_node(
    node: &mut GraphNode,
    registry: &NodeRegistry,
    inference_settings: &[InferenceParamSchema],
) -> Vec<InferenceParamSchema> {
    let Some(base_definition) = registry.get_definition(node.node_type.as_str()) else {
        return Vec::new();
    };
    let current_definition = resolved_definition_json(node, base_definition);
    let merged_settings = build_merged_inference_settings(base_definition, inference_settings);
    let definition = build_dynamic_inference_definition_json(
        current_definition,
        base_definition,
        &merged_settings,
    );
    set_node_definition(node, definition);
    merged_settings
}

fn build_merged_inference_settings(
    base_definition: &NodeDefinition,
    inference_settings: &[InferenceParamSchema],
) -> Vec<InferenceParamSchema> {
    let promoted_port_ids = promoted_inference_setting_port_ids(&base_definition.node_type);
    let upstream_settings =
        strip_foreign_inference_defaults(&base_definition.node_type, inference_settings);
    if promoted_port_ids.is_empty() {
        return upstream_settings;
    }

    let appended_settings = base_definition
        .inputs
        .iter()
        .filter(|port| promoted_port_ids.contains(port.id.as_str()))
        .map(|port| inference_default_port_to_schema(&base_definition.node_type, port))
        .collect::<Vec<_>>();
    merge_inference_settings(upstream_settings, appended_settings)
}

fn build_expand_settings_schema(
    base_definitions: &[NodeDefinition],
    inference_settings: &[InferenceParamSchema],
) -> Vec<InferenceParamSchema> {
    base_definitions.iter().fold(
        inference_settings.to_vec(),
        |current_settings, base_definition| {
            let appended_settings = base_definition
                .inputs
                .iter()
                .filter(|port| {
                    promoted_inference_setting_port_ids(&base_definition.node_type)
                        .contains(port.id.as_str())
                })
                .map(|port| inference_default_port_to_schema(&base_definition.node_type, port))
                .collect::<Vec<_>>();
            merge_inference_settings(current_settings, appended_settings)
        },
    )
}

fn merge_inference_settings(
    upstream_settings: Vec<InferenceParamSchema>,
    appended_settings: Vec<InferenceParamSchema>,
) -> Vec<InferenceParamSchema> {
    let mut merged = upstream_settings;
    let mut seen_keys = merged
        .iter()
        .map(|param| param.key.clone())
        .collect::<HashSet<_>>();
    for param in appended_settings {
        if seen_keys.insert(param.key.clone()) {
            merged.push(param);
        }
    }
    merged
}

fn strip_foreign_inference_defaults(
    node_type: &str,
    inference_settings: &[InferenceParamSchema],
) -> Vec<InferenceParamSchema> {
    inference_settings
        .iter()
        .filter(|param| match param.pantograph_origin.as_deref() {
            Some("inference-default") => {
                param.pantograph_owner_node_type.as_deref() == Some(node_type)
            }
            _ => true,
        })
        .cloned()
        .collect()
}

fn inference_default_port_to_schema(
    node_type: &str,
    port: &PortDefinition,
) -> InferenceParamSchema {
    InferenceParamSchema {
        key: port.id.clone(),
        label: port.label.clone(),
        param_type: port_data_type_to_param_type(&port.data_type).to_string(),
        default: Value::Null,
        description: None,
        constraints: None,
        pantograph_origin: Some("inference-default".to_string()),
        pantograph_owner_node_type: Some(node_type.to_string()),
    }
}

fn build_dynamic_inference_definition_json(
    current_definition: Value,
    base_definition: &NodeDefinition,
    inference_settings: &[InferenceParamSchema],
) -> Value {
    let current_inputs = definition_ports(&current_definition, "inputs");
    let promoted_port_ids = promoted_inference_setting_port_ids(&base_definition.node_type)
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();
    let static_inputs =
        select_static_ports(&current_inputs, &base_definition.inputs, &promoted_port_ids);
    let dynamic_ports = inference_settings
        .iter()
        .map(inference_param_to_port_json)
        .collect::<Vec<_>>();
    let merged_inputs = merge_dynamic_ports(static_inputs, dynamic_ports);
    rebuild_definition_json(current_definition, base_definition, merged_inputs, None)
}

fn build_dynamic_expand_definition_json(
    current_definition: Value,
    base_definition: &NodeDefinition,
    inference_settings: &[InferenceParamSchema],
) -> Value {
    let current_inputs = definition_ports(&current_definition, "inputs");
    let current_outputs = definition_ports(&current_definition, "outputs");
    let dynamic_ports = inference_settings
        .iter()
        .map(inference_param_to_port_json)
        .collect::<Vec<_>>();
    let merged_inputs = merge_dynamic_ports(
        select_static_ports(&current_inputs, &base_definition.inputs, &HashSet::new()),
        dynamic_ports.clone(),
    );
    let merged_outputs = merge_dynamic_ports(
        select_static_ports(&current_outputs, &base_definition.outputs, &HashSet::new()),
        dynamic_ports,
    );
    rebuild_definition_json(
        current_definition,
        base_definition,
        merged_inputs,
        Some(merged_outputs),
    )
}

fn rebuild_definition_json(
    current_definition: Value,
    base_definition: &NodeDefinition,
    inputs: Vec<Value>,
    outputs: Option<Vec<Value>>,
) -> Value {
    let mut object = match current_definition {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    object.insert(
        "node_type".to_string(),
        Value::String(base_definition.node_type.clone()),
    );
    object.insert(
        "label".to_string(),
        Value::String(base_definition.label.clone()),
    );
    object.insert(
        "description".to_string(),
        Value::String(base_definition.description.clone()),
    );
    object.insert(
        "category".to_string(),
        serde_json::to_value(&base_definition.category).unwrap_or(Value::Null),
    );
    object.insert(
        "io_binding_origin".to_string(),
        serde_json::to_value(&base_definition.io_binding_origin).unwrap_or(Value::Null),
    );
    object.insert(
        "execution_mode".to_string(),
        serde_json::to_value(&base_definition.execution_mode).unwrap_or(Value::Null),
    );
    object.insert("inputs".to_string(), Value::Array(inputs));
    if let Some(outputs) = outputs {
        object.insert("outputs".to_string(), Value::Array(outputs));
    } else if !object.contains_key("outputs") {
        object.insert(
            "outputs".to_string(),
            Value::Array(
                base_definition
                    .outputs
                    .iter()
                    .map(base_port_to_json)
                    .collect::<Vec<_>>(),
            ),
        );
    }
    Value::Object(object)
}

fn definition_ports(definition: &Value, field: &str) -> Vec<Value> {
    definition
        .get(field)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn select_static_ports(
    current_ports: &[Value],
    base_ports: &[PortDefinition],
    excluded_port_ids: &HashSet<String>,
) -> Vec<Value> {
    base_ports
        .iter()
        .filter(|port| !excluded_port_ids.contains(&port.id))
        .map(|base_port| {
            current_ports
                .iter()
                .find(|port| port_id(port) == Some(base_port.id.as_str()))
                .cloned()
                .unwrap_or_else(|| base_port_to_json(base_port))
        })
        .collect()
}

fn merge_dynamic_ports(base_ports: Vec<Value>, dynamic_ports: Vec<Value>) -> Vec<Value> {
    let dynamic_port_ids = dynamic_ports
        .iter()
        .filter_map(port_id)
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();
    let mut merged = base_ports
        .into_iter()
        .filter(|port| match port_id(port) {
            Some(id) => !dynamic_port_ids.contains(id),
            None => true,
        })
        .collect::<Vec<_>>();
    merged.extend(dynamic_ports);
    merged
}

fn resolved_definition_json(node: &GraphNode, base_definition: &NodeDefinition) -> Value {
    let base_json = serde_json::to_value(base_definition).unwrap_or(Value::Null);
    let Some(dynamic_definition) = node.data.get("definition") else {
        return base_json;
    };
    if dynamic_definition
        .get("node_type")
        .and_then(Value::as_str)
        .is_some_and(|dynamic_node_type| dynamic_node_type != node.node_type)
    {
        return base_json;
    }
    dynamic_definition.clone()
}

fn set_node_definition(node: &mut GraphNode, definition: Value) {
    let data = ensure_object(&mut node.data);
    data.insert("definition".to_string(), definition);
}

fn set_node_inference_settings(node: &mut GraphNode, inference_settings: &[InferenceParamSchema]) {
    let data = ensure_object(&mut node.data);
    data.insert(
        "inference_settings".to_string(),
        serde_json::to_value(inference_settings).unwrap_or(Value::Array(Vec::new())),
    );
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .expect("value should be an object after normalization")
}

fn inference_param_to_port_json(param: &InferenceParamSchema) -> Value {
    let mut object = Map::new();
    object.insert("id".to_string(), Value::String(param.key.clone()));
    object.insert("label".to_string(), Value::String(param.label.clone()));
    object.insert(
        "data_type".to_string(),
        Value::String(param_type_to_port_data_type(&param.param_type).to_string()),
    );
    object.insert("required".to_string(), Value::Bool(false));
    object.insert("multiple".to_string(), Value::Bool(false));
    if let Some(description) = &param.description {
        object.insert(
            "description".to_string(),
            Value::String(description.clone()),
        );
    }
    object.insert("default_value".to_string(), param.default.clone());
    if let Some(constraints) = &param.constraints {
        object.insert(
            "constraints".to_string(),
            serde_json::to_value(constraints).unwrap_or(Value::Null),
        );
    }
    Value::Object(object)
}

fn base_port_to_json(port: &PortDefinition) -> Value {
    json!({
        "id": port.id,
        "label": port.label,
        "data_type": port.data_type,
        "required": port.required,
        "multiple": port.multiple,
    })
}

fn port_id(port: &Value) -> Option<&str> {
    port.get("id").and_then(Value::as_str)
}

fn param_type_to_port_data_type(param_type: &str) -> &'static str {
    match param_type {
        "Number" | "Integer" => "number",
        "String" => "string",
        "Boolean" => "boolean",
        _ => "any",
    }
}

fn port_data_type_to_param_type(data_type: &PortDataType) -> &'static str {
    match data_type {
        PortDataType::Boolean => "Boolean",
        PortDataType::Number => "Number",
        PortDataType::String => "String",
        PortDataType::KvCache => "String",
        _ => "String",
    }
}

fn promoted_inference_setting_port_ids(node_type: &str) -> HashSet<&'static str> {
    match node_type {
        "audio-generation" => ["duration", "num_inference_steps", "guidance_scale", "seed"]
            .into_iter()
            .collect(),
        "diffusion-inference" => ["steps", "cfg_scale", "seed", "width", "height"]
            .into_iter()
            .collect(),
        "llamacpp-inference" | "ollama-inference" => {
            ["temperature", "max_tokens"].into_iter().collect()
        }
        "pytorch-inference" => ["temperature", "max_tokens", "device", "model_type"]
            .into_iter()
            .collect(),
        "reranker" => ["top_k", "return_documents"].into_iter().collect(),
        _ => HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonicalize_workflow_graph_migrates_legacy_system_prompt_nodes() {
        let registry = NodeRegistry::new();
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "prompt".to_string(),
                    node_type: "system-prompt".to_string(),
                    position: super::super::types::Position { x: 0.0, y: 0.0 },
                    data: json!({ "prompt": "hello" }),
                },
                GraphNode {
                    id: "target".to_string(),
                    node_type: "llm-inference".to_string(),
                    position: super::super::types::Position { x: 100.0, y: 0.0 },
                    data: json!({}),
                },
            ],
            edges: vec![GraphEdge {
                id: "prompt-prompt-target-prompt".to_string(),
                source: "prompt".to_string(),
                source_handle: "prompt".to_string(),
                target: "target".to_string(),
                target_handle: "prompt".to_string(),
            }],
            derived_graph: None,
        };

        let canonical = canonicalize_workflow_graph(graph, &registry);
        let prompt_node = canonical
            .nodes
            .iter()
            .find(|node| node.id == "prompt")
            .expect("prompt node");
        assert_eq!(prompt_node.node_type, "text-input");
        assert_eq!(prompt_node.data["text"], json!("hello"));
        assert_eq!(canonical.edges[0].source_handle, "text");
    }

    #[test]
    fn canonicalize_workflow_graph_hydrates_expand_settings_and_passthrough_edges() {
        let registry = NodeRegistry::new();
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "source".to_string(),
                    node_type: "model-provider".to_string(),
                    position: super::super::types::Position { x: 0.0, y: 0.0 },
                    data: json!({
                        "inference_settings": [
                            {
                                "key": "steps",
                                "label": "Steps",
                                "param_type": "Number",
                                "default": 30,
                            }
                        ]
                    }),
                },
                GraphNode {
                    id: "expand".to_string(),
                    node_type: "expand-settings".to_string(),
                    position: super::super::types::Position { x: 100.0, y: 0.0 },
                    data: json!({}),
                },
                GraphNode {
                    id: "diffusion".to_string(),
                    node_type: "diffusion-inference".to_string(),
                    position: super::super::types::Position { x: 200.0, y: 0.0 },
                    data: json!({}),
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "source-settings-expand-settings".to_string(),
                    source: "source".to_string(),
                    source_handle: "inference_settings".to_string(),
                    target: "expand".to_string(),
                    target_handle: "inference_settings".to_string(),
                },
                GraphEdge {
                    id: "expand-settings-diffusion-settings".to_string(),
                    source: "expand".to_string(),
                    source_handle: "inference_settings".to_string(),
                    target: "diffusion".to_string(),
                    target_handle: "inference_settings".to_string(),
                },
            ],
            derived_graph: None,
        };

        let canonical = canonicalize_workflow_graph(graph, &registry);
        let expand_node = canonical
            .nodes
            .iter()
            .find(|node| node.id == "expand")
            .expect("expand node");
        let diffusion_node = canonical
            .nodes
            .iter()
            .find(|node| node.id == "diffusion")
            .expect("diffusion node");
        let expand_outputs = expand_node.data["definition"]["outputs"]
            .as_array()
            .expect("expand outputs");
        let diffusion_inputs = diffusion_node.data["definition"]["inputs"]
            .as_array()
            .expect("diffusion inputs");

        assert!(expand_outputs
            .iter()
            .any(|port| port["id"] == json!("steps")));
        assert!(diffusion_inputs
            .iter()
            .any(|port| port["id"] == json!("steps")));
        assert!(canonical.edges.iter().any(|edge| {
            edge.source == "expand"
                && edge.source_handle == "steps"
                && edge.target == "diffusion"
                && edge.target_handle == "steps"
        }));
    }
}

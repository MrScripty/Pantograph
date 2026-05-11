//! Backend-owned helpers for embedding workflow runtime preparation.
//!
//! These helpers inspect workflow graphs to determine whether embedding runtime
//! preparation is required and which Puma-Lib model id must back an embedding
//! workflow execution.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use pantograph_runtime_identity::canonical_runtime_backend_key;

fn node_data_string(data: &serde_json::Value, keys: &[&str]) -> Option<String> {
    let obj = data.as_object()?;
    keys.iter().find_map(|key| {
        obj.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn node_pumas_model_ref_model_id(data: &serde_json::Value) -> Option<String> {
    data.get("pumas_model_ref")
        .and_then(|value| value.get("model_id"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn node_task_kind(data: &serde_json::Value) -> Option<String> {
    node_data_string(data, &["task_kind", "taskKind"])
}

fn node_backend_key(data: &serde_json::Value) -> Option<String> {
    node_data_string(
        data,
        &[
            "backend_key",
            "backendKey",
            "recommended_backend",
            "recommendedBackend",
        ],
    )
    .or_else(|| {
        data.get("pumas_model_ref").and_then(|model_ref| {
            node_data_string(
                model_ref,
                &[
                    "backend_key",
                    "backendKey",
                    "recommended_backend",
                    "recommendedBackend",
                ],
            )
        })
    })
}

fn is_canonical_embedding_inference_node(node: &pantograph_workflow_service::GraphNode) -> bool {
    node.node_type == "llm-inference"
        && node_task_kind(&node.data)
            .is_some_and(|task_kind| task_kind.eq_ignore_ascii_case("embedding"))
}

fn derive_models_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate
            .to_string_lossy()
            .ends_with("shared-resources/models")
        {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

fn find_gguf_files_in_dir(dir: &Path, limit: usize) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        format!(
            "Cannot read embedding model directory '{}': {}",
            dir.display(),
            e
        )
    })?;

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        {
            matches.push(path);
            if matches.len() >= limit {
                break;
            }
        }
    }

    Ok(matches)
}

fn find_model_files_by_name(
    models_root: &Path,
    file_name: &std::ffi::OsStr,
    limit: usize,
) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let mut stack = vec![models_root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.is_file() && path.file_name() == Some(file_name) {
                matches.push(path);
                if matches.len() >= limit {
                    return matches;
                }
            }
        }
    }

    matches
}

pub fn resolve_embedding_model_path(model_path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(model_path);
    if candidate.is_file() {
        return Ok(candidate);
    }
    if candidate.is_dir() {
        let matches = find_gguf_files_in_dir(&candidate, 8)?;
        return match matches.len() {
            0 => Err(format!(
                "Embedding model directory '{}' contains no .gguf files. Select a GGUF embedding model in Puma-Lib.",
                model_path
            )),
            1 => Ok(matches[0].clone()),
            _ => {
                let list = matches
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "Embedding model directory '{}' contains multiple .gguf files: {}. Select a single GGUF file path.",
                    model_path, list
                ))
            }
        };
    }

    let file_name = candidate
        .file_name()
        .ok_or_else(|| format!("Embedding model path is invalid: {}", model_path))?;
    let Some(models_root) = derive_models_root(&candidate) else {
        return Err(format!(
            "Embedding model file not found: {}. Update Model Configuration with a valid GGUF file path.",
            model_path
        ));
    };

    let matches = find_model_files_by_name(&models_root, file_name, 8);
    match matches.len() {
        0 => Err(format!(
            "Embedding model file not found: {}. Could not find '{}' under '{}'. Update Model Configuration.",
            model_path,
            file_name.to_string_lossy(),
            models_root.display()
        )),
        1 => {
            log::warn!(
                "Embedding model path '{}' was missing. Using discovered file '{}'",
                model_path,
                matches[0].display()
            );
            Ok(matches[0].clone())
        }
        _ => {
            let list = matches
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "Embedding model file not found at '{}', and multiple candidates matched '{}': {}. Update Model Configuration explicitly.",
                model_path,
                file_name.to_string_lossy(),
                list
            ))
        }
    }
}

pub fn workflow_graph_has_embedding_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph
        .nodes
        .iter()
        .any(is_canonical_embedding_inference_node)
}

pub fn workflow_graph_has_llamacpp_inference_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph.nodes.iter().any(|node| {
        node.node_type == "llm-inference"
            && !is_canonical_embedding_inference_node(node)
            && node_backend_key(&node.data).is_some_and(|backend_key| {
                canonical_runtime_backend_key(&backend_key) == "llama_cpp"
            })
    })
}

pub fn resolve_embedding_model_id_from_workflow_graph(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> Result<Option<String>, String> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();

    let embedding_nodes = graph
        .nodes
        .iter()
        .filter(|node| is_canonical_embedding_inference_node(node))
        .collect::<Vec<_>>();
    if embedding_nodes.is_empty() {
        return Ok(None);
    }

    let mut selected_model_ids = BTreeSet::new();
    for embedding_node in embedding_nodes {
        let mut model_ids_for_node = BTreeSet::new();
        if let Some(model_id) = node_pumas_model_ref_model_id(&embedding_node.data) {
            model_ids_for_node.insert(model_id);
        }
        for edge in graph.edges.iter().filter(|edge| {
            edge.target == embedding_node.id && edge.target_handle == "pumas_model_ref"
        }) {
            let source_node = node_by_id.get(edge.source.as_str()).ok_or_else(|| {
                format!(
                    "Embedding node '{}' references unknown source node '{}'",
                    embedding_node.id, edge.source
                )
            })?;
            if source_node.node_type != "puma-lib" {
                return Err(format!(
                    "Embedding inference node '{}' must receive `pumas_model_ref` from a Puma-Lib node",
                    embedding_node.id
                ));
            }
            let model_id = node_pumas_model_ref_model_id(&source_node.data)
                .or_else(|| node_data_string(&source_node.data, &["model_id", "modelId"]))
                .ok_or_else(|| {
                    format!(
                        "Puma-Lib node '{}' is missing `model_id`. Re-select a model in Puma-Lib.",
                        source_node.id
                    )
                })?;
            model_ids_for_node.insert(model_id);
        }

        if model_ids_for_node.is_empty() {
            return Err(format!(
                "Embedding inference node '{}' must connect Puma-Lib `pumas_model_ref` output to `pumas_model_ref` input or store a Pumas model ref with `model_id`",
                embedding_node.id
            ));
        }
        if model_ids_for_node.len() > 1 {
            return Err(format!(
                "Embedding inference node '{}' has multiple Puma-Lib model IDs; use exactly one",
                embedding_node.id
            ));
        }
        selected_model_ids.extend(model_ids_for_node);
    }

    if selected_model_ids.len() > 1 {
        return Err(
            "All embedding nodes in one workflow run must use the same Puma-Lib model".to_string(),
        );
    }

    Ok(selected_model_ids.into_iter().next())
}

pub async fn prepare_embedding_runtime_for_workflow(
    gateway: &inference::InferenceGateway,
    pumas_api: Option<&pumas_library::PumasApi>,
    mut request: inference::EmbeddingStartRequest,
    embedding_model_id_from_graph: Option<String>,
    needs_embedding_node: bool,
    needs_llamacpp_inference_node: bool,
) -> Result<Option<inference::BackendConfig>, String> {
    if !needs_embedding_node {
        return Ok(None);
    }

    if needs_llamacpp_inference_node {
        return Err(
            "Workflow includes canonical embedding inference and llama.cpp text-generation inference. They currently require different llama.cpp runtime modes; run them in separate workflow executions."
                .to_string(),
        );
    }

    let backend_name = gateway.current_backend_name().await;
    if canonical_runtime_backend_key(&backend_name) != "llama_cpp" {
        return Err(format!(
            "Embedding nodes currently require the `llama.cpp` backend, but active backend is '{}'",
            backend_name
        ));
    }

    let model_id = embedding_model_id_from_graph.ok_or_else(|| {
        "Embedding workflows must connect Puma-Lib `pumas_model_ref` to canonical embedding inference"
            .to_string()
    })?;
    let api = pumas_api.ok_or_else(|| {
        "Puma-Lib runtime is not initialized; cannot resolve model path from model_id".to_string()
    })?;

    let model = api
        .get_model(&model_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to resolve model '{}' from Puma-Lib: {}",
                model_id, e
            )
        })?
        .ok_or_else(|| {
            format!(
                "Puma-Lib model '{}' was not found. Re-select the model in Puma-Lib node.",
                model_id
            )
        })?;

    if !model.model_type.eq_ignore_ascii_case("embedding") {
        return Err(format!(
            "Puma-Lib model '{}' is type '{}' but embedding node requires an embedding model",
            model_id, model.model_type
        ));
    }

    request.gguf_model_path = Some(resolve_embedding_model_path(&model.path)?);
    let prepared = gateway
        .prepare_embedding_runtime(request)
        .await
        .map_err(|e| format!("Failed to start llama.cpp in embedding mode: {}", e))?;

    Ok(prepared.restore_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_workflow_service::{GraphEdge, GraphNode, Position, WorkflowGraph};

    fn graph(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> WorkflowGraph {
        WorkflowGraph {
            nodes,
            edges,
            ..WorkflowGraph::default()
        }
    }

    fn node(id: &str, node_type: &str, data: serde_json::Value) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            position: Position { x: 0.0, y: 0.0 },
            data,
        }
    }

    fn edge(
        id: &str,
        source: &str,
        source_handle: &str,
        target: &str,
        target_handle: &str,
    ) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: source_handle.to_string(),
            target_handle: target_handle.to_string(),
        }
    }

    #[test]
    fn workflow_graph_embedding_helpers_detect_embedding_and_llamacpp_nodes() {
        let graph = graph(
            vec![
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding", "backend_key": "llama_cpp"}),
                ),
                node(
                    "infer",
                    "llm-inference",
                    serde_json::json!({"task_kind": "text_generation", "backend_key": "llama_cpp"}),
                ),
            ],
            Vec::new(),
        );

        assert!(workflow_graph_has_embedding_node(&graph));
        assert!(workflow_graph_has_llamacpp_inference_node(&graph));
    }

    #[test]
    fn resolve_embedding_model_id_returns_connected_puma_lib_model() {
        let graph = graph(
            vec![
                node(
                    "puma",
                    "puma-lib",
                    serde_json::json!({
                        "pumas_model_ref": {
                            "model_id": "embed-model"
                        }
                    }),
                ),
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
            ],
            vec![edge(
                "edge-1",
                "puma",
                "pumas_model_ref",
                "embed",
                "pumas_model_ref",
            )],
        );

        assert_eq!(
            resolve_embedding_model_id_from_workflow_graph(&graph).expect("model id"),
            Some("embed-model".to_string())
        );
    }

    #[test]
    fn resolve_embedding_model_id_rejects_non_puma_lib_sources() {
        let graph = graph(
            vec![
                node(
                    "input",
                    "text-input",
                    serde_json::json!({ "value": "not a model" }),
                ),
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
            ],
            vec![edge("edge-1", "input", "text", "embed", "pumas_model_ref")],
        );

        let error =
            resolve_embedding_model_id_from_workflow_graph(&graph).expect_err("should reject");
        assert!(error.contains("must receive `pumas_model_ref` from a Puma-Lib node"));
    }

    #[test]
    fn resolve_embedding_model_path_returns_existing_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "pantograph-embedding-workflow-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let model_path = temp_dir.join("embed.gguf");
        std::fs::write(&model_path, b"gguf").expect("embedding model file should be written");

        let resolved = resolve_embedding_model_path(
            model_path
                .to_str()
                .expect("temporary embedding path should be utf-8"),
        )
        .expect("embedding model path should resolve");
        assert_eq!(resolved, model_path);

        std::fs::remove_file(&model_path)
            .expect("temporary embedding model file should be removed");
        std::fs::remove_dir(&temp_dir).expect("temporary test directory should be removed");
    }
}

use std::fs;
use std::path::Path;

use crate::workflow::WorkflowServiceError;
use crate::{WorkflowGraphSaveRequest, WorkflowService};

use super::persistence::{FileSystemWorkflowGraphStore, WorkflowGraphStore};
use super::types::{GraphEdge, GraphNode, Position, WorkflowFile, WorkflowGraph};

fn sample_puma_lib_data() -> serde_json::Value {
    serde_json::json!({
        "label": "Puma-Lib",
        "selectionMode": "library",
        "modelName": "tiny-sd-turbo",
        "model_id": "diffusion/cc-nms/tiny-sd-turbo",
        "selected_binding_ids": ["binding-a"],
        "modelPath": "/old/path/tiny-sd-turbo",
        "model_type": "diffusion",
        "task_type_primary": "text-to-image",
        "backend_key": "pytorch",
        "recommended_backend": "diffusers",
        "runtime_engine_hints": ["diffusers", "pytorch"],
        "platform_context": { "os": "linux", "arch": "x86_64" },
        "dependency_bindings": [{ "binding_id": "binding-a" }],
        "dependency_requirements": { "model_id": "diffusion/cc-nms/tiny-sd-turbo" },
        "dependency_requirements_id": "diffusion/cc-nms/tiny-sd-turbo",
        "inference_settings": [{ "key": "steps" }],
        "review_reasons": ["imported"]
    })
}

fn puma_lib_graph(data: serde_json::Value) -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![GraphNode {
            id: "puma-1".to_string(),
            node_type: "puma-lib".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            data,
        }],
        edges: Vec::new(),
        derived_graph: None,
    }
}

fn retired_direct_diffusion_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: -100.0, y: 5.0 },
                data: serde_json::json!({"text": "hello"}),
            },
            GraphNode {
                id: "diffusion".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: Position { x: 42.0, y: 24.0 },
                data: serde_json::json!({
                    "model_path": "/models/juggernaut",
                    "prompt": "hello",
                    "steps": 16
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "image-output".to_string(),
                position: Position { x: 200.0, y: 5.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "prompt-diffusion-prompt".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "diffusion-output-image".to_string(),
                source: "diffusion".to_string(),
                source_handle: "image".to_string(),
                target: "output".to_string(),
                target_handle: "image".to_string(),
            },
        ],
        derived_graph: None,
    }
}

fn retired_llamacpp_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({"text": "hello"}),
            },
            GraphNode {
                id: "llama".to_string(),
                node_type: "llamacpp-inference".to_string(),
                position: Position { x: 42.0, y: 24.0 },
                data: serde_json::json!({
                    "model_path": "/models/chat.gguf",
                    "prompt": "hello"
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 200.0, y: 5.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "prompt-llama-prompt".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "llama".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "llama-output-response".to_string(),
                source: "llama".to_string(),
                source_handle: "response".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    }
}

fn invalid_inference_draft_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: -120.0, y: 0.0 },
                data: serde_json::json!({"text": "draft prompt"}),
            },
            GraphNode {
                id: "inference".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 20.0, y: 0.0 },
                data: serde_json::json!({
                    "label": "Draft inference",
                    "authored_inference_interface": {
                        "schema_version": 1,
                        "descriptor_fingerprint": "iface.stale.v1",
                        "ports": []
                    },
                    "runtime": "cuda"
                }),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    }
}

fn json_contains_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_contains_key(value, key))
        }
        serde_json::Value::Array(values) => {
            values.iter().any(|value| json_contains_key(value, key))
        }
        _ => false,
    }
}

fn write_workflow(
    store_root: &Path,
    file_name: &str,
    workflow: &WorkflowFile,
) -> std::path::PathBuf {
    let workflows_dir = store_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    let path = workflows_dir.join(file_name);
    fs::write(
        &path,
        serde_json::to_string_pretty(workflow).expect("serialize workflow"),
    )
    .expect("write workflow");
    path
}

#[test]
fn load_workflow_rejects_parent_traversal() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());

    let err = store
        .load_workflow("../Cargo.toml".to_string())
        .expect_err("must reject traversal");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn load_workflow_rejects_absolute_path_outside_project_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let temp_file = tempfile::NamedTempFile::new().expect("temp file");

    let err = store
        .load_workflow(temp_file.path().to_string_lossy().to_string())
        .expect_err("must reject absolute path outside project root");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn load_workflow_accepts_file_inside_project_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let workflow = WorkflowFile::new(
        "Inside".to_string(),
        WorkflowGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            derived_graph: None,
        },
    );
    write_workflow(temp.path(), "Inside.json", &workflow);

    let loaded = store
        .load_workflow(".pantograph/workflows/Inside.json".to_string())
        .expect("load workflow");

    assert_eq!(loaded.metadata.id.as_deref(), Some("Inside"));
    assert_eq!(loaded.metadata.name, "Inside");
}

#[test]
fn load_workflow_rejects_invalid_workflow_identity_file_stem() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let workflow = WorkflowFile::new(
        "Invalid Name".to_string(),
        WorkflowGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            derived_graph: None,
        },
    );
    write_workflow(temp.path(), "Invalid Name.json", &workflow);

    let err = store
        .load_workflow(".pantograph/workflows/Invalid Name.json".to_string())
        .expect_err("invalid workflow identity stem should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn list_workflows_skips_invalid_workflow_identity_file_stems() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let valid = WorkflowFile::new(
        "valid-workflow".to_string(),
        WorkflowGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            derived_graph: None,
        },
    );
    let invalid = WorkflowFile::new(
        "Invalid Name".to_string(),
        WorkflowGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            derived_graph: None,
        },
    );
    write_workflow(temp.path(), "valid-workflow.json", &valid);
    write_workflow(temp.path(), "Invalid Name.json", &invalid);

    let workflows = store.list_workflows().expect("list workflows");

    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].id.as_deref(), Some("valid-workflow"));
}

#[test]
fn load_workflow_refreshes_missing_derived_graph_for_diagnostics_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let workflow = WorkflowFile::new(
        "no-fingerprint".to_string(),
        WorkflowGraph {
            nodes: vec![GraphNode {
                id: "input".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({"text": "hello"}),
            }],
            edges: Vec::new(),
            derived_graph: None,
        },
    );
    write_workflow(temp.path(), "no-fingerprint.json", &workflow);

    let loaded = store
        .load_workflow(".pantograph/workflows/no-fingerprint.json".to_string())
        .expect("load workflow");

    assert!(loaded
        .graph
        .derived_graph
        .as_ref()
        .is_some_and(|derived| !derived.graph_fingerprint.is_empty()));
}

#[test]
fn save_workflow_strips_puma_lib_derived_data_with_model_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());

    let path = store
        .save_workflow(
            "tiny-sd-turbo".to_string(),
            puma_lib_graph(sample_puma_lib_data()),
        )
        .expect("save workflow");
    let saved = fs::read_to_string(path).expect("read saved workflow");
    let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
    let data = workflow.graph.nodes[0]
        .data
        .as_object()
        .expect("saved puma-lib data object");

    assert_eq!(
        data.get("model_id").and_then(|value| value.as_str()),
        Some("diffusion/cc-nms/tiny-sd-turbo")
    );
    assert_eq!(
        data.get("selected_binding_ids")
            .and_then(|value| value.as_array())
            .map(|value| value.len()),
        Some(1)
    );
    assert!(!data.contains_key("modelPath"));
    assert!(!data.contains_key("dependency_requirements"));
    assert!(!data.contains_key("inference_settings"));
    assert!(!data.contains_key("recommended_backend"));
}

#[test]
fn save_workflow_preserves_retired_direct_diffusion_for_stale_diagnostics() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());

    let path = store
        .save_workflow(
            "retired-direct-diffusion".to_string(),
            retired_direct_diffusion_graph(),
        )
        .expect("save workflow");
    let saved = fs::read_to_string(path).expect("read saved workflow");
    let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
    let node = workflow
        .graph
        .nodes
        .iter()
        .find(|node| node.id == "diffusion")
        .expect("retired diffusion node");

    assert_eq!(node.node_type, "diffusion-inference");
    assert_eq!(node.position, Position { x: 42.0, y: 24.0 });
    assert_eq!(
        node.data["model_path"],
        serde_json::json!("/models/juggernaut")
    );
    assert_eq!(node.data["steps"], serde_json::json!(16));
    assert!(workflow
        .graph
        .derived_graph
        .as_ref()
        .is_some_and(|derived| !derived.graph_fingerprint.is_empty()));
    assert!(
        workflow.contract_upgrades.is_empty(),
        "current save must not append compatibility migration records"
    );
}

#[test]
fn draft_save_persists_invalid_inference_graph_without_executable_authority() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");

    let response = service
        .workflow_graph_save(
            &store,
            WorkflowGraphSaveRequest {
                name: "invalid inference draft".to_string(),
                graph: invalid_inference_draft_graph(),
            },
        )
        .expect("draft save must preserve editable invalid graph");

    assert!(response.path.ends_with("invalid-inference-draft.json"));

    let saved = fs::read_to_string(response.path).expect("read saved workflow");
    let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
    let saved_json: serde_json::Value = serde_json::from_str(&saved).expect("parse saved json");

    assert!(workflow
        .graph
        .nodes
        .iter()
        .any(|node| node.node_type == "llm-inference"));
    assert!(workflow
        .graph
        .derived_graph
        .as_ref()
        .is_some_and(|derived| !derived.graph_fingerprint.is_empty()));
    assert!(!json_contains_key(
        &saved_json,
        "executable_validation_snapshot"
    ));
    assert!(!json_contains_key(&saved_json, "scheduler_projection"));
    assert!(!json_contains_key(&saved_json, "queue_admission"));
    assert!(!json_contains_key(&saved_json, "submit_gate"));
}

#[test]
fn load_workflow_preserves_retired_inference_nodes_without_migration_records() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let workflow = WorkflowFile::new("legacy-llama".to_string(), retired_llamacpp_graph());
    write_workflow(temp.path(), "legacy-llama.json", &workflow);

    let loaded = store
        .load_workflow(".pantograph/workflows/legacy-llama.json".to_string())
        .expect("load workflow");
    let node = loaded
        .graph
        .nodes
        .iter()
        .find(|node| node.id == "llama")
        .expect("retired llama node");

    assert_eq!(loaded.metadata.id.as_deref(), Some("legacy-llama"));
    assert_eq!(node.node_type, "llamacpp-inference");
    assert_eq!(
        node.data["model_path"],
        serde_json::json!("/models/chat.gguf")
    );
    assert!(loaded
        .graph
        .derived_graph
        .as_ref()
        .is_some_and(|derived| !derived.graph_fingerprint.is_empty()));
    assert!(
        loaded.contract_upgrades.is_empty(),
        "current load must not append compatibility migration records"
    );
}

#[test]
fn save_workflow_strips_puma_lib_model_path_without_model_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let mut data = sample_puma_lib_data();
    data.as_object_mut().expect("object").remove("model_id");

    let path = store
        .save_workflow("path-only".to_string(), puma_lib_graph(data))
        .expect("save workflow");
    let saved = fs::read_to_string(path).expect("read saved workflow");
    let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
    let data = workflow.graph.nodes[0]
        .data
        .as_object()
        .expect("saved puma-lib data object");

    assert!(!data.contains_key("modelPath"));
    assert!(!data.contains_key("model_path"));
    assert!(!data.contains_key("dependency_requirements"));
}

#[test]
fn save_workflow_derives_identity_from_display_name() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let graph = WorkflowGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        derived_graph: None,
    };

    let path = store
        .save_workflow("Maid Workflow / Draft".to_string(), graph)
        .expect("save workflow");

    assert!(path.ends_with("Maid-Workflow-Draft.json"));
    let saved = fs::read_to_string(path).expect("read saved workflow");
    let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
    assert_eq!(workflow.metadata.name, "Maid Workflow / Draft");
}

#[test]
fn save_workflow_rejects_names_without_identity_characters() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let graph = WorkflowGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        derived_graph: None,
    };

    let err = store
        .save_workflow("///".to_string(), graph)
        .expect_err("workflow name should not collapse to an empty identity");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn delete_workflow_removes_valid_workflow_identity_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let graph = WorkflowGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        derived_graph: None,
    };

    let path = store
        .save_workflow("safe-name".to_string(), graph)
        .expect("save workflow");

    assert!(Path::new(&path).exists());

    store
        .delete_workflow("safe-name".to_string())
        .expect("delete workflow");

    assert!(!Path::new(&path).exists());
}

#[test]
fn delete_workflow_rejects_missing_workflow() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());

    let err = store
        .delete_workflow("Missing".to_string())
        .expect_err("missing workflow should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn load_workflow_strips_legacy_puma_lib_derived_data_with_model_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let workflow = WorkflowFile::new("Legacy".to_string(), puma_lib_graph(sample_puma_lib_data()));
    write_workflow(temp.path(), "Legacy.json", &workflow);

    let loaded = store
        .load_workflow(".pantograph/workflows/Legacy.json".to_string())
        .expect("load workflow");
    let data = loaded.graph.nodes[0]
        .data
        .as_object()
        .expect("loaded puma-lib data object");

    assert_eq!(
        data.get("model_id").and_then(|value| value.as_str()),
        Some("diffusion/cc-nms/tiny-sd-turbo")
    );
    assert!(!data.contains_key("modelPath"));
    assert!(!data.contains_key("dependency_requirements"));
    assert!(!data.contains_key("inference_settings"));
}

#[test]
fn load_workflow_strips_legacy_puma_lib_model_path_without_model_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let mut data = sample_puma_lib_data();
    data.as_object_mut().expect("object").remove("model_id");
    let workflow = WorkflowFile::new("legacy-path".to_string(), puma_lib_graph(data));
    write_workflow(temp.path(), "legacy-path.json", &workflow);

    let loaded = store
        .load_workflow(".pantograph/workflows/legacy-path.json".to_string())
        .expect("load workflow");
    let data = loaded.graph.nodes[0]
        .data
        .as_object()
        .expect("loaded puma-lib data object");

    assert!(!data.contains_key("modelPath"));
    assert!(!data.contains_key("model_path"));
    assert!(!data.contains_key("dependency_requirements"));
}

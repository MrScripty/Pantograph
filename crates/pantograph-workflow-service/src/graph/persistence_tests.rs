use std::fs;
use std::path::Path;

use crate::workflow::WorkflowServiceError;

use super::persistence::{FileSystemWorkflowGraphStore, WorkflowGraphStore};
use super::types::{GraphNode, Position, WorkflowFile, WorkflowGraph};

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
fn save_workflow_preserves_puma_lib_model_path_without_model_identity() {
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

    assert_eq!(
        data.get("modelPath").and_then(|value| value.as_str()),
        Some("/old/path/tiny-sd-turbo")
    );
    assert!(!data.contains_key("dependency_requirements"));
}

#[test]
fn save_workflow_rejects_invalid_workflow_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let graph = WorkflowGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        derived_graph: None,
    };

    let err = store
        .save_workflow("Unsafe/Name".to_string(), graph)
        .expect_err("invalid workflow identity should fail");

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
fn load_workflow_preserves_legacy_puma_lib_model_path_without_model_identity() {
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

    assert_eq!(
        data.get("modelPath").and_then(|value| value.as_str()),
        Some("/old/path/tiny-sd-turbo")
    );
    assert!(!data.contains_key("dependency_requirements"));
}

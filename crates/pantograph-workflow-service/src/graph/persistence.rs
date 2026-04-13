use std::fs;
use std::path::{Path, PathBuf};

use node_engine::resolve_path_within_root;

use crate::workflow::WorkflowServiceError;

use super::types::{WorkflowFile, WorkflowGraph, WorkflowGraphMetadata};

const PUMA_LIB_DERIVED_DATA_KEYS: &[&str] = &[
    "modelPath",
    "model_path",
    "model_type",
    "modelType",
    "task_type_primary",
    "taskTypePrimary",
    "backend_key",
    "backendKey",
    "recommended_backend",
    "recommendedBackend",
    "runtime_engine_hints",
    "runtimeEngineHints",
    "requires_custom_code",
    "requiresCustomCode",
    "custom_code_sources",
    "customCodeSources",
    "platform_context",
    "platformContext",
    "dependency_bindings",
    "dependencyBindings",
    "dependency_requirements",
    "dependencyRequirements",
    "dependency_requirements_id",
    "dependencyRequirementsId",
    "inference_settings",
    "inferenceSettings",
    "review_reasons",
    "reviewReasons",
];

fn sanitize_puma_lib_node_data(data: &mut serde_json::Value) {
    let Some(object) = data.as_object_mut() else {
        return;
    };

    for key in PUMA_LIB_DERIVED_DATA_KEYS {
        object.remove(*key);
    }
}

fn sanitize_workflow_graph_persistence_state(graph: &mut WorkflowGraph) {
    for node in &mut graph.nodes {
        if node.node_type == "puma-lib" {
            sanitize_puma_lib_node_data(&mut node.data);
        }
    }
}

pub trait WorkflowGraphStore: Send + Sync {
    fn save_workflow(
        &self,
        name: String,
        graph: WorkflowGraph,
    ) -> Result<String, WorkflowServiceError>;

    fn load_workflow(&self, path: String) -> Result<WorkflowFile, WorkflowServiceError>;

    fn list_workflows(&self) -> Result<Vec<WorkflowGraphMetadata>, WorkflowServiceError>;
}

#[derive(Debug, Clone)]
pub struct FileSystemWorkflowGraphStore {
    project_root: PathBuf,
}

impl FileSystemWorkflowGraphStore {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    fn workflows_dir(&self) -> Result<PathBuf, WorkflowServiceError> {
        let workflows_dir = self.project_root.join(".pantograph").join("workflows");
        fs::create_dir_all(&workflows_dir).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to create workflows directory: {}", e))
        })?;
        Ok(workflows_dir)
    }

    pub fn from_current_crate_root() -> Self {
        let project_root = resolve_runtime_project_root()
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        Self::new(project_root)
    }
}

impl Default for FileSystemWorkflowGraphStore {
    fn default() -> Self {
        Self::from_current_crate_root()
    }
}

fn resolve_runtime_project_root() -> Option<PathBuf> {
    fn looks_like_project_root(path: &Path) -> bool {
        path.join("Cargo.toml").is_file() && path.join("src-tauri").join("Cargo.toml").is_file()
    }

    fn find_project_root_from(seed: &Path) -> Option<PathBuf> {
        let start = if seed.is_file() { seed.parent()? } else { seed };
        for candidate in start.ancestors() {
            if looks_like_project_root(candidate) {
                return Some(candidate.to_path_buf());
            }
        }
        None
    }

    let mut seeds = Vec::new();
    if let Some(path) = std::env::var_os("PANTOGRAPH_PROJECT_ROOT") {
        seeds.push(PathBuf::from(path));
    }
    if let Ok(exe_path) = std::env::current_exe() {
        seeds.push(exe_path);
    }
    if let Ok(current_dir) = std::env::current_dir() {
        seeds.push(current_dir);
    }
    seeds.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    seeds
        .into_iter()
        .find_map(|seed| find_project_root_from(&seed))
}

impl WorkflowGraphStore for FileSystemWorkflowGraphStore {
    fn save_workflow(
        &self,
        name: String,
        graph: WorkflowGraph,
    ) -> Result<String, WorkflowServiceError> {
        let workflows_dir = self.workflows_dir()?;
        let mut graph = graph;
        sanitize_workflow_graph_persistence_state(&mut graph);
        graph.refresh_derived_graph();

        let safe_name: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let file_path = workflows_dir.join(format!("{}.json", safe_name));

        let workflow_file = if file_path.exists() {
            let existing = fs::read_to_string(&file_path).map_err(|e| {
                WorkflowServiceError::Internal(format!("Failed to read existing workflow: {}", e))
            })?;
            let mut existing: WorkflowFile = serde_json::from_str(&existing).map_err(|e| {
                WorkflowServiceError::Internal(format!("Failed to parse existing workflow: {}", e))
            })?;

            existing.metadata.name = name;
            existing.metadata.modified = chrono::Utc::now().to_rfc3339();
            existing.graph = graph;
            existing
        } else {
            WorkflowFile::new(name, graph)
        };

        let json = serde_json::to_string_pretty(&workflow_file).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to serialize workflow: {}", e))
        })?;

        fs::write(&file_path, json).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to write workflow file: {}", e))
        })?;

        Ok(file_path.to_string_lossy().to_string())
    }

    fn load_workflow(&self, path: String) -> Result<WorkflowFile, WorkflowServiceError> {
        let full_path = resolve_path_within_root(&path, &self.project_root).map_err(|e| {
            WorkflowServiceError::InvalidRequest(format!("Invalid workflow path '{}': {}", path, e))
        })?;

        let content = fs::read_to_string(&full_path).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to read workflow file: {}", e))
        })?;

        let mut workflow: WorkflowFile = serde_json::from_str(&content).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to parse workflow file: {}", e))
        })?;
        sanitize_workflow_graph_persistence_state(&mut workflow.graph);
        Ok(workflow)
    }

    fn list_workflows(&self) -> Result<Vec<WorkflowGraphMetadata>, WorkflowServiceError> {
        let workflows_dir = self.workflows_dir()?;
        let entries = fs::read_dir(&workflows_dir).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to read workflows directory: {}", e))
        })?;

        let mut workflows = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                WorkflowServiceError::Internal(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let Ok(content) = fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(mut workflow) = serde_json::from_str::<WorkflowFile>(&content) else {
                    continue;
                };
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    workflow.metadata.id = Some(stem.to_string());
                }
                workflows.push(workflow.metadata);
            }
        }

        workflows.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(workflows)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphSaveRequest {
    pub name: String,
    pub graph: WorkflowGraph,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphSaveResponse {
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphLoadRequest {
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphListResponse {
    pub workflows: Vec<WorkflowGraphMetadata>,
}

#[cfg(test)]
mod tests {
    use super::super::types::{GraphNode, Position};
    use super::*;

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

    #[test]
    fn save_workflow_strips_puma_lib_derived_data() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileSystemWorkflowGraphStore::new(temp.path());
        let graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "puma-1".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: sample_puma_lib_data(),
            }],
            edges: Vec::new(),
            derived_graph: None,
        };

        let path = store
            .save_workflow("Tiny SD Turbo".to_string(), graph)
            .expect("save workflow");
        let saved = fs::read_to_string(path).expect("read saved workflow");
        let workflow: WorkflowFile = serde_json::from_str(&saved).expect("parse saved workflow");
        let node = workflow.graph.nodes.first().expect("saved puma-lib node");
        let data = node.data.as_object().expect("saved puma-lib data object");

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
    fn load_workflow_strips_legacy_puma_lib_derived_data() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileSystemWorkflowGraphStore::new(temp.path());
        let workflows_dir = temp.path().join(".pantograph").join("workflows");
        fs::create_dir_all(&workflows_dir).expect("create workflows dir");
        let path = workflows_dir.join("Legacy.json");

        let workflow = WorkflowFile::new(
            "Legacy".to_string(),
            WorkflowGraph {
                nodes: vec![GraphNode {
                    id: "puma-1".to_string(),
                    node_type: "puma-lib".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: sample_puma_lib_data(),
                }],
                edges: Vec::new(),
                derived_graph: None,
            },
        );
        fs::write(
            &path,
            serde_json::to_string_pretty(&workflow).expect("serialize workflow"),
        )
        .expect("write workflow");

        let loaded = store
            .load_workflow(".pantograph/workflows/Legacy.json".to_string())
            .expect("load workflow");
        let node = loaded.graph.nodes.first().expect("loaded puma-lib node");
        let data = node.data.as_object().expect("loaded puma-lib data object");

        assert_eq!(
            data.get("model_id").and_then(|value| value.as_str()),
            Some("diffusion/cc-nms/tiny-sd-turbo")
        );
        assert!(!data.contains_key("modelPath"));
        assert!(!data.contains_key("dependency_requirements"));
        assert!(!data.contains_key("inference_settings"));
    }
}

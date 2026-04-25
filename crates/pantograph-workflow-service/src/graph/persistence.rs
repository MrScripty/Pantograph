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
    let has_model_identity = object
        .get("model_id")
        .or_else(|| object.get("modelId"))
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());

    for key in PUMA_LIB_DERIVED_DATA_KEYS {
        if !has_model_identity && (*key == "modelPath" || *key == "model_path") {
            continue;
        }
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

    fn delete_workflow(&self, name: String) -> Result<(), WorkflowServiceError>;
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

fn sanitized_workflow_file_stem(name: &str) -> Result<String, WorkflowServiceError> {
    let safe_name: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if safe_name.is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(
            "Workflow name cannot be empty".to_string(),
        ));
    }

    Ok(safe_name)
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

        let safe_name = sanitized_workflow_file_stem(&name)?;
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

    fn delete_workflow(&self, name: String) -> Result<(), WorkflowServiceError> {
        let workflows_dir = self.workflows_dir()?;
        let safe_name = sanitized_workflow_file_stem(&name)?;
        let file_path = workflows_dir.join(format!("{}.json", safe_name));

        if !file_path.exists() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "Workflow '{}' does not exist",
                name
            )));
        }

        fs::remove_file(&file_path).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to delete workflow file: {}", e))
        })
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphDeleteRequest {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphDeleteResponse {}

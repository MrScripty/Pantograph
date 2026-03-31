use std::fs;
use std::path::{Path, PathBuf};

use node_engine::resolve_path_within_root;

use crate::workflow::WorkflowServiceError;

use super::types::{WorkflowFile, WorkflowGraph, WorkflowGraphMetadata};

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

        serde_json::from_str(&content).map_err(|e| {
            WorkflowServiceError::Internal(format!("Failed to parse workflow file: {}", e))
        })
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

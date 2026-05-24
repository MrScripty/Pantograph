use std::fs;
use std::path::{Path, PathBuf};

use node_engine::resolve_path_within_root;
use pantograph_node_contracts::ContractUpgradeRecord;

use crate::workflow::{WorkflowIdentity, WorkflowServiceError};

use super::canonicalization::{
    canonicalize_workflow_graph_with_migrations, WorkflowGraphCanonicalizationResult,
};
use super::registry::NodeRegistry;
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

fn append_contract_upgrade_records(
    records: &mut Vec<ContractUpgradeRecord>,
    new_records: Vec<ContractUpgradeRecord>,
) {
    for record in new_records {
        if !records.contains(&record) {
            records.push(record);
        }
    }
}

fn canonicalize_workflow_graph_for_persistence(
    graph: WorkflowGraph,
) -> WorkflowGraphCanonicalizationResult {
    let mut result = canonicalize_workflow_graph_with_migrations(graph, &NodeRegistry::new());
    sanitize_workflow_graph_persistence_state(&mut result.graph);
    result.graph.refresh_derived_graph();
    result
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

fn workflow_identity_file_stem(name: &str) -> Result<String, WorkflowServiceError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(
            "workflow name must be non-empty".to_string(),
        ));
    }

    let mut file_stem = String::new();
    let mut last_was_separator = false;
    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            file_stem.push(character);
            last_was_separator = false;
        } else if matches!(character, '.' | '-' | '_') {
            if !file_stem.is_empty() && !last_was_separator {
                file_stem.push(character);
                last_was_separator = true;
            }
        } else if !file_stem.is_empty() && !last_was_separator {
            file_stem.push('-');
            last_was_separator = true;
        }
    }

    while file_stem
        .chars()
        .last()
        .is_some_and(|character| !character.is_ascii_alphanumeric())
    {
        file_stem.pop();
    }

    if file_stem.len() > 96 {
        file_stem.truncate(96);
        while file_stem
            .chars()
            .last()
            .is_some_and(|character| !character.is_ascii_alphanumeric())
        {
            file_stem.pop();
        }
    }

    WorkflowIdentity::parse(file_stem)
        .map(WorkflowIdentity::into_string)
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
}

impl WorkflowGraphStore for FileSystemWorkflowGraphStore {
    fn save_workflow(
        &self,
        name: String,
        graph: WorkflowGraph,
    ) -> Result<String, WorkflowServiceError> {
        let workflows_dir = self.workflows_dir()?;
        let canonicalized = canonicalize_workflow_graph_for_persistence(graph);
        let graph = canonicalized.graph;
        let migration_records = canonicalized.migration_records;

        let safe_name = workflow_identity_file_stem(&name)?;
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
            append_contract_upgrade_records(&mut existing.contract_upgrades, migration_records);
            existing
        } else {
            let mut workflow = WorkflowFile::new(name, graph);
            append_contract_upgrade_records(&mut workflow.contract_upgrades, migration_records);
            workflow
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
        if let Some(stem) = full_path.file_stem().and_then(|s| s.to_str()) {
            WorkflowIdentity::parse(stem)
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            workflow.metadata.id = Some(stem.to_string());
        }
        let canonicalized = canonicalize_workflow_graph_for_persistence(workflow.graph);
        workflow.graph = canonicalized.graph;
        append_contract_upgrade_records(
            &mut workflow.contract_upgrades,
            canonicalized.migration_records,
        );
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
                    if WorkflowIdentity::parse(stem).is_err() {
                        continue;
                    }
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
        let safe_name = workflow_identity_file_stem(&name)?;
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

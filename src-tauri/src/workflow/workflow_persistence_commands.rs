use std::fs;
use std::path::PathBuf;

use crate::project_root::resolve_project_root;
use node_engine::resolve_path_within_root;

use super::types::{WorkflowFile, WorkflowGraph, WorkflowMetadata};

fn get_workflows_dir() -> Result<PathBuf, String> {
    let project_root = resolve_project_root()?;
    let workflows_dir = project_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;
    Ok(workflows_dir)
}

pub fn save_workflow(name: String, graph: WorkflowGraph) -> Result<String, String> {
    let workflows_dir = get_workflows_dir()?;
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
        let existing = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read existing workflow: {}", e))?;
        let mut existing: WorkflowFile = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse existing workflow: {}", e))?;

        existing.metadata.name = name;
        existing.metadata.modified = chrono::Utc::now().to_rfc3339();
        existing.graph = graph;
        existing
    } else {
        WorkflowFile::new(name, graph)
    };

    let json = serde_json::to_string_pretty(&workflow_file)
        .map_err(|e| format!("Failed to serialize workflow: {}", e))?;

    fs::write(&file_path, json).map_err(|e| format!("Failed to write workflow file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

pub fn load_workflow(path: String) -> Result<WorkflowFile, String> {
    let project_root = resolve_project_root()?;
    let full_path = resolve_path_within_root(&path, &project_root)
        .map_err(|e| format!("Invalid workflow path '{}': {}", path, e))?;

    let content = fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;

    let workflow: WorkflowFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file: {}", e))?;

    Ok(workflow)
}

pub fn list_workflows() -> Result<Vec<WorkflowMetadata>, String> {
    let workflows_dir = get_workflows_dir()?;

    let mut workflows = Vec::new();

    let entries = fs::read_dir(&workflows_dir)
        .map_err(|e| format!("Failed to read workflows directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Ok(mut workflow) = serde_json::from_str::<WorkflowFile>(&content) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            workflow.metadata.id = Some(stem.to_string());
                        }
                        workflows.push(workflow.metadata);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read workflow file {:?}: {}", path, e);
                }
            }
        }
    }

    workflows.sort_by(|a, b| b.modified.cmp(&a.modified));

    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn project_root() -> PathBuf {
        resolve_project_root().expect("resolve project root")
    }

    #[test]
    fn test_load_workflow_rejects_parent_traversal() {
        let err = load_workflow("../Cargo.toml".to_string()).expect_err("must reject traversal");
        assert!(err.contains("Invalid workflow path"));
    }

    #[test]
    fn test_load_workflow_rejects_absolute_path_outside_project_root() {
        let temp_file = std::env::temp_dir().join(format!(
            "pantograph-workflow-outside-{}.json",
            Uuid::new_v4()
        ));
        fs::write(&temp_file, "{}").expect("write temp file");

        let err = load_workflow(temp_file.to_string_lossy().to_string())
            .expect_err("must reject absolute path outside project root");
        assert!(err.contains("Invalid workflow path"));

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_load_workflow_accepts_file_inside_project_root() {
        let root = project_root();
        let workflows_dir = root.join(".pantograph").join("workflows");
        fs::create_dir_all(&workflows_dir).expect("create workflows dir");

        let file_name = format!("test-load-workflow-{}.json", Uuid::new_v4());
        let workflow_path = workflows_dir.join(&file_name);
        let relative_path = format!(".pantograph/workflows/{}", file_name);

        let workflow = WorkflowFile::new(
            "test workflow".to_string(),
            WorkflowGraph {
                nodes: vec![],
                edges: vec![],
                derived_graph: None,
            },
        );
        let json = serde_json::to_string_pretty(&workflow).expect("serialize workflow");
        fs::write(&workflow_path, json).expect("write workflow");

        let loaded = load_workflow(relative_path).expect("load workflow");
        assert_eq!(loaded.metadata.name, "test workflow");

        let _ = fs::remove_file(&workflow_path);
    }
}

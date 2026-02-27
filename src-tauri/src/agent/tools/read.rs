use node_engine::resolve_path_within_root;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use super::error::ToolError;

// ============================================================================
// ReadGuiFileTool - Read existing Svelte component source
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ReadGuiFileArgs {
    pub path: String,
}

#[derive(Clone)]
pub struct ReadGuiFileTool {
    project_root: PathBuf,
}

impl ReadGuiFileTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn get_generated_path(&self) -> PathBuf {
        self.project_root.join("src").join("generated")
    }
}

impl Tool for ReadGuiFileTool {
    const NAME: &'static str = "read_gui_file";
    type Error = ToolError;
    type Args = ReadGuiFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "Read the source code of an existing Svelte component from the generated directory"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the component file (e.g., 'Button.svelte' or 'forms/Input.svelte')"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let generated_root = self.get_generated_path();
        let full_path = resolve_path_within_root(&args.path, &generated_root)
            .map_err(|e| ToolError::PathNotAllowed(format!("{} ({})", args.path, e)))?;

        tokio::fs::read_to_string(full_path)
            .await
            .map_err(ToolError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn make_temp_project_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("pantograph-read-tool-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("src").join("generated")).expect("create generated dir");
        root
    }

    #[tokio::test]
    async fn test_read_gui_file_rejects_parent_traversal() {
        let project_root = make_temp_project_root();
        let tool = ReadGuiFileTool::new(project_root.clone());

        let err = tool
            .call(ReadGuiFileArgs {
                path: "../secret.svelte".to_string(),
            })
            .await
            .expect_err("must reject traversal path");

        assert!(matches!(err, ToolError::PathNotAllowed(_)));
        let _ = fs::remove_dir_all(project_root);
    }

    #[tokio::test]
    async fn test_read_gui_file_allows_relative_file_inside_generated_root() {
        let project_root = make_temp_project_root();
        let file_path = project_root
            .join("src")
            .join("generated")
            .join("TestComponent.svelte");
        fs::write(&file_path, "<div>ok</div>").expect("write component");

        let tool = ReadGuiFileTool::new(project_root.clone());
        let content = tool
            .call(ReadGuiFileArgs {
                path: "TestComponent.svelte".to_string(),
            })
            .await
            .expect("read component");

        assert_eq!(content, "<div>ok</div>");
        let _ = fs::remove_dir_all(project_root);
    }
}

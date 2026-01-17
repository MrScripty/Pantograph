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
            description: "Read the source code of an existing Svelte component from the generated directory".to_string(),
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
        // Sanitize path to prevent directory traversal
        let sanitized = args.path.replace("..", "").trim_start_matches('/').to_string();
        let full_path = self.get_generated_path().join(&sanitized);

        // Verify the path is within the generated directory
        let canonical = full_path.canonicalize().map_err(ToolError::Io)?;
        let generated_canonical = self.get_generated_path().canonicalize().map_err(ToolError::Io)?;

        if !canonical.starts_with(&generated_canonical) {
            return Err(ToolError::PathNotAllowed(args.path));
        }

        tokio::fs::read_to_string(full_path).await.map_err(ToolError::Io)
    }
}

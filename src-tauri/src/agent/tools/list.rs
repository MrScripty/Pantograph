use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use super::error::ToolError;
use crate::agent::types::TemplateInfo;

// ============================================================================
// ListComponentsTool - List existing component files
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListComponentsArgs {}

#[derive(Clone)]
pub struct ListComponentsTool {
    project_root: PathBuf,
}

impl ListComponentsTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn get_generated_path(&self) -> PathBuf {
        self.project_root.join("src").join("generated")
    }
}

impl Tool for ListComponentsTool {
    const NAME: &'static str = "list_components";
    type Error = ToolError;
    type Args = ListComponentsArgs;
    type Output = Vec<String>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List all existing Svelte component files in the generated directory".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let generated_path = self.get_generated_path();
        let mut components = Vec::new();

        if !generated_path.exists() {
            return Ok(components);
        }

        fn collect_svelte_files(dir: &PathBuf, base: &PathBuf, files: &mut Vec<String>) -> std::io::Result<()> {
            if dir.is_dir() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        collect_svelte_files(&path, base, files)?;
                    } else if path.extension().map_or(false, |ext| ext == "svelte") {
                        if let Ok(relative) = path.strip_prefix(base) {
                            files.push(relative.to_string_lossy().to_string());
                        }
                    }
                }
            }
            Ok(())
        }

        collect_svelte_files(&generated_path, &generated_path, &mut components)
            .map_err(ToolError::Io)?;

        Ok(components)
    }
}

// ============================================================================
// ListTemplatesTool - List available UI templates
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListTemplatesArgs {}

#[derive(Clone)]
pub struct ListTemplatesTool {
    project_root: PathBuf,
}

impl ListTemplatesTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn get_templates_path(&self) -> PathBuf {
        self.project_root.join("src").join("templates")
    }
}

impl Tool for ListTemplatesTool {
    const NAME: &'static str = "list_templates";
    type Error = ToolError;
    type Args = ListTemplatesArgs;
    type Output = Vec<TemplateInfo>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List available UI component templates that can be used as reference".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let templates_path = self.get_templates_path();
        let mut templates = Vec::new();

        if !templates_path.exists() {
            return Ok(templates);
        }

        for entry in std::fs::read_dir(&templates_path).map_err(ToolError::Io)? {
            let entry = entry.map_err(ToolError::Io)?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "svelte") {
                let name = path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Try to extract description from the first comment in the file
                let description = if let Ok(content) = std::fs::read_to_string(&path) {
                    content.lines()
                        .find(|line| line.trim().starts_with("<!--"))
                        .and_then(|line| {
                            let trimmed = line.trim();
                            if trimmed.ends_with("-->") {
                                Some(trimmed[4..trimmed.len()-3].trim().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| format!("{} component template", name))
                } else {
                    format!("{} component template", name)
                };

                templates.push(TemplateInfo {
                    name: name.clone(),
                    description,
                    path: format!("{}.svelte", name),
                });
            }
        }

        Ok(templates)
    }
}

// ============================================================================
// ReadTemplateTool - Read a specific template source
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ReadTemplateArgs {
    pub name: String,
}

#[derive(Clone)]
pub struct ReadTemplateTool {
    project_root: PathBuf,
}

impl ReadTemplateTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn get_templates_path(&self) -> PathBuf {
        self.project_root.join("src").join("templates")
    }
}

impl Tool for ReadTemplateTool {
    const NAME: &'static str = "read_template";
    type Error = ToolError;
    type Args = ReadTemplateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the source code of a UI template component for reference".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the template to read (e.g., 'Button', 'Card')"
                    }
                },
                "required": ["name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Sanitize name - remove any path separators and extensions
        let sanitized = args.name
            .replace("..", "")
            .replace('/', "")
            .replace('\\', "")
            .trim_end_matches(".svelte")
            .to_string();

        let full_path = self.get_templates_path().join(format!("{}.svelte", sanitized));

        if !full_path.exists() {
            return Err(ToolError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Template '{}' not found", args.name)
            )));
        }

        tokio::fs::read_to_string(full_path).await.map_err(ToolError::Io)
    }
}

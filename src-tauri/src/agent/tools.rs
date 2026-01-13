use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use thiserror::Error;

use super::types::{TailwindColors, TemplateInfo};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

impl Serialize for ToolError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

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

// ============================================================================
// WriteGuiFileTool - Create or update a Svelte component
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WriteGuiFileArgs {
    pub path: String,
    pub content: String,
}

#[derive(Clone)]
pub struct WriteGuiFileTool {
    project_root: PathBuf,
}

impl WriteGuiFileTool {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn get_generated_path(&self) -> PathBuf {
        self.project_root.join("src").join("generated")
    }

    fn validate_svelte_content(&self, content: &str) -> Result<(), ToolError> {
        // ============================================================
        // Svelte 5 Syntax Validation - CRITICAL
        // These patterns cause compilation errors in Svelte 5 runes mode
        // ============================================================
        let forbidden_patterns: &[(&str, &str)] = &[
            // Props - must use $props() not export let
            ("export let ", "Use `let { prop } = $props()` instead of `export let prop`"),
            ("export let\t", "Use `let { prop } = $props()` instead of `export let prop`"),
            // Event handlers - must use lowercase without colon
            ("on:click", "Use `onclick` instead of `on:click`"),
            ("on:change", "Use `onchange` instead of `on:change`"),
            ("on:input", "Use `oninput` instead of `on:input`"),
            ("on:submit", "Use `onsubmit` instead of `on:submit`"),
            ("on:keydown", "Use `onkeydown` instead of `on:keydown`"),
            ("on:keyup", "Use `onkeyup` instead of `on:keyup`"),
            ("on:keypress", "Use `onkeypress` instead of `on:keypress`"),
            ("on:mouseenter", "Use `onmouseenter` instead of `on:mouseenter`"),
            ("on:mouseleave", "Use `onmouseleave` instead of `on:mouseleave`"),
            ("on:mouseover", "Use `onmouseover` instead of `on:mouseover`"),
            ("on:mouseout", "Use `onmouseout` instead of `on:mouseout`"),
            ("on:mousedown", "Use `onmousedown` instead of `on:mousedown`"),
            ("on:mouseup", "Use `onmouseup` instead of `on:mouseup`"),
            ("on:focus", "Use `onfocus` instead of `on:focus`"),
            ("on:blur", "Use `onblur` instead of `on:blur`"),
            ("on:scroll", "Use `onscroll` instead of `on:scroll`"),
            ("on:resize", "Use `onresize` instead of `on:resize`"),
            ("on:load", "Use `onload` instead of `on:load`"),
            ("on:error", "Use `onerror` instead of `on:error`"),
        ];

        for (pattern, fix) in forbidden_patterns {
            if content.contains(pattern) {
                return Err(ToolError::Validation(format!(
                    "SVELTE 5 SYNTAX ERROR: Found forbidden pattern '{}'. {}. \
                     Svelte 5 uses runes mode - you MUST use $props() for props and \
                     lowercase event handlers (onclick, onchange, etc.). \
                     Please rewrite the component using correct Svelte 5 syntax.",
                    pattern, fix
                )));
            }
        }

        // ============================================================
        // CSS Validation - Tailwind only
        // ============================================================
        if content.contains("<style>") && !content.contains("@apply") && !content.contains("global(") {
            // Allow style blocks only if they use @apply or :global()
            let style_start = content.find("<style>");
            let style_end = content.find("</style>");
            if let (Some(start), Some(end)) = (style_start, style_end) {
                let style_content = &content[start..end];
                // Check if the style block contains non-Tailwind CSS
                if !style_content.contains("@apply") && !style_content.contains(":global") {
                    return Err(ToolError::Validation(
                        "Custom CSS not allowed. Use Tailwind classes only, or @apply directive.".to_string()
                    ));
                }
            }
        }

        // ============================================================
        // Basic syntax check - verify balanced tags
        // ============================================================
        let script_opens = content.matches("<script").count();
        let script_closes = content.matches("</script>").count();
        if script_opens != script_closes {
            return Err(ToolError::Validation("Unbalanced <script> tags".to_string()));
        }

        Ok(())
    }
}

impl Tool for WriteGuiFileTool {
    const NAME: &'static str = "write_gui_file";
    type Error = ToolError;
    type Args = WriteGuiFileArgs;
    type Output = bool;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create or update a Svelte component file in the generated directory. Use Tailwind CSS classes only - no custom CSS.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path for the component file (e.g., 'Button.svelte')"
                    },
                    "content": {
                        "type": "string",
                        "description": "Complete Svelte component source code using Svelte 5 syntax"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Step 1: Basic pattern validation (fast check for obvious errors)
        self.validate_svelte_content(&args.content)?;

        // Sanitize path to prevent directory traversal
        let sanitized = args.path.replace("..", "").trim_start_matches('/').to_string();
        let full_path = self.get_generated_path().join(&sanitized);

        // Ensure the generated directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(ToolError::Io)?;
        }

        // Step 2: Write to a temp file first for validation
        let temp_path = full_path.with_extension("svelte.tmp");
        tokio::fs::write(&temp_path, &args.content).await.map_err(ToolError::Io)?;

        // Step 3: Validate with Svelte compiler
        let validation_script = self.project_root.join("scripts").join("validate-svelte.mjs");
        let validation_result = tokio::process::Command::new("node")
            .arg(&validation_script)
            .arg(&temp_path)
            .output()
            .await;

        match validation_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if !output.status.success() {
                    // Compilation failed - parse error and return to agent
                    let _ = tokio::fs::remove_file(&temp_path).await;

                    // Try to parse JSON error output
                    if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let error_msg = error_json.get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown compilation error");
                        let line = error_json.get("line")
                            .and_then(|l| l.as_u64());

                        let mut full_error = format!(
                            "SVELTE COMPILATION ERROR: {}",
                            error_msg
                        );
                        if let Some(line_num) = line {
                            full_error.push_str(&format!(" (line {})", line_num));
                        }
                        full_error.push_str(". Please fix the syntax and try again. Remember: use $props() for props (NOT export let), use onclick (NOT on:click), and $state() should only be used for local state variables, NOT inside $props() destructuring.");

                        return Err(ToolError::Validation(full_error));
                    } else {
                        // Couldn't parse JSON, use raw output
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        return Err(ToolError::Validation(format!(
                            "SVELTE COMPILATION ERROR: {}. Please fix the syntax and try again.",
                            stdout.trim()
                        )));
                    }
                }

                // Compilation succeeded - move temp file to final location
                tokio::fs::rename(&temp_path, &full_path).await.map_err(ToolError::Io)?;
                Ok(true)
            }
            Err(e) => {
                // Node command failed to run - fall back to allowing the file
                // (validation script might not be available)
                log::warn!("Svelte validation script failed to run: {}. Proceeding without compiler validation.", e);
                let _ = tokio::fs::remove_file(&temp_path).await;
                tokio::fs::write(&full_path, &args.content).await.map_err(ToolError::Io)?;
                Ok(true)
            }
        }
    }
}

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
// GetTailwindColorsTool - Return available Tailwind color palette
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetTailwindColorsArgs {}

#[derive(Clone)]
pub struct GetTailwindColorsTool;

impl GetTailwindColorsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetTailwindColorsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GetTailwindColorsTool {
    const NAME: &'static str = "get_tailwind_colors";
    type Error = ToolError;
    type Args = GetTailwindColorsArgs;
    type Output = TailwindColors;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the available Tailwind CSS color palette with all color names and their shades".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        use std::collections::HashMap;

        let mut colors = HashMap::new();

        // Standard Tailwind color palette
        let shades = vec![
            "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"
        ].into_iter().map(String::from).collect::<Vec<_>>();

        for color in &["slate", "gray", "zinc", "neutral", "stone", "red", "orange", "amber",
                       "yellow", "lime", "green", "emerald", "teal", "cyan", "sky", "blue",
                       "indigo", "violet", "purple", "fuchsia", "pink", "rose"] {
            colors.insert(color.to_string(), shades.clone());
        }

        // Special colors
        colors.insert("white".to_string(), vec!["".to_string()]);
        colors.insert("black".to_string(), vec!["".to_string()]);
        colors.insert("transparent".to_string(), vec!["".to_string()]);

        Ok(TailwindColors { colors })
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

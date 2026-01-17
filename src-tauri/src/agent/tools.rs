use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use super::enricher::{EnricherRegistry, ErrorCategory};
use super::rag::SharedRagManager;
use super::types::{TailwindColors, TemplateInfo, WriteTracker};
use crate::config::{ImportValidationMode, SandboxConfig};
use crate::hotload_sandbox::runtime_sandbox::validate_runtime_semantics;

// ============================================================================
// Standard HTML Elements (for validation)
// ============================================================================

const STANDARD_HTML_ELEMENTS: &[&str] = &[
    // Document metadata
    "base", "head", "link", "meta", "style", "title",
    // Sectioning
    "body", "address", "article", "aside", "footer", "header",
    "h1", "h2", "h3", "h4", "h5", "h6", "hgroup", "main", "nav", "section", "search",
    // Text content
    "blockquote", "dd", "div", "dl", "dt", "figcaption", "figure", "hr", "li", "menu", "ol", "p", "pre", "ul",
    // Inline text semantics
    "a", "abbr", "b", "bdi", "bdo", "br", "cite", "code", "data", "dfn", "em", "i", "kbd", "mark", "q",
    "rp", "rt", "ruby", "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "wbr",
    // Image and multimedia
    "area", "audio", "img", "map", "track", "video",
    // Embedded content
    "embed", "iframe", "object", "param", "picture", "portal", "source",
    // SVG and MathML
    "svg", "math",
    // Scripting
    "canvas", "noscript", "script",
    // Edits
    "del", "ins",
    // Table content
    "caption", "col", "colgroup", "table", "tbody", "td", "tfoot", "th", "thead", "tr",
    // Forms
    "button", "datalist", "fieldset", "form", "input", "label", "legend", "meter",
    "optgroup", "option", "output", "progress", "select", "textarea",
    // Interactive elements
    "details", "dialog", "summary",
    // Web Components
    "slot", "template",
];

/// Capitalize the first letter of a string (for suggesting PascalCase component names)
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

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
    write_tracker: Option<WriteTracker>,
    sandbox_config: SandboxConfig,
    enricher_registry: Arc<EnricherRegistry>,
}

impl WriteGuiFileTool {
    pub fn new(project_root: PathBuf, enricher_registry: Arc<EnricherRegistry>) -> Self {
        Self {
            project_root,
            write_tracker: None,
            sandbox_config: SandboxConfig::default(),
            enricher_registry,
        }
    }

    pub fn with_tracker(project_root: PathBuf, tracker: WriteTracker, enricher_registry: Arc<EnricherRegistry>) -> Self {
        Self {
            project_root,
            write_tracker: Some(tracker),
            sandbox_config: SandboxConfig::default(),
            enricher_registry,
        }
    }

    pub fn with_sandbox_config(mut self, config: SandboxConfig) -> Self {
        self.sandbox_config = config;
        self
    }

    /// Enrich an error message with relevant documentation and return as ToolError::Validation
    async fn validation_error(&self, message: String, category: ErrorCategory) -> ToolError {
        let enriched = self.enricher_registry.enrich(&message, &category).await;
        ToolError::Validation(enriched)
    }

    fn get_generated_path(&self) -> PathBuf {
        self.project_root.join("src").join("generated")
    }

    /// Extract template content from Svelte file (excludes script and style blocks)
    fn extract_template_content(content: &str) -> String {
        let mut result = content.to_string();

        // Remove <script>...</script> blocks
        while let Some(start) = result.find("<script") {
            if let Some(end) = result[start..].find("</script>") {
                result = format!("{}{}", &result[..start], &result[start + end + 9..]);
            } else {
                break;
            }
        }

        // Remove <style>...</style> blocks
        while let Some(start) = result.find("<style") {
            if let Some(end) = result[start..].find("</style>") {
                result = format!("{}{}", &result[..start], &result[start + end + 8..]);
            } else {
                break;
            }
        }

        result
    }

    fn validate_svelte_content(&self, content: &str) -> Result<(), (String, ErrorCategory)> {
        // ============================================================
        // Svelte 5 Syntax Validation - CRITICAL
        // These patterns cause compilation errors in Svelte 5 runes mode
        // ============================================================

        // Strip comments before validation to avoid false positives
        // (e.g., comments explaining what NOT to do shouldn't trigger errors)
        let content_no_comments: String = content
            .lines()
            .map(|line| {
                // Remove single-line comments (// ...)
                if let Some(idx) = line.find("//") {
                    &line[..idx]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

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
            if content_no_comments.contains(pattern) {
                // Return a tuple with the error message and category
                // The caller will enrich this with documentation
                return Err((
                    format!(
                        "SVELTE 5 SYNTAX ERROR: Found forbidden pattern '{}'. {}. \
                         Svelte 5 uses runes mode - you MUST use $props() for props and \
                         lowercase event handlers (onclick, onchange, etc.). \
                         Please rewrite the component using correct Svelte 5 syntax.",
                        pattern, fix
                    ),
                    ErrorCategory::SveltePattern,
                ));
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
                    return Err((
                        "Custom CSS not allowed. Use Tailwind classes only, or @apply directive.".to_string(),
                        ErrorCategory::Styling,
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
            return Err((
                "Unbalanced <script> tags".to_string(),
                ErrorCategory::SvelteCompiler,
            ));
        }

        // ============================================================
        // HTML Element Validation - disallow non-standard elements
        // ============================================================
        let template_content = Self::extract_template_content(content);

        // Match lowercase tags that look like HTML elements
        let element_regex = regex::Regex::new(r"<([a-z][a-z0-9]*)[^>]*[/]?>").unwrap();
        for cap in element_regex.captures_iter(&template_content) {
            if let Some(tag_match) = cap.get(1) {
                let tag_name = tag_match.as_str();

                // Allow standard HTML elements
                if STANDARD_HTML_ELEMENTS.contains(&tag_name) {
                    continue;
                }

                // Allow custom elements with hyphens (valid Web Components)
                if tag_name.contains('-') {
                    continue;
                }

                // Reject non-standard elements without hyphens
                // Use HtmlElement category to avoid triggering Svelte doc enrichment
                return Err((
                    format!(
                        "NON-STANDARD HTML ELEMENT: '<{}>' is not a valid HTML element. \
                         Did you mean to use a Svelte component? Use PascalCase: '<{}>' instead. \
                         Or for a custom element, add a hyphen: '<my-{}>' (Web Components require a hyphen).",
                        tag_name,
                        capitalize_first(tag_name),
                        tag_name
                    ),
                    ErrorCategory::HtmlElement,
                ));
            }
        }

        Ok(())
    }

    /// Validate imports based on the configured validation mode
    async fn validate_imports(&self, file_path: &PathBuf) -> Result<(), (String, ErrorCategory)> {
        let script_name = match self.sandbox_config.import_validation_mode {
            ImportValidationMode::None => return Ok(()),
            ImportValidationMode::ImportResolve => "validate-imports.mjs",
            ImportValidationMode::ViteIntegration => "validate-vite.mjs",
            ImportValidationMode::EsbuildBundle => "validate-esbuild.mjs",
        };

        let validation_script = self.project_root.join("scripts").join(script_name);

        // Prepare additional allowed packages as JSON
        let allowed_packages_json = if !self.sandbox_config.allowed_packages.is_empty() {
            serde_json::to_string(&self.sandbox_config.allowed_packages).unwrap_or_default()
        } else {
            String::new()
        };

        let mut cmd = tokio::process::Command::new("node");
        cmd.arg(&validation_script)
            .arg(file_path)
            .arg(&self.project_root);

        // Add allowed packages for import resolution mode
        if matches!(self.sandbox_config.import_validation_mode, ImportValidationMode::ImportResolve) && !allowed_packages_json.is_empty() {
            cmd.arg(&allowed_packages_json);
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.sandbox_config.validation_timeout_ms),
            cmd.output()
        ).await;

        match result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Parse JSON error output
                    if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let error_msg = error_json.get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown import validation error")
                            .to_string();

                        let line = error_json.get("line")
                            .and_then(|l| l.as_u64());

                        let mut full_error = format!("IMPORT VALIDATION ERROR: {}", error_msg);
                        if let Some(line_num) = line {
                            full_error.push_str(&format!(" (line {})", line_num));
                        }

                        // Add suggestions for common mistakes
                        full_error.push_str("\n\nCommon fixes:\n");
                        full_error.push_str("- Check package name spelling (e.g., 'lucide-svelte' not 'lucid')\n");
                        full_error.push_str("- Ensure the package is in package.json dependencies\n");
                        full_error.push_str("- Use relative paths for local components (./Component.svelte)");

                        return Err((full_error, ErrorCategory::ImportResolution));
                    } else {
                        return Err((
                            format!("IMPORT VALIDATION ERROR: {}", stdout.trim()),
                            ErrorCategory::ImportResolution,
                        ));
                    }
                }
                Ok(())
            }
            Ok(Err(e)) => {
                // Script failed to run - log but don't block
                log::warn!("Import validation script failed to run: {}. Skipping import validation.", e);
                Ok(())
            }
            Err(_) => {
                // Timeout
                Err((
                    format!(
                        "IMPORT VALIDATION ERROR: Validation timed out after {}ms. \
                         This may indicate a complex import graph or slow disk I/O.",
                        self.sandbox_config.validation_timeout_ms
                    ),
                    ErrorCategory::ImportResolution,
                ))
            }
        }
    }

    /// Validate code quality using ESLint (if enabled)
    async fn validate_lint(&self, file_path: &PathBuf) -> Result<(), (String, ErrorCategory)> {
        if !self.sandbox_config.lint_enabled {
            return Ok(());
        }

        let lint_script = self.project_root.join("scripts").join("validate-lint.mjs");

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.sandbox_config.validation_timeout_ms),
            tokio::process::Command::new("node")
                .arg(&lint_script)
                .arg(file_path)
                .arg(&self.project_root)
                .output()
        ).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if !output.status.success() {
                    // Parse JSON error output
                    if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let error_msg = error_json.get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown linting error")
                            .to_string();

                        let line = error_json.get("line")
                            .and_then(|l| l.as_u64());

                        let mut full_error = format!("LINTING ERROR: {}", error_msg);
                        if let Some(line_num) = line {
                            full_error.push_str(&format!(" (line {})", line_num));
                        }

                        full_error.push_str("\n\nCommon fixes:\n");
                        full_error.push_str("- Don't use `undefined` explicitly - use `null` or omit initialization\n");
                        full_error.push_str("- Remove unused variables\n");
                        full_error.push_str("- Check for accidental type coercion");

                        return Err((full_error, ErrorCategory::Linting));
                    } else {
                        return Err((
                            format!("LINTING ERROR: {}", stdout.trim()),
                            ErrorCategory::Linting,
                        ));
                    }
                }
                Ok(())
            }
            Ok(Err(e)) => {
                // Script failed to run - log but don't block
                log::warn!("Lint validation script failed to run: {}. Skipping lint validation.", e);
                Ok(())
            }
            Err(_) => {
                // Timeout - log warning but don't block
                log::warn!("Lint validation timed out. Skipping lint validation.");
                Ok(())
            }
        }
    }

    /// Validate that template expressions don't contain JSX syntax
    /// This catches React-style patterns like {condition && <element>} that would
    /// cause cryptic "Unexpected token" errors from the Svelte compiler.
    async fn validate_jsx_in_template(&self, file_path: &PathBuf) -> Result<(), (String, ErrorCategory)> {
        let validation_script = self.project_root.join("scripts").join("validate-jsx-in-template.mjs");

        // Skip if validation script doesn't exist
        if !validation_script.exists() {
            log::debug!("JSX validation script not found, skipping JSX validation");
            return Ok(());
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.sandbox_config.validation_timeout_ms),
            tokio::process::Command::new("node")
                .arg(&validation_script)
                .arg(file_path)
                .output()
        ).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if !output.status.success() {
                    // Parse JSON error output
                    if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let error_msg = error_json.get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Found JSX syntax in template")
                            .to_string();

                        // Use SveltePattern category so it triggers the enricher
                        return Err((error_msg, ErrorCategory::SveltePattern));
                    } else {
                        return Err((
                            format!("JSX SYNTAX ERROR: {}", stdout.trim()),
                            ErrorCategory::SveltePattern,
                        ));
                    }
                }
                Ok(())
            }
            Ok(Err(e)) => {
                // Script failed to run - log but don't block
                log::warn!("JSX validation script failed to run: {}. Skipping JSX validation.", e);
                Ok(())
            }
            Err(_) => {
                // Timeout - log warning but don't block
                log::warn!("JSX validation timed out. Skipping JSX validation.");
                Ok(())
            }
        }
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
        if let Err((msg, category)) = self.validate_svelte_content(&args.content) {
            return Err(self.validation_error(msg, category).await);
        }

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

        // Step 2.5: Check for JSX patterns in template (before Svelte compiler)
        // This provides helpful error messages instead of cryptic "Unexpected token" errors
        if let Err((msg, category)) = self.validate_jsx_in_template(&temp_path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(self.validation_error(msg, category).await);
        }

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
                    let (error_msg, line) = if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let msg = error_json.get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown compilation error")
                            .to_string();
                        let line_num = error_json.get("line")
                            .and_then(|l| l.as_u64());
                        (msg, line_num)
                    } else {
                        (stdout.trim().to_string(), None)
                    };

                    let mut full_error = format!(
                        "SVELTE COMPILATION ERROR: {}",
                        error_msg
                    );
                    if let Some(line_num) = line {
                        full_error.push_str(&format!(" (line {})", line_num));
                    }
                    full_error.push_str(". Please fix the syntax and try again.");

                    // Use enricher pipeline to add relevant documentation
                    return Err(self.validation_error(full_error, ErrorCategory::SvelteCompiler).await);
                }

                // Step 3.5: Import validation based on sandbox config
                // This catches imports that won't resolve at bundle time
                if let Err((msg, category)) = self.validate_imports(&temp_path).await {
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    return Err(self.validation_error(msg, category).await);
                }

                // Step 3.6: ESLint validation (if enabled)
                // This catches code quality issues like explicit undefined usage, unused variables, etc.
                if let Err((msg, category)) = self.validate_lint(&temp_path).await {
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    return Err(self.validation_error(msg, category).await);
                }

                // Step 4: Runtime semantic validation using rustyscript sandbox
                // This catches errors that pass syntax validation but would fail at runtime
                // (e.g., using primitive values as components)
                match validate_runtime_semantics(&args.content, 5000) {
                    Ok(()) => {
                        // All validations passed
                    }
                    Err(crate::hotload_sandbox::runtime_sandbox::RuntimeValidationError::Timeout) => {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        let msg = "RUNTIME VALIDATION ERROR: Code execution timed out after 5000ms. \
                             This may indicate an infinite loop in your script. \
                             Please check for while(true), for(;;), or recursive calls without exit conditions.".to_string();
                        return Err(self.validation_error(msg, ErrorCategory::RuntimeSemantic).await);
                    }
                    Err(crate::hotload_sandbox::runtime_sandbox::RuntimeValidationError::SemanticError { message, line }) => {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        let mut error_msg = format!("RUNTIME SEMANTIC ERROR: {}", message);
                        if let Some(line_num) = line {
                            error_msg.push_str(&format!(" (around line {})", line_num));
                        }
                        error_msg.push_str(
                            "\n\nThis error occurs because the code passes syntax validation \
                             but would fail when actually rendered. Common causes:\n\
                             - Using a string/number variable as a component (<MyVar /> where MyVar = \"text\")\n\
                             - Using undefined variables in the template\n\
                             - Components must be imported Svelte components, not primitive values"
                        );
                        return Err(self.validation_error(error_msg, ErrorCategory::RuntimeSemantic).await);
                    }
                    Err(e) => {
                        // Other runtime errors - log but don't block (might be false positive)
                        log::debug!("Runtime validation warning (non-blocking): {}", e);
                    }
                }

                // Compilation and runtime validation succeeded - move temp file to final location
                tokio::fs::rename(&temp_path, &full_path).await.map_err(ToolError::Io)?;

                // Record successful write
                if let Some(ref tracker) = self.write_tracker {
                    if let Ok(mut writes) = tracker.lock() {
                        writes.push(sanitized.clone());
                    }
                }

                Ok(true)
            }
            Err(e) => {
                // Node command failed to run - fall back to allowing the file
                // (validation script might not be available)
                log::warn!("Svelte validation script failed to run: {}. Proceeding without compiler validation.", e);
                let _ = tokio::fs::remove_file(&temp_path).await;
                tokio::fs::write(&full_path, &args.content).await.map_err(ToolError::Io)?;

                // Record successful write
                if let Some(ref tracker) = self.write_tracker {
                    if let Ok(mut writes) = tracker.lock() {
                        writes.push(sanitized.clone());
                    }
                }

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

// ============================================================================
// SearchSvelteDocsTool - Search Svelte 5 documentation (kept for programmatic use, not registered with agent)
// ============================================================================

use super::docs::DocsManager;
use super::docs_search::{search_docs, DocSearchOutput};

#[derive(Debug, Deserialize)]
pub struct SearchSvelteDocsArgs {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    3
}

#[derive(Clone)]
pub struct SearchSvelteDocsTool {
    docs_manager: Arc<DocsManager>,
}

impl SearchSvelteDocsTool {
    pub fn new(docs_manager: Arc<DocsManager>) -> Self {
        Self { docs_manager }
    }
}

impl Tool for SearchSvelteDocsTool {
    const NAME: &'static str = "search_svelte_docs";
    type Error = ToolError;
    type Args = SearchSvelteDocsArgs;
    type Output = DocSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search Svelte 5 documentation for syntax, APIs, and best practices. \
                          Use this when you need to verify Svelte 5 runes syntax ($state, $derived, $effect, $props), \
                          event handlers (onclick, onmouseenter), component patterns, or fix errors.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (e.g., '$state', 'event handlers', 'props', 'onclick')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 3)",
                        "default": 3
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        log::debug!("[search_svelte_docs] Searching for: {}", args.query);

        // Check if docs are available (does not auto-download)
        if let Err(e) = self.docs_manager.ensure_docs_available().await {
            // Return empty results with a note instead of failing
            // This allows the agent to continue without docs
            log::warn!("[search_svelte_docs] Docs not available: {}", e);
            return Ok(DocSearchOutput {
                query: args.query,
                total_matches: 0,
                results: vec![],
            });
        }

        // Load search index
        let index = match self.docs_manager.load_index() {
            Ok(idx) => idx,
            Err(e) => {
                log::warn!("[search_svelte_docs] Failed to load index: {}", e);
                return Ok(DocSearchOutput {
                    query: args.query,
                    total_matches: 0,
                    results: vec![],
                });
            }
        };

        log::debug!(
            "[search_svelte_docs] Loaded index with {} entries, searching...",
            index.entries.len()
        );

        // Perform fuzzy search
        let results = search_docs(&index, &args.query, args.limit);

        log::debug!(
            "[search_svelte_docs] Found {} results for query '{}'",
            results.len(),
            args.query
        );

        Ok(DocSearchOutput {
            query: args.query,
            total_matches: results.len(),
            results,
        })
    }
}

// ============================================================================
// SearchSvelteDocsVectorTool - Semantic search using LanceDB vectors
// ============================================================================

use super::types::DocChunk;

/// Output structure for the vector search tool
#[derive(Debug, Serialize)]
pub struct VectorDocSearchOutput {
    /// The original query
    pub query: String,
    /// Search results ordered by relevance
    pub results: Vec<VectorDocResult>,
    /// Total number of matches found
    pub total_matches: usize,
}

/// A vector search result with chunk details
#[derive(Debug, Serialize)]
pub struct VectorDocResult {
    /// Document title
    pub doc_title: String,
    /// Chunk/section title
    pub title: String,
    /// Section name
    pub section: String,
    /// Header context (breadcrumb path)
    pub header_context: String,
    /// Full content of the chunk
    pub content: String,
    /// Whether this chunk contains code examples
    pub has_code: bool,
}

impl From<DocChunk> for VectorDocResult {
    fn from(chunk: DocChunk) -> Self {
        Self {
            doc_title: chunk.doc_title,
            title: chunk.title,
            section: chunk.section,
            header_context: chunk.header_context,
            content: chunk.content,
            has_code: chunk.has_code,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchSvelteDocsVectorArgs {
    pub query: String,
    #[serde(default = "default_vector_search_limit")]
    pub limit: usize,
}

fn default_vector_search_limit() -> usize {
    3
}

#[derive(Clone)]
pub struct SearchSvelteDocsVectorTool {
    rag_manager: SharedRagManager,
}

impl SearchSvelteDocsVectorTool {
    pub fn new(rag_manager: SharedRagManager) -> Self {
        Self { rag_manager }
    }
}

impl Tool for SearchSvelteDocsVectorTool {
    const NAME: &'static str = "search_svelte_docs_vector";
    type Error = ToolError;
    type Args = SearchSvelteDocsVectorArgs;
    type Output = VectorDocSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Semantic search Svelte 5 documentation using vector embeddings. \
                          More accurate than keyword search for finding conceptually related content. \
                          Use this when you need to understand Svelte 5 concepts, fix errors, or find related documentation. \
                          Requires documentation to be indexed first.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing what you're looking for (e.g., 'how to declare reactive props', 'event handler syntax')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 3)",
                        "default": 3
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let rag_guard = self.rag_manager.read().await;

        // Perform semantic vector search
        let chunks = rag_guard
            .search(&args.query, args.limit)
            .await
            .map_err(|e| ToolError::Validation(format!("Vector search failed: {}. Make sure documentation is indexed.", e)))?;

        let results: Vec<VectorDocResult> = chunks.into_iter().map(VectorDocResult::from).collect();
        let total = results.len();

        Ok(VectorDocSearchOutput {
            query: args.query,
            results,
            total_matches: total,
        })
    }
}

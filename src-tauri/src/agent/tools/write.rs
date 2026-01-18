use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use super::error::ToolError;
use super::validation::{capitalize_first, MATHML_ELEMENTS, STANDARD_HTML_ELEMENTS, SVG_ELEMENTS};
use crate::agent::enricher::{EnricherRegistry, ErrorCategory};
use crate::agent::types::WriteTracker;
use crate::config::{ImportValidationMode, SandboxConfig};
use crate::hotload_sandbox::runtime_sandbox::validate_runtime_semantics;
use crate::llm::commands::version::update_tracking_after_commit;

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

                // Allow standard HTML elements, SVG elements, and MathML elements
                if STANDARD_HTML_ELEMENTS.contains(&tag_name)
                    || SVG_ELEMENTS.contains(&tag_name)
                    || MATHML_ELEMENTS.contains(&tag_name)
                {
                    continue;
                }

                // Allow custom elements with hyphens (valid Web Components)
                if tag_name.contains('-') {
                    continue;
                }

                // Reject non-standard elements without hyphens
                // Use SveltePattern to trigger doc enrichment - semantic search will
                // return relevant docs (e.g., svelte:component for dynamic components)
                return Err((
                    format!(
                        "NON-STANDARD HTML ELEMENT: '<{}>' is not a valid HTML element. \
                         Did you mean to use a Svelte component? Use PascalCase: '<{}>' instead. \
                         Or for a custom element, add a hyphen: '<my-{}>' (Web Components require a hyphen).",
                        tag_name,
                        capitalize_first(tag_name),
                        tag_name
                    ),
                    ErrorCategory::SveltePattern,
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

                        // Extract suggestions from the JSON response
                        let suggestions: Vec<String> = error_json.get("errors")
                            .and_then(|e| e.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|e| e.get("suggestions"))
                            .and_then(|s| s.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect())
                            .unwrap_or_default();

                        let mut full_error = format!("IMPORT VALIDATION ERROR: {}", error_msg);
                        if let Some(line_num) = line {
                            full_error.push_str(&format!(" (line {})", line_num));
                        }

                        // Use dynamic suggestions if available, otherwise show generic help
                        if !suggestions.is_empty() {
                            full_error.push_str(&format!("\n\nDid you mean: {}?",
                                suggestions.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")));
                        }
                        full_error.push_str("\n\nEnsure the package is listed in package.json dependencies.");

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

    /// Validate design system compliance (advisory - returns warnings, not errors)
    /// This checks for non-design-system colors, emoji usage, etc.
    async fn validate_design_system(&self, file_path: &PathBuf) -> Vec<String> {
        let validation_script = self.project_root.join("scripts").join("validate-design-system.mjs");

        // Skip if validation script doesn't exist
        if !validation_script.exists() {
            log::debug!("Design system validation script not found, skipping");
            return vec![];
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

                if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(warnings) = result_json.get("warnings").and_then(|w| w.as_array()) {
                        return warnings
                            .iter()
                            .filter_map(|w| w.as_str().map(String::from))
                            .collect();
                    }
                }
                vec![]
            }
            Ok(Err(e)) => {
                log::debug!("Design system validation script failed: {}", e);
                vec![]
            }
            Err(_) => {
                log::debug!("Design system validation timed out");
                vec![]
            }
        }
    }

    /// Initialize git repo in generated folder if not exists (for versioning)
    fn ensure_git_repo(&self) -> Result<(), std::io::Error> {
        let generated_dir = self.get_generated_path();
        let git_dir = generated_dir.join(".git");

        if !git_dir.exists() {
            // Create the generated directory if it doesn't exist
            std::fs::create_dir_all(&generated_dir)?;

            // Initialize git repo
            let output = std::process::Command::new("git")
                .args(["init"])
                .current_dir(&generated_dir)
                .output()?;

            if output.status.success() {
                log::info!("[write_gui_file] Initialized git repo in src/generated/");

                // Create a .gitignore inside the generated folder
                let gitignore_path = generated_dir.join(".gitignore");
                if !gitignore_path.exists() {
                    std::fs::write(&gitignore_path, "# Temporary validation files\n*.tmp\n")?;
                }

                // Initial commit
                let _ = std::process::Command::new("git")
                    .args(["add", "."])
                    .current_dir(&generated_dir)
                    .output();
                let _ = std::process::Command::new("git")
                    .args(["commit", "-m", "Initialize generated components", "--allow-empty"])
                    .current_dir(&generated_dir)
                    .output();
            } else {
                log::warn!("[write_gui_file] Failed to initialize git repo: {:?}", String::from_utf8_lossy(&output.stderr));
            }
        }

        Ok(())
    }

    /// Commit the file change to git (for undo/redo support)
    fn commit_change(&self, path: &str, is_new: bool) {
        let generated_dir = self.get_generated_path();

        // Ensure git repo exists
        if let Err(e) = self.ensure_git_repo() {
            log::warn!("[write_gui_file] Failed to ensure git repo: {}", e);
            return;
        }

        // Stage the file
        let stage_result = std::process::Command::new("git")
            .args(["add", path])
            .current_dir(&generated_dir)
            .output();

        if let Err(e) = stage_result {
            log::warn!("[write_gui_file] Failed to stage file: {}", e);
            return;
        }

        // Commit with descriptive message
        let action = if is_new { "Create" } else { "Update" };
        let message = format!("{} {}", action, path);

        let commit_result = std::process::Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(&generated_dir)
            .output();

        match commit_result {
            Ok(output) if output.status.success() => {
                log::info!("[write_gui_file] Git committed: {}", message);
                // Update tracking files for non-destructive undo/redo
                update_tracking_after_commit(&generated_dir);
            }
            Ok(output) => {
                // Non-zero exit but not an error (e.g., nothing to commit)
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::debug!("[write_gui_file] Git commit notice: {}", stderr);
            }
            Err(e) => {
                log::warn!("[write_gui_file] Failed to commit: {}", e);
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

                // Step 5: Design system validation (advisory - logs warnings but doesn't block)
                let design_warnings = self.validate_design_system(&temp_path).await;
                if !design_warnings.is_empty() {
                    log::info!(
                        "[write_gui_file] Design system warnings for {}: {:?}",
                        sanitized,
                        design_warnings
                    );
                }

                // Check if this is a new file or an update (for git commit message)
                let is_new_file = !full_path.exists();

                // Compilation and runtime validation succeeded - move temp file to final location
                tokio::fs::rename(&temp_path, &full_path).await.map_err(ToolError::Io)?;

                // Git versioning - commit the change for undo/redo support
                self.commit_change(&sanitized, is_new_file);

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

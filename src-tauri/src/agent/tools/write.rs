use node_engine::resolve_path_within_root;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[path = "write_validation.rs"]
mod write_validation;
#[path = "write_versioning.rs"]
mod write_versioning;

use super::error::ToolError;
use crate::agent::enricher::{EnricherRegistry, ErrorCategory};
use crate::agent::types::WriteTracker;
use crate::config::SandboxConfig;
use crate::hotload_sandbox::runtime_sandbox::validate_runtime_semantics;

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
    pub fn with_tracker(
        project_root: PathBuf,
        tracker: WriteTracker,
        enricher_registry: Arc<EnricherRegistry>,
    ) -> Self {
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

    fn validate_svelte_content(&self, content: &str) -> Result<(), (String, ErrorCategory)> {
        write_validation::validate_svelte_content(content)
    }

    /// Validate imports based on the configured validation mode
    async fn validate_imports(&self, file_path: &PathBuf) -> Result<(), (String, ErrorCategory)> {
        write_validation::validate_imports(&self.project_root, &self.sandbox_config, file_path)
            .await
    }

    /// Validate code quality using ESLint (if enabled)
    async fn validate_lint(&self, file_path: &PathBuf) -> Result<(), (String, ErrorCategory)> {
        write_validation::validate_lint(&self.project_root, &self.sandbox_config, file_path).await
    }

    /// Validate design system compliance (advisory - returns warnings, not errors)
    /// This checks for non-design-system colors, emoji usage, etc.
    async fn validate_design_system(&self, file_path: &PathBuf) -> Vec<String> {
        write_validation::validate_design_system(
            &self.project_root,
            &self.sandbox_config,
            file_path,
        )
        .await
    }

    /// Commit the file change to git (for undo/redo support)
    fn commit_change(&self, path: &str, is_new: bool) {
        write_versioning::commit_change(&self.get_generated_path(), path, is_new);
    }

    /// Validate that template expressions don't contain JSX syntax
    /// This catches React-style patterns like {condition && <element>} that would
    /// cause cryptic "Unexpected token" errors from the Svelte compiler.
    async fn validate_jsx_in_template(
        &self,
        file_path: &PathBuf,
    ) -> Result<(), (String, ErrorCategory)> {
        write_validation::validate_jsx_in_template(
            &self.project_root,
            &self.sandbox_config,
            file_path,
        )
        .await
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

        let generated_root = self.get_generated_path();
        tokio::fs::create_dir_all(&generated_root)
            .await
            .map_err(ToolError::Io)?;
        let full_path = resolve_path_within_root(&args.path, &generated_root)
            .map_err(|e| ToolError::PathNotAllowed(format!("{} ({})", args.path, e)))?;
        let generated_root_canonical = generated_root.canonicalize().map_err(ToolError::Io)?;
        let relative_path = full_path
            .strip_prefix(&generated_root_canonical)
            .map_err(|_| ToolError::PathNotAllowed(args.path.clone()))?;
        if relative_path.as_os_str().is_empty() {
            return Err(ToolError::PathNotAllowed(args.path));
        }
        let sanitized = relative_path.to_string_lossy().replace('\\', "/");

        // Ensure the generated directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(ToolError::Io)?;
        }

        // Step 2: Write to a temp file first for validation
        let temp_path = full_path.with_extension("svelte.tmp");
        tokio::fs::write(&temp_path, &args.content)
            .await
            .map_err(ToolError::Io)?;

        // Step 2.5: Check for JSX patterns in template (before Svelte compiler)
        // This provides helpful error messages instead of cryptic "Unexpected token" errors
        if let Err((msg, category)) = self.validate_jsx_in_template(&temp_path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(self.validation_error(msg, category).await);
        }

        // Step 3: Validate with Svelte compiler
        let validation_script = self
            .project_root
            .join("scripts")
            .join("validate-svelte.mjs");
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
                    let (error_msg, line) = if let Ok(error_json) =
                        serde_json::from_str::<serde_json::Value>(&stdout)
                    {
                        let msg = error_json
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown compilation error")
                            .to_string();
                        let line_num = error_json.get("line").and_then(|l| l.as_u64());
                        (msg, line_num)
                    } else {
                        (stdout.trim().to_string(), None)
                    };

                    let mut full_error = format!("SVELTE COMPILATION ERROR: {}", error_msg);
                    if let Some(line_num) = line {
                        full_error.push_str(&format!(" (line {})", line_num));
                    }
                    full_error.push_str(". Please fix the syntax and try again.");

                    // Use enricher pipeline to add relevant documentation
                    return Err(self
                        .validation_error(full_error, ErrorCategory::SvelteCompiler)
                        .await);
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
                tokio::fs::rename(&temp_path, &full_path)
                    .await
                    .map_err(ToolError::Io)?;

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
                tokio::fs::write(&full_path, &args.content)
                    .await
                    .map_err(ToolError::Io)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::WriteTracker;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn make_temp_project_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("pantograph-write-tool-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[tokio::test]
    async fn test_write_gui_file_rejects_parent_traversal() {
        let project_root = make_temp_project_root();
        let tracker: WriteTracker = Arc::new(Mutex::new(Vec::new()));
        let enricher_registry = Arc::new(EnricherRegistry::new());
        let tool = WriteGuiFileTool::with_tracker(project_root.clone(), tracker, enricher_registry);

        let err = tool
            .call(WriteGuiFileArgs {
                path: "../escape.svelte".to_string(),
                content: "<div>ok</div>".to_string(),
            })
            .await
            .expect_err("must reject traversal path");

        assert!(matches!(err, ToolError::PathNotAllowed(_)));
        let _ = fs::remove_dir_all(project_root);
    }
}

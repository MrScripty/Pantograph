//! Validator Task
//!
//! Validates Svelte component source code using multiple validation stages:
//! 1. Pattern validation (Svelte 5 syntax)
//! 2. Compilation validation (via external script)
//! 3. Runtime semantic validation (via external executor)
//!
//! This task wraps the validation logic to be usable in workflow graphs.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Validation result details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Error message if validation failed
    pub error: Option<String>,
    /// Error category for enrichment
    pub category: Option<String>,
    /// Line number if available
    pub line: Option<u32>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            error: None,
            category: None,
            line: None,
        }
    }
}

/// Configuration for the validator task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Timeout for validation in milliseconds
    pub timeout_ms: u64,
    /// Whether to validate Svelte 5 patterns
    pub check_patterns: bool,
    /// Whether to run runtime semantic validation
    pub check_runtime: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            check_patterns: true,
            check_runtime: true,
        }
    }
}

/// Validator Task
///
/// Validates Svelte component source code for correctness.
/// This task performs pattern-based validation to catch common
/// Svelte 5 syntax errors.
///
/// Note: Full compilation and runtime validation requires the
/// task executor to have access to the validation infrastructure
/// (Node.js scripts, boa_engine sandbox). This task performs
/// pattern-based checks that can be done in pure Rust.
///
/// # Inputs (from context)
/// - `{task_id}.input.code` (required) - Svelte component source code
/// - `{task_id}.input.timeout_ms` (optional) - Validation timeout
///
/// # Outputs (to context)
/// - `{task_id}.output.valid` - Boolean indicating validation passed
/// - `{task_id}.output.error` - Error message if validation failed
/// - `{task_id}.output.category` - Error category (SveltePattern/SvelteCompiler/RuntimeSemantic)
#[derive(Clone)]
pub struct ValidatorTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration
    config: Option<ValidatorConfig>,
}

impl ValidatorTask {
    /// Port ID for code input
    pub const PORT_CODE: &'static str = "code";
    /// Port ID for timeout input
    pub const PORT_TIMEOUT_MS: &'static str = "timeout_ms";
    /// Port ID for valid output
    pub const PORT_VALID: &'static str = "valid";
    /// Port ID for error output
    pub const PORT_ERROR: &'static str = "error";
    /// Port ID for category output
    pub const PORT_CATEGORY: &'static str = "category";

    /// Create a new validator task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: ValidatorConfig) -> Self {
        Self {
            task_id: task_id.into(),
            config: Some(config),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Validate Svelte content for common pattern errors.
    /// This catches Svelte 5 runes mode issues like `export let` and `on:click`.
    fn validate_patterns(code: &str) -> Result<(), (String, &'static str)> {
        // Strip comments before validation to avoid false positives
        let code_no_comments: String = code
            .lines()
            .map(|line| {
                if let Some(idx) = line.find("//") {
                    &line[..idx]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Forbidden patterns for Svelte 5 runes mode
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
            if code_no_comments.contains(pattern) {
                return Err((
                    format!(
                        "SVELTE 5 SYNTAX ERROR: Found forbidden pattern '{}'. {}. \
                         Svelte 5 uses runes mode - you MUST use $props() for props and \
                         lowercase event handlers (onclick, onchange, etc.).",
                        pattern, fix
                    ),
                    "SveltePattern",
                ));
            }
        }

        // Check for unbalanced script tags
        let script_opens = code.matches("<script").count();
        let script_closes = code.matches("</script>").count();
        if script_opens != script_closes {
            return Err((
                "Unbalanced <script> tags".to_string(),
                "SvelteCompiler",
            ));
        }

        Ok(())
    }
}

impl TaskDescriptor for ValidatorTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "validator".to_string(),
            category: NodeCategory::Processing,
            label: "Validator".to_string(),
            description: "Validates Svelte component code for correctness".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_CODE, "Code", PortDataType::String),
                PortMetadata::optional(Self::PORT_TIMEOUT_MS, "Timeout (ms)", PortDataType::Number),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_VALID, "Valid", PortDataType::Boolean),
                PortMetadata::optional(Self::PORT_ERROR, "Error", PortDataType::String),
                PortMetadata::optional(Self::PORT_CATEGORY, "Category", PortDataType::String),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ValidatorTask::descriptor));

#[async_trait]
impl Task for ValidatorTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: code
        let code_key = ContextKeys::input(&self.task_id, Self::PORT_CODE);
        let code: String = context.get(&code_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'code' at key '{}'",
                code_key
            ))
        })?;

        // Get configuration
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<ValidatorConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        log::debug!(
            "ValidatorTask {}: validating {} chars of code",
            self.task_id,
            code.len()
        );

        // Perform pattern validation
        let validation_result = if config.check_patterns {
            match Self::validate_patterns(&code) {
                Ok(()) => ValidationResult {
                    valid: true,
                    error: None,
                    category: None,
                    line: None,
                },
                Err((error, category)) => ValidationResult {
                    valid: false,
                    error: Some(error),
                    category: Some(category.to_string()),
                    line: None,
                },
            }
        } else {
            ValidationResult::default()
        };

        // Store outputs in context
        let valid_key = ContextKeys::output(&self.task_id, Self::PORT_VALID);
        context.set(&valid_key, validation_result.valid).await;

        let error_key = ContextKeys::output(&self.task_id, Self::PORT_ERROR);
        context
            .set(&error_key, validation_result.error.clone().unwrap_or_default())
            .await;

        let category_key = ContextKeys::output(&self.task_id, Self::PORT_CATEGORY);
        context
            .set(&category_key, validation_result.category.clone().unwrap_or_default())
            .await;

        log::debug!(
            "ValidatorTask {}: validation complete, valid={}",
            self.task_id,
            validation_result.valid
        );

        Ok(TaskResult::new(
            Some(serde_json::to_string(&validation_result).unwrap_or_default()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ValidatorTask::new("my_validator");
        assert_eq!(task.id(), "my_validator");
    }

    #[test]
    fn test_with_config() {
        let config = ValidatorConfig {
            timeout_ms: 10000,
            check_patterns: false,
            check_runtime: true,
        };
        let task = ValidatorTask::with_config("task1", config);
        assert_eq!(task.config.as_ref().unwrap().timeout_ms, 10000);
        assert!(!task.config.as_ref().unwrap().check_patterns);
    }

    #[test]
    fn test_default_config() {
        let config = ValidatorConfig::default();
        assert_eq!(config.timeout_ms, 5000);
        assert!(config.check_patterns);
        assert!(config.check_runtime);
    }

    #[test]
    fn test_descriptor() {
        let meta = ValidatorTask::descriptor();
        assert_eq!(meta.node_type, "validator");
        assert_eq!(meta.category, NodeCategory::Processing);
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 3);
    }

    #[test]
    fn test_validate_patterns_valid() {
        let code = r#"
<script>
  let { count } = $props();
</script>
<button onclick={() => count++}>
  Count: {count}
</button>
"#;
        let result = ValidatorTask::validate_patterns(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_patterns_export_let() {
        let code = r#"
<script>
  export let count = 0;
</script>
<div>{count}</div>
"#;
        let result = ValidatorTask::validate_patterns(code);
        assert!(result.is_err());
        let (error, category) = result.unwrap_err();
        assert!(error.contains("export let"));
        assert_eq!(category, "SveltePattern");
    }

    #[test]
    fn test_validate_patterns_on_click() {
        let code = r#"
<script>
  let count = $state(0);
</script>
<button on:click={() => count++}>
  Count: {count}
</button>
"#;
        let result = ValidatorTask::validate_patterns(code);
        assert!(result.is_err());
        let (error, category) = result.unwrap_err();
        assert!(error.contains("on:click"));
        assert_eq!(category, "SveltePattern");
    }

    #[test]
    fn test_validate_patterns_unbalanced_script() {
        let code = r#"
<script>
  let count = 0;
<div>{count}</div>
"#;
        let result = ValidatorTask::validate_patterns(code);
        assert!(result.is_err());
        let (error, category) = result.unwrap_err();
        assert!(error.contains("Unbalanced"));
        assert_eq!(category, "SvelteCompiler");
    }

    #[tokio::test]
    async fn test_validate_valid_code() {
        let task = ValidatorTask::new("test_validator");
        let context = Context::new();

        let valid_code = r#"
<script>
  let { name } = $props();
</script>
<div>Hello, {name}!</div>
"#;

        let code_key = ContextKeys::input("test_validator", "code");
        context.set(&code_key, valid_code.to_string()).await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let valid_key = ContextKeys::output("test_validator", "valid");
        let valid: Option<bool> = context.get(&valid_key).await;
        assert_eq!(valid, Some(true));
    }

    #[tokio::test]
    async fn test_validate_invalid_code() {
        let task = ValidatorTask::new("test_validator");
        let context = Context::new();

        let invalid_code = r#"
<script>
  export let name = "world";
</script>
<div on:click={() => {}}>Hello, {name}!</div>
"#;

        let code_key = ContextKeys::input("test_validator", "code");
        context.set(&code_key, invalid_code.to_string()).await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let valid_key = ContextKeys::output("test_validator", "valid");
        let valid: Option<bool> = context.get(&valid_key).await;
        assert_eq!(valid, Some(false));

        let error_key = ContextKeys::output("test_validator", "error");
        let error: Option<String> = context.get(&error_key).await;
        assert!(error.is_some());
        assert!(error.unwrap().contains("export let"));
    }

    #[tokio::test]
    async fn test_missing_code_error() {
        let task = ValidatorTask::new("test_validator");
        let context = Context::new();

        // Run without setting code - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}

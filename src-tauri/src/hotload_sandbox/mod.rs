//! Hotload Sandbox Module
//!
//! Provides sandboxed validation of Svelte components before they are written to disk.
//! Uses two-stage validation:
//! 1. Svelte syntax validation via Node.js (existing approach)
//! 2. Runtime semantic validation via rustyscript sandbox
//!
//! This module catches errors that pass syntax validation but would fail at runtime,
//! such as using primitive values as components.

pub mod runtime_sandbox;
pub mod svelte_validator;

pub use runtime_sandbox::{validate_runtime_semantics, RuntimeValidationError};
pub use svelte_validator::{validate_svelte_syntax, SvelteValidationError, SvelteValidationResult};

use thiserror::Error;

/// Combined validation error type
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Svelte syntax error: {0}")]
    Syntax(#[from] SvelteValidationError),

    #[error("Runtime validation error: {0}")]
    Runtime(#[from] RuntimeValidationError),
}

/// Result of full validation
#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub error_line: Option<u64>,
    pub error_type: Option<ValidationErrorType>,
    pub documentation_hint: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ValidationErrorType {
    Syntax,
    Runtime,
    Timeout,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            error: None,
            error_line: None,
            error_type: None,
            documentation_hint: None,
        }
    }

    pub fn failure(error: String, error_type: ValidationErrorType) -> Self {
        Self {
            valid: false,
            error: Some(error),
            error_line: None,
            error_type: Some(error_type),
            documentation_hint: None,
        }
    }

    pub fn with_line(mut self, line: u64) -> Self {
        self.error_line = Some(line);
        self
    }

    pub fn with_hint(mut self, hint: String) -> Self {
        self.documentation_hint = Some(hint);
        self
    }
}

/// Perform full validation of Svelte component source code.
///
/// This runs both syntax validation (via Node.js/Svelte compiler) and
/// runtime semantic validation (via rustyscript sandbox).
///
/// # Arguments
/// * `content` - The Svelte component source code
/// * `project_root` - Path to project root (for Node.js script)
/// * `timeout_ms` - Timeout for runtime validation in milliseconds
///
/// # Returns
/// * `Ok(ValidationResult)` - Validation completed (check `valid` field)
/// * `Err(ValidationError)` - Validation process itself failed
pub async fn validate_component(
    content: &str,
    project_root: &std::path::Path,
    timeout_ms: u64,
) -> Result<ValidationResult, ValidationError> {
    // Stage 1: Svelte syntax validation
    match validate_svelte_syntax(content, project_root).await {
        Ok(result) if !result.valid => {
            return Ok(ValidationResult {
                valid: false,
                error: result.error,
                error_line: result.line,
                error_type: Some(ValidationErrorType::Syntax),
                documentation_hint: None,
            });
        }
        Err(e) => {
            // Node.js validation failed to run - log warning but continue
            log::warn!("Svelte syntax validation failed to run: {}. Continuing with runtime validation.", e);
        }
        Ok(_) => {
            // Syntax validation passed
        }
    }

    // Stage 2: Runtime semantic validation
    match validate_runtime_semantics(content, timeout_ms) {
        Ok(()) => Ok(ValidationResult::success()),
        Err(RuntimeValidationError::Timeout) => Ok(ValidationResult::failure(
            format!("Code execution timed out after {}ms. This may indicate an infinite loop.", timeout_ms),
            ValidationErrorType::Timeout,
        )),
        Err(RuntimeValidationError::SemanticError { message, line }) => {
            let mut result = ValidationResult::failure(message, ValidationErrorType::Runtime);
            if let Some(l) = line {
                result = result.with_line(l);
            }
            Ok(result)
        }
        Err(e) => Ok(ValidationResult::failure(e.to_string(), ValidationErrorType::Runtime)),
    }
}

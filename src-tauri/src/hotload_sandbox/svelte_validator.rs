//! Svelte Syntax Validator
//!
//! Uses Node.js and the Svelte compiler to validate component syntax.
//! This is the first stage of validation that catches syntax errors.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SvelteValidationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to run validation script: {0}")]
    ScriptError(String),

    #[error("Failed to parse validation output: {0}")]
    ParseError(String),
}

#[derive(Debug)]
pub struct SvelteValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub line: Option<u64>,
    pub column: Option<u64>,
}

impl SvelteValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            error: None,
            line: None,
            column: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            valid: false,
            error: Some(error),
            line: None,
            column: None,
        }
    }
}

/// Validate Svelte component syntax using the Node.js validation script.
///
/// # Arguments
/// * `content` - The Svelte component source code
/// * `project_root` - Path to project root where scripts/validate-svelte.mjs is located
///
/// # Returns
/// * `Ok(SvelteValidationResult)` - Validation completed
/// * `Err(SvelteValidationError)` - Validation process failed
pub async fn validate_svelte_syntax(
    content: &str,
    project_root: &Path,
) -> Result<SvelteValidationResult, SvelteValidationError> {
    // Create temp file for validation
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("svelte_validate_{}.svelte", std::process::id()));

    // Write content to temp file
    tokio::fs::write(&temp_file, content).await?;

    // Run validation script
    let validation_script = project_root.join("scripts").join("validate-svelte.mjs");

    let result = tokio::process::Command::new("node")
        .arg(&validation_script)
        .arg(&temp_file)
        .output()
        .await;

    // Clean up temp file
    let _ = tokio::fs::remove_file(&temp_file).await;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            if output.status.success() {
                // Try to parse JSON output
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if json.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                        return Ok(SvelteValidationResult::success());
                    }
                }
                Ok(SvelteValidationResult::success())
            } else {
                // Parse error from JSON output
                parse_error_output(&stdout)
            }
        }
        Err(e) => Err(SvelteValidationError::ScriptError(e.to_string())),
    }
}

fn parse_error_output(stdout: &str) -> Result<SvelteValidationResult, SvelteValidationError> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
        let error = json
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown compilation error")
            .to_string();

        let line = json.get("line").and_then(|l| l.as_u64());
        let column = json.get("column").and_then(|c| c.as_u64());

        Ok(SvelteValidationResult {
            valid: false,
            error: Some(error),
            line,
            column,
        })
    } else {
        // Couldn't parse JSON, use raw output as error
        Ok(SvelteValidationResult::failure(stdout.trim().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_output_valid_json() {
        let json = r#"{"valid": false, "error": "Test error", "line": 5}"#;
        let result = parse_error_output(json).unwrap();
        assert!(!result.valid);
        assert_eq!(result.error, Some("Test error".to_string()));
        assert_eq!(result.line, Some(5));
    }

    #[test]
    fn test_parse_error_output_invalid_json() {
        let raw = "Some raw error message";
        let result = parse_error_output(raw).unwrap();
        assert!(!result.valid);
        assert_eq!(result.error, Some("Some raw error message".to_string()));
    }
}

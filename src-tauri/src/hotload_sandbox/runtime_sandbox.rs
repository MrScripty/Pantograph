//! Runtime Sandbox Validator
//!
//! Uses boa_engine (pure Rust JavaScript engine) to execute JavaScript code in a sandboxed environment.
//! This catches semantic errors that pass syntax validation but would fail at runtime,
//! such as using primitive values as components.

use boa_engine::{Context, Source};
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeValidationError {
    #[error("Code execution timed out")]
    Timeout,

    #[error("Semantic error: {message}")]
    SemanticError { message: String, line: Option<u64> },

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Failed to extract script: {0}")]
    ExtractionError(String),
}

/// Extract JavaScript code from a Svelte component's <script> block.
fn extract_script_content(svelte_source: &str) -> Option<String> {
    // Match <script> or <script lang="ts"> blocks
    let script_regex = Regex::new(r#"<script[^>]*>([\s\S]*?)</script>"#).ok()?;

    if let Some(captures) = script_regex.captures(svelte_source) {
        Some(captures.get(1)?.as_str().to_string())
    } else {
        None
    }
}

/// Extract component usage from Svelte template (variables used as <Component />).
fn extract_component_usage(svelte_source: &str) -> Vec<String> {
    let mut components = Vec::new();

    // Match capitalized tags that look like component usage: <MyComponent /> or <MyComponent>
    let component_regex = Regex::new(r"<([A-Z][a-zA-Z0-9]*)[^>]*/?>");
    if let Ok(regex) = component_regex {
        for cap in regex.captures_iter(svelte_source) {
            if let Some(name) = cap.get(1) {
                components.push(name.as_str().to_string());
            }
        }
    }

    components
}

/// Strip TypeScript type annotations from JavaScript code.
/// This is a simple transform that handles common patterns.
fn strip_typescript(script_content: &str) -> String {
    let mut result = script_content.to_string();

    // Remove type annotations from variable declarations: const x: Type = ...
    let type_annotation_regex = Regex::new(r":\s*[A-Za-z_][A-Za-z0-9_<>,\s\[\]|&]*\s*=").unwrap();
    result = type_annotation_regex.replace_all(&result, " =").to_string();

    // Remove type annotations from function parameters: (x: Type) => ...
    let param_type_regex = Regex::new(r"(\w+)\s*:\s*[A-Za-z_][A-Za-z0-9_<>,\s\[\]|&]*").unwrap();
    result = param_type_regex.replace_all(&result, "$1").to_string();

    // Remove 'as Type' casts
    let as_cast_regex = Regex::new(r"\s+as\s+[A-Za-z_][A-Za-z0-9_<>,\s\[\]|&]*").unwrap();
    result = as_cast_regex.replace_all(&result, "").to_string();

    // Remove interface/type declarations (entire lines)
    let interface_regex = Regex::new(r"(?m)^\s*(export\s+)?(interface|type)\s+\w+[^;{]*(\{[^}]*\}|=[^;]+);?\s*$").unwrap();
    result = interface_regex.replace_all(&result, "").to_string();

    result
}

/// Generate validation JavaScript that checks if component variables are valid.
fn generate_validation_script(script_content: &str, component_names: &[String]) -> String {
    let mut validation = String::new();

    // Strip TypeScript annotations before executing
    let js_content = strip_typescript(script_content);

    // Wrap in a try-catch to handle any execution errors
    validation.push_str("var __validation_result = '__VALIDATION_SUCCESS__';\n");
    validation.push_str("try {\n");

    // Add the original script content (with TS stripped)
    validation.push_str(&js_content);
    validation.push('\n');

    // Add validation checks for each component used in template
    for name in component_names {
        validation.push_str(&format!(
            r#"
// Validate component: {name}
if (typeof {name} !== 'undefined') {{
    var __val_{name} = {name};
    if (typeof __val_{name} === 'string') {{
        throw new Error("SEMANTIC_ERROR: '{name}' is a string (\"" + __val_{name}.slice(0, 50) + "...\"), not a component. You cannot use a string as <{name} />.");
    }}
    if (typeof __val_{name} === 'number') {{
        throw new Error("SEMANTIC_ERROR: '{name}' is a number (" + __val_{name} + "), not a component. You cannot use a number as <{name} />.");
    }}
    if (__val_{name} === null) {{
        throw new Error("SEMANTIC_ERROR: '{name}' is null, not a component. You cannot use null as <{name} />.");
    }}
    if (typeof __val_{name} !== 'function' && typeof __val_{name} !== 'object') {{
        throw new Error("SEMANTIC_ERROR: '{name}' has invalid type (" + typeof __val_{name} + "). Components must be functions or objects.");
    }}
}}
"#,
            name = name
        ));
    }

    validation.push_str("} catch (e) {\n");
    validation.push_str("  __validation_result = 'ERROR: ' + e.message;\n");
    validation.push_str("}\n");
    validation.push_str("__validation_result;\n");

    validation
}

/// Validate Svelte component runtime semantics using a sandboxed JavaScript environment.
///
/// This function:
/// 1. Extracts the <script> content from the Svelte component
/// 2. Extracts component usage from the template (capitalized tags)
/// 3. Runs the script in a sandbox with validation checks
/// 4. Catches errors like using primitives as components
///
/// # Arguments
/// * `svelte_source` - The complete Svelte component source code
/// * `timeout_ms` - Maximum execution time in milliseconds
///
/// # Returns
/// * `Ok(())` - Validation passed
/// * `Err(RuntimeValidationError)` - Validation failed with details
pub fn validate_runtime_semantics(
    svelte_source: &str,
    timeout_ms: u64,
) -> Result<(), RuntimeValidationError> {
    // Extract script content
    let script_content = match extract_script_content(svelte_source) {
        Some(content) => content,
        None => {
            // No script block - nothing to validate
            return Ok(());
        }
    };

    // If script is empty or just whitespace, nothing to validate
    if script_content.trim().is_empty() {
        return Ok(());
    }

    // Extract component usage from template
    let component_names = extract_component_usage(svelte_source);

    // Generate validation script
    let validation_script = generate_validation_script(&script_content, &component_names);

    // Run in a separate thread with timeout
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = Arc::clone(&completed);
    let script_clone = validation_script.clone();

    let handle = thread::spawn(move || {
        let mut context = Context::default();

        // Execute the validation script
        let result = context.eval(Source::from_bytes(&script_clone));
        completed_clone.store(true, Ordering::SeqCst);

        match result {
            Ok(value) => {
                let result_str = value.to_string(&mut context).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                Ok(result_str)
            }
            Err(e) => Err(e.to_string()),
        }
    });

    // Wait for completion with timeout
    let timeout = Duration::from_millis(timeout_ms);
    let start = std::time::Instant::now();

    loop {
        if completed.load(Ordering::SeqCst) {
            break;
        }
        if start.elapsed() >= timeout {
            // Note: We can't actually kill the thread, but we return timeout
            // The thread will eventually complete or be cleaned up when the process exits
            return Err(RuntimeValidationError::Timeout);
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Get the result
    match handle.join() {
        Ok(Ok(result_str)) => handle_validation_result(&result_str),
        Ok(Err(error_str)) => handle_js_error(&error_str),
        Err(_) => Err(RuntimeValidationError::RuntimeError(
            "JavaScript execution panicked".to_string(),
        )),
    }
}

fn handle_validation_result(result: &str) -> Result<(), RuntimeValidationError> {
    if result == "__VALIDATION_SUCCESS__" {
        Ok(())
    } else if result.starts_with("ERROR: ") {
        let error_msg = result.trim_start_matches("ERROR: ");
        handle_js_error(error_msg)
    } else {
        // Unexpected result, but not necessarily an error
        Ok(())
    }
}

fn handle_js_error(error_str: &str) -> Result<(), RuntimeValidationError> {
    // Check if it's a timeout
    if error_str.contains("timeout") || error_str.contains("Timeout") || error_str.contains("timed out") {
        return Err(RuntimeValidationError::Timeout);
    }

    // Check if it's our semantic error
    if error_str.contains("SEMANTIC_ERROR:") {
        let message = error_str
            .split("SEMANTIC_ERROR:")
            .nth(1)
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| error_str.to_string());

        return Err(RuntimeValidationError::SemanticError {
            message,
            line: None,
        });
    }

    // Other runtime error (could be syntax error in JS, undefined variable, etc.)
    if error_str.contains("is not defined") {
        // Svelte 5 runes are compile-time constructs, not runtime functions.
        // The sandbox can't execute them, but that's expected - not an error.
        // Complete list of Svelte 5 runes (as of v5.25):
        // - $state, $state.raw, $state.snapshot
        // - $derived, $derived.by
        // - $effect, $effect.pre, $effect.tracking, $effect.root
        // - $props, $bindable, $inspect, $host, $id
        let svelte_runes = [
            "$props", "$state", "$derived", "$effect", "$bindable", "$inspect", "$host", "$id",
        ];
        if svelte_runes.iter().any(|rune| error_str.contains(rune)) {
            log::debug!("Ignoring expected rune undefined error: {}", error_str);
            return Ok(());
        }

        return Err(RuntimeValidationError::SemanticError {
            message: format!("Undefined variable: {}", error_str),
            line: None,
        });
    }

    // Some errors are expected (like import statements that won't work in sandbox)
    if error_str.contains("Cannot use import statement")
        || error_str.contains("import")
        || error_str.contains("export")
        || error_str.contains("SyntaxError")
    {
        // Import/export statements and some syntax patterns can't be validated in sandbox
        // This is OK - the Svelte compiler already validated the syntax
        return Ok(());
    }

    // Log other errors but don't fail validation
    log::debug!(
        "Runtime validation encountered non-fatal error: {}",
        error_str
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_script_content() {
        let svelte = r#"
<script>
  const x = 5;
</script>
<div>{x}</div>
"#;
        let script = extract_script_content(svelte).unwrap();
        assert!(script.contains("const x = 5"));
    }

    #[test]
    fn test_extract_script_content_typescript() {
        let svelte = r#"
<script lang="ts">
  const x: number = 5;
</script>
<div>{x}</div>
"#;
        let script = extract_script_content(svelte).unwrap();
        assert!(script.contains("const x: number = 5"));
    }

    #[test]
    fn test_extract_component_usage() {
        let svelte = r#"
<script>
  const MyComponent = "oops";
</script>
<MyComponent />
<AnotherComponent prop={value} />
<div>regular element</div>
"#;
        let components = extract_component_usage(svelte);
        assert!(components.contains(&"MyComponent".to_string()));
        assert!(components.contains(&"AnotherComponent".to_string()));
        assert!(!components.contains(&"div".to_string()));
    }

    #[test]
    fn test_validate_empty_script() {
        let svelte = r#"
<script>
</script>
<div>Hello</div>
"#;
        let result = validate_runtime_semantics(svelte, 5000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_no_script() {
        let svelte = r#"<div>Hello</div>"#;
        let result = validate_runtime_semantics(svelte, 5000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_string_as_component() {
        let svelte = r#"
<script>
  const MyComponent = "this is a string";
</script>
<MyComponent />
"#;
        let result = validate_runtime_semantics(svelte, 5000);
        assert!(result.is_err());
        if let Err(RuntimeValidationError::SemanticError { message, .. }) = result {
            assert!(message.contains("string"));
            assert!(message.contains("MyComponent"));
        } else {
            panic!("Expected SemanticError");
        }
    }

    #[test]
    fn test_validate_number_as_component() {
        let svelte = r#"
<script>
  const Counter = 42;
</script>
<Counter />
"#;
        let result = validate_runtime_semantics(svelte, 5000);
        assert!(result.is_err());
        if let Err(RuntimeValidationError::SemanticError { message, .. }) = result {
            assert!(message.contains("number"));
            assert!(message.contains("Counter"));
        } else {
            panic!("Expected SemanticError");
        }
    }

    #[test]
    fn test_validate_object_as_component_ok() {
        let svelte = r#"
<script>
  const MyComponent = { render: function() {} };
</script>
<MyComponent />
"#;
        let result = validate_runtime_semantics(svelte, 5000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_strip_typescript() {
        let ts_code = "const x: number = 5;";
        let js_code = strip_typescript(ts_code);
        assert!(js_code.contains("const x = 5"));
        assert!(!js_code.contains(": number"));
    }
}

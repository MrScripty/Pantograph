use std::collections::HashMap;

use crate::error::{NodeEngineError, Result};

pub(crate) fn execute_validator(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let code = inputs
        .get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing code input".to_string()))?;

    let forbidden_patterns: &[(&str, &str)] = &[
        (
            "export let ",
            "Use `let { prop } = $props()` instead of `export let prop`",
        ),
        ("on:click", "Use `onclick` instead of `on:click`"),
        ("on:change", "Use `onchange` instead of `on:change`"),
        ("on:input", "Use `oninput` instead of `on:input`"),
        ("on:submit", "Use `onsubmit` instead of `on:submit`"),
    ];

    // Strip single-line comments before checking patterns
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

    let mut valid = true;
    let mut error = String::new();
    let mut category = String::new();

    for (pattern, fix) in forbidden_patterns {
        if code_no_comments.contains(pattern) {
            valid = false;
            error = format!(
                "SVELTE 5 SYNTAX ERROR: Found forbidden pattern '{}'. {}.",
                pattern, fix
            );
            category = "SveltePattern".to_string();
            break;
        }
    }

    if valid {
        let script_opens = code.matches("<script").count();
        let script_closes = code.matches("</script>").count();
        if script_opens != script_closes {
            valid = false;
            error = "Unbalanced <script> tags".to_string();
            category = "SvelteCompiler".to_string();
        }
    }

    let mut outputs = HashMap::new();
    outputs.insert("valid".to_string(), serde_json::json!(valid));
    outputs.insert("error".to_string(), serde_json::json!(error));
    outputs.insert("category".to_string(), serde_json::json!(category));
    Ok(outputs)
}

pub(crate) fn execute_json_filter(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let json = inputs
        .get("json")
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing json input".to_string()))?;

    let path = inputs
        .get("_data")
        .and_then(|d| d.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or("");

    let (value, found) = extract_json_path(json, path);

    let mut outputs = HashMap::new();
    outputs.insert("value".to_string(), value);
    outputs.insert("found".to_string(), serde_json::json!(found));
    Ok(outputs)
}

/// Extract a value from JSON using a dot-delimited path expression.
///
/// Supports object field access (`field.subfield`), array indexing (`[0]`),
/// and combinations (`field[0].subfield`).
fn extract_json_path(json: &serde_json::Value, path: &str) -> (serde_json::Value, bool) {
    if path.is_empty() {
        return (json.clone(), true);
    }

    let mut current = json;
    let mut remaining = path;

    while !remaining.is_empty() {
        // Handle array indexing: [N]
        if remaining.starts_with('[') {
            if let Some(end) = remaining.find(']') {
                let index_str = &remaining[1..end];
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(val) = current.get(index) {
                        current = val;
                        remaining = &remaining[end + 1..];
                        if remaining.starts_with('.') {
                            remaining = &remaining[1..];
                        }
                        continue;
                    }
                }
            }
            return (serde_json::Value::Null, false);
        }

        // Handle object field access
        let (field, rest) = if let Some(dot_pos) = remaining.find('.') {
            let bracket_pos = remaining.find('[').unwrap_or(remaining.len());
            if dot_pos < bracket_pos {
                (&remaining[..dot_pos], &remaining[dot_pos + 1..])
            } else {
                (&remaining[..bracket_pos], &remaining[bracket_pos..])
            }
        } else if let Some(bracket_pos) = remaining.find('[') {
            (&remaining[..bracket_pos], &remaining[bracket_pos..])
        } else {
            (remaining, "")
        };

        if !field.is_empty() {
            if let Some(val) = current.get(field) {
                current = val;
            } else {
                return (serde_json::Value::Null, false);
            }
        }
        remaining = rest;
    }

    (current.clone(), true)
}

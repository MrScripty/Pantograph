use crate::agent::enricher::ErrorCategory;
use crate::config::{ImportValidationMode, SandboxConfig};
use std::path::Path;

use crate::agent::tools::validation::{
    MATHML_ELEMENTS, STANDARD_HTML_ELEMENTS, SVG_ELEMENTS, capitalize_first,
};

/// Extract template content from Svelte file (excludes script and style blocks).
pub fn extract_template_content(content: &str) -> String {
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

pub fn validate_svelte_content(content: &str) -> Result<(), (String, ErrorCategory)> {
    // Strip comments before validation to avoid false positives.
    let content_no_comments: String = content
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

    let forbidden_patterns: &[(&str, &str)] = &[
        (
            "export let ",
            "Use `let { prop } = $props()` instead of `export let prop`",
        ),
        (
            "export let\t",
            "Use `let { prop } = $props()` instead of `export let prop`",
        ),
        ("on:click", "Use `onclick` instead of `on:click`"),
        ("on:change", "Use `onchange` instead of `on:change`"),
        ("on:input", "Use `oninput` instead of `on:input`"),
        ("on:submit", "Use `onsubmit` instead of `on:submit`"),
        ("on:keydown", "Use `onkeydown` instead of `on:keydown`"),
        ("on:keyup", "Use `onkeyup` instead of `on:keyup`"),
        ("on:keypress", "Use `onkeypress` instead of `on:keypress`"),
        (
            "on:mouseenter",
            "Use `onmouseenter` instead of `on:mouseenter`",
        ),
        (
            "on:mouseleave",
            "Use `onmouseleave` instead of `on:mouseleave`",
        ),
        (
            "on:mouseover",
            "Use `onmouseover` instead of `on:mouseover`",
        ),
        ("on:mouseout", "Use `onmouseout` instead of `on:mouseout`"),
        (
            "on:mousedown",
            "Use `onmousedown` instead of `on:mousedown`",
        ),
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

    if content.contains("<style>") && !content.contains("@apply") && !content.contains("global(") {
        let style_start = content.find("<style>");
        let style_end = content.find("</style>");
        if let (Some(start), Some(end)) = (style_start, style_end) {
            let style_content = &content[start..end];
            if !style_content.contains("@apply") && !style_content.contains(":global") {
                return Err((
                    "Custom CSS not allowed. Use Tailwind classes only, or @apply directive."
                        .to_string(),
                    ErrorCategory::Styling,
                ));
            }
        }
    }

    let script_opens = content.matches("<script").count();
    let script_closes = content.matches("</script>").count();
    if script_opens != script_closes {
        return Err((
            "Unbalanced <script> tags".to_string(),
            ErrorCategory::SvelteCompiler,
        ));
    }

    let template_content = extract_template_content(content);
    let element_regex = regex::Regex::new(r"<([a-z][a-z0-9]*)[^>]*[/]?>").unwrap();
    for cap in element_regex.captures_iter(&template_content) {
        if let Some(tag_match) = cap.get(1) {
            let tag_name = tag_match.as_str();
            if STANDARD_HTML_ELEMENTS.contains(&tag_name)
                || SVG_ELEMENTS.contains(&tag_name)
                || MATHML_ELEMENTS.contains(&tag_name)
            {
                continue;
            }
            if tag_name.contains('-') {
                continue;
            }

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

/// Validate imports based on the configured validation mode.
pub async fn validate_imports(
    project_root: &Path,
    sandbox_config: &SandboxConfig,
    file_path: &Path,
) -> Result<(), (String, ErrorCategory)> {
    let script_name = match sandbox_config.import_validation_mode {
        ImportValidationMode::None => return Ok(()),
        ImportValidationMode::ImportResolve => "validate-imports.mjs",
        ImportValidationMode::ViteIntegration => "validate-vite.mjs",
        ImportValidationMode::EsbuildBundle => "validate-esbuild.mjs",
    };

    let validation_script = project_root.join("scripts").join(script_name);

    let allowed_packages_json = if !sandbox_config.allowed_packages.is_empty() {
        serde_json::to_string(&sandbox_config.allowed_packages).unwrap_or_default()
    } else {
        String::new()
    };

    let mut cmd = tokio::process::Command::new("node");
    cmd.arg(&validation_script).arg(file_path).arg(project_root);

    if matches!(
        sandbox_config.import_validation_mode,
        ImportValidationMode::ImportResolve
    ) && !allowed_packages_json.is_empty()
    {
        cmd.arg(&allowed_packages_json);
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(sandbox_config.validation_timeout_ms),
        cmd.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    let error_msg = error_json
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown import validation error")
                        .to_string();
                    let line = error_json.get("line").and_then(|l| l.as_u64());
                    let suggestions: Vec<String> = error_json
                        .get("errors")
                        .and_then(|e| e.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|e| e.get("suggestions"))
                        .and_then(|s| s.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut full_error = format!("IMPORT VALIDATION ERROR: {}", error_msg);
                    if let Some(line_num) = line {
                        full_error.push_str(&format!(" (line {})", line_num));
                    }
                    if !suggestions.is_empty() {
                        full_error.push_str(&format!(
                            "\n\nDid you mean: {}?",
                            suggestions
                                .iter()
                                .map(|s| format!("\"{}\"", s))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    full_error
                        .push_str("\n\nEnsure the package is listed in package.json dependencies.");
                    return Err((full_error, ErrorCategory::ImportResolution));
                }
                return Err((
                    format!("IMPORT VALIDATION ERROR: {}", stdout.trim()),
                    ErrorCategory::ImportResolution,
                ));
            }
            Ok(())
        }
        Ok(Err(e)) => {
            log::warn!(
                "Import validation script failed to run: {}. Skipping import validation.",
                e
            );
            Ok(())
        }
        Err(_) => Err((
            format!(
                "IMPORT VALIDATION ERROR: Validation timed out after {}ms. \
                 This may indicate a complex import graph or slow disk I/O.",
                sandbox_config.validation_timeout_ms
            ),
            ErrorCategory::ImportResolution,
        )),
    }
}

/// Validate code quality using ESLint (if enabled).
pub async fn validate_lint(
    project_root: &Path,
    sandbox_config: &SandboxConfig,
    file_path: &Path,
) -> Result<(), (String, ErrorCategory)> {
    if !sandbox_config.lint_enabled {
        return Ok(());
    }

    let lint_script = project_root.join("scripts").join("validate-lint.mjs");
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(sandbox_config.validation_timeout_ms),
        tokio::process::Command::new("node")
            .arg(&lint_script)
            .arg(file_path)
            .arg(project_root)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !output.status.success() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    let error_msg = error_json
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown linting error")
                        .to_string();
                    let line = error_json.get("line").and_then(|l| l.as_u64());
                    let mut full_error = format!("LINTING ERROR: {}", error_msg);
                    if let Some(line_num) = line {
                        full_error.push_str(&format!(" (line {})", line_num));
                    }
                    full_error.push_str("\n\nCommon fixes:\n");
                    full_error.push_str(
                        "- Don't use `undefined` explicitly - use `null` or omit initialization\n",
                    );
                    full_error.push_str("- Remove unused variables\n");
                    full_error.push_str("- Check for accidental type coercion");
                    return Err((full_error, ErrorCategory::Linting));
                }
                return Err((
                    format!("LINTING ERROR: {}", stdout.trim()),
                    ErrorCategory::Linting,
                ));
            }
            Ok(())
        }
        Ok(Err(e)) => {
            log::warn!(
                "Lint validation script failed to run: {}. Skipping lint validation.",
                e
            );
            Ok(())
        }
        Err(_) => {
            log::warn!("Lint validation timed out. Skipping lint validation.");
            Ok(())
        }
    }
}

/// Validate design system compliance (advisory - returns warnings, not errors).
pub async fn validate_design_system(
    project_root: &Path,
    sandbox_config: &SandboxConfig,
    file_path: &Path,
) -> Vec<String> {
    let validation_script = project_root
        .join("scripts")
        .join("validate-design-system.mjs");
    if !validation_script.exists() {
        log::debug!("Design system validation script not found, skipping");
        return vec![];
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(sandbox_config.validation_timeout_ms),
        tokio::process::Command::new("node")
            .arg(&validation_script)
            .arg(file_path)
            .output(),
    )
    .await;

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

/// Validate that template expressions don't contain JSX syntax.
pub async fn validate_jsx_in_template(
    project_root: &Path,
    sandbox_config: &SandboxConfig,
    file_path: &Path,
) -> Result<(), (String, ErrorCategory)> {
    let validation_script = project_root
        .join("scripts")
        .join("validate-jsx-in-template.mjs");

    if !validation_script.exists() {
        log::debug!("JSX validation script not found, skipping JSX validation");
        return Ok(());
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(sandbox_config.validation_timeout_ms),
        tokio::process::Command::new("node")
            .arg(&validation_script)
            .arg(file_path)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !output.status.success() {
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    let error_msg = error_json
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Found JSX syntax in template")
                        .to_string();
                    return Err((error_msg, ErrorCategory::SveltePattern));
                }
                return Err((
                    format!("JSX SYNTAX ERROR: {}", stdout.trim()),
                    ErrorCategory::SveltePattern,
                ));
            }
            Ok(())
        }
        Ok(Err(e)) => {
            log::warn!(
                "JSX validation script failed to run: {}. Skipping JSX validation.",
                e
            );
            Ok(())
        }
        Err(_) => {
            log::warn!("JSX validation timed out. Skipping JSX validation.");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_template_content_removes_script_and_style_blocks() {
        let input = "<script>const a = 1;</script><div>Hello</div><style>.x{}</style>";
        let out = extract_template_content(input);
        assert!(out.contains("<div>Hello</div>"));
        assert!(!out.contains("<script>"));
        assert!(!out.contains("<style>"));
    }

    #[test]
    fn test_validate_svelte_content_rejects_export_let_pattern() {
        let content = "<script>export let name;</script><div>{name}</div>";
        let err = validate_svelte_content(content).expect_err("must reject export let");
        assert!(err.0.contains("SVELTE 5 SYNTAX ERROR"));
        assert_eq!(err.1, ErrorCategory::SveltePattern);
    }

    #[test]
    fn test_validate_svelte_content_rejects_unbalanced_script_tags() {
        let content = "<script>const a = 1;<div>hi</div>";
        let err = validate_svelte_content(content).expect_err("must reject unbalanced script");
        assert!(err.0.contains("Unbalanced <script> tags"));
        assert_eq!(err.1, ErrorCategory::SvelteCompiler);
    }
}

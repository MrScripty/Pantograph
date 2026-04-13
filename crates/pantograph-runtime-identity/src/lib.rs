//! Shared backend identity helpers for Pantograph runtime, workflow, and host layers.
//!
//! Runtime-facing contracts use stable backend keys such as `llama_cpp`,
//! while model dependency and execution surfaces use engine keys such as
//! `llamacpp`. This crate centralizes both normalization rules so individual
//! crates do not need to maintain their own alias tables.

use std::collections::BTreeSet;

pub const DEFAULT_FRONTEND_RUNTIME_NAME: &str = "openai-compatible";

pub fn canonical_runtime_backend_key(name: &str) -> String {
    match ascii_alnum_token(name).as_str() {
        "llamacpp" => "llama_cpp".to_string(),
        "ollama" => "ollama".to_string(),
        "candle" => "candle".to_string(),
        "pytorch" => "pytorch".to_string(),
        other => other.to_string(),
    }
}

pub fn canonical_engine_backend_key(value: Option<&str>) -> Option<String> {
    let normalized = value
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())?;

    match normalized.as_str() {
        "llama.cpp" | "llama-cpp" | "llama_cpp" | "llamacpp" => Some("llamacpp".to_string()),
        "onnxruntime" | "onnx-runtime" | "onnx_runtime" => Some("onnx-runtime".to_string()),
        "torch" | "pytorch" => Some("pytorch".to_string()),
        "stable-audio" | "stable_audio" => Some("stable_audio".to_string()),
        other => Some(other.to_string()),
    }
}

pub fn normalize_runtime_identifier(name: &str) -> String {
    normalize_runtime_identifier_with_fallback(name, DEFAULT_FRONTEND_RUNTIME_NAME)
}

pub fn normalize_runtime_identifier_with_fallback(name: &str, fallback: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        fallback.replace('-', "_")
    } else {
        normalized
    }
}

pub fn backend_key_aliases(display_name: &str, runtime_id: &str) -> Vec<String> {
    let mut aliases = BTreeSet::new();
    let trimmed = display_name.trim();

    aliases.insert(runtime_id.to_string());
    if !trimmed.is_empty() {
        aliases.insert(trimmed.to_string());
    }

    let collapsed = runtime_id.replace('_', "");
    if !collapsed.is_empty() {
        aliases.insert(collapsed);
    }

    aliases.into_iter().collect()
}

fn ascii_alnum_token(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_runtime_backend_key_normalizes_known_aliases() {
        assert_eq!(canonical_runtime_backend_key("llama.cpp"), "llama_cpp");
        assert_eq!(canonical_runtime_backend_key("llama_cpp"), "llama_cpp");
        assert_eq!(canonical_runtime_backend_key("PyTorch"), "pytorch");
        assert_eq!(
            canonical_runtime_backend_key("OpenAI Compatible"),
            "openaicompatible"
        );
    }

    #[test]
    fn canonical_engine_backend_key_normalizes_known_aliases() {
        assert_eq!(
            canonical_engine_backend_key(Some("llama_cpp")),
            Some("llamacpp".to_string())
        );
        assert_eq!(
            canonical_engine_backend_key(Some("onnx_runtime")),
            Some("onnx-runtime".to_string())
        );
        assert_eq!(
            canonical_engine_backend_key(Some("torch")),
            Some("pytorch".to_string())
        );
    }

    #[test]
    fn normalize_runtime_identifier_preserves_unknown_display_names() {
        assert_eq!(
            normalize_runtime_identifier("OpenAI Compatible"),
            "openai_compatible"
        );
        assert_eq!(
            normalize_runtime_identifier_with_fallback("", "fallback-name"),
            "fallback_name"
        );
    }

    #[test]
    fn backend_key_aliases_include_runtime_display_and_collapsed_forms() {
        assert_eq!(
            backend_key_aliases("llama.cpp", "llama_cpp"),
            vec![
                "llama.cpp".to_string(),
                "llama_cpp".to_string(),
                "llamacpp".to_string()
            ]
        );
    }
}

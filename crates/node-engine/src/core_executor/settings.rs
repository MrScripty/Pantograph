use std::collections::HashMap;

use crate::error::Result;

/// Expand inference settings schema into individual per-parameter outputs.
///
/// Reads the `inference_settings` JSON array, passes it through unchanged,
/// and emits each parameter's resolved override-or-default value on a port
/// keyed by `param.key`.
pub(crate) fn execute_expand_settings(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let settings_value = inputs
        .get("inference_settings")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));

    let mut outputs = HashMap::new();
    // Pass through schema unchanged for downstream inference node
    outputs.insert("inference_settings".to_string(), settings_value.clone());

    // Emit each parameter's default as an individual output port
    if let Some(schema) = settings_value.as_array() {
        for param in schema {
            if let Some(key) = param.get("key").and_then(|k| k.as_str()) {
                let value = read_optional_input_value(inputs, key).unwrap_or_else(|| {
                    param
                        .get("default")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                });
                let runtime_value = resolve_inference_setting_runtime_value(param, value);
                if !runtime_value.is_null() {
                    outputs.insert(key.to_string(), runtime_value);
                }
            }
        }
    }

    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Inference settings helper
// ---------------------------------------------------------------------------

/// Build a settings map from the `inference_settings` schema and port inputs.
///
/// The `inference_settings` input carries a JSON array of parameter schemas
/// (from pumas-library). For each param in the schema, uses the connected
/// port value if present, otherwise falls back to the schema's default.
///
/// Supports both legacy primitive defaults and option objects shaped like
/// `{ "label": "...", "value": ... }` by resolving to the runtime value.
pub(crate) fn resolve_inference_setting_runtime_value(
    parameter: &serde_json::Value,
    value: serde_json::Value,
) -> serde_json::Value {
    let normalized = if let serde_json::Value::Object(map) = &value {
        if map.contains_key("label") {
            if let Some(option_value) = map.get("value") {
                option_value.clone()
            } else {
                value
            }
        } else {
            value
        }
    } else {
        value
    };

    let Some(candidate_label) = normalized.as_str() else {
        return normalized;
    };

    let allowed_values = parameter
        .get("constraints")
        .and_then(|constraints| constraints.get("allowed_values"))
        .and_then(|values| values.as_array());
    let Some(allowed_values) = allowed_values else {
        return normalized;
    };

    for option in allowed_values {
        if let serde_json::Value::Object(option_map) = option {
            let option_label = option_map
                .get("label")
                .or_else(|| option_map.get("name"))
                .or_else(|| option_map.get("key"))
                .and_then(|v| v.as_str());
            if option_label == Some(candidate_label) {
                if let Some(option_value) = option_map.get("value") {
                    return option_value.clone();
                }
            }
        }
    }

    normalized
}

pub(crate) fn build_extra_settings(
    inputs: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut settings = HashMap::new();

    let schema: Vec<serde_json::Value> = inputs
        .get("inference_settings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for param in &schema {
        if let Some(key) = param.get("key").and_then(|k| k.as_str()) {
            let value = inputs.get(key).cloned().unwrap_or_else(|| {
                param
                    .get("default")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            });
            let runtime_value = resolve_inference_setting_runtime_value(param, value);
            if !runtime_value.is_null() {
                settings.insert(key.to_string(), runtime_value);
            }
        }
    }

    settings
}

pub(crate) fn read_optional_input_string(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    inputs
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|d| d.get(key))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

pub(crate) fn read_optional_input_value(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<serde_json::Value> {
    inputs
        .get(key)
        .cloned()
        .or_else(|| inputs.get("_data").and_then(|d| d.get(key)).cloned())
}

#[cfg_attr(
    not(any(feature = "inference-nodes", feature = "audio-nodes")),
    allow(dead_code)
)]
pub(crate) fn read_optional_input_string_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<String> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_string(inputs, key))
}

#[cfg_attr(
    not(any(feature = "inference-nodes", feature = "audio-nodes")),
    allow(dead_code)
)]
pub(crate) fn read_optional_input_value_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<serde_json::Value> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_value(inputs, key))
}

#[cfg_attr(
    not(any(feature = "inference-nodes", feature = "audio-nodes")),
    allow(dead_code)
)]
pub(crate) fn read_optional_input_bool(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<bool> {
    let value = read_optional_input_value(inputs, key)?;
    if let Some(boolean) = value.as_bool() {
        return Some(boolean);
    }
    value
        .as_str()
        .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        })
}

#[cfg_attr(
    not(any(feature = "inference-nodes", feature = "audio-nodes")),
    allow(dead_code)
)]
pub(crate) fn read_optional_input_bool_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<bool> {
    aliases
        .iter()
        .find_map(|key| read_optional_input_bool(inputs, key))
}

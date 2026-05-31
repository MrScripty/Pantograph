use super::*;

impl TauriTaskExecutor {
    #[allow(dead_code)]
    pub(in crate::task_executor) fn parse_dependency_requirements_input(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<node_engine::ModelDependencyRequirements> {
        let raw = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_requirements", "dependencyRequirements"],
        )?;
        serde_json::from_value(raw).ok()
    }

    #[allow(dead_code)]
    pub(in crate::task_executor) fn read_input_dependency_override_patches(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<node_engine::DependencyOverridePatchV1> {
        let Some(raw) = Self::read_optional_input_value_aliases(
            inputs,
            &[
                "dependency_override_patches",
                "dependencyOverridePatches",
                "manual_overrides",
                "manualOverrides",
            ],
        ) else {
            return Vec::new();
        };

        if raw.is_null() {
            return Vec::new();
        }
        if raw.is_object() {
            return serde_json::from_value::<node_engine::DependencyOverridePatchV1>(raw)
                .map(|single| vec![single])
                .unwrap_or_default();
        }
        serde_json::from_value::<Vec<node_engine::DependencyOverridePatchV1>>(raw)
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub(in crate::task_executor) fn platform_context_from_requirement_key(
        platform_key: &str,
    ) -> Option<serde_json::Value> {
        let normalized = platform_key.trim();
        if normalized.is_empty() {
            return None;
        }

        let mut parts = normalized.split('-');
        let os = parts.next().unwrap_or_default().trim();
        let arch = parts.next().unwrap_or_default().trim();
        if os.is_empty() || arch.is_empty() {
            return None;
        }

        Some(serde_json::json!({ "os": os, "arch": arch }))
    }

    #[allow(dead_code)]
    pub(in crate::task_executor) fn read_input_selected_binding_ids(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let Some(raw) = Self::read_optional_input_value_aliases(
            inputs,
            &["selected_binding_ids", "selectedBindingIds"],
        ) else {
            return Vec::new();
        };

        raw.as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    #[allow(dead_code)]
    pub(in crate::task_executor) fn infer_task_type_primary(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> String {
        if let Some(task) = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        ) {
            if !task.trim().is_empty() {
                return task;
            }
        }

        let model_type =
            Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
                .unwrap_or_default()
                .to_lowercase();

        if node_type == "audio-generation" || model_type == "audio" {
            return "text-to-audio".to_string();
        }

        match model_type.as_str() {
            "diffusion" => "text-to-image".to_string(),
            "vision" => "image-to-text".to_string(),
            "embedding" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    pub(in crate::task_executor) fn python_runtime_handles_node(node_type: &str) -> bool {
        match node_type {
            "audio-generation" | "onnx-inference" => true,
            _ => false,
        }
    }

    pub(in crate::task_executor) fn sanitize_key_component(raw: &str) -> String {
        raw.chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>()
    }

    pub(in crate::task_executor) fn stable_hash_hex(value: &str) -> String {
        let mut digest = Self::FNV64_OFFSET_BASIS;
        for byte in value.as_bytes() {
            digest ^= *byte as u64;
            digest = digest.wrapping_mul(Self::FNV64_PRIME);
        }
        format!("{:016x}", digest)
    }
}

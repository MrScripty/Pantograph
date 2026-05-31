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

    #[allow(dead_code)]
    pub(in crate::task_executor) fn build_model_dependency_request(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> ModelDependencyRequest {
        let requirements = Self::parse_dependency_requirements_input(inputs);
        let package_facts = Self::read_resolved_model_package_facts_for_preflight(inputs);

        let task_type_primary = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        )
        .filter(|task| !task.trim().is_empty())
        .or_else(|| Self::task_type_primary_from_package_facts(package_facts.as_ref()))
        .unwrap_or_else(|| Self::infer_task_type_primary(node_type, inputs));
        let model_id = Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"])
            .or_else(|| Self::model_id_from_pumas_model_ref_input(inputs));
        let platform_context = Self::read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        )
        .or_else(|| {
            requirements
                .as_ref()
                .and_then(|r| Self::platform_context_from_requirement_key(&r.platform_key))
        });

        let mut selected_binding_ids = Self::read_input_selected_binding_ids(inputs);
        if selected_binding_ids.is_empty() {
            if let Some(req) = &requirements {
                selected_binding_ids = req.selected_binding_ids.clone();
            }
        }

        ModelDependencyRequest {
            node_type: node_type.to_string(),
            model_path: String::new(),
            model_id,
            model_type: Self::read_optional_input_string_aliases(
                inputs,
                &["model_type", "modelType"],
            ),
            task_type_primary: Some(task_type_primary),
            backend_key: None,
            platform_context,
            selected_binding_ids,
            dependency_override_patches: Self::read_input_dependency_override_patches(inputs),
        }
    }

    #[allow(dead_code)]
    fn model_id_from_pumas_model_ref_input(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        Self::read_optional_input_value_aliases(inputs, &["pumas_model_ref", "pumasModelRef"])
            .and_then(|model_ref| {
                ["model_id", "modelId"].iter().find_map(|key| {
                    model_ref
                        .get(*key)
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                })
            })
    }

    #[allow(dead_code)]
    fn read_resolved_model_package_facts_for_preflight(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<inference::ResolvedModelPackageFacts> {
        Self::read_optional_input_value_aliases(
            inputs,
            &[
                "resolved_model_package_facts",
                "resolvedModelPackageFacts",
                "model_package_facts",
                "modelPackageFacts",
            ],
        )
        .filter(|raw| !raw.is_null())
        .and_then(|raw| serde_json::from_value(raw).ok())
    }

    #[allow(dead_code)]
    fn task_type_primary_from_package_facts(
        facts: Option<&inference::ResolvedModelPackageFacts>,
    ) -> Option<String> {
        facts
            .and_then(|facts| facts.task.task_type_primary.clone())
            .filter(|task| !task.trim().is_empty())
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

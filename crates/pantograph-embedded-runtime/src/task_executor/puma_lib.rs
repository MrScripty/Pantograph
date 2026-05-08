use super::*;
use workflow_nodes::setup::{PumasSelectedModelDetail, PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

impl TauriTaskExecutor {
    pub(super) fn insert_puma_lib_output_string(
        outputs: &mut HashMap<String, serde_json::Value>,
        key: &str,
        value: Option<String>,
    ) {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            outputs.insert(key.to_string(), serde_json::json!(value));
        }
    }

    async fn resolve_puma_lib_model_record(
        api: &Arc<pumas_library::PumasApi>,
        model_id: Option<&str>,
    ) -> std::result::Result<Option<pumas_library::ModelRecord>, String> {
        if let Some(model_id) = model_id {
            return api
                .get_model(model_id)
                .await
                .map_err(|error| format!("Failed to query Puma-Lib model '{model_id}': {error}"));
        }

        Ok(None)
    }

    async fn resolve_puma_lib_selected_detail(
        selector_access: &Arc<PumasSelectorAccess>,
        model_id: &str,
    ) -> std::result::Result<Option<PumasSelectedModelDetail>, String> {
        let detail = selector_access
            .selected_model_detail(model_id)
            .await
            .map_err(|error| {
                format!("Failed to query Puma-Lib selected detail for '{model_id}': {error}")
            })?;
        if detail.selector_row.is_none() && detail.descriptor.is_none() {
            Ok(None)
        } else {
            Ok(Some(detail))
        }
    }

    fn apply_puma_lib_selected_detail(
        detail: PumasSelectedModelDetail,
        requested_model_id: &str,
        model_path: &mut String,
        model_id: &mut Option<String>,
        model_type: &mut Option<String>,
        task_type_primary: &mut Option<String>,
        recommended_backend: &mut Option<String>,
    ) {
        let row = detail.selector_row.as_ref();
        let descriptor = detail.descriptor.as_ref();

        *model_id = Some(
            descriptor
                .map(|descriptor| descriptor.model_id.clone())
                .or_else(|| row.map(|row| row.model_ref.model_id.clone()))
                .unwrap_or_else(|| requested_model_id.to_string()),
        );
        if let Some(entry_path) = descriptor
            .map(|descriptor| descriptor.entry_path.trim())
            .filter(|path| !path.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                row.and_then(|row| row.executable_entry_path())
                    .map(ToOwned::to_owned)
            })
        {
            *model_path = entry_path;
        }
        if let Some(value) = descriptor
            .map(|descriptor| descriptor.model_type.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                row.and_then(|row| row.model_type.as_deref())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
        {
            *model_type = Some(value);
        }
        if let Some(task) = descriptor
            .map(|descriptor| descriptor.task_type_primary.trim())
            .filter(|task| !task.is_empty() && *task != "unknown")
            .map(ToOwned::to_owned)
            .or_else(|| {
                row.and_then(|row| row.task_type_primary.as_deref())
                    .map(str::trim)
                    .filter(|task| !task.is_empty() && *task != "unknown")
                    .map(ToOwned::to_owned)
            })
        {
            *task_type_primary = Some(task);
        }
        if let Some(backend) = descriptor
            .and_then(|descriptor| descriptor.recommended_backend.as_deref())
            .map(str::trim)
            .filter(|backend| !backend.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                row.and_then(|row| row.recommended_backend.as_deref())
                    .map(str::trim)
                    .filter(|backend| !backend.is_empty())
                    .map(ToOwned::to_owned)
            })
        {
            *recommended_backend = Some(backend);
        }
    }

    fn selected_detail_inference_settings_value(
        detail: &PumasSelectedModelDetail,
        requested_model_id: &str,
    ) -> Option<serde_json::Value> {
        if detail.inference_settings.is_empty() {
            return None;
        }

        match serde_json::to_value(&detail.inference_settings) {
            Ok(value)
                if value
                    .as_array()
                    .is_some_and(|settings| !settings.is_empty()) =>
            {
                Some(value)
            }
            Ok(_) => None,
            Err(error) => {
                log::warn!(
                    "Puma-Lib inference-settings serialization failed for '{}': {}",
                    requested_model_id,
                    error
                );
                None
            }
        }
    }

    async fn resolve_puma_lib_full_package_facts(
        api: &Arc<pumas_library::PumasApi>,
        model_id: &str,
    ) -> Option<serde_json::Value> {
        match api.resolve_model_package_facts(model_id).await {
            Ok(package_facts) => match serde_json::to_value(&package_facts) {
                Ok(value) => Some(value),
                Err(error) => {
                    log::warn!(
                        "Puma-Lib package-facts serialization failed for '{}': {}",
                        model_id,
                        error
                    );
                    None
                }
            },
            Err(error) => {
                log::warn!(
                    "Puma-Lib package-facts lookup failed for '{}': {}",
                    model_id,
                    error
                );
                None
            }
        }
    }

    pub(super) async fn execute_puma_lib(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut model_path =
            Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                .unwrap_or_default();
        let mut model_id =
            Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"]);
        let mut model_type =
            Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"]);
        let mut task_type_primary = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        );
        let backend_key =
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"]);
        let mut recommended_backend = Self::read_optional_input_string_aliases(
            inputs,
            &["recommended_backend", "recommendedBackend"],
        );
        let mut hydrated_inference_settings = None;
        let mut resolved_from_pumas = false;
        let mut resolved_model_package_facts = None;

        let requested_model_id = model_id.clone();
        if let Some(requested_model_id) = requested_model_id.as_deref() {
            if let Some(selector_access) =
                extensions.get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            {
                match Self::resolve_puma_lib_selected_detail(&selector_access, requested_model_id)
                    .await
                {
                    Ok(Some(detail)) => {
                        resolved_from_pumas = true;
                        hydrated_inference_settings =
                            Self::selected_detail_inference_settings_value(
                                &detail,
                                requested_model_id,
                            );
                        Self::apply_puma_lib_selected_detail(
                            detail,
                            requested_model_id,
                            &mut model_path,
                            &mut model_id,
                            &mut model_type,
                            &mut task_type_primary,
                            &mut recommended_backend,
                        );
                    }
                    Ok(None) => {
                        log::warn!(
                            "Puma-Lib selected detail for '{}' was not found during workflow execution; using saved node data",
                            requested_model_id
                        );
                    }
                    Err(error) => {
                        log::warn!(
                            "Puma-Lib selected-detail lookup failed during workflow execution: {}; using saved node data",
                            error
                        );
                    }
                }
            }
        }

        if resolved_from_pumas {
            if let (Some(api), Some(model_id)) = (
                extensions.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API),
                model_id.as_deref(),
            ) {
                resolved_model_package_facts =
                    Self::resolve_puma_lib_full_package_facts(&api, model_id).await;
            }
        } else if let Some(api) =
            extensions.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
        {
            match Self::resolve_puma_lib_model_record(&api, requested_model_id.as_deref()).await {
                Ok(Some(model)) => {
                    resolved_from_pumas = true;
                    model_id = Some(model.id.clone());
                    if !model.path.trim().is_empty() {
                        model_path = model.path.clone();
                    }
                    if model_type
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        model_type = Some(model.model_type.clone());
                    }

                    match api.resolve_model_execution_descriptor(&model.id).await {
                        Ok(descriptor) => {
                            if !descriptor.entry_path.trim().is_empty() {
                                model_path = descriptor.entry_path;
                            }
                            if !descriptor.model_type.trim().is_empty() {
                                model_type = Some(descriptor.model_type);
                            }
                            let task = descriptor.task_type_primary.trim();
                            if !task.is_empty() && task != "unknown" {
                                task_type_primary = Some(task.to_string());
                            }
                            if let Some(backend) = descriptor
                                .recommended_backend
                                .as_deref()
                                .map(str::trim)
                                .filter(|backend| !backend.is_empty())
                            {
                                recommended_backend = Some(backend.to_string());
                            }
                        }
                        Err(error) => {
                            log::warn!(
                                "Puma-Lib execution descriptor lookup failed for '{}': {}",
                                model.id,
                                error
                            );
                        }
                    }

                    resolved_model_package_facts =
                        Self::resolve_puma_lib_full_package_facts(&api, &model.id).await;
                }
                Ok(None) => {
                    if let Some(model_id) = requested_model_id.as_deref() {
                        log::warn!(
                            "Puma-Lib model '{}' was not found during workflow execution; using saved node data",
                            model_id
                        );
                    }
                }
                Err(error) => {
                    log::warn!(
                        "Puma-Lib lookup failed during workflow execution: {}; using saved node data",
                        error,
                    );
                }
            }
        }

        let inference_settings = hydrated_inference_settings
            .or_else(|| {
                Self::read_optional_input_value_aliases(
                    inputs,
                    &["inference_settings", "inferenceSettings"],
                )
                .filter(|value| value.is_array())
            })
            .unwrap_or_else(|| serde_json::json!([]));

        let mut outputs = HashMap::new();
        outputs.insert(
            "model_path".to_string(),
            serde_json::json!(model_path.clone()),
        );
        outputs.insert("inference_settings".to_string(), inference_settings);
        let mut pumas_model_ref = serde_json::Map::new();
        pumas_model_ref.insert("source".to_string(), serde_json::json!("puma-lib"));
        pumas_model_ref.insert(
            "status".to_string(),
            serde_json::json!(if resolved_from_pumas {
                "resolved"
            } else if model_id.is_some() {
                "identity_unverified"
            } else {
                "path_only"
            }),
        );
        pumas_model_ref.insert("model_path".to_string(), serde_json::json!(model_path));
        for (key, value) in [
            ("model_id", model_id.as_deref()),
            ("model_type", model_type.as_deref()),
            ("task_type_primary", task_type_primary.as_deref()),
            ("backend_key", backend_key.as_deref()),
            ("recommended_backend", recommended_backend.as_deref()),
        ] {
            if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
                pumas_model_ref.insert(key.to_string(), serde_json::json!(value));
            }
        }
        outputs.insert(
            "pumas_model_ref".to_string(),
            serde_json::Value::Object(pumas_model_ref),
        );
        if let Some(resolved_model_package_facts) = resolved_model_package_facts {
            outputs.insert(
                "resolved_model_package_facts".to_string(),
                resolved_model_package_facts,
            );
        }
        Self::insert_puma_lib_output_string(&mut outputs, "model_id", model_id);
        Self::insert_puma_lib_output_string(&mut outputs, "model_type", model_type);
        Self::insert_puma_lib_output_string(&mut outputs, "task_type_primary", task_type_primary);
        Self::insert_puma_lib_output_string(&mut outputs, "backend_key", backend_key);
        Self::insert_puma_lib_output_string(
            &mut outputs,
            "recommended_backend",
            recommended_backend,
        );

        if let Some(selected_binding_ids) = Self::read_optional_input_value_aliases(
            inputs,
            &["selected_binding_ids", "selectedBindingIds"],
        )
        .filter(|value| value.is_array())
        {
            outputs.insert("selected_binding_ids".to_string(), selected_binding_ids);
        }
        if let Some(platform_context) = Self::read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        )
        .filter(|value| value.is_object())
        {
            outputs.insert("platform_context".to_string(), platform_context);
        }
        if let Some(dependency_bindings) = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_bindings", "dependencyBindings"],
        )
        .filter(|value| value.is_array())
        {
            outputs.insert("dependency_bindings".to_string(), dependency_bindings);
        }
        if let Some(dependency_requirements) = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_requirements", "dependencyRequirements"],
        )
        .filter(|value| value.is_object())
        {
            outputs.insert(
                "dependency_requirements".to_string(),
                dependency_requirements,
            );
        }
        Self::insert_puma_lib_output_string(
            &mut outputs,
            "dependency_requirements_id",
            Self::read_optional_input_string_aliases(
                inputs,
                &["dependency_requirements_id", "dependencyRequirementsId"],
            ),
        );

        log::debug!("PumaLib: providing model path '{}'", model_path);
        Ok(outputs)
    }
}

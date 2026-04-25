use super::*;

impl TauriTaskExecutor {
    pub(super) fn puma_lib_task_type_from_pipeline_tag(pipeline_tag: &str) -> String {
        match pipeline_tag.trim().to_lowercase().as_str() {
            "text-to-audio" | "text-to-speech" => "text-to-audio".to_string(),
            "automatic-speech-recognition" => "audio-to-text".to_string(),
            "text-to-image" | "image-to-image" => "text-to-image".to_string(),
            "image-classification" | "object-detection" | "image-to-text" => {
                "image-to-text".to_string()
            }
            "feature-extraction" | "sentence-similarity" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    pub(super) fn puma_lib_metadata_string(
        metadata: &serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
    ) -> Option<String> {
        keys.iter().find_map(|key| {
            metadata
                .get(*key)
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
    }

    pub(super) fn insert_puma_lib_output_string(
        outputs: &mut HashMap<String, serde_json::Value>,
        key: &str,
        value: Option<String>,
    ) {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            outputs.insert(key.to_string(), serde_json::json!(value));
        }
    }

    fn normalized_puma_lib_model_name(value: &str) -> String {
        value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    }

    fn puma_lib_record_matches_name(record: &pumas_library::ModelRecord, requested: &str) -> bool {
        let requested = Self::normalized_puma_lib_model_name(requested);
        if requested.is_empty() {
            return false;
        }

        [
            record.id.as_str(),
            record.official_name.as_str(),
            record.cleaned_name.as_str(),
        ]
        .into_iter()
        .any(|candidate| Self::normalized_puma_lib_model_name(candidate) == requested)
    }

    async fn find_puma_lib_model_by_name(
        api: &Arc<pumas_library::PumasApi>,
        model_name: &str,
    ) -> std::result::Result<Option<pumas_library::ModelRecord>, String> {
        let models = api
            .list_models()
            .await
            .map_err(|error| format!("Failed to list Puma-Lib models: {error}"))?;
        Ok(models
            .into_iter()
            .find(|record| Self::puma_lib_record_matches_name(record, model_name)))
    }

    async fn resolve_puma_lib_model_record(
        api: &Arc<pumas_library::PumasApi>,
        model_id: Option<&str>,
        model_name: Option<&str>,
    ) -> std::result::Result<Option<pumas_library::ModelRecord>, String> {
        if let Some(model_id) = model_id {
            return api
                .get_model(model_id)
                .await
                .map_err(|error| format!("Failed to query Puma-Lib model '{model_id}': {error}"));
        }

        if let Some(model_name) = model_name {
            return Self::find_puma_lib_model_by_name(api, model_name).await;
        }

        Ok(None)
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
        let model_name =
            Self::read_optional_input_string_aliases(inputs, &["model_name", "modelName"]);
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

        if let Some(api) = extensions.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
        {
            let requested_model_id = model_id.clone();
            match Self::resolve_puma_lib_model_record(
                &api,
                requested_model_id.as_deref(),
                model_name.as_deref(),
            )
            .await
            {
                Ok(Some(model)) => {
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

                    if let Some(metadata) = model.metadata.as_object() {
                        if task_type_primary
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                        {
                            task_type_primary = Self::puma_lib_metadata_string(
                                metadata,
                                &[
                                    "task_type_primary",
                                    "taskTypePrimary",
                                    "task_type",
                                    "taskType",
                                ],
                            )
                            .or_else(|| {
                                Self::puma_lib_metadata_string(
                                    metadata,
                                    &["pipeline_tag", "pipelineTag"],
                                )
                                .map(|value| Self::puma_lib_task_type_from_pipeline_tag(&value))
                            });
                        }

                        if recommended_backend
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                        {
                            recommended_backend = Self::puma_lib_metadata_string(
                                metadata,
                                &["recommended_backend", "recommendedBackend"],
                            );
                        }
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
                        }
                        Err(error) => {
                            log::warn!(
                                "Puma-Lib execution descriptor lookup failed for '{}': {}",
                                model.id,
                                error
                            );
                        }
                    }
                }
                Ok(None) => {
                    if let Some(model_id) = requested_model_id.as_deref() {
                        log::warn!(
                            "Puma-Lib model '{}' was not found during workflow execution; using saved node data",
                            model_id
                        );
                    } else if let Some(model_name) = model_name.as_deref() {
                        log::warn!(
                            "Puma-Lib model named '{}' was not found during workflow execution; using saved node data",
                            model_name
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

        let inference_settings = Self::read_optional_input_value_aliases(
            inputs,
            &["inference_settings", "inferenceSettings"],
        )
        .filter(|value| value.is_array())
        .unwrap_or_else(|| serde_json::json!([]));

        let mut outputs = HashMap::new();
        outputs.insert("model_path".to_string(), serde_json::json!(model_path));
        outputs.insert("inference_settings".to_string(), inference_settings);
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

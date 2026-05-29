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

    fn read_puma_lib_model_ref(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        Self::read_optional_input_value_aliases(inputs, &["pumas_model_ref", "pumasModelRef"])
            .filter(|value| value.is_object())
    }

    fn model_id_from_pumas_model_ref(value: &serde_json::Value) -> Option<String> {
        value
            .get("model_id")
            .or_else(|| value.get("modelId"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
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
        detail: &PumasSelectedModelDetail,
        requested_model_id: &str,
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

    fn selected_detail_model_ref_value(
        detail: &PumasSelectedModelDetail,
    ) -> Option<serde_json::Value> {
        detail
            .package_summary_result
            .as_ref()
            .and_then(|result| result.summary.as_ref())
            .and_then(|summary| serde_json::to_value(&summary.model_ref).ok())
            .or_else(|| {
                detail
                    .selector_row
                    .as_ref()
                    .and_then(|row| serde_json::to_value(&row.model_ref).ok())
            })
    }

    pub(super) async fn execute_puma_lib(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut pumas_model_ref = Self::read_puma_lib_model_ref(inputs);
        let mut model_id =
            Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"]).or_else(
                || {
                    pumas_model_ref
                        .as_ref()
                        .and_then(Self::model_id_from_pumas_model_ref)
                },
            );
        let mut model_type =
            Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"]);
        let mut task_type_primary = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        );
        let mut recommended_backend = Self::read_optional_input_string_aliases(
            inputs,
            &["recommended_backend", "recommendedBackend"],
        );

        let requested_model_id = model_id.clone();
        if let Some(requested_model_id) = requested_model_id.as_deref() {
            if let Some(selector_access) =
                extensions.get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            {
                match Self::resolve_puma_lib_selected_detail(&selector_access, requested_model_id)
                    .await
                {
                    Ok(Some(detail)) => {
                        pumas_model_ref = Self::selected_detail_model_ref_value(&detail);
                        Self::apply_puma_lib_selected_detail(
                            &detail,
                            requested_model_id,
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
            } else {
                log::warn!(
                    "Puma-Lib selector access is not available for '{}' during workflow execution; using saved node data",
                    requested_model_id
                );
            }
        }

        let mut outputs = HashMap::new();
        if pumas_model_ref.is_none() {
            if let Some(model_id) = model_id.as_deref().filter(|value| !value.trim().is_empty()) {
                pumas_model_ref = Some(serde_json::json!({ "model_id": model_id }));
            }
        }
        if let Some(pumas_model_ref) = pumas_model_ref {
            outputs.insert("pumas_model_ref".to_string(), pumas_model_ref);
        }
        Self::insert_puma_lib_output_string(&mut outputs, "model_id", model_id);
        Self::insert_puma_lib_output_string(&mut outputs, "model_type", model_type);
        Self::insert_puma_lib_output_string(&mut outputs, "task_type_primary", task_type_primary);
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

        log::debug!(
            "PumaLib: providing Pumas model reference for '{}'",
            outputs
                .get("model_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
        );
        Ok(outputs)
    }
}

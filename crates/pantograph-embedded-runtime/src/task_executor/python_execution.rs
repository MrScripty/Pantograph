use super::*;

impl TauriTaskExecutor {
    pub(super) fn collect_model_ref_env_ids(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let model_ref = inputs.get("model_ref");
        let Some(bindings) = model_ref
            .and_then(|v| v.get("dependencyBindings"))
            .and_then(|v| v.as_array())
        else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for binding in bindings {
            let env_id = binding
                .get("envId")
                .and_then(|v| v.as_str())
                .or_else(|| binding.get("env_id").and_then(|v| v.as_str()));
            if let Some(env_id) = env_id {
                let trimmed = env_id.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    pub(super) fn collect_environment_ref_env_ids(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let environment_ref =
            Self::read_optional_input_value_aliases(inputs, &["environment_ref", "environmentRef"]);
        let Some(environment_ref) = environment_ref else {
            return Vec::new();
        };

        let mut out = Vec::new();
        if let Some(env_id) = environment_ref
            .get("env_id")
            .and_then(|v| v.as_str())
            .or_else(|| environment_ref.get("envId").and_then(|v| v.as_str()))
        {
            let trimmed = env_id.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
        if let Some(env_ids) = environment_ref
            .get("env_ids")
            .and_then(|v| v.as_array())
            .or_else(|| environment_ref.get("envIds").and_then(|v| v.as_array()))
        {
            for value in env_ids {
                if let Some(env_id) = value.as_str() {
                    let trimmed = env_id.trim();
                    if !trimmed.is_empty() {
                        out.push(trimmed.to_string());
                    }
                }
            }
        }

        out.sort();
        out.dedup();
        out
    }

    pub(super) fn collect_runtime_env_ids(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let mut out = Self::collect_model_ref_env_ids(inputs);
        out.extend(Self::collect_environment_ref_env_ids(inputs));
        out.sort();
        out.dedup();
        out
    }

    pub(super) fn python_runtime_recorder(
        extensions: &ExecutorExtensions,
    ) -> Option<Arc<PythonRuntimeExecutionRecorder>> {
        extensions
            .get::<Arc<PythonRuntimeExecutionRecorder>>(
                runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
            )
            .cloned()
    }

    pub(super) fn python_runtime_backend_id(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> String {
        if let Some(backend_key) =
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
                .and_then(|value| Self::canonical_backend_key(Some(&value)))
        {
            return backend_key;
        }

        if let Some(engine) = inputs
            .get("model_ref")
            .and_then(|value| value.get("engine"))
            .and_then(|value| value.as_str())
            .and_then(|value| Self::canonical_backend_key(Some(value)))
        {
            return engine;
        }

        match node_type {
            "onnx-inference" => "onnx-runtime".to_string(),
            "audio-generation" => "stable_audio".to_string(),
            _ => "pytorch".to_string(),
        }
    }

    pub(super) fn python_runtime_model_target(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        inputs
            .get("model_ref")
            .and_then(|value| value.get("modelPath"))
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
    }

    pub(super) fn python_runtime_instance_id(runtime_id: &str, env_ids: &[String]) -> String {
        if env_ids.is_empty() {
            return format!("python-runtime:{}:default", runtime_id);
        }

        if env_ids.len() == 1 {
            return format!(
                "python-runtime:{}:{}",
                runtime_id,
                Self::sanitize_key_component(&env_ids[0])
            );
        }

        let env_material = env_ids.join("|");
        format!(
            "python-runtime:{}:{}",
            runtime_id,
            Self::stable_hash_hex(&env_material)
        )
    }

    pub(super) fn python_runtime_execution_metadata(
        node_type: &str,
        request: &PythonNodeExecutionRequest,
        runtime_reused: bool,
    ) -> PythonRuntimeExecutionMetadata {
        let runtime_id = Self::python_runtime_backend_id(node_type, &request.inputs);
        PythonRuntimeExecutionMetadata {
            snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some(runtime_id.clone()),
                runtime_instance_id: Some(Self::python_runtime_instance_id(
                    &runtime_id,
                    &request.env_ids,
                )),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(runtime_reused),
                lifecycle_decision_reason: None,
                active: true,
                last_error: None,
            },
            model_target: Self::python_runtime_model_target(&request.inputs),
            health_assessment: None,
        }
    }

    pub(super) fn apply_inference_setting_defaults(
        inputs: &mut HashMap<String, serde_json::Value>,
    ) {
        let schema = inputs
            .get("inference_settings")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        for parameter in &schema {
            let Some(key) = parameter.get("key").and_then(|value| value.as_str()) else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }

            let has_non_null_value = inputs.get(key).is_some_and(|value| !value.is_null());
            let candidate_value = if has_non_null_value {
                inputs.get(key).cloned()
            } else {
                parameter.get("default").cloned()
            };
            let Some(raw_value) = candidate_value else {
                continue;
            };
            let resolved_value =
                Self::resolve_inference_setting_runtime_value(parameter, raw_value);
            if resolved_value.is_null() {
                continue;
            }
            inputs.insert(key.to_string(), resolved_value);
        }
    }

    pub(super) fn promote_runtime_metadata(inputs: &mut HashMap<String, serde_json::Value>) {
        for key in ["task_type_primary", "model_type", "backend_key"] {
            let nested = inputs.get("_data").and_then(|data| data.get(key)).cloned();
            let Some(value) = nested.or_else(|| Self::read_optional_input_value(inputs, key))
            else {
                continue;
            };
            if value.is_null() {
                continue;
            }

            let should_override = match inputs.get(key) {
                None => true,
                Some(existing) if existing.is_null() => true,
                Some(existing) if existing.as_str() == Some("unknown") => true,
                Some(existing) if existing.as_str() == Some("") => true,
                _ => false,
            };

            if should_override {
                inputs.insert(key.to_string(), value);
            }
        }
    }

    pub(super) fn resolve_inference_setting_runtime_value(
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

    pub(super) fn read_optional_input_string(
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

    pub(super) fn read_optional_input_value(
        inputs: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<serde_json::Value> {
        inputs
            .get(key)
            .cloned()
            .or_else(|| inputs.get("_data").and_then(|d| d.get(key)).cloned())
    }

    pub(super) fn read_optional_input_string_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<String> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_string(inputs, key))
    }

    pub(super) fn read_optional_input_value_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<serde_json::Value> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_value(inputs, key))
    }

    pub(super) async fn execute_python_node(
        &self,
        task_id: &str,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut runtime_inputs = inputs.clone();
        Self::apply_inference_setting_defaults(&mut runtime_inputs);
        Self::promote_runtime_metadata(&mut runtime_inputs);
        if let Some(model_ref) = self
            .enforce_dependency_preflight(node_type, inputs, extensions)
            .await?
        {
            let value = serde_json::to_value(model_ref).map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to serialize resolved model_ref for python runtime adapter: {}",
                    err
                ))
            })?;
            runtime_inputs.insert("model_ref".to_string(), value);
        }

        let request = PythonNodeExecutionRequest {
            node_type: node_type.to_string(),
            inputs: runtime_inputs.clone(),
            env_ids: Self::collect_runtime_env_ids(&runtime_inputs),
        };
        let recorder = Self::python_runtime_recorder(extensions);
        let mut runtime_metadata =
            Self::python_runtime_execution_metadata(node_type, &request, false);
        runtime_metadata.snapshot.lifecycle_decision_reason = runtime_metadata
            .snapshot
            .normalized_lifecycle_decision_reason();

        let streamed_any = Arc::new(AtomicBool::new(false));
        let stream_handler: Option<PythonStreamHandler> = Self::resolve_stream_target(extensions)
            .map(|(sink, execution_id)| {
                let streamed_any = streamed_any.clone();
                let task_id = task_id.to_string();
                Arc::new(move |chunk: serde_json::Value| {
                    streamed_any.store(true, Ordering::Relaxed);
                    let _ = sink.send(WorkflowEvent::task_stream(
                        &task_id,
                        &execution_id,
                        "stream",
                        chunk,
                    ));
                }) as PythonStreamHandler
            });

        let outputs = self
            .python_runtime
            .execute_node_with_stream(request, stream_handler)
            .await
            .map_err(|error| {
                if let Some(recorder) = recorder.as_ref() {
                    let mut failed = runtime_metadata.clone();
                    let previous_failures = recorder.previous_consecutive_failures(
                        failed.snapshot.runtime_instance_id.as_deref(),
                    );
                    failed.snapshot.active = false;
                    failed.snapshot.last_error = Some(error.clone());
                    failed.snapshot.lifecycle_decision_reason =
                        failed.snapshot.normalized_lifecycle_decision_reason();
                    failed.health_assessment = Some(failed_runtime_health_assessment(
                        error.clone(),
                        previous_failures,
                        Self::PYTHON_RUNTIME_FAILURE_THRESHOLD,
                    ));
                    recorder.record(failed);
                }
                NodeEngineError::ExecutionFailed(error)
            })?;
        if let Some(recorder) = recorder.as_ref() {
            runtime_metadata.snapshot.active = false;
            recorder.record(runtime_metadata);
        }
        if !streamed_any.load(Ordering::Relaxed) && Self::supports_buffered_stream_replay(node_type)
        {
            Self::emit_python_stream_events(task_id, &outputs, extensions);
        }
        Ok(outputs)
    }

    pub(super) fn supports_buffered_stream_replay(node_type: &str) -> bool {
        node_type != "audio-generation"
    }

    pub(super) fn resolve_stream_target(
        extensions: &ExecutorExtensions,
    ) -> Option<(Arc<dyn EventSink>, String)> {
        let sink = extensions
            .get::<Arc<dyn EventSink>>(runtime_extension_keys::EVENT_SINK)?
            .clone();
        let execution_id = extensions
            .get::<String>(runtime_extension_keys::EXECUTION_ID)
            .cloned()
            .unwrap_or_default();
        Some((sink, execution_id))
    }

    pub(super) fn emit_python_stream_events(
        task_id: &str,
        outputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) {
        let Some(stream_payload) = outputs.get("stream") else {
            return;
        };
        let Some((sink, execution_id)) = Self::resolve_stream_target(extensions) else {
            return;
        };

        let send_stream = |chunk: serde_json::Value| {
            let _ = sink.send(WorkflowEvent::task_stream(
                task_id,
                &execution_id,
                "stream",
                chunk,
            ));
        };

        match stream_payload {
            serde_json::Value::Null => {}
            serde_json::Value::Array(items) => {
                for item in items {
                    send_stream(item.clone());
                }
            }
            other => send_stream(other.clone()),
        }
    }
}

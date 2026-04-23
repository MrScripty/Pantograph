use super::*;

impl TauriTaskExecutor {
    pub(super) fn parse_requirements_fallback(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<node_engine::ModelDependencyRequirements> {
        let raw = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_requirements", "dependencyRequirements"],
        )?;
        serde_json::from_value(raw).ok()
    }

    pub(super) fn read_input_dependency_override_patches(
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

    pub(super) fn fallback_platform_context_from_key(
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

    pub(super) fn read_input_selected_binding_ids(
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

    pub(super) fn infer_task_type_primary(
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
        if node_type == "diffusion-inference" {
            return "text-to-image".to_string();
        }

        match model_type.as_str() {
            "diffusion" => "text-to-image".to_string(),
            "vision" => "image-to-text".to_string(),
            "embedding" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    pub(super) fn infer_backend_key(node_type: &str) -> Option<String> {
        match node_type {
            "audio-generation" => Some("stable_audio".to_string()),
            "pytorch-inference" => Some("pytorch".to_string()),
            // Leave diffusion unspecified when the graph does not provide a
            // concrete backend so Pumas can apply the model's recommended
            // execution profile.
            "diffusion-inference" => None,
            "onnx-inference" => Some("onnx-runtime".to_string()),
            _ => Some("pytorch".to_string()),
        }
    }

    pub(super) fn preferred_backend_key(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        requirements: Option<&ModelDependencyRequirements>,
    ) -> Option<String> {
        if node_type == "diffusion-inference" {
            if let Some(backend) = Self::canonical_backend_key(
                Self::read_optional_input_string_aliases(
                    inputs,
                    &["recommended_backend", "recommendedBackend"],
                )
                .as_deref(),
            ) {
                return Some(backend);
            }
        }

        Self::canonical_backend_key(
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
                .as_deref(),
        )
        .or_else(|| {
            Self::canonical_backend_key(
                requirements.as_ref().and_then(|r| r.backend_key.as_deref()),
            )
        })
    }

    pub(super) fn build_model_dependency_request(
        node_type: &str,
        model_path: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> ModelDependencyRequest {
        let requirements = Self::parse_requirements_fallback(inputs);
        let backend_key = Self::preferred_backend_key(node_type, inputs, requirements.as_ref())
            .or_else(|| Self::infer_backend_key(node_type));

        let task_type_primary = Self::infer_task_type_primary(node_type, inputs);
        let model_id = Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"])
            .or_else(|| requirements.as_ref().map(|r| r.model_id.clone()));
        let platform_context = Self::read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        )
        .or_else(|| {
            requirements
                .as_ref()
                .and_then(|r| Self::fallback_platform_context_from_key(&r.platform_key))
        });

        let mut selected_binding_ids = Self::read_input_selected_binding_ids(inputs);
        if selected_binding_ids.is_empty() {
            if let Some(req) = &requirements {
                selected_binding_ids = req.selected_binding_ids.clone();
            }
        }

        ModelDependencyRequest {
            node_type: node_type.to_string(),
            model_path: model_path.to_string(),
            model_id,
            model_type: Self::read_optional_input_string_aliases(
                inputs,
                &["model_type", "modelType"],
            ),
            task_type_primary: Some(task_type_primary),
            backend_key,
            platform_context,
            selected_binding_ids,
            dependency_override_patches: Self::read_input_dependency_override_patches(inputs),
        }
    }

    pub(super) fn dependency_mode(inputs: &HashMap<String, serde_json::Value>) -> String {
        Self::read_optional_input_string_aliases(inputs, &["mode"])
            .map(|mode| mode.trim().to_lowercase())
            .filter(|mode| mode == "auto" || mode == "manual")
            .unwrap_or_else(|| "auto".to_string())
    }

    pub(super) fn allows_local_python_fallback(status: &ModelDependencyStatus) -> bool {
        if status.state == DependencyState::Unresolved
            && status.code.as_deref() == Some("no_dependency_bindings")
        {
            return true;
        }

        status.state == DependencyState::Missing
            && !status.bindings.is_empty()
            && status.bindings.iter().all(|binding| {
                binding.state == DependencyState::Missing
                    && binding.code.as_deref() == Some("requirements_missing")
                    && binding.failed_requirements.is_empty()
            })
    }

    pub(super) fn canonical_requirement_fingerprint(
        requirements: &node_engine::ModelDependencyRequirements,
    ) -> String {
        let mut rows = Vec::new();
        let selected = requirements
            .selected_binding_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        for binding in &requirements.bindings {
            if !selected.is_empty() && !selected.contains(&binding.binding_id) {
                continue;
            }
            for req in &binding.requirements {
                rows.push(format!(
                    "{}|{}|{}|{}",
                    binding.binding_id, req.kind, req.name, req.exact_pin
                ));
            }
        }
        rows.sort();
        rows.join(";")
    }

    pub(super) fn sanitize_key_component(raw: &str) -> String {
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

    pub(super) fn dependency_env_store_root() -> PathBuf {
        let base = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        base.join("pantograph").join("dependency_envs")
    }

    pub(super) fn stable_hash_hex(value: &str) -> String {
        let mut digest = Self::FNV64_OFFSET_BASIS;
        for byte in value.as_bytes() {
            digest ^= *byte as u64;
            digest = digest.wrapping_mul(Self::FNV64_PRIME);
        }
        format!("{:016x}", digest)
    }

    pub(super) fn resolve_environment_ref(
        status: &ModelDependencyStatus,
    ) -> std::result::Result<serde_json::Value, String> {
        let requirements = &status.requirements;
        let selected = if requirements.selected_binding_ids.is_empty() {
            requirements
                .bindings
                .iter()
                .map(|b| b.binding_id.clone())
                .collect::<Vec<_>>()
        } else {
            requirements.selected_binding_ids.clone()
        };

        let env_ids = status
            .bindings
            .iter()
            .filter_map(|row| row.env_id.clone())
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        let primary_env_id = env_ids.first().cloned();

        let mut selected_bindings = requirements
            .bindings
            .iter()
            .filter(|binding| selected.contains(&binding.binding_id))
            .collect::<Vec<_>>();
        if selected_bindings.is_empty() {
            selected_bindings = requirements.bindings.iter().collect::<Vec<_>>();
        }

        let environment_kind = selected_bindings
            .iter()
            .find_map(|binding| binding.environment_kind.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let python_override = selected_bindings
            .iter()
            .find_map(|binding| binding.python_executable_override.clone());

        let state_value = serde_json::to_value(&status.state).map_err(|err| {
            format!(
                "Failed to serialize dependency status state for environment_ref: {}",
                err
            )
        })?;
        let state = state_value
            .as_str()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unresolved".to_string());

        let python_executable = if let Some(override_path) = python_override {
            Some(override_path)
        } else if !env_ids.is_empty()
            && (environment_kind == "python" || environment_kind == "python-venv")
        {
            crate::python_runtime::resolve_python_executable_for_env_ids(&env_ids)
                .ok()
                .map(|path| path.to_string_lossy().to_string())
        } else {
            None
        };

        let backend_key = requirements
            .backend_key
            .clone()
            .unwrap_or_else(|| "any".to_string());
        let requirements_fingerprint = Self::canonical_requirement_fingerprint(requirements);
        let key_material = format!(
            "{}|{}|{}|{}",
            primary_env_id.clone().unwrap_or_else(|| "none".to_string()),
            requirements.platform_key,
            backend_key,
            requirements_fingerprint
        );
        let environment_key =
            Self::sanitize_key_component(&format!("v1:{}", Self::stable_hash_hex(&key_material)));

        let manifest_dir = Self::dependency_env_store_root()
            .join(environment_kind.replace(':', "_"))
            .join(&environment_key);
        std::fs::create_dir_all(&manifest_dir).map_err(|err| {
            format!(
                "Failed to create dependency environment manifest directory '{}': {}",
                manifest_dir.display(),
                err
            )
        })?;
        let manifest_path = manifest_dir.join("manifest.json");
        let manifest = serde_json::json!({
            "contract_version": 1,
            "generated_at": Utc::now().to_rfc3339(),
            "environment_key": environment_key,
            "environment_kind": environment_kind,
            "env_id": primary_env_id,
            "env_ids": env_ids,
            "python_executable": python_executable,
            "state": state,
            "requirements_fingerprint": requirements_fingerprint,
            "platform_key": requirements.platform_key,
            "backend_key": requirements.backend_key,
            "selected_binding_ids": requirements.selected_binding_ids,
            "requirements": requirements,
            "status": status,
        });
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).map_err(|err| {
                format!(
                    "Failed to serialize dependency environment manifest '{}': {}",
                    manifest_path.display(),
                    err
                )
            })?,
        )
        .map_err(|err| {
            format!(
                "Failed to write dependency environment manifest '{}': {}",
                manifest_path.display(),
                err
            )
        })?;

        Ok(serde_json::json!({
            "contract_version": 1,
            "environment_key": environment_key,
            "environment_kind": environment_kind,
            "env_id": manifest["env_id"],
            "env_ids": manifest["env_ids"],
            "python_executable": python_executable,
            "state": state,
            "requirements_fingerprint": requirements_fingerprint,
            "platform_key": requirements.platform_key,
            "backend_key": requirements.backend_key,
            "manifest_path": manifest_path.to_string_lossy().to_string(),
        }))
    }

    pub(super) async fn execute_dependency_environment(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let Some(resolver) = extensions
            .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
        else {
            return Err(NodeEngineError::ExecutionFailed(
                "Dependency environment node requires dependency resolver extension".to_string(),
            ));
        };

        let model_path =
            Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed(
                        "Missing model_path input. Connect Puma-Lib model_path output.".to_string(),
                    )
                })?;
        let mode = Self::dependency_mode(inputs);
        let request =
            Self::build_model_dependency_request("dependency-environment", &model_path, inputs);
        let requirements = resolver
            .resolve_model_dependency_requirements(request.clone())
            .await
            .map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency environment resolve failed: {}",
                    err
                ))
            })?;

        let mut status = resolver
            .check_dependencies(request.clone())
            .await
            .map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency environment check failed: {}",
                    err
                ))
            })?;
        if mode == "auto" && matches!(status.state, DependencyState::Missing) {
            let install = resolver
                .install_dependencies(request)
                .await
                .map_err(|err| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency environment install failed: {}",
                        err
                    ))
                })?;
            status = ModelDependencyStatus {
                state: install.state,
                code: install.code,
                message: install.message,
                requirements: install.requirements,
                bindings: install.bindings,
                checked_at: install.installed_at,
            };
        }

        let ui_state = if mode == "manual"
            && matches!(
                status.state,
                DependencyState::Missing | DependencyState::Unresolved
            ) {
            "needs_user_input".to_string()
        } else {
            serde_json::to_value(&status.state)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unresolved".to_string())
        };
        let environment_ref = Self::resolve_environment_ref(&status).map_err(|err| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency environment failed to emit environment_ref: {}",
                err
            ))
        })?;

        let mut outputs = HashMap::new();
        outputs.insert("environment_ref".to_string(), environment_ref);
        outputs.insert(
            "dependency_requirements".to_string(),
            serde_json::to_value(&requirements).map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to serialize dependency requirements output: {}",
                    err
                ))
            })?,
        );
        outputs.insert(
            "dependency_status".to_string(),
            serde_json::json!({
                "mode": mode,
                "ui_state": ui_state,
                "state": status.state,
                "code": status.code,
                "message": status.message,
                "checked_at": status.checked_at,
                "requirements": status.requirements,
                "bindings": status.bindings,
            }),
        );
        Ok(outputs)
    }

    pub(super) async fn enforce_dependency_preflight(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<Option<node_engine::ModelRefV2>> {
        if node_type != "pytorch-inference"
            && node_type != "diffusion-inference"
            && node_type != "audio-generation"
            && node_type != "onnx-inference"
        {
            return Ok(None);
        }

        let environment_ref =
            Self::read_optional_input_value_aliases(inputs, &["environment_ref", "environmentRef"]);
        let environment_gate_enabled = environment_ref.is_some();
        if let Some(environment_ref) = &environment_ref {
            let state = environment_ref
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("unresolved");
            if state != "ready" {
                let payload = serde_json::json!({
                    "kind": "environment_ref_gate",
                    "node_type": node_type,
                    "state": state,
                    "environment_ref": environment_ref,
                });
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight blocked execution: {}",
                    payload
                )));
            }
        }

        let Some(resolver) = extensions
            .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
        else {
            if environment_gate_enabled {
                return Ok(None);
            }
            return Err(NodeEngineError::ExecutionFailed(
                "Dependency preflight blocked execution: dependency resolver is not configured"
                    .to_string(),
            ));
        };

        let model_path = inputs
            .get("model_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(
                    "Missing model_path input. Connect a Puma-Lib node.".to_string(),
                )
            })?;

        let request = Self::build_model_dependency_request(node_type, model_path, inputs);
        if environment_gate_enabled {
            let resolved = resolver
                .resolve_model_ref(request, None)
                .await
                .map_err(|e| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency preflight failed to resolve model_ref from ready environment_ref: {}",
                        e
                    ))
                })?;
            if let Some(ref model_ref) = resolved {
                model_ref
                    .validate()
                    .map_err(NodeEngineError::ExecutionFailed)?;
            }
            return Ok(resolved);
        }

        let requirements = resolver
            .resolve_model_dependency_requirements(request.clone())
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight requirements resolution failed for '{}': {}",
                    node_type, e
                ))
            })?;

        let status = resolver
            .check_dependencies(request.clone())
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight check failed for '{}': {}",
                    node_type, e
                ))
            })?;

        if Self::allows_local_python_fallback(&status) {
            let resolved = resolver.resolve_model_ref(request, Some(requirements)).await.map_err(
                |e| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency preflight failed to resolve model_ref for local Python fallback: {}",
                        e
                    ))
                },
            )?;
            if let Some(ref model_ref) = resolved {
                model_ref
                    .validate()
                    .map_err(NodeEngineError::ExecutionFailed)?;
            }
            return Ok(resolved);
        }

        if status.state != DependencyState::Ready {
            let payload = serde_json::json!({
                "kind": "dependency_preflight",
                "node_type": node_type,
                "model_path": model_path,
                "validation_state": requirements.validation_state,
                "validation_errors": requirements.validation_errors,
                "selected_binding_ids": requirements.selected_binding_ids,
                "state": status.state,
                "code": status.code,
                "bindings": status.bindings,
                "message": status.message,
            });
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight blocked execution: {}",
                payload
            )));
        }

        let resolved = resolver
            .resolve_model_ref(request, Some(requirements))
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight failed to resolve model_ref: {}",
                    e
                ))
            })?;
        if let Some(ref model_ref) = resolved {
            model_ref
                .validate()
                .map_err(NodeEngineError::ExecutionFailed)?;
        }

        Ok(resolved)
    }
}

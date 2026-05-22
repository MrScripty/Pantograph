use super::*;

mod helpers;

impl TauriTaskExecutor {
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

        let mode = Self::dependency_mode(inputs);
        let request = Self::build_model_dependency_request("dependency-environment", inputs);
        if request
            .model_id
            .as_deref()
            .is_none_or(|model_id| model_id.trim().is_empty())
        {
            return Err(NodeEngineError::ExecutionFailed(
                "Missing pumas_model_ref/model_id input. Connect Puma-Lib pumas_model_ref output."
                    .to_string(),
            ));
        }
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
        if !Self::python_runtime_handles_node(node_type) {
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

        let request = Self::build_model_dependency_request(node_type, inputs);
        let request_model_id = match request.model_id.as_deref() {
            Some(model_id) if !model_id.trim().is_empty() => model_id.to_string(),
            _ => {
                return Err(NodeEngineError::ExecutionFailed(
                    "Missing pumas_model_ref/model_id input. Connect Puma-Lib pumas_model_ref output."
                        .to_string(),
                ));
            }
        };
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

        if status.state != DependencyState::Ready {
            let payload = serde_json::json!({
                "kind": "dependency_preflight",
                "node_type": node_type,
                "model_id": request_model_id,
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

use super::*;

mod helpers;

impl TauriTaskExecutor {
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

use super::*;

mod helpers;

impl TauriTaskExecutor {
    pub(super) async fn enforce_dependency_preflight(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        _extensions: &ExecutorExtensions,
    ) -> Result<()> {
        if !Self::python_runtime_handles_node(node_type) {
            return Ok(());
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

        let mut payload = serde_json::json!({
            "kind": "dependency_preflight_retired",
            "node_type": node_type,
            "environment_ref_gate": environment_gate_enabled,
            "message": "embedded-runtime dependency_preflight is diagnostic-only for retired Python runtime execution paths; runtime execution requires scheduler task state/results and runtime-host readiness",
            "blocked_before": [
                "ModelDependencyResolver",
                "ModelDependencyRequest",
                "ModelRefV2",
                "python_runtime_adapter"
            ],
        });
        if let Some(model_id) = Self::dependency_preflight_model_id(inputs) {
            payload["model_id"] = serde_json::Value::String(model_id);
        }
        Err(NodeEngineError::ExecutionFailed(format!(
            "Dependency preflight blocked execution: {}",
            payload
        )))
    }

    fn dependency_preflight_model_id(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"]).or_else(|| {
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
        })
    }
}

use super::model_dependencies::SharedModelDependencyResolver;
use tauri::State;

fn build_model_dependency_request(
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyRequest, String> {
    fn clean_optional(value: Option<String>) -> Option<String> {
        value.and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    let node_type = node_type.trim().to_string();
    if node_type.is_empty() {
        return Err("node_type is required".to_string());
    }

    let model_path = model_path.trim().to_string();
    if model_path.is_empty() {
        return Err("model_path is required".to_string());
    }

    let mut selected_out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for binding_id in selected_binding_ids.unwrap_or_default() {
        let trimmed = binding_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            selected_out.push(owned);
        }
    }

    Ok(node_engine::ModelDependencyRequest {
        node_type,
        model_path,
        model_id: clean_optional(model_id),
        model_type: clean_optional(model_type),
        task_type_primary: clean_optional(task_type_primary),
        backend_key: clean_optional(backend_key),
        platform_context,
        selected_binding_ids: selected_out,
    })
}

/// Resolve model dependency plan for a model-backed workflow node.
pub async fn resolve_model_dependency_plan(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyPlan, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.resolve_plan_request(request).await
}

/// Check model dependencies for a model-backed workflow node.
pub async fn check_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.check_request(request).await
}

/// Install dependencies for a model-backed workflow node.
pub async fn install_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyInstallResult, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.install_request(request).await
}

/// Read the latest cached dependency status, or run a fresh check if absent.
pub async fn get_model_dependency_status(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    if let Some(cached) = resolver.cached_status(&request).await {
        Ok(cached)
    } else {
        resolver.check_request(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_model_dependency_request_rejects_empty_required_fields() {
        let err = build_model_dependency_request(
            "  ".to_string(),
            "/tmp/model".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("node_type"));

        let err = build_model_dependency_request(
            "pytorch-inference".to_string(),
            " ".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("model_path"));
    }

    #[test]
    fn test_build_model_dependency_request_normalizes_optional_and_bindings() {
        let request = build_model_dependency_request(
            " pytorch-inference ".to_string(),
            " /tmp/model ".to_string(),
            Some(" ".to_string()),
            Some(" diffusion ".to_string()),
            Some(" text-to-image ".to_string()),
            Some(" pytorch ".to_string()),
            None,
            Some(vec![
                " binding-a ".to_string(),
                "".to_string(),
                "binding-a".to_string(),
                "binding-b".to_string(),
            ]),
        )
        .unwrap();

        assert_eq!(request.node_type, "pytorch-inference");
        assert_eq!(request.model_path, "/tmp/model");
        assert_eq!(request.model_id, None);
        assert_eq!(request.model_type.as_deref(), Some("diffusion"));
        assert_eq!(request.task_type_primary.as_deref(), Some("text-to-image"));
        assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
        assert_eq!(
            request.selected_binding_ids,
            vec!["binding-a".to_string(), "binding-b".to_string()]
        );
    }
}

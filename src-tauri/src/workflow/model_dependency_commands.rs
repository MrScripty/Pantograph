use super::model_dependencies::SharedModelDependencyResolver;
use std::sync::Arc;
use tauri::State;

use super::commands::SharedExtensions;

fn build_model_dependency_request(
    request: node_engine::ModelDependencyRequest,
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

    let node_type = request.node_type.trim().to_string();
    if node_type.is_empty() {
        return Err("node_type is required".to_string());
    }

    let model_path = request.model_path.trim().to_string();
    if model_path.is_empty() {
        return Err("model_path is required".to_string());
    }

    let mut selected_out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for binding_id in request.selected_binding_ids {
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
        model_id: clean_optional(request.model_id),
        model_type: clean_optional(request.model_type),
        task_type_primary: clean_optional(request.task_type_primary),
        backend_key: clean_optional(request.backend_key),
        platform_context: request.platform_context,
        selected_binding_ids: selected_out,
        dependency_override_patches: request.dependency_override_patches,
    })
}

/// Resolve model dependency requirements for a model-backed workflow node.
pub async fn resolve_model_dependency_requirements(
    resolver: State<'_, SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyRequirements, String> {
    let request = build_model_dependency_request(request)?;
    resolver.resolve_requirements_request(request).await
}

/// Check model dependencies for a model-backed workflow node.
pub async fn check_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(request)?;
    resolver.check_request(request).await
}

/// Install dependencies for a model-backed workflow node.
pub async fn install_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyInstallResult, String> {
    let request = build_model_dependency_request(request)?;
    resolver.install_request(request).await
}

/// Read the latest cached dependency status, or run a fresh check if absent.
pub async fn get_model_dependency_status(
    resolver: State<'_, SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(request)?;
    if let Some(cached) = resolver.cached_status(&request).await {
        Ok(cached)
    } else {
        resolver.check_request(request).await
    }
}

async fn require_pumas_api(
    extensions: &State<'_, SharedExtensions>,
) -> Result<Arc<pumas_library::PumasApi>, String> {
    let ext = extensions.read().await;
    ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
        .cloned()
        .ok_or_else(|| "Pumas API not available in executor extensions".to_string())
}

/// Return current dependency pin compliance audit report from pumas-library.
pub async fn audit_dependency_pin_compliance(
    extensions: State<'_, SharedExtensions>,
) -> Result<pumas_library::model_library::DependencyPinAuditReport, String> {
    let api = require_pumas_api(&extensions).await?;
    api.audit_dependency_pin_compliance()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_model_dependency_request_rejects_empty_required_fields() {
        let err = build_model_dependency_request(node_engine::ModelDependencyRequest {
            node_type: "  ".to_string(),
            model_path: "/tmp/model".to_string(),
            ..Default::default()
        })
        .unwrap_err();
        assert!(err.contains("node_type"));

        let err = build_model_dependency_request(node_engine::ModelDependencyRequest {
            node_type: "pytorch-inference".to_string(),
            model_path: " ".to_string(),
            ..Default::default()
        })
        .unwrap_err();
        assert!(err.contains("model_path"));
    }

    #[test]
    fn test_build_model_dependency_request_normalizes_optional_and_bindings() {
        let request = build_model_dependency_request(node_engine::ModelDependencyRequest {
            node_type: " pytorch-inference ".to_string(),
            model_path: " /tmp/model ".to_string(),
            model_id: Some(" ".to_string()),
            model_type: Some(" diffusion ".to_string()),
            task_type_primary: Some(" text-to-image ".to_string()),
            backend_key: Some(" pytorch ".to_string()),
            selected_binding_ids: vec![
                " binding-a ".to_string(),
                "".to_string(),
                "binding-a".to_string(),
                "binding-b".to_string(),
            ],
            ..Default::default()
        })
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
        assert!(request.dependency_override_patches.is_empty());
    }

    #[test]
    fn test_dependency_pin_audit_report_serializes_expected_shape() {
        let report = pumas_library::model_library::DependencyPinAuditReport {
            generated_at: "2026-02-27T00:00:00Z".to_string(),
            total_models_scanned: 1,
            total_bindings_scanned: 2,
            issue_count: 1,
            binding_issues: vec![
                pumas_library::model_library::DependencyPinAuditBindingIssue {
                    model_id: "m1".to_string(),
                    binding_id: "b1".to_string(),
                    profile_id: "p1".to_string(),
                    profile_version: 2,
                    binding_kind: "required".to_string(),
                    backend_key: Some("pytorch".to_string()),
                    error_code: "unpinned_dependency".to_string(),
                    message: Some("missing torch".to_string()),
                    missing_pins: vec!["torch".to_string()],
                    required_pins: vec![pumas_library::model_library::ModelDependencyRequiredPin {
                        name: "torch".to_string(),
                        reasons: vec!["backend_required".to_string()],
                    }],
                },
            ],
            profile_issues: Vec::new(),
        };

        let value = serde_json::to_value(report).expect("report should serialize");
        assert!(value.get("generated_at").is_some());
        assert!(value.get("total_models_scanned").is_some());
        assert!(value.get("binding_issues").is_some());
    }
}

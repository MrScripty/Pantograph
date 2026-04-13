use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;

use super::model_dependencies::SharedModelDependencyResolver;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyEnvironmentAction {
    Resolve,
    Check,
    Install,
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEnvironmentActionRequest {
    pub action: DependencyEnvironmentAction,
    #[serde(default)]
    pub mode: Option<String>,
    pub model_path: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    #[serde(default)]
    pub task_type_primary: Option<String>,
    #[serde(default)]
    pub backend_key: Option<String>,
    #[serde(default)]
    pub platform_context: Option<serde_json::Value>,
    #[serde(default)]
    pub selected_binding_ids: Vec<String>,
    #[serde(default)]
    pub dependency_requirements: Option<node_engine::ModelDependencyRequirements>,
    #[serde(default)]
    pub dependency_override_patches: Vec<node_engine::DependencyOverridePatchV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEnvironmentActionResponse {
    pub node_data: Value,
}

pub async fn run_dependency_environment_action(
    resolver: State<'_, SharedModelDependencyResolver>,
    request: DependencyEnvironmentActionRequest,
) -> Result<DependencyEnvironmentActionResponse, String> {
    let mode = normalize_mode(request.mode.as_deref());
    let resolver_request = build_model_dependency_request(&request)?;

    let node_data = match request.action {
        DependencyEnvironmentAction::Resolve => {
            let mut requirements = resolver
                .resolve_requirements_request(resolver_request)
                .await?;
            let selected_binding_ids = effective_selected_binding_ids(
                &request.selected_binding_ids,
                Some(&requirements),
            );
            requirements.selected_binding_ids = selected_binding_ids.clone();
            json!({
                "mode": mode,
                "selected_binding_ids": selected_binding_ids,
                "dependency_requirements": requirements,
            })
        }
        DependencyEnvironmentAction::Check => {
            let status = resolver.check_request(resolver_request).await?;
            build_dependency_environment_node_data(&mode, status)?
        }
        DependencyEnvironmentAction::Install => {
            let result = resolver.install_request(resolver_request).await?;
            let status = node_engine::ModelDependencyStatus {
                state: result.state,
                code: result.code,
                message: result.message,
                requirements: result.requirements,
                bindings: result.bindings,
                checked_at: result.installed_at,
            };
            build_dependency_environment_node_data(&mode, status)?
        }
        DependencyEnvironmentAction::Run => {
            let mut status = resolver.check_request(resolver_request.clone()).await?;
            if mode == "auto" && status.state == node_engine::DependencyState::Missing {
                let _ = resolver.install_request(resolver_request.clone()).await?;
                status = resolver.check_request(resolver_request).await?;
            }
            build_dependency_environment_node_data(&mode, status)?
        }
    };

    Ok(DependencyEnvironmentActionResponse { node_data })
}

fn build_model_dependency_request(
    request: &DependencyEnvironmentActionRequest,
) -> Result<node_engine::ModelDependencyRequest, String> {
    let model_path = request.model_path.trim().to_string();
    if model_path.is_empty() {
        return Err("model_path is required".to_string());
    }

    let requirements = request.dependency_requirements.as_ref();
    let platform_context = request
        .platform_context
        .clone()
        .or_else(|| {
            requirements
                .map(|value| value.platform_key.as_str())
                .and_then(fallback_platform_context_from_key)
        });
    let selected_binding_ids =
        effective_selected_binding_ids(&request.selected_binding_ids, requirements);

    Ok(node_engine::ModelDependencyRequest {
        node_type: "dependency-environment".to_string(),
        model_path,
        model_id: clean_optional(request.model_id.clone())
            .or_else(|| requirements.map(|value| value.model_id.clone())),
        model_type: clean_optional(request.model_type.clone()),
        task_type_primary: clean_optional(request.task_type_primary.clone()),
        backend_key: clean_optional(request.backend_key.clone())
            .or_else(|| requirements.and_then(|value| value.backend_key.clone())),
        platform_context,
        selected_binding_ids,
        dependency_override_patches: request.dependency_override_patches.clone(),
    })
}

fn build_dependency_environment_node_data(
    mode: &str,
    mut status: node_engine::ModelDependencyStatus,
) -> Result<Value, String> {
    let selected_binding_ids =
        effective_selected_binding_ids(&status.requirements.selected_binding_ids, Some(&status.requirements));
    status.requirements.selected_binding_ids = selected_binding_ids.clone();
    let environment_ref = resolve_environment_ref(&status)?;

    Ok(json!({
        "mode": mode,
        "selected_binding_ids": selected_binding_ids,
        "dependency_requirements": status.requirements,
        "dependency_status": build_dependency_status_payload(mode, &status)?,
        "environment_ref": environment_ref,
    }))
}

fn build_dependency_status_payload(
    mode: &str,
    status: &node_engine::ModelDependencyStatus,
) -> Result<Value, String> {
    let state = serde_json::to_value(&status.state)
        .map_err(|error| format!("Failed to serialize dependency status state: {error}"))?;
    let ui_state = if mode == "manual"
        && matches!(
            status.state,
            node_engine::DependencyState::Missing | node_engine::DependencyState::Unresolved
        ) {
        "needs_user_input".to_string()
    } else {
        state
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "unresolved".to_string())
    };

    Ok(json!({
        "mode": mode,
        "ui_state": ui_state,
        "state": status.state,
        "code": status.code,
        "message": status.message,
        "checked_at": status.checked_at,
        "requirements": status.requirements,
        "bindings": status.bindings,
    }))
}

fn resolve_environment_ref(
    status: &node_engine::ModelDependencyStatus,
) -> Result<serde_json::Value, String> {
    let requirements = &status.requirements;
    let selected_binding_ids =
        effective_selected_binding_ids(&requirements.selected_binding_ids, Some(requirements));

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
        .filter(|binding| selected_binding_ids.contains(&binding.binding_id))
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

    let state = serde_json::to_value(&status.state)
        .map_err(|error| format!("Failed to serialize dependency status state: {error}"))?
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "unresolved".to_string());

    let python_executable = python_override;

    let backend_key = requirements
        .backend_key
        .clone()
        .unwrap_or_else(|| "any".to_string());
    let requirements_fingerprint = canonical_requirement_fingerprint(requirements);
    let key_material = format!(
        "{}|{}|{}|{}",
        primary_env_id.clone().unwrap_or_else(|| "none".to_string()),
        requirements.platform_key,
        backend_key,
        requirements_fingerprint
    );
    let environment_key = sanitize_key_component(&format!("v1:{}", stable_hash_hex(&key_material)));

    let manifest_dir = dependency_env_store_root()
        .join(environment_kind.replace(':', "_"))
        .join(&environment_key);
    std::fs::create_dir_all(&manifest_dir).map_err(|error| {
        format!(
            "Failed to create dependency environment manifest directory '{}': {}",
            manifest_dir.display(),
            error
        )
    })?;
    let manifest_path = manifest_dir.join("manifest.json");
    let manifest = json!({
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
        "selected_binding_ids": selected_binding_ids,
        "requirements": requirements,
        "status": status,
    });
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| {
            format!(
                "Failed to serialize dependency environment manifest '{}': {}",
                manifest_path.display(),
                error
            )
        })?,
    )
    .map_err(|error| {
        format!(
            "Failed to write dependency environment manifest '{}': {}",
            manifest_path.display(),
            error
        )
    })?;

    Ok(json!({
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

fn normalize_mode(mode: Option<&str>) -> String {
    mode.map(str::trim)
        .map(str::to_ascii_lowercase)
        .filter(|value| value == "auto" || value == "manual")
        .unwrap_or_else(|| "auto".to_string())
}

fn effective_selected_binding_ids(
    selected_binding_ids: &[String],
    requirements: Option<&node_engine::ModelDependencyRequirements>,
) -> Vec<String> {
    let mut out = sanitize_selected_binding_ids(selected_binding_ids.iter().cloned().collect());
    if out.is_empty() {
        if let Some(requirements) = requirements {
            out = sanitize_selected_binding_ids(requirements.selected_binding_ids.clone());
        }
    }
    if out.is_empty() {
        if let Some(requirements) = requirements {
            out = requirements
                .bindings
                .iter()
                .map(|binding| binding.binding_id.clone())
                .collect();
        }
    }
    out
}

fn sanitize_selected_binding_ids(selected_binding_ids: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for binding_id in selected_binding_ids {
        let trimmed = binding_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            out.push(owned);
        }
    }
    out
}

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

fn fallback_platform_context_from_key(platform_key: &str) -> Option<serde_json::Value> {
    let mut parts = platform_key.splitn(2, '-');
    let os = parts.next()?.trim();
    let arch = parts.next()?.trim();
    if os.is_empty() || arch.is_empty() {
        return None;
    }
    Some(json!({ "os": os, "arch": arch }))
}

fn canonical_requirement_fingerprint(
    requirements: &node_engine::ModelDependencyRequirements,
) -> String {
    let selected = requirements
        .selected_binding_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut rows = Vec::new();
    for binding in &requirements.bindings {
        if !selected.is_empty() && !selected.contains(&binding.binding_id) {
            continue;
        }
        for requirement in &binding.requirements {
            rows.push(format!(
                "{}|{}|{}|{}",
                binding.binding_id, requirement.kind, requirement.name, requirement.exact_pin
            ));
        }
    }
    rows.sort();
    rows.join(";")
}

fn stable_hash_hex(value: &str) -> String {
    const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV64_PRIME: u64 = 0x100000001b3;

    let mut digest = FNV64_OFFSET_BASIS;
    for byte in value.as_bytes() {
        digest ^= *byte as u64;
        digest = digest.wrapping_mul(FNV64_PRIME);
    }
    format!("{digest:016x}")
}

fn sanitize_key_component(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn dependency_env_store_root() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir);
    base.join("pantograph").join("dependency_envs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_requirements() -> node_engine::ModelDependencyRequirements {
        node_engine::ModelDependencyRequirements {
            model_id: "model-a".to_string(),
            platform_key: "linux-x86_64".to_string(),
            backend_key: Some("pytorch".to_string()),
            dependency_contract_version: 1,
            validation_state: node_engine::DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            bindings: vec![
                node_engine::ModelDependencyBinding {
                    binding_id: "binding-a".to_string(),
                    profile_id: "profile-a".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: None,
                    environment_kind: Some("python-venv".to_string()),
                    env_id: Some("env-a".to_string()),
                    python_executable_override: None,
                    validation_state: node_engine::DependencyValidationState::Resolved,
                    validation_errors: Vec::new(),
                    requirements: vec![node_engine::ModelDependencyRequirement {
                        kind: "pip".to_string(),
                        name: "torch".to_string(),
                        exact_pin: "==2.0.0".to_string(),
                        index_url: None,
                        extra_index_urls: Vec::new(),
                        markers: None,
                        python_requires: None,
                        platform_constraints: Vec::new(),
                        hashes: Vec::new(),
                        source: None,
                    }],
                },
                node_engine::ModelDependencyBinding {
                    binding_id: "binding-b".to_string(),
                    profile_id: "profile-b".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: None,
                    environment_kind: Some("python-venv".to_string()),
                    env_id: Some("env-b".to_string()),
                    python_executable_override: None,
                    validation_state: node_engine::DependencyValidationState::Resolved,
                    validation_errors: Vec::new(),
                    requirements: Vec::new(),
                },
            ],
            selected_binding_ids: vec!["binding-b".to_string()],
        }
    }

    #[test]
    fn normalize_mode_accepts_only_supported_values() {
        assert_eq!(normalize_mode(Some("auto")), "auto");
        assert_eq!(normalize_mode(Some("manual")), "manual");
        assert_eq!(normalize_mode(Some("AUTO")), "auto");
        assert_eq!(normalize_mode(Some("other")), "auto");
        assert_eq!(normalize_mode(None), "auto");
    }

    #[test]
    fn effective_selected_binding_ids_prefers_explicit_then_requirements_then_all_bindings() {
        let requirements = sample_requirements();

        assert_eq!(
            effective_selected_binding_ids(&[" explicit ".to_string()], Some(&requirements)),
            vec!["explicit".to_string()]
        );
        assert_eq!(
            effective_selected_binding_ids(&[], Some(&requirements)),
            vec!["binding-b".to_string()]
        );

        let mut requirements_without_selection = requirements;
        requirements_without_selection.selected_binding_ids.clear();
        assert_eq!(
            effective_selected_binding_ids(&[], Some(&requirements_without_selection)),
            vec!["binding-a".to_string(), "binding-b".to_string()]
        );
    }

    #[test]
    fn build_model_dependency_request_uses_requirements_fallbacks() {
        let request = build_model_dependency_request(&DependencyEnvironmentActionRequest {
            action: DependencyEnvironmentAction::Resolve,
            mode: Some("manual".to_string()),
            model_path: "/models/example".to_string(),
            model_id: None,
            model_type: Some("diffusion".to_string()),
            task_type_primary: Some("text-to-image".to_string()),
            backend_key: None,
            platform_context: None,
            selected_binding_ids: Vec::new(),
            dependency_requirements: Some(sample_requirements()),
            dependency_override_patches: Vec::new(),
        })
        .expect("request");

        assert_eq!(request.node_type, "dependency-environment");
        assert_eq!(request.model_path, "/models/example");
        assert_eq!(request.model_id.as_deref(), Some("model-a"));
        assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
        assert_eq!(
            request.platform_context,
            Some(json!({ "os": "linux", "arch": "x86_64" }))
        );
        assert_eq!(request.selected_binding_ids, vec!["binding-b".to_string()]);
    }
}

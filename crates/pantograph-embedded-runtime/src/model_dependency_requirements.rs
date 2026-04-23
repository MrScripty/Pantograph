use std::collections::{HashMap, HashSet};
use std::path::Path;

use node_engine::{
    DependencyOverridePatchV1, DependencyOverrideScope, DependencyState, DependencyValidationError,
    DependencyValidationErrorScope, DependencyValidationState, ModelDependencyBinding,
    ModelDependencyBindingStatus, ModelDependencyRequirement, ModelDependencyRequirements,
};

pub(super) fn map_validation_state(
    state: pumas_library::model_library::DependencyValidationState,
) -> DependencyValidationState {
    match state {
        pumas_library::model_library::DependencyValidationState::Resolved => {
            DependencyValidationState::Resolved
        }
        pumas_library::model_library::DependencyValidationState::UnknownProfile => {
            DependencyValidationState::UnknownProfile
        }
        pumas_library::model_library::DependencyValidationState::InvalidProfile => {
            DependencyValidationState::InvalidProfile
        }
        pumas_library::model_library::DependencyValidationState::ProfileConflict => {
            DependencyValidationState::ProfileConflict
        }
    }
}

fn map_validation_scope(
    scope: pumas_library::model_library::DependencyValidationErrorScope,
) -> DependencyValidationErrorScope {
    match scope {
        pumas_library::model_library::DependencyValidationErrorScope::TopLevel => {
            DependencyValidationErrorScope::TopLevel
        }
        pumas_library::model_library::DependencyValidationErrorScope::Binding => {
            DependencyValidationErrorScope::Binding
        }
    }
}

pub(super) fn map_validation_error(
    error: &pumas_library::model_library::DependencyValidationError,
) -> DependencyValidationError {
    DependencyValidationError {
        code: error.code.clone(),
        scope: map_validation_scope(error.scope),
        binding_id: error.binding_id.clone(),
        field: error.field.clone(),
        message: error.message.clone(),
    }
}

fn map_requirement(
    requirement: &pumas_library::model_library::ModelDependencyRequirement,
) -> ModelDependencyRequirement {
    ModelDependencyRequirement {
        kind: requirement.kind.clone(),
        name: requirement.name.clone(),
        exact_pin: requirement.exact_pin.clone(),
        index_url: requirement.index_url.clone(),
        extra_index_urls: requirement.extra_index_urls.clone(),
        markers: requirement.markers.clone(),
        python_requires: requirement.python_requires.clone(),
        platform_constraints: requirement.platform_constraints.clone(),
        hashes: requirement.hashes.clone(),
        source: requirement.source.clone(),
    }
}

pub(super) fn map_binding(
    binding: &pumas_library::model_library::ModelDependencyBindingRequirements,
) -> ModelDependencyBinding {
    ModelDependencyBinding {
        binding_id: binding.binding_id.clone(),
        profile_id: binding.profile_id.clone(),
        profile_version: binding.profile_version,
        profile_hash: binding.profile_hash.clone(),
        backend_key: binding.backend_key.clone(),
        platform_selector: binding.platform_selector.clone(),
        environment_kind: binding.environment_kind.clone(),
        env_id: binding.env_id.clone(),
        python_executable_override: None,
        validation_state: map_validation_state(binding.validation_state),
        validation_errors: binding
            .validation_errors
            .iter()
            .map(map_validation_error)
            .collect(),
        requirements: binding.requirements.iter().map(map_requirement).collect(),
    }
}

pub(super) fn sort_bindings(bindings: &mut [ModelDependencyBinding]) {
    bindings.sort_by(|a, b| a.binding_id.cmp(&b.binding_id));
}

pub(super) fn select_binding_ids_for_requirements(
    requested: Option<&Vec<String>>,
    bindings: &[ModelDependencyBinding],
) -> Vec<String> {
    let available = bindings
        .iter()
        .map(|b| b.binding_id.clone())
        .collect::<HashSet<_>>();

    if let Some(requested_ids) = requested {
        requested_ids
            .iter()
            .filter(|id| available.contains(*id))
            .cloned()
            .collect()
    } else {
        bindings.iter().map(|b| b.binding_id.clone()).collect()
    }
}

pub(super) fn runtime_state_from_validation(
    validation_state: DependencyValidationState,
) -> DependencyState {
    match validation_state {
        DependencyValidationState::Resolved => DependencyState::Resolved,
        DependencyValidationState::UnknownProfile => DependencyState::Unresolved,
        DependencyValidationState::InvalidProfile | DependencyValidationState::ProfileConflict => {
            DependencyState::Invalid
        }
    }
}

pub(super) fn aggregate_binding_runtime_state(
    rows: &[ModelDependencyBindingStatus],
) -> DependencyState {
    if rows.is_empty() {
        return DependencyState::Unresolved;
    }
    if rows
        .iter()
        .any(|row| matches!(row.state, DependencyState::Failed))
    {
        return DependencyState::Failed;
    }
    if rows
        .iter()
        .any(|row| matches!(row.state, DependencyState::Missing))
    {
        return DependencyState::Missing;
    }
    if rows
        .iter()
        .all(|row| matches!(row.state, DependencyState::Ready))
    {
        return DependencyState::Ready;
    }
    DependencyState::Resolved
}

pub(super) fn normalize_exact_pin(pin: &str) -> String {
    pin.trim().trim_start_matches("==").to_string()
}

fn requirement_spec(requirement: &ModelDependencyRequirement) -> String {
    let pin = requirement.exact_pin.trim();
    let mut spec = if pin.starts_with(['=', '!', '<', '>', '~']) {
        format!("{}{}", requirement.name, pin)
    } else {
        format!("{}=={}", requirement.name, pin)
    };
    if let Some(markers) = &requirement.markers {
        let trimmed = markers.trim();
        if !trimmed.is_empty() {
            spec.push_str("; ");
            spec.push_str(trimmed);
        }
    }
    spec
}

pub(super) fn requirement_install_target(requirement: &ModelDependencyRequirement) -> String {
    let Some(source) = requirement
        .source
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return requirement_spec(requirement);
    };

    if let Some(path) = source.strip_prefix("wheel_source_path=") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    source.to_string()
}

fn normalize_override_value(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("override field '{}' cannot be empty", field));
    }
    Ok(trimmed.to_string())
}

fn validate_override_url(value: &str, field: &str) -> Result<String, String> {
    let normalized = normalize_override_value(value, field)?;
    let lower = normalized.to_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("file://")
    {
        return Ok(normalized);
    }
    Err(format!(
        "override field '{}' must use http://, https://, or file:// URL scheme",
        field
    ))
}

fn validate_python_executable_override(value: &str) -> Result<String, String> {
    let normalized = normalize_override_value(value, "python_executable")?;
    let candidate = Path::new(&normalized);
    if candidate.exists() {
        return Ok(normalized);
    }
    if which::which(&normalized).is_ok() {
        return Ok(normalized);
    }
    Err(format!(
        "python_executable override was not found as a file path or PATH command: {}",
        normalized
    ))
}

fn validate_wheel_source_path(value: &str) -> Result<String, String> {
    let normalized = normalize_override_value(value, "wheel_source_path")?;
    let candidate = Path::new(&normalized);
    if candidate.exists() {
        return Ok(normalized);
    }
    Err(format!(
        "wheel_source_path override does not exist on disk: {}",
        normalized
    ))
}

fn normalize_extra_index_urls(values: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for value in values {
        let normalized = validate_override_url(value, "extra_index_urls")?;
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    out.sort();
    Ok(out)
}

fn patch_has_any_fields(patch: &DependencyOverridePatchV1) -> bool {
    patch.fields.python_executable.is_some()
        || patch.fields.index_url.is_some()
        || patch.fields.extra_index_urls.is_some()
        || patch.fields.wheel_source_path.is_some()
        || patch.fields.package_source_override.is_some()
}

fn apply_fields_to_requirement(
    requirement: &mut ModelDependencyRequirement,
    patch: &DependencyOverridePatchV1,
) -> Result<(), String> {
    if let Some(index_url) = patch.fields.index_url.as_deref() {
        requirement.index_url = Some(validate_override_url(index_url, "index_url")?);
    }
    if let Some(extra_index_urls) = patch.fields.extra_index_urls.as_ref() {
        requirement.extra_index_urls = normalize_extra_index_urls(extra_index_urls)?;
    }
    if let Some(wheel_source_path) = patch.fields.wheel_source_path.as_deref() {
        let path = validate_wheel_source_path(wheel_source_path)?;
        requirement.source = Some(format!("wheel_source_path={}", path));
    }
    if let Some(source_override) = patch.fields.package_source_override.as_deref() {
        requirement.source = Some(normalize_override_value(
            source_override,
            "package_source_override",
        )?);
    }
    Ok(())
}

pub(super) fn apply_dependency_override_patches(
    mut requirements: ModelDependencyRequirements,
    patches: &[DependencyOverridePatchV1],
) -> Result<ModelDependencyRequirements, String> {
    if patches.is_empty() {
        return Ok(requirements);
    }

    let binding_index = requirements
        .bindings
        .iter()
        .enumerate()
        .map(|(idx, binding)| (binding.binding_id.clone(), idx))
        .collect::<HashMap<_, _>>();

    for patch in patches {
        if patch.contract_version != 1 {
            return Err(format!(
                "unsupported dependency override contract_version {} (expected 1)",
                patch.contract_version
            ));
        }

        if let Some(source) = patch.source.as_deref() {
            if source.trim() != "user" {
                return Err(format!(
                    "unsupported dependency override source '{}' (expected 'user')",
                    source
                ));
            }
        }

        if let Some(updated_at) = patch.updated_at.as_deref() {
            chrono::DateTime::parse_from_rfc3339(updated_at)
                .map_err(|err| format!("invalid override updated_at '{}': {}", updated_at, err))?;
        }

        if !patch_has_any_fields(patch) {
            return Err(format!(
                "override patch for binding '{}' has no override fields",
                patch.binding_id
            ));
        }

        let binding_id = patch.binding_id.trim();
        if binding_id.is_empty() {
            return Err("override patch binding_id is required".to_string());
        }
        let Some(&binding_idx) = binding_index.get(binding_id) else {
            return Err(format!(
                "override patch references unknown binding_id '{}'",
                binding_id
            ));
        };
        let binding = requirements.bindings.get_mut(binding_idx).ok_or_else(|| {
            format!(
                "override patch binding index not found for '{}'",
                binding_id
            )
        })?;

        if let Some(python_executable) = patch.fields.python_executable.as_deref() {
            let validated = validate_python_executable_override(python_executable)?;
            match patch.scope {
                DependencyOverrideScope::Binding => {
                    binding.python_executable_override = Some(validated);
                }
                DependencyOverrideScope::Requirement => {
                    return Err(format!(
                        "python_executable override is only valid for binding scope (binding '{}')",
                        binding_id
                    ));
                }
            }
        }

        match patch.scope {
            DependencyOverrideScope::Binding => {
                if patch
                    .requirement_name
                    .as_deref()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
                {
                    return Err(format!(
                        "binding-scope override for '{}' must not set requirement_name",
                        binding_id
                    ));
                }

                for requirement in &mut binding.requirements {
                    apply_fields_to_requirement(requirement, patch)?;
                }
            }
            DependencyOverrideScope::Requirement => {
                let requirement_name = patch
                    .requirement_name
                    .as_deref()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        format!(
                            "requirement-scope override for '{}' requires requirement_name",
                            binding_id
                        )
                    })?;
                let normalized_name = requirement_name.to_lowercase();

                let mut matched = 0usize;
                for requirement in &mut binding.requirements {
                    if requirement.name.to_lowercase() == normalized_name {
                        apply_fields_to_requirement(requirement, patch)?;
                        matched += 1;
                    }
                }
                if matched == 0 {
                    return Err(format!(
                        "requirement-scope override for '{}' references unknown requirement '{}'",
                        binding_id, requirement_name
                    ));
                }
            }
        }
    }

    Ok(requirements)
}

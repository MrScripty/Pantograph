//! Model dependency resolver used by workflow execution preflight and UI commands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock};

use node_engine::{
    extension_keys, DependencyOverridePatchV1, DependencyOverrideScope, DependencyState,
    DependencyValidationError, DependencyValidationErrorScope, DependencyValidationState,
    ModelDependencyBinding, ModelDependencyBindingStatus, ModelDependencyInstallResult,
    ModelDependencyRequest, ModelDependencyRequirement, ModelDependencyRequirements,
    ModelDependencyResolver, ModelDependencyStatus, ModelRefV2,
};

/// Shared dependency resolver state.
pub type SharedModelDependencyResolver = Arc<TauriModelDependencyResolver>;

const SUPPORTED_DEPENDENCY_CONTRACT_VERSION: u32 = 1;

pub type DependencyActivityEmitter = Arc<dyn Fn(DependencyActivityEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DependencyActivityEvent {
    pub timestamp: String,
    pub node_type: String,
    pub model_path: String,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

#[derive(Debug, Clone)]
struct DependencyActivityContext {
    node_type: String,
    model_path: String,
}

impl DependencyActivityContext {
    fn from_request(request: &ModelDependencyRequest) -> Self {
        Self {
            node_type: request.node_type.trim().to_string(),
            model_path: request.model_path.trim().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedModelDescriptor {
    model_id: String,
    model_path: String,
    model_type: Option<String>,
    task_type_primary: String,
    platform_key: String,
    backend_key: Option<String>,
    selected_binding_ids: Option<Vec<String>>,
    model_id_resolved: bool,
}

/// Tauri host implementation for model dependency resolution/check/install.
pub struct TauriModelDependencyResolver {
    shared_extensions: Arc<RwLock<node_engine::ExecutorExtensions>>,
    _project_root: PathBuf,
    status_cache: RwLock<HashMap<String, ModelDependencyStatus>>,
    install_locks: RwLock<HashMap<String, Arc<Mutex<()>>>>,
    activity_emitter: std::sync::RwLock<Option<DependencyActivityEmitter>>,
}

impl TauriModelDependencyResolver {
    pub fn new(
        shared_extensions: Arc<RwLock<node_engine::ExecutorExtensions>>,
        project_root: PathBuf,
    ) -> Self {
        Self {
            shared_extensions,
            _project_root: project_root,
            status_cache: RwLock::new(HashMap::new()),
            install_locks: RwLock::new(HashMap::new()),
            activity_emitter: std::sync::RwLock::new(None),
        }
    }

    pub fn set_activity_emitter(&self, emitter: DependencyActivityEmitter) {
        if let Ok(mut slot) = self.activity_emitter.write() {
            *slot = Some(emitter);
        }
    }

    pub async fn cached_status(
        &self,
        request: &ModelDependencyRequest,
    ) -> Option<ModelDependencyStatus> {
        let cache = self.status_cache.read().await;
        cache.get(&Self::cache_key(request)).cloned()
    }

    fn current_activity_emitter(&self) -> Option<DependencyActivityEmitter> {
        self.activity_emitter
            .read()
            .ok()
            .and_then(|slot| slot.as_ref().cloned())
    }

    fn emit_activity_with_emitter(
        emitter: Option<&DependencyActivityEmitter>,
        context: &DependencyActivityContext,
        phase: &str,
        message: impl Into<String>,
        binding_id: Option<&str>,
        requirement_name: Option<&str>,
        stream: Option<&str>,
    ) {
        let Some(emitter) = emitter else {
            return;
        };
        emitter(DependencyActivityEvent {
            timestamp: Utc::now().to_rfc3339(),
            node_type: context.node_type.clone(),
            model_path: context.model_path.clone(),
            phase: phase.to_string(),
            message: message.into(),
            binding_id: binding_id.map(|v| v.to_string()),
            requirement_name: requirement_name.map(|v| v.to_string()),
            stream: stream.map(|v| v.to_string()),
        });
    }

    fn emit_activity(
        &self,
        context: &DependencyActivityContext,
        phase: &str,
        message: impl Into<String>,
        binding_id: Option<&str>,
        requirement_name: Option<&str>,
        stream: Option<&str>,
    ) {
        let emitter = self.current_activity_emitter();
        Self::emit_activity_with_emitter(
            emitter.as_ref(),
            context,
            phase,
            message,
            binding_id,
            requirement_name,
            stream,
        );
    }

    fn stable_platform_key(platform_context: &Option<serde_json::Value>) -> String {
        let mut parts = Vec::new();
        if let Some(context) = platform_context {
            if let Some(os) = context.get("os").and_then(|v| v.as_str()) {
                if !os.trim().is_empty() {
                    parts.push(os.to_lowercase());
                }
            }
            if let Some(arch) = context.get("arch").and_then(|v| v.as_str()) {
                if !arch.trim().is_empty() {
                    parts.push(arch.to_lowercase());
                }
            }
            if parts.is_empty() {
                parts.push(Self::stable_json(context));
            }
        }

        if parts.is_empty() {
            std::env::consts::OS.to_string() + "-" + std::env::consts::ARCH
        } else {
            parts.join("-")
        }
    }

    fn stable_json(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Object(map) => {
                let mut keys = map.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                let mut out = String::from("{");
                for (idx, key) in keys.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    out.push_str(key);
                    out.push(':');
                    if let Some(v) = map.get(key) {
                        out.push_str(&Self::stable_json(v));
                    }
                }
                out.push('}');
                out
            }
            serde_json::Value::Array(items) => {
                let mut out = String::from("[");
                for (idx, item) in items.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    out.push_str(&Self::stable_json(item));
                }
                out.push(']');
                out
            }
            _ => value.to_string(),
        }
    }

    fn normalize_path(path: &str) -> String {
        let p = Path::new(path);
        if let Ok(canon) = p.canonicalize() {
            canon.to_string_lossy().to_string()
        } else {
            path.to_string()
        }
    }

    fn cache_key(request: &ModelDependencyRequest) -> String {
        let model_key = request
            .model_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| request.model_path.clone());
        let backend_key = request
            .backend_key
            .clone()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_else(|| "unspecified".to_string());
        let platform_key = Self::stable_platform_key(&request.platform_context);

        let mut selected = request
            .selected_binding_ids
            .iter()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>();
        selected.sort();

        format!(
            "{}|{}|{}|{}",
            model_key,
            backend_key,
            platform_key,
            selected.join(",")
        )
    }

    fn infer_engine(node_type: &str, model_type: Option<&str>) -> String {
        match node_type {
            "audio-generation" => "stable_audio".to_string(),
            "pytorch-inference" => "pytorch".to_string(),
            "llamacpp-inference" => "llamacpp".to_string(),
            "ollama-inference" => "ollama".to_string(),
            _ => {
                if model_type.unwrap_or_default().eq_ignore_ascii_case("audio") {
                    "stable_audio".to_string()
                } else {
                    "pytorch".to_string()
                }
            }
        }
    }

    fn map_pipeline_tag_to_task(pipeline_tag: &str) -> String {
        match pipeline_tag.to_lowercase().as_str() {
            "text-to-audio" | "text-to-speech" => "text-to-audio".to_string(),
            "automatic-speech-recognition" => "audio-to-text".to_string(),
            "text-to-image" | "image-to-image" => "text-to-image".to_string(),
            "feature-extraction" | "sentence-similarity" => "feature-extraction".to_string(),
            "image-classification" | "object-detection" | "image-to-text" => {
                "image-to-text".to_string()
            }
            _ => "text-generation".to_string(),
        }
    }

    fn metadata_string(
        metadata: &serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
    ) -> Option<String> {
        keys.iter().find_map(|key| {
            metadata
                .get(*key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }

    fn normalized_backend_key(value: &Option<String>) -> Option<String> {
        value
            .as_ref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    }

    fn normalized_selected_binding_ids(value: &[String]) -> Option<Vec<String>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for id in value {
            let trimmed = id.trim();
            if trimmed.is_empty() {
                continue;
            }
            let owned = trimmed.to_string();
            if seen.insert(owned.clone()) {
                out.push(owned);
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn make_requirements_id(
        model_id: &str,
        backend_key: Option<&str>,
        platform_key: &str,
        selected_binding_ids: &[String],
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            model_id,
            backend_key.unwrap_or("any"),
            platform_key,
            selected_binding_ids.join(",")
        )
    }

    fn map_validation_state(
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

    fn map_validation_error(
        error: &pumas_library::model_library::DependencyValidationError,
    ) -> DependencyValidationError {
        DependencyValidationError {
            code: error.code.clone(),
            scope: Self::map_validation_scope(error.scope),
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

    fn map_binding(
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
            validation_state: Self::map_validation_state(binding.validation_state),
            validation_errors: binding
                .validation_errors
                .iter()
                .map(Self::map_validation_error)
                .collect(),
            requirements: binding
                .requirements
                .iter()
                .map(Self::map_requirement)
                .collect(),
        }
    }

    fn sort_bindings(bindings: &mut [ModelDependencyBinding]) {
        bindings.sort_by(|a, b| a.binding_id.cmp(&b.binding_id));
    }

    fn select_binding_ids_for_requirements(
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

    fn runtime_state_from_validation(
        validation_state: DependencyValidationState,
    ) -> DependencyState {
        match validation_state {
            DependencyValidationState::Resolved => DependencyState::Resolved,
            DependencyValidationState::UnknownProfile => DependencyState::Unresolved,
            DependencyValidationState::InvalidProfile
            | DependencyValidationState::ProfileConflict => DependencyState::Invalid,
        }
    }

    fn aggregate_binding_runtime_state(rows: &[ModelDependencyBindingStatus]) -> DependencyState {
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

    fn normalize_exact_pin(pin: &str) -> String {
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

    fn normalize_override_value(value: &str, field: &str) -> Result<String, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(format!("override field '{}' cannot be empty", field));
        }
        Ok(trimmed.to_string())
    }

    fn validate_override_url(value: &str, field: &str) -> Result<String, String> {
        let normalized = Self::normalize_override_value(value, field)?;
        let lower = normalized.to_lowercase();
        if lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("file://")
        {
            return Ok(normalized);
        }
        Err(format!(
            "override field '{}' must use http://, https://, or file:// URL scheme",
            field
        ))
    }

    fn validate_python_executable_override(value: &str) -> Result<String, String> {
        let normalized = Self::normalize_override_value(value, "python_executable")?;
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
        let normalized = Self::normalize_override_value(value, "wheel_source_path")?;
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
            let normalized = Self::validate_override_url(value, "extra_index_urls")?;
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
            requirement.index_url = Some(Self::validate_override_url(index_url, "index_url")?);
        }
        if let Some(extra_index_urls) = patch.fields.extra_index_urls.as_ref() {
            requirement.extra_index_urls = Self::normalize_extra_index_urls(extra_index_urls)?;
        }
        if let Some(wheel_source_path) = patch.fields.wheel_source_path.as_deref() {
            let path = Self::validate_wheel_source_path(wheel_source_path)?;
            requirement.source = Some(format!("wheel_source_path={}", path));
        }
        if let Some(source_override) = patch.fields.package_source_override.as_deref() {
            requirement.source = Some(Self::normalize_override_value(
                source_override,
                "package_source_override",
            )?);
        }
        Ok(())
    }

    fn apply_dependency_override_patches(
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
                chrono::DateTime::parse_from_rfc3339(updated_at).map_err(|err| {
                    format!("invalid override updated_at '{}': {}", updated_at, err)
                })?;
            }

            if !Self::patch_has_any_fields(patch) {
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
                let validated = Self::validate_python_executable_override(python_executable)?;
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
                        Self::apply_fields_to_requirement(requirement, patch)?;
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
                            Self::apply_fields_to_requirement(requirement, patch)?;
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

    async fn get_pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let ext = self.shared_extensions.read().await;
        ext.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
            .cloned()
    }

    async fn resolve_model_record_with_api(
        &self,
        api: &Arc<pumas_library::PumasApi>,
        request: &ModelDependencyRequest,
    ) -> Result<Option<pumas_library::ModelRecord>, String> {
        if let Some(model_id) = request.model_id.as_deref() {
            if !model_id.trim().is_empty() {
                return api
                    .get_model(model_id)
                    .await
                    .map_err(|e| format!("Failed to query model '{model_id}': {e}"));
            }
        }

        let all = api
            .list_models()
            .await
            .map_err(|e| format!("Failed to list models: {e}"))?;
        let target = Self::normalize_path(&request.model_path);
        Ok(all.into_iter().find(|record| {
            let rp = Self::normalize_path(&record.path);
            rp == target || target == record.path || record.path == request.model_path
        }))
    }

    async fn resolve_descriptor(
        &self,
        request: &ModelDependencyRequest,
        api: Option<&Arc<pumas_library::PumasApi>>,
    ) -> Result<ResolvedModelDescriptor, String> {
        let resolved_record = if let Some(api) = api {
            self.resolve_model_record_with_api(api, request).await?
        } else {
            None
        };

        let mut model_id = request
            .model_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| request.model_path.clone());
        let mut model_path = request.model_path.clone();
        let mut model_type = request.model_type.clone();
        let mut task_type_primary = request
            .task_type_primary
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "text-generation".to_string());

        if let Some(record) = resolved_record {
            model_id = record.id;
            model_path = record.path;
            model_type = Some(record.model_type.clone());
            if let Some(meta) = record.metadata.as_object() {
                if let Some(task) = Self::metadata_string(
                    meta,
                    &[
                        "task_type_primary",
                        "taskTypePrimary",
                        "task_type",
                        "taskType",
                    ],
                ) {
                    task_type_primary = task;
                } else if let Some(tag) =
                    Self::metadata_string(meta, &["pipeline_tag", "pipelineTag"])
                {
                    task_type_primary = Self::map_pipeline_tag_to_task(&tag);
                }
            }
        }

        let model_id_resolved = request
            .model_id
            .as_ref()
            .map(|id| !id.trim().is_empty())
            .unwrap_or(false)
            || model_id != request.model_path;

        Ok(ResolvedModelDescriptor {
            model_id,
            model_path,
            model_type,
            task_type_primary,
            platform_key: Self::stable_platform_key(&request.platform_context),
            backend_key: Self::normalized_backend_key(&request.backend_key),
            selected_binding_ids: Self::normalized_selected_binding_ids(
                &request.selected_binding_ids,
            ),
            model_id_resolved,
        })
    }

    fn unresolved_requirements(
        descriptor: &ResolvedModelDescriptor,
        code: &str,
        message: String,
    ) -> ModelDependencyRequirements {
        let selected_binding_ids = descriptor.selected_binding_ids.clone().unwrap_or_default();
        ModelDependencyRequirements {
            model_id: descriptor.model_id.clone(),
            platform_key: descriptor.platform_key.clone(),
            backend_key: descriptor.backend_key.clone(),
            dependency_contract_version: SUPPORTED_DEPENDENCY_CONTRACT_VERSION,
            validation_state: DependencyValidationState::UnknownProfile,
            validation_errors: vec![DependencyValidationError {
                code: code.to_string(),
                scope: DependencyValidationErrorScope::TopLevel,
                binding_id: None,
                field: None,
                message,
            }],
            bindings: Vec::new(),
            selected_binding_ids,
        }
    }

    fn validate_resolver_payload(
        payload: &pumas_library::model_library::ModelDependencyRequirementsResolution,
    ) -> Result<(), String> {
        if payload.model_id.trim().is_empty() {
            return Err("resolver payload missing model_id".to_string());
        }
        if payload.platform_key.trim().is_empty() {
            return Err("resolver payload missing platform_key".to_string());
        }
        for binding in &payload.bindings {
            if binding.binding_id.trim().is_empty() {
                return Err("resolver payload contains binding with empty binding_id".to_string());
            }
            if binding.profile_id.trim().is_empty() {
                return Err(format!(
                    "resolver payload binding '{}' missing profile_id",
                    binding.binding_id
                ));
            }
            for requirement in &binding.requirements {
                if requirement.kind.trim().is_empty() || requirement.name.trim().is_empty() {
                    return Err(format!(
                        "resolver payload binding '{}' contains invalid requirement",
                        binding.binding_id
                    ));
                }
                if requirement.exact_pin.trim().is_empty() {
                    return Err(format!(
                        "resolver payload binding '{}' contains requirement '{}' with empty exact_pin",
                        binding.binding_id, requirement.name
                    ));
                }
            }
        }
        Ok(())
    }

    async fn pip_show_version(python: &Path, package_name: &str) -> Result<Option<String>, String> {
        let output = Command::new(python)
            .arg("-m")
            .arg("pip")
            .arg("show")
            .arg(package_name)
            .output()
            .await
            .map_err(|err| format!("Failed to run pip show for '{package_name}': {err}"))?;

        if !output.status.success() {
            return Ok(None);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(version) = line.strip_prefix("Version:") {
                let trimmed = version.trim();
                if !trimmed.is_empty() {
                    return Ok(Some(trimmed.to_string()));
                }
            }
        }
        Ok(None)
    }

    async fn is_requirement_satisfied(
        python: &Path,
        requirement: &ModelDependencyRequirement,
    ) -> Result<bool, String> {
        let installed = Self::pip_show_version(python, &requirement.name).await?;
        let Some(installed_version) = installed else {
            return Ok(false);
        };
        Ok(Self::normalize_exact_pin(&requirement.exact_pin) == installed_version.trim())
    }

    async fn consume_install_stream<R>(
        reader: R,
        stream_name: &'static str,
        emitter: Option<DependencyActivityEmitter>,
        context: DependencyActivityContext,
        binding_id: String,
        requirement_name: String,
    ) -> Vec<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut captured = Vec::new();
        let mut lines = BufReader::new(reader).lines();
        while let Ok(next) = lines.next_line().await {
            let Some(line) = next else {
                break;
            };
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            captured.push(trimmed.clone());
            Self::emit_activity_with_emitter(
                emitter.as_ref(),
                &context,
                "install_stream",
                trimmed,
                Some(&binding_id),
                Some(&requirement_name),
                Some(stream_name),
            );
        }
        captured
    }

    async fn pip_install_requirement(
        &self,
        python: &Path,
        requirement: &ModelDependencyRequirement,
        context: Option<&DependencyActivityContext>,
        binding_id: &str,
    ) -> Result<(), String> {
        let spec = Self::requirement_spec(requirement);
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                format!("pip install {}", spec),
                Some(binding_id),
                Some(&requirement.name),
                None,
            );
        }

        let mut command = Command::new(python);
        command
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--disable-pip-version-check")
            .arg(spec)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(index_url) = requirement.index_url.as_deref() {
            let trimmed = index_url.trim();
            if !trimmed.is_empty() {
                command.arg("--index-url").arg(trimmed);
            }
        }
        for extra in &requirement.extra_index_urls {
            let trimmed = extra.trim();
            if !trimmed.is_empty() {
                command.arg("--extra-index-url").arg(trimmed);
            }
        }

        let mut child = command.spawn().map_err(|err| {
            format!(
                "Failed to run pip install for '{}': {}",
                requirement.name, err
            )
        })?;

        let emitter = self.current_activity_emitter();
        let context_value = context.cloned().unwrap_or(DependencyActivityContext {
            node_type: "unknown".to_string(),
            model_path: "unknown".to_string(),
        });
        let stdout_task = child.stdout.take().map(|stdout| {
            tokio::spawn(Self::consume_install_stream(
                stdout,
                "stdout",
                emitter.clone(),
                context_value.clone(),
                binding_id.to_string(),
                requirement.name.clone(),
            ))
        });
        let stderr_task = child.stderr.take().map(|stderr| {
            tokio::spawn(Self::consume_install_stream(
                stderr,
                "stderr",
                emitter.clone(),
                context_value.clone(),
                binding_id.to_string(),
                requirement.name.clone(),
            ))
        });

        let status = child.wait().await.map_err(|err| {
            format!(
                "Failed waiting for pip install process for '{}': {}",
                requirement.name, err
            )
        })?;

        let stdout_lines = match stdout_task {
            Some(handle) => handle.await.unwrap_or_default(),
            None => Vec::new(),
        };
        let stderr_lines = match stderr_task {
            Some(handle) => handle.await.unwrap_or_default(),
            None => Vec::new(),
        };

        if status.success() {
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "install",
                    "pip install completed",
                    Some(binding_id),
                    Some(&requirement.name),
                    None,
                );
            }
            return Ok(());
        }

        let details = if !stderr_lines.is_empty() {
            stderr_lines.join(" | ")
        } else {
            stdout_lines.join(" | ")
        };
        let message = format!("pip install failed for '{}': {}", requirement.name, details);
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                message.clone(),
                Some(binding_id),
                Some(&requirement.name),
                None,
            );
        }
        Err(message)
    }

    async fn check_binding_with_python(
        &self,
        binding: &ModelDependencyBinding,
        python_override: Option<&Path>,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        if let Some(context) = context {
            self.emit_activity(
                context,
                "check",
                "checking binding requirements",
                Some(&binding.binding_id),
                None,
                None,
            );
        }

        if binding.validation_state != DependencyValidationState::Resolved {
            let state = Self::runtime_state_from_validation(binding.validation_state.clone());
            let code = binding.validation_errors.first().map(|e| e.code.clone());
            let message = binding.validation_errors.first().map(|e| e.message.clone());
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state,
                code,
                message,
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    format!(
                        "binding state={} code={}",
                        serde_json::to_value(&row.state)
                            .ok()
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| "unknown".to_string()),
                        row.code.clone().unwrap_or_else(|| "none".to_string())
                    ),
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        if binding.env_id.as_deref().unwrap_or("").trim().is_empty() {
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state: DependencyState::Unresolved,
                code: Some("env_id_missing".to_string()),
                message: Some("Dependency binding has no env_id".to_string()),
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    "binding has no env_id",
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        let environment_kind = binding
            .environment_kind
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if environment_kind != "python" && environment_kind != "python-venv" {
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state: DependencyState::Failed,
                code: Some("unsupported_environment_kind".to_string()),
                message: Some(format!(
                    "Unsupported environment_kind '{}'",
                    environment_kind
                )),
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    row.message
                        .clone()
                        .unwrap_or_else(|| "unsupported environment".to_string()),
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        let python = if let Some(existing) = python_override {
            existing.to_path_buf()
        } else {
            let env_ids = binding.env_id.clone().into_iter().collect::<Vec<_>>();
            match super::python_runtime::resolve_python_executable_for_env_ids(&env_ids) {
                Ok(path) => path,
                Err(err) => {
                    let row = ModelDependencyBindingStatus {
                        binding_id: binding.binding_id.clone(),
                        env_id: binding.env_id.clone(),
                        state: DependencyState::Failed,
                        code: Some("python_runtime_unavailable".to_string()),
                        message: Some(err),
                        missing_requirements: Vec::new(),
                        installed_requirements: Vec::new(),
                        failed_requirements: Vec::new(),
                    };
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "check",
                            row.message
                                .clone()
                                .unwrap_or_else(|| "python runtime unavailable".to_string()),
                            Some(&binding.binding_id),
                            None,
                            None,
                        );
                    }
                    return row;
                }
            }
        };

        let mut missing_requirements = Vec::new();
        let mut failed_requirements = Vec::new();
        for requirement in &binding.requirements {
            if requirement.kind != "python_package" {
                failed_requirements.push(requirement.name.clone());
                continue;
            }
            match Self::is_requirement_satisfied(&python, requirement).await {
                Ok(true) => {}
                Ok(false) => missing_requirements.push(requirement.name.clone()),
                Err(_) => failed_requirements.push(requirement.name.clone()),
            }
        }

        let state = if !failed_requirements.is_empty() {
            DependencyState::Failed
        } else if !missing_requirements.is_empty() {
            DependencyState::Missing
        } else {
            DependencyState::Ready
        };
        let code = match state {
            DependencyState::Failed => Some("dependency_check_failed".to_string()),
            DependencyState::Missing => Some("requirements_missing".to_string()),
            _ => None,
        };

        let row = ModelDependencyBindingStatus {
            binding_id: binding.binding_id.clone(),
            env_id: binding.env_id.clone(),
            state,
            code,
            message: None,
            missing_requirements,
            installed_requirements: Vec::new(),
            failed_requirements,
        };
        if let Some(context) = context {
            self.emit_activity(
                context,
                "check",
                format!(
                    "binding state={} missing={} failed={}",
                    serde_json::to_value(&row.state)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown".to_string()),
                    row.missing_requirements.len(),
                    row.failed_requirements.len()
                ),
                Some(&binding.binding_id),
                None,
                None,
            );
        }
        row
    }

    async fn check_binding(
        &self,
        binding: &ModelDependencyBinding,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        let python_override = binding
            .python_executable_override
            .as_deref()
            .map(PathBuf::from);
        self.check_binding_with_python(binding, python_override.as_deref(), context)
            .await
    }

    async fn install_binding_requirements(
        &self,
        binding: &ModelDependencyBinding,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        let env_id = binding.env_id.clone().unwrap_or_default();
        if env_id.trim().is_empty() {
            return self.check_binding(binding, context).await;
        }

        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                "starting binding install",
                Some(&binding.binding_id),
                None,
                None,
            );
        }

        let lock = self.get_or_create_install_lock(&env_id).await;
        let _guard = lock.lock().await;

        if binding.validation_state != DependencyValidationState::Resolved {
            return self.check_binding(binding, context).await;
        }

        let python = if let Some(override_path) = binding.python_executable_override.as_deref() {
            PathBuf::from(override_path)
        } else {
            let env_ids = vec![env_id];
            match super::python_runtime::resolve_python_executable_for_env_ids(&env_ids) {
                Ok(path) => path,
                Err(err) => {
                    let mut row = self.check_binding(binding, context).await;
                    row.state = DependencyState::Failed;
                    row.code = Some("python_runtime_unavailable".to_string());
                    row.message = Some(err);
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "install",
                            row.message
                                .clone()
                                .unwrap_or_else(|| "python runtime unavailable".to_string()),
                            Some(&binding.binding_id),
                            None,
                            None,
                        );
                    }
                    return row;
                }
            }
        };

        let mut installed_requirements = Vec::new();
        let mut failed_requirements = Vec::new();
        for requirement in &binding.requirements {
            if requirement.kind != "python_package" {
                failed_requirements.push(requirement.name.clone());
                continue;
            }

            match Self::is_requirement_satisfied(&python, requirement).await {
                Ok(true) => {
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "install",
                            "requirement already satisfied",
                            Some(&binding.binding_id),
                            Some(&requirement.name),
                            None,
                        );
                    }
                    continue;
                }
                Ok(false) => {}
                Err(_) => {}
            }

            match self
                .pip_install_requirement(&python, requirement, context, &binding.binding_id)
                .await
            {
                Ok(()) => installed_requirements.push(requirement.name.clone()),
                Err(_) => failed_requirements.push(requirement.name.clone()),
            }
        }

        let mut post_check = self
            .check_binding_with_python(binding, Some(&python), context)
            .await;
        post_check.installed_requirements = installed_requirements;
        if !failed_requirements.is_empty() {
            post_check.failed_requirements.extend(failed_requirements);
            post_check.failed_requirements.sort();
            post_check.failed_requirements.dedup();
            post_check.state = DependencyState::Failed;
            post_check.code = Some("dependency_install_failed".to_string());
        }
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                format!(
                    "binding state={} installed={} failed={}",
                    serde_json::to_value(&post_check.state)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown".to_string()),
                    post_check.installed_requirements.len(),
                    post_check.failed_requirements.len()
                ),
                Some(&binding.binding_id),
                None,
                None,
            );
        }
        post_check
    }

    async fn get_or_create_install_lock(&self, env_id: &str) -> Arc<Mutex<()>> {
        {
            let map = self.install_locks.read().await;
            if let Some(lock) = map.get(env_id) {
                return lock.clone();
            }
        }
        let mut map = self.install_locks.write().await;
        map.entry(env_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn pick_bindings_for_execution<'a>(
        requirements: &'a ModelDependencyRequirements,
    ) -> Vec<&'a ModelDependencyBinding> {
        let selected = requirements
            .selected_binding_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        if selected.is_empty() {
            return requirements.bindings.iter().collect();
        }

        requirements
            .bindings
            .iter()
            .filter(|binding| selected.contains(&binding.binding_id))
            .collect()
    }

    pub async fn resolve_requirements_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyRequirements, String> {
        let context = DependencyActivityContext::from_request(&request);
        self.emit_activity(
            &context,
            "resolve",
            "starting dependency requirement resolution",
            None,
            None,
            None,
        );
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;

        let Some(api) = api else {
            let unresolved = Self::unresolved_requirements(
                &descriptor,
                "pumas_api_unavailable",
                "Pumas dependency resolver API is not available in executor extensions".to_string(),
            );
            self.emit_activity(
                &context,
                "resolve",
                "pumas api unavailable",
                None,
                None,
                None,
            );
            return Ok(unresolved);
        };

        if !descriptor.model_id_resolved {
            let unresolved = Self::unresolved_requirements(
                &descriptor,
                "unknown_profile",
                "Unable to resolve model_id for dependency requirements".to_string(),
            );
            self.emit_activity(
                &context,
                "resolve",
                "model_id could not be resolved",
                None,
                None,
                None,
            );
            return Ok(unresolved);
        }

        let raw = api
            .resolve_model_dependency_requirements(
                &descriptor.model_id,
                &descriptor.platform_key,
                descriptor.backend_key.as_deref(),
            )
            .await
            .map_err(|err| {
                format!(
                    "Failed to resolve dependency requirements for model '{}': {}",
                    descriptor.model_id, err
                )
            })?;

        if raw.dependency_contract_version != SUPPORTED_DEPENDENCY_CONTRACT_VERSION {
            return Err(format!(
                "Unsupported dependency_contract_version {} (expected {})",
                raw.dependency_contract_version, SUPPORTED_DEPENDENCY_CONTRACT_VERSION
            ));
        }
        Self::validate_resolver_payload(&raw)?;

        let mut bindings = raw
            .bindings
            .iter()
            .map(Self::map_binding)
            .collect::<Vec<_>>();
        Self::sort_bindings(&mut bindings);
        let selected_binding_ids = Self::select_binding_ids_for_requirements(
            descriptor.selected_binding_ids.as_ref(),
            &bindings,
        );

        let requirements = ModelDependencyRequirements {
            model_id: raw.model_id,
            platform_key: raw.platform_key,
            backend_key: raw.backend_key,
            dependency_contract_version: raw.dependency_contract_version,
            validation_state: Self::map_validation_state(raw.validation_state),
            validation_errors: raw
                .validation_errors
                .iter()
                .map(Self::map_validation_error)
                .collect(),
            bindings,
            selected_binding_ids,
        };

        let requirements = Self::apply_dependency_override_patches(
            requirements,
            &request.dependency_override_patches,
        )?;
        self.emit_activity(
            &context,
            "resolve",
            format!(
                "resolved validation={} bindings={}",
                serde_json::to_value(&requirements.validation_state)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string()),
                requirements.bindings.len()
            ),
            None,
            None,
            None,
        );
        Ok(requirements)
    }

    pub async fn check_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyStatus, String> {
        let context = DependencyActivityContext::from_request(&request);
        self.emit_activity(
            &context,
            "check",
            "starting dependency check",
            None,
            None,
            None,
        );
        let requirements = self.resolve_requirements_request(request.clone()).await?;

        let status = if requirements.validation_state != DependencyValidationState::Resolved {
            ModelDependencyStatus {
                state: Self::runtime_state_from_validation(requirements.validation_state.clone()),
                code: requirements
                    .validation_errors
                    .first()
                    .map(|e| e.code.clone()),
                message: requirements
                    .validation_errors
                    .first()
                    .map(|e| e.message.clone()),
                requirements,
                bindings: Vec::new(),
                checked_at: Some(Utc::now().to_rfc3339()),
            }
        } else {
            let selected = Self::pick_bindings_for_execution(&requirements);
            if selected.is_empty() {
                return Ok(ModelDependencyStatus {
                    state: DependencyState::Unresolved,
                    code: Some("no_dependency_bindings".to_string()),
                    message: Some(
                        "No dependency bindings were resolved for this model selection".to_string(),
                    ),
                    requirements,
                    bindings: Vec::new(),
                    checked_at: Some(Utc::now().to_rfc3339()),
                });
            }

            let mut rows = Vec::new();
            for binding in selected {
                rows.push(self.check_binding(binding, Some(&context)).await);
            }
            ModelDependencyStatus {
                state: Self::aggregate_binding_runtime_state(&rows),
                code: None,
                message: None,
                requirements,
                bindings: rows,
                checked_at: Some(Utc::now().to_rfc3339()),
            }
        };

        let mut cache = self.status_cache.write().await;
        cache.insert(Self::cache_key(&request), status.clone());
        self.emit_activity(
            &context,
            "check",
            format!(
                "check complete state={}",
                serde_json::to_value(&status.state)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            None,
            None,
            None,
        );
        Ok(status)
    }

    pub async fn install_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyInstallResult, String> {
        let context = DependencyActivityContext::from_request(&request);
        self.emit_activity(
            &context,
            "install",
            "starting dependency install",
            None,
            None,
            None,
        );
        let requirements = self.resolve_requirements_request(request.clone()).await?;

        if requirements.validation_state != DependencyValidationState::Resolved {
            return Ok(ModelDependencyInstallResult {
                state: Self::runtime_state_from_validation(requirements.validation_state.clone()),
                code: requirements
                    .validation_errors
                    .first()
                    .map(|e| e.code.clone()),
                message: requirements
                    .validation_errors
                    .first()
                    .map(|e| e.message.clone()),
                requirements,
                bindings: Vec::new(),
                installed_at: Some(Utc::now().to_rfc3339()),
            });
        }

        let selected = Self::pick_bindings_for_execution(&requirements);
        if selected.is_empty() {
            return Ok(ModelDependencyInstallResult {
                state: DependencyState::Unresolved,
                code: Some("no_dependency_bindings".to_string()),
                message: Some(
                    "No dependency bindings were resolved for this model selection".to_string(),
                ),
                requirements,
                bindings: Vec::new(),
                installed_at: Some(Utc::now().to_rfc3339()),
            });
        }

        {
            let mut cache = self.status_cache.write().await;
            cache.insert(
                Self::cache_key(&request),
                ModelDependencyStatus {
                    state: DependencyState::Installing,
                    code: None,
                    message: Some("Installing dependencies...".to_string()),
                    requirements: requirements.clone(),
                    bindings: Vec::new(),
                    checked_at: Some(Utc::now().to_rfc3339()),
                },
            );
        }

        let mut rows = Vec::new();
        for binding in selected {
            rows.push(
                self.install_binding_requirements(binding, Some(&context))
                    .await,
            );
        }

        let state = Self::aggregate_binding_runtime_state(&rows);
        let code = match state {
            DependencyState::Failed => Some("dependency_install_failed".to_string()),
            DependencyState::Missing => Some("requirements_missing".to_string()),
            _ => None,
        };

        let install = ModelDependencyInstallResult {
            state: state.clone(),
            code,
            message: None,
            requirements: requirements.clone(),
            bindings: rows.clone(),
            installed_at: Some(Utc::now().to_rfc3339()),
        };
        let mut cache = self.status_cache.write().await;
        cache.insert(
            Self::cache_key(&request),
            ModelDependencyStatus {
                state,
                code: install.code.clone(),
                message: install.message.clone(),
                requirements,
                bindings: rows,
                checked_at: Some(Utc::now().to_rfc3339()),
            },
        );

        self.emit_activity(
            &context,
            "install",
            format!(
                "install complete state={}",
                serde_json::to_value(&install.state)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            None,
            None,
            None,
        );
        Ok(install)
    }

    pub async fn resolve_model_ref_request(
        &self,
        request: ModelDependencyRequest,
        requirements: Option<ModelDependencyRequirements>,
    ) -> Result<Option<ModelRefV2>, String> {
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;
        let resolved_requirements = if let Some(requirements) = requirements {
            requirements
        } else {
            self.resolve_requirements_request(request.clone()).await?
        };

        let selected = resolved_requirements
            .selected_binding_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let bindings = if selected.is_empty() {
            resolved_requirements.bindings.clone()
        } else {
            resolved_requirements
                .bindings
                .into_iter()
                .filter(|binding| selected.contains(&binding.binding_id))
                .collect::<Vec<_>>()
        };

        let requirements_id = Self::make_requirements_id(
            &descriptor.model_id,
            descriptor.backend_key.as_deref(),
            &descriptor.platform_key,
            &resolved_requirements.selected_binding_ids,
        );
        let engine = Self::infer_engine(&request.node_type, descriptor.model_type.as_deref());
        let model_ref = ModelRefV2 {
            contract_version: 2,
            engine,
            model_id: descriptor.model_id,
            model_path: descriptor.model_path,
            task_type_primary: descriptor.task_type_primary,
            dependency_bindings: bindings,
            dependency_requirements_id: Some(requirements_id),
        };
        model_ref.validate()?;
        Ok(Some(model_ref))
    }
}

#[async_trait]
impl ModelDependencyResolver for TauriModelDependencyResolver {
    async fn resolve_model_dependency_requirements(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyRequirements, String> {
        self.resolve_requirements_request(request).await
    }

    async fn check_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyStatus, String> {
        self.check_request(request).await
    }

    async fn install_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyInstallResult, String> {
        self.install_request(request).await
    }

    async fn resolve_model_ref(
        &self,
        request: ModelDependencyRequest,
        requirements: Option<ModelDependencyRequirements>,
    ) -> Result<Option<ModelRefV2>, String> {
        self.resolve_model_ref_request(request, requirements).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn test_resolver() -> TauriModelDependencyResolver {
        TauriModelDependencyResolver::new(
            Arc::new(RwLock::new(node_engine::ExecutorExtensions::default())),
            PathBuf::from("."),
        )
    }

    fn sample_request() -> ModelDependencyRequest {
        ModelDependencyRequest {
            node_type: "pytorch-inference".to_string(),
            model_path: "/tmp/model".to_string(),
            model_id: Some("model-id".to_string()),
            model_type: Some("diffusion".to_string()),
            task_type_primary: Some("text-to-image".to_string()),
            backend_key: Some("pytorch".to_string()),
            platform_context: Some(serde_json::json!({
                "os": "linux",
                "arch": "x86_64"
            })),
            selected_binding_ids: vec!["binding-b".to_string(), "binding-a".to_string()],
            dependency_override_patches: Vec::new(),
        }
    }

    #[test]
    fn cache_key_is_deterministic_for_binding_order() {
        let mut left = sample_request();
        left.selected_binding_ids = vec!["binding-b".to_string(), "binding-a".to_string()];
        let mut right = sample_request();
        right.selected_binding_ids = vec!["binding-a".to_string(), "binding-b".to_string()];

        assert_eq!(
            TauriModelDependencyResolver::cache_key(&left),
            TauriModelDependencyResolver::cache_key(&right)
        );
    }

    #[test]
    fn aggregate_state_for_empty_bindings_is_unresolved() {
        let rows: Vec<ModelDependencyBindingStatus> = Vec::new();
        assert_eq!(
            TauriModelDependencyResolver::aggregate_binding_runtime_state(&rows),
            DependencyState::Unresolved
        );
    }

    #[tokio::test]
    async fn resolve_without_api_returns_unknown_profile_requirements() {
        let resolver = test_resolver();
        let requirements = resolver
            .resolve_requirements_request(sample_request())
            .await
            .unwrap();

        assert_eq!(
            requirements.validation_state,
            DependencyValidationState::UnknownProfile
        );
        assert_eq!(
            requirements
                .validation_errors
                .first()
                .map(|e| e.code.as_str()),
            Some("pumas_api_unavailable")
        );
    }

    #[tokio::test]
    async fn check_without_api_returns_unresolved_and_caches_status() {
        let resolver = test_resolver();
        let request = sample_request();
        let status = resolver.check_request(request.clone()).await.unwrap();

        assert_eq!(status.state, DependencyState::Unresolved);
        assert_eq!(status.code.as_deref(), Some("pumas_api_unavailable"));

        let cached = resolver.cached_status(&request).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().state, DependencyState::Unresolved);
    }

    #[tokio::test]
    async fn resolve_model_ref_filters_to_selected_bindings() {
        let resolver = test_resolver();
        let request = sample_request();
        let requirements = ModelDependencyRequirements {
            model_id: "model-id".to_string(),
            platform_key: "linux-x86_64".to_string(),
            backend_key: Some("pytorch".to_string()),
            dependency_contract_version: 1,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            bindings: vec![
                ModelDependencyBinding {
                    binding_id: "binding-a".to_string(),
                    profile_id: "profile-a".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: Some("linux-x86_64".to_string()),
                    environment_kind: Some("python".to_string()),
                    env_id: Some("env-a".to_string()),
                    python_executable_override: None,
                    validation_state: DependencyValidationState::Resolved,
                    validation_errors: Vec::new(),
                    requirements: Vec::new(),
                },
                ModelDependencyBinding {
                    binding_id: "binding-b".to_string(),
                    profile_id: "profile-b".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: Some("linux-x86_64".to_string()),
                    environment_kind: Some("python".to_string()),
                    env_id: Some("env-b".to_string()),
                    python_executable_override: None,
                    validation_state: DependencyValidationState::Resolved,
                    validation_errors: Vec::new(),
                    requirements: Vec::new(),
                },
            ],
            selected_binding_ids: vec!["binding-a".to_string()],
        };

        let model_ref = resolver
            .resolve_model_ref_request(request, Some(requirements))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(model_ref.contract_version, 2);
        assert_eq!(model_ref.dependency_bindings.len(), 1);
        assert_eq!(model_ref.dependency_bindings[0].binding_id, "binding-a");
    }

    #[test]
    fn override_patches_apply_binding_level_python_and_indexes() {
        let python_path = std::env::current_exe()
            .expect("current exe should exist")
            .to_string_lossy()
            .to_string();
        let requirements = ModelDependencyRequirements {
            model_id: "model-id".to_string(),
            platform_key: "linux-x86_64".to_string(),
            backend_key: Some("pytorch".to_string()),
            dependency_contract_version: 1,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            bindings: vec![ModelDependencyBinding {
                binding_id: "binding-a".to_string(),
                profile_id: "profile-a".to_string(),
                profile_version: 1,
                profile_hash: None,
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("env-a".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: vec![ModelDependencyRequirement {
                    kind: "python_package".to_string(),
                    name: "torch".to_string(),
                    exact_pin: "==2.1.0".to_string(),
                    index_url: None,
                    extra_index_urls: Vec::new(),
                    markers: None,
                    python_requires: None,
                    platform_constraints: Vec::new(),
                    hashes: Vec::new(),
                    source: None,
                }],
            }],
            selected_binding_ids: vec!["binding-a".to_string()],
        };

        let patch = DependencyOverridePatchV1 {
            contract_version: 1,
            binding_id: "binding-a".to_string(),
            scope: DependencyOverrideScope::Binding,
            requirement_name: None,
            fields: node_engine::DependencyOverrideFieldsV1 {
                python_executable: Some(python_path),
                index_url: Some("https://download.pytorch.org/whl/cu124".to_string()),
                extra_index_urls: Some(vec!["https://pypi.org/simple".to_string()]),
                wheel_source_path: None,
                package_source_override: None,
            },
            source: Some("user".to_string()),
            updated_at: Some("2026-02-28T00:00:00Z".to_string()),
        };

        let patched =
            TauriModelDependencyResolver::apply_dependency_override_patches(requirements, &[patch])
                .expect("patch should apply");
        let binding = &patched.bindings[0];
        assert!(binding.python_executable_override.is_some());
        assert_eq!(
            binding.requirements[0].index_url.as_deref(),
            Some("https://download.pytorch.org/whl/cu124")
        );
        assert_eq!(
            binding.requirements[0].extra_index_urls,
            vec!["https://pypi.org/simple".to_string()]
        );
    }

    #[test]
    fn override_patches_reject_unknown_binding() {
        let requirements = ModelDependencyRequirements {
            model_id: "model-id".to_string(),
            platform_key: "linux-x86_64".to_string(),
            backend_key: Some("pytorch".to_string()),
            dependency_contract_version: 1,
            validation_state: DependencyValidationState::Resolved,
            validation_errors: Vec::new(),
            bindings: Vec::new(),
            selected_binding_ids: Vec::new(),
        };
        let patch = DependencyOverridePatchV1 {
            contract_version: 1,
            binding_id: "binding-missing".to_string(),
            scope: DependencyOverrideScope::Binding,
            requirement_name: None,
            fields: node_engine::DependencyOverrideFieldsV1 {
                python_executable: Some("python3".to_string()),
                index_url: None,
                extra_index_urls: None,
                wheel_source_path: None,
                package_source_override: None,
            },
            source: Some("user".to_string()),
            updated_at: None,
        };

        let err =
            TauriModelDependencyResolver::apply_dependency_override_patches(requirements, &[patch])
                .expect_err("unknown binding should fail");
        assert!(err.contains("unknown binding_id"));
    }
}

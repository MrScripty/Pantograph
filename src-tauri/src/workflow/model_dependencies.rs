//! Model dependency resolver used by workflow execution preflight and UI commands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use node_engine::{
    extension_keys, DependencyState, ModelDependencyBinding, ModelDependencyBindingStatus,
    ModelDependencyInstallResult, ModelDependencyPinSummary, ModelDependencyPlan,
    ModelDependencyRequest, ModelDependencyRequiredPin, ModelDependencyResolver,
    ModelDependencyStatus, ModelRefV2,
};

/// Shared dependency resolver state.
pub type SharedModelDependencyResolver = Arc<TauriModelDependencyResolver>;

#[derive(Debug, Clone)]
struct ResolvedModelDescriptor {
    model_id: String,
    model_path: String,
    model_type: Option<String>,
    task_type_primary: String,
    review_reasons: Vec<String>,
    platform_key: String,
    backend_key: Option<String>,
    selected_binding_ids: Option<Vec<String>>,
    model_id_resolved: bool,
}

/// Tauri host implementation for model dependency checks/installs.
///
/// This resolver delegates dependency authority to pumas-library APIs.
/// If those APIs are unavailable for a request, it returns conservative
/// non-ready states instead of speculative local installs.
pub struct TauriModelDependencyResolver {
    shared_extensions: Arc<RwLock<node_engine::ExecutorExtensions>>,
    _project_root: PathBuf,
    status_cache: RwLock<HashMap<String, ModelDependencyStatus>>,
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
        }
    }

    pub async fn cached_status(
        &self,
        request: &ModelDependencyRequest,
    ) -> Option<ModelDependencyStatus> {
        let cache = self.status_cache.read().await;
        cache.get(&Self::cache_key(request)).cloned()
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

    fn metadata_bool(metadata: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> bool {
        keys.iter().any(|key| {
            metadata
                .get(*key)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
    }

    fn collect_review_reasons(
        metadata: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<String> {
        let mut reasons = Vec::new();

        if let Some(single) = Self::metadata_string(metadata, &["review_reason", "reviewReason"]) {
            reasons.push(single);
        }

        if let Some(values) = metadata.get("review_reasons").and_then(|v| v.as_array()) {
            for value in values {
                if let Some(reason) = value.as_str() {
                    let trimmed = reason.trim();
                    if !trimmed.is_empty() {
                        reasons.push(trimmed.to_string());
                    }
                }
            }
        }

        if Self::metadata_bool(
            metadata,
            &[
                "metadata_needs_review",
                "metadataNeedsReview",
                "requires_custom_code",
                "requiresCustomCode",
            ],
        ) && reasons.is_empty()
        {
            reasons.push("metadata-needs-review".to_string());
        }

        reasons.sort();
        reasons.dedup();
        reasons
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

    fn is_required_binding_kind(binding_kind: &str) -> bool {
        matches!(
            binding_kind.to_lowercase().as_str(),
            "required" | "required_core" | "required_custom"
        )
    }

    fn make_plan_id(
        model_id: &str,
        backend_key: Option<&str>,
        platform_key: &str,
        selected_binding_ids: &[String],
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            model_id,
            backend_key.unwrap_or("unspecified"),
            platform_key,
            selected_binding_ids.join(",")
        )
    }

    fn map_state_from_pumas(
        state: pumas_library::model_library::DependencyState,
        error_code: Option<&str>,
    ) -> DependencyState {
        if matches!(error_code, Some("required_binding_omitted")) {
            return DependencyState::RequiredBindingOmitted;
        }

        match state {
            pumas_library::model_library::DependencyState::Ready => DependencyState::Ready,
            pumas_library::model_library::DependencyState::Missing => DependencyState::Missing,
            pumas_library::model_library::DependencyState::Failed => DependencyState::Failed,
            pumas_library::model_library::DependencyState::UnknownProfile => {
                DependencyState::UnknownProfile
            }
            pumas_library::model_library::DependencyState::ManualInterventionRequired => {
                DependencyState::ManualInterventionRequired
            }
            pumas_library::model_library::DependencyState::ProfileConflict => {
                DependencyState::ProfileConflict
            }
        }
    }

    fn sort_binding_plans(
        bindings: &mut [pumas_library::model_library::ModelDependencyBindingPlan],
    ) {
        bindings.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.binding_id.cmp(&b.binding_id))
        });
    }

    fn map_binding(
        binding: &pumas_library::model_library::ModelDependencyBindingPlan,
    ) -> ModelDependencyBinding {
        ModelDependencyBinding {
            binding_id: binding.binding_id.clone(),
            profile_id: binding.profile_id.clone(),
            profile_version: binding.profile_version,
            profile_hash: binding.profile_hash.clone(),
            binding_kind: binding.binding_kind.clone(),
            backend_key: binding.backend_key.clone(),
            platform_selector: binding.platform_selector.clone(),
            env_id: binding.env_id.clone(),
            pin_summary: None,
            required_pins: Vec::new(),
            missing_pins: Vec::new(),
        }
    }

    fn map_binding_status(
        binding: &pumas_library::model_library::ModelDependencyBindingPlan,
    ) -> ModelDependencyBindingStatus {
        let state =
            Self::map_state_from_pumas(binding.state.clone(), binding.error_code.as_deref());
        let component_label = format!("{}@{}", binding.profile_id, binding.profile_version);
        let mut missing_components = Vec::new();
        let mut failed_components = Vec::new();

        if state == DependencyState::Missing {
            missing_components.push(component_label.clone());
        }
        if matches!(
            state,
            DependencyState::Failed
                | DependencyState::UnknownProfile
                | DependencyState::ManualInterventionRequired
                | DependencyState::ProfileConflict
                | DependencyState::RequiredBindingOmitted
        ) {
            failed_components.push(component_label);
        }

        ModelDependencyBindingStatus {
            binding_id: binding.binding_id.clone(),
            env_id: binding.env_id.clone(),
            state,
            code: None,
            missing_components,
            installed_components: Vec::new(),
            failed_components,
            message: binding.message.clone(),
            pin_summary: None,
            required_pins: Vec::new(),
            missing_pins: Vec::new(),
        }
    }

    fn select_binding_ids_for_plan(
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

    fn required_binding_ids(bindings: &[ModelDependencyBinding]) -> Vec<String> {
        bindings
            .iter()
            .filter(|b| Self::is_required_binding_kind(&b.binding_kind))
            .map(|b| b.binding_id.clone())
            .collect()
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
        let mut review_reasons = Vec::new();

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
                review_reasons = Self::collect_review_reasons(meta);
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
            review_reasons,
            platform_key: Self::stable_platform_key(&request.platform_context),
            backend_key: Self::normalized_backend_key(&request.backend_key),
            selected_binding_ids: Self::normalized_selected_binding_ids(
                &request.selected_binding_ids,
            ),
            model_id_resolved,
        })
    }

    fn conservative_plan(
        descriptor: &ResolvedModelDescriptor,
        state: DependencyState,
        code: &str,
        message: String,
    ) -> ModelDependencyPlan {
        let selected_binding_ids = descriptor.selected_binding_ids.clone().unwrap_or_default();
        ModelDependencyPlan {
            state,
            code: Some(code.to_string()),
            message: Some(message),
            review_reasons: descriptor.review_reasons.clone(),
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: Vec::new(),
            selected_binding_ids,
            required_binding_ids: Vec::new(),
            missing_pins: Vec::new(),
        }
    }

    fn conservative_status(
        descriptor: &ResolvedModelDescriptor,
        state: DependencyState,
        code: &str,
        message: String,
    ) -> ModelDependencyStatus {
        let selected_binding_ids = descriptor.selected_binding_ids.clone().unwrap_or_default();
        ModelDependencyStatus {
            state,
            code: Some(code.to_string()),
            message: Some(message),
            review_reasons: descriptor.review_reasons.clone(),
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: Vec::new(),
            checked_at: Some(Utc::now().to_rfc3339()),
            missing_pins: Vec::new(),
        }
    }

    fn conservative_install(
        descriptor: &ResolvedModelDescriptor,
        state: DependencyState,
        code: &str,
        message: String,
    ) -> ModelDependencyInstallResult {
        let selected_binding_ids = descriptor.selected_binding_ids.clone().unwrap_or_default();
        ModelDependencyInstallResult {
            state,
            code: Some(code.to_string()),
            message: Some(message),
            review_reasons: descriptor.review_reasons.clone(),
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: Vec::new(),
            installed_at: Some(Utc::now().to_rfc3339()),
            missing_pins: Vec::new(),
        }
    }

    pub async fn resolve_plan_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyPlan, String> {
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;

        let Some(api) = api else {
            return Ok(Self::conservative_plan(
                &descriptor,
                DependencyState::ManualInterventionRequired,
                "manual_intervention_required",
                "Pumas dependency API is not available for this runtime".to_string(),
            ));
        };

        if !descriptor.model_id_resolved {
            return Ok(Self::conservative_plan(
                &descriptor,
                DependencyState::UnknownProfile,
                "unknown_profile",
                "Unable to resolve model_id for dependency planning".to_string(),
            ));
        }

        let mut raw_plan = match api
            .resolve_model_dependency_plan(
                &descriptor.model_id,
                &descriptor.platform_key,
                descriptor.backend_key.as_deref(),
            )
            .await
        {
            Ok(plan) => plan,
            Err(err) => {
                return Ok(Self::conservative_plan(
                    &descriptor,
                    DependencyState::ManualInterventionRequired,
                    "manual_intervention_required",
                    format!("Pumas plan resolution failed: {err}"),
                ));
            }
        };

        // Keep deterministic ordering for UI and persisted node data.
        let mut bindings = std::mem::take(&mut raw_plan.bindings);
        Self::sort_binding_plans(&mut bindings);
        let mapped_bindings = bindings.iter().map(Self::map_binding).collect::<Vec<_>>();
        let selected_binding_ids = Self::select_binding_ids_for_plan(
            descriptor.selected_binding_ids.as_ref(),
            &mapped_bindings,
        );
        let required_binding_ids = Self::required_binding_ids(&mapped_bindings);

        Ok(ModelDependencyPlan {
            state: Self::map_state_from_pumas(raw_plan.state, raw_plan.error_code.as_deref()),
            code: raw_plan.error_code,
            message: raw_plan.message,
            review_reasons: descriptor.review_reasons,
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: mapped_bindings,
            selected_binding_ids,
            required_binding_ids,
            missing_pins: Vec::new(),
        })
    }

    pub async fn check_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyStatus, String> {
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;

        let Some(api) = api else {
            let status = Self::conservative_status(
                &descriptor,
                DependencyState::ManualInterventionRequired,
                "manual_intervention_required",
                "Pumas dependency API is not available for this runtime".to_string(),
            );
            let mut cache = self.status_cache.write().await;
            cache.insert(Self::cache_key(&request), status.clone());
            return Ok(status);
        };

        if !descriptor.model_id_resolved {
            let status = Self::conservative_status(
                &descriptor,
                DependencyState::UnknownProfile,
                "unknown_profile",
                "Unable to resolve model_id for dependency checks".to_string(),
            );
            let mut cache = self.status_cache.write().await;
            cache.insert(Self::cache_key(&request), status.clone());
            return Ok(status);
        }

        let selected = descriptor.selected_binding_ids.clone();
        let result = match api
            .check_model_dependencies(
                &descriptor.model_id,
                &descriptor.platform_key,
                descriptor.backend_key.as_deref(),
                selected.clone(),
            )
            .await
        {
            Ok(result) => result,
            Err(err) => {
                let status = Self::conservative_status(
                    &descriptor,
                    DependencyState::ManualInterventionRequired,
                    "manual_intervention_required",
                    format!("Pumas dependency check failed: {err}"),
                );
                let mut cache = self.status_cache.write().await;
                cache.insert(Self::cache_key(&request), status.clone());
                return Ok(status);
            }
        };

        let mut bindings = result.bindings;
        Self::sort_binding_plans(&mut bindings);
        let mapped_bindings = bindings
            .iter()
            .map(Self::map_binding_status)
            .collect::<Vec<_>>();
        let selected_binding_ids = result
            .selected_binding_ids
            .or(descriptor.selected_binding_ids.clone())
            .unwrap_or_default();

        let status = ModelDependencyStatus {
            state: Self::map_state_from_pumas(result.state, result.error_code.as_deref()),
            code: result.error_code,
            message: result.message,
            review_reasons: descriptor.review_reasons,
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: mapped_bindings,
            checked_at: Some(Utc::now().to_rfc3339()),
            missing_pins: Vec::new(),
        };

        let mut cache = self.status_cache.write().await;
        cache.insert(Self::cache_key(&request), status.clone());
        Ok(status)
    }

    pub async fn install_request(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyInstallResult, String> {
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;

        let Some(api) = api else {
            return Ok(Self::conservative_install(
                &descriptor,
                DependencyState::ManualInterventionRequired,
                "manual_intervention_required",
                "Pumas dependency API is not available for this runtime".to_string(),
            ));
        };

        if !descriptor.model_id_resolved {
            return Ok(Self::conservative_install(
                &descriptor,
                DependencyState::UnknownProfile,
                "unknown_profile",
                "Unable to resolve model_id for dependency installation".to_string(),
            ));
        }

        {
            let mut cache = self.status_cache.write().await;
            cache.insert(
                Self::cache_key(&request),
                ModelDependencyStatus {
                    state: DependencyState::Installing,
                    code: None,
                    message: Some("Installing dependencies...".to_string()),
                    review_reasons: descriptor.review_reasons.clone(),
                    plan_id: Some(Self::make_plan_id(
                        &descriptor.model_id,
                        descriptor.backend_key.as_deref(),
                        &descriptor.platform_key,
                        descriptor
                            .selected_binding_ids
                            .clone()
                            .unwrap_or_default()
                            .as_slice(),
                    )),
                    bindings: Vec::new(),
                    checked_at: Some(Utc::now().to_rfc3339()),
                    missing_pins: Vec::new(),
                },
            );
        }

        let selected = descriptor.selected_binding_ids.clone();
        let result = match api
            .install_model_dependencies(
                &descriptor.model_id,
                &descriptor.platform_key,
                descriptor.backend_key.as_deref(),
                selected.clone(),
            )
            .await
        {
            Ok(result) => result,
            Err(err) => {
                return Ok(Self::conservative_install(
                    &descriptor,
                    DependencyState::ManualInterventionRequired,
                    "manual_intervention_required",
                    format!("Pumas dependency install failed: {err}"),
                ));
            }
        };

        let mut bindings = result.bindings;
        Self::sort_binding_plans(&mut bindings);
        let installed_set = result
            .installed_binding_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();

        let mut mapped_bindings = bindings
            .iter()
            .map(Self::map_binding_status)
            .collect::<Vec<_>>();
        for row in &mut mapped_bindings {
            let component_label = row
                .failed_components
                .first()
                .cloned()
                .or_else(|| row.missing_components.first().cloned())
                .unwrap_or_else(|| "profile".to_string());
            if installed_set.contains(&row.binding_id) {
                row.installed_components.push(component_label);
                row.failed_components.clear();
                row.missing_components.clear();
            }
        }

        let selected_binding_ids = result
            .selected_binding_ids
            .or(descriptor.selected_binding_ids.clone())
            .unwrap_or_default();

        let install = ModelDependencyInstallResult {
            state: Self::map_state_from_pumas(result.state, result.error_code.as_deref()),
            code: result.error_code,
            message: result.message,
            review_reasons: descriptor.review_reasons.clone(),
            plan_id: Some(Self::make_plan_id(
                &descriptor.model_id,
                descriptor.backend_key.as_deref(),
                &descriptor.platform_key,
                &selected_binding_ids,
            )),
            bindings: mapped_bindings.clone(),
            installed_at: Some(Utc::now().to_rfc3339()),
            missing_pins: Vec::new(),
        };

        let status_after_install = ModelDependencyStatus {
            state: install.state.clone(),
            code: install.code.clone(),
            message: install.message.clone(),
            review_reasons: install.review_reasons.clone(),
            plan_id: install.plan_id.clone(),
            bindings: mapped_bindings,
            checked_at: Some(Utc::now().to_rfc3339()),
            missing_pins: Vec::new(),
        };
        let mut cache = self.status_cache.write().await;
        cache.insert(Self::cache_key(&request), status_after_install);

        Ok(install)
    }

    pub async fn resolve_model_ref_request(
        &self,
        request: ModelDependencyRequest,
        plan: Option<ModelDependencyPlan>,
    ) -> Result<Option<ModelRefV2>, String> {
        let api = self.get_pumas_api().await;
        let descriptor = self.resolve_descriptor(&request, api.as_ref()).await?;
        let resolved_plan = if let Some(plan) = plan {
            plan
        } else {
            self.resolve_plan_request(request.clone()).await?
        };

        let selected_set = resolved_plan
            .selected_binding_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let bindings = if selected_set.is_empty() {
            resolved_plan.bindings.clone()
        } else {
            resolved_plan
                .bindings
                .into_iter()
                .filter(|binding| selected_set.contains(&binding.binding_id))
                .collect::<Vec<_>>()
        };

        let engine = Self::infer_engine(&request.node_type, descriptor.model_type.as_deref());
        let model_ref = ModelRefV2 {
            contract_version: 2,
            engine,
            model_id: descriptor.model_id,
            model_path: descriptor.model_path,
            task_type_primary: descriptor.task_type_primary,
            dependency_bindings: bindings,
            dependency_plan_id: resolved_plan.plan_id,
        };
        model_ref.validate()?;
        Ok(Some(model_ref))
    }
}

#[async_trait]
impl ModelDependencyResolver for TauriModelDependencyResolver {
    async fn resolve_model_dependency_plan(
        &self,
        request: ModelDependencyRequest,
    ) -> Result<ModelDependencyPlan, String> {
        self.resolve_plan_request(request).await
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
        plan: Option<ModelDependencyPlan>,
    ) -> Result<Option<ModelRefV2>, String> {
        self.resolve_model_ref_request(request, plan).await
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
        }
    }

    #[test]
    fn maps_required_binding_omitted_to_explicit_state() {
        let state = TauriModelDependencyResolver::map_state_from_pumas(
            pumas_library::model_library::DependencyState::Failed,
            Some("required_binding_omitted"),
        );
        assert_eq!(state, DependencyState::RequiredBindingOmitted);
    }

    #[test]
    fn required_binding_kind_covers_v2_variants() {
        assert!(TauriModelDependencyResolver::is_required_binding_kind(
            "required_core"
        ));
        assert!(TauriModelDependencyResolver::is_required_binding_kind(
            "required_custom"
        ));
        assert!(TauriModelDependencyResolver::is_required_binding_kind(
            "required"
        ));
        assert!(!TauriModelDependencyResolver::is_required_binding_kind(
            "optional"
        ));
    }

    #[test]
    fn maps_profile_conflict_and_unknown_profile_states() {
        let profile_conflict = TauriModelDependencyResolver::map_state_from_pumas(
            pumas_library::model_library::DependencyState::ProfileConflict,
            None,
        );
        assert_eq!(profile_conflict, DependencyState::ProfileConflict);

        let unknown = TauriModelDependencyResolver::map_state_from_pumas(
            pumas_library::model_library::DependencyState::UnknownProfile,
            None,
        );
        assert_eq!(unknown, DependencyState::UnknownProfile);
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

    #[tokio::test]
    async fn resolve_plan_without_api_returns_manual_intervention_required() {
        let resolver = test_resolver();
        let plan = resolver
            .resolve_plan_request(sample_request())
            .await
            .unwrap();

        assert_eq!(plan.state, DependencyState::ManualInterventionRequired);
        assert_eq!(plan.code.as_deref(), Some("manual_intervention_required"));
        assert!(plan.bindings.is_empty());
    }

    #[tokio::test]
    async fn check_without_api_returns_and_caches_manual_intervention_required() {
        let resolver = test_resolver();
        let request = sample_request();
        let status = resolver.check_request(request.clone()).await.unwrap();

        assert_eq!(status.state, DependencyState::ManualInterventionRequired);
        assert_eq!(status.code.as_deref(), Some("manual_intervention_required"));
        assert!(status.bindings.is_empty());

        let cached = resolver.cached_status(&request).await;
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().state,
            DependencyState::ManualInterventionRequired
        );
    }

    #[tokio::test]
    async fn install_without_api_returns_manual_intervention_required() {
        let resolver = test_resolver();
        let install = resolver.install_request(sample_request()).await.unwrap();

        assert_eq!(install.state, DependencyState::ManualInterventionRequired);
        assert_eq!(
            install.code.as_deref(),
            Some("manual_intervention_required")
        );
        assert!(install.bindings.is_empty());
    }

    #[tokio::test]
    async fn resolve_model_ref_filters_to_selected_bindings() {
        let resolver = test_resolver();
        let request = sample_request();
        let plan = ModelDependencyPlan {
            state: DependencyState::Ready,
            code: None,
            message: None,
            review_reasons: Vec::new(),
            plan_id: Some("plan-1".to_string()),
            bindings: vec![
                ModelDependencyBinding {
                    binding_id: "binding-a".to_string(),
                    profile_id: "profile-a".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    binding_kind: "required".to_string(),
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: Some("linux-x86_64".to_string()),
                    env_id: "env-a".to_string(),
                    pin_summary: None,
                    required_pins: Vec::new(),
                    missing_pins: Vec::new(),
                },
                ModelDependencyBinding {
                    binding_id: "binding-b".to_string(),
                    profile_id: "profile-b".to_string(),
                    profile_version: 1,
                    profile_hash: None,
                    binding_kind: "optional".to_string(),
                    backend_key: Some("pytorch".to_string()),
                    platform_selector: Some("linux-x86_64".to_string()),
                    env_id: "env-b".to_string(),
                    pin_summary: None,
                    required_pins: Vec::new(),
                    missing_pins: Vec::new(),
                },
            ],
            selected_binding_ids: vec!["binding-a".to_string()],
            required_binding_ids: vec!["binding-a".to_string()],
            missing_pins: Vec::new(),
        };

        let model_ref = resolver
            .resolve_model_ref_request(request, Some(plan))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(model_ref.contract_version, 2);
        assert_eq!(model_ref.dependency_bindings.len(), 1);
        assert_eq!(model_ref.dependency_bindings[0].binding_id, "binding-a");
    }
}

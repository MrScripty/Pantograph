//! Model dependency resolver used by workflow execution preflight and UI commands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock};

use node_engine::{
    extension_keys, DependencyOverridePatchV1, DependencyState, DependencyValidationError,
    DependencyValidationErrorScope, DependencyValidationState, ModelDependencyBinding,
    ModelDependencyBindingStatus, ModelDependencyInstallResult, ModelDependencyRequest,
    ModelDependencyRequirement, ModelDependencyRequirements, ModelDependencyResolver,
    ModelDependencyStatus, ModelRefV2,
};

#[path = "model_dependency_activity.rs"]
mod activity;
#[path = "model_dependency_descriptors.rs"]
mod descriptors;
#[path = "model_dependency_python.rs"]
mod python_environment;
#[path = "model_dependency_requirements.rs"]
mod requirements;

use activity::DependencyActivityContext;
pub use activity::{DependencyActivityEmitter, DependencyActivityEvent};
use descriptors::ResolvedModelDescriptor;

/// Shared dependency resolver state.
pub type SharedModelDependencyResolver = Arc<TauriModelDependencyResolver>;

const SUPPORTED_DEPENDENCY_CONTRACT_VERSION: u32 = 1;

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
        activity::emit_activity_with_emitter(
            Some(emitter),
            context,
            phase,
            message,
            binding_id,
            requirement_name,
            stream,
        );
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

    fn cache_key(request: &ModelDependencyRequest) -> String {
        descriptors::cache_key(request)
    }

    fn aggregate_binding_runtime_state(rows: &[ModelDependencyBindingStatus]) -> DependencyState {
        requirements::aggregate_binding_runtime_state(rows)
    }

    fn requirement_install_target(requirement: &ModelDependencyRequirement) -> String {
        requirements::requirement_install_target(requirement)
    }

    fn apply_dependency_override_patches(
        requirements: ModelDependencyRequirements,
        patches: &[DependencyOverridePatchV1],
    ) -> Result<ModelDependencyRequirements, String> {
        requirements::apply_dependency_override_patches(requirements, patches)
    }

    async fn get_pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let ext = self.shared_extensions.read().await;
        ext.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
            .cloned()
    }

    async fn resolve_descriptor(
        &self,
        request: &ModelDependencyRequest,
        api: Option<&Arc<pumas_library::PumasApi>>,
    ) -> Result<ResolvedModelDescriptor, String> {
        descriptors::resolve_descriptor(request, api).await
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

    fn pick_bindings_for_execution(
        requirements: &ModelDependencyRequirements,
    ) -> Vec<&ModelDependencyBinding> {
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
            .map(requirements::map_binding)
            .collect::<Vec<_>>();
        requirements::sort_bindings(&mut bindings);
        let selected_binding_ids = requirements::select_binding_ids_for_requirements(
            descriptor.selected_binding_ids.as_ref(),
            &bindings,
        );

        let requirements = ModelDependencyRequirements {
            model_id: raw.model_id,
            platform_key: raw.platform_key,
            backend_key: raw.backend_key,
            dependency_contract_version: raw.dependency_contract_version,
            validation_state: requirements::map_validation_state(raw.validation_state),
            validation_errors: raw
                .validation_errors
                .iter()
                .map(requirements::map_validation_error)
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
                state: requirements::runtime_state_from_validation(
                    requirements.validation_state.clone(),
                ),
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
                state: requirements::runtime_state_from_validation(
                    requirements.validation_state.clone(),
                ),
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

        let requirements_id = descriptors::make_requirements_id(
            &descriptor.model_id,
            descriptor.backend_key.as_deref(),
            &descriptor.platform_key,
            &resolved_requirements.selected_binding_ids,
        );
        let engine = descriptors::infer_engine(
            descriptor.backend_key.as_deref(),
            &request.node_type,
            descriptor.model_type.as_deref(),
        );
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
#[path = "model_dependencies_tests.rs"]
mod tests;

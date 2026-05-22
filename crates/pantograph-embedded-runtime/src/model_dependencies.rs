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
#[path = "model_dependency_operations.rs"]
mod operations;
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

//! Model dependency contracts used by workflow execution.
//!
//! This module defines:
//! - A host-provided dependency resolver trait
//! - Resolver/check/install result contracts
//! - The `model_ref` v2 runtime contract used by inference/unload nodes

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Pantograph-owned runtime lifecycle state for dependency handling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    Unresolved,
    Invalid,
    Resolved,
    Checking,
    Missing,
    Installing,
    Ready,
    Failed,
}

/// Resolver validation state from the Pumas resolve-only dependency contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyValidationState {
    Resolved,
    UnknownProfile,
    InvalidProfile,
    ProfileConflict,
}

/// Validation error scope for resolver contract payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyValidationErrorScope {
    TopLevel,
    Binding,
}

/// Override scope for Pantograph-managed dependency patches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyOverrideScope {
    Binding,
    Requirement,
}

/// Supported override fields for dependency patch contract v1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct DependencyOverrideFieldsV1 {
    #[serde(default)]
    pub python_executable: Option<String>,
    #[serde(default)]
    pub index_url: Option<String>,
    #[serde(default)]
    pub extra_index_urls: Option<Vec<String>>,
    #[serde(default)]
    pub wheel_source_path: Option<String>,
    #[serde(default)]
    pub package_source_override: Option<String>,
}

/// Manual override patch contract v1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyOverridePatchV1 {
    #[serde(default = "default_dependency_override_contract_version")]
    pub contract_version: u32,
    pub binding_id: String,
    pub scope: DependencyOverrideScope,
    #[serde(default)]
    pub requirement_name: Option<String>,
    #[serde(default)]
    pub fields: DependencyOverrideFieldsV1,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_dependency_override_contract_version() -> u32 {
    1
}

/// Structured resolver validation error entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyValidationError {
    pub code: String,
    pub scope: DependencyValidationErrorScope,
    #[serde(default)]
    pub binding_id: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    pub message: String,
}

/// Request payload passed to a host dependency resolver.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyRequest {
    /// Workflow node type currently requesting execution.
    pub node_type: String,
    /// Absolute or relative path to the selected model.
    pub model_path: String,
    /// Optional model ID from model metadata/index.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional model family/type from metadata.
    #[serde(default)]
    pub model_type: Option<String>,
    /// Optional task type hint (e.g. `text-to-audio`).
    #[serde(default)]
    pub task_type_primary: Option<String>,
    /// Optional backend selector (e.g. `pytorch`, `llamacpp`).
    #[serde(default)]
    pub backend_key: Option<String>,
    /// Optional platform context payload forwarded to dependency resolvers.
    #[serde(default)]
    pub platform_context: Option<serde_json::Value>,
    /// Optional selected binding IDs. Empty means resolver default selection.
    #[serde(default)]
    pub selected_binding_ids: Vec<String>,
    /// Optional manual override patches from dependency-environment node.
    #[serde(default)]
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
}

/// Per-binding requirement entry from resolver contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyRequirement {
    pub kind: String,
    pub name: String,
    pub exact_pin: String,
    #[serde(default)]
    pub index_url: Option<String>,
    #[serde(default)]
    pub extra_index_urls: Vec<String>,
    #[serde(default)]
    pub markers: Option<String>,
    #[serde(default)]
    pub python_requires: Option<String>,
    #[serde(default)]
    pub platform_constraints: Vec<String>,
    #[serde(default)]
    pub hashes: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
}

/// A resolved dependency binding row from resolver output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyBinding {
    pub binding_id: String,
    pub profile_id: String,
    pub profile_version: i64,
    #[serde(default)]
    pub profile_hash: Option<String>,
    #[serde(default)]
    pub backend_key: Option<String>,
    #[serde(default)]
    pub platform_selector: Option<String>,
    #[serde(default)]
    pub environment_kind: Option<String>,
    #[serde(default)]
    pub env_id: Option<String>,
    #[serde(default)]
    pub python_executable_override: Option<String>,
    pub validation_state: DependencyValidationState,
    #[serde(default)]
    pub validation_errors: Vec<DependencyValidationError>,
    #[serde(default)]
    pub requirements: Vec<ModelDependencyRequirement>,
}

/// Structured resolver output for dependency requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyRequirements {
    pub model_id: String,
    pub platform_key: String,
    #[serde(default)]
    pub backend_key: Option<String>,
    pub dependency_contract_version: u32,
    pub validation_state: DependencyValidationState,
    #[serde(default)]
    pub validation_errors: Vec<DependencyValidationError>,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBinding>,
    #[serde(default)]
    pub selected_binding_ids: Vec<String>,
}

/// Per-binding status row returned by Pantograph check/install operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyBindingStatus {
    pub binding_id: String,
    #[serde(default)]
    pub env_id: Option<String>,
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub missing_requirements: Vec<String>,
    #[serde(default)]
    pub installed_requirements: Vec<String>,
    #[serde(default)]
    pub failed_requirements: Vec<String>,
}

/// Structured result for dependency checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyStatus {
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    pub requirements: ModelDependencyRequirements,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBindingStatus>,
    #[serde(default)]
    pub checked_at: Option<String>,
}

/// Structured result for dependency installation actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDependencyInstallResult {
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    pub requirements: ModelDependencyRequirements,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBindingStatus>,
    #[serde(default)]
    pub installed_at: Option<String>,
}

/// Canonical model reference contract emitted by inference nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRefV2 {
    pub contract_version: u32,
    pub engine: String,
    pub model_id: String,
    pub model_path: String,
    pub task_type_primary: String,
    #[serde(default)]
    pub dependency_bindings: Vec<ModelDependencyBinding>,
    #[serde(default)]
    pub dependency_requirements_id: Option<String>,
}

impl ModelRefV2 {
    /// Validate a model reference payload from JSON.
    pub fn validate_value(value: &serde_json::Value) -> std::result::Result<Self, String> {
        let model_ref: Self = serde_json::from_value(value.clone())
            .map_err(|e| format!("invalid model_ref payload: {e}"))?;
        model_ref.validate()?;
        Ok(model_ref)
    }

    /// Validate required contract fields.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.contract_version != 2 {
            return Err(format!(
                "model_ref contract_version must be 2, got {}",
                self.contract_version
            ));
        }
        if self.engine.trim().is_empty() {
            return Err("model_ref missing 'engine' field".to_string());
        }
        if self.model_id.trim().is_empty() {
            return Err("model_ref missing 'model_id' field".to_string());
        }
        if self.model_path.trim().is_empty() {
            return Err("model_ref missing 'model_path' field".to_string());
        }
        if self.task_type_primary.trim().is_empty() {
            return Err("model_ref missing 'task_type_primary' field".to_string());
        }

        let mut ids = std::collections::HashSet::new();
        for binding in &self.dependency_bindings {
            if binding.binding_id.trim().is_empty() {
                return Err("model_ref dependency binding missing 'binding_id'".to_string());
            }
            if !ids.insert(binding.binding_id.clone()) {
                return Err(format!(
                    "model_ref contains duplicate dependency binding_id '{}'",
                    binding.binding_id
                ));
            }
        }

        Ok(())
    }
}

/// Host extension trait for resolve/check/install and model-ref hydration.
#[async_trait]
pub trait ModelDependencyResolver: Send + Sync {
    /// Resolve dependency requirements for a model request.
    async fn resolve_model_dependency_requirements(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyRequirements, String>;

    /// Check whether dependencies required for this model request are ready.
    async fn check_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyStatus, String>;

    /// Install dependencies required for this model request.
    async fn install_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyInstallResult, String>;

    /// Resolve enriched `model_ref` contract fields from host metadata.
    async fn resolve_model_ref(
        &self,
        request: ModelDependencyRequest,
        requirements: Option<ModelDependencyRequirements>,
    ) -> std::result::Result<Option<ModelRefV2>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ref_v2_validation_accepts_valid_payload() {
        let value = serde_json::json!({
            "contractVersion": 2,
            "engine": "pytorch",
            "modelId": "stable-audio-open-1.0",
            "modelPath": "/models/stable-audio",
            "taskTypePrimary": "text-to-audio",
            "dependencyRequirementsId": "audio/stabilityai/stable-audio-open-1_0:linux-x86_64:stable_audio",
            "dependencyBindings": [
                {
                    "binding_id": "binding.stable_audio.core.linux_x86_64",
                    "profile_id": "profile.stable_audio.core",
                    "profile_version": 2,
                    "profile_hash": "abc123",
                    "backend_key": "stable_audio",
                    "platform_selector": "linux-x86_64",
                    "environment_kind": "python",
                    "env_id": "python:profile.stable_audio.core:2:abc123:linux-x86_64:stable_audio",
                    "validation_state": "resolved",
                    "validation_errors": [],
                    "requirements": []
                }
            ]
        });
        let parsed = ModelRefV2::validate_value(&value).unwrap();
        assert_eq!(parsed.contract_version, 2);
        assert_eq!(parsed.engine, "pytorch");
        assert_eq!(parsed.dependency_bindings.len(), 1);
    }

    #[test]
    fn model_ref_v2_validation_rejects_duplicate_binding_ids() {
        let value = serde_json::json!({
            "contractVersion": 2,
            "engine": "pytorch",
            "modelId": "stable-audio-open-1.0",
            "modelPath": "/models/stable-audio",
            "taskTypePrimary": "text-to-audio",
            "dependencyBindings": [
                {
                    "binding_id": "dup",
                    "profile_id": "stable-audio",
                    "profile_version": 1,
                    "validation_state": "resolved",
                    "validation_errors": [],
                    "requirements": []
                },
                {
                    "binding_id": "dup",
                    "profile_id": "stable-audio",
                    "profile_version": 1,
                    "validation_state": "resolved",
                    "validation_errors": [],
                    "requirements": []
                }
            ]
        });
        let err = ModelRefV2::validate_value(&value).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn requirements_payload_deserializes_and_accepts_unknown_fields() {
        let value = serde_json::json!({
            "model_id": "audio/stabilityai/stable-audio-open-1_0",
            "platform_key": "linux-x86_64",
            "backend_key": "stable_audio",
            "dependency_contract_version": 1,
            "validation_state": "resolved",
            "validation_errors": [],
            "bindings": [
                {
                    "binding_id": "binding.stable_audio.core.linux_x86_64",
                    "profile_id": "profile.stable_audio.core",
                    "profile_version": 2,
                    "profile_hash": "abc123",
                    "backend_key": "stable_audio",
                    "platform_selector": "linux-x86_64",
                    "environment_kind": "python",
                    "env_id": "python:profile.stable_audio.core:2:abc123:linux-x86_64:stable_audio",
                    "validation_state": "resolved",
                    "validation_errors": [],
                    "requirements": [
                        {
                            "kind": "python_package",
                            "name": "stable-audio-tools",
                            "exact_pin": "==0.0.19",
                            "unknown_field": "ignored"
                        }
                    ],
                    "unknown_nested": true
                }
            ],
            "unknown_top_level": true
        });

        let parsed: ModelDependencyRequirements = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.dependency_contract_version, 1);
        assert_eq!(parsed.bindings.len(), 1);
        assert_eq!(parsed.bindings[0].requirements.len(), 1);
        assert_eq!(
            parsed.bindings[0].requirements[0].name,
            "stable-audio-tools"
        );
    }
}

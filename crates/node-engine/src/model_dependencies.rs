//! Model dependency contracts used by workflow execution.
//!
//! This module defines:
//! - A host-provided dependency resolver trait
//! - Dependency plan/check/install result contracts
//! - The `model_ref` v2 runtime contract used by inference/unload nodes

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// High-level dependency state for a model execution environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    Ready,
    Missing,
    Installing,
    Failed,
    UnknownProfile,
    ManualInterventionRequired,
    ProfileConflict,
    RequiredBindingOmitted,
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
}

/// A resolved dependency binding row used for dependency checks and model refs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyBinding {
    pub binding_id: String,
    pub profile_id: String,
    pub profile_version: i64,
    #[serde(default)]
    pub profile_hash: Option<String>,
    pub binding_kind: String,
    #[serde(default)]
    pub backend_key: Option<String>,
    #[serde(default)]
    pub platform_selector: Option<String>,
    pub env_id: String,
    #[serde(default)]
    pub pin_summary: Option<ModelDependencyPinSummary>,
    #[serde(default)]
    pub required_pins: Vec<ModelDependencyRequiredPin>,
    #[serde(default)]
    pub missing_pins: Vec<String>,
}

/// Per-binding dependency pin summary for UI and policy checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyPinSummary {
    pub pinned: bool,
    pub required_count: u32,
    pub pinned_count: u32,
    pub missing_count: u32,
}

/// Per-binding required pin entry with requirement provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyRequiredPin {
    pub name: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Structured dependency plan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyPlan {
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBinding>,
    #[serde(default)]
    pub selected_binding_ids: Vec<String>,
    #[serde(default)]
    pub required_binding_ids: Vec<String>,
    #[serde(default)]
    pub missing_pins: Vec<String>,
}

/// Per-binding status row returned by check/install APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyBindingStatus {
    pub binding_id: String,
    pub env_id: String,
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub missing_components: Vec<String>,
    #[serde(default)]
    pub installed_components: Vec<String>,
    #[serde(default)]
    pub failed_components: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub pin_summary: Option<ModelDependencyPinSummary>,
    #[serde(default)]
    pub required_pins: Vec<ModelDependencyRequiredPin>,
    #[serde(default)]
    pub missing_pins: Vec<String>,
}

/// Structured result for dependency checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyStatus {
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBindingStatus>,
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub missing_pins: Vec<String>,
}

/// Structured result for dependency installation actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyInstallResult {
    pub state: DependencyState,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub bindings: Vec<ModelDependencyBindingStatus>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub missing_pins: Vec<String>,
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
    pub dependency_plan_id: Option<String>,
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

        // Ensure deterministic uniqueness for bindings in runtime payloads.
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

/// Host extension trait for dependency checks/install and model-ref hydration.
#[async_trait]
pub trait ModelDependencyResolver: Send + Sync {
    /// Resolve dependency plan (bindings + selected set) for a model request.
    async fn resolve_model_dependency_plan(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyPlan, String>;

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
        plan: Option<ModelDependencyPlan>,
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
            "dependencyPlanId": "stable-audio-open-1.0:linux-x86_64:pytorch",
            "dependencyBindings": [
                {
                    "bindingId": "stable-audio-default",
                    "profileId": "stable-audio",
                    "profileVersion": 1,
                    "profileHash": "abc123",
                    "bindingKind": "required",
                    "backendKey": "pytorch",
                    "platformSelector": "linux-x86_64",
                    "envId": "venv:stable-audio:1:abc123:linux-x86_64:pytorch"
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
                    "bindingId": "dup",
                    "profileId": "stable-audio",
                    "profileVersion": 1,
                    "bindingKind": "required",
                    "envId": "one"
                },
                {
                    "bindingId": "dup",
                    "profileId": "stable-audio",
                    "profileVersion": 1,
                    "bindingKind": "required",
                    "envId": "two"
                }
            ]
        });
        let err = ModelRefV2::validate_value(&value).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn dependency_plan_deserializes_pin_fields_and_missing_pins() {
        let value = serde_json::json!({
            "state": "manual_intervention_required",
            "code": "unpinned_dependency",
            "missingPins": ["torch"],
            "bindings": [
                {
                    "bindingId": "b1",
                    "profileId": "profile.pytorch",
                    "profileVersion": 2,
                    "bindingKind": "required",
                    "envId": "venv:profile.pytorch:2",
                    "pinSummary": {
                        "pinned": false,
                        "requiredCount": 2,
                        "pinnedCount": 1,
                        "missingCount": 1
                    },
                    "requiredPins": [
                        { "name": "torch", "reasons": ["backend_required"] },
                        { "name": "torchvision", "reasons": ["modality_required"] }
                    ],
                    "missingPins": ["torch"]
                }
            ]
        });

        let parsed: ModelDependencyPlan = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.missing_pins, vec!["torch".to_string()]);
        assert_eq!(parsed.bindings.len(), 1);
        assert_eq!(parsed.bindings[0].required_pins.len(), 2);
        assert_eq!(parsed.bindings[0].missing_pins, vec!["torch".to_string()]);
        assert_eq!(
            parsed.bindings[0].pin_summary.as_ref().map(|summary| summary.missing_count),
            Some(1)
        );
    }

    #[test]
    fn dependency_status_deserializes_binding_code_and_pin_fields() {
        let value = serde_json::json!({
            "state": "manual_intervention_required",
            "code": "unpinned_dependency",
            "missingPins": ["torch"],
            "bindings": [
                {
                    "bindingId": "b1",
                    "envId": "venv:profile.pytorch:2",
                    "state": "manual_intervention_required",
                    "code": "unpinned_dependency",
                    "missingComponents": ["profile.pytorch@2"],
                    "pinSummary": {
                        "pinned": false,
                        "requiredCount": 1,
                        "pinnedCount": 0,
                        "missingCount": 1
                    },
                    "requiredPins": [{ "name": "torch", "reasons": ["backend_required"] }],
                    "missingPins": ["torch"]
                }
            ]
        });

        let parsed: ModelDependencyStatus = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.missing_pins, vec!["torch".to_string()]);
        assert_eq!(parsed.bindings.len(), 1);
        assert_eq!(parsed.bindings[0].code.as_deref(), Some("unpinned_dependency"));
        assert_eq!(parsed.bindings[0].required_pins.len(), 1);
        assert_eq!(parsed.bindings[0].missing_pins, vec!["torch".to_string()]);
    }

    #[test]
    fn dependency_plan_ignores_unknown_fields() {
        let value = serde_json::json!({
            "state": "ready",
            "unknownTopLevel": { "future": true },
            "bindings": [
                {
                    "bindingId": "b1",
                    "profileId": "profile",
                    "profileVersion": 1,
                    "bindingKind": "required",
                    "envId": "env",
                    "unknownNested": ["future"]
                }
            ]
        });

        let parsed: ModelDependencyPlan = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.state, DependencyState::Ready);
        assert_eq!(parsed.bindings.len(), 1);
        assert_eq!(parsed.bindings[0].binding_id, "b1");
    }
}

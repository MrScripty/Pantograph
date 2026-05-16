//! Runtime/package dependency requirement declarations owned by inference.
//!
//! These declarations are factual contract data. They do not probe the local
//! environment, rank candidates, select runtimes, or dispatch workers.

use serde::{Deserialize, Serialize};

use crate::capability_availability::{
    CapabilityAvailabilityId, CapabilityAvailabilityState, DependencyReadinessFact,
    DependencyReadinessResolverOwner, DependencyReadinessSubjectKind,
};
use crate::device_contracts::{BackendId, RuntimeVariantId};
use crate::model_contracts::InferenceTaskId;

const PYTORCH_RUNTIME_ID: &str = "pytorch";
const PYTORCH_DIFFUSERS_IMAGE_PACKAGES: &[&str] =
    &["diffusers", "transformers", "accelerate", "torch", "pillow"];

/// Whether a declared package/dependency is mandatory for execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyRequirementNecessity {
    /// Execution must not proceed without this package/dependency.
    Required,
    /// The package/dependency unlocks optional behavior but is not mandatory.
    Optional,
}

impl DependencyRequirementNecessity {
    /// Return true when the requirement must be ready before execution.
    #[must_use]
    pub fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Inference-owned declaration that a runtime/task needs one package/dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyRequirementDeclaration {
    /// Package/dependency subject kind.
    pub subject_kind: DependencyReadinessSubjectKind,
    /// Executable runtime/backend that owns the requirement.
    pub runtime_id: BackendId,
    /// Concrete runtime variant that owns the requirement, when variant-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantId>,
    /// Canonical task scope when the requirement is task-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<InferenceTaskId>,
    /// Model-family scope when the requirement is family-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_family_id: Option<CapabilityAvailabilityId>,
    /// Package/dependency id such as `torch` or `diffusers`.
    pub dependency_id: CapabilityAvailabilityId,
    /// Whether execution requires this dependency.
    pub necessity: DependencyRequirementNecessity,
}

impl DependencyRequirementDeclaration {
    /// Build a package requirement from validated parts.
    #[must_use]
    pub fn package(
        runtime_id: BackendId,
        dependency_id: CapabilityAvailabilityId,
        necessity: DependencyRequirementNecessity,
    ) -> Self {
        Self::new(
            DependencyReadinessSubjectKind::Package,
            runtime_id,
            dependency_id,
            necessity,
        )
    }

    /// Build a managed-dependency requirement from validated parts.
    #[must_use]
    pub fn dependency(
        runtime_id: BackendId,
        dependency_id: CapabilityAvailabilityId,
        necessity: DependencyRequirementNecessity,
    ) -> Self {
        Self::new(
            DependencyReadinessSubjectKind::Dependency,
            runtime_id,
            dependency_id,
            necessity,
        )
    }

    fn new(
        subject_kind: DependencyReadinessSubjectKind,
        runtime_id: BackendId,
        dependency_id: CapabilityAvailabilityId,
        necessity: DependencyRequirementNecessity,
    ) -> Self {
        Self {
            subject_kind,
            runtime_id,
            runtime_variant_id: None,
            task_id: None,
            model_family_id: None,
            dependency_id,
            necessity,
        }
    }

    /// Attach a runtime variant scope.
    #[must_use]
    pub fn with_runtime_variant_id(mut self, runtime_variant_id: RuntimeVariantId) -> Self {
        self.runtime_variant_id = Some(runtime_variant_id);
        self
    }

    /// Attach a task scope.
    #[must_use]
    pub fn with_task_id(mut self, task_id: InferenceTaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Attach a model-family scope.
    #[must_use]
    pub fn with_model_family_id(mut self, model_family_id: CapabilityAvailabilityId) -> Self {
        self.model_family_id = Some(model_family_id);
        self
    }

    /// Return true when this requirement is mandatory for execution.
    #[must_use]
    pub fn is_required(&self) -> bool {
        self.necessity.is_required()
    }

    /// Project a resolved state for this requirement into scheduler readiness proof.
    #[must_use]
    pub fn to_readiness_fact(
        &self,
        state: CapabilityAvailabilityState,
        resolver_owner: DependencyReadinessResolverOwner,
    ) -> DependencyReadinessFact {
        let mut fact = match self.subject_kind {
            DependencyReadinessSubjectKind::Package => DependencyReadinessFact::package(
                self.runtime_id.clone(),
                self.dependency_id.clone(),
                state,
                resolver_owner,
            ),
            DependencyReadinessSubjectKind::Dependency => DependencyReadinessFact::dependency(
                self.runtime_id.clone(),
                self.dependency_id.clone(),
                state,
                resolver_owner,
            ),
        };

        if let Some(runtime_variant_id) = self.runtime_variant_id.clone() {
            fact = fact.with_runtime_variant_id(runtime_variant_id);
        }
        if let Some(task_id) = self.task_id.clone() {
            fact = fact.with_task_id(task_id);
        }
        if let Some(model_family_id) = self.model_family_id.clone() {
            fact = fact.with_model_family_id(model_family_id);
        }

        fact
    }
}

/// Inference-owned package requirements for PyTorch/Diffusers image execution.
#[must_use]
pub fn pytorch_diffusers_image_generation_package_requirements(
) -> Vec<DependencyRequirementDeclaration> {
    let runtime_id =
        BackendId::parse(PYTORCH_RUNTIME_ID).expect("pytorch runtime id must be valid");

    PYTORCH_DIFFUSERS_IMAGE_PACKAGES
        .iter()
        .map(|dependency_id| {
            DependencyRequirementDeclaration::package(
                runtime_id.clone(),
                CapabilityAvailabilityId::parse(dependency_id)
                    .expect("pytorch diffusers package id must be valid"),
                DependencyRequirementNecessity::Required,
            )
            .with_task_id(InferenceTaskId::ImageGeneration)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn package_ids(declarations: &[DependencyRequirementDeclaration]) -> Vec<&str> {
        declarations
            .iter()
            .map(|declaration| declaration.dependency_id.as_str())
            .collect()
    }

    #[test]
    fn pytorch_diffusers_image_requirements_are_factual_required_packages() {
        let declarations = pytorch_diffusers_image_generation_package_requirements();

        assert_eq!(
            package_ids(&declarations),
            vec!["diffusers", "transformers", "accelerate", "torch", "pillow"]
        );
        for declaration in declarations {
            assert_eq!(
                declaration.subject_kind,
                DependencyReadinessSubjectKind::Package
            );
            assert_eq!(declaration.runtime_id.as_str(), "pytorch");
            assert_eq!(declaration.task_id, Some(InferenceTaskId::ImageGeneration));
            assert_eq!(
                declaration.necessity,
                DependencyRequirementNecessity::Required
            );
            assert!(declaration.is_required());
            assert!(declaration.runtime_variant_id.is_none());
            assert!(declaration.model_family_id.is_none());
        }
    }

    #[test]
    fn dependency_requirement_declaration_serde_keeps_optional_scope_defaulted() {
        let declaration: DependencyRequirementDeclaration = serde_json::from_value(json!({
            "subject_kind": "package",
            "runtime_id": "pytorch",
            "task_id": "image_generation",
            "dependency_id": "torch",
            "necessity": "required"
        }))
        .expect("decode requirement declaration");

        assert_eq!(
            declaration.subject_kind,
            DependencyReadinessSubjectKind::Package
        );
        assert_eq!(declaration.runtime_id.as_str(), "pytorch");
        assert_eq!(declaration.task_id, Some(InferenceTaskId::ImageGeneration));
        assert_eq!(declaration.dependency_id.as_str(), "torch");
        assert_eq!(
            declaration.necessity,
            DependencyRequirementNecessity::Required
        );
        assert!(declaration.runtime_variant_id.is_none());
        assert!(declaration.model_family_id.is_none());
    }

    #[test]
    fn dependency_requirement_declaration_validates_ids() {
        let error = serde_json::from_value::<DependencyRequirementDeclaration>(json!({
            "subject_kind": "package",
            "runtime_id": "PyTorch",
            "dependency_id": "torch",
            "necessity": "required"
        }))
        .expect_err("invalid runtime id must fail");

        assert!(error.to_string().contains("invalid identifier shape"));

        let error = serde_json::from_value::<DependencyRequirementDeclaration>(json!({
            "subject_kind": "package",
            "runtime_id": "pytorch",
            "dependency_id": "Torch",
            "necessity": "required"
        }))
        .expect_err("invalid dependency id must fail");

        assert!(error.to_string().contains("invalid identifier shape"));
    }

    #[test]
    fn dependency_requirement_projection_preserves_scope_without_policy() {
        let declaration = pytorch_diffusers_image_generation_package_requirements()
            .into_iter()
            .find(|declaration| declaration.dependency_id.as_str() == "diffusers")
            .expect("diffusers requirement");

        let readiness = declaration.to_readiness_fact(
            CapabilityAvailabilityState::NotInstalled,
            DependencyReadinessResolverOwner::EmbeddedRuntime,
        );

        assert_eq!(
            readiness.subject_kind,
            DependencyReadinessSubjectKind::Package
        );
        assert_eq!(readiness.runtime_id.as_str(), "pytorch");
        assert_eq!(readiness.task_id, Some(InferenceTaskId::ImageGeneration));
        assert_eq!(readiness.dependency_id.as_str(), "diffusers");
        assert_eq!(readiness.state, CapabilityAvailabilityState::NotInstalled);
        assert_eq!(
            readiness.resolver_owner,
            DependencyReadinessResolverOwner::EmbeddedRuntime
        );
        assert!(!readiness.is_ready());
    }
}

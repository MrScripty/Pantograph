//! Embedded-runtime dependency-readiness resolution.
//!
//! This module maps inference-owned dependency declarations plus host-observed
//! package state into typed readiness facts. It does not probe Python, install
//! packages, rank candidates, or select runtimes.

use std::collections::BTreeSet;

use inference::{
    CapabilityAvailabilityError, CapabilityAvailabilityId, CapabilityAvailabilityReason,
    CapabilityAvailabilityState, DependencyReadinessFact, DependencyReadinessResolverOwner,
    DependencyReadinessSubjectKind, DependencyRequirementDeclaration,
};

const PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE: &str = "python_runtime_unavailable";
const PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE: &str = "python_package_not_installed";
const UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE: &str = "unsupported_dependency_kind";

/// Host-observed Python package state supplied to the readiness resolver.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PythonPackageReadinessSnapshot {
    /// Whether the Python sidecar executable was available to the host.
    pub python_available: bool,
    /// Validated package ids known to be import/install-ready.
    pub installed_package_ids: BTreeSet<CapabilityAvailabilityId>,
    /// Bounded reason for Python runtime unavailability, when unavailable.
    pub unavailable_reason: Option<CapabilityAvailabilityReason>,
}

impl PythonPackageReadinessSnapshot {
    /// Build a snapshot for an available Python runtime with known packages.
    #[must_use]
    pub fn available(installed_package_ids: BTreeSet<CapabilityAvailabilityId>) -> Self {
        Self {
            python_available: true,
            installed_package_ids,
            unavailable_reason: None,
        }
    }

    /// Build a snapshot for an unavailable Python runtime.
    #[must_use]
    pub fn unavailable(unavailable_reason: CapabilityAvailabilityReason) -> Self {
        Self {
            python_available: false,
            installed_package_ids: BTreeSet::new(),
            unavailable_reason: Some(unavailable_reason),
        }
    }
}

/// Resolve Python package declarations into dependency-readiness facts.
pub fn resolve_python_package_readiness(
    declarations: &[DependencyRequirementDeclaration],
    snapshot: &PythonPackageReadinessSnapshot,
) -> Result<Vec<DependencyReadinessFact>, CapabilityAvailabilityError> {
    declarations
        .iter()
        .map(|declaration| resolve_python_package_readiness_fact(declaration, snapshot))
        .collect()
}

fn resolve_python_package_readiness_fact(
    declaration: &DependencyRequirementDeclaration,
    snapshot: &PythonPackageReadinessSnapshot,
) -> Result<DependencyReadinessFact, CapabilityAvailabilityError> {
    if declaration.subject_kind != DependencyReadinessSubjectKind::Package {
        return readiness_with_reason(
            declaration,
            CapabilityAvailabilityState::RequiresRuntimeCapability,
            UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE,
            "Embedded Python dependency readiness only resolves package requirements.",
        );
    }

    if !snapshot.python_available {
        let reason = snapshot
            .unavailable_reason
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "Python runtime is not available.".to_string());
        return readiness_with_reason(
            declaration,
            CapabilityAvailabilityState::MissingDependency,
            PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE,
            &reason,
        );
    }

    if snapshot
        .installed_package_ids
        .contains(&declaration.dependency_id)
    {
        return Ok(declaration.to_readiness_fact(
            CapabilityAvailabilityState::Available,
            DependencyReadinessResolverOwner::EmbeddedRuntime,
        ));
    }

    readiness_with_reason(
        declaration,
        CapabilityAvailabilityState::NotInstalled,
        PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE,
        &format!(
            "Python package '{}' is not installed for runtime '{}'.",
            declaration.dependency_id, declaration.runtime_id
        ),
    )
}

fn readiness_with_reason(
    declaration: &DependencyRequirementDeclaration,
    state: CapabilityAvailabilityState,
    reason_code: &str,
    reason: &str,
) -> Result<DependencyReadinessFact, CapabilityAvailabilityError> {
    Ok(declaration
        .to_readiness_fact(state, DependencyReadinessResolverOwner::EmbeddedRuntime)
        .with_reason_code(CapabilityAvailabilityId::parse(reason_code)?)
        .with_reason(CapabilityAvailabilityReason::parse(reason)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn availability_id(value: &str) -> CapabilityAvailabilityId {
        CapabilityAvailabilityId::parse(value).expect("valid availability id")
    }

    fn reason(value: &str) -> CapabilityAvailabilityReason {
        CapabilityAvailabilityReason::parse(value).expect("valid reason")
    }

    fn installed_package_ids(values: &[&str]) -> BTreeSet<CapabilityAvailabilityId> {
        values.iter().map(|value| availability_id(value)).collect()
    }

    #[test]
    fn resolves_available_pytorch_diffusers_packages_without_policy() {
        let declarations = inference::pytorch_diffusers_image_generation_package_requirements();
        let snapshot = PythonPackageReadinessSnapshot::available(installed_package_ids(&[
            "diffusers",
            "transformers",
            "accelerate",
            "torch",
            "pillow",
        ]));

        let facts = resolve_python_package_readiness(&declarations, &snapshot)
            .expect("resolve package readiness");

        assert_eq!(facts.len(), 5);
        for fact in facts {
            assert_eq!(fact.runtime_id.as_str(), "pytorch");
            assert_eq!(fact.state, CapabilityAvailabilityState::Available);
            assert_eq!(
                fact.resolver_owner,
                DependencyReadinessResolverOwner::EmbeddedRuntime
            );
            assert!(fact.reason_code.is_none());
            assert!(fact.reason.is_none());
            assert!(fact.is_ready());
        }
    }

    #[test]
    fn missing_python_package_resolves_to_not_installed_readiness() {
        let declarations = inference::pytorch_diffusers_image_generation_package_requirements();
        let snapshot = PythonPackageReadinessSnapshot::available(installed_package_ids(&[
            "transformers",
            "accelerate",
            "torch",
            "pillow",
        ]));

        let facts = resolve_python_package_readiness(&declarations, &snapshot)
            .expect("resolve package readiness");
        let diffusers = facts
            .iter()
            .find(|fact| fact.dependency_id.as_str() == "diffusers")
            .expect("diffusers readiness fact");

        assert_eq!(diffusers.state, CapabilityAvailabilityState::NotInstalled);
        assert_eq!(
            diffusers.reason_code.as_ref().unwrap().as_str(),
            PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE
        );
        assert_eq!(
            diffusers.reason.as_ref().unwrap().as_str(),
            "Python package 'diffusers' is not installed for runtime 'pytorch'."
        );
        assert!(!diffusers.is_ready());
    }

    #[test]
    fn unavailable_python_runtime_blocks_all_python_packages() {
        let declarations = inference::pytorch_diffusers_image_generation_package_requirements();
        let snapshot = PythonPackageReadinessSnapshot::unavailable(reason(
            "Python runtime is not configured.",
        ));

        let facts = resolve_python_package_readiness(&declarations, &snapshot)
            .expect("resolve package readiness");

        assert_eq!(facts.len(), 5);
        for fact in facts {
            assert_eq!(fact.state, CapabilityAvailabilityState::MissingDependency);
            assert_eq!(
                fact.reason_code.as_ref().unwrap().as_str(),
                PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE
            );
            assert_eq!(
                fact.reason.as_ref().unwrap().as_str(),
                "Python runtime is not configured."
            );
            assert!(!fact.is_ready());
        }
    }

    #[test]
    fn unsupported_declaration_kind_resolves_to_non_selectable_fact() {
        let declaration = DependencyRequirementDeclaration::dependency(
            inference::BackendId::parse("pytorch").expect("valid backend"),
            availability_id("pytorch_sidecar"),
            inference::DependencyRequirementNecessity::Required,
        );
        let snapshot =
            PythonPackageReadinessSnapshot::available(installed_package_ids(&["pytorch_sidecar"]));

        let facts = resolve_python_package_readiness(&[declaration], &snapshot)
            .expect("resolve package readiness");

        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].state,
            CapabilityAvailabilityState::RequiresRuntimeCapability
        );
        assert_eq!(
            facts[0].reason_code.as_ref().unwrap().as_str(),
            UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE
        );
        assert!(!facts[0].is_ready());
    }
}

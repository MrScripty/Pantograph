use std::collections::{BTreeMap, BTreeSet};

use inference::{BackendId, CapabilityAvailabilityId};
use pantograph_dependency_environment_service::DependencyRequirementsPayload;
use pantograph_dependency_planning::{
    DependencyBindingProfileId, DependencyEnvironmentKind, DependencyRequirementBinding,
    DependencyRequirementKind, ValidatedDependencyEnvironmentRequest,
};

use crate::package_readiness_provider::{
    PackageReadinessEnvironmentSelector, PackageReadinessProbeRequest,
};

pub(crate) fn python_probe_request_for_payload(
    request: &ValidatedDependencyEnvironmentRequest,
    payload: &DependencyRequirementsPayload,
) -> Result<PackageReadinessProbeRequest, ProbeShapeError> {
    let selected_bindings = selected_bindings(payload);
    let requirement_by_name = payload
        .requirements
        .iter()
        .map(|requirement| (requirement.name.clone(), requirement))
        .collect::<BTreeMap<_, _>>();
    let mut dependency_ids = BTreeSet::new();

    for binding in &selected_bindings {
        if binding.environment_kind != DependencyEnvironmentKind::Python {
            return Err(ProbeShapeError::new(
                "dependency_environment.bindings.environment_kind",
                "Dependency readiness probes currently support only Python package bindings.",
            ));
        }
        let Some(requirement) = requirement_by_name.get(&binding.requirement_name) else {
            return Err(ProbeShapeError::new(
                "dependency_environment.bindings.requirement_name",
                "Selected dependency binding references an unknown requirement.",
            ));
        };
        if requirement.kind != DependencyRequirementKind::PythonPackage {
            return Err(ProbeShapeError::new(
                "dependency_environment.requirements.kind",
                "Dependency readiness probes currently support only Python package requirements.",
            ));
        }
        let dependency_id = match CapabilityAvailabilityId::parse(requirement.name.as_str()) {
            Ok(dependency_id) => dependency_id,
            Err(_) => {
                return Err(ProbeShapeError::new(
                    "dependency_environment.requirements.name",
                    "Python package requirement name is not a valid probe id.",
                ));
            }
        };
        dependency_ids.insert(dependency_id);
    }

    let Some(runtime_id) = payload
        .identity_key
        .scheduler_intent
        .requested_runtime_id
        .as_ref()
    else {
        return Err(ProbeShapeError::new(
            "dependency_environment.identity_key.scheduler_intent.requested_runtime_id",
            "Dependency readiness package probes require an explicit scheduler runtime id.",
        ));
    };
    let executable_backend_key = BackendId::parse(runtime_id.as_str()).map_err(|_| {
        ProbeShapeError::new(
            "dependency_environment.identity_key.scheduler_intent.requested_runtime_id",
            "Scheduler runtime id is not a valid executable backend id.",
        )
    })?;
    let scheduler_runtime_id =
        CapabilityAvailabilityId::parse(runtime_id.as_str()).map_err(|_| {
            ProbeShapeError::new(
                "dependency_environment.identity_key.scheduler_intent.requested_runtime_id",
                "Scheduler runtime id is not a valid package probe attribution id.",
            )
        })?;

    Ok(PackageReadinessProbeRequest {
        executable_backend_key,
        scheduler_runtime_id,
        runtime_variant_id: None,
        environment: python_environment_selector(request, &selected_bindings)?,
        dependency_ids: dependency_ids.into_iter().collect(),
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbeShapeError {
    pub(crate) field_path: &'static str,
    pub(crate) message: &'static str,
}

impl ProbeShapeError {
    fn new(field_path: &'static str, message: &'static str) -> Self {
        Self {
            field_path,
            message,
        }
    }
}

fn python_environment_selector(
    request: &ValidatedDependencyEnvironmentRequest,
    selected_bindings: &[DependencyRequirementBinding],
) -> Result<PackageReadinessEnvironmentSelector, ProbeShapeError> {
    if let Some(environment_ref) = request.as_request().environment_ref.as_ref() {
        return Ok(PackageReadinessEnvironmentSelector::PythonEnvironment {
            environment_id: capability_id_from_environment_id(
                environment_ref.environment_id.as_str(),
            )?,
        });
    }

    let selected_profiles = selected_bindings
        .iter()
        .filter_map(|binding| binding.profile_id.as_ref())
        .collect::<BTreeSet<_>>();
    match selected_profiles.len() {
        0 => Ok(PackageReadinessEnvironmentSelector::DefaultHostPython),
        1 => {
            let profile = selected_profiles
                .into_iter()
                .next()
                .expect("one selected profile exists");
            Ok(PackageReadinessEnvironmentSelector::PythonEnvironment {
                environment_id: capability_id_from_profile_id(profile)?,
            })
        }
        _ => Err(ProbeShapeError::new(
            "dependency_environment.bindings.profile_id",
            "Dependency readiness probes require selected Python bindings to target one environment profile.",
        )),
    }
}

fn capability_id_from_environment_id(
    environment_id: &str,
) -> Result<CapabilityAvailabilityId, ProbeShapeError> {
    CapabilityAvailabilityId::parse(environment_id).map_err(|_| {
        ProbeShapeError::new(
            "dependency_environment.request.environment_ref.environment_id",
            "Dependency environment id is not a valid package probe environment id.",
        )
    })
}

fn capability_id_from_profile_id(
    profile_id: &DependencyBindingProfileId,
) -> Result<CapabilityAvailabilityId, ProbeShapeError> {
    CapabilityAvailabilityId::parse(profile_id.as_str()).map_err(|_| {
        ProbeShapeError::new(
            "dependency_environment.bindings.profile_id",
            "Dependency binding profile id is not a valid package probe environment id.",
        )
    })
}

fn selected_bindings(payload: &DependencyRequirementsPayload) -> Vec<DependencyRequirementBinding> {
    let selected_ids = payload.selected_binding_ids.iter().collect::<BTreeSet<_>>();
    payload
        .bindings
        .iter()
        .filter(|binding| selected_ids.contains(&binding.binding_id))
        .cloned()
        .collect()
}

//! Per-selected-binding dependency inventory dispatch.
//!
//! This module owns provider registration, dispatch planning, scoped payload
//! routing, and provider-owned not-implemented observations. Concrete provider
//! modules own source-specific observation logic.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use inference::CapabilityAvailabilityReason;
use pantograph_dependency_environment_service::DependencyRequirementsPayload;
use pantograph_dependency_planning::{
    DependencyBindingId, DependencyEnvironmentKind, DependencyRequirementBinding,
    DependencyRequirementKind, DependencyRequirementName,
};

use crate::dependency_environment_probe_selector::ProbeShapeError;
use crate::dependency_environment_probe_snapshot::{
    invalid_probe_shape_observations, observations_from_probe_failures,
};
use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};
use crate::package_readiness_provider::{
    PackageReadinessProbeFailure, PackageReadinessProviderDiagnosticCode,
};

pub(crate) struct DependencyInventoryDispatchProvider {
    python_provider: Arc<dyn DependencyInventoryProvider>,
    #[cfg(any(test, feature = "standalone"))]
    managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
    #[cfg(any(test, feature = "standalone"))]
    runtime_feature_provider: Arc<dyn DependencyInventoryProvider>,
    #[cfg(any(test, feature = "standalone"))]
    device_toolchain_provider: Arc<dyn DependencyInventoryProvider>,
    not_implemented_provider: NotImplementedDependencyInventoryProvider,
}

impl DependencyInventoryDispatchProvider {
    pub(crate) fn new(python_provider: Arc<dyn DependencyInventoryProvider>) -> Self {
        Self {
            python_provider,
            #[cfg(any(test, feature = "standalone"))]
            managed_runtime_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            #[cfg(any(test, feature = "standalone"))]
            runtime_feature_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            #[cfg(any(test, feature = "standalone"))]
            device_toolchain_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_managed_runtime(
        python_provider: Arc<dyn DependencyInventoryProvider>,
        managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
    ) -> Self {
        Self {
            python_provider,
            managed_runtime_provider,
            runtime_feature_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            device_toolchain_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_managed_runtime_and_runtime_feature(
        python_provider: Arc<dyn DependencyInventoryProvider>,
        managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
        runtime_feature_provider: Arc<dyn DependencyInventoryProvider>,
    ) -> Self {
        Self {
            python_provider,
            managed_runtime_provider,
            runtime_feature_provider,
            device_toolchain_provider: Arc::new(NotImplementedDependencyInventoryProvider),
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }

    #[cfg(any(test, feature = "standalone"))]
    pub(crate) fn new_with_managed_runtime_and_runtime_feature_and_device_toolchain(
        python_provider: Arc<dyn DependencyInventoryProvider>,
        managed_runtime_provider: Arc<dyn DependencyInventoryProvider>,
        runtime_feature_provider: Arc<dyn DependencyInventoryProvider>,
        device_toolchain_provider: Arc<dyn DependencyInventoryProvider>,
    ) -> Self {
        Self {
            python_provider,
            managed_runtime_provider,
            runtime_feature_provider,
            device_toolchain_provider,
            not_implemented_provider: NotImplementedDependencyInventoryProvider,
        }
    }
}

#[async_trait]
impl DependencyInventoryProvider for DependencyInventoryDispatchProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        let dispatch_plan = DependencyInventoryDispatchPlan::for_payload(&request.payload);
        let mut rows = Vec::new();
        let mut diagnostics = Vec::new();

        if !dispatch_plan.invalid_binding_ids.is_empty() {
            let payload = scoped_payload(&request.payload, &dispatch_plan.invalid_binding_ids);
            let (invalid_rows, invalid_diagnostics) = invalid_probe_shape_observations(
                &request.item,
                &payload,
                ProbeShapeError {
                    field_path: "dependency_environment.bindings",
                    message: "Selected dependency binding does not match the referenced requirement kind.",
                },
            );
            rows.extend(invalid_rows);
            diagnostics.extend(invalid_diagnostics);
        }

        if !dispatch_plan.python_binding_ids.is_empty() {
            let payload = scoped_payload(&request.payload, &dispatch_plan.python_binding_ids);
            let observation = self
                .python_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(any(test, feature = "standalone"))]
        if !dispatch_plan.managed_runtime_binding_ids.is_empty() {
            let payload =
                scoped_payload(&request.payload, &dispatch_plan.managed_runtime_binding_ids);
            let observation = self
                .managed_runtime_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(any(test, feature = "standalone"))]
        if !dispatch_plan.runtime_feature_binding_ids.is_empty() {
            let payload =
                scoped_payload(&request.payload, &dispatch_plan.runtime_feature_binding_ids);
            let observation = self
                .runtime_feature_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(any(test, feature = "standalone"))]
        if !dispatch_plan.device_toolchain_binding_ids.is_empty() {
            let payload = scoped_payload(
                &request.payload,
                &dispatch_plan.device_toolchain_binding_ids,
            );
            let observation = self
                .device_toolchain_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        #[cfg(not(any(test, feature = "standalone")))]
        let not_implemented_binding_ids = dispatch_plan
            .not_implemented_binding_ids
            .iter()
            .chain(dispatch_plan.managed_runtime_binding_ids.iter())
            .chain(dispatch_plan.runtime_feature_binding_ids.iter())
            .chain(dispatch_plan.device_toolchain_binding_ids.iter())
            .collect::<Vec<_>>();
        #[cfg(any(test, feature = "standalone"))]
        let not_implemented_binding_ids = dispatch_plan
            .not_implemented_binding_ids
            .iter()
            .collect::<Vec<_>>();

        for binding_id in not_implemented_binding_ids {
            let payload = scoped_payload(&request.payload, std::slice::from_ref(binding_id));
            let observation = self
                .not_implemented_provider
                .observe(request.with_payload(payload))
                .await;
            rows.extend(observation.rows);
            diagnostics.extend(observation.diagnostics);
        }

        DependencyInventoryObservation::new(rows, diagnostics)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DependencyInventoryDispatchPlan {
    python_binding_ids: Vec<DependencyBindingId>,
    managed_runtime_binding_ids: Vec<DependencyBindingId>,
    runtime_feature_binding_ids: Vec<DependencyBindingId>,
    device_toolchain_binding_ids: Vec<DependencyBindingId>,
    not_implemented_binding_ids: Vec<DependencyBindingId>,
    invalid_binding_ids: Vec<DependencyBindingId>,
}

impl DependencyInventoryDispatchPlan {
    fn for_payload(payload: &DependencyRequirementsPayload) -> Self {
        let requirement_by_name = payload
            .requirements
            .iter()
            .map(|requirement| (requirement.name.clone(), requirement.kind))
            .collect::<BTreeMap<_, _>>();

        let mut plan = Self::default();
        for binding in selected_bindings(payload) {
            let Some(requirement_kind) = requirement_by_name.get(&binding.requirement_name) else {
                plan.invalid_binding_ids.push(binding.binding_id);
                continue;
            };
            match dispatch_target(binding.environment_kind, *requirement_kind) {
                Some(DependencyInventoryDispatchTarget::PythonPackage) => {
                    plan.python_binding_ids.push(binding.binding_id);
                }
                #[cfg(any(test, feature = "standalone"))]
                Some(DependencyInventoryDispatchTarget::ManagedRuntime) => {
                    plan.managed_runtime_binding_ids.push(binding.binding_id);
                }
                #[cfg(any(test, feature = "standalone"))]
                Some(DependencyInventoryDispatchTarget::RuntimeFeature) => {
                    plan.runtime_feature_binding_ids.push(binding.binding_id);
                }
                #[cfg(any(test, feature = "standalone"))]
                Some(DependencyInventoryDispatchTarget::DeviceToolchain) => {
                    plan.device_toolchain_binding_ids.push(binding.binding_id);
                }
                Some(DependencyInventoryDispatchTarget::NotImplemented) => {
                    plan.not_implemented_binding_ids.push(binding.binding_id);
                }
                None => plan.invalid_binding_ids.push(binding.binding_id),
            }
        }
        plan
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyInventoryDispatchTarget {
    PythonPackage,
    #[cfg(any(test, feature = "standalone"))]
    ManagedRuntime,
    #[cfg(any(test, feature = "standalone"))]
    RuntimeFeature,
    #[cfg(any(test, feature = "standalone"))]
    DeviceToolchain,
    NotImplemented,
}

fn dispatch_target(
    environment_kind: DependencyEnvironmentKind,
    requirement_kind: DependencyRequirementKind,
) -> Option<DependencyInventoryDispatchTarget> {
    match (environment_kind, requirement_kind) {
        (DependencyEnvironmentKind::Python, DependencyRequirementKind::PythonPackage) => {
            Some(DependencyInventoryDispatchTarget::PythonPackage)
        }
        (
            DependencyEnvironmentKind::ManagedBinary,
            DependencyRequirementKind::RuntimeManagedBinary,
        ) => Some(managed_runtime_dispatch_target()),
        (DependencyEnvironmentKind::RuntimeFeature, DependencyRequirementKind::RuntimeFeature) => {
            Some(runtime_feature_dispatch_target())
        }
        (
            DependencyEnvironmentKind::DeviceToolchain,
            DependencyRequirementKind::DeviceToolchain,
        ) => Some(device_toolchain_dispatch_target()),
        (DependencyEnvironmentKind::SystemPackage, DependencyRequirementKind::SystemPackage) => {
            Some(DependencyInventoryDispatchTarget::NotImplemented)
        }
        _ => None,
    }
}

#[cfg(any(test, feature = "standalone"))]
fn managed_runtime_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::ManagedRuntime
}

#[cfg(not(any(test, feature = "standalone")))]
fn managed_runtime_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::NotImplemented
}

#[cfg(any(test, feature = "standalone"))]
fn runtime_feature_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::RuntimeFeature
}

#[cfg(not(any(test, feature = "standalone")))]
fn runtime_feature_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::NotImplemented
}

#[cfg(any(test, feature = "standalone"))]
fn device_toolchain_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::DeviceToolchain
}

#[cfg(not(any(test, feature = "standalone")))]
fn device_toolchain_dispatch_target() -> DependencyInventoryDispatchTarget {
    DependencyInventoryDispatchTarget::NotImplemented
}

struct NotImplementedDependencyInventoryProvider;

#[async_trait]
impl DependencyInventoryProvider for NotImplementedDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        let failure = PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::ProbeNotImplemented,
            None,
            CapabilityAvailabilityReason::parse(&not_implemented_reason(&request.payload))
                .expect("inventory provider not implemented reason is valid"),
        );
        let (rows, diagnostics) =
            observations_from_probe_failures(&request.item, &request.payload, vec![failure]);
        DependencyInventoryObservation::new(rows, diagnostics)
    }
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

fn scoped_payload(
    payload: &DependencyRequirementsPayload,
    selected_binding_ids: &[DependencyBindingId],
) -> DependencyRequirementsPayload {
    let selected_ids = selected_binding_ids.iter().collect::<BTreeSet<_>>();
    let bindings = payload
        .bindings
        .iter()
        .filter(|binding| selected_ids.contains(&binding.binding_id))
        .cloned()
        .collect::<Vec<_>>();
    let requirement_names = bindings
        .iter()
        .map(|binding| binding.requirement_name.clone())
        .collect::<BTreeSet<DependencyRequirementName>>();
    let requirements = payload
        .requirements
        .iter()
        .filter(|requirement| requirement_names.contains(&requirement.name))
        .cloned()
        .collect::<Vec<_>>();
    DependencyRequirementsPayload {
        dependency_requirements_id: payload.dependency_requirements_id.clone(),
        identity_key: payload.identity_key.clone(),
        requirements,
        bindings,
        selected_binding_ids: selected_binding_ids.to_vec(),
    }
}

fn not_implemented_reason(payload: &DependencyRequirementsPayload) -> String {
    let selected_count = payload.selected_binding_ids.len();
    format!(
        "Dependency inventory provider is not implemented for {selected_count} selected binding(s)."
    )
}

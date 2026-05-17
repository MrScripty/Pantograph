//! Runtime-scoped package-readiness provider contracts.
//!
//! This module owns the embedded-runtime host-observation boundary for package
//! readiness. It shapes typed provider requests, deduplicates probe work within
//! one technical-fit request, and projects probe snapshots into inference-owned
//! dependency-readiness facts. It does not select runtimes, rank candidates,
//! inspect graph inputs, call dependency-environment preflight, or dispatch
//! workers.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use inference::{
    BackendId, CapabilityAvailabilityId, CapabilityAvailabilityReason, CapabilityAvailabilityState,
    DependencyReadinessFact, DependencyReadinessResolverOwner, DependencyReadinessSubjectKind,
    DependencyRequirementDeclaration, RuntimeVariantId,
};

use crate::dependency_readiness::{
    resolve_python_package_readiness, PythonPackageReadinessSnapshot,
};

const PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE: &str = "python_runtime_unavailable";
const PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE: &str = "python_package_not_installed";
const PROVIDER_PROBE_NOT_IMPLEMENTED_REASON_CODE: &str = "provider_probe_not_implemented";
const PROVIDER_UNSUPPORTED_PLATFORM_REASON_CODE: &str = "provider_unsupported_platform";
const PROVIDER_PROBE_TIMED_OUT_REASON_CODE: &str = "provider_probe_timed_out";
const PROVIDER_PROBE_PROCESS_FAILED_REASON_CODE: &str = "provider_probe_process_failed";
const PROVIDER_INVALID_PACKAGE_ID_REASON_CODE: &str = "provider_invalid_package_id";
const UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE: &str = "unsupported_dependency_kind";

/// Package environment scope used by readiness providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PackageReadinessEnvironmentSelector {
    /// The default host Python executable resolved by embedded-runtime.
    DefaultHostPython,
    /// A typed Python environment id, reserved for managed/runtime-owned envs.
    PythonEnvironment {
        environment_id: CapabilityAvailabilityId,
    },
}

/// One provider request for dependency declarations under one runtime scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReadinessProviderRequest {
    /// Executable backend key used by scheduler candidate matching.
    pub executable_backend_key: BackendId,
    /// Scheduler-facing runtime id for attribution and later managed inventory.
    pub scheduler_runtime_id: CapabilityAvailabilityId,
    /// Runtime variant scope when the package state is variant-specific.
    pub runtime_variant_id: Option<RuntimeVariantId>,
    /// Package environment the provider should inspect.
    pub environment: PackageReadinessEnvironmentSelector,
    /// Inference-owned package/dependency declarations to resolve.
    pub declarations: Vec<DependencyRequirementDeclaration>,
}

impl PackageReadinessProviderRequest {
    /// Build a provider request from validated parts.
    #[must_use]
    pub fn new(
        executable_backend_key: BackendId,
        scheduler_runtime_id: CapabilityAvailabilityId,
        runtime_variant_id: Option<RuntimeVariantId>,
        environment: PackageReadinessEnvironmentSelector,
        declarations: Vec<DependencyRequirementDeclaration>,
    ) -> Self {
        Self {
            executable_backend_key,
            scheduler_runtime_id,
            runtime_variant_id,
            environment,
            declarations,
        }
    }

    fn probe_key(&self) -> PackageReadinessProbeKey {
        PackageReadinessProbeKey::from_request(self)
    }
}

/// Provider output for one package-readiness request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[must_use]
pub struct PackageReadinessProviderOutput {
    /// Scheduler/admission dependency-readiness facts.
    pub facts: Vec<DependencyReadinessFact>,
    /// Bounded typed diagnostics emitted while resolving facts.
    pub diagnostics: Vec<PackageReadinessProviderDiagnostic>,
}

/// Machine-readable provider diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PackageReadinessProviderDiagnosticCode {
    /// The Python runtime/executable was unavailable.
    PythonUnavailable,
    /// A required package was not installed in the selected environment.
    MissingPackage,
    /// The dependency kind is not supported by this provider.
    UnsupportedDependencyKind,
    /// A package id could not be safely probed.
    InvalidPackageId,
    /// The provider/probe is reserved but not implemented.
    ProbeNotImplemented,
    /// The provider cannot run on this platform.
    UnsupportedPlatform,
    /// The probe exceeded its timeout.
    ProbeTimedOut,
    /// The probe process failed.
    ProbeProcessFailed,
}

/// Bounded typed provider diagnostic with runtime/environment attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReadinessProviderDiagnostic {
    /// Machine-readable diagnostic code.
    pub code: PackageReadinessProviderDiagnosticCode,
    /// Executable backend key used by scheduler candidate matching.
    pub executable_backend_key: BackendId,
    /// Scheduler-facing runtime id for attribution.
    pub scheduler_runtime_id: CapabilityAvailabilityId,
    /// Runtime variant scope when known.
    pub runtime_variant_id: Option<RuntimeVariantId>,
    /// Package environment that was inspected.
    pub environment: PackageReadinessEnvironmentSelector,
    /// Package/dependency id that caused the diagnostic, when scoped.
    pub dependency_id: Option<CapabilityAvailabilityId>,
    /// Bounded diagnostic reason.
    pub reason: CapabilityAvailabilityReason,
}

/// Probe request sent to a host/environment-specific runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReadinessProbeRequest {
    /// Executable backend key used by scheduler candidate matching.
    pub executable_backend_key: BackendId,
    /// Scheduler-facing runtime id for attribution.
    pub scheduler_runtime_id: CapabilityAvailabilityId,
    /// Runtime variant scope when known.
    pub runtime_variant_id: Option<RuntimeVariantId>,
    /// Package environment that should be inspected.
    pub environment: PackageReadinessEnvironmentSelector,
    /// Package ids to probe in this environment.
    pub dependency_ids: Vec<CapabilityAvailabilityId>,
}

/// Probe outcome from a host/environment-specific runner.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum PackageReadinessProbeOutcome {
    /// Probe completed and observed a Python package snapshot.
    Snapshot(PythonPackageReadinessSnapshot),
    /// Probe failed before a reliable package snapshot could be produced.
    Failed(Vec<PackageReadinessProbeFailure>),
}

/// One typed probe failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReadinessProbeFailure {
    /// Failure code.
    pub code: PackageReadinessProviderDiagnosticCode,
    /// Package/dependency id scoped to the failure, when known.
    pub dependency_id: Option<CapabilityAvailabilityId>,
    /// Bounded failure reason.
    pub reason: CapabilityAvailabilityReason,
}

impl PackageReadinessProbeFailure {
    /// Build a typed probe failure.
    #[must_use]
    pub fn new(
        code: PackageReadinessProviderDiagnosticCode,
        dependency_id: Option<CapabilityAvailabilityId>,
        reason: CapabilityAvailabilityReason,
    ) -> Self {
        Self {
            code,
            dependency_id,
            reason,
        }
    }
}

/// Host/environment-specific package probe runner.
#[async_trait]
pub trait PackageReadinessProbeRunner: Send + Sync {
    /// Resolve installed package ids for one deduplicated provider request.
    async fn probe(&self, request: PackageReadinessProbeRequest) -> PackageReadinessProbeOutcome;
}

/// Runtime-scoped package-readiness provider.
#[derive(Debug)]
pub struct PackageReadinessProvider<R> {
    probe_runner: R,
}

impl<R> PackageReadinessProvider<R>
where
    R: PackageReadinessProbeRunner,
{
    /// Build a provider around a concrete probe runner.
    #[must_use]
    pub fn new(probe_runner: R) -> Self {
        Self { probe_runner }
    }

    /// Resolve package-readiness facts for provider requests.
    pub async fn resolve(
        &self,
        requests: &[PackageReadinessProviderRequest],
    ) -> Vec<PackageReadinessProviderOutput> {
        let mut probe_cache =
            BTreeMap::<PackageReadinessProbeKey, PackageReadinessProbeOutcome>::new();
        let mut outputs = Vec::with_capacity(requests.len());

        for request in requests {
            let key = request.probe_key();
            let outcome = if key.dependency_ids.is_empty() {
                PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
                    BTreeSet::new(),
                ))
            } else if let Some(cached) = probe_cache.get(&key) {
                cached.clone()
            } else {
                let probe_request = key.to_probe_request();
                let outcome = self.probe_runner.probe(probe_request).await;
                probe_cache.insert(key, outcome.clone());
                outcome
            };

            outputs.push(resolve_request_from_probe_outcome(request, outcome));
        }

        outputs
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PackageReadinessProbeKey {
    executable_backend_key: BackendId,
    scheduler_runtime_id: CapabilityAvailabilityId,
    runtime_variant_id: Option<RuntimeVariantId>,
    environment: PackageReadinessEnvironmentSelector,
    dependency_ids: Vec<CapabilityAvailabilityId>,
}

impl PackageReadinessProbeKey {
    fn from_request(request: &PackageReadinessProviderRequest) -> Self {
        let dependency_ids = request
            .declarations
            .iter()
            .filter(|declaration| {
                declaration.subject_kind == DependencyReadinessSubjectKind::Package
            })
            .map(|declaration| declaration.dependency_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        Self {
            executable_backend_key: request.executable_backend_key.clone(),
            scheduler_runtime_id: request.scheduler_runtime_id.clone(),
            runtime_variant_id: request.runtime_variant_id.clone(),
            environment: request.environment.clone(),
            dependency_ids,
        }
    }

    fn to_probe_request(&self) -> PackageReadinessProbeRequest {
        PackageReadinessProbeRequest {
            executable_backend_key: self.executable_backend_key.clone(),
            scheduler_runtime_id: self.scheduler_runtime_id.clone(),
            runtime_variant_id: self.runtime_variant_id.clone(),
            environment: self.environment.clone(),
            dependency_ids: self.dependency_ids.clone(),
        }
    }
}

fn resolve_request_from_probe_outcome(
    request: &PackageReadinessProviderRequest,
    outcome: PackageReadinessProbeOutcome,
) -> PackageReadinessProviderOutput {
    match outcome {
        PackageReadinessProbeOutcome::Snapshot(snapshot) => {
            let facts = resolve_python_package_readiness(&request.declarations, &snapshot)
                .unwrap_or_else(|error| {
                    request
                        .declarations
                        .iter()
                        .map(|declaration| {
                            readiness_fact_from_probe_failure(
                                declaration,
                                CapabilityAvailabilityState::MissingDependency,
                                PROVIDER_INVALID_PACKAGE_ID_REASON_CODE,
                                CapabilityAvailabilityReason::parse(error.to_string())
                                    .unwrap_or_else(|_| {
                                        CapabilityAvailabilityReason::parse(
                                            "Package readiness projection failed.",
                                        )
                                        .expect("fallback reason is valid")
                                    }),
                            )
                        })
                        .collect()
                });
            let diagnostics = facts
                .iter()
                .filter_map(|fact| diagnostic_from_fact(request, fact))
                .collect();
            PackageReadinessProviderOutput { facts, diagnostics }
        }
        PackageReadinessProbeOutcome::Failed(failures) => {
            let facts = request
                .declarations
                .iter()
                .map(|declaration| {
                    let scoped_failure = failures.iter().find(|failure| {
                        failure
                            .dependency_id
                            .as_ref()
                            .is_some_and(|id| id == &declaration.dependency_id)
                    });
                    let fallback_failure = failures
                        .iter()
                        .find(|failure| failure.dependency_id.is_none());
                    let failure = scoped_failure.or(fallback_failure);
                    let code = failure
                        .map(|failure| failure.code)
                        .unwrap_or(PackageReadinessProviderDiagnosticCode::ProbeProcessFailed);
                    let reason = failure
                        .map(|failure| failure.reason.clone())
                        .unwrap_or_else(|| {
                            CapabilityAvailabilityReason::parse("Package readiness probe failed.")
                                .expect("fallback reason is valid")
                        });
                    readiness_fact_from_probe_failure(
                        declaration,
                        state_for_probe_failure(code),
                        reason_code_for_probe_failure(code),
                        reason,
                    )
                })
                .collect::<Vec<_>>();
            let mut diagnostics = facts
                .iter()
                .filter_map(|fact| diagnostic_from_fact(request, fact))
                .collect::<Vec<_>>();
            for failure in failures {
                if failure.dependency_id.is_none() {
                    diagnostics.push(PackageReadinessProviderDiagnostic {
                        code: failure.code,
                        executable_backend_key: request.executable_backend_key.clone(),
                        scheduler_runtime_id: request.scheduler_runtime_id.clone(),
                        runtime_variant_id: request.runtime_variant_id.clone(),
                        environment: request.environment.clone(),
                        dependency_id: None,
                        reason: failure.reason,
                    });
                }
            }
            PackageReadinessProviderOutput { facts, diagnostics }
        }
    }
}

fn readiness_fact_from_probe_failure(
    declaration: &DependencyRequirementDeclaration,
    state: CapabilityAvailabilityState,
    reason_code: &str,
    reason: CapabilityAvailabilityReason,
) -> DependencyReadinessFact {
    declaration
        .to_readiness_fact(state, DependencyReadinessResolverOwner::EmbeddedRuntime)
        .with_reason_code(
            CapabilityAvailabilityId::parse(reason_code)
                .expect("provider reason code constants must be valid"),
        )
        .with_reason(reason)
}

fn state_for_probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
) -> CapabilityAvailabilityState {
    match code {
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented => {
            CapabilityAvailabilityState::NotImplemented
        }
        PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            CapabilityAvailabilityState::UnsupportedPlatform
        }
        PackageReadinessProviderDiagnosticCode::InvalidPackageId
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed
        | PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::MissingPackage
        | PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            CapabilityAvailabilityState::MissingDependency
        }
    }
}

fn reason_code_for_probe_failure(code: PackageReadinessProviderDiagnosticCode) -> &'static str {
    match code {
        PackageReadinessProviderDiagnosticCode::PythonUnavailable => {
            PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::InvalidPackageId => {
            PROVIDER_INVALID_PACKAGE_ID_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented => {
            PROVIDER_PROBE_NOT_IMPLEMENTED_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            PROVIDER_UNSUPPORTED_PLATFORM_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::ProbeTimedOut => {
            PROVIDER_PROBE_TIMED_OUT_REASON_CODE
        }
        PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            PROVIDER_PROBE_PROCESS_FAILED_REASON_CODE
        }
    }
}

fn diagnostic_from_fact(
    request: &PackageReadinessProviderRequest,
    fact: &DependencyReadinessFact,
) -> Option<PackageReadinessProviderDiagnostic> {
    if fact.is_ready() {
        return None;
    }

    Some(PackageReadinessProviderDiagnostic {
        code: diagnostic_code_from_fact(fact),
        executable_backend_key: request.executable_backend_key.clone(),
        scheduler_runtime_id: request.scheduler_runtime_id.clone(),
        runtime_variant_id: fact
            .runtime_variant_id
            .clone()
            .or_else(|| request.runtime_variant_id.clone()),
        environment: request.environment.clone(),
        dependency_id: Some(fact.dependency_id.clone()),
        reason: fact.reason.clone().unwrap_or_else(|| {
            CapabilityAvailabilityReason::parse("Package readiness fact is not ready.")
                .expect("fallback reason is valid")
        }),
    })
}

fn diagnostic_code_from_fact(
    fact: &DependencyReadinessFact,
) -> PackageReadinessProviderDiagnosticCode {
    use PackageReadinessProviderDiagnosticCode as Code;

    if let Some(reason_code) = fact.reason_code.as_ref().map(|code| code.as_str()) {
        return match reason_code {
            PYTHON_RUNTIME_UNAVAILABLE_REASON_CODE => Code::PythonUnavailable,
            PYTHON_PACKAGE_NOT_INSTALLED_REASON_CODE => Code::MissingPackage,
            UNSUPPORTED_DEPENDENCY_KIND_REASON_CODE => Code::UnsupportedDependencyKind,
            PROVIDER_PROBE_NOT_IMPLEMENTED_REASON_CODE => Code::ProbeNotImplemented,
            PROVIDER_UNSUPPORTED_PLATFORM_REASON_CODE => Code::UnsupportedPlatform,
            PROVIDER_PROBE_TIMED_OUT_REASON_CODE => Code::ProbeTimedOut,
            PROVIDER_PROBE_PROCESS_FAILED_REASON_CODE => Code::ProbeProcessFailed,
            PROVIDER_INVALID_PACKAGE_ID_REASON_CODE => Code::InvalidPackageId,
            _ => Code::ProbeProcessFailed,
        };
    }

    match fact.state {
        CapabilityAvailabilityState::NotInstalled => Code::MissingPackage,
        CapabilityAvailabilityState::NotImplemented => Code::ProbeNotImplemented,
        CapabilityAvailabilityState::UnsupportedPlatform => Code::UnsupportedPlatform,
        CapabilityAvailabilityState::RequiresRuntimeCapability => Code::UnsupportedDependencyKind,
        _ => Code::ProbeProcessFailed,
    }
}

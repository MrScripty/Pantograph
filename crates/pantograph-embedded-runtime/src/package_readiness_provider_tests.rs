use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use inference::{
    BackendId, CapabilityAvailabilityId, CapabilityAvailabilityReason, CapabilityAvailabilityState,
    DependencyReadinessFact, DependencyRequirementDeclaration, RuntimeVariantId,
};

use crate::dependency_readiness::PythonPackageReadinessSnapshot;
use crate::package_readiness_provider::{
    PackageReadinessEnvironmentSelector, PackageReadinessProbeFailure,
    PackageReadinessProbeOutcome, PackageReadinessProbeRequest, PackageReadinessProbeRunner,
    PackageReadinessProvider, PackageReadinessProviderDiagnosticCode,
    PackageReadinessProviderRequest,
};

#[derive(Debug, Clone)]
struct FakeProbeRunner {
    outcome: PackageReadinessProbeOutcome,
    requests: Arc<Mutex<Vec<PackageReadinessProbeRequest>>>,
}

#[async_trait]
impl PackageReadinessProbeRunner for FakeProbeRunner {
    async fn probe(&self, request: PackageReadinessProbeRequest) -> PackageReadinessProbeOutcome {
        self.requests
            .lock()
            .expect("probe requests lock")
            .push(request);
        self.outcome.clone()
    }
}

fn backend_id(value: &str) -> BackendId {
    BackendId::parse(value).expect("valid backend id")
}

fn availability_id(value: &str) -> CapabilityAvailabilityId {
    CapabilityAvailabilityId::parse(value).expect("valid availability id")
}

fn runtime_variant_id(value: &str) -> RuntimeVariantId {
    RuntimeVariantId::parse(value).expect("valid runtime variant id")
}

fn reason(value: &str) -> CapabilityAvailabilityReason {
    CapabilityAvailabilityReason::parse(value).expect("valid reason")
}

fn installed_package_ids(values: &[&str]) -> BTreeSet<CapabilityAvailabilityId> {
    values.iter().map(|value| availability_id(value)).collect()
}

fn provider_request() -> PackageReadinessProviderRequest {
    PackageReadinessProviderRequest::new(
        backend_id("pytorch"),
        availability_id("pytorch"),
        Some(runtime_variant_id("pytorch.cuda")),
        PackageReadinessEnvironmentSelector::DefaultHostPython,
        inference::pytorch_diffusers_image_generation_package_requirements()
            .into_iter()
            .map(|declaration| {
                declaration.with_runtime_variant_id(runtime_variant_id("pytorch.cuda"))
            })
            .collect(),
    )
}

fn fake_provider(
    outcome: PackageReadinessProbeOutcome,
) -> (
    PackageReadinessProvider<FakeProbeRunner>,
    Arc<Mutex<Vec<PackageReadinessProbeRequest>>>,
) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let runner = FakeProbeRunner {
        outcome,
        requests: requests.clone(),
    };
    (PackageReadinessProvider::new(runner), requests)
}

#[tokio::test]
async fn provider_resolves_available_packages_with_typed_scope() {
    let (provider, requests) = fake_provider(PackageReadinessProbeOutcome::Snapshot(
        PythonPackageReadinessSnapshot::available(installed_package_ids(&[
            "diffusers",
            "transformers",
            "accelerate",
            "torch",
            "pillow",
        ])),
    ));

    let outputs = provider.resolve(&[provider_request()]).await;

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].facts.len(), 5);
    assert!(outputs[0]
        .facts
        .iter()
        .all(DependencyReadinessFact::is_ready));
    assert!(outputs[0].diagnostics.is_empty());

    let captured = requests.lock().expect("probe requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].executable_backend_key.as_str(), "pytorch");
    assert_eq!(captured[0].scheduler_runtime_id.as_str(), "pytorch");
    assert_eq!(
        captured[0]
            .runtime_variant_id
            .as_ref()
            .map(RuntimeVariantId::as_str),
        Some("pytorch.cuda")
    );
    assert_eq!(
        captured[0].environment,
        PackageReadinessEnvironmentSelector::DefaultHostPython
    );
    assert_eq!(
        captured[0]
            .dependency_ids
            .iter()
            .map(CapabilityAvailabilityId::as_str)
            .collect::<Vec<_>>(),
        vec!["accelerate", "diffusers", "pillow", "torch", "transformers"]
    );
}

#[tokio::test]
async fn provider_deduplicates_probe_requests_by_runtime_environment_and_dependencies() {
    let (provider, requests) = fake_provider(PackageReadinessProbeOutcome::Snapshot(
        PythonPackageReadinessSnapshot::available(installed_package_ids(&[
            "diffusers",
            "transformers",
            "accelerate",
            "torch",
            "pillow",
        ])),
    ));

    let request = provider_request();
    let mut reordered = request.clone();
    reordered.declarations.reverse();
    let outputs = provider.resolve(&[request, reordered]).await;

    assert_eq!(outputs.len(), 2);
    assert_eq!(requests.lock().expect("probe requests lock").len(), 1);
}

#[tokio::test]
async fn provider_reports_missing_packages_as_facts_and_diagnostics() {
    let (provider, _requests) = fake_provider(PackageReadinessProbeOutcome::Snapshot(
        PythonPackageReadinessSnapshot::available(installed_package_ids(&[
            "transformers",
            "accelerate",
            "torch",
            "pillow",
        ])),
    ));

    let outputs = provider.resolve(&[provider_request()]).await;
    let diffusers = outputs[0]
        .facts
        .iter()
        .find(|fact| fact.dependency_id.as_str() == "diffusers")
        .expect("diffusers fact");

    assert_eq!(diffusers.state, CapabilityAvailabilityState::NotInstalled);
    assert_eq!(outputs[0].diagnostics.len(), 1);
    assert_eq!(
        outputs[0].diagnostics[0].code,
        PackageReadinessProviderDiagnosticCode::MissingPackage
    );
    assert_eq!(
        outputs[0].diagnostics[0]
            .dependency_id
            .as_ref()
            .map(CapabilityAvailabilityId::as_str),
        Some("diffusers")
    );
}

#[tokio::test]
async fn provider_reports_unavailable_python_without_worker_discovery() {
    let (provider, _requests) = fake_provider(PackageReadinessProbeOutcome::Snapshot(
        PythonPackageReadinessSnapshot::unavailable(reason("Python runtime is not configured.")),
    ));

    let outputs = provider.resolve(&[provider_request()]).await;

    assert_eq!(outputs[0].facts.len(), 5);
    assert!(outputs[0].facts.iter().all(|fact| {
        fact.state == CapabilityAvailabilityState::MissingDependency && !fact.is_ready()
    }));
    assert_eq!(outputs[0].diagnostics.len(), 5);
    assert!(outputs[0].diagnostics.iter().all(|diagnostic| {
        diagnostic.code == PackageReadinessProviderDiagnosticCode::PythonUnavailable
    }));
}

#[tokio::test]
async fn provider_reports_unsupported_dependency_kind_without_probe_fallback() {
    let mut request = provider_request();
    request.declarations = vec![DependencyRequirementDeclaration::dependency(
        backend_id("pytorch"),
        availability_id("pytorch_sidecar"),
        inference::DependencyRequirementNecessity::Required,
    )];
    let (provider, requests) = fake_provider(PackageReadinessProbeOutcome::Snapshot(
        PythonPackageReadinessSnapshot::available(installed_package_ids(&["pytorch_sidecar"])),
    ));

    let outputs = provider.resolve(&[request]).await;

    assert!(requests.lock().expect("probe requests lock").is_empty());
    assert_eq!(outputs[0].facts.len(), 1);
    assert_eq!(
        outputs[0].facts[0].state,
        CapabilityAvailabilityState::RequiresRuntimeCapability
    );
    assert_eq!(
        outputs[0].diagnostics[0].code,
        PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind
    );
}

#[tokio::test]
async fn provider_projects_invalid_package_id_probe_failure() {
    let (provider, _requests) = fake_provider(PackageReadinessProbeOutcome::Failed(vec![
        PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::InvalidPackageId,
            Some(availability_id("diffusers")),
            reason("Package id cannot be probed safely."),
        ),
    ]));

    let outputs = provider.resolve(&[provider_request()]).await;

    assert!(outputs[0].diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PackageReadinessProviderDiagnosticCode::InvalidPackageId
            && diagnostic
                .dependency_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "diffusers")
    }));
}

#[tokio::test]
async fn provider_projects_probe_failure_to_typed_facts_and_diagnostics() {
    let (provider, _requests) = fake_provider(PackageReadinessProbeOutcome::Failed(vec![
        PackageReadinessProbeFailure::new(
            PackageReadinessProviderDiagnosticCode::ProbeTimedOut,
            None,
            reason("Package readiness probe timed out."),
        ),
    ]));

    let outputs = provider.resolve(&[provider_request()]).await;

    assert_eq!(outputs[0].facts.len(), 5);
    assert!(outputs[0].facts.iter().all(|fact| {
        fact.state == CapabilityAvailabilityState::MissingDependency && !fact.is_ready()
    }));
    assert!(outputs[0].diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PackageReadinessProviderDiagnosticCode::ProbeTimedOut
            && diagnostic.dependency_id.is_none()
    }));
}

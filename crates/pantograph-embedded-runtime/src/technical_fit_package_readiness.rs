use crate::package_readiness_provider::{
    PackageReadinessEnvironmentSelector, PackageReadinessProbeRunner, PackageReadinessProvider,
    PackageReadinessProviderRequest,
};

pub(crate) async fn dependency_readiness_facts_for_technical_fit<R>(
    provider: &PackageReadinessProvider<R>,
    request: &pantograph_workflow_service::WorkflowTechnicalFitRequest,
    available_backends: &[inference::BackendInfo],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> Vec<inference::DependencyReadinessFact>
where
    R: PackageReadinessProbeRunner,
{
    let provider_requests =
        package_readiness_provider_requests(request, available_backends, package_facts);
    provider
        .resolve(&provider_requests)
        .await
        .into_iter()
        .flat_map(|output| output.facts)
        .collect()
}

fn package_readiness_provider_requests(
    request: &pantograph_workflow_service::WorkflowTechnicalFitRequest,
    available_backends: &[inference::BackendInfo],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> Vec<PackageReadinessProviderRequest> {
    let graph_runtime_requirement = super::graph_runtime_requirement_from_request(request);
    let mut requests = Vec::new();

    for facts in package_facts {
        let task_id = super::execution_evidence_task_id_from_package_facts(facts);
        let report = inference::normalize_execution_evidence(inference::ExecutionEvidenceRequest {
            task_id,
            package_facts: facts,
            backends: available_backends,
            graph_runtime_requirement: graph_runtime_requirement.as_ref(),
        });
        for candidate in report.candidates {
            let Some(provider_request) = provider_request_for_candidate(&candidate) else {
                continue;
            };
            if !requests.contains(&provider_request) {
                requests.push(provider_request);
            }
        }
    }

    requests
}

fn provider_request_for_candidate(
    candidate: &inference::ExecutionBackendCandidateEvidence,
) -> Option<PackageReadinessProviderRequest> {
    if candidate.backend_key != "pytorch"
        || candidate.task_id != inference::InferenceTaskId::ImageGeneration
    {
        return None;
    }

    Some(PackageReadinessProviderRequest::new(
        inference::BackendId::parse("pytorch").expect("pytorch backend id must be valid"),
        inference::CapabilityAvailabilityId::parse("pytorch")
            .expect("pytorch runtime id must be valid"),
        None,
        PackageReadinessEnvironmentSelector::DefaultHostPython,
        inference::pytorch_diffusers_image_generation_package_requirements(),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    use crate::dependency_readiness::PythonPackageReadinessSnapshot;
    use crate::package_readiness_provider::{
        PackageReadinessProbeOutcome, PackageReadinessProbeRequest,
    };

    use super::*;

    #[derive(Debug, Clone)]
    struct SnapshotRunner {
        snapshot: PythonPackageReadinessSnapshot,
    }

    #[async_trait::async_trait]
    impl PackageReadinessProbeRunner for SnapshotRunner {
        async fn probe(
            &self,
            _request: PackageReadinessProbeRequest,
        ) -> PackageReadinessProbeOutcome {
            PackageReadinessProbeOutcome::Snapshot(self.snapshot.clone())
        }
    }

    #[derive(Debug, Clone, Default)]
    struct RecordingRunner {
        requests: Arc<Mutex<Vec<PackageReadinessProbeRequest>>>,
    }

    #[async_trait::async_trait]
    impl PackageReadinessProbeRunner for RecordingRunner {
        async fn probe(
            &self,
            request: PackageReadinessProbeRequest,
        ) -> PackageReadinessProbeOutcome {
            self.requests
                .lock()
                .expect("package readiness request lock")
                .push(request);
            PackageReadinessProbeOutcome::Snapshot(PythonPackageReadinessSnapshot::available(
                BTreeSet::new(),
            ))
        }
    }

    fn availability_id(value: &str) -> inference::CapabilityAvailabilityId {
        inference::CapabilityAvailabilityId::parse(value).expect("valid availability id")
    }

    fn installed_package_ids(values: &[&str]) -> BTreeSet<inference::CapabilityAvailabilityId> {
        values.iter().map(|value| availability_id(value)).collect()
    }

    fn image_generation_request() -> pantograph_workflow_service::WorkflowTechnicalFitRequest {
        pantograph_workflow_service::build_workflow_technical_fit_request(
            "workflow-a",
            &pantograph_workflow_service::WorkflowRuntimeRequirements {
                resource_estimates: Vec::new(),
                required_models: vec!["image/stable-diffusion/tiny-sd".to_string()],
                required_backends: vec!["pytorch".to_string()],
                required_extensions: Vec::new(),
            },
            None,
            None,
            None,
            None,
        )
    }

    fn diffusers_package_facts() -> inference::ResolvedModelPackageFacts {
        serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
        .expect("decode image generation package facts fixture")
    }

    fn standard_text_package_facts() -> inference::ResolvedModelPackageFacts {
        serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
        ))
        .expect("decode standard text-generation package facts fixture")
    }

    fn pytorch_backend() -> inference::BackendInfo {
        inference::BackendInfo {
            name: "pytorch".to_string(),
            backend_key: "pytorch".to_string(),
            description: "test pytorch backend".to_string(),
            capabilities: inference::BackendCapabilities {
                facts: inference::BackendCapabilityFacts {
                    tasks: vec![inference::BackendTaskCapability::stable(
                        inference::InferenceTaskId::ImageGeneration,
                        vec![inference::InferenceModality::Text],
                        vec![inference::InferenceModality::Image],
                    )],
                    preprocessing: inference::BackendComponentCapability::RequiresPackageComponent,
                    postprocessing: inference::BackendComponentCapability::BackendManaged,
                    model_sources: inference::BackendModelSourceCapabilityFacts {
                        artifact_kinds: vec![inference::ModelArtifactKind::DiffusersBundle],
                        backend_hints: vec![inference::BackendHintLabel::Diffusers],
                        custom_code: inference::BackendFeatureSupport::Unsupported,
                    },
                    features: inference::BackendFeatureCapabilityFacts {
                        streaming: inference::BackendFeatureSupport::Unsupported,
                        device_selection: inference::BackendFeatureSupport::Supported,
                        external_connection: inference::BackendFeatureSupport::Unsupported,
                        kv_cache: inference::BackendFeatureSupport::Unsupported,
                    },
                    runtime_variants: vec![inference::RuntimeVariantCapability {
                        runtime_variant_id: inference::RuntimeVariantId::parse("pytorch.cuda")
                            .expect("valid runtime variant id"),
                        device_class: inference::InferenceDeviceClass::Cuda,
                        available: true,
                        diagnostics: Vec::new(),
                    }],
                },
                ..inference::BackendCapabilities::default()
            },
            default_start_mode: inference::backend::BackendDefaultStartMode::Inference,
            active: false,
            available: true,
            unavailable_reason: None,
            can_install: false,
            runtime_binary_id: None,
        }
    }

    fn llama_cpp_backend() -> inference::BackendInfo {
        let mut backend = pytorch_backend();
        backend.name = "llama_cpp".to_string();
        backend.backend_key = "llama_cpp".to_string();
        backend.capabilities.facts.tasks = vec![inference::BackendTaskCapability::stable(
            inference::InferenceTaskId::TextGeneration,
            vec![inference::InferenceModality::Text],
            vec![inference::InferenceModality::Text],
        )];
        backend.capabilities.facts.model_sources.artifact_kinds =
            vec![inference::ModelArtifactKind::Gguf];
        backend.capabilities.facts.model_sources.backend_hints =
            vec![inference::BackendHintLabel::LlamaCpp];
        backend.capabilities.facts.runtime_variants[0].runtime_variant_id =
            inference::RuntimeVariantId::parse("llama_cpp.cuda")
                .expect("llama_cpp runtime variant id must be valid");
        backend
    }

    fn standard_text_generation_request() -> pantograph_workflow_service::WorkflowTechnicalFitRequest
    {
        pantograph_workflow_service::build_workflow_technical_fit_request(
            "workflow-standard-text",
            &pantograph_workflow_service::WorkflowRuntimeRequirements {
                resource_estimates: Vec::new(),
                required_models: vec!["llm/llama/tiny-gguf".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            None,
            None,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn collects_pytorch_diffusers_readiness_from_provider() {
        let provider = PackageReadinessProvider::new(SnapshotRunner {
            snapshot: PythonPackageReadinessSnapshot::available(installed_package_ids(&[
                "diffusers",
                "transformers",
                "accelerate",
                "torch",
                "pillow",
            ])),
        });

        let facts = dependency_readiness_facts_for_technical_fit(
            &provider,
            &image_generation_request(),
            &[pytorch_backend()],
            &[diffusers_package_facts()],
        )
        .await;

        assert_eq!(facts.len(), 5);
        assert!(facts
            .iter()
            .all(inference::DependencyReadinessFact::is_ready));
        assert!(facts
            .iter()
            .any(|fact| fact.dependency_id.as_str() == "diffusers"));
    }

    #[tokio::test]
    async fn standard_text_model_without_pumas_profile_does_not_probe_package_readiness() {
        let package_facts = standard_text_package_facts();
        let backend = llama_cpp_backend();
        let evidence =
            inference::normalize_execution_evidence(inference::ExecutionEvidenceRequest {
                task_id: inference::InferenceTaskId::TextGeneration,
                package_facts: &package_facts,
                backends: std::slice::from_ref(&backend),
                graph_runtime_requirement: None,
            });
        assert!(evidence
            .candidates
            .iter()
            .any(|candidate| candidate.backend_key == "llama_cpp"));

        let runner = RecordingRunner::default();
        let requests = runner.requests.clone();
        let provider = PackageReadinessProvider::new(runner);
        let facts = dependency_readiness_facts_for_technical_fit(
            &provider,
            &standard_text_generation_request(),
            &[backend],
            &[package_facts],
        )
        .await;

        assert!(facts.is_empty());
        assert!(requests
            .lock()
            .expect("package readiness request lock")
            .is_empty());
    }
}

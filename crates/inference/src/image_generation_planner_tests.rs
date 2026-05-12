use super::*;
use crate::device_contracts::{InferenceDevicePolicy, RuntimeVariantId};

fn package_fixture(name: &str) -> ResolvedModelPackageFacts {
    let raw = match name {
        "diffusers_sd_text_to_image_package_facts.json" => include_str!(
            "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ),
        "gguf_text_generation_package_facts.json" => include_str!(
            "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
        ),
        other => panic!("unknown package fixture: {other}"),
    };
    serde_json::from_str(raw).expect("fixture should decode")
}

fn image_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        model: "image/stable-diffusion/tiny-sd".to_string(),
        prompt: "a compact test image".to_string(),
        negative_prompt: Some("blur".to_string()),
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(8),
        guidance_scale: Some(7.5),
        seed: Some(42),
        scheduler: Some("euler".to_string()),
        num_images_per_prompt: Some(2),
        init_image: None,
        mask_image: None,
        strength: None,
        extra_options: serde_json::Value::Null,
    }
}

fn backend_decision(backend_id: &str) -> BackendExecutionDecision {
    let backend_id = BackendId::parse(backend_id).expect("valid backend id");
    let runtime_variant_id =
        RuntimeVariantId::parse("pytorch.diffusers").expect("valid runtime variant");
    let device_decision = DeviceResolutionDecision {
        policy: InferenceDevicePolicy::Auto,
        runtime_variant_id: runtime_variant_id.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(InferenceDeviceId::parse("cpu").expect("valid device id")),
        diagnostics: Vec::new(),
    };
    BackendExecutionDecision {
        selected_backend_id: backend_id,
        selected_runtime_variant_id: runtime_variant_id,
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(InferenceDeviceId::parse("cpu").expect("valid device id")),
        device_decision,
        selected_task_id: Some(InferenceTaskId::ImageGeneration),
        selected_model_ref: None,
        diagnostics: Vec::new(),
    }
}

fn diagnostic_codes(
    outcome: &ImageGenerationPlanningOutcome,
) -> Vec<ImageGenerationPlannerDiagnosticCode> {
    match outcome {
        ImageGenerationPlanningOutcome::Planned { .. } => Vec::new(),
        ImageGenerationPlanningOutcome::Rejected { diagnostics } => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect(),
    }
}

#[test]
fn planner_accepts_pumas_diffusers_stable_diffusion_facts() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = image_request();
    let decision = backend_decision("pytorch");
    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    let ImageGenerationPlanningOutcome::Planned { plan } = outcome else {
        panic!("expected valid image-generation plan");
    };

    assert_eq!(plan.model_ref.model_id, "image/stable-diffusion/tiny-sd");
    assert_eq!(plan.backend_id.as_str(), "pytorch");
    assert_eq!(plan.runtime_variant_id.as_str(), "pytorch.diffusers");
    assert_eq!(plan.family, ImageGenerationFamilyLabel::StableDiffusion);
    assert_eq!(plan.pipeline_class, "StableDiffusionPipeline");
    assert_eq!(plan.estimated_output_rgba_bytes, Some(2_097_152));
    assert_eq!(
        plan.required_components,
        STABLE_DIFFUSION_REQUIRED_COMPONENTS.to_vec()
    );
}

#[test]
fn planner_rejects_missing_diffusers_facts_without_backend_fallback() {
    let facts = package_fixture("gguf_text_generation_package_facts.json");
    let request = image_request();
    let decision = backend_decision("pytorch");
    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    let codes = diagnostic_codes(&outcome);
    assert!(codes.contains(&ImageGenerationPlannerDiagnosticCode::MissingDiffusersEvidence));
    assert!(codes.contains(&ImageGenerationPlannerDiagnosticCode::UnsupportedTaskEvidence));
}

#[test]
fn planner_rejects_ambiguous_family_evidence() {
    let mut facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    facts
        .diffusers
        .as_mut()
        .expect("diffusers facts")
        .family_evidence
        .push(crate::model_contracts::ImageGenerationFamilyEvidence {
            family: ImageGenerationFamilyLabel::Flux,
            source: crate::model_contracts::ImageGenerationFamilyEvidenceSource::PipelineClass,
            value_source: crate::model_contracts::PackageFactValueSource::Config,
            source_path: Some("model_index.json".to_string()),
            message: None,
        });

    let request = image_request();
    let decision = backend_decision("pytorch");
    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    assert!(diagnostic_codes(&outcome)
        .contains(&ImageGenerationPlannerDiagnosticCode::AmbiguousFamilyEvidence));
}

#[test]
fn planner_rejects_non_pytorch_backend_decision_without_diffusers_alias() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = image_request();
    let decision = backend_decision("diffusers");
    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    assert!(diagnostic_codes(&outcome)
        .contains(&ImageGenerationPlannerDiagnosticCode::UnsupportedBackend));
}

#[test]
fn planner_rejects_invalid_dimensions_before_resource_estimate() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = ImageGenerationRequest {
        width: Some(0),
        ..image_request()
    };
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    assert!(diagnostic_codes(&outcome)
        .contains(&ImageGenerationPlannerDiagnosticCode::InvalidNumericOption));
}

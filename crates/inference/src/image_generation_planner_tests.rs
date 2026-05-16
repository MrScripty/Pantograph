use super::*;
use crate::device_contracts::{InferenceDevicePolicy, RuntimeVariantId};
use crate::types::EncodedImage;

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
        denoising_scheduler: Some("euler".to_string()),
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
        selection_policy_trace: None,
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

fn rejected_diagnostics(
    outcome: &ImageGenerationPlanningOutcome,
) -> &[ImageGenerationPlannerDiagnostic] {
    match outcome {
        ImageGenerationPlanningOutcome::Planned { .. } => &[],
        ImageGenerationPlanningOutcome::Rejected { diagnostics } => diagnostics,
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
    assert_eq!(
        plan.denoising_scheduler
            .as_ref()
            .expect("scheduler should parse")
            .as_str(),
        "euler"
    );
    assert_eq!(plan.estimated_output_rgba_bytes, Some(2_097_152));
    assert_eq!(
        plan.required_components,
        STABLE_DIFFUSION_REQUIRED_COMPONENTS.to_vec()
    );
    let plan_json = serde_json::to_value(&plan).expect("plan should serialize");
    assert_eq!(plan_json["denoising_scheduler"], "euler");
    assert!(plan_json.get("scheduler").is_none());
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
fn planner_rejects_unsupported_single_family_without_generic_diffusers_fallback() {
    let mut facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let diffusers = facts.diffusers.as_mut().expect("diffusers facts");
    diffusers.family_evidence = vec![crate::model_contracts::ImageGenerationFamilyEvidence {
        family: ImageGenerationFamilyLabel::Flux,
        source: crate::model_contracts::ImageGenerationFamilyEvidenceSource::PipelineClass,
        value_source: crate::model_contracts::PackageFactValueSource::Config,
        source_path: Some("model_index.json".to_string()),
        message: None,
    }];
    let request = image_request();
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    assert!(diagnostic_codes(&outcome)
        .contains(&ImageGenerationPlannerDiagnosticCode::UnsupportedFamily));
}

#[test]
fn planner_reports_exact_missing_component_role_path() {
    let mut facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    facts
        .diffusers
        .as_mut()
        .expect("diffusers facts")
        .components
        .retain(|component| component.role != DiffusersComponentRole::Vae);
    let request = image_request();
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    let diagnostics = rejected_diagnostics(&outcome);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ImageGenerationPlannerDiagnosticCode::MissingComponentRole
            && diagnostic.field_path == "package_facts.diffusers.components.vae"
    }));
}

#[test]
fn planner_rejects_invalid_denoising_scheduler_option_id() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = ImageGenerationRequest {
        denoising_scheduler: Some("EulerDiscreteScheduler".to_string()),
        ..image_request()
    };
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    let diagnostics = rejected_diagnostics(&outcome);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ImageGenerationPlannerDiagnosticCode::InvalidDenoisingSchedulerOptionId
            && diagnostic.field_path == "request.denoising_scheduler"
    }));
}

#[test]
fn denoising_scheduler_option_id_round_trips_as_primitive_string() {
    let option_id =
        DenoisingSchedulerOptionId::parse("flow_match_euler").expect("valid scheduler option id");

    let encoded = serde_json::to_string(&option_id).expect("scheduler id should encode");
    let decoded: DenoisingSchedulerOptionId =
        serde_json::from_str(&encoded).expect("scheduler id should decode");

    assert_eq!(encoded, "\"flow_match_euler\"");
    assert_eq!(decoded.as_str(), "flow_match_euler");
    assert!(DenoisingSchedulerOptionId::parse("flow_match_euler.").is_err());
    assert!(DenoisingSchedulerOptionId::parse("2m").is_err());
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

#[test]
fn planner_rejects_non_finite_guidance_scale() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = ImageGenerationRequest {
        guidance_scale: Some(f32::INFINITY),
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

#[test]
fn planner_rejects_unsupported_image_options_without_silent_ignore() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = ImageGenerationRequest {
        init_image: Some(EncodedImage {
            data_base64: "aW1hZ2U=".to_string(),
            mime_type: "image/png".to_string(),
            width: Some(32),
            height: Some(32),
        }),
        mask_image: Some(EncodedImage {
            data_base64: "bWFzaw==".to_string(),
            mime_type: "image/png".to_string(),
            width: Some(32),
            height: Some(32),
        }),
        strength: Some(0.5),
        extra_options: serde_json::json!({
            "adapter:opaque_option": true,
        }),
        ..image_request()
    };
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    let diagnostics = rejected_diagnostics(&outcome);
    for field_path in [
        "request.init_image",
        "request.mask_image",
        "request.strength",
        "request.extra_options",
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ImageGenerationPlannerDiagnosticCode::UnsupportedOption
                && diagnostic.field_path == field_path
        }));
    }
}

#[test]
fn planner_rejects_resource_estimate_overflow_without_allocation() {
    let facts = package_fixture("diffusers_sd_text_to_image_package_facts.json");
    let request = ImageGenerationRequest {
        width: Some(u32::MAX),
        height: Some(u32::MAX),
        num_images_per_prompt: Some(2),
        ..image_request()
    };
    let decision = backend_decision("pytorch");

    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        backend_decision: &decision,
    });

    assert!(diagnostic_codes(&outcome)
        .contains(&ImageGenerationPlannerDiagnosticCode::ResourceEstimateOverflow));
}

use super::{generate_image_envelope_from_plan, image_generation_result_from_worker_response};
use crate::device_contracts::{
    BackendId, DeviceResolutionDecision, InferenceDeviceClass, InferenceDeviceId,
    InferenceDevicePolicy, RuntimeVariantId,
};
use crate::image_generation_planner::{DenoisingSchedulerOptionId, ImageGenerationExecutionPlan};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, PumasArtifactEntryPath, PumasModelRef,
};

#[test]
fn test_generate_image_envelope_from_plan_validates_worker_request() {
    let plan = image_plan();

    let envelope =
        generate_image_envelope_from_plan("req-image-plan", &plan).expect("envelope should build");

    assert_eq!(envelope.request_id, "req-image-plan");
    assert_eq!(envelope.payload.model_ref.model_id, "image/example/tiny-sd");
    assert_eq!(
        envelope.payload.artifact_entry_path,
        PumasArtifactEntryPath::parse("image/example/tiny-sd").expect("valid artifact path")
    );
    assert_eq!(envelope.payload.pipeline_class, "StableDiffusionPipeline");
    assert_eq!(
        envelope
            .payload
            .device
            .as_ref()
            .map(|device| device.as_str()),
        Some("cpu")
    );
    assert_eq!(envelope.payload.prompt, "a compact test image");
}

#[test]
fn test_image_generation_result_from_worker_response_maps_images() {
    let response =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_response.json");

    let result = image_generation_result_from_worker_response("req-image-001", response)
        .expect("response should map");

    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].mime_type, "image/png");
    assert_eq!(result.images[0].data_base64, "iVBORw0KGgo=");
    assert_eq!(result.seed_used, Some(42));
    assert!(result.metadata["denoising_scheduler"].is_null());
}

#[test]
fn test_image_generation_result_rejects_request_id_mismatch() {
    let response =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_response.json");

    let error = image_generation_result_from_worker_response("req-other", response)
        .expect_err("mismatched response ids should fail");

    assert!(error
        .to_string()
        .contains("generate_image response request_id mismatch"));
}

fn image_plan() -> ImageGenerationExecutionPlan {
    let runtime_variant_id =
        RuntimeVariantId::parse("pytorch.diffusers").expect("valid runtime variant");
    let selected_device_id = InferenceDeviceId::parse("cpu").expect("valid device id");
    let device_decision = DeviceResolutionDecision {
        policy: InferenceDevicePolicy::Auto,
        runtime_variant_id: runtime_variant_id.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(selected_device_id.clone()),
        diagnostics: Vec::new(),
    };
    ImageGenerationExecutionPlan {
        model_ref: PumasModelRef {
            model_id: "image/example/tiny-sd".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
        artifact_entry_path: PumasArtifactEntryPath::parse("image/example/tiny-sd")
            .expect("valid artifact path"),
        backend_id: BackendId::parse("pytorch").expect("valid backend id"),
        runtime_variant_id,
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(selected_device_id),
        device_decision,
        family: ImageGenerationFamilyLabel::StableDiffusion,
        pipeline_class: "StableDiffusionPipeline".to_string(),
        required_components: vec![
            DiffusersComponentRole::PipelineIndex,
            DiffusersComponentRole::Scheduler,
            DiffusersComponentRole::Tokenizer,
            DiffusersComponentRole::TextEncoder,
            DiffusersComponentRole::Unet,
            DiffusersComponentRole::Vae,
        ],
        prompt: "a compact test image".to_string(),
        negative_prompt: Some("blur".to_string()),
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(8),
        guidance_scale: Some(7.5),
        seed: Some(42),
        denoising_scheduler: Some(
            DenoisingSchedulerOptionId::parse("euler").expect("valid scheduler id"),
        ),
        num_images_per_prompt: Some(1),
        estimated_output_rgba_bytes: Some(1_048_576),
    }
}

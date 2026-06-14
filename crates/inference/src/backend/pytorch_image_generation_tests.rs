use super::{
    generate_image_batch_envelope_from_execution_request, generate_image_envelope_from_plan,
    image_generation_batch_response_from_worker_response,
    image_generation_result_from_worker_response, reject_incompatible_pytorch_batch,
};
use crate::device_contracts::{
    BackendId, DeviceResolutionDecision, InferenceDeviceClass, InferenceDeviceId,
    InferenceDevicePolicy, RuntimeVariantId,
};
use crate::image_generation_batch::{
    ImageGenerationBatchDiagnosticCode, ImageGenerationBatchExecutionMemberRequest,
    ImageGenerationBatchExecutionRequest, ImageGenerationBatchExecutionState,
    ImageGenerationBatchMemberExecutionState,
};
use crate::image_generation_planner::{DenoisingSchedulerOptionId, ImageGenerationExecutionPlan};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef, MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
};
use crate::resource_estimates::{InferenceResourceEstimate, InferenceResourceEstimateKind};
use crate::{ImageGenerationRequest, InferenceExecutionTelemetryScope};

#[test]
fn test_generate_image_envelope_from_plan_validates_worker_request() {
    let plan = image_plan();

    let envelope =
        generate_image_envelope_from_plan("req-image-plan", &plan).expect("envelope should build");

    assert_eq!(envelope.request_id, "req-image-plan");
    assert_eq!(envelope.payload.model_ref.model_id, "image/example/tiny-sd");
    assert_eq!(
        envelope.payload.artifact_load_target.local_load_path,
        "/pumas/models/image/example/tiny-sd"
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
fn test_generate_image_batch_envelope_from_execution_request_validates_worker_request() {
    let request = image_batch_request();

    let envelope =
        generate_image_batch_envelope_from_execution_request("req-image-batch-plan", &request)
            .expect("batch envelope should build");

    assert_eq!(envelope.request_id, "req-image-batch-plan");
    assert_eq!(envelope.payload.batch_execution_id, "batch-001");
    assert_eq!(envelope.payload.anchor_member_id, "member-001");
    assert_eq!(envelope.payload.members.len(), 2);
    assert_eq!(envelope.payload.members[0].member_id, "member-001");
    assert_eq!(
        envelope.payload.members[0].request.prompt,
        "a compact test image"
    );
    assert_eq!(envelope.payload.members[1].member_id, "member-002");
    assert_eq!(
        envelope.payload.members[1].request.prompt,
        "a second compact test image"
    );
}

#[test]
fn test_image_generation_result_from_worker_response_maps_images() {
    let response =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_response.json");

    let telemetry_scope = InferenceExecutionTelemetryScope::new();
    let result = image_generation_result_from_worker_response(
        "req-image-001",
        response,
        &telemetry_scope.recorder(),
    )
    .expect("response should map");

    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].mime_type, "image/png");
    assert_eq!(result.images[0].data_base64, "iVBORw0KGgo=");
    assert_eq!(result.seed_used, Some(42));
    assert!(result.metadata["denoising_scheduler"].is_null());
}

#[test]
fn test_image_generation_batch_response_from_worker_response_maps_members() {
    let response = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_response.json"
    );

    let telemetry_scope = InferenceExecutionTelemetryScope::new();
    let result = image_generation_batch_response_from_worker_response(
        "req-image-batch-001",
        "image-batch-001",
        response,
        &telemetry_scope.recorder(),
    )
    .expect("batch response should map");

    assert_eq!(result.batch_execution_id, "image-batch-001");
    assert_eq!(
        result.state,
        ImageGenerationBatchExecutionState::PartiallyCompleted
    );
    assert_eq!(result.members.len(), 2);
    assert_eq!(
        result.members[0].state,
        ImageGenerationBatchMemberExecutionState::Completed
    );
    assert_eq!(
        result.members[0]
            .result
            .as_ref()
            .expect("first member result")
            .images[0]
            .mime_type,
        "image/png"
    );
    assert_eq!(
        result.members[1].state,
        ImageGenerationBatchMemberExecutionState::Failed
    );
    assert_eq!(
        result.members[1].diagnostics[0].code,
        ImageGenerationBatchDiagnosticCode::MemberExecutionFailed
    );
}

#[test]
fn test_pytorch_batch_compatibility_rejects_mismatched_dimensions() {
    let mut request = image_batch_request();
    request.members[1].plan.width = Some(768);

    let response = reject_incompatible_pytorch_batch(&request)
        .expect("mismatched dimensions should be rejected before worker dispatch");

    assert_eq!(response.state, ImageGenerationBatchExecutionState::Rejected);
    assert_eq!(response.members.len(), 2);
    assert_eq!(
        response.members[0].state,
        ImageGenerationBatchMemberExecutionState::Rejected
    );
    assert_eq!(
        response.diagnostics[0].code,
        ImageGenerationBatchDiagnosticCode::BatchExecutionRejected
    );
    assert!(response.diagnostics[0]
        .message
        .contains("same dimensions and generation settings"));
}

#[test]
fn test_image_generation_result_rejects_request_id_mismatch() {
    let response =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_response.json");

    let telemetry_scope = InferenceExecutionTelemetryScope::new();
    let error = image_generation_result_from_worker_response(
        "req-other",
        response,
        &telemetry_scope.recorder(),
    )
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
        artifact_load_target: PumasArtifactLoadTarget {
            model_ref: PumasModelRef {
                model_id: "image/example/tiny-sd".to_string(),
                revision: None,
                selected_artifact_id: None,
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            artifact_kind: ModelArtifactKind::DiffusersBundle,
            local_load_path: "/pumas/models/image/example/tiny-sd".to_string(),
            load_path_kind: PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("test-root".to_string()),
            storage_kind: ModelStorageKind::LibraryOwned,
            validation_state: ModelValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(MODEL_PACKAGE_FACTS_CONTRACT_VERSION),
        },
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
        resource_estimates: vec![InferenceResourceEstimate::available(
            InferenceResourceEstimateKind::OutputRgbaBytes,
            1_048_576,
        )],
    }
}

fn image_batch_request() -> ImageGenerationBatchExecutionRequest {
    let mut first_plan = image_plan();
    first_plan.denoising_scheduler = None;
    first_plan.prompt = "a compact test image".to_string();
    first_plan.seed = Some(42);
    let mut second_plan = first_plan.clone();
    second_plan.prompt = "a second compact test image".to_string();
    second_plan.seed = Some(43);

    ImageGenerationBatchExecutionRequest {
        batch_execution_id: "batch-001".to_string(),
        anchor_member_id: "member-001".to_string(),
        members: vec![
            ImageGenerationBatchExecutionMemberRequest {
                member_id: "member-001".to_string(),
                request: image_request("a compact test image", Some(42)),
                plan: first_plan,
            },
            ImageGenerationBatchExecutionMemberRequest {
                member_id: "member-002".to_string(),
                request: image_request("a second compact test image", Some(43)),
                plan: second_plan,
            },
        ],
    }
}

fn image_request(prompt: &str, seed: Option<u64>) -> ImageGenerationRequest {
    ImageGenerationRequest {
        model: "image/example/tiny-sd".to_string(),
        prompt: prompt.to_string(),
        negative_prompt: Some("blur".to_string()),
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(8),
        guidance_scale: Some(7.5),
        seed,
        denoising_scheduler: None,
        num_images_per_prompt: Some(1),
        init_image: None,
        mask_image: None,
        strength: None,
        extra_options: serde_json::Value::Null,
    }
}

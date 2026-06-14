use serde_json::Value;

use super::{
    ImageGenerationBatchContractError, ImageGenerationBatchDiagnostic,
    ImageGenerationBatchDiagnosticCode, ImageGenerationBatchDiagnosticSeverity,
    ImageGenerationBatchExecutionMemberRequest, ImageGenerationBatchExecutionMemberResponse,
    ImageGenerationBatchExecutionRequest, ImageGenerationBatchExecutionResponse,
    ImageGenerationBatchExecutionState, ImageGenerationBatchMemberExecutionState,
};
use crate::device_contracts::{
    BackendId, DeviceResolutionDecision, InferenceDeviceClass, InferenceDeviceId,
    InferenceDevicePolicy, RuntimeVariantId,
};
use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef, MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
};
use crate::resource_estimates::{InferenceResourceEstimate, InferenceResourceEstimateKind};
use crate::types::{EncodedImage, ImageGenerationRequest, ImageGenerationResult};

#[test]
fn validates_planned_batch_members_with_stable_anchor() {
    let request = ImageGenerationBatchExecutionRequest {
        batch_execution_id: "batch-001".to_string(),
        anchor_member_id: "member-001".to_string(),
        members: vec![batch_member("member-001", "paper lantern")],
    };

    assert_eq!(request.validate(), Ok(()));
}

#[test]
fn rejects_duplicate_members_before_backend_execution() {
    let request = ImageGenerationBatchExecutionRequest {
        batch_execution_id: "batch-001".to_string(),
        anchor_member_id: "member-001".to_string(),
        members: vec![
            batch_member("member-001", "paper lantern"),
            batch_member("member-001", "paper lantern"),
        ],
    };

    assert!(matches!(
        request.validate(),
        Err(ImageGenerationBatchContractError::DuplicateMemberId { member_id })
            if member_id == "member-001"
    ));
}

#[test]
fn rejects_unknown_anchor_member() {
    let request = ImageGenerationBatchExecutionRequest {
        batch_execution_id: "batch-001".to_string(),
        anchor_member_id: "missing-member".to_string(),
        members: vec![batch_member("member-001", "paper lantern")],
    };

    assert!(matches!(
        request.validate(),
        Err(ImageGenerationBatchContractError::UnknownAnchorMemberId { anchor_member_id })
            if anchor_member_id == "missing-member"
    ));
}

#[test]
fn rejects_request_plan_correlation_drift() {
    let mut member = batch_member("member-001", "paper lantern");
    member.plan.prompt = "different prompt".to_string();

    assert!(matches!(
        member.validate(),
        Err(ImageGenerationBatchContractError::RequestPlanMismatch {
            member_id,
            field_path: "prompt",
        }) if member_id == "member-001"
    ));
}

#[test]
fn validates_completed_response_members_with_results() {
    let response = ImageGenerationBatchExecutionResponse {
        batch_execution_id: "batch-001".to_string(),
        state: ImageGenerationBatchExecutionState::Completed,
        members: vec![completed_member("member-001")],
        diagnostics: Vec::new(),
    };

    assert_eq!(response.validate(), Ok(()));
}

#[test]
fn rejects_completed_batch_with_failed_member() {
    let response = ImageGenerationBatchExecutionResponse {
        batch_execution_id: "batch-001".to_string(),
        state: ImageGenerationBatchExecutionState::Completed,
        members: vec![failed_member("member-001")],
        diagnostics: Vec::new(),
    };

    assert!(matches!(
        response.validate(),
        Err(ImageGenerationBatchContractError::CompletedBatchHasNonCompletedMembers)
    ));
}

#[test]
fn validates_terminal_partial_completion() {
    let response = ImageGenerationBatchExecutionResponse {
        batch_execution_id: "batch-001".to_string(),
        state: ImageGenerationBatchExecutionState::PartiallyCompleted,
        members: vec![completed_member("member-001"), failed_member("member-002")],
        diagnostics: Vec::new(),
    };

    assert_eq!(response.validate(), Ok(()));
}

#[test]
fn rejects_terminal_member_without_diagnostics() {
    let response = ImageGenerationBatchExecutionResponse {
        batch_execution_id: "batch-001".to_string(),
        state: ImageGenerationBatchExecutionState::Failed,
        members: vec![ImageGenerationBatchExecutionMemberResponse {
            member_id: "member-001".to_string(),
            state: ImageGenerationBatchMemberExecutionState::Failed,
            result: None,
            diagnostics: Vec::new(),
        }],
        diagnostics: Vec::new(),
    };

    assert!(matches!(
        response.validate(),
        Err(ImageGenerationBatchContractError::TerminalMemberMissingDiagnostics {
            member_id,
            state: ImageGenerationBatchMemberExecutionState::Failed,
        }) if member_id == "member-001"
    ));
}

fn batch_member(member_id: &str, prompt: &str) -> ImageGenerationBatchExecutionMemberRequest {
    ImageGenerationBatchExecutionMemberRequest {
        member_id: member_id.to_string(),
        request: image_request(prompt),
        plan: image_plan(prompt),
    }
}

fn image_request(prompt: &str) -> ImageGenerationRequest {
    ImageGenerationRequest {
        model: "mock-image-model".to_string(),
        prompt: prompt.to_string(),
        negative_prompt: None,
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(20),
        guidance_scale: Some(4.0),
        seed: Some(7),
        denoising_scheduler: None,
        num_images_per_prompt: Some(1),
        init_image: None,
        mask_image: None,
        strength: None,
        extra_options: Value::Null,
    }
}

fn image_plan(prompt: &str) -> ImageGenerationExecutionPlan {
    let runtime_variant_id =
        RuntimeVariantId::parse("pytorch.diffusers").expect("valid runtime variant id");
    let selected_device_id = InferenceDeviceId::parse("cuda:0").expect("valid device id");

    ImageGenerationExecutionPlan {
        model_ref: model_ref(),
        artifact_entry_path: PumasArtifactEntryPath::parse("image/mock-image-model")
            .expect("valid artifact path"),
        artifact_load_target: PumasArtifactLoadTarget {
            model_ref: model_ref(),
            artifact_kind: ModelArtifactKind::DiffusersBundle,
            local_load_path: "/pumas/models/image/mock-image-model".to_string(),
            load_path_kind: PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("test-root".to_string()),
            storage_kind: ModelStorageKind::LibraryOwned,
            validation_state: ModelValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(MODEL_PACKAGE_FACTS_CONTRACT_VERSION),
        },
        backend_id: BackendId::parse("pytorch").expect("valid backend id"),
        runtime_variant_id: runtime_variant_id.clone(),
        selected_device_class: InferenceDeviceClass::Cuda,
        selected_device_id: Some(selected_device_id.clone()),
        device_decision: DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Explicit {
                device_class: InferenceDeviceClass::Cuda,
                device_id: Some(selected_device_id.clone()),
            },
            runtime_variant_id,
            selected_device_class: InferenceDeviceClass::Cuda,
            selected_device_id: Some(selected_device_id),
            diagnostics: Vec::new(),
        },
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
        prompt: prompt.to_string(),
        negative_prompt: None,
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(20),
        guidance_scale: Some(4.0),
        seed: Some(7),
        denoising_scheduler: None,
        num_images_per_prompt: Some(1),
        resource_estimates: vec![InferenceResourceEstimate::available(
            InferenceResourceEstimateKind::OutputRgbaBytes,
            512_u64 * 512 * 4,
        )],
    }
}

fn model_ref() -> PumasModelRef {
    PumasModelRef {
        model_id: "mock-image-model".to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some("diffusers".to_string()),
        selected_artifact_path: Some("image/mock-image-model".to_string()),
        migration_diagnostics: Vec::new(),
    }
}

fn completed_member(member_id: &str) -> ImageGenerationBatchExecutionMemberResponse {
    ImageGenerationBatchExecutionMemberResponse {
        member_id: member_id.to_string(),
        state: ImageGenerationBatchMemberExecutionState::Completed,
        result: Some(ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: "aW1hZ2U=".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(7),
            metadata: Value::Null,
        }),
        diagnostics: Vec::new(),
    }
}

fn failed_member(member_id: &str) -> ImageGenerationBatchExecutionMemberResponse {
    ImageGenerationBatchExecutionMemberResponse {
        member_id: member_id.to_string(),
        state: ImageGenerationBatchMemberExecutionState::Failed,
        result: None,
        diagnostics: vec![ImageGenerationBatchDiagnostic {
            code: ImageGenerationBatchDiagnosticCode::MemberExecutionFailed,
            severity: ImageGenerationBatchDiagnosticSeverity::Error,
            member_id: Some(member_id.to_string()),
            field_path: "members".to_string(),
            message: "member failed".to_string(),
        }],
    }
}

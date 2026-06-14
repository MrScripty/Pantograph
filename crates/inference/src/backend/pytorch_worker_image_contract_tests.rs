use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::pytorch_worker_image_contract::{
    validate_generate_image_batch_envelope, validate_generate_image_envelope,
    PyTorchGenerateImageBatchRequest, PyTorchGenerateImageBatchResult, PyTorchGenerateImageRequest,
    PyTorchGenerateImageResult,
};
use crate::backend::BackendError;
use crate::device_contracts::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, InferenceDevicePolicy, RuntimeVariantId,
};
use crate::image_generation_planner::{
    plan_image_generation_execution, ImageGenerationPlanningInput, ImageGenerationPlanningOutcome,
};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasModelRef,
};
use crate::{ImageGenerationRequest, InferenceTaskId, ResolvedModelPackageFacts};
use pyo3::prelude::*;
use std::ffi::CString;

fn load_worker_image_contract_module<'py>(py: Python<'py>) -> Bound<'py, pyo3::types::PyModule> {
    let worker_contract_source = CString::new(include_str!("../../torch/worker_contract.py"))
        .expect("worker contract source should not contain nul bytes");
    let worker_contract = pyo3::types::PyModule::from_code(
        py,
        &worker_contract_source,
        c"worker_contract.py",
        c"worker_contract",
    )
    .expect("worker_contract module should load");
    let sys = py.import("sys").expect("sys should import");
    sys.getattr("modules")
        .expect("sys.modules should exist")
        .set_item("worker_contract", worker_contract)
        .expect("worker_contract should register in sys.modules");

    let image_contract_source = CString::new(include_str!("../../torch/worker_image_contract.py"))
        .expect("worker image contract source should not contain nul bytes");
    pyo3::types::PyModule::from_code(
        py,
        &image_contract_source,
        c"worker_image_contract.py",
        c"worker_image_contract",
    )
    .expect("worker_image_contract module should load")
}

fn artifact_load_target(facts: &ResolvedModelPackageFacts) -> PumasArtifactLoadTarget {
    PumasArtifactLoadTarget {
        model_ref: facts.model_ref.clone(),
        artifact_kind: ModelArtifactKind::DiffusersBundle,
        local_load_path: "/pumas/models/image/stable-diffusion/tiny-sd".to_string(),
        load_path_kind: PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("test-root".to_string()),
        storage_kind: ModelStorageKind::LibraryOwned,
        validation_state: ModelValidationState::Valid,
        content_fingerprint: None,
        package_facts_contract_version: Some(facts.package_facts_contract_version),
    }
}

#[test]
fn test_pytorch_worker_generate_image_request_fixture_decodes() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateImageRequest> =
        serde_json::from_str(fixture).expect("decode worker image request fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GenerateImage);
    assert_eq!(
        envelope.payload.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        envelope.payload.family,
        ImageGenerationFamilyLabel::StableDiffusion
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
    assert!(envelope
        .payload
        .required_components
        .contains(&DiffusersComponentRole::Unet));
    validate_generate_image_envelope(&envelope).expect("fixture should validate");
}

#[test]
fn test_pytorch_worker_generate_image_response_fixture_decodes() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_response.json");
    let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
        serde_json::from_str(fixture).expect("decode worker image response fixture");

    let PyTorchWorkerResponse::Ok(success) = response else {
        panic!("expected image response success fixture");
    };
    assert_eq!(success.request_id, "req-image-001");
    assert_eq!(success.result.images.len(), 1);
    assert_eq!(success.result.images[0].mime_type, "image/png");
    assert_eq!(success.result.seed_used, Some(42));
}

#[test]
fn test_pytorch_worker_generate_image_batch_request_fixture_decodes() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateImageBatchRequest> =
        serde_json::from_str(fixture).expect("decode worker image batch request fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::GenerateImageBatch
    );
    assert_eq!(envelope.payload.batch_execution_id, "image-batch-001");
    assert_eq!(envelope.payload.anchor_member_id, "member-001");
    assert_eq!(envelope.payload.members.len(), 2);
    assert_eq!(envelope.payload.members[0].member_id, "member-001");
    assert_eq!(
        envelope.payload.members[0].request.prompt,
        "a compact test image"
    );
    assert_eq!(envelope.payload.members[1].member_id, "member-002");
    assert_eq!(
        envelope.payload.members[1]
            .request
            .device
            .as_ref()
            .map(|device| device.as_str()),
        Some("cpu")
    );
    validate_generate_image_batch_envelope(&envelope).expect("fixture should validate");
}

#[test]
fn test_pytorch_worker_generate_image_batch_response_fixture_decodes() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_response.json"
    );
    let response: PyTorchWorkerResponse<PyTorchGenerateImageBatchResult> =
        serde_json::from_str(fixture).expect("decode worker image batch response fixture");

    let PyTorchWorkerResponse::Ok(success) = response else {
        panic!("expected image batch response success fixture");
    };
    assert_eq!(success.request_id, "req-image-batch-001");
    assert_eq!(success.result.batch_execution_id, "image-batch-001");
    assert_eq!(success.result.members.len(), 2);
    assert_eq!(success.result.members[0].member_id, "member-001");
    assert!(success.result.members[0].result.is_some());
    assert_eq!(success.result.members[1].member_id, "member-002");
    assert!(success.result.members[1].error.is_some());
}

#[test]
fn test_pytorch_worker_generate_image_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateImageRequest> =
        serde_json::from_str(fixture).expect("decode worker image request fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match validate_generate_image_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_image_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_image_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateImageRequest> =
        serde_json::from_str(fixture).expect("decode worker image request fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match validate_generate_image_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("generate_image envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_image_batch_envelope_rejects_duplicate_member_ids() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateImageBatchRequest> =
        serde_json::from_str(fixture).expect("decode worker image batch request fixture");
    envelope.payload.members[1].member_id = "member-001".to_string();

    match validate_generate_image_batch_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("duplicate member_id member-001"));
        }
        other => panic!("expected duplicate-member config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_image_request_rejects_unknown_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
    ))
    .expect("decode image request value");
    value["payload"]["trust_remote_code"] = serde_json::json!(true);

    let error = serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchGenerateImageRequest>>(value)
        .expect_err("image request payload should reject unknown fields");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn test_pytorch_worker_generate_image_request_maps_from_validated_plan() {
    let facts: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
    ))
    .expect("decode image package facts");
    let request = ImageGenerationRequest {
        model: "image/stable-diffusion/tiny-sd".to_string(),
        prompt: "a compact test image".to_string(),
        negative_prompt: Some("blur".to_string()),
        width: Some(512),
        height: Some(512),
        num_inference_steps: Some(8),
        guidance_scale: Some(7.5),
        seed: Some(42),
        denoising_scheduler: None,
        num_images_per_prompt: Some(1),
        init_image: None,
        mask_image: None,
        strength: None,
        extra_options: serde_json::Value::Null,
    };
    let decision = backend_decision();
    let outcome = plan_image_generation_execution(ImageGenerationPlanningInput {
        request: &request,
        package_facts: &facts,
        artifact_load_target: &artifact_load_target(&facts),
        backend_decision: &decision,
    });
    let ImageGenerationPlanningOutcome::Planned { plan } = outcome else {
        panic!("expected validated image plan");
    };

    let worker_request = PyTorchGenerateImageRequest::from(&plan);

    assert_eq!(
        worker_request.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        worker_request.artifact_load_target.local_load_path,
        "/pumas/models/image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        worker_request.family,
        ImageGenerationFamilyLabel::StableDiffusion
    );
    assert_eq!(worker_request.pipeline_class, "StableDiffusionPipeline");
    assert_eq!(
        worker_request.device.as_ref().map(|device| device.as_str()),
        Some("cpu")
    );
    assert_eq!(worker_request.prompt, "a compact test image");
    validate_generate_image_envelope(&PyTorchWorkerEnvelope::new(
        "req-image-plan",
        PyTorchWorkerOperation::GenerateImage,
        worker_request,
    ))
    .expect("planned worker request should validate");
}

#[test]
fn test_python_worker_generate_image_contract_projects_planned_kwargs() {
    Python::with_gil(|py| {
        let module = load_worker_image_contract_module(py);
        let envelope = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        );

        let projected = module
            .call_method1("generate_image_kwargs_from_envelope", (envelope,))
            .expect("image envelope should validate");
        let device = projected
            .get_item("device")
            .expect("device key should exist")
            .extract::<String>()
            .expect("device should be a string");
        let generation_kwargs = projected
            .get_item("generation_kwargs")
            .expect("generation kwargs should exist");
        let local_load_path = projected
            .get_item("local_load_path")
            .expect("local load path key should exist")
            .extract::<String>()
            .expect("local load path should be a string");

        assert_eq!(device, "cpu");
        assert_eq!(
            local_load_path,
            "/pumas/models/image/stable-diffusion/tiny-sd"
        );
        assert_eq!(
            generation_kwargs
                .get_item("prompt")
                .expect("prompt key should exist")
                .extract::<String>()
                .expect("prompt should be a string"),
            "a compact test image"
        );
        assert_eq!(
            generation_kwargs
                .get_item("num_inference_steps")
                .expect("steps key should exist")
                .extract::<u32>()
                .expect("steps should be an integer"),
            8
        );
        assert_eq!(
            generation_kwargs
                .get_item("denoising_scheduler")
                .expect("denoising scheduler key should exist")
                .extract::<Option<String>>()
                .expect("denoising scheduler should be optional"),
            None
        );
    });
}

#[test]
fn test_python_worker_generate_image_batch_contract_projects_member_kwargs() {
    Python::with_gil(|py| {
        let module = load_worker_image_contract_module(py);
        let envelope = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_batch_request.json"
        );

        let projected = module
            .call_method1("generate_image_batch_kwargs_from_envelope", (envelope,))
            .expect("image batch envelope should validate");
        assert_eq!(
            projected
                .get_item("batch_execution_id")
                .expect("batch execution id key should exist")
                .extract::<String>()
                .expect("batch execution id should be a string"),
            "image-batch-001"
        );
        assert_eq!(
            projected
                .get_item("anchor_member_id")
                .expect("anchor member id key should exist")
                .extract::<String>()
                .expect("anchor member id should be a string"),
            "member-001"
        );
        let members = projected
            .get_item("members")
            .expect("members key should exist");
        assert_eq!(
            members
                .call_method0("__len__")
                .expect("members should have a length")
                .extract::<usize>()
                .expect("members length should be an integer"),
            2
        );
        let first_member = members.get_item(0).expect("first member should exist");
        assert_eq!(
            first_member
                .get_item("member_id")
                .expect("member id key should exist")
                .extract::<String>()
                .expect("member id should be a string"),
            "member-001"
        );
        let planned = first_member
            .get_item("planned")
            .expect("planned key should exist");
        let generation_kwargs = planned
            .get_item("generation_kwargs")
            .expect("generation kwargs should exist");
        assert_eq!(
            generation_kwargs
                .get_item("prompt")
                .expect("prompt key should exist")
                .extract::<String>()
                .expect("prompt should be a string"),
            "a compact test image"
        );
    });
}

#[test]
fn test_python_worker_generate_image_contract_rejects_unknown_payload_fields() {
    Python::with_gil(|py| {
        let module = load_worker_image_contract_module(py);
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]["trust_remote_code"] = serde_json::json!(true);

        let error = module
            .call_method1(
                "generate_image_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("unknown image payload fields should fail validation");

        assert!(error
            .to_string()
            .contains("unsupported key(s): trust_remote_code"));
    });
}

#[test]
fn test_python_worker_generate_image_contract_requires_rust_selected_device() {
    Python::with_gil(|py| {
        let module = load_worker_image_contract_module(py);
        let mut envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_image_request.json"
        ))
        .expect("decode image request fixture");
        envelope["payload"]
            .as_object_mut()
            .expect("payload should be an object")
            .remove("device");

        let error = module
            .call_method1(
                "generate_image_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("missing planned device should fail validation");

        assert!(error
            .to_string()
            .contains("payload.device must be selected by Rust"));
    });
}

fn backend_decision() -> BackendExecutionDecision {
    let backend_id = BackendId::parse("pytorch").expect("valid backend id");
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
    BackendExecutionDecision {
        selected_backend_id: backend_id,
        selected_runtime_variant_id: runtime_variant_id,
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(selected_device_id),
        device_decision,
        selected_task_id: Some(InferenceTaskId::ImageGeneration),
        selected_model_ref: Some(PumasModelRef {
            model_id: "pumas://models/image/stable-diffusion/tiny-sd".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
        diagnostics: Vec::new(),
        dependency_readiness: crate::pytorch_diffusers_image_generation_package_requirements()
            .into_iter()
            .map(|declaration| {
                declaration.to_readiness_fact(
                    crate::CapabilityAvailabilityState::Available,
                    crate::DependencyReadinessResolverOwner::EmbeddedRuntime,
                )
            })
            .collect(),
        selection_policy_trace: None,
    }
}

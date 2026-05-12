use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::pytorch_worker_image_contract::{
    validate_generate_image_envelope, PyTorchGenerateImageRequest, PyTorchGenerateImageResult,
};
use crate::backend::BackendError;
use crate::device_contracts::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, InferenceDevicePolicy, RuntimeVariantId,
};
use crate::image_generation_planner::{
    plan_image_generation_execution, ImageGenerationPlanningInput, ImageGenerationPlanningOutcome,
};
use crate::model_contracts::{DiffusersComponentRole, ImageGenerationFamilyLabel};
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
        scheduler: Some("euler".to_string()),
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
        worker_request.artifact_entry_path,
        "image/stable-diffusion/tiny-sd"
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

        assert_eq!(device, "cpu");
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
        selected_model_ref: None,
        diagnostics: Vec::new(),
    }
}

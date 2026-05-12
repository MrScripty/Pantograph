use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::pytorch_worker_image_contract::{
    validate_generate_image_envelope, PyTorchGenerateImageRequest, PyTorchGenerateImageResult,
};
use crate::backend::BackendError;
use crate::model_contracts::{DiffusersComponentRole, ImageGenerationFamilyLabel};

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

use inference::{
    BackendExecutionDecision, DeviceResolutionDiagnosticCode, InferenceDeviceClass,
    InferenceTaskId, RuntimeVariantCapability,
};

const BACKEND_EXECUTION_DECISION_FIXTURE: &str =
    include_str!("fixtures/device_contracts/backend_execution_decision.json");
const RUNTIME_VARIANT_CAPABILITY_FIXTURE: &str =
    include_str!("fixtures/device_contracts/runtime_variant_capability.json");

#[test]
fn backend_execution_decision_fixture_decodes_through_public_contract() {
    let decision: BackendExecutionDecision =
        serde_json::from_str(BACKEND_EXECUTION_DECISION_FIXTURE)
            .expect("backend execution decision fixture should decode");

    assert_eq!(decision.selected_backend_id.as_str(), "pytorch");
    assert_eq!(
        decision.selected_runtime_variant_id.as_str(),
        "pytorch.cuda"
    );
    assert_eq!(decision.selected_device_class, InferenceDeviceClass::Cuda);
    assert_eq!(
        decision.selected_task_id,
        Some(InferenceTaskId::ImageGeneration)
    );
    assert_eq!(
        decision
            .selected_model_ref
            .as_ref()
            .map(|model| model.model_id.as_str()),
        Some("pumas://model/juggernaut-xl")
    );

    let encoded = serde_json::to_string(&decision).expect("encode decision");
    let decoded: BackendExecutionDecision =
        serde_json::from_str(&encoded).expect("decode encoded decision");
    assert_eq!(decoded.selected_backend_id, decision.selected_backend_id);
    assert_eq!(
        decoded.device_decision.selected_device_id,
        decision.device_decision.selected_device_id
    );
}

#[test]
fn runtime_variant_capability_fixture_preserves_diagnostics() {
    let capability: RuntimeVariantCapability =
        serde_json::from_str(RUNTIME_VARIANT_CAPABILITY_FIXTURE)
            .expect("runtime variant capability fixture should decode");

    assert_eq!(capability.runtime_variant_id.as_str(), "llama_cpp.cuda");
    assert_eq!(capability.device_class, InferenceDeviceClass::Cuda);
    assert!(!capability.available);
    assert_eq!(
        capability.diagnostics[0].code,
        DeviceResolutionDiagnosticCode::MissingRuntimeVariant
    );
    assert_eq!(
        capability.diagnostics[0]
            .backend_id
            .as_ref()
            .map(|backend_id| backend_id.as_str()),
        Some("llama_cpp")
    );
}

#[test]
fn device_contract_fixtures_reject_invalid_raw_identifiers() {
    let invalid = serde_json::json!({
        "runtime_variant_id": "llama_cpp.cuda",
        "device_class": "cuda",
        "available": true,
        "diagnostics": [
            {
                "code": "legacy_device_rejected",
                "severity": "error",
                "message": "legacy CUDA selector rejected",
                "device_id": "CUDA0"
            }
        ]
    });

    let error = serde_json::from_value::<RuntimeVariantCapability>(invalid)
        .expect_err("legacy raw device id should fail fixture decoding");
    assert!(error.to_string().contains("invalid identifier shape"));
}

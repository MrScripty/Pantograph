use inference::{
    BackendExecutionCandidate, BackendExecutionDecision, DeviceResolutionDecision,
    DeviceResolutionDiagnosticCode, DeviceResolutionRequest, InferenceDeviceClass,
    InferenceDevicePolicy, InferenceTaskId, LlamaCppDeviceInventoryFact, RuntimeVariantCapability,
};

const BACKEND_EXECUTION_DECISION_FIXTURE: &str =
    include_str!("fixtures/device_contracts/backend_execution_decision.json");
const BACKEND_EXECUTION_CANDIDATE_FIXTURE: &str =
    include_str!("fixtures/device_contracts/backend_execution_candidate.json");
const DEVICE_RESOLUTION_DECISION_FIXTURE: &str =
    include_str!("fixtures/device_contracts/device_resolution_decision.json");
const DEVICE_RESOLUTION_REQUEST_FIXTURE: &str =
    include_str!("fixtures/device_contracts/device_resolution_request.json");
const RUNTIME_VARIANT_CAPABILITY_FIXTURE: &str =
    include_str!("fixtures/device_contracts/runtime_variant_capability.json");
const LLAMACPP_DEVICE_INVENTORY_FACT_FIXTURE: &str =
    include_str!("fixtures/device_contracts/llamacpp_device_inventory_fact.json");

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
    let trace = decision
        .selection_policy_trace
        .as_ref()
        .expect("fixture should carry scheduler policy trace");
    assert_eq!(trace.policy_version, 1);
    assert_eq!(
        trace
            .candidate_set_summary
            .as_ref()
            .map(|summary| summary.eligible_candidate_count),
        Some(1)
    );
    assert_eq!(
        trace.ranking_reason.as_deref(),
        Some("readiness_history_preferred")
    );
    assert_eq!(
        trace.seed_basis.as_deref(),
        Some("workflow:image-generation:juggernaut-xl")
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
fn backend_execution_candidate_fixture_preserves_scheduler_evidence() {
    let candidate: BackendExecutionCandidate =
        serde_json::from_str(BACKEND_EXECUTION_CANDIDATE_FIXTURE)
            .expect("backend execution candidate fixture should decode");

    assert_eq!(candidate.backend_id.as_str(), "pytorch");
    assert!(candidate.model_compatible);
    assert_eq!(
        candidate
            .model_ref
            .as_ref()
            .map(|model| model.model_id.as_str()),
        Some("pumas://model/juggernaut-xl")
    );
    assert_eq!(
        candidate.supported_task_ids,
        vec![InferenceTaskId::ImageGeneration]
    );
    assert_eq!(candidate.runtime_variant_id.as_str(), "pytorch.cuda");
    assert_eq!(candidate.device_class, InferenceDeviceClass::Cuda);
    assert_eq!(
        candidate.device_id.as_ref().map(|id| id.as_str()),
        Some("cuda:0")
    );
    assert_eq!(
        candidate
            .resource_estimate
            .as_ref()
            .and_then(|estimate| estimate.vram_mb),
        Some(8_192)
    );
    assert_eq!(
        candidate
            .observed_throughput
            .as_ref()
            .and_then(|throughput| throughput.images_per_minute),
        Some(4.5)
    );

    let encoded = serde_json::to_value(&candidate).expect("encode candidate");
    let fixture: serde_json::Value =
        serde_json::from_str(BACKEND_EXECUTION_CANDIDATE_FIXTURE).expect("fixture parses");
    assert_eq!(encoded, fixture);
}

#[test]
fn device_resolution_request_fixture_preserves_explicit_policy() {
    let request: DeviceResolutionRequest = serde_json::from_str(DEVICE_RESOLUTION_REQUEST_FIXTURE)
        .expect("device resolution request fixture should decode");

    let InferenceDevicePolicy::Explicit {
        device_class,
        device_id,
    } = &request.policy
    else {
        panic!("request fixture should use explicit policy");
    };
    assert_eq!(*device_class, InferenceDeviceClass::Cuda);
    assert_eq!(device_id.as_ref().map(|id| id.as_str()), Some("cuda:0"));
    assert_eq!(request.runtime_variant_id.as_str(), "pytorch.cuda");
    assert_eq!(
        request.candidate_device_classes,
        vec![InferenceDeviceClass::Cpu, InferenceDeviceClass::Cuda]
    );

    let encoded = serde_json::to_value(&request).expect("encode request");
    let fixture: serde_json::Value =
        serde_json::from_str(DEVICE_RESOLUTION_REQUEST_FIXTURE).expect("fixture parses");
    assert_eq!(encoded, fixture);
}

#[test]
fn device_resolution_decision_fixture_preserves_runtime_load_device_choice() {
    let decision: DeviceResolutionDecision =
        serde_json::from_str(DEVICE_RESOLUTION_DECISION_FIXTURE)
            .expect("device resolution decision fixture should decode");

    let InferenceDevicePolicy::Explicit {
        device_class,
        device_id,
    } = &decision.policy
    else {
        panic!("decision fixture should use explicit policy");
    };
    assert_eq!(*device_class, InferenceDeviceClass::Cuda);
    assert_eq!(device_id.as_ref().map(|id| id.as_str()), Some("cuda:0"));
    assert_eq!(decision.runtime_variant_id.as_str(), "pytorch.cuda");
    assert_eq!(decision.selected_device_class, InferenceDeviceClass::Cuda);
    assert_eq!(
        decision.selected_device_id.as_ref().map(|id| id.as_str()),
        Some("cuda:0")
    );
    assert!(decision.diagnostics.is_empty());

    let encoded = serde_json::to_value(&decision).expect("encode decision");
    let fixture: serde_json::Value =
        serde_json::from_str(DEVICE_RESOLUTION_DECISION_FIXTURE).expect("fixture parses");
    assert_eq!(encoded, fixture);
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
fn llamacpp_device_inventory_fact_fixture_preserves_canonical_projection() {
    let fact: LlamaCppDeviceInventoryFact =
        serde_json::from_str(LLAMACPP_DEVICE_INVENTORY_FACT_FIXTURE)
            .expect("llama.cpp device inventory fact fixture should decode");

    assert_eq!(fact.backend_device_id, "CUDA0");
    assert_eq!(fact.device_class, Some(InferenceDeviceClass::Cuda));
    assert_eq!(
        fact.device_id.as_ref().map(|id| id.as_str()),
        Some("cuda:0")
    );
    assert_eq!(fact.total_vram_mb, 8_188);
    assert!(fact.diagnostics.is_empty());

    let encoded = serde_json::to_value(&fact).expect("encode inventory fact");
    let fixture: serde_json::Value =
        serde_json::from_str(LLAMACPP_DEVICE_INVENTORY_FACT_FIXTURE).expect("fixture parses");
    assert_eq!(encoded, fixture);
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

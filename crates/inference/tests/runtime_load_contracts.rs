use inference::{
    InferenceDeviceClass, InferenceDevicePolicy, ManagedBinaryId, ManagedRuntimeReadinessState,
    RuntimeLoadPhase, RuntimeLoadPhaseRecord,
};

const RUNTIME_LOAD_PHASE_RECORD_FIXTURE: &str =
    include_str!("fixtures/runtime_load/runtime_load_phase_record.json");

#[test]
fn runtime_load_phase_record_fixture_preserves_resolved_device_decision() {
    let record: RuntimeLoadPhaseRecord = serde_json::from_str(RUNTIME_LOAD_PHASE_RECORD_FIXTURE)
        .expect("runtime load phase fixture should decode");

    assert_eq!(record.phase, RuntimeLoadPhase::DependencyResolved);
    assert_eq!(record.runtime.runtime_id, ManagedBinaryId::LlamaCpp);
    assert_eq!(
        record.runtime.readiness_state,
        ManagedRuntimeReadinessState::Ready
    );
    assert_eq!(
        record.device_decision.runtime_variant_id.as_str(),
        "llama_cpp.cpu"
    );
    assert_eq!(
        record.device_decision.selected_device_class,
        InferenceDeviceClass::Cpu
    );
    assert_eq!(
        record
            .device_decision
            .selected_device_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("cpu")
    );
    assert_eq!(
        record
            .command
            .as_ref()
            .map(|command| command.args.as_slice()),
        Some(&["--port".to_string(), "8080".to_string()][..])
    );

    let InferenceDevicePolicy::Explicit {
        device_class,
        device_id,
    } = &record.device_decision.policy
    else {
        panic!("runtime-load fixture should carry explicit resolved policy");
    };
    assert_eq!(*device_class, InferenceDeviceClass::Cpu);
    assert_eq!(device_id.as_ref().map(|id| id.as_str()), Some("cpu"));

    let encoded = serde_json::to_value(&record).expect("encode runtime load phase");
    let fixture: serde_json::Value =
        serde_json::from_str(RUNTIME_LOAD_PHASE_RECORD_FIXTURE).expect("fixture parses");
    assert_eq!(encoded, fixture);
}

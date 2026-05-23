use pantograph_scheduler::{
    SchedulerContractError, SchedulerResourceFitState, SchedulerResourceObservationError,
    SchedulerResourceObserver, SchedulerResourceResidencySnapshot, SchedulerRuntimeReadinessState,
    ValidatedSchedulerResourceResidencySnapshot, SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION,
};

#[test]
fn valid_resource_residency_fixture_decodes_and_validates() {
    let snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must match scheduler resource residency contract");

    let validated = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect("fixture must validate before scheduler policy consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION
    );
    assert_eq!(validated.as_ref().device_resources.len(), 2);
}

#[test]
fn rejects_executable_paths_and_worker_launch_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "observed_at_unix_ms": 1779481200000_u64,
        "local_load_path": "/models/juggernaut/model.safetensors",
        "worker_command": "python worker.py"
    });

    let error = serde_json::from_value::<SchedulerResourceResidencySnapshot>(value)
        .expect_err("resource snapshots must reject executable path and launch fields");

    assert!(
        error
            .to_string()
            .contains("unknown field `local_load_path`")
            || error.to_string().contains("unknown field `worker_command`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_resource_accounting_overflow() {
    let mut snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    snapshot.device_resources[0].available_bytes = u64::MAX;

    let error = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect_err("resource accounting must use checked arithmetic");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "device_resource.available_bytes",
            reason: "available plus reserved bytes must not overflow"
        }
    );
}

#[test]
fn rejects_resource_accounting_above_total() {
    let mut snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    snapshot.device_resources[0].available_bytes = snapshot.device_resources[0].total_bytes;

    let error = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect_err("available plus reserved bytes must not exceed total");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "device_resource.available_bytes",
            reason: "available plus reserved bytes must not exceed total bytes"
        }
    );
}

#[test]
fn unavailable_runtime_readiness_requires_diagnostics() {
    let mut snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    snapshot.runtime_readiness[0].state = SchedulerRuntimeReadinessState::Failed;
    snapshot.runtime_readiness[0].diagnostics.clear();

    let error = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect_err("failed runtime readiness must explain why");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "runtime_readiness.diagnostics"
        }
    );
}

#[test]
fn duplicate_device_resource_observation_is_rejected() {
    let mut snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    let duplicate = snapshot.device_resources[0].clone();
    snapshot.device_resources.push(duplicate);

    let error = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect_err("resource observations must be unique by device and kind");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "device_resources",
            reason: "device resource observations must be unique by device and resource kind"
        }
    );
}

#[test]
fn impossible_fit_requires_diagnostics() {
    let mut snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    snapshot.fit_assessments[0].state = SchedulerResourceFitState::ImpossibleFit;
    snapshot.fit_assessments[0].diagnostics.clear();

    let error = ValidatedSchedulerResourceResidencySnapshot::try_from(snapshot)
        .expect_err("impossible fit must explain why scheduler cannot run the task");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "resource_fit_assessment.diagnostics"
        }
    );
}

#[test]
fn observer_trait_returns_validated_snapshot() {
    struct FakeObserver(SchedulerResourceResidencySnapshot);

    impl SchedulerResourceObserver for FakeObserver {
        fn observe(
            &self,
        ) -> Result<ValidatedSchedulerResourceResidencySnapshot, SchedulerResourceObservationError>
        {
            ValidatedSchedulerResourceResidencySnapshot::try_from(self.0.clone())
                .map_err(SchedulerResourceObservationError::from)
        }
    }

    let snapshot: SchedulerResourceResidencySnapshot = serde_json::from_str(include_str!(
        "fixtures/resource_residency_snapshot_valid.json"
    ))
    .expect("fixture must decode");
    let observed = FakeObserver(snapshot)
        .observe()
        .expect("fake observer must return a validated snapshot");

    assert_eq!(observed.as_ref().runtime_readiness.len(), 2);
}

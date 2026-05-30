use pantograph_dependency_planning::{
    known_device_classes, known_device_toolchain_ids, known_runtime_feature_ids,
    DependencyInventoryObservationFreshness, DependencyPlanningContractError,
    DependencyProviderSourceAlternative, DependencyProviderSourceState, DeviceClassSourceId,
    DeviceToolchainProviderSourceSnapshot, RuntimeFeatureProviderSourceSnapshot,
    RuntimeFeatureSourceId, ValidatedDeviceToolchainProviderSourceSnapshot,
    ValidatedRuntimeFeatureProviderSourceSnapshot,
};

const PROVIDER_SOURCE_FIXTURE: &str =
    include_str!("fixtures/dependency_provider_source_snapshots.json");

#[test]
fn provider_source_fixture_decodes_runtime_feature_and_device_toolchain_snapshots() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");

    let runtime_feature: RuntimeFeatureProviderSourceSnapshot =
        serde_json::from_value(fixture["runtime_feature"].clone())
            .expect("runtime feature source snapshot should decode");
    let runtime_feature = ValidatedRuntimeFeatureProviderSourceSnapshot::try_from(runtime_feature)
        .expect("runtime feature source snapshot should validate");

    let device_toolchain: DeviceToolchainProviderSourceSnapshot =
        serde_json::from_value(fixture["device_toolchain"].clone())
            .expect("device toolchain source snapshot should decode");
    let device_toolchain =
        ValidatedDeviceToolchainProviderSourceSnapshot::try_from(device_toolchain)
            .expect("device toolchain source snapshot should validate");

    assert_eq!(runtime_feature.as_snapshot().rows.len(), 2);
    assert_eq!(device_toolchain.as_snapshot().rows.len(), 2);
}

#[test]
fn provider_source_contract_pins_canonical_vocabularies() {
    assert_eq!(
        known_runtime_feature_ids(),
        &[
            "streaming",
            "device_selection",
            "external_connection",
            "kv_cache",
            "custom_code",
            "preprocessing",
            "postprocessing",
            "request_lifecycle",
        ]
    );
    assert_eq!(
        known_device_toolchain_ids(),
        &[
            "cuda_runtime",
            "metal_runtime",
            "mps_runtime",
            "llamacpp_device_inventory",
            "pytorch_device_probe",
        ]
    );
    assert_eq!(known_device_classes(), &["cpu", "cuda", "metal", "mps"]);
}

#[test]
fn runtime_feature_source_rejects_unknown_feature_ids() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");
    let mut snapshot: RuntimeFeatureProviderSourceSnapshot =
        serde_json::from_value(fixture["runtime_feature"].clone())
            .expect("runtime feature source snapshot should decode");
    snapshot.rows[0].feature_id =
        RuntimeFeatureSourceId::parse("display_label_feature").expect("feature id syntax");

    assert_eq!(
        ValidatedRuntimeFeatureProviderSourceSnapshot::try_from(snapshot)
            .expect_err("unknown feature ids should fail validation"),
        DependencyPlanningContractError::InvalidField {
            field: "runtime_feature.source_id",
            reason: "runtime feature source id is not in the canonical provider-source vocabulary",
        }
    );
}

#[test]
fn device_toolchain_source_rejects_unknown_device_classes() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");
    let mut snapshot: DeviceToolchainProviderSourceSnapshot =
        serde_json::from_value(fixture["device_toolchain"].clone())
            .expect("device toolchain source snapshot should decode");
    snapshot.rows[0].device_class =
        Some(DeviceClassSourceId::parse("display_gpu").expect("device class syntax"));

    assert_eq!(
        ValidatedDeviceToolchainProviderSourceSnapshot::try_from(snapshot)
            .expect_err("unknown device classes should fail validation"),
        DependencyPlanningContractError::InvalidField {
            field: "device_class.source_id",
            reason: "device class source id is not in the canonical provider-source vocabulary",
        }
    );
}

#[test]
fn provider_source_rejects_stale_rows_without_diagnostics() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");
    let mut snapshot: RuntimeFeatureProviderSourceSnapshot =
        serde_json::from_value(fixture["runtime_feature"].clone())
            .expect("runtime feature source snapshot should decode");
    snapshot.rows[0].state = DependencyProviderSourceState::Stale;
    snapshot.rows[0].freshness = DependencyInventoryObservationFreshness::Stale;

    assert_eq!(
        ValidatedRuntimeFeatureProviderSourceSnapshot::try_from(snapshot)
            .expect_err("stale source rows should explain their staleness"),
        DependencyPlanningContractError::MissingField {
            field: "runtime_feature_provider_source.diagnostics",
        }
    );
}

#[test]
fn provider_source_rejects_unknown_legacy_fields() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");
    let mut runtime_feature = fixture["runtime_feature"].clone();
    runtime_feature["rows"][0]
        .as_object_mut()
        .expect("row should be object")
        .insert(
            "backend_display_name".to_string(),
            serde_json::json!("PyTorch"),
        );

    ValidatedRuntimeFeatureProviderSourceSnapshot::try_from(runtime_feature)
        .expect_err("source contract should reject display-shaped legacy fields");
}

#[test]
fn provider_source_bounds_alternative_count() {
    let fixture: serde_json::Value =
        serde_json::from_str(PROVIDER_SOURCE_FIXTURE).expect("fixture should decode");
    let mut snapshot: DeviceToolchainProviderSourceSnapshot =
        serde_json::from_value(fixture["device_toolchain"].clone())
            .expect("device toolchain source snapshot should decode");
    snapshot.rows[0].alternatives = vec![DependencyProviderSourceAlternative::default(); 9];

    assert_eq!(
        ValidatedDeviceToolchainProviderSourceSnapshot::try_from(snapshot)
            .expect_err("alternatives should be bounded"),
        DependencyPlanningContractError::FieldTooLong {
            field: "dependency_provider_source_alternatives",
            max_len: 8,
        }
    );
}

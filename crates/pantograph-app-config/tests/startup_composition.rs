use std::sync::{Arc, Barrier};

use pantograph_app_config::{AppConfig, ConfigError};
use pantograph_runtime_registry::{
    RuntimeRegistryError, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim, RuntimeRetentionHint,
};

const DECLARATION: &str = include_str!(
    "../../pantograph-runtime-registry/tests/fixtures/startup_shared_resource_config.json"
);

#[tokio::test]
async fn resident_estimates_survive_actual_cold_load_and_save_without_inventing_residency() {
    let directory = tempfile::tempdir().unwrap();
    let mut input: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
    input["runtime_model_resident_estimates"] = serde_json::json!([{
        "runtime_id": "Torch", "model_id": "/models/exact-target",
        "requirements": { "claims": [{ "kind": "ram_bytes", "bytes": 40 }, { "kind": "vram_bytes", "bytes": 0 }] }
    }]);
    tokio::fs::write(directory.path().join("config.json"), input.to_string())
        .await
        .unwrap();
    let (config, registry) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert_eq!(
        config
            .runtime_resources
            .runtime_model_resident_estimates
            .len(),
        1
    );
    assert!(registry
        .snapshot()
        .runtimes
        .iter()
        .all(|runtime| runtime.model_resource_residency.is_none()));
    config.save(&directory.path().to_path_buf()).await.unwrap();
    let (restored, cold) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert_eq!(config.runtime_resources, restored.runtime_resources);
    assert!(cold
        .snapshot()
        .runtimes
        .iter()
        .all(|runtime| runtime.models.is_empty()));
    let frame = pantograph_runtime_registry::RuntimeProducerObservation {
        source_id: "portable-owner".into(),
        sequence: 1,
        allocation_state: pantograph_runtime_registry::RuntimeProducerAllocationState::Resident,
        observation: pantograph_runtime_registry::RuntimeObservation {
            runtime_id: "pytorch".into(),
            display_name: "PyTorch".into(),
            backend_keys: vec!["pytorch".into()],
            model_id: Some("/models/exact-target".into()),
            runtime_instance_id: Some("instance".into()),
            status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    };
    let published = cold.observe_runtime_producer(frame).unwrap();
    assert_eq!(
        published
            .model_resource_residency
            .unwrap()
            .requirements
            .unwrap()
            .claims[0]
            .bytes,
        40
    );
    assert_eq!(
        cold.evaluate_reservation(request("candle", "probe", 0))
            .unwrap()
            .observation()
            .resource_domains[0]
            .available_bytes,
        60
    );
}

#[test]
fn invalid_resident_estimates_fail_actual_app_config_composition() {
    let base: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
    let valid = serde_json::json!({ "runtime_id": "pytorch", "model_id": "exact", "requirements": { "claims": [{ "kind": "ram_bytes", "bytes": 0 }] } });
    for invalid in [
        serde_json::json!([{ "runtime_id": "invented", "model_id": "exact", "requirements": { "claims": [{ "kind": "ram_bytes", "bytes": 40 }] } }]),
        serde_json::json!([{ "runtime_id": "pytorch", "model_id": "", "requirements": { "claims": [{ "kind": "ram_bytes", "bytes": 40 }] } }]),
        serde_json::json!([{ "runtime_id": "pytorch", "model_id": "exact", "requirements": { "claims": [] } }]),
        serde_json::json!([valid.clone(), valid.clone()]),
        serde_json::json!([{ "runtime_id": "pytorch", "model_id": "exact", "requirements": { "claims": [{ "kind": "ram_bytes", "bytes": u64::MAX }, { "kind": "ram_bytes", "bytes": 1 }] } }]),
    ] {
        let mut input = base.clone();
        input["runtime_model_resident_estimates"] = invalid;
        let config: AppConfig = serde_json::from_value(input).unwrap();
        assert!(config.runtime_resources.compose_registry().is_err());
    }
    let mut input = base;
    input["runtime_model_resident_estimates"] = serde_json::json!([valid]);
    assert!(serde_json::from_value::<AppConfig>(input)
        .unwrap()
        .runtime_resources
        .compose_registry()
        .is_ok());
}

fn request(runtime: &str, owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        model_id: Some("model".into()),
        usage_profile: None,
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(bytes),
        ])),
    }
}

#[tokio::test]
async fn actual_app_config_cold_load_save_and_composition_admit_only_one_shared_request() {
    let directory = tempfile::tempdir().unwrap();
    tokio::fs::write(directory.path().join("config.json"), DECLARATION)
        .await
        .unwrap();
    let (config, registry) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert_eq!(config.device.device, "auto");
    assert_eq!(config.device.gpu_layers, -1);
    assert_eq!(config.runtime_resources.runtime_resource_domains.len(), 1);
    config.save(&directory.path().to_path_buf()).await.unwrap();
    let saved: serde_json::Value = serde_json::from_str(
        &tokio::fs::read_to_string(directory.path().join("config.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(saved.get("runtime_resource_domains").is_some());
    assert!(saved.get("runtime_resources").is_none());
    let (restored, cold_registry) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert_eq!(restored.runtime_resources, config.runtime_resources);
    for registry in [registry, cold_registry] {
        assert!(registry.snapshot().reservations.is_empty());
        let registry = Arc::new(registry);
        let a = registry
            .evaluate_reservation(request("pytorch", "a", 80))
            .unwrap();
        let b = registry
            .evaluate_reservation(request("candle", "b", 80))
            .unwrap();
        let barrier = Barrier::new(2);
        let results = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                barrier.wait();
                a.commit()
            });
            let b = scope.spawn(|| {
                barrier.wait();
                b.commit()
            });
            [a.join().unwrap(), b.join().unwrap()]
        });
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(
                    result,
                    Err(RuntimeRegistryError::ResourceDomainAdmissionRejected { .. })
                ))
                .count(),
            1
        );
        assert_eq!(registry.snapshot().reservations.len(), 1);
    }
}

#[tokio::test]
async fn actual_startup_composition_preserves_replacement_rollback_and_transfer() {
    let directory = tempfile::tempdir().unwrap();
    tokio::fs::write(directory.path().join("config.json"), DECLARATION)
        .await
        .unwrap();
    let (_, registry) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    let registry = Arc::new(registry);
    let original = registry
        .acquire_reservation(request("pytorch", "a", 60))
        .unwrap();
    let replacement = request("pytorch", "a", 10);
    let expected = registry
        .evaluate_reservation(replacement.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(replacement, &expected, |_| Ok::<(), ()>(()))
        .unwrap();
    assert!(registry
        .acquire_reservation(request("candle", "b", 41))
        .is_err());
    drop(custody);
    assert_eq!(
        registry.snapshot().reservations[0].reservation_id,
        original.reservation_id
    );
    let replacement = registry
        .evaluate_reservation(request("pytorch", "a", 70))
        .unwrap();
    let other = registry
        .acquire_reservation(request("candle", "b", 40))
        .unwrap();
    assert!(replacement.commit().is_err());
    registry.release_reservation(other.reservation_id).unwrap();
    let replacement = request("pytorch", "a", 10);
    let expected = registry
        .evaluate_reservation(replacement.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(replacement, &expected, |_| Ok::<(), ()>(()))
        .unwrap();
    custody.transfer().unwrap();
    registry
        .acquire_reservation(request("candle", "b", 90))
        .unwrap();
}

#[tokio::test]
async fn absent_legacy_and_empty_declarations_preserve_startup_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let (missing, registry) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert!(missing
        .runtime_resources
        .runtime_resource_domains
        .is_empty());
    assert!(registry.snapshot().runtimes.is_empty());
    let (_, registry) =
        AppConfig::load_with_runtime_registry(&directory.path().join("missing/nested"))
            .await
            .unwrap();
    assert!(registry.snapshot().runtimes.is_empty());
    for json in [
        r#"{"models":{},"connection_mode":{"type":"None"}}"#,
        r#"{"models":{},"connection_mode":{"type":"None"},"runtime_resource_domains":[]}"#,
    ] {
        tokio::fs::write(directory.path().join("config.json"), json)
            .await
            .unwrap();
        let (config, registry) = AppConfig::load_with_runtime_registry(directory.path())
            .await
            .unwrap();
        assert_eq!(config.device.device, "auto");
        assert!(config.runtime_resources.runtime_resource_domains.is_empty());
        assert!(registry.snapshot().runtimes.is_empty());
    }
}

#[tokio::test]
async fn malformed_or_unreadable_present_config_never_falls_back_to_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    tokio::fs::write(&path, "{malformed").await.unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::Parse(_))
    ));
    let mut invalid: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
    invalid["runtime_resource_domains"][0]["safety_margin_bytes"] = serde_json::json!(101);
    tokio::fs::write(&path, invalid.to_string()).await.unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::RuntimeResourceDomains(_))
    ));
    invalid["runtime_resource_domains"][0]["bindings"][0]["resource_kind"] =
        serde_json::json!("unknown");
    tokio::fs::write(&path, invalid.to_string()).await.unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::Parse(_))
    ));
    tokio::fs::remove_file(&path).await.unwrap();
    tokio::fs::create_dir(&path).await.unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::Io(_))
    ));
}

#[tokio::test]
async fn non_directory_config_parent_is_an_io_error_instead_of_absence() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("not-a-directory");
    tokio::fs::write(&parent, "file").await.unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(&parent).await,
        Err(ConfigError::Io(_))
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn broken_and_looping_config_symlinks_are_errors_instead_of_absence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::os::unix::fs::symlink(directory.path().join("missing-target.json"), &path).unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::Io(_))
    ));
    tokio::fs::remove_file(&path).await.unwrap();
    std::os::unix::fs::symlink("config.json", &path).unwrap();
    assert!(matches!(
        AppConfig::load_with_runtime_registry(directory.path()).await,
        Err(ConfigError::Io(_))
    ));
    let broken_parent = directory.path().join("broken-parent");
    std::os::unix::fs::symlink(directory.path().join("missing-directory"), &broken_parent).unwrap();
    for parent in [broken_parent.clone(), broken_parent.join("nested")] {
        assert!(matches!(
            AppConfig::load_with_runtime_registry(&parent).await,
            Err(ConfigError::Io(_))
        ));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn inaccessible_config_parent_preserves_the_os_read_error() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("private");
    tokio::fs::create_dir(&parent).await.unwrap();
    let path = parent.join("config.json");
    tokio::fs::write(&path, DECLARATION).await.unwrap();
    tokio::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o0))
        .await
        .unwrap();
    let read = tokio::fs::read_to_string(&path).await;
    let loaded = AppConfig::load_with_runtime_registry(&parent).await;
    tokio::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
    match (read, loaded) {
        (Err(read_error), Err(ConfigError::Io(load_error))) => {
            assert_eq!(read_error.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(load_error.kind(), read_error.kind());
            eprintln!("actual config read and startup both returned PermissionDenied");
        }
        // A privileged runner may read mode-000 directories. It must still
        // compose the declared constraints instead of falling back to defaults.
        (Ok(_), Ok((config, registry))) => {
            assert_eq!(config.runtime_resources.runtime_resource_domains.len(), 1);
            assert_eq!(registry.snapshot().runtimes.len(), 2);
        }
        _ => panic!("startup did not preserve the actual filesystem read outcome"),
    }
}

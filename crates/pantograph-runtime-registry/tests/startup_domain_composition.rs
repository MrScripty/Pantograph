use std::sync::{Arc, Barrier};

use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeRegistry, RuntimeRegistryError, RuntimeRegistryStatus,
    RuntimeReservationRequest, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
    RuntimeResourceDomainConfig, RuntimeRetentionHint,
};

const DECLARATION: &str = include_str!("fixtures/startup_shared_resource_config.json");

struct SavedConfig(std::path::PathBuf);

impl Drop for SavedConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Exercise the same configuration projection/factory that app_setup consumes.
/// Other application fields are ignored by this owned projection; Tauri and
/// inference are intentionally absent from this portable composition suite.
fn compose(json: &str) -> Arc<RuntimeRegistry> {
    let config: RuntimeResourceDomainConfig = serde_json::from_str(json).unwrap();
    Arc::new(config.compose_registry().unwrap())
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

#[test]
fn persisted_config_cold_composition_keeps_declared_capacity_and_canonical_ids() {
    // Each read/factory starts with fresh registry state. This exercises the
    // persisted resource projection, not desktop IPC or a native app process.
    let decoded: RuntimeResourceDomainConfig = serde_json::from_str(DECLARATION).unwrap();
    let mut saved: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
    saved["runtime_resource_domains"] =
        serde_json::to_value(&decoded.runtime_resource_domains).unwrap();
    let persisted = serde_json::to_string(&saved).unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let file = SavedConfig(std::env::temp_dir().join(format!(
        "pantograph-resource-config-{}-{nonce}.json",
        std::process::id()
    )));
    std::fs::write(&file.0, persisted).unwrap();
    for _ in 0..2 {
        let registry = compose(&std::fs::read_to_string(&file.0).unwrap());
        let initial = registry.snapshot();
        assert!(initial.reservations.is_empty());
        assert_eq!(initial.runtimes.len(), 2);
        assert!(initial
            .runtimes
            .iter()
            .all(|runtime| runtime.status == RuntimeRegistryStatus::Stopped
                && runtime.backend_keys.is_empty()));
        registry
            .acquire_reservation(request("pytorch", "a", 80))
            .unwrap();
        assert!(matches!(
            registry.acquire_reservation(request("candle", "b", 80)),
            Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
                available_bytes: 20,
                requested_bytes: 80,
                ..
            })
        ));
    }
}

#[test]
fn configured_startup_competition_uses_atomic_commits_across_runtimes() {
    let registry = compose(DECLARATION);
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

#[test]
fn configured_provisional_replacement_retains_rollback_capacity_until_transfer() {
    let registry = compose(DECLARATION);
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
        registry
            .evaluate_reservation(request("candle", "probe", 0))
            .unwrap()
            .observation()
            .resource_domains[0]
            .reserved_bytes,
        60
    );
    assert_eq!(
        registry.snapshot().reservations[0].reservation_id,
        original.reservation_id
    );
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

#[test]
fn composed_failed_replacement_preserves_predecessor_and_selected_commit_rechecks_capacity() {
    let registry = compose(DECLARATION);
    registry
        .acquire_reservation(request("pytorch", "a", 30))
        .unwrap();
    let replacement = registry
        .evaluate_reservation(request("pytorch", "a", 70))
        .unwrap();
    registry
        .acquire_reservation(request("candle", "b", 50))
        .unwrap();
    assert!(matches!(
        replacement.commit(),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected { .. })
    ));
    assert_eq!(
        registry
            .evaluate_reservation(request("candle", "probe", 0))
            .unwrap()
            .observation()
            .resource_domains[0]
            .reserved_bytes,
        80
    );
}

#[test]
fn producer_reconciliation_keeps_shared_constraints_without_inventing_readiness() {
    let registry = compose(DECLARATION);
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "pytorch".into(),
        display_name: "Actual producer".into(),
        backend_keys: vec!["torch".into()],
        runtime_instance_id: Some("actual.1".into()),
        status: RuntimeRegistryStatus::Ready,
        model_id: None,
        last_error: None,
    }]);
    registry
        .acquire_reservation(request("pytorch", "a", 80))
        .unwrap();
    assert!(registry
        .acquire_reservation(request("candle", "b", 21))
        .is_err());
    let observation = registry
        .evaluate_reservation(request("candle", "probe", 0))
        .unwrap();
    assert_eq!(
        observation.observation().resource_domains[0].reserved_bytes,
        80
    );
}

#[test]
fn unified_memory_configuration_charges_both_kinds_once_in_the_same_pool() {
    let mut value: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
    value["runtime_resource_domains"][0]["bindings"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"runtime_id":"pytorch", "resource_kind":"vram_bytes"}));
    let registry = compose(&value.to_string());
    let mut a = request("pytorch", "a", 40);
    a.requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(40));
    registry.acquire_reservation(a).unwrap();
    let observation = registry
        .evaluate_reservation(request("candle", "probe", 0))
        .unwrap();
    assert_eq!(observation.observation().resource_domains.len(), 1);
    assert_eq!(
        observation.observation().resource_domains[0].reserved_bytes,
        80
    );
    assert_eq!(
        observation.observation().resource_domains[0].available_bytes,
        20
    );
}

#[test]
fn legacy_or_empty_config_has_no_shared_domains_and_does_not_seed_runtimes() {
    for json in [
        r#"{"models":{},"connection_mode":{"type":"None"}}"#,
        r#"{"runtime_resource_domains":[]}"#,
    ] {
        let registry = compose(json);
        assert!(registry.snapshot().runtimes.is_empty());
        assert!(registry.snapshot().reservations.is_empty());
    }
}

#[test]
fn invalid_declarations_fail_composition_instead_of_becoming_an_unconfigured_registry() {
    for alter in [
        "duplicate-domain",
        "overlap",
        "unknown-runtime",
        "duplicate-alias",
        "margin",
    ] {
        let mut value: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
        match alter {
            "duplicate-domain" => {
                let duplicate = value["runtime_resource_domains"][0].clone();
                value["runtime_resource_domains"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            "overlap" => {
                let mut duplicate = value["runtime_resource_domains"][0].clone();
                duplicate["domain_id"] = serde_json::json!("other");
                value["runtime_resource_domains"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            "unknown-runtime" => {
                value["runtime_resource_domains"][0]["bindings"][0]["runtime_id"] =
                    serde_json::json!("imagined-runtime")
            }
            "duplicate-alias" => {
                value["runtime_resource_domains"][0]["bindings"][1]["runtime_id"] =
                    serde_json::json!("PyTorch")
            }
            "margin" => {
                value["runtime_resource_domains"][0]["safety_margin_bytes"] = serde_json::json!(101)
            }
            _ => unreachable!(),
        }
        let config: RuntimeResourceDomainConfig = serde_json::from_value(value).unwrap();
        assert!(config.compose_registry().is_err(), "{alter}");
    }
    for field in ["resource_kind", "binding_device"] {
        let mut value: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
        value["runtime_resource_domains"][0]["bindings"][0][field] = serde_json::json!("made-up");
        assert!(serde_json::from_value::<RuntimeResourceDomainConfig>(value).is_err());
    }
    assert!(serde_json::from_str::<RuntimeResourceDomainConfig>(
        r#"{"runtime_resource_domains":null}"#
    )
    .is_err());
}

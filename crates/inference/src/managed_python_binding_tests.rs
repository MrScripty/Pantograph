//! Independently authored negative-start contract tests. No Python calls.
use crate::managed_python_binding::{
    ManagedPythonBindingRefusal as Refusal, ManagedPythonRuntimeStartRequest as Request,
    ManagedPythonStartObservation as Observation, MANAGED_PYTHON_START_CONTRACT_VERSION,
    TOKENIZER_PROVIDER_BUILD_BINDING_SHA256, TOKENIZER_PROVIDER_EXTENSION_SHA256,
    TOKENIZER_PROVIDER_WHEEL_SHA256,
};
use crate::{CapabilityAvailabilityId, RuntimeVariantId};
use serde_json::{json, Value};
use std::cell::Cell;

fn request() -> Request {
    Request {
        contract_version: 1,
        environment_id: CapabilityAvailabilityId::parse("pumas.torch.cpu.reviewed").unwrap(),
        runtime_revision: "immutable-local-2026-10-09".into(),
        runtime_manifest_sha256: "0123456789abcdef".repeat(4),
        runtime_variant_id: RuntimeVariantId::parse("pytorch.cpu").unwrap(),
        provider_wheel_sha256: "678b155145bb06c271ad6d8eb2df95a8a8173155323fafd4b102e50349940b95"
            .into(),
        provider_extension_sha256:
            "5a7eacfad202bcacf122716f5a45e9796d098c8e65f86a56536efddceb573423".into(),
        provider_build_binding_sha256:
            "b2a4fc2a43b6ae5631b2f5ec45deb5394cd2059e456f97213a87d1bc5d1f703a".into(),
    }
}

fn assert_invalid_before_query(value: &Request) {
    let calls = Cell::new(0);
    let refusal = value.start_refusal(|| {
        calls.set(calls.get() + 1);
        panic!("invalid metadata must refuse before querying interpreter state")
    });
    assert_eq!(refusal, Refusal::InvalidStartRequest);
    assert_eq!(calls.get(), 0);
    assert_eq!(value.validate(), Err(Refusal::InvalidStartRequest));
}

#[test]
fn managed_binding_exact_reviewed_artifact_identities_remain_pinned() {
    let value = request();
    assert_eq!(MANAGED_PYTHON_START_CONTRACT_VERSION, 1);
    assert_eq!(value.provider_wheel_sha256, TOKENIZER_PROVIDER_WHEEL_SHA256);
    assert_eq!(
        value.provider_extension_sha256,
        TOKENIZER_PROVIDER_EXTENSION_SHA256
    );
    assert_eq!(
        value.provider_build_binding_sha256,
        TOKENIZER_PROVIDER_BUILD_BINDING_SHA256
    );
    assert_eq!(value.validate(), Ok(()));
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serde_json::from_value::<Request>(serialized).unwrap(),
        value
    );
}

#[test]
fn managed_binding_bad_versions_and_runtime_variants_never_query_state() {
    for version in [0, 2, u32::MAX] {
        let mut value = request();
        value.contract_version = version;
        assert_invalid_before_query(&value);
    }
    for variant in ["pytorch.cuda", "pytorch.mps", "candle.cpu"] {
        let mut value = request();
        value.runtime_variant_id = RuntimeVariantId::parse(variant).unwrap();
        assert_invalid_before_query(&value);
    }
}

#[test]
fn managed_binding_artifact_or_source_association_changes_never_query_state() {
    let baseline = serde_json::to_value(request()).unwrap();
    for field in [
        "provider_wheel_sha256",
        "provider_extension_sha256",
        "provider_build_binding_sha256",
    ] {
        for replacement in [
            String::new(),
            "0".repeat(64),
            "f".repeat(64),
            "A".repeat(64),
            "0".repeat(65),
        ] {
            let mut encoded = baseline.clone();
            encoded[field] = Value::String(replacement);
            assert_invalid_before_query(&serde_json::from_value(encoded).unwrap());
        }
    }
}

#[test]
fn managed_binding_manifest_digest_has_exact_lowercase_hex_shape() {
    for digest in [
        String::new(),
        "0".repeat(63),
        "0".repeat(65),
        "G".repeat(64),
        "A".repeat(64),
        "é".repeat(32),
        format!("{}\n", "0".repeat(64)),
    ] {
        let mut value = request();
        value.runtime_manifest_sha256 = digest;
        assert_invalid_before_query(&value);
    }
}

#[test]
fn managed_binding_revision_limits_exclude_paths_urls_and_whitespace() {
    for revision in [
        String::new(),
        "x".repeat(257),
        " local".into(),
        "local ".into(),
        "/tmp/runtime".into(),
        "../runtime".into(),
        "https://owner/runtime".into(),
        "rev\n1".into(),
        "é".into(),
    ] {
        let mut value = request();
        value.runtime_revision = revision;
        assert_invalid_before_query(&value);
    }
    for revision in ["a".to_owned(), "x".repeat(256), "rev_1.2+local-3".into()] {
        let mut value = request();
        value.runtime_revision = revision;
        assert_eq!(value.validate(), Ok(()));
        assert_eq!(
            value.start_refusal(|| Observation {
                legacy_worker_initialized: false,
            }),
            Refusal::RegisteredOwnerCustodyUnavailable
        );
    }
}

#[test]
fn managed_binding_either_legacy_worker_flag_remains_closed() {
    for legacy_worker_initialized in [false, true] {
        let value = request();
        let before = value.clone();
        let calls = Cell::new(0);
        let refusal = value.start_refusal(|| {
            calls.set(calls.get() + 1);
            Observation {
                legacy_worker_initialized,
            }
        });
        let expected = if legacy_worker_initialized {
            Refusal::LegacyWorkerAlreadyInitialized
        } else {
            Refusal::RegisteredOwnerCustodyUnavailable
        };
        assert_eq!(refusal, expected);
        assert_eq!(calls.get(), 1);
        assert_eq!(value, before);
    }
}

#[test]
fn managed_binding_plausible_revisions_manifests_and_environments_are_not_custody() {
    for environment in ["pumas.torch.cpu.old", "pumas.torch.cpu.new"] {
        for revision in ["old-revision", "current-revision"] {
            for manifest in ["0".repeat(64), "f".repeat(64)] {
                let mut value = request();
                value.environment_id = CapabilityAvailabilityId::parse(environment).unwrap();
                value.runtime_revision = revision.into();
                value.runtime_manifest_sha256 = manifest;
                assert_eq!(value.validate(), Ok(()));
                assert_eq!(
                    value.start_refusal(|| Observation {
                        legacy_worker_initialized: false,
                    }),
                    Refusal::RegisteredOwnerCustodyUnavailable
                );
            }
        }
    }
}

#[test]
fn managed_binding_serialized_unknown_fields_cannot_forge_lease_or_import_proof() {
    let baseline = serde_json::to_value(request()).unwrap();
    for (field, fake) in [
        (
            "owner_lease",
            json!({"valid": true, "environment_id": "pumas.torch.cpu.reviewed"}),
        ),
        ("registered_import", json!(true)),
        ("python_initialized", json!(true)),
        ("custody_verified", json!(true)),
        ("provider_source_sha256", json!("f".repeat(64))),
    ] {
        let mut encoded = baseline.clone();
        encoded[field] = fake;
        assert!(
            serde_json::from_value::<Request>(encoded).is_err(),
            "{field}"
        );
    }
    for field in [
        "environment_id",
        "runtime_variant_id",
        "runtime_revision",
        "runtime_manifest_sha256",
        "provider_build_binding_sha256",
    ] {
        let mut encoded = baseline.clone();
        encoded.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<Request>(encoded).is_err(),
            "missing {field}"
        );
    }
    for bad_id in ["", "../escape", "OwnerUppercase", "bad id"] {
        let mut encoded = baseline.clone();
        encoded["environment_id"] = json!(bad_id);
        assert!(serde_json::from_value::<Request>(encoded).is_err());
    }
}

#[test]
fn managed_binding_refusal_enum_is_exhaustively_negative() {
    fn explain(refusal: Refusal) -> &'static str {
        match refusal {
            Refusal::InvalidStartRequest => "invalid",
            Refusal::LegacyWorkerAlreadyInitialized => "legacy",
            Refusal::RegisteredOwnerCustodyUnavailable => "custody unavailable",
        }
    }
    for refusal in [
        Refusal::InvalidStartRequest,
        Refusal::LegacyWorkerAlreadyInitialized,
        Refusal::RegisteredOwnerCustodyUnavailable,
    ] {
        assert!(!explain(refusal).is_empty());
        assert!(!refusal.to_string().is_empty());
    }
}

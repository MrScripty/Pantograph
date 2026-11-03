use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactPayloadKind, ArtifactPolicy, ArtifactStore, ArtifactStoreError,
    ArtifactStreamChunkWriteRequest, ArtifactStreamOpenRequest, ArtifactWriteRequest,
};

fn policy(max_disk_bytes: Option<u64>) -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "artifact-global-default".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes,
        max_memory_bytes: Some(64 * 1024),
        max_single_artifact_bytes: Some(128 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

fn attribution() -> ArtifactAttribution {
    ArtifactAttribution {
        workflow_run_id: "run_1".to_string(),
        workflow_id: Some("workflow_1".to_string()),
        workflow_version_id: Some("version_1".to_string()),
        node_id: Some("node_1".to_string()),
        port_id: Some("output".to_string()),
        model_id: None,
        runtime_id: None,
    }
}

#[test]
fn artifact_store_enforces_global_disk_budget_for_retained_bodies() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(Some(10))).expect("open store");

    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("disk_a".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"123456".to_vec(),
        })
        .expect("write first body");

    let too_large = store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("disk_b".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"78901".to_vec(),
        })
        .expect_err("disk limit");
    assert!(matches!(
        too_large,
        ArtifactStoreError::DiskLimitExceeded {
            actual_bytes: 11,
            max_bytes: 10
        }
    ));
    assert!(matches!(
        store.descriptor("disk_b"),
        Err(ArtifactStoreError::NotFound { .. })
    ));
}

#[test]
fn artifact_store_enforces_global_disk_budget_for_stream_growth() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(Some(8))).expect("open store");

    store
        .open_stream(ArtifactStreamOpenRequest {
            artifact_id: Some("disk_stream".to_string()),
            payload_kind: ArtifactPayloadKind::Video,
            media_type: "video/ivf".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
        })
        .expect("open stream");
    store
        .append_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "disk_stream".to_string(),
            sequence: 0,
            body: b"12345".to_vec(),
        })
        .expect("append first chunk");

    let too_large = store
        .append_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "disk_stream".to_string(),
            sequence: 1,
            body: b"6789".to_vec(),
        })
        .expect_err("disk limit");
    assert!(matches!(
        too_large,
        ArtifactStoreError::DiskLimitExceeded {
            actual_bytes: 9,
            max_bytes: 8
        }
    ));
    assert_eq!(store.stats().expect("stats").streaming_body_bytes, 5);
}

#[test]
fn creation_defaults_do_not_replace_a_persisted_policy() {
    let temp = tempfile::tempdir().expect("temp dir");
    let initial = policy(Some(10));
    let store = ArtifactStore::open_with_default_policy(temp.path(), initial.clone())
        .expect("create store with defaults");
    assert_eq!(store.policy(), &initial);
    drop(store);

    let changed_defaults = policy(Some(20));
    for _ in 0..2 {
        let reopened =
            ArtifactStore::open_with_default_policy(temp.path(), changed_defaults.clone())
                .expect("reopen store with different creation defaults");
        assert_eq!(reopened.policy(), &initial);
    }
}

#[test]
fn explicit_policy_override_remains_persisted_on_subsequent_default_open() {
    let temp = tempfile::tempdir().expect("temp dir");
    let initial = policy(Some(10));
    let store = ArtifactStore::open_with_default_policy(temp.path(), initial.clone())
        .expect("create store");
    drop(store);

    let explicit = ArtifactPolicy {
        policy_id: "host-override".to_string(),
        policy_version: 3,
        ttl_seconds: Some(30),
        max_disk_bytes: Some(12),
        max_memory_bytes: Some(4),
        max_single_artifact_bytes: Some(8),
        spill_threshold_bytes: Some(2),
        delete_on_consume: true,
    };
    let overridden = ArtifactStore::open(temp.path(), explicit.clone()).expect("explicit override");
    assert_eq!(overridden.policy(), &explicit);
    drop(overridden);

    let reopened = ArtifactStore::open_with_default_policy(temp.path(), initial)
        .expect("reopen overridden store");
    assert_eq!(reopened.policy(), &explicit);
}

#[test]
fn creation_defaults_never_replace_an_invalid_manifest() {
    let temp = tempfile::tempdir().expect("temp dir");
    let manifest = temp.path().join("manifest.json");
    let invalid = b"{incomplete manifest";
    std::fs::write(&manifest, invalid).expect("write invalid manifest");

    let error = ArtifactStore::open_with_default_policy(temp.path(), policy(None))
        .expect_err("invalid persisted state must reject reopen");
    assert!(matches!(error, ArtifactStoreError::Manifest(_)));
    assert_eq!(std::fs::read(manifest).expect("read manifest"), invalid);
}

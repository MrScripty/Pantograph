use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactBodyTransport, ArtifactConsumeAcknowledgementRequest,
    ArtifactFormatMetadata, ArtifactLifecycleState, ArtifactPayloadKind, ArtifactPolicy,
    ArtifactReadRequest, ArtifactStore, ArtifactStoreError, ArtifactStreamChunkWriteRequest,
    ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest, ArtifactStreamReadRequest,
    ArtifactWriteRequest, IoArtifactRetentionState, WorkflowErrorCode, WorkflowService,
};

fn policy(delete_on_consume: bool) -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "artifact-global-default".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(64 * 1024),
        max_single_artifact_bytes: Some(128 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume,
    }
}

fn attribution() -> ArtifactAttribution {
    ArtifactAttribution {
        workflow_run_id: "run_1".to_string(),
        workflow_id: Some("workflow_1".to_string()),
        workflow_version_id: Some("version_1".to_string()),
        node_id: Some("node_1".to_string()),
        port_id: Some("image".to_string()),
        model_id: None,
        runtime_id: None,
    }
}

fn image_format() -> ArtifactFormatMetadata {
    ArtifactFormatMetadata {
        format_id: "jpg".to_string(),
        media_type: "image/jpeg".to_string(),
        codec_id: None,
        quality_percent: Some(75),
        bitrate_kbps: None,
        crf: None,
        bit_depth: Some("8bit".to_string()),
        color_profile_id: Some("srgb".to_string()),
        converter_id: Some("oiiotool".to_string()),
        converter_version: Some("2.5.0".to_string()),
        library_version: Some("openimageio-2.5.0".to_string()),
        conversion_id: None,
        conversion_status: None,
        conversion_command_id: None,
        conversion_dependencies: Vec::new(),
    }
}

#[test]
fn artifact_store_writes_descriptor_and_reads_body_without_path_leak() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");

    let descriptor = store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_1".to_string()),
            payload_kind: ArtifactPayloadKind::Image,
            media_type: "image/jpeg".to_string(),
            format: Some(image_format()),
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"image-body".to_vec(),
        })
        .expect("write artifact");

    assert_eq!(descriptor.artifact_id, "artifact_1");
    assert_eq!(descriptor.byte_length, Some(10));
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Retained);
    assert_eq!(
        descriptor.retention_state,
        IoArtifactRetentionState::Retained
    );
    assert_eq!(
        descriptor.read_handle.as_deref(),
        Some("artifact-read://artifact_1")
    );
    assert!(!serde_json::to_string(&descriptor)
        .expect("serialize descriptor")
        .contains(temp.path().to_string_lossy().as_ref()));

    let read = store
        .read_body(ArtifactReadRequest {
            artifact_id: "artifact_1".to_string(),
            byte_range_start: Some(0),
            byte_range_end_exclusive: Some(5),
        })
        .expect("read artifact body");
    assert_eq!(read.body, b"image".to_vec());
    assert_eq!(read.response.media_type, "image/jpeg");
    assert_eq!(
        read.response.body_transport,
        ArtifactBodyTransport::BinaryBody
    );
    assert_eq!(read.response.byte_length, 5);
    assert!(!read.response.complete);
}

#[test]
fn artifact_store_clamps_preview_range_to_available_body() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");

    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_preview".to_string()),
            payload_kind: ArtifactPayloadKind::Text,
            media_type: "text/plain".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"short preview".to_vec(),
        })
        .expect("write artifact");

    let read = store
        .read_body(ArtifactReadRequest {
            artifact_id: "artifact_preview".to_string(),
            byte_range_start: Some(0),
            byte_range_end_exclusive: Some(64 * 1024),
        })
        .expect("read clamped preview range");

    assert_eq!(read.body, b"short preview".to_vec());
    assert_eq!(read.response.byte_length, 13);
    assert!(read.response.complete);

    assert!(matches!(
        store.read_body(ArtifactReadRequest {
            artifact_id: "artifact_preview".to_string(),
            byte_range_start: Some(64 * 1024),
            byte_range_end_exclusive: Some(64 * 1024 + 1),
        }),
        Err(ArtifactStoreError::InvalidByteRange)
    ));
}

#[test]
fn artifact_store_recovers_manifest_and_missing_bodies_as_metadata_only() {
    let temp = tempfile::tempdir().expect("temp dir");
    {
        let mut store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");
        store
            .write_artifact(ArtifactWriteRequest {
                artifact_id: Some("artifact_2".to_string()),
                payload_kind: ArtifactPayloadKind::Audio,
                media_type: "audio/ogg".to_string(),
                format: None,
                attribution: attribution(),
                artifact_role: None,
                parent_artifact_id: None,
                revision_index: None,
                body: b"audio-body".to_vec(),
            })
            .expect("write artifact");
    }

    let recovered = ArtifactStore::open(temp.path(), policy(false)).expect("reopen store");
    assert_eq!(
        recovered
            .descriptor("artifact_2")
            .expect("descriptor")
            .retention_state,
        IoArtifactRetentionState::Retained
    );

    std::fs::remove_file(temp.path().join("bodies").join("artifact_2.bin")).expect("remove body");
    let reconciled = ArtifactStore::open(temp.path(), policy(false)).expect("reopen missing body");
    let descriptor = reconciled.descriptor("artifact_2").expect("descriptor");
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Failed);
    assert_eq!(
        descriptor.retention_state,
        IoArtifactRetentionState::MetadataOnly
    );
    assert!(descriptor.read_handle.is_none());
    assert!(matches!(
        reconciled.read_body(ArtifactReadRequest {
            artifact_id: "artifact_2".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        }),
        Err(ArtifactStoreError::BodyUnavailable { .. })
    ));
}

#[test]
fn artifact_store_enforces_size_limit_and_valid_ids() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut limited_policy = policy(false);
    limited_policy.max_single_artifact_bytes = Some(4);
    let mut store = ArtifactStore::open(temp.path(), limited_policy).expect("open store");

    let too_large = store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_3".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"12345".to_vec(),
        })
        .expect_err("size limit");
    assert!(matches!(
        too_large,
        ArtifactStoreError::ArtifactTooLarge {
            actual_bytes: 5,
            max_bytes: 4
        }
    ));

    let invalid_id = store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("../escape".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"123".to_vec(),
        })
        .expect_err("invalid id");
    assert!(matches!(invalid_id, ArtifactStoreError::InvalidArtifactId));
}

#[test]
fn artifact_store_consume_and_retention_delete_body_but_keep_metadata() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(true)).expect("open store");
    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_4".to_string()),
            payload_kind: ArtifactPayloadKind::Audio,
            media_type: "audio/ogg".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"audio-body".to_vec(),
        })
        .expect("write artifact");

    let consumed = store
        .acknowledge_consume(ArtifactConsumeAcknowledgementRequest {
            artifact_id: "artifact_4".to_string(),
            consumer_id: "client_1".to_string(),
        })
        .expect("consume");
    assert!(!consumed.retained_after_consume);
    assert_eq!(
        store
            .descriptor("artifact_4")
            .expect("descriptor")
            .retention_state,
        IoArtifactRetentionState::Deleted
    );
    assert_eq!(store.stats().expect("stats").metadata_only_count, 1);

    let mut retain_policy = policy(false);
    retain_policy.ttl_seconds = Some(1);
    store.update_policy(retain_policy).expect("update policy");
    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_5".to_string()),
            payload_kind: ArtifactPayloadKind::Audio,
            media_type: "audio/ogg".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"audio-body".to_vec(),
        })
        .expect("write second artifact");

    assert_eq!(store.apply_retention_cleanup(u64::MAX).expect("cleanup"), 1);
    assert_eq!(
        store
            .descriptor("artifact_5")
            .expect("descriptor")
            .retention_state,
        IoArtifactRetentionState::Deleted
    );
}

#[test]
fn artifact_store_retention_deletion_preserves_audit_descriptor_and_fails_body_reads() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(true)).expect("open store");
    let original = store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("retention_audit".to_string()),
            payload_kind: ArtifactPayloadKind::Image,
            media_type: "image/jpeg".to_string(),
            format: Some(image_format()),
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"image-body".to_vec(),
        })
        .expect("write artifact");

    store
        .acknowledge_consume(ArtifactConsumeAcknowledgementRequest {
            artifact_id: "retention_audit".to_string(),
            consumer_id: "client_1".to_string(),
        })
        .expect("consume and delete body");

    assert!(!temp
        .path()
        .join("bodies")
        .join("retention_audit.bin")
        .exists());
    let descriptor = store.descriptor("retention_audit").expect("descriptor");
    assert_eq!(descriptor.artifact_id, original.artifact_id);
    assert_eq!(descriptor.payload_kind, original.payload_kind);
    assert_eq!(descriptor.byte_length, original.byte_length);
    assert_eq!(descriptor.content_hash, original.content_hash);
    assert_eq!(descriptor.format, original.format);
    assert_eq!(descriptor.attribution, original.attribution);
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Deleted);
    assert_eq!(
        descriptor.retention_state,
        IoArtifactRetentionState::Deleted
    );
    assert!(descriptor.access_modes.is_empty());
    assert!(descriptor.read_handle.is_none());
    assert_eq!(
        descriptor.retention_reason.as_deref(),
        Some("body_deleted_by_policy")
    );
    assert_eq!(store.stats().expect("stats").metadata_only_count, 1);
    assert!(matches!(
        store.read_body(ArtifactReadRequest {
            artifact_id: "retention_audit".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        }),
        Err(ArtifactStoreError::BodyUnavailable { artifact_id }) if artifact_id == "retention_audit"
    ));

    let reopened = ArtifactStore::open(temp.path(), policy(true)).expect("reopen store");
    let reopened_descriptor = reopened
        .descriptor("retention_audit")
        .expect("reopened descriptor");
    assert_eq!(reopened_descriptor, descriptor);
    assert!(matches!(
        reopened.read_body(ArtifactReadRequest {
            artifact_id: "retention_audit".to_string(),
            byte_range_start: Some(0),
            byte_range_end_exclusive: Some(1),
        }),
        Err(ArtifactStoreError::BodyUnavailable { artifact_id }) if artifact_id == "retention_audit"
    ));
}

#[test]
fn workflow_service_retention_cleanup_keeps_descriptor_queryable_while_body_is_unavailable() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut cleanup_policy = policy(false);
    cleanup_policy.ttl_seconds = Some(1);
    let store = ArtifactStore::open(temp.path(), cleanup_policy).expect("open store");
    let service = WorkflowService::new().with_artifact_store(store);
    let original = service
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("service_retention_audit".to_string()),
            payload_kind: ArtifactPayloadKind::Audio,
            media_type: "audio/ogg".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"audio-body".to_vec(),
        })
        .expect("write through service");

    assert_eq!(
        service
            .apply_artifact_store_retention_cleanup(u64::MAX)
            .expect("cleanup through service"),
        1
    );

    let descriptor = service
        .artifact_descriptor(
            pantograph_workflow_service::ArtifactDescriptorQueryRequest {
                artifact_id: "service_retention_audit".to_string(),
            },
        )
        .expect("descriptor through service")
        .artifact
        .expect("artifact exists");
    assert_eq!(descriptor.artifact_id, original.artifact_id);
    assert_eq!(descriptor.payload_kind, original.payload_kind);
    assert_eq!(descriptor.byte_length, original.byte_length);
    assert_eq!(descriptor.content_hash, original.content_hash);
    assert_eq!(descriptor.attribution, original.attribution);
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Deleted);
    assert_eq!(
        descriptor.retention_state,
        IoArtifactRetentionState::Deleted
    );
    assert!(descriptor.access_modes.is_empty());
    assert!(descriptor.read_handle.is_none());
    let error = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: "service_retention_audit".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect_err("retention cleanup should remove artifact body");
    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(
        error
            .message()
            .contains("artifact body is unavailable: service_retention_audit"),
        "unexpected artifact body error: {}",
        error.message()
    );
}

#[test]
fn workflow_service_artifact_store_facade_uses_configured_store() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");
    let service = WorkflowService::new().with_artifact_store(store);

    service
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("artifact_6".to_string()),
            payload_kind: ArtifactPayloadKind::Image,
            media_type: "image/jpeg".to_string(),
            format: Some(image_format()),
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"image-body".to_vec(),
        })
        .expect("write through service");

    let descriptor = service
        .artifact_descriptor(
            pantograph_workflow_service::ArtifactDescriptorQueryRequest {
                artifact_id: "artifact_6".to_string(),
            },
        )
        .expect("descriptor through service")
        .artifact
        .expect("artifact exists");
    assert_eq!(descriptor.artifact_id, "artifact_6");

    let body = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: "artifact_6".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read through service");
    assert_eq!(body.body, b"image-body".to_vec());
    assert_eq!(
        service
            .artifact_store_stats()
            .expect("stats through service")
            .retained_body_count,
        1
    );
}

#[test]
fn artifact_store_streams_chunks_and_finalizes_descriptor_without_serialized_bodies() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");

    let streaming = store
        .open_stream(ArtifactStreamOpenRequest {
            artifact_id: Some("artifact_stream_1".to_string()),
            payload_kind: ArtifactPayloadKind::Video,
            media_type: "video/ivf".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
        })
        .expect("open stream");

    assert_eq!(streaming.lifecycle_state, ArtifactLifecycleState::Streaming);
    assert_eq!(
        streaming.stream_handle.as_deref(),
        Some("artifact-stream://artifact_stream_1")
    );
    assert!(streaming.read_handle.is_none());

    let first_chunk = store
        .append_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "artifact_stream_1".to_string(),
            sequence: 0,
            body: b"chunk-one".to_vec(),
        })
        .expect("append first chunk");
    let second_chunk = store
        .append_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "artifact_stream_1".to_string(),
            sequence: 1,
            body: b"-chunk-two".to_vec(),
        })
        .expect("append second chunk");

    assert_eq!(first_chunk.byte_length, 9);
    assert_eq!(second_chunk.sequence, 1);
    assert!(first_chunk.content_hash.is_some());
    assert!(!serde_json::to_string(&first_chunk)
        .expect("serialize chunk metadata")
        .contains("chunk-one"));
    assert_eq!(store.stats().expect("stats").streaming_body_bytes, 19);
    let stream_read = store
        .read_stream_body(ArtifactStreamReadRequest {
            artifact_id: "artifact_stream_1".to_string(),
            byte_range_start: Some(9),
            byte_range_end_exclusive: Some(19),
        })
        .expect("read currently available stream bytes");
    assert_eq!(stream_read.body, b"-chunk-two".to_vec());
    assert_eq!(stream_read.response.byte_length, 10);
    assert_eq!(stream_read.response.available_byte_length, 19);
    assert_eq!(
        stream_read.response.stream_handle,
        "artifact-stream://artifact_stream_1"
    );
    assert_eq!(
        stream_read.response.lifecycle_state,
        ArtifactLifecycleState::Streaming
    );
    assert!(!serde_json::to_string(&stream_read.response)
        .expect("serialize stream read metadata")
        .contains("chunk-two"));
    assert!(matches!(
        store.read_body(ArtifactReadRequest {
            artifact_id: "artifact_stream_1".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        }),
        Err(ArtifactStoreError::BodyUnavailable { .. })
    ));

    let manifest = std::fs::read_to_string(temp.path().join("manifest.json")).expect("manifest");
    assert!(!manifest.contains("chunk-one"));
    assert!(!manifest.contains("chunk-two"));

    let finalized = store
        .finalize_stream(ArtifactStreamFinalizeRequest {
            artifact_id: "artifact_stream_1".to_string(),
        })
        .expect("finalize stream");
    assert_eq!(finalized.lifecycle_state, ArtifactLifecycleState::Retained);
    assert_eq!(finalized.byte_length, Some(19));
    assert!(finalized.stream_handle.is_none());
    assert_eq!(
        finalized.read_handle.as_deref(),
        Some("artifact-read://artifact_stream_1")
    );

    let read = store
        .read_body(ArtifactReadRequest {
            artifact_id: "artifact_stream_1".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read finalized stream");
    assert_eq!(read.body, b"chunk-one-chunk-two".to_vec());
    assert!(!serde_json::to_string(&finalized)
        .expect("serialize descriptor")
        .contains("chunk-one"));
    assert!(!serde_json::to_string(&read.response)
        .expect("serialize read metadata")
        .contains("chunk-one"));
    assert_eq!(store.stats().expect("stats").streaming_body_count, 0);
    assert_eq!(store.stats().expect("stats").retained_body_count, 1);
}

#[test]
fn artifact_store_preserves_child_revision_relationship_metadata() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");

    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("diffusion_final".to_string()),
            payload_kind: ArtifactPayloadKind::Image,
            media_type: "image/jpeg".to_string(),
            format: Some(image_format()),
            attribution: attribution(),
            artifact_role: Some("final".to_string()),
            parent_artifact_id: None,
            revision_index: None,
            body: b"final-image".to_vec(),
        })
        .expect("write parent artifact");

    let preview = store
        .open_stream(ArtifactStreamOpenRequest {
            artifact_id: Some("diffusion_preview_3".to_string()),
            payload_kind: ArtifactPayloadKind::Image,
            media_type: "image/jpeg".to_string(),
            format: Some(image_format()),
            attribution: attribution(),
            artifact_role: Some("preview_revision".to_string()),
            parent_artifact_id: Some("diffusion_final".to_string()),
            revision_index: Some(3),
        })
        .expect("open preview stream");

    assert_eq!(preview.artifact_role.as_deref(), Some("preview_revision"));
    assert_eq!(
        preview.parent_artifact_id.as_deref(),
        Some("diffusion_final")
    );
    assert_eq!(preview.revision_index, Some(3));

    store
        .append_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "diffusion_preview_3".to_string(),
            sequence: 0,
            body: b"preview".to_vec(),
        })
        .expect("append preview");
    let finalized = store
        .finalize_stream(ArtifactStreamFinalizeRequest {
            artifact_id: "diffusion_preview_3".to_string(),
        })
        .expect("finalize preview");

    assert_eq!(
        finalized.parent_artifact_id.as_deref(),
        Some("diffusion_final")
    );
    assert_eq!(finalized.revision_index, Some(3));
    assert_eq!(finalized.artifact_role.as_deref(), Some("preview_revision"));
}

#[test]
fn artifact_store_accounts_for_memory_cache_and_enforces_budget() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut cache_policy = policy(false);
    cache_policy.spill_threshold_bytes = Some(8);
    cache_policy.max_memory_bytes = Some(10);
    let mut store = ArtifactStore::open(temp.path(), cache_policy).expect("open store");

    for (artifact_id, body) in [
        ("cache_a", b"1234".to_vec()),
        ("cache_b", b"567890".to_vec()),
        ("cache_c", b"not-cached-large".to_vec()),
    ] {
        store
            .write_artifact(ArtifactWriteRequest {
                artifact_id: Some(artifact_id.to_string()),
                payload_kind: ArtifactPayloadKind::GenericBinary,
                media_type: "application/octet-stream".to_string(),
                format: None,
                attribution: attribution(),
                artifact_role: None,
                parent_artifact_id: None,
                revision_index: None,
                body,
            })
            .expect("write artifact");
    }

    let stats = store.stats().expect("stats");
    assert_eq!(stats.retained_body_count, 3);
    assert_eq!(stats.memory_cache_body_count, 2);
    assert_eq!(stats.memory_cache_body_bytes, 10);

    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("cache_d".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"xx".to_vec(),
        })
        .expect("write uncached small artifact when budget is full");
    let stats = store.stats().expect("stats");
    assert_eq!(stats.retained_body_count, 4);
    assert_eq!(stats.memory_cache_body_count, 2);
    assert_eq!(stats.memory_cache_body_bytes, 10);

    let reopened = ArtifactStore::open(temp.path(), store.policy().clone()).expect("reopen store");
    let reopened_stats = reopened.stats().expect("reopened stats");
    assert_eq!(reopened_stats.memory_cache_body_count, 2);
    assert_eq!(reopened_stats.memory_cache_body_bytes, 10);
}

#[test]
fn artifact_store_cleanup_evicts_memory_cache_and_read_handle_body() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut cleanup_policy = policy(false);
    cleanup_policy.ttl_seconds = Some(1);
    cleanup_policy.spill_threshold_bytes = Some(32);
    cleanup_policy.max_memory_bytes = Some(32);
    let mut store = ArtifactStore::open(temp.path(), cleanup_policy).expect("open store");

    store
        .write_artifact(ArtifactWriteRequest {
            artifact_id: Some("cleanup_cached".to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: b"cached-body".to_vec(),
        })
        .expect("write artifact");
    assert_eq!(store.stats().expect("stats").memory_cache_body_count, 1);

    assert_eq!(store.apply_retention_cleanup(u64::MAX).expect("cleanup"), 1);
    let stats = store.stats().expect("stats");
    assert_eq!(stats.metadata_only_count, 1);
    assert_eq!(stats.memory_cache_body_count, 0);
    assert!(matches!(
        store.read_body(ArtifactReadRequest {
            artifact_id: "cleanup_cached".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        }),
        Err(ArtifactStoreError::BodyUnavailable { .. })
    ));
    let descriptor = store.descriptor("cleanup_cached").expect("descriptor");
    assert_eq!(
        descriptor.retention_state,
        IoArtifactRetentionState::Deleted
    );
    assert!(descriptor.read_handle.is_none());
}

#[test]
fn workflow_service_stream_facade_finalizes_readable_artifact() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = ArtifactStore::open(temp.path(), policy(false)).expect("open store");
    let service = WorkflowService::new().with_artifact_store(store);

    service
        .open_artifact_stream(ArtifactStreamOpenRequest {
            artifact_id: Some("service_stream".to_string()),
            payload_kind: ArtifactPayloadKind::Audio,
            media_type: "audio/ogg".to_string(),
            format: None,
            attribution: attribution(),
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
        })
        .expect("open stream through service");
    service
        .append_artifact_stream_chunk(ArtifactStreamChunkWriteRequest {
            artifact_id: "service_stream".to_string(),
            sequence: 0,
            body: b"audio".to_vec(),
        })
        .expect("append stream through service");
    let descriptor = service
        .finalize_artifact_stream(ArtifactStreamFinalizeRequest {
            artifact_id: "service_stream".to_string(),
        })
        .expect("finalize stream through service");
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Retained);

    let body = service
        .read_artifact_body(ArtifactReadRequest {
            artifact_id: "service_stream".to_string(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })
        .expect("read through service");
    assert_eq!(body.body, b"audio".to_vec());
}

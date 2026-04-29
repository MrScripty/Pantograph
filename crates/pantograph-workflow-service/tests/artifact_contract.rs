use pantograph_workflow_service::{
    ArtifactAccessMode, ArtifactAttribution, ArtifactBodyTransport,
    ArtifactConsumeAcknowledgementRequest, ArtifactConsumeAcknowledgementResponse,
    ArtifactDescriptor, ArtifactDescriptorQueryRequest, ArtifactDescriptorQueryResponse,
    ArtifactFormatCapabilities, ArtifactFormatMetadata, ArtifactFormatSettings,
    ArtifactLifecycleState, ArtifactPayloadKind, ArtifactPolicy, ArtifactReadRequest,
    ArtifactReadResponse, ArtifactStreamChunkRecord, IoArtifactRetentionState,
    ManagedRedistributableCategory, ManagedRedistributableReadinessState,
    ManagedRedistributableStatus, ManagedRedistributableStatusQueryResponse, MediaFormatOption,
};

#[test]
fn artifact_descriptor_contract_snapshot_uses_references_not_payload_bodies() {
    let artifact = ArtifactDescriptor {
        artifact_id: "artifact-run-1-image-output-image".to_string(),
        payload_kind: ArtifactPayloadKind::Image,
        lifecycle_state: ArtifactLifecycleState::Retained,
        retention_state: IoArtifactRetentionState::Retained,
        byte_length: Some(1_440_866),
        content_hash: Some("blake3:image-hash".to_string()),
        format: Some(ArtifactFormatMetadata {
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
        }),
        attribution: ArtifactAttribution {
            workflow_run_id: "run-1".to_string(),
            workflow_id: Some("juggernaut".to_string()),
            workflow_version_id: Some("wf-version-1".to_string()),
            node_id: Some("image-output".to_string()),
            port_id: Some("image".to_string()),
            model_id: Some("model-1".to_string()),
            runtime_id: Some("torch".to_string()),
        },
        access_modes: vec![ArtifactAccessMode::Read, ArtifactAccessMode::Download],
        read_handle: Some("artifact-read://artifact-run-1-image-output-image".to_string()),
        stream_handle: None,
        retention_reason: None,
    };

    let response = ArtifactDescriptorQueryResponse {
        artifact: Some(artifact),
    };
    let value = serde_json::to_value(response).expect("serialize artifact descriptor");

    assert_eq!(
        value,
        serde_json::json!({
            "artifact": {
                "artifact_id": "artifact-run-1-image-output-image",
                "payload_kind": "image",
                "lifecycle_state": "retained",
                "retention_state": "retained",
                "byte_length": 1440866,
                "content_hash": "blake3:image-hash",
                "format": {
                    "format_id": "jpg",
                    "media_type": "image/jpeg",
                    "quality_percent": 75,
                    "bit_depth": "8bit",
                    "color_profile_id": "srgb",
                    "converter_id": "oiiotool",
                    "converter_version": "2.5.0",
                    "library_version": "openimageio-2.5.0"
                },
                "attribution": {
                    "workflow_run_id": "run-1",
                    "workflow_id": "juggernaut",
                    "workflow_version_id": "wf-version-1",
                    "node_id": "image-output",
                    "port_id": "image",
                    "model_id": "model-1",
                    "runtime_id": "torch"
                },
                "access_modes": ["read", "download"],
                "read_handle": "artifact-read://artifact-run-1-image-output-image"
            }
        })
    );

    assert!(!value.to_string().contains("data:image"));
}

#[test]
fn artifact_access_contracts_are_handle_based() {
    let query = ArtifactDescriptorQueryRequest {
        artifact_id: "artifact-1".to_string(),
    };
    let read = ArtifactReadRequest {
        artifact_id: "artifact-1".to_string(),
        byte_range_start: Some(0),
        byte_range_end_exclusive: Some(1024),
    };
    let response = ArtifactReadResponse {
        artifact_id: "artifact-1".to_string(),
        media_type: "audio/ogg".to_string(),
        body_transport: ArtifactBodyTransport::BinaryBody,
        read_handle: "artifact-read://artifact-1".to_string(),
        byte_length: 4096,
        content_hash: Some("blake3:audio".to_string()),
        complete: false,
    };
    let consume = ArtifactConsumeAcknowledgementRequest {
        artifact_id: "artifact-1".to_string(),
        consumer_id: "client-session-1".to_string(),
    };
    let consume_response = ArtifactConsumeAcknowledgementResponse {
        artifact_id: "artifact-1".to_string(),
        retained_after_consume: true,
    };
    let chunk = ArtifactStreamChunkRecord {
        artifact_id: "artifact-1".to_string(),
        stream_handle: "artifact-stream://artifact-1".to_string(),
        sequence: 7,
        byte_length: 2048,
        lifecycle_state: ArtifactLifecycleState::Streaming,
        content_hash: None,
    };

    assert_eq!(
        serde_json::to_value(query).expect("query"),
        serde_json::json!({"artifact_id": "artifact-1"})
    );
    assert_eq!(
        serde_json::to_value(read).expect("read request"),
        serde_json::json!({
            "artifact_id": "artifact-1",
            "byte_range_start": 0,
            "byte_range_end_exclusive": 1024
        })
    );
    assert_eq!(
        serde_json::to_value(response).expect("read response"),
        serde_json::json!({
            "artifact_id": "artifact-1",
            "media_type": "audio/ogg",
            "body_transport": "binary_body",
            "read_handle": "artifact-read://artifact-1",
            "byte_length": 4096,
            "content_hash": "blake3:audio",
            "complete": false
        })
    );
    assert_eq!(
        serde_json::to_value(consume).expect("consume request"),
        serde_json::json!({
            "artifact_id": "artifact-1",
            "consumer_id": "client-session-1"
        })
    );
    assert_eq!(
        serde_json::to_value(consume_response).expect("consume response"),
        serde_json::json!({
            "artifact_id": "artifact-1",
            "retained_after_consume": true
        })
    );
    assert_eq!(
        serde_json::to_value(chunk).expect("stream chunk"),
        serde_json::json!({
            "artifact_id": "artifact-1",
            "stream_handle": "artifact-stream://artifact-1",
            "sequence": 7,
            "byte_length": 2048,
            "lifecycle_state": "streaming"
        })
    );
}

#[test]
fn artifact_format_defaults_match_stage_11_requirements() {
    let settings = ArtifactFormatSettings::default();

    assert_eq!(
        serde_json::to_value(settings).expect("format settings"),
        serde_json::json!({
            "image": {
                "format_id": "jpg",
                "quality_percent": 75,
                "color_profile_id": "srgb"
            },
            "audio": {
                "container_id": "ogg",
                "codec_id": "opus",
                "bitrate_kbps": 96
            },
            "video": {
                "container_id": "ivf",
                "codec_id": "svt_av1",
                "crf": 32,
                "bit_depth": "8bit"
            },
            "three_d": {
                "format_id": "glb"
            }
        })
    );
}

#[test]
fn media_capabilities_and_managed_redistributables_are_typed() {
    let capabilities = ArtifactFormatCapabilities {
        image_formats: vec![MediaFormatOption {
            format_id: "jpg".to_string(),
            display_name: "JPEG".to_string(),
            media_type: "image/jpeg".to_string(),
            codec_ids: Vec::new(),
            quality_min_percent: Some(1),
            quality_max_percent: Some(100),
            bitrate_min_kbps: None,
            bitrate_max_kbps: None,
            crf_min: None,
            crf_max: None,
            bit_depths: vec!["8bit".to_string()],
            color_profile_ids: vec!["srgb".to_string()],
            provided_by_dependency_id: "oiiotool".to_string(),
            provided_by_version: Some("2.5.0".to_string()),
        }],
        audio_formats: Vec::new(),
        video_formats: Vec::new(),
        three_d_formats: Vec::new(),
    };
    let dependencies = ManagedRedistributableStatusQueryResponse {
        dependencies: vec![
            ManagedRedistributableStatus {
                dependency_id: "ffmpeg".to_string(),
                category: ManagedRedistributableCategory::ToolBinary,
                display_name: "ffmpeg".to_string(),
                selected_version: Some("7.0".to_string()),
                active_version: Some("7.0".to_string()),
                readiness_state: ManagedRedistributableReadinessState::Ready,
                license_id: Some("LGPL-2.1-or-later".to_string()),
                source_owner: "ffmpeg".to_string(),
                platform: "x86_64-unknown-linux-gnu".to_string(),
                expected_files: vec!["ffmpeg".to_string()],
                missing_files: Vec::new(),
                checksum: Some("sha256:ffmpeg".to_string()),
                unavailable_reason: None,
            },
            ManagedRedistributableStatus {
                dependency_id: "opencolorio".to_string(),
                category: ManagedRedistributableCategory::NativeLibraryArtifact,
                display_name: "OpenColorIO".to_string(),
                selected_version: None,
                active_version: None,
                readiness_state: ManagedRedistributableReadinessState::Missing,
                license_id: Some("BSD-3-Clause".to_string()),
                source_owner: "AcademySoftwareFoundation".to_string(),
                platform: "x86_64-unknown-linux-gnu".to_string(),
                expected_files: vec!["libOpenColorIO.so".to_string()],
                missing_files: vec!["libOpenColorIO.so".to_string()],
                checksum: None,
                unavailable_reason: Some("not_installed".to_string()),
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(capabilities).expect("capabilities"),
        serde_json::json!({
            "image_formats": [{
                "format_id": "jpg",
                "display_name": "JPEG",
                "media_type": "image/jpeg",
                "codec_ids": [],
                "quality_min_percent": 1,
                "quality_max_percent": 100,
                "bit_depths": ["8bit"],
                "color_profile_ids": ["srgb"],
                "provided_by_dependency_id": "oiiotool",
                "provided_by_version": "2.5.0"
            }],
            "audio_formats": [],
            "video_formats": [],
            "three_d_formats": []
        })
    );
    assert_eq!(
        serde_json::to_value(dependencies).expect("dependencies"),
        serde_json::json!({
            "dependencies": [
                {
                    "dependency_id": "ffmpeg",
                    "category": "tool_binary",
                    "display_name": "ffmpeg",
                    "selected_version": "7.0",
                    "active_version": "7.0",
                    "readiness_state": "ready",
                    "license_id": "LGPL-2.1-or-later",
                    "source_owner": "ffmpeg",
                    "platform": "x86_64-unknown-linux-gnu",
                    "expected_files": ["ffmpeg"],
                    "missing_files": [],
                    "checksum": "sha256:ffmpeg"
                },
                {
                    "dependency_id": "opencolorio",
                    "category": "native_library_artifact",
                    "display_name": "OpenColorIO",
                    "readiness_state": "missing",
                    "license_id": "BSD-3-Clause",
                    "source_owner": "AcademySoftwareFoundation",
                    "platform": "x86_64-unknown-linux-gnu",
                    "expected_files": ["libOpenColorIO.so"],
                    "missing_files": ["libOpenColorIO.so"],
                    "unavailable_reason": "not_installed"
                }
            ]
        })
    );
}

#[test]
fn artifact_policy_contract_preserves_global_cache_controls() {
    let policy = ArtifactPolicy {
        policy_id: "artifact-global-default".to_string(),
        policy_version: 1,
        ttl_seconds: Some(604_800),
        max_disk_bytes: Some(536_870_912),
        max_memory_bytes: Some(67_108_864),
        max_single_artifact_bytes: Some(134_217_728),
        spill_threshold_bytes: Some(1_048_576),
        delete_on_consume: false,
    };

    assert_eq!(
        serde_json::to_value(policy).expect("artifact policy"),
        serde_json::json!({
            "policy_id": "artifact-global-default",
            "policy_version": 1,
            "ttl_seconds": 604800,
            "max_disk_bytes": 536870912,
            "max_memory_bytes": 67108864,
            "max_single_artifact_bytes": 134217728,
            "spill_threshold_bytes": 1048576,
            "delete_on_consume": false
        })
    );
}

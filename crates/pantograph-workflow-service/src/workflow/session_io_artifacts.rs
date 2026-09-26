use pantograph_diagnostics_ledger::{
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, IoArtifactAccessMode,
    IoArtifactConversionDependency, IoArtifactConversionStatus, IoArtifactFormatMetadata,
    IoArtifactLifecycleState, IoArtifactPayloadKind, IoArtifactRetentionState,
};

use super::{
    ArtifactAccessMode, ArtifactAttribution, ArtifactConversionStatus, ArtifactDescriptor,
    ArtifactFormatMetadata, ArtifactLifecycleState, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowPortBinding, WorkflowService, WorkflowServiceError,
};

const RETAINED_WORKFLOW_IO_VALUE_MAX_BYTES: usize = 64 * 1024;

pub(super) struct WorkflowIoArtifactMetadata {
    pub(super) artifact_fact_id: String,
    pub(super) payload_artifact_id: String,
    pub(super) artifact_id: String,
    pub(super) logical_payload_lineage_id: String,
    pub(super) media_type: Option<String>,
    pub(super) size_bytes: Option<u64>,
    pub(super) content_hash: Option<String>,
    pub(super) payload_ref: Option<String>,
    pub(super) privacy_class: DiagnosticEventPrivacyClass,
    pub(super) retention_class: DiagnosticEventRetentionClass,
    pub(super) retention_state: IoArtifactRetentionState,
    pub(super) retention_reason: Option<String>,
    pub(super) payload_kind: Option<IoArtifactPayloadKind>,
    pub(super) lifecycle_state: Option<IoArtifactLifecycleState>,
    pub(super) access_modes: Vec<IoArtifactAccessMode>,
    pub(super) read_handle: Option<String>,
    pub(super) stream_handle: Option<String>,
    pub(super) format: Option<IoArtifactFormatMetadata>,
}

pub(super) fn workflow_io_artifact_metadata(
    service: &WorkflowService,
    workflow_run_id: &str,
    workflow_id: &str,
    workflow_version_id: &str,
    role_label: &str,
    binding: &WorkflowPortBinding,
    model_id: Option<&str>,
    runtime_id: Option<&str>,
) -> Result<WorkflowIoArtifactMetadata, WorkflowServiceError> {
    let artifact_fact_id = workflow_io_artifact_fact_id(
        workflow_run_id,
        role_label,
        &binding.node_id,
        &binding.port_id,
    );
    let payload_family = workflow_io_payload_family(role_label);
    let payload_artifact_id = workflow_io_payload_artifact_id(
        workflow_run_id,
        payload_family,
        &binding.node_id,
        &binding.port_id,
    );
    let logical_payload_lineage_id = workflow_io_logical_payload_lineage_id(
        workflow_run_id,
        payload_family,
        &binding.node_id,
        &binding.port_id,
    );
    if let Ok(descriptor) = serde_json::from_value::<ArtifactDescriptor>(binding.value.clone()) {
        return Ok(workflow_io_artifact_metadata_from_descriptor(
            descriptor,
            artifact_fact_id,
            logical_payload_lineage_id,
        ));
    }

    let materialized = workflow_io_artifact_body(&binding.value)?;
    let artifact_id = payload_artifact_id.clone();
    // Runtime-attributed outputs are complete inference results. Their size is
    // governed by the artifact store policy, rather than the bounded fallback
    // used for arbitrary oversized workflow values.
    let requires_complete_retention = model_id.is_some() || runtime_id.is_some();
    if materialized.body.len() <= RETAINED_WORKFLOW_IO_VALUE_MAX_BYTES
        || requires_complete_retention
    {
        if let Ok(writer) = service.artifact_writer() {
            let write_result = writer.write_artifact(ArtifactWriteRequest {
                artifact_id: Some(artifact_id),
                payload_kind: materialized.payload_kind,
                media_type: materialized.media_type.clone(),
                format: Some(workflow_io_artifact_format_metadata(
                    &materialized.media_type,
                )),
                attribution: ArtifactAttribution {
                    workflow_run_id: workflow_run_id.to_string(),
                    workflow_id: Some(workflow_id.to_string()),
                    workflow_version_id: Some(workflow_version_id.to_string()),
                    node_id: Some(binding.node_id.clone()),
                    port_id: Some(binding.port_id.clone()),
                    model_id: model_id.map(str::to_string),
                    runtime_id: runtime_id.map(str::to_string),
                },
                artifact_role: Some(role_label.to_string()),
                parent_artifact_id: None,
                revision_index: None,
                body: materialized.body.clone(),
            });
            if let Ok(descriptor) = write_result {
                return Ok(workflow_io_artifact_metadata_from_descriptor(
                    descriptor,
                    artifact_fact_id,
                    logical_payload_lineage_id,
                ));
            }
        }
    }

    Ok(workflow_io_artifact_metadata_only(
        artifact_fact_id,
        payload_artifact_id,
        logical_payload_lineage_id,
        &materialized.body,
        &materialized.media_type,
    ))
}

fn workflow_io_artifact_fact_id(
    workflow_run_id: &str,
    artifact_role: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash = blake3::hash(
        format!("{workflow_run_id}:fact:{artifact_role}:{node_id}:{port_id}").as_bytes(),
    );
    format!("workflow-io-fact-{hash}")
}

fn workflow_io_logical_payload_lineage_id(
    workflow_run_id: &str,
    payload_family: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash = blake3::hash(
        format!("{workflow_run_id}:lineage:{payload_family}:{node_id}:{port_id}").as_bytes(),
    );
    format!("workflow-io-lineage-{hash}")
}

fn workflow_io_payload_artifact_id(
    workflow_run_id: &str,
    payload_family: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash = blake3::hash(
        format!("{workflow_run_id}:payload:{payload_family}:{node_id}:{port_id}").as_bytes(),
    );
    format!("workflow-io-{hash}")
}

fn workflow_io_payload_family(role_label: &str) -> &'static str {
    match role_label {
        "node_output" | "workflow_output" => "output",
        "node_input" | "workflow_input" => "input",
        _ => "unknown",
    }
}

struct WorkflowIoArtifactBody {
    body: Vec<u8>,
    media_type: String,
    payload_kind: ArtifactPayloadKind,
}

fn workflow_io_artifact_body(
    value: &serde_json::Value,
) -> Result<WorkflowIoArtifactBody, WorkflowServiceError> {
    if let Some(text) = value.as_str() {
        return Ok(WorkflowIoArtifactBody {
            body: text.as_bytes().to_vec(),
            media_type: "text/plain".to_string(),
            payload_kind: ArtifactPayloadKind::Text,
        });
    }
    let body = serde_json::to_vec(value).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow I/O artifact metadata: {error}"
        ))
    })?;
    Ok(WorkflowIoArtifactBody {
        body,
        media_type: "application/json".to_string(),
        payload_kind: ArtifactPayloadKind::Structured,
    })
}

fn workflow_io_artifact_metadata_from_descriptor(
    descriptor: ArtifactDescriptor,
    artifact_fact_id: String,
    logical_payload_lineage_id: String,
) -> WorkflowIoArtifactMetadata {
    let payload_artifact_id = descriptor.artifact_id.clone();
    WorkflowIoArtifactMetadata {
        artifact_fact_id,
        payload_artifact_id,
        artifact_id: descriptor.artifact_id.clone(),
        logical_payload_lineage_id,
        media_type: descriptor
            .format
            .as_ref()
            .map(|format| format.media_type.clone()),
        size_bytes: descriptor.byte_length,
        content_hash: descriptor.content_hash.clone(),
        payload_ref: Some(format!("artifact://{}", descriptor.artifact_id)),
        privacy_class: DiagnosticEventPrivacyClass::SensitiveReference,
        retention_class: DiagnosticEventRetentionClass::PayloadReference,
        retention_state: descriptor.retention_state,
        retention_reason: descriptor.retention_reason.clone(),
        payload_kind: Some(io_artifact_payload_kind(descriptor.payload_kind)),
        lifecycle_state: Some(io_artifact_lifecycle_state(descriptor.lifecycle_state)),
        access_modes: descriptor
            .access_modes
            .into_iter()
            .map(io_artifact_access_mode)
            .collect(),
        read_handle: descriptor.read_handle,
        stream_handle: descriptor.stream_handle,
        format: descriptor.format.map(io_artifact_format_metadata),
    }
}

fn workflow_io_artifact_metadata_only(
    artifact_fact_id: String,
    payload_artifact_id: String,
    logical_payload_lineage_id: String,
    body: &[u8],
    media_type: &str,
) -> WorkflowIoArtifactMetadata {
    WorkflowIoArtifactMetadata {
        artifact_fact_id,
        payload_artifact_id: payload_artifact_id.clone(),
        artifact_id: payload_artifact_id,
        logical_payload_lineage_id,
        media_type: Some(media_type.to_string()),
        size_bytes: Some(body.len() as u64),
        content_hash: Some(format!("blake3:{}", blake3::hash(body))),
        payload_ref: None,
        privacy_class: DiagnosticEventPrivacyClass::UserMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        retention_state: IoArtifactRetentionState::MetadataOnly,
        retention_reason: Some(
            "workflow value body is not retained in the I/O artifact ledger".to_string(),
        ),
        payload_kind: None,
        lifecycle_state: None,
        access_modes: Vec::new(),
        read_handle: None,
        stream_handle: None,
        format: None,
    }
}

fn workflow_io_artifact_format_metadata(media_type: &str) -> ArtifactFormatMetadata {
    ArtifactFormatMetadata {
        format_id: match media_type {
            "text/plain" => "plain_text",
            "application/json" => "json",
            _ => "generic",
        }
        .to_string(),
        media_type: media_type.to_string(),
        codec_id: None,
        quality_percent: None,
        bitrate_kbps: None,
        crf: None,
        bit_depth: None,
        color_profile_id: None,
        converter_id: None,
        converter_version: None,
        library_version: None,
        conversion_id: None,
        conversion_status: None,
        conversion_command_id: None,
        conversion_dependencies: Vec::new(),
    }
}

fn io_artifact_payload_kind(kind: ArtifactPayloadKind) -> IoArtifactPayloadKind {
    match kind {
        ArtifactPayloadKind::Text => IoArtifactPayloadKind::Text,
        ArtifactPayloadKind::Image => IoArtifactPayloadKind::Image,
        ArtifactPayloadKind::Audio => IoArtifactPayloadKind::Audio,
        ArtifactPayloadKind::Video => IoArtifactPayloadKind::Video,
        ArtifactPayloadKind::ThreeD => IoArtifactPayloadKind::ThreeD,
        ArtifactPayloadKind::LargeTable => IoArtifactPayloadKind::LargeTable,
        ArtifactPayloadKind::GenericBinary => IoArtifactPayloadKind::GenericBinary,
        ArtifactPayloadKind::Structured => IoArtifactPayloadKind::Structured,
    }
}

fn io_artifact_lifecycle_state(state: ArtifactLifecycleState) -> IoArtifactLifecycleState {
    match state {
        ArtifactLifecycleState::Declared => IoArtifactLifecycleState::Declared,
        ArtifactLifecycleState::Writing => IoArtifactLifecycleState::Writing,
        ArtifactLifecycleState::Streaming => IoArtifactLifecycleState::Streaming,
        ArtifactLifecycleState::Finalizing => IoArtifactLifecycleState::Finalizing,
        ArtifactLifecycleState::Retained => IoArtifactLifecycleState::Retained,
        ArtifactLifecycleState::Failed => IoArtifactLifecycleState::Failed,
        ArtifactLifecycleState::Expired => IoArtifactLifecycleState::Expired,
        ArtifactLifecycleState::Deleted => IoArtifactLifecycleState::Deleted,
    }
}

fn io_artifact_access_mode(mode: ArtifactAccessMode) -> IoArtifactAccessMode {
    match mode {
        ArtifactAccessMode::Read => IoArtifactAccessMode::Read,
        ArtifactAccessMode::Download => IoArtifactAccessMode::Download,
        ArtifactAccessMode::Stream => IoArtifactAccessMode::Stream,
    }
}

fn io_artifact_format_metadata(format: ArtifactFormatMetadata) -> IoArtifactFormatMetadata {
    IoArtifactFormatMetadata {
        format_id: format.format_id,
        media_type: format.media_type,
        codec_id: format.codec_id,
        quality_percent: format.quality_percent,
        bitrate_kbps: format.bitrate_kbps,
        crf: format.crf,
        bit_depth: format.bit_depth,
        color_profile_id: format.color_profile_id,
        converter_id: format.converter_id,
        converter_version: format.converter_version,
        library_version: format.library_version,
        conversion_id: format.conversion_id,
        conversion_status: format.conversion_status.map(io_artifact_conversion_status),
        conversion_command_id: format.conversion_command_id,
        conversion_dependencies: format
            .conversion_dependencies
            .into_iter()
            .map(io_artifact_conversion_dependency)
            .collect(),
    }
}

fn io_artifact_conversion_status(status: ArtifactConversionStatus) -> IoArtifactConversionStatus {
    match status {
        ArtifactConversionStatus::Converted => IoArtifactConversionStatus::Converted,
        ArtifactConversionStatus::PassedThrough => IoArtifactConversionStatus::PassedThrough,
        ArtifactConversionStatus::Failed => IoArtifactConversionStatus::Failed,
    }
}

fn io_artifact_conversion_dependency(
    dependency: super::ArtifactConversionDependency,
) -> IoArtifactConversionDependency {
    IoArtifactConversionDependency {
        dependency_id: dependency.dependency_id,
        active_version: dependency.active_version,
        lease_id: dependency.lease_id,
        lease_holder: dependency.lease_holder,
    }
}

#[cfg(test)]
mod tests {
    use pantograph_diagnostics_ledger::IoArtifactRetentionState;
    use tempfile::TempDir;

    use super::workflow_io_artifact_metadata;
    use crate::workflow::{
        ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowPortBinding, WorkflowService,
    };

    #[test]
    fn generated_large_structured_output_is_retained_with_model_attribution() {
        let root = TempDir::new().expect("artifact store root");
        let store = ArtifactStore::open(
            root.path(),
            ArtifactPolicy {
                policy_id: "generated-output-test".to_string(),
                policy_version: 1,
                ttl_seconds: None,
                max_disk_bytes: Some(1024 * 1024),
                max_memory_bytes: Some(1024 * 1024),
                max_single_artifact_bytes: Some(1024 * 1024),
                spill_threshold_bytes: Some(1024),
                delete_on_consume: false,
            },
        )
        .expect("open artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let embedding = serde_json::Value::Array(
            (0..4096)
                .map(|_| serde_json::json!(0.12345678901234567))
                .collect(),
        );
        let binding = WorkflowPortBinding {
            node_id: "embedding-node".to_string(),
            port_id: "embedding".to_string(),
            value: embedding.clone(),
        };

        let metadata = workflow_io_artifact_metadata(
            &service,
            "run-generated-large-output",
            "workflow-generated-output",
            "version-1",
            "node_output",
            &binding,
            Some("model-qwen-embedding"),
            Some("runtime-llama-cpp"),
        )
        .expect("retain generated output");
        let expected_body = serde_json::to_vec(&embedding).expect("encode embedding");
        assert!(expected_body.len() > super::RETAINED_WORKFLOW_IO_VALUE_MAX_BYTES);
        assert_eq!(metadata.retention_state, IoArtifactRetentionState::Retained);
        assert!(metadata.payload_ref.is_some());
        assert!(metadata.read_handle.is_some());

        let descriptor = service
            .artifact_descriptor(crate::workflow::ArtifactDescriptorQueryRequest {
                artifact_id: metadata.artifact_id.clone(),
            })
            .expect("query retained descriptor")
            .artifact
            .expect("retained descriptor");
        assert_eq!(
            descriptor.attribution.model_id.as_deref(),
            Some("model-qwen-embedding")
        );
        assert_eq!(
            descriptor.attribution.runtime_id.as_deref(),
            Some("runtime-llama-cpp")
        );

        let retained = service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: metadata.artifact_id,
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read complete generated output");
        assert!(retained.response.complete);
        assert_eq!(retained.body, expected_body);
    }
}

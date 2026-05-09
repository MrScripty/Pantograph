use pantograph_diagnostics_ledger::{
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, IoArtifactAccessMode,
    IoArtifactConversionDependency, IoArtifactConversionStatus, IoArtifactFormatMetadata,
    IoArtifactLifecycleState, IoArtifactPayloadKind, IoArtifactRetentionState,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_workflow_service::{
    ArtifactAccessMode, ArtifactAttribution, ArtifactConversionStatus, ArtifactDescriptor,
    ArtifactFormatMetadata, ArtifactLifecycleState, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowService,
};

const RETAINED_NODE_IO_VALUE_MAX_BYTES: usize = 64 * 1024;

pub(crate) struct NodeIoArtifactMetadata {
    pub(crate) artifact_fact_id: String,
    pub(crate) payload_artifact_id: String,
    pub(crate) artifact_id: String,
    pub(crate) logical_payload_lineage_id: String,
    pub(crate) media_type: Option<String>,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) content_hash: Option<String>,
    pub(crate) payload_ref: Option<String>,
    pub(crate) privacy_class: DiagnosticEventPrivacyClass,
    pub(crate) retention_class: DiagnosticEventRetentionClass,
    pub(crate) retention_state: IoArtifactRetentionState,
    pub(crate) retention_reason: Option<String>,
    pub(crate) payload_kind: Option<IoArtifactPayloadKind>,
    pub(crate) lifecycle_state: Option<IoArtifactLifecycleState>,
    pub(crate) access_modes: Vec<IoArtifactAccessMode>,
    pub(crate) read_handle: Option<String>,
    pub(crate) format: Option<IoArtifactFormatMetadata>,
}

pub(crate) fn node_output_artifact_metadata(
    service: &WorkflowService,
    workflow_run_id: &WorkflowRunId,
    workflow_id: &WorkflowId,
    node_id: &str,
    port_id: &str,
    value: &serde_json::Value,
) -> NodeIoArtifactMetadata {
    node_io_artifact_metadata(
        service,
        workflow_run_id,
        workflow_id,
        "node_output",
        node_id,
        port_id,
        value,
    )
}

pub(crate) fn node_input_artifact_metadata(
    service: &WorkflowService,
    workflow_run_id: &WorkflowRunId,
    workflow_id: &WorkflowId,
    node_id: &str,
    port_id: &str,
    value: &serde_json::Value,
) -> NodeIoArtifactMetadata {
    node_io_artifact_metadata(
        service,
        workflow_run_id,
        workflow_id,
        "node_input",
        node_id,
        port_id,
        value,
    )
}

fn node_io_artifact_metadata(
    service: &WorkflowService,
    workflow_run_id: &WorkflowRunId,
    workflow_id: &WorkflowId,
    role_label: &str,
    node_id: &str,
    port_id: &str,
    value: &serde_json::Value,
) -> NodeIoArtifactMetadata {
    let artifact_fact_id =
        node_io_artifact_fact_id(workflow_run_id.as_str(), role_label, node_id, port_id);
    let logical_payload_lineage_id =
        node_io_logical_payload_lineage_id(workflow_run_id.as_str(), node_id, port_id);
    if let Ok(descriptor) = serde_json::from_value::<ArtifactDescriptor>(value.clone()) {
        return node_io_artifact_metadata_from_descriptor(
            descriptor,
            artifact_fact_id,
            logical_payload_lineage_id,
        );
    }

    let materialized = node_io_artifact_body(value);
    let artifact_id = node_io_artifact_id(workflow_run_id.as_str(), role_label, node_id, port_id);
    if materialized.body.len() <= RETAINED_NODE_IO_VALUE_MAX_BYTES {
        let descriptor = service.write_artifact(ArtifactWriteRequest {
            artifact_id: Some(artifact_id.clone()),
            payload_kind: materialized.payload_kind,
            media_type: materialized.media_type.clone(),
            format: Some(node_io_artifact_format_metadata(&materialized.media_type)),
            attribution: ArtifactAttribution {
                workflow_run_id: workflow_run_id.as_str().to_string(),
                workflow_id: Some(workflow_id.as_str().to_string()),
                workflow_version_id: None,
                node_id: Some(node_id.to_string()),
                port_id: Some(port_id.to_string()),
                model_id: None,
                runtime_id: None,
            },
            artifact_role: Some(role_label.to_string()),
            parent_artifact_id: None,
            revision_index: None,
            body: materialized.body.clone(),
        });
        if let Ok(descriptor) = descriptor {
            return node_io_artifact_metadata_from_descriptor(
                descriptor,
                artifact_fact_id,
                logical_payload_lineage_id,
            );
        }
    }

    node_io_artifact_metadata_only(
        workflow_run_id.as_str(),
        role_label,
        node_id,
        port_id,
        artifact_fact_id,
        logical_payload_lineage_id,
        &materialized.body,
        &materialized.media_type,
    )
}

struct NodeIoArtifactBody {
    body: Vec<u8>,
    media_type: String,
    payload_kind: ArtifactPayloadKind,
}

fn node_io_artifact_id(
    workflow_run_id: &str,
    artifact_role: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash =
        blake3::hash(format!("{workflow_run_id}:{artifact_role}:{node_id}:{port_id}").as_bytes());
    format!("workflow-io-{hash}")
}

fn node_io_artifact_fact_id(
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

fn node_io_logical_payload_lineage_id(
    workflow_run_id: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash = blake3::hash(format!("{workflow_run_id}:lineage:{node_id}:{port_id}").as_bytes());
    format!("workflow-io-lineage-{hash}")
}

fn node_io_artifact_body(value: &serde_json::Value) -> NodeIoArtifactBody {
    if let Some(text) = value.as_str() {
        return NodeIoArtifactBody {
            body: text.as_bytes().to_vec(),
            media_type: "text/plain".to_string(),
            payload_kind: ArtifactPayloadKind::Text,
        };
    }

    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"null".to_vec());
    NodeIoArtifactBody {
        body,
        media_type: "application/json".to_string(),
        payload_kind: ArtifactPayloadKind::Structured,
    }
}

fn node_io_artifact_metadata_from_descriptor(
    descriptor: ArtifactDescriptor,
    artifact_fact_id: String,
    logical_payload_lineage_id: String,
) -> NodeIoArtifactMetadata {
    let payload_artifact_id = descriptor.artifact_id.clone();
    NodeIoArtifactMetadata {
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
        format: descriptor.format.map(io_artifact_format_metadata),
    }
}

fn node_io_artifact_metadata_only(
    workflow_run_id: &str,
    role_label: &str,
    node_id: &str,
    port_id: &str,
    artifact_fact_id: String,
    logical_payload_lineage_id: String,
    body: &[u8],
    media_type: &str,
) -> NodeIoArtifactMetadata {
    let payload_artifact_id = node_io_artifact_id(workflow_run_id, role_label, node_id, port_id);
    NodeIoArtifactMetadata {
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
        format: None,
    }
}

fn node_io_artifact_format_metadata(media_type: &str) -> ArtifactFormatMetadata {
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
        conversion_status: format.conversion_status.map(|status| match status {
            ArtifactConversionStatus::Converted => IoArtifactConversionStatus::Converted,
            ArtifactConversionStatus::PassedThrough => IoArtifactConversionStatus::PassedThrough,
            ArtifactConversionStatus::Failed => IoArtifactConversionStatus::Failed,
        }),
        conversion_command_id: format.conversion_command_id,
        conversion_dependencies: format
            .conversion_dependencies
            .into_iter()
            .map(|dependency| IoArtifactConversionDependency {
                dependency_id: dependency.dependency_id,
                active_version: dependency.active_version,
                lease_id: dependency.lease_id,
                lease_holder: dependency.lease_holder,
            })
            .collect(),
    }
}

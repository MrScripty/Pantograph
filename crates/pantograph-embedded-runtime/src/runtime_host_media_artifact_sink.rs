use inference::EncodedImage;
use pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef;
use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowArtifactWriter, WorkflowServiceError,
};
use thiserror::Error;

const RUNTIME_HOST_IMAGE_ARTIFACT_ROLE: &str = "runtime_host_image_output";

pub(crate) struct RuntimeHostImageArtifactWriteRequest<'a> {
    pub(crate) workflow_run_id: &'a str,
    pub(crate) workflow_id: &'a str,
    pub(crate) node_id: &'a str,
    pub(crate) task_id: &'a str,
    pub(crate) port_id: &'a str,
    pub(crate) image_index: usize,
    pub(crate) image: &'a EncodedImage,
    pub(crate) model_id: Option<&'a str>,
    pub(crate) runtime_id: Option<&'a str>,
}

pub(crate) trait RuntimeHostMediaArtifactSink: Send + Sync {
    fn write_image_output(
        &self,
        request: RuntimeHostImageArtifactWriteRequest<'_>,
    ) -> Result<RuntimeHostExecutionMediaArtifactRef, RuntimeHostMediaArtifactSinkError>;
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum RuntimeHostMediaArtifactSinkError {
    #[error("runtime-host image output base64 decode failed for {task_id}.{port_id}[{image_index}]: {message}")]
    InvalidImagePayload {
        task_id: String,
        port_id: String,
        image_index: usize,
        message: String,
    },
    #[error("runtime-host image output artifact write failed for {task_id}.{port_id}[{image_index}]: {source}")]
    ArtifactWriteFailed {
        task_id: String,
        port_id: String,
        image_index: usize,
        #[source]
        source: WorkflowServiceError,
    },
}

#[derive(Clone)]
pub(crate) struct WorkflowServiceRuntimeHostMediaArtifactSink {
    artifact_writer: WorkflowArtifactWriter,
}

impl WorkflowServiceRuntimeHostMediaArtifactSink {
    #[must_use]
    pub(crate) fn new(artifact_writer: WorkflowArtifactWriter) -> Self {
        Self { artifact_writer }
    }
}

impl RuntimeHostMediaArtifactSink for WorkflowServiceRuntimeHostMediaArtifactSink {
    fn write_image_output(
        &self,
        request: RuntimeHostImageArtifactWriteRequest<'_>,
    ) -> Result<RuntimeHostExecutionMediaArtifactRef, RuntimeHostMediaArtifactSinkError> {
        let body =
            crate::media_base64::decode_base64(&request.image.data_base64).map_err(|message| {
                RuntimeHostMediaArtifactSinkError::InvalidImagePayload {
                    task_id: request.task_id.to_string(),
                    port_id: request.port_id.to_string(),
                    image_index: request.image_index,
                    message,
                }
            })?;
        let artifact_id = runtime_host_image_artifact_id(
            request.workflow_run_id,
            request.task_id,
            request.node_id,
            request.port_id,
            request.image_index,
        );
        let descriptor = self
            .artifact_writer
            .write_artifact(ArtifactWriteRequest {
                artifact_id: Some(artifact_id.clone()),
                payload_kind: ArtifactPayloadKind::Image,
                media_type: request.image.mime_type.clone(),
                format: Some(image_artifact_format_metadata(&request.image.mime_type)),
                attribution: ArtifactAttribution {
                    workflow_run_id: request.workflow_run_id.to_string(),
                    workflow_id: Some(request.workflow_id.to_string()),
                    workflow_version_id: None,
                    node_id: Some(request.node_id.to_string()),
                    port_id: Some(request.port_id.to_string()),
                    model_id: request.model_id.map(str::to_string),
                    runtime_id: request.runtime_id.map(str::to_string),
                },
                artifact_role: Some(RUNTIME_HOST_IMAGE_ARTIFACT_ROLE.to_string()),
                parent_artifact_id: None,
                revision_index: Some(request.image_index as u64),
                body,
            })
            .map_err(
                |source| RuntimeHostMediaArtifactSinkError::ArtifactWriteFailed {
                    task_id: request.task_id.to_string(),
                    port_id: request.port_id.to_string(),
                    image_index: request.image_index,
                    source,
                },
            )?;

        Ok(RuntimeHostExecutionMediaArtifactRef {
            artifact_id: descriptor.artifact_id,
            media_type: Some(runtime_host_media_type_id(&request.image.mime_type)),
        })
    }
}

fn runtime_host_image_artifact_id(
    workflow_run_id: &str,
    task_id: &str,
    node_id: &str,
    port_id: &str,
    image_index: usize,
) -> String {
    let hash = blake3::hash(
        format!(
            "{workflow_run_id}:runtime-host-image-output:{task_id}:{node_id}:{port_id}:{image_index}"
        )
        .as_bytes(),
    );
    format!("runtime-host-image-{hash}")
}

fn image_artifact_format_metadata(media_type: &str) -> ArtifactFormatMetadata {
    ArtifactFormatMetadata {
        format_id: image_format_id(media_type).to_string(),
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

fn image_format_id(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpeg",
        "image/webp" => "webp",
        _ => "image",
    }
}

fn runtime_host_media_type_id(media_type: &str) -> String {
    media_type
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_workflow_service::{
        ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowErrorCode, WorkflowService,
    };
    use tempfile::TempDir;

    #[test]
    fn shared_artifact_writer_sink_writes_image_output_as_path_free_artifact_ref() {
        let temp = TempDir::new().expect("temp artifact dir");
        let writer = artifact_writer(&temp);
        let service = WorkflowService::new().with_artifact_writer(writer.clone());
        let sink = WorkflowServiceRuntimeHostMediaArtifactSink::new(writer);
        let image = EncodedImage {
            data_base64: "aGVsbG8=".to_string(),
            mime_type: "image/png".to_string(),
            width: Some(16),
            height: Some(16),
        };

        let artifact_ref = sink
            .write_image_output(RuntimeHostImageArtifactWriteRequest {
                workflow_run_id: "run.2026-05-31.001",
                workflow_id: "workflow.image",
                node_id: "node.image_generation",
                task_id: "task.image_generation.001",
                port_id: "image",
                image_index: 0,
                image: &image,
                model_id: Some("model.stable-diffusion"),
                runtime_id: Some("runtime.pytorch"),
            })
            .expect("image output should write to artifact store");

        assert!(artifact_ref.artifact_id.starts_with("runtime-host-image-"));
        assert_eq!(artifact_ref.media_type.as_deref(), Some("image_png"));
        let body = service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: artifact_ref.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("artifact body should be retained");
        assert_eq!(body.body, b"hello");
        assert_eq!(body.response.media_type, "image/png");
        let descriptor = service
            .artifact_descriptor(
                pantograph_workflow_service::ArtifactDescriptorQueryRequest {
                    artifact_id: artifact_ref.artifact_id,
                },
            )
            .expect("artifact descriptor should be queryable")
            .artifact
            .expect("artifact descriptor should exist");
        assert_eq!(
            descriptor.artifact_role.as_deref(),
            Some(RUNTIME_HOST_IMAGE_ARTIFACT_ROLE)
        );
        assert_eq!(descriptor.revision_index, Some(0));
        assert_eq!(descriptor.attribution.workflow_run_id, "run.2026-05-31.001");
        assert_eq!(
            descriptor.attribution.model_id.as_deref(),
            Some("model.stable-diffusion")
        );
        assert_eq!(
            descriptor.attribution.runtime_id.as_deref(),
            Some("runtime.pytorch")
        );
        assert_eq!(
            descriptor
                .format
                .as_ref()
                .map(|format| format.format_id.as_str()),
            Some("png")
        );
    }

    #[test]
    fn workflow_service_sink_rejects_invalid_image_payload() {
        let temp = TempDir::new().expect("temp artifact dir");
        let sink = WorkflowServiceRuntimeHostMediaArtifactSink::new(artifact_writer(&temp));
        let image = EncodedImage {
            data_base64: "****".to_string(),
            mime_type: "image/png".to_string(),
            width: None,
            height: None,
        };

        let error = sink
            .write_image_output(RuntimeHostImageArtifactWriteRequest {
                workflow_run_id: "run.2026-05-31.001",
                workflow_id: "workflow.image",
                node_id: "node.image_generation",
                task_id: "task.image_generation.001",
                port_id: "image",
                image_index: 0,
                image: &image,
                model_id: None,
                runtime_id: None,
            })
            .expect_err("invalid base64 must fail closed");

        assert!(matches!(
            error,
            RuntimeHostMediaArtifactSinkError::InvalidImagePayload { .. }
        ));
        assert!(error
            .to_string()
            .contains("task.image_generation.001.image[0]"));
    }

    #[test]
    fn shared_artifact_writer_sink_reports_artifact_write_failure() {
        let temp = TempDir::new().expect("temp artifact dir");
        let writer = artifact_writer(&temp);
        let service = WorkflowService::new().with_artifact_writer(writer.clone());
        let sink = WorkflowServiceRuntimeHostMediaArtifactSink::new(writer);
        drop(service);
        drop(temp);
        let image = EncodedImage {
            data_base64: "aGVsbG8=".to_string(),
            mime_type: "image/png".to_string(),
            width: None,
            height: None,
        };

        let error = sink
            .write_image_output(RuntimeHostImageArtifactWriteRequest {
                workflow_run_id: "run.2026-05-31.001",
                workflow_id: "workflow.image",
                node_id: "node.image_generation",
                task_id: "task.image_generation.001",
                port_id: "image",
                image_index: 0,
                image: &image,
                model_id: None,
                runtime_id: None,
            })
            .expect_err("missing artifact store must fail closed");

        let RuntimeHostMediaArtifactSinkError::ArtifactWriteFailed { source, .. } = error else {
            panic!("expected artifact write failure");
        };
        assert_eq!(source.code(), WorkflowErrorCode::InternalError);
        assert!(source.to_string().contains("artifact store io error"));
    }

    fn artifact_writer(temp: &TempDir) -> WorkflowArtifactWriter {
        let artifact_store = ArtifactStore::open(temp.path().join("artifacts"), artifact_policy())
            .expect("open artifact store");
        WorkflowArtifactWriter::new(artifact_store)
    }

    fn artifact_policy() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "runtime-host-media-sink-test".to_string(),
            policy_version: 1,
            ttl_seconds: None,
            max_disk_bytes: None,
            max_memory_bytes: None,
            max_single_artifact_bytes: None,
            spill_threshold_bytes: None,
            delete_on_consume: false,
        }
    }
}

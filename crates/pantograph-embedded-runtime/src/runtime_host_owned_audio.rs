//! Path-free owned recording ingress and verified snapshot resolution.
use pantograph_workflow_service::workflow::WorkflowSchedulerTaskMediaArtifactRef;
use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowArtifactWriter,
};

#[cfg(test)]
type SnapshotReadBarrier = std::sync::Arc<dyn Fn(&std::sync::Arc<[u8]>) + Send + Sync>;

#[derive(Clone)]
pub struct OwnedAudioInputStore {
    writer: WorkflowArtifactWriter,
    #[cfg(test)]
    snapshot_read_barrier: Option<SnapshotReadBarrier>,
    #[cfg(test)]
    before_snapshot_admission: Option<std::sync::Arc<tokio::sync::Notify>>,
}
impl OwnedAudioInputStore {
    pub fn new(writer: WorkflowArtifactWriter) -> Self {
        Self {
            writer,
            #[cfg(test)]
            snapshot_read_barrier: None,
            #[cfg(test)]
            before_snapshot_admission: None,
        }
    }
    /// Host-owned byte ingress, no path/URL and no implicit policy enlargement.
    pub fn import_wav(
        &self,
        workflow_id: &str,
        source_run_id: &str,
        body: Vec<u8>,
    ) -> Result<WorkflowSchedulerTaskMediaArtifactRef, String> {
        if workflow_id.trim().is_empty() || source_run_id.trim().is_empty() {
            return Err("owned WAV requires workflow and source-run attribution".into());
        }
        inference::validate_owned_wav(&body).map_err(|e| e.to_string())?;
        let hash = format!("blake3:{}", blake3::hash(&body).to_hex());
        let id = inference::owned_audio_id(workflow_id, source_run_id, &hash);
        let format: ArtifactFormatMetadata = serde_json::from_value(
            serde_json::json!({"format_id":"wav","media_type":"audio/wav","codec_id":"pcm_s16le"}),
        )
        .map_err(|e| e.to_string())?;
        let attribution: ArtifactAttribution = serde_json::from_value(
            serde_json::json!({"workflow_id":workflow_id,"workflow_run_id":source_run_id}),
        )
        .map_err(|e| e.to_string())?;
        self.writer
            .write_artifact(ArtifactWriteRequest {
                artifact_id: Some(id.clone()),
                payload_kind: ArtifactPayloadKind::Audio,
                media_type: "audio/wav".into(),
                format: Some(format),
                attribution,
                artifact_role: Some("owned_audio_input".into()),
                parent_artifact_id: None,
                revision_index: None,
                body,
            })
            .map_err(|e| e.to_string())?;
        Ok(WorkflowSchedulerTaskMediaArtifactRef {
            artifact_id: id,
            media_type: Some("audio_wav".into()),
        })
    }
    #[cfg(test)]
    pub(crate) async fn resolve(
        &self,
        artifact_id: String,
        workflow_id: String,
    ) -> Result<inference::OwnedAudioWav, String> {
        self.resolve_for_execution(artifact_id, workflow_id, None)
            .await
    }
    pub(crate) async fn resolve_for_execution(
        &self,
        artifact_id: String,
        workflow_id: String,
        cancellation: Option<
            pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle,
        >,
    ) -> Result<inference::OwnedAudioWav, String> {
        #[cfg(test)]
        if let Some(entered) = &self.before_snapshot_admission {
            entered.notify_one();
        }
        let permit = inference::acquire_owned_audio_admission()
            .await
            .map_err(|e| e.to_string())?;
        if cancellation.as_ref().is_some_and(|c| c.snapshot().state != pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::Running) {
            return Err("owned audio cancelled before snapshot read".into());
        }
        let writer = self.writer.clone();
        #[cfg(test)]
        let read_barrier = self.snapshot_read_barrier.clone();
        // Detached blocking read retains its admission even if the caller disappears.
        tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            let snapshot = if let Some(barrier) = read_barrier {
                writer.verified_snapshot_with_read_barrier(
                    &artifact_id,
                    inference::OWNED_AUDIO_MAX_BYTES,
                    |body| barrier(body),
                )
            } else {
                writer.verified_snapshot(&artifact_id, inference::OWNED_AUDIO_MAX_BYTES)
            };
            #[cfg(not(test))]
            let snapshot = writer.verified_snapshot(&artifact_id, inference::OWNED_AUDIO_MAX_BYTES);
            let snapshot = snapshot.map_err(|e| e.to_string())?;
            let d = snapshot.descriptor;
            if d.payload_kind != ArtifactPayloadKind::Audio
                || d.artifact_role.as_deref() != Some("owned_audio_input")
                || d.attribution.workflow_id.as_deref() != Some(workflow_id.as_str())
                || d.format.as_ref().is_none_or(|f| {
                    f.media_type != "audio/wav"
                        || f.format_id != "wav"
                        || f.codec_id.as_deref() != Some("pcm_s16le")
                })
            {
                return Err("owned WAV attribution/format mismatch".into());
            }
            inference::OwnedAudioWav::verified(
                artifact_id,
                workflow_id,
                d.attribution.workflow_run_id,
                d.content_hash.ok_or("owned WAV hash required")?,
                snapshot.body,
                permit,
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
#[cfg(test)]
#[path = "runtime_host_owned_audio_tests.rs"]
pub(crate) mod tests;

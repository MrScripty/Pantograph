//! Immutable host-verified WAV custody; never hydrated from a serialized request.
use crate::BackendError;
use std::sync::Arc;
pub const OWNED_AUDIO_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const OWNED_AUDIO_MAX_SECONDS: u32 = 300;
static SNAPSHOTS: once_cell::sync::Lazy<Arc<tokio::sync::Semaphore>> =
    once_cell::sync::Lazy::new(|| Arc::new(tokio::sync::Semaphore::new(2)));
pub struct OwnedAudioAdmission(tokio::sync::OwnedSemaphorePermit);
pub async fn acquire_owned_audio_admission() -> Result<OwnedAudioAdmission, BackendError> {
    SNAPSHOTS
        .clone()
        .acquire_owned()
        .await
        .map(OwnedAudioAdmission)
        .map_err(|_| BackendError::Config("owned audio admission closed".into()))
}
#[derive(Clone, serde::Serialize)]
pub struct OwnedAudioWav {
    artifact_id: String,
    workflow_id: String,
    source_run_id: String,
    content_hash: String,
    sample_rate: u32,
    channels: u16,
    frames: usize,
    #[serde(skip)]
    bytes: Arc<[u8]>,
    #[serde(skip)]
    _permit: Arc<tokio::sync::OwnedSemaphorePermit>,
}
impl std::fmt::Debug for OwnedAudioWav {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedAudioWav")
            .field("byte_length", &self.bytes.len())
            .field("sample_rate", &self.sample_rate)
            .field("frames", &self.frames)
            .finish_non_exhaustive()
    }
}
impl PartialEq for OwnedAudioWav {
    fn eq(&self, other: &Self) -> bool {
        self.artifact_id == other.artifact_id
            && self.workflow_id == other.workflow_id
            && self.source_run_id == other.source_run_id
            && self.content_hash == other.content_hash
            && self.bytes == other.bytes
    }
}
impl OwnedAudioWav {
    pub fn verified(
        artifact_id: String,
        workflow_id: String,
        source_run_id: String,
        content_hash: String,
        bytes: Arc<[u8]>,
        permit: OwnedAudioAdmission,
    ) -> Result<Self, BackendError> {
        let (sample_rate, channels, frames) = crate::selected_audio_execution::validate_wav_bytes(
            &bytes,
            OWNED_AUDIO_MAX_BYTES,
            OWNED_AUDIO_MAX_SECONDS,
            None,
        )?;
        if workflow_id.trim().is_empty()
            || source_run_id.trim().is_empty()
            || content_hash != format!("blake3:{}", blake3::hash(&bytes).to_hex())
            || artifact_id != owned_audio_id(&workflow_id, &source_run_id, &content_hash)
        {
            return Err(BackendError::Config(
                "owned audio content identity mismatch".into(),
            ));
        }
        Ok(Self {
            artifact_id,
            workflow_id,
            source_run_id,
            content_hash,
            sample_rate,
            channels,
            frames,
            bytes,
            _permit: Arc::new(permit.0),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }
    pub fn source_run_id(&self) -> &str {
        &self.source_run_id
    }
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
    pub fn duration_seconds(&self) -> f32 {
        self.frames as f32 / self.sample_rate as f32
    }
    pub fn validate_request(
        &self,
        request: &crate::AudioTranscriptionRequest,
    ) -> Result<(), BackendError> {
        if request.audio.is_some()
            || request.audio_ref.as_deref() != Some(self.artifact_id.as_str())
            || !crate::empty_audio_options(&request.extra_options)
            || request
                .chunk_length_s
                .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > OWNED_AUDIO_MAX_SECONDS as f32)
            || request
                .task
                .as_deref()
                .is_some_and(|v| !matches!(v, "transcribe" | "translate"))
        {
            return Err(BackendError::Config(
                "owned audio source/options mismatch".into(),
            ));
        }
        Ok(())
    }
}
pub fn owned_audio_id(workflow: &str, source_run: &str, content_hash: &str) -> String {
    let tuple = serde_json::to_vec(&(workflow, source_run, content_hash)).expect("string tuple");
    format!("audio-{}", blake3::hash(&tuple).to_hex())
}
pub fn validate_owned_wav(bytes: &[u8]) -> Result<(), BackendError> {
    crate::selected_audio_execution::validate_wav_bytes(
        bytes,
        OWNED_AUDIO_MAX_BYTES,
        OWNED_AUDIO_MAX_SECONDS,
        None,
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn owned_audio_global_admission_bounds_waiters_and_releases_on_abort() {
        // Reserve both atomically so concurrent one-slot fixtures can finish first.
        let permits = SNAPSHOTS.clone().acquire_many_owned(2).await.unwrap();
        let waiter = tokio::spawn(acquire_owned_audio_admission());
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        waiter.abort();
        assert!(matches!(waiter.await, Err(error) if error.is_cancelled()));
        drop(permits);
        let admitted = acquire_owned_audio_admission().await.unwrap();
        drop(admitted);
    }
}

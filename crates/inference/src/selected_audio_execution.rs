//! Validated scheduler selection and the separately approved executable target.
use std::path::Path;

use crate::backend::BackendError;
use crate::{
    BackendExecutionDecision, InferenceDeviceClass, InferenceDevicePolicy, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceTaskId, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef,
};
#[cfg(feature = "backend-pytorch")]
use crate::{InferenceDeviceId, ResolvedModelPackageFacts};

pub(crate) struct SelectedAudioLoad<'a> {
    #[cfg(feature = "backend-pytorch")]
    pub(crate) package: &'a ResolvedModelPackageFacts,
    #[cfg(feature = "backend-pytorch")]
    pub(crate) target: &'a PumasArtifactLoadTarget,
    #[cfg(feature = "backend-pytorch")]
    pub(crate) device: &'a InferenceDeviceId,
    // Validation also gates unsupported-owner refusal when PyTorch is absent.
    // No backend consumes load fields in that build, but keep the borrow contract.
    #[cfg(not(feature = "backend-pytorch"))]
    _selection: std::marker::PhantomData<&'a ()>,
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::Config(format!("selected audio: {}", message.into()))
}

fn same_model(left: &PumasModelRef, right: &PumasModelRef) -> bool {
    left.model_id.trim_start_matches("pumas://models/")
        == right.model_id.trim_start_matches("pumas://models/")
        && left.selected_artifact_id == right.selected_artifact_id
        && left.selected_artifact_path == right.selected_artifact_path
}

impl<'a> SelectedAudioLoad<'a> {
    pub(crate) async fn validate(
        request: &'a InferenceExecutionRequest,
        target: &'a PumasArtifactLoadTarget,
        decision: &'a BackendExecutionDecision,
    ) -> Result<Self, BackendError> {
        request
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        if !empty_audio_options(&request.extra_options) {
            return Err(invalid("nonempty backend options are unsupported"));
        }
        let InferenceExecutionInput::AudioTranscription { request: audio } = &request.input else {
            return Err(invalid("canonical audio transcription input required"));
        };
        validate_selected_audio_request(audio)?;
        if target
            .content_fingerprint
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(invalid("known selected content fingerprint required"));
        }
        if request.generation_options.is_some() {
            return Err(invalid(
                "generation controls are unsupported for selected audio",
            ));
        }
        if request
            .request_id
            .as_deref()
            .is_none_or(|id| id.trim().is_empty())
        {
            return Err(invalid("execution request id is required"));
        }
        target
            .validate_for_handoff()
            .map_err(|error| invalid(error.to_string()))?;
        let package = request
            .resolved_model_package_facts
            .as_ref()
            .ok_or_else(|| invalid("resolved package facts are required"))?;
        let model = request
            .model_ref
            .as_ref()
            .ok_or_else(|| invalid("request model identity is required"))?;
        let selected = decision
            .selected_model_ref
            .as_ref()
            .ok_or_else(|| invalid("scheduler model identity is required"))?;
        if audio.model.trim_start_matches("pumas://models/")
            != selected.model_id.trim_start_matches("pumas://models/")
        {
            return Err(invalid("audio model must preserve selected identity"));
        }
        for reference in [model, selected, &package.model_ref] {
            reference
                .validate()
                .map_err(|error| invalid(error.to_string()))?;
            if !same_model(reference, &target.model_ref) {
                return Err(invalid(
                    "request/package/target/scheduler model or artifact mismatch",
                ));
            }
        }
        if model
            .revision
            .as_ref()
            .is_some_and(|revision| selected.revision.as_ref() != Some(revision))
        {
            return Err(invalid(
                "selected revision must preserve the explicit requested revision",
            ));
        }
        for requested in [model, selected] {
            if let Some(revision) = &requested.revision {
                if package.model_ref.revision.as_ref() != Some(revision)
                    || target.model_ref.revision.as_ref() != Some(revision)
                {
                    return Err(invalid(
                        "explicit requested revision is missing or mismatched",
                    ));
                }
            }
        }
        if package
            .model_ref
            .revision
            .as_ref()
            .is_some_and(|revision| target.model_ref.revision.as_ref() != Some(revision))
        {
            return Err(invalid("target must preserve the known package revision"));
        }
        if request.task_id != InferenceTaskId::AudioTranscription
            || decision.selected_task_id != Some(request.task_id.clone())
            || !matches!(
                request.input,
                InferenceExecutionInput::AudioTranscription { .. }
            )
        {
            return Err(invalid(
                "exact requested and selected canonical audio task required",
            ));
        }
        let task = crate::resolve_task_registry_entry_from_evidence(&package.task)
            .map_err(|error| invalid(format!("invalid package task evidence: {error:?}")))?;
        if task.task_id != request.task_id {
            return Err(invalid(
                "package task must match the requested canonical audio task",
            ));
        }
        if !package.uses_current_contract()
            || target.package_facts_contract_version != Some(package.package_facts_contract_version)
        {
            return Err(invalid(
                "current package and target contract versions are required",
            ));
        }
        let _entry_path = PumasArtifactEntryPath::parse(&package.artifact.entry_path)
            .map_err(|error| invalid(error.to_string()))?;
        if package.artifact.validation_state != ModelValidationState::Valid
            || target.validation_state != ModelValidationState::Valid
            || !package.artifact.validation_errors.is_empty()
            || target.artifact_kind != package.artifact.artifact_kind
            || target.artifact_kind != ModelArtifactKind::HfCompatibleDirectory
            || target.storage_kind == ModelStorageKind::Unknown
            || target.storage_kind != package.artifact.storage_kind
        {
            return Err(invalid(
                "a valid matching local HF ASR directory is required",
            ));
        }
        if target.load_path_kind != PumasArtifactLoadPathKind::Directory
            || !Path::new(&target.local_load_path).is_absolute()
            || !tokio::fs::metadata(&target.local_load_path)
                .await
                .map_err(|error| invalid(format!("cannot inspect executable target: {error}")))?
                .is_dir()
        {
            return Err(invalid(
                "Pumas executable target must be an existing absolute directory",
            ));
        }
        if package.custom_code.requires_custom_code {
            return Err(invalid("custom model code is denied"));
        }
        let device = decision
            .selected_device_id
            .as_ref()
            .ok_or_else(|| invalid("concrete scheduler device is required"))?;
        if decision.selected_device_class != InferenceDeviceClass::Cpu || device.as_str() != "cpu" {
            return Err(invalid(
                "selected audio currently supports explicit CPU only",
            ));
        }
        let runtime = decision.selected_runtime_variant_id.as_str();
        if !matches!(runtime, "pytorch.cpu") {
            return Err(invalid("selected audio requires the PyTorch CPU variant"));
        }
        let nested = &decision.device_decision;
        if crate::backend::canonical_backend_key(decision.selected_backend_id.as_str()) != "pytorch"
            || nested.runtime_variant_id != decision.selected_runtime_variant_id
            || nested.selected_device_class != decision.selected_device_class
            || nested.selected_device_id != decision.selected_device_id
        {
            return Err(invalid("scheduler runtime/device decisions disagree"));
        }
        if let InferenceDevicePolicy::Explicit {
            device_class,
            device_id,
        } = &nested.policy
        {
            if *device_class != decision.selected_device_class
                || device_id.as_ref().is_some_and(|id| id != device)
            {
                return Err(invalid(
                    "selected device conflicts with explicit device policy",
                ));
            }
        }
        Ok(Self {
            #[cfg(feature = "backend-pytorch")]
            package,
            #[cfg(feature = "backend-pytorch")]
            target,
            #[cfg(feature = "backend-pytorch")]
            device,
            #[cfg(not(feature = "backend-pytorch"))]
            _selection: std::marker::PhantomData,
        })
    }
}

/// Only absent/null or an empty map represents no unsupported backend controls.
pub fn empty_audio_options(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_object().is_some_and(|options| options.is_empty())
}

/// Bounded fixture route, not an admission contract for arbitrary recordings.
pub fn validate_selected_audio_request(
    request: &crate::AudioTranscriptionRequest,
) -> Result<(), BackendError> {
    if request.audio_ref.is_some() {
        return Err(invalid(
            "audio references are unsupported in the small-WAV slice",
        ));
    }
    if !empty_audio_options(&request.extra_options) {
        return Err(invalid("nonempty audio options are unsupported"));
    }
    if request
        .chunk_length_s
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(invalid("chunk length must be finite and positive"));
    }
    validate_small_wav_audio(
        request
            .audio
            .as_ref()
            .ok_or_else(|| invalid("inline WAV audio required"))?,
    )
}

pub const SELECTED_AUDIO_MAX_WAV_BYTES: usize = 48 * 1024;
pub const SELECTED_AUDIO_MAX_ENCODED_BYTES: usize = 64 * 1024;

/// Admit canonical RIFF PCM16 WAV only: fmt16 then data, <=1s, mono/stereo,
/// 8..48kHz, <=48KiB including headers. No truncation or format conversion.
pub fn validate_small_wav_audio(audio: &crate::EncodedAudio) -> Result<(), BackendError> {
    use base64::Engine as _;
    let encoded = audio.data_base64.trim();
    if audio.mime_type != "audio/wav" || audio.data_base64.len() > SELECTED_AUDIO_MAX_ENCODED_BYTES
    {
        return Err(invalid(
            "small-WAV slice requires audio/wav within 64KiB encoded",
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| invalid("audio must be canonical base64 WAV"))?;
    if bytes.len() < 46 || bytes.len() > SELECTED_AUDIO_MAX_WAV_BYTES {
        return Err(invalid("WAV must be nonempty and at most 48KiB decoded"));
    }
    let u16_at = |i| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at = |i| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let channels = u16_at(22);
    let rate = u32_at(24);
    let align = u16_at(32);
    let data = u32_at(40) as usize;
    if &bytes[0..4] != b"RIFF"
        || u32_at(4) as usize != bytes.len() - 8
        || &bytes[8..12] != b"WAVE"
        || &bytes[12..16] != b"fmt "
        || u32_at(16) != 16
        || u16_at(20) != 1
        || !matches!(channels, 1 | 2)
        || !(8000..=48000).contains(&rate)
        || align != channels * 2
        || u16_at(34) != 16
        || u32_at(28) != rate * u32::from(align)
        || &bytes[36..40] != b"data"
        || data != bytes.len() - 44
        || data == 0
        || !data.is_multiple_of(usize::from(align))
        || data / usize::from(align) > rate as usize
        || audio.sample_rate_hz.is_some_and(|hint| hint != rate)
    {
        return Err(invalid("unsupported WAV: require exact PCM16 fmt16/data, mono/stereo, 8..48kHz, <=1s and matching rate"));
    }
    Ok(())
}

#[cfg(all(test, feature = "backend-pytorch"))]
pub(crate) fn fixture() -> (
    tempfile::TempDir,
    InferenceExecutionRequest,
    PumasArtifactLoadTarget,
    BackendExecutionDecision,
) {
    let directory = tempfile::tempdir().unwrap();
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../tests/fixtures/inference_package_facts/hf_audio_transcription_package_facts.json"
    ))
    .unwrap();
    package.model_ref.model_id = "synthetic/asr-fixture".into();
    package.model_ref.revision = Some("synthetic-r1".into());
    package.transformers = None;
    package.custom_code.requires_custom_code = false;
    package.custom_code.custom_code_sources.clear();
    package.custom_code.auto_map_sources.clear();
    let target = PumasArtifactLoadTarget {
        model_ref: package.model_ref.clone(),
        artifact_kind: package.artifact.artifact_kind.clone(),
        local_load_path: directory.path().to_str().unwrap().into(),
        load_path_kind: PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("test-root".into()),
        storage_kind: package.artifact.storage_kind.clone(),
        validation_state: ModelValidationState::Valid,
        content_fingerprint: Some("synthetic-content-r1".into()),
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    let device = InferenceDeviceId::parse("cpu").unwrap();
    let runtime = crate::RuntimeVariantId::parse("pytorch.cpu").unwrap();
    let decision = BackendExecutionDecision {
        selected_backend_id: crate::BackendId::parse("pytorch").unwrap(),
        selected_runtime_variant_id: runtime.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(device.clone()),
        device_decision: crate::DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id: runtime,
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device),
            diagnostics: vec![],
        },
        selected_task_id: Some(InferenceTaskId::AudioTranscription),
        selected_model_ref: Some(package.model_ref.clone()),
        diagnostics: vec![],
        dependency_readiness: vec![],
        selection_policy_trace: None,
    };
    let request = InferenceExecutionRequest {
        request_id: Some("selected-audio-test".into()),
        task_id: InferenceTaskId::AudioTranscription,
        model_ref: Some(package.model_ref.clone()),
        model_name: None,
        resolved_model_package_facts: Some(package),
        input: InferenceExecutionInput::AudioTranscription {
            request: crate::AudioTranscriptionRequest {
                model: "synthetic/asr-fixture".into(),
                audio: Some(crate::EncodedAudio {
                    data_base64: include_str!("../tests/fixtures/selected_audio/tiny_pcm16.base64")
                        .into(),
                    mime_type: "audio/wav".into(),
                    sample_rate_hz: Some(16000),
                }),
                audio_ref: None,
                language: Some("en".into()),
                prompt: Some("fixture context".into()),
                task: Some("transcribe".into()),
                chunk_length_s: Some(0.5),
                extra_options: serde_json::json!({}),
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };
    (directory, request, target, decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    fn encoded(bytes: &[u8]) -> crate::EncodedAudio {
        crate::EncodedAudio {
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
            mime_type: "audio/wav".into(),
            sample_rate_hz: None,
        }
    }
    #[test]
    fn selected_audio_wav_bounds_and_header_are_exact_without_truncation() {
        let fixture = include_bytes!("../tests/fixtures/selected_audio/tiny_pcm16.wav");
        validate_small_wav_audio(&encoded(fixture)).unwrap();
        for (offset, value) in [
            (4, 0u32),
            (16, 18),
            (20, 3),
            (22, 3),
            (24, 7999),
            (28, 0),
            (32, 0),
            (34, 32),
            (40, u32::MAX),
        ] {
            let mut bytes = fixture.to_vec();
            let width = if matches!(offset, 20 | 22 | 32 | 34) {
                2
            } else {
                4
            };
            bytes[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
            assert!(
                validate_small_wav_audio(&encoded(&bytes)).is_err(),
                "offset={offset}"
            );
        }
        let mut trailing = fixture.to_vec();
        trailing.extend_from_slice(b"JUNK");
        assert!(validate_small_wav_audio(&encoded(&trailing)).is_err());
        // Valid canonical headers, but over the duration or absolute byte bound.
        for frames in [16001usize, 24576] {
            let mut bytes = fixture[..44].to_vec();
            bytes.resize(44 + frames * 2, 0);
            let size = bytes.len();
            bytes[4..8].copy_from_slice(&((size - 8) as u32).to_le_bytes());
            bytes[40..44].copy_from_slice(&((size - 44) as u32).to_le_bytes());
            assert!(validate_small_wav_audio(&encoded(&bytes)).is_err());
        }
        let mut rate = encoded(fixture);
        rate.sample_rate_hz = Some(8000);
        assert!(validate_small_wav_audio(&rate).is_err());
        let mut huge = encoded(fixture);
        huge.data_base64 = "A".repeat(SELECTED_AUDIO_MAX_ENCODED_BYTES + 1);
        assert!(validate_small_wav_audio(&huge).is_err());
        huge.data_base64 = format!(
            "{}{}",
            " ".repeat(SELECTED_AUDIO_MAX_ENCODED_BYTES),
            encoded(fixture).data_base64
        );
        assert!(validate_small_wav_audio(&huge).is_err());
    }
    #[tokio::test]
    #[cfg(feature = "backend-pytorch")]
    async fn selected_audio_selection_requires_exact_task_cpu_target_and_known_content() {
        let (_directory, request, target, decision) = fixture();
        SelectedAudioLoad::validate(&request, &target, &decision)
            .await
            .unwrap();
        for mode in [
            "revision",
            "unknown_content",
            "artifact",
            "device",
            "runtime",
            "task",
            "model",
        ] {
            let mut target = target.clone();
            let mut decision = decision.clone();
            match mode {
                "revision" => target.model_ref.revision = Some("other".into()),
                "unknown_content" => target.content_fingerprint = None,
                "artifact" => target.model_ref.selected_artifact_id = Some("other".into()),
                "device" => decision.selected_device_id = None,
                "runtime" => decision.selected_runtime_variant_id = "pytorch.cuda".parse().unwrap(),
                "task" => decision.selected_task_id = Some(InferenceTaskId::TextGeneration),
                "model" => decision.selected_model_ref.as_mut().unwrap().model_id = "other".into(),
                _ => unreachable!(),
            }
            assert!(
                SelectedAudioLoad::validate(&request, &target, &decision)
                    .await
                    .is_err(),
                "{mode}"
            );
        }
    }
}

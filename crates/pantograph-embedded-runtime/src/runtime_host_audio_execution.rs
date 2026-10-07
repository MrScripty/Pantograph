//! Canonical structured audio inputs and legacy-compatible outputs under scheduler identity.
use inference::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, InferenceDevicePolicy, InferenceExecutionInput, InferenceExecutionRequest,
    InferenceExecutionResult, InferenceTaskId, PumasArtifactLoadTarget, ResolvedModelPackageFacts,
    RuntimeVariantId,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, RuntimeHostExecutionOutput, RuntimeHostExecutionOutputValue,
    RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
};
use std::collections::HashMap;

pub(crate) const AUDIO_TASK: &str = "audio_transcription";
type Result<T> = std::result::Result<T, String>;

pub(crate) struct RuntimeHostAudioProjection {
    pub request: InferenceExecutionRequest,
    pub target: PumasArtifactLoadTarget,
    pub decision: BackendExecutionDecision,
}

fn inputs(request: &RuntimeHostExecutionRequest) -> Result<HashMap<String, serde_json::Value>> {
    if request.handoff.task_intent.task_type.as_str() != AUDIO_TASK {
        return Err("audio task required".into());
    }
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or("scheduler selection required")?;
    if !matches!(selected.selected_runtime_id.as_str(), "pytorch")
        || !matches!(
            selected
                .selected_runtime_variant_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("pytorch.cpu")
        )
        || selected.selected_device_ids.len() != 1
        || selected.selected_device_ids[0].as_str() != "cpu"
        || !selected.task_intent.trait_settings.is_empty()
        || !selected.runtime_trait_settings.is_empty()
    {
        return Err("audio requires scheduler-selected PyTorch CPU with supported controls".into());
    }
    let mut values = HashMap::new();
    for input in &request.materialized_inputs {
        let value = match (input.port_id.as_str(), &input.value) {
            ("audio", RuntimeHostExecutionInputValue::String(value)) => serde_json::json!(value),
            ("audio" | "extra_options", RuntimeHostExecutionInputValue::Json(value)) => {
                value.clone()
            }
            (
                "language" | "lang" | "prompt" | "context" | "asr_task" | "asrTask" | "task",
                RuntimeHostExecutionInputValue::String(value),
            ) => serde_json::json!(value),
            ("stream", RuntimeHostExecutionInputValue::Bool(value)) => serde_json::json!(value),
            (
                "chunk_length_s" | "chunkLengthS" | "chunk_length_seconds",
                RuntimeHostExecutionInputValue::F64(value),
            ) => serde_json::Value::Number(value.clone()),
            (
                "chunk_length_s" | "chunkLengthS" | "chunk_length_seconds",
                RuntimeHostExecutionInputValue::U64(value),
            ) => serde_json::json!(value),
            (
                "chunk_length_s" | "chunkLengthS" | "chunk_length_seconds",
                RuntimeHostExecutionInputValue::I64(value),
            ) => serde_json::json!(value),
            (
                "chunk_length_s" | "chunkLengthS" | "chunk_length_seconds",
                RuntimeHostExecutionInputValue::String(value),
            ) => serde_json::json!(value),
            _ => {
                return Err(format!(
                    "unsupported small-WAV audio input {}",
                    input.port_id
                ))
            }
        };
        if values.insert(input.port_id.clone(), value).is_some() {
            return Err("duplicate audio input".into());
        }
    }
    if values
        .get("stream")
        .is_some_and(|value| value != &serde_json::json!(false))
    {
        return Err("audio streaming is unsupported".into());
    }
    Ok(values)
}

fn canonical_input(
    values: &HashMap<String, serde_json::Value>,
    model: &str,
) -> Result<InferenceExecutionInput> {
    let value = values.get("audio").ok_or("inline WAV audio required")?;
    let audio = if let Some(data) = value.as_str() {
        inference::EncodedAudio {
            data_base64: data.into(),
            mime_type: "audio/wav".into(),
            sample_rate_hz: None,
        }
    } else {
        let object = value
            .as_object()
            .ok_or("audio must be inline base64 string or encoded-audio object")?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "data_base64"
                    | "dataBase64"
                    | "audio_base64"
                    | "audioBase64"
                    | "audio_data"
                    | "audioData"
                    | "mime_type"
                    | "mimeType"
                    | "sample_rate_hz"
                    | "sampleRateHz"
            )
        }) {
            return Err(
                "unsupported audio object field or audio reference; small-WAV slice only".into(),
            );
        }
        let data = [
            "data_base64",
            "dataBase64",
            "audio_base64",
            "audioBase64",
            "audio_data",
            "audioData",
        ]
        .into_iter()
        .find_map(|key| object.get(key))
        .and_then(|v| v.as_str())
        .ok_or("inline audio data string required")?;
        let mime = object.get("mime_type").or_else(|| object.get("mimeType"));
        let mime = mime
            .map(|v| v.as_str().ok_or("audio MIME must be a string"))
            .transpose()?
            .unwrap_or("audio/wav");
        let rate = object
            .get("sample_rate_hz")
            .or_else(|| object.get("sampleRateHz"));
        let rate = rate
            .map(|v| {
                v.as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("sample rate must be an unsigned integer")
            })
            .transpose()?;
        inference::EncodedAudio {
            data_base64: data.into(),
            mime_type: mime.into(),
            sample_rate_hz: rate,
        }
    };
    let string = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| values.get(*key))
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
    };
    let chunk = ["chunk_length_s", "chunkLengthS", "chunk_length_seconds"]
        .into_iter()
        .find_map(|key| values.get(key))
        .map(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                .map(|v| v as f32)
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or("chunk length must be finite and positive")
        })
        .transpose()?;
    let options = values
        .get("extra_options")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let request = inference::AudioTranscriptionRequest {
        model: model.into(),
        audio: Some(audio),
        audio_ref: None,
        language: string(&["language", "lang"]),
        prompt: string(&["prompt", "context"]),
        task: string(&["asr_task", "asrTask", "task"]),
        chunk_length_s: chunk,
        extra_options: options,
    };
    inference::validate_selected_audio_request(&request).map_err(|error| error.to_string())?;
    Ok(InferenceExecutionInput::AudioTranscription { request })
}

pub(crate) fn validate_runtime_host_audio_request(
    request: &RuntimeHostExecutionRequest,
) -> Result<()> {
    let values = inputs(request)?;
    canonical_input(&values, "input-validation")?;
    Ok(())
}

pub(crate) fn project_runtime_host_audio(
    request: &ValidatedRuntimeHostExecutionRequest,
    package: ResolvedModelPackageFacts,
    target: PumasArtifactLoadTarget,
) -> Result<RuntimeHostAudioProjection> {
    let request = request.as_ref();
    let values = inputs(request)?;
    let input = canonical_input(
        &values,
        &request
            .handoff
            .dispatch_decision
            .as_ref()
            .ok_or("scheduler selection required")?
            .selected_model_ref
            .model_id,
    )?;
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or("scheduler selection required")?;
    let model = super::runtime_host_text_execution::project_model_ref(&selected.selected_model_ref);
    let mut requested = model.clone();
    requested.revision = request.handoff.task_intent.model_ref.revision.clone();
    let runtime = RuntimeVariantId::parse(
        selected
            .selected_runtime_variant_id
            .as_ref()
            .ok_or("runtime variant required")?
            .as_str(),
    )
    .map_err(|error| error.to_string())?;
    let device = InferenceDeviceId::parse("cpu").map_err(|error| error.to_string())?;
    let decision = BackendExecutionDecision {
        selected_backend_id: BackendId::parse("pytorch").map_err(|error| error.to_string())?,
        selected_runtime_variant_id: runtime.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(device.clone()),
        device_decision: DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id: runtime,
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device),
            diagnostics: vec![],
        },
        selected_task_id: Some(InferenceTaskId::AudioTranscription),
        selected_model_ref: Some(model),
        diagnostics: vec![],
        dependency_readiness: vec![],
        selection_policy_trace: None,
    };
    Ok(RuntimeHostAudioProjection {
        request: InferenceExecutionRequest {
            request_id: Some(request.execution_request_id.clone()),
            task_id: InferenceTaskId::AudioTranscription,
            model_ref: Some(requested),
            model_name: None,
            resolved_model_package_facts: Some(package),
            input,
            generation_options: None,
            extra_options: serde_json::Value::Null,
        },
        target,
        decision,
    })
}

pub(crate) fn audio_outputs(
    result: InferenceExecutionResult,
) -> Result<Vec<RuntimeHostExecutionOutput>> {
    let InferenceExecutionResult::AudioTranscription {
        result,
        option_diagnostics,
    } = result
    else {
        return Err("audio transcription result kind required".into());
    };
    let values = [
        ("response", serde_json::json!(result.text)),
        ("stream", serde_json::Value::Null),
        ("text", serde_json::json!(result.text)),
        ("language", serde_json::json!(result.language)),
        (
            "duration_seconds",
            serde_json::json!(result.duration_seconds),
        ),
        (
            "segments",
            serde_json::to_value(result.segments).map_err(|e| e.to_string())?,
        ),
        ("metadata", result.metadata),
        (
            "diagnostics",
            serde_json::to_value(option_diagnostics).map_err(|e| e.to_string())?,
        ),
    ];
    Ok(values
        .into_iter()
        .map(|(port_id, value)| RuntimeHostExecutionOutput {
            port_id: port_id.into(),
            value: match value {
                serde_json::Value::String(s) => RuntimeHostExecutionOutputValue::String(s),
                value => RuntimeHostExecutionOutputValue::Json(value),
            },
        })
        .collect())
}

#[cfg(all(test, feature = "backend-candle"))]
#[path = "runtime_host_audio_execution_tests.rs"]
pub(crate) mod tests;

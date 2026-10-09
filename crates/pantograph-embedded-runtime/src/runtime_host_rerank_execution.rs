//! Canonical structured rerank inputs and legacy-compatible outputs under scheduler identity.
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

pub(crate) const RERANK_TASK: &str = "rerank";
type Result<T> = std::result::Result<T, String>;

pub(crate) struct RuntimeHostRerankProjection {
    pub request: InferenceExecutionRequest,
    pub target: PumasArtifactLoadTarget,
    pub decision: BackendExecutionDecision,
}

fn inputs(request: &RuntimeHostExecutionRequest) -> Result<HashMap<String, serde_json::Value>> {
    if request.handoff.task_intent.task_type.as_str() != RERANK_TASK {
        return Err("rerank task required".into());
    }
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or("scheduler selection required")?;
    if !matches!(
        selected.selected_runtime_id.as_str(),
        "llamacpp" | "llama_cpp"
    ) || !matches!(
        selected
            .selected_runtime_variant_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("llama_cpp.cpu" | "llamacpp.cpu")
    ) || selected.selected_device_ids.len() != 1
        || selected.selected_device_ids[0].as_str() != "cpu"
        || !selected.task_intent.trait_settings.is_empty()
        || !selected.runtime_trait_settings.is_empty()
    {
        return Err(
            "rerank requires scheduler-selected llama.cpp CPU with supported controls".into(),
        );
    }
    let mut values = HashMap::new();
    for input in &request.materialized_inputs {
        if !matches!(
            input.port_id.as_str(),
            "query"
                | "documents"
                | "documents_json"
                | "top_n"
                | "topN"
                | "top_k"
                | "topK"
                | "return_documents"
                | "returnDocuments"
                | "task_options"
                | "extra_options"
                | "stream"
        ) {
            return Err(format!("unsupported rerank input {}", input.port_id));
        }
        let value = match (input.port_id.as_str(), &input.value) {
            ("query", RuntimeHostExecutionInputValue::TranscriptText(value))
                if value.len() <= 65536 =>
            {
                serde_json::json!(value)
            }
            ("query" | "documents_json", RuntimeHostExecutionInputValue::String(value)) => {
                serde_json::json!(value)
            }
            (
                "documents" | "task_options" | "extra_options",
                RuntimeHostExecutionInputValue::Json(value),
            ) => value.clone(),
            (
                "stream" | "return_documents" | "returnDocuments",
                RuntimeHostExecutionInputValue::Bool(value),
            ) => serde_json::json!(value),
            ("top_n" | "topN" | "top_k" | "topK", RuntimeHostExecutionInputValue::I64(value)) => {
                serde_json::json!(value)
            }
            ("top_n" | "topN" | "top_k" | "topK", RuntimeHostExecutionInputValue::U64(value)) => {
                serde_json::json!(value)
            }
            (
                "top_n" | "topN" | "top_k" | "topK",
                RuntimeHostExecutionInputValue::String(value),
            ) => serde_json::json!(value),
            _ => return Err(format!("unsupported rerank value for {}", input.port_id)),
        };
        if values.insert(input.port_id.clone(), value).is_some() {
            return Err("duplicate rerank input".into());
        }
    }
    if values
        .get("stream")
        .is_some_and(|value| value != &serde_json::json!(false))
    {
        return Err("rerank streaming is unsupported".into());
    }
    Ok(values)
}

fn canonical_input(values: &HashMap<String, serde_json::Value>) -> Result<InferenceExecutionInput> {
    let query = values
        .get("query")
        .and_then(|value| value.as_str())
        .ok_or("rerank query required")?;
    if query.trim().is_empty() {
        return Err("rerank query must not be blank".into());
    }
    let parsed;
    let documents = if let Some(value) = values.get("documents") {
        value
    } else {
        parsed = serde_json::from_str(
            values
                .get("documents_json")
                .and_then(|value| value.as_str())
                .ok_or("rerank documents required")?,
        )
        .map_err(|_| "documents_json must be valid JSON")?;
        &parsed
    };
    let mut texts = Vec::new();
    for value in documents
        .as_array()
        .ok_or("rerank documents must be an array")?
    {
        let text = value
            .as_str()
            .or_else(|| value.get("text").and_then(|value| value.as_str()))
            .or_else(|| value.get("content").and_then(|value| value.as_str()))
            .or_else(|| value.get("document").and_then(|value| value.as_str()))
            .ok_or("rerank documents require text/content/document strings")?;
        if !text.trim().is_empty() {
            texts.push(text.to_owned());
        }
    }
    if texts.is_empty() {
        return Err("rerank documents must not be empty".into());
    }
    let options = values.get("task_options");
    if options.is_some_and(|value| !value.is_object()) {
        return Err("rerank task_options must be an object".into());
    }
    let top_n = ["top_n", "topN", "top_k", "topK"]
        .into_iter()
        .find_map(|key| values.get(key))
        .or_else(|| {
            ["top_n", "topN", "top_k", "topK"]
                .into_iter()
                .find_map(|key| options.and_then(|value| value.get(key)))
        })
        .map(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                .filter(|value| *value > 0)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or("rerank top_n must be positive")
        })
        .transpose()?;
    let return_documents = values
        .get("return_documents")
        .or_else(|| values.get("returnDocuments"))
        .or_else(|| options.and_then(|value| value.get("return_documents")))
        .map(|value| {
            value
                .as_bool()
                .ok_or("rerank return_documents must be boolean")
        })
        .transpose()?
        .unwrap_or(true);
    Ok(InferenceExecutionInput::Rerank {
        query: query.to_owned(),
        documents: texts,
        top_n,
        return_documents,
    })
}

pub(crate) fn validate_runtime_host_rerank_request(
    request: &RuntimeHostExecutionRequest,
) -> Result<()> {
    let values = inputs(request)?;
    canonical_input(&values)?;
    if values
        .get("extra_options")
        .is_some_and(|value| !value.is_object())
    {
        return Err("rerank extra_options must be an object".into());
    }
    Ok(())
}

pub(crate) fn project_runtime_host_rerank(
    request: &ValidatedRuntimeHostExecutionRequest,
    package: ResolvedModelPackageFacts,
    target: PumasArtifactLoadTarget,
) -> Result<RuntimeHostRerankProjection> {
    let request = request.as_ref();
    let values = inputs(request)?;
    let input = canonical_input(&values)?;
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
        selected_backend_id: BackendId::parse("llama_cpp").map_err(|error| error.to_string())?,
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
        selected_task_id: Some(InferenceTaskId::Rerank),
        selected_model_ref: Some(model),
        diagnostics: vec![],
        dependency_readiness: vec![],
        selection_policy_trace: None,
    };
    let extra_options = values
        .get("extra_options")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !extra_options.is_object() {
        return Err("rerank extra_options must be an object".into());
    }
    Ok(RuntimeHostRerankProjection {
        request: InferenceExecutionRequest {
            request_id: Some(request.execution_request_id.clone()),
            task_id: InferenceTaskId::Rerank,
            model_ref: Some(requested),
            model_name: None,
            resolved_model_package_facts: Some(package),
            input,
            generation_options: None,
            extra_options,
        },
        target,
        decision,
    })
}

pub(crate) fn rerank_outputs(
    result: InferenceExecutionResult,
) -> Result<Vec<RuntimeHostExecutionOutput>> {
    let InferenceExecutionResult::Rerank {
        response,
        option_diagnostics,
    } = result
    else {
        return Err("rerank result kind required".into());
    };
    let top_document = response
        .results
        .first()
        .and_then(|item| item.document.clone());
    let top_score = response.results.first().map(|item| item.score);
    Ok(vec![
        RuntimeHostExecutionOutput {
            port_id: "results".into(),
            value: RuntimeHostExecutionOutputValue::Json(
                serde_json::to_value(&response.results).map_err(|error| error.to_string())?,
            ),
        },
        RuntimeHostExecutionOutput {
            port_id: "scores".into(),
            value: RuntimeHostExecutionOutputValue::Json(serde_json::json!(response
                .results
                .iter()
                .map(|item| item.score)
                .collect::<Vec<_>>())),
        },
        RuntimeHostExecutionOutput {
            port_id: "top_document".into(),
            value: top_document
                .map(RuntimeHostExecutionOutputValue::String)
                .unwrap_or(RuntimeHostExecutionOutputValue::Json(
                    serde_json::Value::Null,
                )),
        },
        RuntimeHostExecutionOutput {
            port_id: "top_score".into(),
            value: RuntimeHostExecutionOutputValue::Json(serde_json::json!(top_score)),
        },
        RuntimeHostExecutionOutput {
            port_id: "diagnostics".into(),
            value: RuntimeHostExecutionOutputValue::Json(
                serde_json::to_value(option_diagnostics).map_err(|error| error.to_string())?,
            ),
        },
    ])
}

#[cfg(all(test, feature = "backend-candle"))]
#[path = "runtime_host_rerank_execution_tests.rs"]
pub(crate) mod tests;

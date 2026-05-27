use std::collections::HashMap;

use pantograph_dependency_planning::{
    DependencyBindingId, DependencyNodeTypeId, DependencyPlanningCallerContext,
    DependencyPlanningIdentityKey, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyTaskId, DeviceIntentId, PumasModelRef, RuntimeIntentId, SchedulerIntent,
};

use super::super::{read_optional_input_string_aliases, read_optional_input_value_aliases};
use super::input_projection::{infer_task_type_primary, read_input_selected_binding_ids};

const DEPENDENCY_PLANNING_SOURCE_PORT_ID: &str = "pumas_model_ref";

pub(crate) fn build_dependency_planning_request(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<DependencyPlanningRequest, String> {
    let model_ref = read_dependency_planning_pumas_model_ref(inputs)?;
    if model_ref.selected_artifact_path.is_some() {
        return Err(
            "Dependency planning request rejected pumas_model_ref.selected_artifact_path; node-engine dependency identity must stay path-free"
                .to_string(),
        );
    }

    let task_id = DependencyTaskId::parse(infer_task_type_primary(node_type, inputs))
        .map_err(|error| error.to_string())?;
    let scheduler_intent = dependency_planning_scheduler_intent(inputs)?;
    let platform_context = dependency_planning_platform_context(inputs)?;
    let selected_binding_ids = dependency_planning_selected_binding_ids(inputs)?;
    let caller_context = dependency_planning_caller_context(node_type)?;

    let request = DependencyPlanningRequest {
        model_ref: model_ref.clone(),
        task_id,
        task_type: None,
        expected_artifact_kind: None,
        scheduler_intent,
        platform_context,
        selected_binding_ids,
        dependency_override_patches: Vec::new(),
        trait_intents: Vec::new(),
        caller_context,
    };
    request.validate().map_err(|error| error.to_string())?;

    let identity_key = DependencyPlanningIdentityKey {
        model_ref,
        task_id: request.task_id.clone(),
        task_type: request.task_type.clone(),
        expected_artifact_kind: request.expected_artifact_kind.clone(),
        scheduler_intent: request.scheduler_intent.clone(),
        platform_context: request.platform_context.clone(),
        selected_binding_ids: request.selected_binding_ids.clone(),
    };
    identity_key.validate().map_err(|error| error.to_string())?;

    Ok(request)
}

fn read_dependency_planning_pumas_model_ref(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<PumasModelRef, String> {
    let Some(raw) =
        read_optional_input_value_aliases(inputs, &["pumas_model_ref", "pumasModelRef"])
    else {
        return Err(
            "Missing pumas_model_ref input. Connect Puma-Lib pumas_model_ref output.".to_string(),
        );
    };
    serde_json::from_value::<PumasModelRef>(raw)
        .map_err(|error| format!("Invalid pumas_model_ref input: {error}"))
        .and_then(|model_ref| {
            model_ref
                .validate()
                .map_err(|error| error.to_string())
                .map(|_| model_ref)
        })
}

fn dependency_planning_scheduler_intent(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<SchedulerIntent, String> {
    let requested_runtime_id =
        read_optional_input_string_aliases(inputs, &["runtime_id", "runtimeId"])
            .map(RuntimeIntentId::parse)
            .transpose()
            .map_err(|error| error.to_string())?;
    let requested_device_id =
        read_optional_input_string_aliases(inputs, &["device_id", "deviceId"])
            .map(DeviceIntentId::parse)
            .transpose()
            .map_err(|error| error.to_string())?;

    Ok(SchedulerIntent {
        requested_runtime_id,
        requested_device_id,
    })
}

fn dependency_planning_platform_context(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Option<DependencyPlanningPlatformContext>, String> {
    let Some(raw) =
        read_optional_input_value_aliases(inputs, &["platform_context", "platformContext"])
    else {
        return Ok(None);
    };
    let Some(platform_key) = raw
        .get("platform_key")
        .or_else(|| raw.get("platformKey"))
        .and_then(serde_json::Value::as_str)
    else {
        return Err(
            "Invalid platform_context input: expected typed platform_key field".to_string(),
        );
    };

    DependencyPlanningPlatformContext::parse_platform_key(platform_key)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn dependency_planning_selected_binding_ids(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Vec<DependencyBindingId>, String> {
    read_input_selected_binding_ids(inputs)
        .into_iter()
        .map(DependencyBindingId::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn dependency_planning_caller_context(
    node_type: &str,
) -> Result<DependencyPlanningCallerContext, String> {
    Ok(DependencyPlanningCallerContext {
        source_node_type: Some(
            DependencyNodeTypeId::parse(node_type).map_err(|error| error.to_string())?,
        ),
        workflow_id: None,
        node_id: None,
        port_id: Some(DEPENDENCY_PLANNING_SOURCE_PORT_ID.to_string()),
        run_id: None,
    })
}

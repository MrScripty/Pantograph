use pantograph_dependency_planning::PumasModelRef;
use pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue;
use pantograph_scheduler::{
    SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
};

use super::{materialize_runtime_host_inputs, WorkflowRuntimeHostTaskInputMappingError};
use crate::workflow::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskInputBinding,
    WorkflowSchedulerTaskMediaArtifactRef, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue, WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

#[test]
fn materializes_path_free_runtime_host_inputs_from_completed_task_results() {
    let task = runtime_task(vec![
        input_binding("prompt", "text", "prompt"),
        input_binding("seed", "value", "seed"),
        input_binding("mask", "artifact", "mask"),
    ]);
    let results = vec![
        task_result(
            "prompt",
            "text",
            WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string()),
        ),
        task_result("seed", "value", WorkflowSchedulerTaskResultValue::U64(42)),
        task_result(
            "mask",
            "artifact",
            WorkflowSchedulerTaskResultValue::MediaArtifactRef(
                WorkflowSchedulerTaskMediaArtifactRef {
                    artifact_id: "artifact.mask.001".to_string(),
                    media_type: Some("image/png".to_string()),
                },
            ),
        ),
    ];

    let inputs =
        materialize_runtime_host_inputs(&task, &results).expect("runtime inputs materialize");

    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0].port_id, "prompt");
    assert_eq!(
        inputs[0].value,
        RuntimeHostExecutionInputValue::String("paint a red cube".to_string())
    );
    assert_eq!(inputs[1].value, RuntimeHostExecutionInputValue::U64(42));
    assert_eq!(
        inputs[2].value,
        RuntimeHostExecutionInputValue::MediaArtifactRef(
            pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef {
                artifact_id: "artifact.mask.001".to_string(),
                media_type: Some("image/png".to_string()),
            }
        )
    );
}

#[test]
fn skips_model_ref_binding_because_model_identity_lives_in_scheduler_handoff() {
    let task = runtime_task(vec![
        input_binding("model-selector", "pumas_model_ref", "pumas_model_ref"),
        input_binding("prompt", "text", "prompt"),
    ]);
    let results = vec![
        task_result(
            "model-selector",
            "pumas_model_ref",
            WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
                model_id: "image/example/tiny-diffusion".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers-bundle".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            }),
        ),
        task_result(
            "prompt",
            "text",
            WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string()),
        ),
    ];

    let inputs =
        materialize_runtime_host_inputs(&task, &results).expect("runtime inputs materialize");

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].port_id, "prompt");
}

#[test]
fn rejects_retired_model_ref_target_port() {
    let task = runtime_task(vec![input_binding(
        "model-selector",
        "pumas_model_ref",
        "model_ref",
    )]);
    let results = vec![task_result(
        "model-selector",
        "pumas_model_ref",
        WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
            model_id: "image/example/tiny-diffusion".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
    )];

    let error = materialize_runtime_host_inputs(&task, &results)
        .expect_err("retired model_ref target must not be accepted as model identity");

    assert!(matches!(
        error,
        WorkflowRuntimeHostTaskInputMappingError::UnsupportedMaterializedInput {
            value_type: "pumas_model_ref",
            target_port_id,
            ..
        } if target_port_id == "model_ref"
    ));
}

#[test]
fn rejects_model_ref_on_non_model_runtime_input_port() {
    let task = runtime_task(vec![input_binding(
        "model-selector",
        "pumas_model_ref",
        "prompt",
    )]);
    let results = vec![task_result(
        "model-selector",
        "pumas_model_ref",
        WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
            model_id: "image/example/tiny-diffusion".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
    )];

    let error = materialize_runtime_host_inputs(&task, &results)
        .expect_err("model refs are not runtime-host materialized input values");

    assert!(matches!(
        error,
        WorkflowRuntimeHostTaskInputMappingError::UnsupportedMaterializedInput {
            value_type: "pumas_model_ref",
            target_port_id,
            ..
        } if target_port_id == "prompt"
    ));
}

#[test]
fn rejects_missing_materialized_input_before_runtime_host_dispatch() {
    let task = runtime_task(vec![input_binding("prompt", "text", "prompt")]);

    let error = materialize_runtime_host_inputs(&task, &[])
        .expect_err("missing input must block runtime-host request materialization");

    assert!(matches!(
        error,
        WorkflowRuntimeHostTaskInputMappingError::MissingMaterializedInput {
            source_task_id,
            source_port_id,
            ..
        } if source_task_id == "prompt" && source_port_id == "text"
    ));
}

#[test]
fn rejects_failed_upstream_result_before_runtime_host_dispatch() {
    let task = runtime_task(vec![input_binding("prompt", "text", "prompt")]);
    let mut failed = task_result(
        "prompt",
        "text",
        WorkflowSchedulerTaskResultValue::String("failed".to_string()),
    );
    failed.status = WorkflowSchedulerTaskResultStatus::Failed;

    let error = materialize_runtime_host_inputs(&task, &[failed])
        .expect_err("failed upstream result must block runtime-host request materialization");

    assert!(matches!(
        error,
        WorkflowRuntimeHostTaskInputMappingError::InvalidMaterializedInput { .. }
    ));
}

fn runtime_task(input_bindings: Vec<WorkflowSchedulerTaskInputBinding>) -> WorkflowSchedulerTask {
    WorkflowSchedulerTask {
        workflow_id: SchedulerWorkflowId::parse("workflow.image").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run.image.001").expect("run id"),
        node_id: SchedulerNodeId::parse("infer").expect("node id"),
        task_id: SchedulerTaskId::parse("infer").expect("task id"),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: Vec::new(),
        input_bindings,
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        runtime_source_context: None,
        diagnostics: Vec::new(),
    }
}

fn input_binding(
    source_task_id: &str,
    source_port_id: &str,
    target_port_id: &str,
) -> WorkflowSchedulerTaskInputBinding {
    WorkflowSchedulerTaskInputBinding {
        source_node_id: SchedulerNodeId::parse(source_task_id).expect("source node id"),
        source_task_id: SchedulerTaskId::parse(source_task_id).expect("source task id"),
        source_port_id: source_port_id.to_string(),
        target_port_id: target_port_id.to_string(),
    }
}

fn task_result(
    task_id: &str,
    port_id: &str,
    value: WorkflowSchedulerTaskResultValue,
) -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: "workflow.image".to_string(),
        workflow_run_id: "run.image.001".to_string(),
        node_id: task_id.to_string(),
        task_id: task_id.to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs: vec![WorkflowSchedulerTaskResultOutput {
            port_id: port_id.to_string(),
            value,
        }],
        diagnostics: Vec::new(),
        terminal_metadata: None,
    }
}

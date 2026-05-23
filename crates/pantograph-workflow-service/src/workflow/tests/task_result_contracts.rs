use pantograph_dependency_planning::PumasModelRef;
use serde_json::json;

use super::*;

fn valid_result() -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: "workflow-task-results".to_string(),
        workflow_run_id: "run-task-results".to_string(),
        node_id: "model-node".to_string(),
        task_id: "model-node".to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs: vec![
            WorkflowSchedulerTaskResultOutput {
                port_id: "pumas_model_ref".to_string(),
                value: WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
                    model_id: "image/example/tiny-diffusion".to_string(),
                    revision: Some("main".to_string()),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                }),
            },
            WorkflowSchedulerTaskResultOutput {
                port_id: "image".to_string(),
                value: WorkflowSchedulerTaskResultValue::MediaArtifactRef(
                    WorkflowSchedulerTaskMediaArtifactRef {
                        artifact_id: "artifact-image-1".to_string(),
                        media_type: Some("image/png".to_string()),
                    },
                ),
            },
        ],
        diagnostics: vec![WorkflowSchedulerTaskResultDiagnostic {
            code: "materialized".to_string(),
            severity: WorkflowSchedulerTaskResultDiagnosticSeverity::Info,
            message: "task result materialized".to_string(),
            port_id: None,
        }],
        terminal_metadata: Some(WorkflowSchedulerTaskResultTerminalMetadata {
            completed_at_ms: Some(42),
            attempt: Some(1),
        }),
    }
}

#[test]
fn scheduler_task_result_validates_path_free_typed_outputs() {
    let result = valid_result();

    result.validate().expect("valid result");
    let encoded = serde_json::to_value(&result).expect("encode result");

    assert_eq!(encoded["schema_version"], 1);
    assert_eq!(
        encoded["outputs"][0]["value"]["value_type"],
        "pumas_model_ref"
    );
    assert_eq!(
        encoded["outputs"][1]["value"]["value"]["artifact_id"],
        "artifact-image-1"
    );
    assert_eq!(encoded.to_string().contains("model_path"), false);
    assert_eq!(encoded.to_string().contains("local_load_path"), false);
    assert_eq!(encoded.to_string().contains("runtime_handoff"), false);
}

#[test]
fn scheduler_task_result_rejects_blank_identity() {
    let mut result = valid_result();
    result.task_id = "  ".to_string();

    let error = result.validate().expect_err("blank task id should fail");

    assert_eq!(
        error,
        WorkflowSchedulerTaskResultError::BlankId { field: "task_id" }
    );
}

#[test]
fn scheduler_task_result_rejects_unknown_path_metadata() {
    let error = serde_json::from_value::<WorkflowSchedulerTaskResult>(json!({
        "schema_version": 1,
        "workflow_id": "workflow-task-results",
        "workflow_run_id": "run-task-results",
        "node_id": "model-node",
        "task_id": "model-node",
        "status": "completed",
        "outputs": [],
        "local_load_path": "/tmp/legacy-model"
    }))
    .expect_err("unknown executable path metadata should be rejected");

    assert!(error.to_string().contains("unknown field"));
}

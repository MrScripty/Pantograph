use super::*;
use crate::workflow::{
    WorkflowExecutionPlan, WorkflowExecutionPlanNodeDecision, WorkflowExecutionSessionRunRequest,
    WorkflowInferenceDeviceClass, WorkflowInferenceTaskId,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};

use super::super::policy::{
    WorkflowExecutionSessionAdmissionRuntimePosture, WorkflowExecutionSessionWarmCompatibility,
};

fn empty_run_request() -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: "ignored".to_string(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: Vec::new(),
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: None,
    }
}

#[test]
fn admission_input_marks_loaded_runtime_reuse_as_incompatible_when_override_diverges() {
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = store
        .create_session(
            "wf-1".to_string(),
            Some("interactive".to_string()),
            None,
            vec!["llama_cpp".to_string()],
            vec!["model-a".to_string()],
            true,
        )
        .expect("create session");
    store
        .mark_runtime_loaded(&session_id, true)
        .expect("mark runtime loaded");

    let mut request = empty_run_request();
    request.override_selection = Some(WorkflowTechnicalFitOverride {
        runtime_id: None,
        runtime_variant_id: None,
        model_id: Some("model-b".to_string()),
        backend_key: Some("pytorch".to_string()),
    });
    let queue_id = store
        .enqueue_run(&session_id, &request)
        .expect("enqueue run");

    let state = store.active.get(&session_id).expect("session state");
    let input = WorkflowExecutionSessionStore::admission_input_from_state(state);
    let candidate = input
        .candidates
        .iter()
        .find(|candidate| candidate.workflow_run_id == queue_id)
        .expect("candidate");

    assert_eq!(
        input.runtime_posture,
        WorkflowExecutionSessionAdmissionRuntimePosture::Loaded
    );
    assert!(!candidate.affine_runtime_reuse);
    assert_eq!(
        candidate.warm_session_compatibility,
        WorkflowExecutionSessionWarmCompatibility::Incompatible
    );
}

#[test]
fn admission_input_marks_loaded_runtime_reuse_as_compatible_without_override_divergence() {
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = store
        .create_session(
            "wf-1".to_string(),
            Some("interactive".to_string()),
            None,
            vec!["llama_cpp".to_string()],
            vec!["model-a".to_string()],
            true,
        )
        .expect("create session");
    store
        .mark_runtime_loaded(&session_id, true)
        .expect("mark runtime loaded");

    let queue_id = store
        .enqueue_run(&session_id, &empty_run_request())
        .expect("enqueue run");

    let state = store.active.get(&session_id).expect("session state");
    let input = WorkflowExecutionSessionStore::admission_input_from_state(state);
    let candidate = input
        .candidates
        .iter()
        .find(|candidate| candidate.workflow_run_id == queue_id)
        .expect("candidate");

    assert!(candidate.affine_runtime_reuse);
    assert_eq!(
        candidate.warm_session_compatibility,
        WorkflowExecutionSessionWarmCompatibility::Compatible
    );
}

#[test]
fn active_run_records_run_scoped_execution_plan() {
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = store
        .create_session(
            "workflow-image-plan".to_string(),
            None,
            None,
            vec!["pytorch".to_string()],
            vec!["stable-diffusion-xl".to_string()],
            true,
        )
        .expect("create session");
    let workflow_run_id = store
        .enqueue_run(&session_id, &empty_run_request())
        .expect("enqueue run");
    store
        .begin_queued_run(&session_id, &workflow_run_id)
        .expect("begin run")
        .expect("dequeued run");

    let execution_plan = WorkflowExecutionPlan::new(
        WorkflowRunId::try_from(workflow_run_id.clone()).expect("valid run id"),
        WorkflowId::try_from("workflow-image-plan".to_string()).expect("valid workflow id"),
        vec![WorkflowExecutionPlanNodeDecision::new(
            "image-node-1",
            "pytorch",
            "pytorch-runtime",
            "pytorch.cuda",
            WorkflowInferenceDeviceClass::Cuda,
            WorkflowInferenceTaskId::ImageGeneration,
        )
        .expect("valid node decision")],
    )
    .expect("valid execution plan");

    store
        .set_active_run_execution_plan(&session_id, &workflow_run_id, execution_plan)
        .expect("record execution plan");

    let active_run = store
        .active
        .get(&session_id)
        .and_then(|session| session.active_run.as_ref())
        .expect("active run");
    assert_eq!(
        active_run
            .execution_plan
            .as_ref()
            .expect("execution plan")
            .workflow_run_id()
            .as_str(),
        workflow_run_id
    );
    assert_eq!(
        store
            .active_run_execution_plan(&session_id, &workflow_run_id)
            .expect("query active plan")
            .expect("execution plan")
            .workflow_run_id()
            .as_str(),
        workflow_run_id
    );
    assert!(store
        .active_run_execution_plan(&session_id, "other-run")
        .expect("query mismatched active plan")
        .is_none());
}

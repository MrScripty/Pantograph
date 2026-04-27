use super::*;
use crate::workflow::WorkflowExecutionSessionRunRequest;

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

use super::*;
use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph,
};
use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_diagnostics_ledger::WorkflowExecutionSessionResumeState;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerRuntimeDeviceConstraints, SchedulerTaskExecutionIntent,
    SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionApplyResult,
    SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
    SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};

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

fn scheduler_task_intent(workflow_run_id: &str, task_id: &str) -> SchedulableTaskIntent {
    SchedulableTaskIntent {
        contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: pantograph_scheduler::SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        fairness_key: None,
        task_type: DependencyTaskId::parse("image_generation").expect("task type"),
        model_ref: PumasModelRef {
            model_id: "image/example/tiny-diffusion".to_string(),
            revision: None,
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
        constraints: SchedulerRuntimeDeviceConstraints::default(),
        trait_settings: Vec::new(),
        dependency_override_patches: Vec::new(),
        estimate_hints: Vec::new(),
    }
}

fn scheduler_transition(
    workflow_run_id: &str,
    task_id: &str,
    transition_id: &str,
    expected_previous_state: Option<SchedulerTaskStateKind>,
    next_state: SchedulerTaskStateKind,
) -> SchedulerTaskStateTransition {
    let task_intent = scheduler_task_intent(workflow_run_id, task_id);
    SchedulerTaskStateTransition {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        transition_id: SchedulerTaskStateTransitionId::parse(transition_id).expect("transition id"),
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: pantograph_scheduler::SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        expected_previous_state,
        next_state: scheduler_state(next_state, task_intent),
    }
}

fn scheduler_record(
    workflow_run_id: &str,
    task_id: &str,
    state: SchedulerTaskStateKind,
) -> SchedulerTaskStateRecord {
    let task_intent = scheduler_task_intent(workflow_run_id, task_id);
    SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: pantograph_scheduler::SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        state: scheduler_state(state, task_intent),
        state_version: 1,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition-existing")
            .expect("transition id"),
    }
}

fn scheduler_task_graph(workflow_run_id: &str, task_ids: &[&str]) -> WorkflowSchedulerTaskGraph {
    let workflow_id = SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id");
    let workflow_run_id = SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id");
    let tasks = task_ids
        .iter()
        .map(|task_id| {
            let task_id = SchedulerTaskId::parse(task_id).expect("task id");
            WorkflowSchedulerTask {
                workflow_id: workflow_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
                node_id: pantograph_scheduler::SchedulerNodeId::parse(task_id.as_str())
                    .expect("node id"),
                task_id,
                node_type: "llm-inference".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                diagnostics: Vec::new(),
            }
        })
        .collect();
    WorkflowSchedulerTaskGraph {
        schema_version: crate::workflow::WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id,
        workflow_run_id,
        tasks,
    }
}

fn scheduler_state(
    state: SchedulerTaskStateKind,
    task_intent: SchedulableTaskIntent,
) -> SchedulerTaskState {
    match state {
        SchedulerTaskStateKind::AwaitingInputs => SchedulerTaskState::AwaitingInputs {
            diagnostics: Vec::new(),
        },
        SchedulerTaskStateKind::InputUnavailable => SchedulerTaskState::InputUnavailable {
            diagnostics: scheduler_state_diagnostics(),
        },
        SchedulerTaskStateKind::Invalid => SchedulerTaskState::Invalid {
            diagnostics: scheduler_state_diagnostics(),
        },
        SchedulerTaskStateKind::Ready => SchedulerTaskState::Ready {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::WaitingDependencyReadiness => {
            SchedulerTaskState::WaitingDependencyReadiness {
                execution_intent: runtime_execution_intent(task_intent),
            }
        }
        SchedulerTaskStateKind::WaitingResources => SchedulerTaskState::WaitingResources {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::WaitingBatch => SchedulerTaskState::WaitingBatch {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::Running => SchedulerTaskState::Running {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::PausedDeferred => SchedulerTaskState::PausedDeferred {
            execution_intent: runtime_execution_intent(task_intent),
            diagnostics: scheduler_state_diagnostics(),
        },
        SchedulerTaskStateKind::RetryableFailed => SchedulerTaskState::RetryableFailed {
            execution_intent: runtime_execution_intent(task_intent),
            diagnostics: scheduler_state_diagnostics(),
        },
        SchedulerTaskStateKind::TerminalFailed => SchedulerTaskState::TerminalFailed {
            diagnostics: scheduler_state_diagnostics(),
        },
        SchedulerTaskStateKind::Completed => SchedulerTaskState::Completed {
            execution_intent: runtime_execution_intent(task_intent),
        },
        _ => panic!("test helper does not support unknown scheduler task state kind"),
    }
}

fn runtime_execution_intent(task_intent: SchedulableTaskIntent) -> SchedulerTaskExecutionIntent {
    SchedulerTaskExecutionIntent::Runtime { task_intent }
}

fn scheduler_state_diagnostics() -> Vec<pantograph_scheduler::SchedulerTaskStateDiagnostic> {
    vec![pantograph_scheduler::SchedulerTaskStateDiagnostic {
        severity: pantograph_scheduler::SchedulerTaskStateDiagnosticSeverity::Error,
        code: pantograph_scheduler::SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
        message: "test scheduler diagnostic".to_string(),
        hint: None,
    }]
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
fn active_run_applies_scheduler_task_state_transitions() {
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

    store
        .set_active_run_scheduler_task_state(
            &session_id,
            &workflow_run_id,
            scheduler_task_graph(&workflow_run_id, &["image-task"]),
            vec![scheduler_record(
                &workflow_run_id,
                "image-task",
                SchedulerTaskStateKind::AwaitingInputs,
            )],
        )
        .expect("set task state");

    let (_, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("task state")
        .expect("active task state");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].state.kind(),
        SchedulerTaskStateKind::AwaitingInputs
    );
    assert_eq!(records[0].state_version, 1);

    let ready = scheduler_transition(
        &workflow_run_id,
        "image-task",
        "transition-ready",
        Some(SchedulerTaskStateKind::AwaitingInputs),
        SchedulerTaskStateKind::Ready,
    );
    let _ready_result = store
        .apply_active_run_scheduler_task_transition(&session_id, &workflow_run_id, ready)
        .expect("ready task transition");
    assert!(matches!(
        _ready_result,
        SchedulerTaskStateTransitionApplyResult::Applied(_)
    ));
    let (_, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("task state after ready")
        .expect("active task state");
    assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Ready);
    assert_eq!(records[0].state_version, 2);
}

#[test]
fn active_run_rejects_task_records_for_different_run() {
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

    let err = store
        .set_active_run_scheduler_task_state(
            &session_id,
            &workflow_run_id,
            scheduler_task_graph(&workflow_run_id, &["image-task"]),
            vec![scheduler_record(
                "run-other",
                "image-task",
                SchedulerTaskStateKind::AwaitingInputs,
            )],
        )
        .expect_err("different run record should fail");
    assert!(err
        .to_string()
        .contains("belongs to workflow run 'run-other'"));
}

#[test]
fn active_run_bootstrap_recovery_snapshot_classifies_runtime_task_states() {
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

    store
        .set_active_run_scheduler_task_state(
            &session_id,
            &workflow_run_id,
            scheduler_task_graph(
                &workflow_run_id,
                &[
                    "awaiting-inputs",
                    "waiting-readiness",
                    "paused-deferred",
                    "retryable-failed",
                    "ready-runtime",
                    "running-runtime",
                    "terminal-failed",
                    "completed-runtime",
                ],
            ),
            vec![
                scheduler_record(
                    &workflow_run_id,
                    "awaiting-inputs",
                    SchedulerTaskStateKind::AwaitingInputs,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "waiting-readiness",
                    SchedulerTaskStateKind::WaitingDependencyReadiness,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "paused-deferred",
                    SchedulerTaskStateKind::PausedDeferred,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "retryable-failed",
                    SchedulerTaskStateKind::RetryableFailed,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "ready-runtime",
                    SchedulerTaskStateKind::Ready,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "running-runtime",
                    SchedulerTaskStateKind::Running,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "terminal-failed",
                    SchedulerTaskStateKind::TerminalFailed,
                ),
                scheduler_record(
                    &workflow_run_id,
                    "completed-runtime",
                    SchedulerTaskStateKind::Completed,
                ),
            ],
        )
        .expect("set task state");

    let snapshot = store
        .active_run_bootstrap_recovery_snapshot(&session_id, &workflow_run_id)
        .expect("bootstrap recovery snapshot")
        .expect("active run snapshot");
    let actions = snapshot
        .runtime_tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task.action))
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(
        actions.get("awaiting-inputs"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::ResumeProgressLoop)
    );
    assert_eq!(
        actions.get("waiting-readiness"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::RetryDependencyReadiness)
    );
    assert_eq!(
        actions.get("paused-deferred"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::RetryDependencyReadiness)
    );
    assert_eq!(
        actions.get("retryable-failed"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::RetryDependencyReadiness)
    );
    assert_eq!(
        actions.get("ready-runtime"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::RedispatchReadyRuntime)
    );
    assert_eq!(
        actions.get("running-runtime"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::RuntimeRecoveryRequired)
    );
    assert_eq!(
        actions.get("terminal-failed"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::TerminalDiagnostic)
    );
    assert_eq!(
        actions.get("completed-runtime"),
        Some(&WorkflowSchedulerBootstrapRecoveryAction::Completed)
    );
}

#[test]
fn active_run_bootstrap_recovery_snapshot_reports_missing_runtime_task_state() {
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

    store
        .set_active_run_scheduler_task_state(
            &session_id,
            &workflow_run_id,
            scheduler_task_graph(&workflow_run_id, &["image-task"]),
            Vec::new(),
        )
        .expect("set task graph without records");

    let snapshot = store
        .active_run_bootstrap_recovery_snapshot(&session_id, &workflow_run_id)
        .expect("bootstrap recovery snapshot")
        .expect("active run snapshot");

    assert_eq!(snapshot.runtime_tasks.len(), 1);
    assert_eq!(snapshot.runtime_tasks[0].task_id, "image-task");
    assert_eq!(snapshot.runtime_tasks[0].state_kind, None);
    assert_eq!(
        snapshot.runtime_tasks[0].action,
        WorkflowSchedulerBootstrapRecoveryAction::MissingTaskStateRecord
    );
}

#[test]
fn dependency_readiness_resume_candidates_use_bootstrap_recovery_classifier() {
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

    store
        .set_active_run_scheduler_task_state(
            &session_id,
            &workflow_run_id,
            scheduler_task_graph(&workflow_run_id, &["image-task"]),
            vec![scheduler_record(
                &workflow_run_id,
                "image-task",
                SchedulerTaskStateKind::RetryableFailed,
            )],
        )
        .expect("set task state");

    assert_eq!(
        store.active_run_dependency_readiness_resume_state(&session_id, &workflow_run_id),
        Some(WorkflowExecutionSessionResumeState::DependencyReadinessPending)
    );
    let candidates = store.dependency_readiness_resume_candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].session_id, session_id);
    assert_eq!(candidates[0].workflow_run_id, workflow_run_id);
}

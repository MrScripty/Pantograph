use pantograph_scheduler::{
    SchedulerBatchDiagnostic, SchedulerBatchDiagnosticCode, SchedulerBatchDiagnosticSeverity,
    SchedulerBatchPolicyDecision, SchedulerBatchPolicyState, SchedulerContractError,
    ValidatedSchedulerBatchPolicyDecision, SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION,
};

#[test]
fn valid_batch_policy_fixture_decodes_and_validates() {
    let decision: SchedulerBatchPolicyDecision =
        serde_json::from_str(include_str!("fixtures/batch_policy_decision_valid.json"))
            .expect("fixture must match scheduler batching policy contract");

    let validated = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect("fixture must validate before dispatch policy consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION
    );
    assert_eq!(validated.as_ref().candidates.len(), 2);
}

#[test]
fn rejects_path_and_worker_launch_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "batching_group_id": "batch.image_generation.001",
        "state": "compatible",
        "max_batch_size": 2,
        "selected_batch_size": 1,
        "total_incremental_memory_bytes": 1,
        "local_load_path": "/models/juggernaut/model.safetensors",
        "worker_command": "python worker.py",
        "candidates": []
    });

    let error = serde_json::from_value::<SchedulerBatchPolicyDecision>(value)
        .expect_err("batch policy decisions must reject path and worker launch fields");

    assert!(
        error
            .to_string()
            .contains("unknown field `local_load_path`")
            || error.to_string().contains("unknown field `worker_command`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_incompatible_runtime() {
    let mut decision = valid_decision();
    decision.candidates[1].selected_runtime_id =
        "other-runtime".parse().expect("test runtime id must parse");
    decision.candidates[1]
        .task_intent
        .constraints
        .requested_runtime_id = Some("other-runtime".parse().expect("test runtime id must parse"));

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch candidates must share runtime");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_runtime_id",
            reason: "batch candidates must share runtime"
        }
    );
}

#[test]
fn selected_runtime_must_satisfy_task_requirement() {
    let mut decision = valid_decision();
    decision.candidates[0].selected_runtime_id =
        "other-runtime".parse().expect("test runtime id must parse");
    decision.candidates[1].selected_runtime_id =
        "other-runtime".parse().expect("test runtime id must parse");

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("candidate selected runtime must satisfy task intent");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "batch_candidate.selected_runtime_id",
            reason: "selected runtime must satisfy task intent runtime requirement"
        }
    );
}

#[test]
fn rejects_incompatible_model_ref() {
    let mut decision = valid_decision();
    decision.candidates[1].selected_model_ref.model_id = "pumas://models/other".to_string();
    decision.candidates[1].task_intent.model_ref.model_id = "pumas://models/other".to_string();

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch candidates must share model ref");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_model_ref",
            reason: "batch candidates must share model ref"
        }
    );
}

#[test]
fn rejects_incompatible_input_shape() {
    let mut decision = valid_decision();
    decision.candidates[1].input_shape_signature = "sdxl.768x768.batch1".to_string();

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch candidates must share input shape");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "input_shape_signature",
            reason: "batch candidates must share input shape"
        }
    );
}

#[test]
fn rejects_duplicate_candidate_task_identity() {
    let mut decision = valid_decision();
    decision.candidates[1].workflow_run_id = decision.candidates[0].workflow_run_id.clone();
    decision.candidates[1].task_id = decision.candidates[0].task_id.clone();
    decision.candidates[1].task_intent.workflow_run_id =
        decision.candidates[0].workflow_run_id.clone();
    decision.candidates[1].task_intent.task_id = decision.candidates[0].task_id.clone();

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch candidates must be unique by workflow run and task");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "batch_policy.candidates",
            reason: "batch candidates must not contain duplicate workflow-run task ids"
        }
    );
}

#[test]
fn rejects_memory_total_mismatch() {
    let mut decision = valid_decision();
    decision.total_incremental_memory_bytes += 1;

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch memory total must match candidate memory impact");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "total_incremental_memory_bytes",
            reason: "batch memory total must equal candidate incremental bytes"
        }
    );
}

#[test]
fn rejected_batch_requires_diagnostics() {
    let mut decision = valid_decision();
    decision.state = SchedulerBatchPolicyState::Rejected;
    decision.diagnostics.clear();

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("rejected batch decisions must explain why");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "batch_policy.diagnostics"
        }
    );
}

#[test]
fn selected_batch_size_must_not_exceed_max() {
    let mut decision = valid_decision();
    decision.selected_batch_size = decision.max_batch_size + 1;

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("selected batch size must fit max batch size");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_batch_size",
            reason: "selected batch size must not exceed max batch size"
        }
    );
}

#[test]
fn selected_batch_size_must_not_exceed_candidate_count() {
    let mut decision = valid_decision();
    decision.max_batch_size = 4;
    decision.selected_batch_size = 3;

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("selected batch size must fit the candidate set");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_batch_size",
            reason: "selected batch size must not exceed candidate count"
        }
    );
}

#[test]
fn rejects_empty_batch_diagnostic_text() {
    let mut decision = valid_decision();
    decision.diagnostics.push(SchedulerBatchDiagnostic {
        severity: SchedulerBatchDiagnosticSeverity::Info,
        code: SchedulerBatchDiagnosticCode::SchedulerBatchPolicyError,
        message: " ".to_string(),
        hint: None,
    });

    let error = ValidatedSchedulerBatchPolicyDecision::try_from(decision)
        .expect_err("batch diagnostics must be bounded non-empty text");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "batch_diagnostic.message"
        }
    );
}

fn valid_decision() -> SchedulerBatchPolicyDecision {
    serde_json::from_str(include_str!("fixtures/batch_policy_decision_valid.json"))
        .expect("fixture must decode")
}

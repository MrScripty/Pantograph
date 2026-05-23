use pantograph_scheduler::{
    SchedulerContractError, SchedulerReadinessAdmissionAction, SchedulerReadinessAdmissionDecision,
    SchedulerReadinessAdmissionDiagnostic, SchedulerReadinessAdmissionDiagnosticCode,
    SchedulerReadinessAdmissionRequest, SchedulerReadinessAdmissionSeverity,
    SchedulerReadinessAdmissionState, ValidatedSchedulerReadinessAdmissionDecision,
    ValidatedSchedulerReadinessAdmissionRequest, SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
};

#[test]
fn valid_admission_request_accepts_schedulable_task_intent() {
    let task_intent =
        serde_json::from_str(include_str!("fixtures/schedulable_task_intent_valid.json"))
            .expect("task intent fixture must decode");
    let request = SchedulerReadinessAdmissionRequest {
        contract_version: SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
        task_intent,
        policy: pantograph_dependency_planning::DependencyReadinessPolicy::CheckOnly,
    };

    let validated = ValidatedSchedulerReadinessAdmissionRequest::try_from(request)
        .expect("valid task intent and readiness policy must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION
    );
}

#[test]
fn valid_ready_decision_fixture_decodes_and_validates() {
    let fixture = include_str!("fixtures/readiness_admission_decision_ready.json");
    let decision: SchedulerReadinessAdmissionDecision =
        serde_json::from_str(fixture).expect("fixture must match readiness admission contract");

    let validated = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect("fixture must validate before runtime host handoff consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().state,
        SchedulerReadinessAdmissionState::Ready
    );
    assert!(validated.as_ref().readiness_proof.is_some());
}

#[test]
fn rejects_path_shaped_readiness_admission_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "model_path": "/models/juggernaut",
        "task_intent": {
            "contract_version": 1,
            "workflow_id": "workflow.image_generation",
            "workflow_run_id": "run.001",
            "node_id": "node.llm_inference",
            "task_id": "task.001",
            "task_type": "image_generation",
            "model_ref": {
                "model_id": "pumas://models/juggernaut-xl-v10"
            }
        },
        "policy": "check_only",
        "action": "defer",
        "state": "deferred",
        "diagnostics": [
            {
                "severity": "info",
                "code": "dependency_not_ready",
                "message": "Dependency readiness has not been checked yet."
            }
        ]
    });

    let error = serde_json::from_value::<SchedulerReadinessAdmissionDecision>(value)
        .expect_err("readiness admission must reject path-shaped dependency identity fields");

    assert!(
        error.to_string().contains("unknown field `model_path`"),
        "unexpected error: {error}"
    );
}

#[test]
fn ready_admission_requires_readiness_proof() {
    let mut decision: SchedulerReadinessAdmissionDecision = serde_json::from_str(include_str!(
        "fixtures/readiness_admission_decision_ready.json"
    ))
    .expect("fixture must decode");
    decision.readiness_proof = None;

    let error = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect_err("ready admission must carry dependency readiness proof");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "readiness_proof"
        }
    );
}

#[test]
fn readiness_proof_must_match_task_intent() {
    let mut decision: SchedulerReadinessAdmissionDecision = serde_json::from_str(include_str!(
        "fixtures/readiness_admission_decision_ready.json"
    ))
    .expect("fixture must decode");
    let proof = decision
        .readiness_proof
        .as_mut()
        .expect("fixture carries proof");
    proof.preflight_result.identity_key.model_ref.model_id = "pumas://models/different".to_string();

    let error = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect_err("readiness proof must match the admitted task intent");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "readiness_proof.preflight_result.identity_key.model_ref",
            reason: "readiness proof model ref must match scheduler task intent"
        }
    );
}

#[test]
fn deferred_admission_requires_diagnostics_and_no_ready_proof() {
    let mut decision: SchedulerReadinessAdmissionDecision = serde_json::from_str(include_str!(
        "fixtures/readiness_admission_decision_ready.json"
    ))
    .expect("fixture must decode");
    decision.state = SchedulerReadinessAdmissionState::Deferred;
    decision.action = SchedulerReadinessAdmissionAction::Defer;
    decision.readiness_proof = None;

    let error = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect_err("deferred admission must explain why the task is not ready");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "readiness_admission.diagnostics"
        }
    );
}

#[test]
fn terminal_failed_admission_must_use_fail_action() {
    let mut decision: SchedulerReadinessAdmissionDecision = serde_json::from_str(include_str!(
        "fixtures/readiness_admission_decision_ready.json"
    ))
    .expect("fixture must decode");
    decision.state = SchedulerReadinessAdmissionState::TerminalFailed;
    decision.action = SchedulerReadinessAdmissionAction::Defer;
    decision.readiness_proof = None;
    decision
        .diagnostics
        .push(SchedulerReadinessAdmissionDiagnostic {
            severity: SchedulerReadinessAdmissionSeverity::Error,
            code: SchedulerReadinessAdmissionDiagnosticCode::DependencyUnavailable,
            message: "Required dependency readiness cannot be satisfied.".to_string(),
            hint: None,
        });

    let error = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect_err("terminal admission failure must use the fail action");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "readiness_admission.action",
            reason: "terminal scheduler admission failure must use fail action"
        }
    );
}

#[test]
fn rejects_unsupported_readiness_admission_contract_version() {
    let mut decision: SchedulerReadinessAdmissionDecision = serde_json::from_str(include_str!(
        "fixtures/readiness_admission_decision_ready.json"
    ))
    .expect("fixture must decode");
    decision.contract_version = 2;

    let error = ValidatedSchedulerReadinessAdmissionDecision::try_from(decision)
        .expect_err("unsupported versions must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler readiness admission contract version"
        }
    );
}

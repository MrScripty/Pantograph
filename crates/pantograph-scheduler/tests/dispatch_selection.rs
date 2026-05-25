use pantograph_scheduler::{
    select_scheduler_dispatch, SchedulerDispatchCandidate, SchedulerDispatchSelectionDecision,
    SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionRequest,
    SchedulerDispatchSelectionState, ValidatedSchedulerDispatchSelectionDecision,
    ValidatedSchedulerDispatchSelectionRequest, SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};

fn valid_request() -> SchedulerDispatchSelectionRequest {
    serde_json::from_str(include_str!(
        "fixtures/dispatch_selection_request_valid.json"
    ))
    .expect("fixture must decode")
}

fn select(request: SchedulerDispatchSelectionRequest) -> SchedulerDispatchSelectionDecision {
    select_scheduler_dispatch(
        ValidatedSchedulerDispatchSelectionRequest::try_from(request)
            .expect("request must validate"),
    )
    .expect("selection should produce a validated decision")
    .into_inner()
}

#[test]
fn valid_dispatch_selection_fixture_decodes_validates_and_selects_candidate() {
    let request = valid_request();
    let validated_request = ValidatedSchedulerDispatchSelectionRequest::try_from(request)
        .expect("fixture must validate before scheduler selection consumes it");

    let decision = select_scheduler_dispatch(validated_request)
        .expect("selection should produce a validated decision");

    assert_eq!(
        decision.as_ref().contract_version,
        SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION
    );
    assert_eq!(
        decision.as_ref().state,
        SchedulerDispatchSelectionState::Selected
    );
    let dispatch_decision = decision
        .as_ref()
        .dispatch_decision
        .as_ref()
        .expect("selected dispatch must carry a dispatch decision");
    assert_eq!(
        dispatch_decision.selected_runtime_id.as_str(),
        "diffusers-pytorch"
    );
    assert_eq!(
        dispatch_decision.reservation_lease_id.as_str(),
        "reservation.001"
    );
}

#[test]
fn selection_decision_fixture_rejects_no_selection_without_diagnostics() {
    let request = valid_request();
    let decision = SchedulerDispatchSelectionDecision {
        contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
        task_intent: request.task_intent,
        state: SchedulerDispatchSelectionState::NoSelection,
        dispatch_decision: None,
        diagnostics: Vec::new(),
    };

    let error = ValidatedSchedulerDispatchSelectionDecision::try_from(decision)
        .expect_err("no-selection decisions must explain why dispatch did not run");

    assert_eq!(
        error,
        pantograph_scheduler::SchedulerContractError::MissingField {
            field: "dispatch_selection.diagnostics"
        }
    );
}

#[test]
fn rejects_path_and_load_target_fields_in_request_or_candidate_payloads() {
    let mut value = serde_json::to_value(valid_request()).expect("request must serialize");
    value["local_load_path"] = serde_json::json!("/models/juggernaut/model.safetensors");

    let error = serde_json::from_value::<SchedulerDispatchSelectionRequest>(value)
        .expect_err("dispatch selection requests must reject path-shaped fields");
    assert!(
        error
            .to_string()
            .contains("unknown field `local_load_path`"),
        "unexpected error: {error}"
    );

    let mut value = serde_json::to_value(valid_request()).expect("request must serialize");
    value["candidates"][0]["load_target"] = serde_json::json!({
        "local_path": "/models/juggernaut"
    });
    let error = serde_json::from_value::<SchedulerDispatchSelectionRequest>(value)
        .expect_err("dispatch candidates must reject executable load-target fields");
    assert!(
        error.to_string().contains("unknown field `load_target`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_technical_fit_candidate_shape_as_dispatch_candidate() {
    let mut value = serde_json::to_value(valid_request()).expect("request must serialize");
    value["candidates"][0]["backend_key"] = serde_json::json!("diffusers");
    value["candidates"][0]["compatibility_issues"] = serde_json::json!([
        {
            "kind": "path",
            "phase": "compatibility",
            "message": "path-shaped compatibility metadata is not dispatch evidence",
            "path": "/models/juggernaut"
        }
    ]);

    let error = serde_json::from_value::<SchedulerDispatchSelectionRequest>(value)
        .expect_err("technical-fit candidate fields are not the dispatch contract");
    assert!(
        error.to_string().contains("unknown field `backend_key`")
            || error
                .to_string()
                .contains("unknown field `compatibility_issues`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_batch_candidate_shape_as_dispatch_candidate() {
    let mut value = serde_json::to_value(valid_request()).expect("request must serialize");
    value["candidates"][0]["task_family"] = serde_json::json!("image_generation");
    value["candidates"][0]["input_shape_signature"] = serde_json::json!("1024x1024");

    let error = serde_json::from_value::<SchedulerDispatchSelectionRequest>(value)
        .expect_err("batch candidate fields are not the dispatch contract");
    assert!(
        error.to_string().contains("unknown field `task_family`")
            || error
                .to_string()
                .contains("unknown field `input_shape_signature`"),
        "unexpected error: {error}"
    );
}

#[test]
fn explicit_runtime_constraint_is_a_hard_requirement() {
    let mut request = valid_request();
    request.candidates[0].selected_runtime_id =
        "other-runtime".parse().expect("test runtime id must parse");

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement
    );
    assert!(decision.dispatch_decision.is_none());
}

#[test]
fn explicit_device_constraint_is_a_hard_requirement() {
    let mut request = valid_request();
    request.candidates[0].selected_device_ids =
        vec!["cuda:1".parse().expect("test device id must parse")];
    request.candidates[0]
        .reservation
        .as_mut()
        .expect("fixture carries reservation")
        .device_id = "cuda:1".parse().expect("test device id must parse");

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::IncompatibleDeviceRequirement
    );
    assert!(decision.dispatch_decision.is_none());
}

#[test]
fn no_candidates_fail_closed_with_typed_diagnostics() {
    let mut request = valid_request();
    request.candidates.clear();

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates
    );
}

#[test]
fn missing_reservation_fact_fails_closed_without_placeholder_lease() {
    let mut request = valid_request();
    request.candidates[0].reservation = None;

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::MissingReservation
    );
    assert!(decision.dispatch_decision.is_none());
}

#[test]
fn missing_resource_fit_fact_fails_closed() {
    let mut request = valid_request();
    request.candidates[0].resource_fit_assessment = None;

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::MissingResourceFit
    );
}

#[test]
fn duplicate_candidate_ids_fail_closed_with_typed_diagnostics() {
    let mut request = valid_request();
    let duplicate = request.candidates[0].clone();
    request.candidates.push(duplicate);

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::DuplicateCandidateId
    );
}

#[test]
fn multiple_eligible_candidates_do_not_fall_back_to_candidate_id_ordering() {
    let mut request = valid_request();
    let mut second: SchedulerDispatchCandidate = request.candidates[0].clone();
    second.candidate_id = "candidate.diffusers.cuda1"
        .parse()
        .expect("candidate id must parse");
    second.selected_device_ids = vec!["cuda:1".parse().expect("device id must parse")];
    second
        .reservation
        .as_mut()
        .expect("fixture carries reservation")
        .reservation_lease_id = "reservation.002"
        .parse()
        .expect("reservation id must parse");
    second
        .reservation
        .as_mut()
        .expect("fixture carries reservation")
        .device_id = "cuda:1".parse().expect("device id must parse");
    request.task_intent.constraints.requested_device_id = None;
    request.candidates.push(second);

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::AmbiguousRanking
    );
    assert!(decision.dispatch_decision.is_none());
}

#[test]
fn candidate_source_diagnostics_do_not_make_missing_facts_authoritative() {
    let mut request = valid_request();
    request.candidates[0].reservation = None;
    let candidate_id = request.candidates[0].candidate_id.clone();
    request.candidates[0].candidate_source_diagnostics.push(
        pantograph_scheduler::SchedulerDispatchSelectionDiagnostic {
            severity: pantograph_scheduler::SchedulerDispatchSelectionDiagnosticSeverity::Info,
            code: SchedulerDispatchSelectionDiagnosticCode::CandidateSelected,
            message: "source claims this candidate is ready".to_string(),
            candidate_id: Some(candidate_id),
            hint: None,
        },
    );

    let decision = select(request);

    assert_eq!(decision.state, SchedulerDispatchSelectionState::NoSelection);
    assert_eq!(
        decision.diagnostics[0].code,
        SchedulerDispatchSelectionDiagnosticCode::MissingReservation
    );
}

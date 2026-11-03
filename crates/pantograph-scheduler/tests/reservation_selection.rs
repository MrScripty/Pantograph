use pantograph_scheduler::{
    select_scheduler_candidate_for_reservation, select_scheduler_dispatch,
    SchedulerDispatchReservationSelection, SchedulerDispatchSelectionDiagnosticCode,
    SchedulerDispatchSelectionRequest, SchedulerDispatchSelectionState,
    ValidatedSchedulerDispatchSelectionRequest,
};

fn offers() -> SchedulerDispatchSelectionRequest {
    let mut request: SchedulerDispatchSelectionRequest = serde_json::from_str(include_str!(
        "fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    for candidate in &mut request.candidates {
        candidate.reservations.clear();
    }
    request
}

#[test]
fn unreserved_choice_cannot_be_used_as_an_executable_dispatch() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers()).unwrap();
    let choice = select_scheduler_candidate_for_reservation(&request);
    assert!(matches!(
        choice,
        SchedulerDispatchReservationSelection::Selected { .. }
    ));
    let dispatch = select_scheduler_dispatch(request).unwrap().into_inner();
    assert_eq!(dispatch.state, SchedulerDispatchSelectionState::NoSelection);
    assert!(dispatch.diagnostics.iter().any(|diagnostic| diagnostic.code
        == SchedulerDispatchSelectionDiagnosticCode::MissingReservation));
}

#[test]
fn ambiguous_and_duplicate_offers_do_not_choose_an_arbitrary_candidate() {
    for duplicate in [false, true] {
        let mut request = offers();
        let mut other = request.candidates[0].clone();
        if !duplicate {
            other.candidate_id = "candidate.other".parse().unwrap();
        }
        request.candidates.push(other);
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(request).unwrap();
        let SchedulerDispatchReservationSelection::NoSelection { diagnostics } =
            select_scheduler_candidate_for_reservation(&request)
        else {
            panic!("must not choose");
        };
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == if duplicate {
                SchedulerDispatchSelectionDiagnosticCode::DuplicateCandidateId
            } else {
                SchedulerDispatchSelectionDiagnosticCode::AmbiguousRanking
            }));
    }
}

#[test]
fn reservation_selection_retains_explicit_constraints_and_missing_fit_rejection() {
    for missing_fit in [false, true] {
        let mut request = offers();
        if missing_fit {
            request.candidates[0].resource_fit_assessment = None;
        } else {
            request.candidates[0].selected_runtime_id = "unrequested-runtime".parse().unwrap();
        }
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(request).unwrap();
        let SchedulerDispatchReservationSelection::NoSelection { diagnostics } =
            select_scheduler_candidate_for_reservation(&request)
        else {
            panic!("must not choose");
        };
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == if missing_fit {
                SchedulerDispatchSelectionDiagnosticCode::MissingResourceFit
            } else {
                SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement
            }));
    }
}

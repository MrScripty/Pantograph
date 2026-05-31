use serde_json::json;

use super::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleContractError, ReservationLifecycleEvent, ReservationLifecycleOutcome,
    ValidatedReservationLifecycleApplication, ValidatedReservationLifecycleEvent,
    RESERVATION_LIFECYCLE_CONTRACT_VERSION,
};

#[test]
fn reservation_lifecycle_event_fixture_decodes_and_validates() {
    let event: ReservationLifecycleEvent = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_event_unselected.json"
    ))
    .expect("reservation lifecycle event fixture must decode");

    let validated =
        ValidatedReservationLifecycleEvent::try_from(event).expect("event must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        RESERVATION_LIFECYCLE_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().outcome,
        ReservationLifecycleOutcome::CandidateUnselected
    );
    assert_eq!(
        validated.as_ref().reservation_lease_id.as_str(),
        "runtime-registry.42"
    );
    assert_eq!(
        validated
            .as_ref()
            .candidate_id
            .as_ref()
            .expect("candidate id")
            .as_str(),
        "candidate.diffusers.cuda0"
    );
}

#[test]
fn reservation_lifecycle_event_rejects_path_shaped_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_event_unselected.json"
    ))
    .expect("reservation lifecycle event fixture must decode as value");
    value["model_path"] = json!("/models/juggernaut");

    let error = serde_json::from_value::<ReservationLifecycleEvent>(value)
        .expect_err("reservation lifecycle event must reject path-shaped fields");

    assert!(
        error.to_string().contains("unknown field `model_path`"),
        "{error}"
    );
}

#[test]
fn reservation_lifecycle_event_requires_diagnostics_for_failure_outcomes() {
    let mut event: ReservationLifecycleEvent = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_event_unselected.json"
    ))
    .expect("reservation lifecycle event fixture must decode");
    event.outcome = ReservationLifecycleOutcome::RuntimeHostFailed;
    event.diagnostics.clear();

    let error = ValidatedReservationLifecycleEvent::try_from(event)
        .expect_err("runtime-host failure lifecycle events must explain failure");

    assert_eq!(
        error,
        ReservationLifecycleContractError::MissingField {
            field: "diagnostics"
        }
    );
}

#[test]
fn reservation_lifecycle_event_rejects_too_many_diagnostics() {
    let mut event: ReservationLifecycleEvent = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_event_unselected.json"
    ))
    .expect("reservation lifecycle event fixture must decode");
    let diagnostic = event
        .diagnostics
        .first()
        .expect("fixture must contain diagnostic")
        .clone();
    event.diagnostics = vec![diagnostic; 65];

    let error = ValidatedReservationLifecycleEvent::try_from(event)
        .expect_err("diagnostic vectors must be bounded");

    assert_eq!(
        error,
        ReservationLifecycleContractError::TooManyDiagnostics {
            actual: 65,
            max: 64
        }
    );
}

#[test]
fn reservation_lifecycle_application_fixture_decodes_and_validates() {
    let application: ReservationLifecycleApplication = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_application_applied.json"
    ))
    .expect("reservation lifecycle application fixture must decode");

    let validated = ValidatedReservationLifecycleApplication::try_from(application)
        .expect("application must validate");

    assert_eq!(
        validated.as_ref().state,
        ReservationLifecycleApplicationState::Applied
    );
    assert_eq!(
        validated.as_ref().reservation_lease_id.as_str(),
        "runtime-registry.42"
    );
}

#[test]
fn reservation_lifecycle_failed_application_requires_diagnostics() {
    let mut application: ReservationLifecycleApplication = serde_json::from_str(include_str!(
        "../tests/fixtures/reservation_lifecycle_application_applied.json"
    ))
    .expect("reservation lifecycle application fixture must decode");
    application.state = ReservationLifecycleApplicationState::Failed;
    application.diagnostics.clear();

    let error = ValidatedReservationLifecycleApplication::try_from(application)
        .expect_err("failed lifecycle applications must explain failure");

    assert_eq!(
        error,
        ReservationLifecycleContractError::MissingField {
            field: "diagnostics"
        }
    );
}

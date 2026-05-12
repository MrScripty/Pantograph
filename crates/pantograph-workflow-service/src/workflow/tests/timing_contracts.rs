use super::*;

#[test]
fn workflow_timing_attempt_record_round_trips_contract_shape() {
    let attempt_id = WorkflowTimingAttemptId::try_from(
        "timing_attempt_550e8400-e29b-41d4-a716-446655440000".to_string(),
    )
    .expect("attempt id");
    let record = WorkflowTimingAttemptRecord::completed(
        attempt_id,
        WorkflowTimingAttemptKind::RuntimeModelLoad,
        WorkflowTimingAttribution {
            workflow_id: Some("workflow-a".to_string()),
            workflow_run_id: Some("run-a".to_string()),
            workflow_execution_session_id: Some("session-a".to_string()),
            runtime_id: Some("llama_cpp".to_string()),
            runtime_variant_id: Some("llama_cpp.cuda".to_string()),
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            device_class: Some("cuda".to_string()),
            device_id: Some("cuda:0".to_string()),
        },
        100,
        175,
    )
    .expect("completed timing record");

    let json = serde_json::to_value(&record).expect("serialize timing record");

    assert_eq!(
        json["attempt_id"],
        "timing_attempt_550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(json["attempt_kind"], "runtime_model_load");
    assert_eq!(json["attribution"]["workflow_run_id"], "run-a");
    assert_eq!(json["attribution"]["runtime_variant_id"], "llama_cpp.cuda");
    assert_eq!(json["attribution"]["device_id"], "cuda:0");
    assert_eq!(json["started_at_ms"], 100);
    assert_eq!(json["completed_at_ms"], 175);
    assert_eq!(json["duration_ms"], 75);

    let parsed: WorkflowTimingAttemptRecord =
        serde_json::from_value(json).expect("deserialize timing record");
    assert_eq!(parsed.duration_ms, Some(75));
}

#[test]
fn workflow_timing_attempt_id_rejects_non_canonical_prefix() {
    let error = WorkflowTimingAttemptId::try_from("attempt-1".to_string())
        .expect_err("non-canonical timing id must fail");

    assert!(matches!(
        error,
        WorkflowTimingContractError::InvalidAttemptIdPrefix
    ));
}

#[test]
fn workflow_timing_duration_rejects_timestamp_underflow() {
    let attempt_id = WorkflowTimingAttemptId::try_from(
        "timing_attempt_550e8400-e29b-41d4-a716-446655440001".to_string(),
    )
    .expect("attempt id");
    let error = checked_timing_duration_ms(&attempt_id, 200, 199)
        .expect_err("duration underflow must fail");

    assert!(matches!(
        &error,
        WorkflowTimingContractError::DurationUnderflow {
            attempt_id: error_attempt_id,
            started_at_ms: 200,
            completed_at_ms: 199,
        } if error_attempt_id == &attempt_id
    ));

    let diagnostic = WorkflowTimingDiagnostic::from_contract_error(
        &error,
        WorkflowTimingAttemptKind::RuntimeWarmup,
    )
    .expect("duration underflow has diagnostic projection");
    assert_eq!(
        diagnostic.code,
        WorkflowTimingDiagnosticCode::TimestampUnderflow
    );
    assert_eq!(diagnostic.severity, WorkflowTimingDiagnosticSeverity::Error);
    assert_eq!(diagnostic.attempt_id, attempt_id);
}

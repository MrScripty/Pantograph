use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, DependencyEnvironmentAction,
    DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentResult,
    DependencyEnvironmentActionIntentStatus, DraftGraphEnqueueDisabledReason,
    DraftGraphValidationStatus, DraftGraphValidationSummary, InferenceAvailabilityStatus,
    InferenceConnectionSurface, InferenceConnectionSurfaceStatus, InferenceDiagnosticCode,
    InferenceDiagnosticSeverity, InferenceInterfaceContractError, InferenceInterfaceDescriptor,
    InferenceInterfaceDiagnostic, InferenceInterfaceDriftReport, InferencePortOptions,
    InferenceValueType, ValidatedAuthoredInferenceInterfaceSnapshot,
    ValidatedDependencyEnvironmentActionIntent, ValidatedDependencyEnvironmentActionIntentResult,
    ValidatedDraftGraphValidationSummary, ValidatedInferenceConnectionSurface,
    ValidatedInferenceInterfaceDescriptor,
};

const DESCRIPTOR: &str = include_str!("fixtures/descriptor_image_generation_ready.json");
const AUTHORED_SNAPSHOT: &str = include_str!("fixtures/authored_snapshot_image_generation.json");
const DRIFT_REPORT: &str = include_str!("fixtures/drift_report_blocking.json");
const VALIDATION_SUMMARY: &str = include_str!("fixtures/validation_summary_blocked.json");
const CONNECTION_SURFACE_CURRENT: &str =
    include_str!("fixtures/connection_surface_image_generation_current.json");
const CONNECTION_SURFACE_DRIFT_BLOCKED: &str =
    include_str!("fixtures/connection_surface_image_generation_drift_blocked.json");

#[test]
fn descriptor_fixture_decodes_validates_and_round_trips() {
    let descriptor: InferenceInterfaceDescriptor =
        serde_json::from_str(DESCRIPTOR).expect("descriptor fixture should decode");

    let validated = ValidatedInferenceInterfaceDescriptor::try_from(descriptor)
        .expect("descriptor fixture should validate");

    assert_eq!(
        validated.as_descriptor().availability.status,
        InferenceAvailabilityStatus::Available
    );
    assert_eq!(validated.as_descriptor().inputs.len(), 3);
    assert!(matches!(
        validated.as_descriptor().inputs[0].value_type,
        InferenceValueType::Scalar(_)
    ));
    assert!(matches!(
        validated.as_descriptor().inputs[2].options,
        InferencePortOptions::Enum { .. }
    ));

    let encoded = serde_json::to_string(validated.as_descriptor())
        .expect("descriptor fixture should re-encode");
    let decoded: InferenceInterfaceDescriptor =
        serde_json::from_str(&encoded).expect("encoded descriptor should decode");
    decoded
        .validate()
        .expect("encoded descriptor should validate");
}

#[test]
fn authored_snapshot_fixture_preserves_saved_graph_shape_only() {
    let snapshot: AuthoredInferenceInterfaceSnapshot =
        serde_json::from_str(AUTHORED_SNAPSHOT).expect("snapshot fixture should decode");

    let validated = ValidatedAuthoredInferenceInterfaceSnapshot::try_from(snapshot)
        .expect("snapshot fixture should validate");

    assert_eq!(validated.as_snapshot().inputs.len(), 2);
    assert_eq!(validated.as_snapshot().outputs.len(), 1);
    assert_eq!(
        validated.as_snapshot().descriptor_fingerprint.as_str(),
        "iface.sd15.text_to_image.v1"
    );

    let raw: serde_json::Value =
        serde_json::from_str(AUTHORED_SNAPSHOT).expect("snapshot fixture should parse");
    assert!(raw.get("model_ref").is_none());
    assert!(raw.get("local_load_path").is_none());
    assert!(raw.get("scheduler_decision").is_none());
}

#[test]
fn drift_report_fixture_carries_blocking_typed_changes() {
    let report: InferenceInterfaceDriftReport =
        serde_json::from_str(DRIFT_REPORT).expect("drift report fixture should decode");

    report
        .validate()
        .expect("drift report fixture should validate");

    assert!(report.blocking);
    assert_eq!(report.changes.len(), 2);
    assert_eq!(report.diagnostics.len(), 1);
}

#[test]
fn validation_summary_fixture_is_enqueue_authority() {
    let summary: DraftGraphValidationSummary =
        serde_json::from_str(VALIDATION_SUMMARY).expect("summary fixture should decode");

    let validated = ValidatedDraftGraphValidationSummary::try_from(summary)
        .expect("summary fixture should validate");

    assert_eq!(
        validated.as_summary().status,
        DraftGraphValidationStatus::Blocked
    );
    assert!(!validated.as_summary().executable);
    assert!(validated
        .as_summary()
        .enqueue_disabled_reasons
        .contains(&DraftGraphEnqueueDisabledReason::DriftRequiresReview));
}

#[test]
fn executable_summary_requires_executable_status() {
    let error = DraftGraphValidationSummary {
        status: DraftGraphValidationStatus::Pending,
        executable: true,
        enqueue_disabled_reasons: Vec::new(),
        diagnostics_count: 0,
        blocking_diagnostics_count: 0,
    }
    .validate()
    .expect_err("pending summaries must not be executable");

    assert_eq!(
        error,
        InferenceInterfaceContractError::InvalidField {
            field: "validation_summary.executable",
            reason: "only executable summaries may set executable true"
        }
    );
}

#[test]
fn dependency_environment_action_intent_carries_only_graph_identity_and_action() {
    let intent: DependencyEnvironmentActionIntent = serde_json::from_value(serde_json::json!({
        "contract_version": 1,
        "graph_session_id": "graph-session-1",
        "graph_revision": "77f4c49c8a1b68d2",
        "validation_session_id": "validation-session-1",
        "target_node_id": "dependency-env-node-1",
        "action": "resolve"
    }))
    .expect("intent should decode");

    let validated = ValidatedDependencyEnvironmentActionIntent::try_from(intent)
        .expect("intent should validate");

    assert_eq!(
        validated.as_intent().action,
        DependencyEnvironmentAction::Resolve
    );
    assert_eq!(
        validated.as_intent().graph_revision.as_str(),
        "77f4c49c8a1b68d2"
    );

    let encoded =
        serde_json::to_value(validated.as_intent()).expect("intent should encode as json");
    assert!(encoded.get("pumas_model_ref").is_none());
    assert!(encoded.get("dependency_environment_request").is_none());
    assert!(encoded.get("dependency_planning_request").is_none());
    assert!(encoded.get("model_path").is_none());
}

#[test]
fn dependency_environment_action_intent_rejects_legacy_or_backend_owned_fields() {
    let error = serde_json::from_value::<DependencyEnvironmentActionIntent>(serde_json::json!({
        "contract_version": 1,
        "graph_session_id": "graph-session-1",
        "graph_revision": "77f4c49c8a1b68d2",
        "target_node_id": "dependency-env-node-1",
        "action": "check",
        "model_path": "/models/sd15",
        "pumas_model_ref": {"model_id": "model-1"},
        "platform_context": {"os": "linux"}
    }))
    .expect_err("intent must deny frontend-built planning or path fields");

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected serde error: {error}"
    );
}

#[test]
fn dependency_environment_action_intent_rejects_blank_revision_and_run_action() {
    let revision_error =
        serde_json::from_value::<DependencyEnvironmentActionIntent>(serde_json::json!({
            "contract_version": 1,
            "graph_session_id": "graph-session-1",
            "graph_revision": " ",
            "target_node_id": "dependency-env-node-1",
            "action": "install"
        }))
        .expect_err("blank graph revisions must fail closed");

    assert!(
        revision_error.to_string().contains("graph_revision"),
        "unexpected serde error: {revision_error}"
    );

    let action_error =
        serde_json::from_value::<DependencyEnvironmentActionIntent>(serde_json::json!({
            "contract_version": 1,
            "graph_session_id": "graph-session-1",
            "graph_revision": "77f4c49c8a1b68d2",
            "target_node_id": "dependency-env-node-1",
            "action": "run"
        }))
        .expect_err("retired run actions must not decode");

    assert!(
        action_error.to_string().contains("unknown variant"),
        "unexpected serde error: {action_error}"
    );
}

#[test]
fn dependency_environment_action_intent_result_requires_blocking_diagnostics() {
    let result = DependencyEnvironmentActionIntentResult {
        contract_version: 1,
        graph_session_id: "graph-session-1".parse().expect("valid graph session id"),
        graph_revision: "77f4c49c8a1b68d2".parse().expect("valid graph revision"),
        validation_session_id: None,
        target_node_id: "dependency-env-node-1"
            .parse()
            .expect("valid target node id"),
        action: DependencyEnvironmentAction::Check,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: Vec::new(),
    };

    let error = result
        .validate()
        .expect_err("blocked results must explain why they are blocked");

    assert_eq!(
        error,
        InferenceInterfaceContractError::MissingField {
            field: "dependency_environment_action_intent_result.diagnostics"
        }
    );
}

#[test]
fn dependency_environment_action_intent_result_preserves_typed_diagnostics() {
    let result = DependencyEnvironmentActionIntentResult {
        contract_version: 1,
        graph_session_id: "graph-session-1".parse().expect("valid graph session id"),
        graph_revision: "77f4c49c8a1b68d2".parse().expect("valid graph revision"),
        validation_session_id: None,
        target_node_id: "dependency-env-node-1"
            .parse()
            .expect("valid target node id"),
        action: DependencyEnvironmentAction::Resolve,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code: InferenceDiagnosticCode::ValidationSummaryMissing,
            message: "Inference validation has not completed for this graph revision.".to_string(),
            hint: Some("Run descriptor validation before resolving dependencies.".to_string()),
            port_id: None,
        }],
    };

    let validated = ValidatedDependencyEnvironmentActionIntentResult::try_from(result)
        .expect("blocked result should validate with diagnostics");

    assert_eq!(
        validated.as_result().diagnostics[0].code,
        InferenceDiagnosticCode::ValidationSummaryMissing
    );
}

#[test]
fn dependency_environment_sidecar_diagnostics_are_typed_contract_codes() {
    let expected = [
        (
            InferenceDiagnosticCode::DependencySidecarTargetWrongType,
            "dependency_sidecar_target_wrong_type",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarAssociationMissing,
            "dependency_sidecar_association_missing",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarAssociationDuplicate,
            "dependency_sidecar_association_duplicate",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
            "dependency_sidecar_association_invalid",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarDescriptorStale,
            "dependency_sidecar_descriptor_stale",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarDescriptorUnavailable,
            "dependency_sidecar_descriptor_unavailable",
        ),
        (
            InferenceDiagnosticCode::DependencySidecarDescriptorInvalid,
            "dependency_sidecar_descriptor_invalid",
        ),
    ];

    for (code, wire_name) in expected {
        let encoded = serde_json::to_value(code).expect("diagnostic code should encode");
        assert_eq!(encoded, serde_json::json!(wire_name));

        let decoded: InferenceDiagnosticCode =
            serde_json::from_value(encoded).expect("diagnostic code should decode");
        assert_eq!(decoded, code);
    }
}

#[test]
fn connection_surface_fixture_decodes_validates_and_round_trips() {
    let surface: InferenceConnectionSurface =
        serde_json::from_str(CONNECTION_SURFACE_CURRENT).expect("surface fixture should decode");

    let validated = ValidatedInferenceConnectionSurface::try_from(surface)
        .expect("surface fixture should validate");

    assert_eq!(
        validated.as_surface().status,
        InferenceConnectionSurfaceStatus::Current
    );
    assert_eq!(
        validated
            .as_surface()
            .descriptor_fingerprint
            .as_ref()
            .expect("current surface has descriptor fingerprint")
            .as_str(),
        "iface.sd15.text_to_image.v1"
    );
    assert_eq!(validated.as_surface().inputs.len(), 2);
    assert_eq!(validated.as_surface().outputs.len(), 1);
    assert!(validated.as_surface().validation_summary.executable);

    let encoded =
        serde_json::to_string(validated.as_surface()).expect("surface fixture should re-encode");
    let decoded: InferenceConnectionSurface =
        serde_json::from_str(&encoded).expect("encoded surface should decode");
    decoded.validate().expect("encoded surface should validate");
}

#[test]
fn connection_surface_fixture_carries_blocking_drift_diagnostics() {
    let surface: InferenceConnectionSurface =
        serde_json::from_str(CONNECTION_SURFACE_DRIFT_BLOCKED)
            .expect("blocked surface fixture should decode");

    let validated = ValidatedInferenceConnectionSurface::try_from(surface)
        .expect("blocked surface fixture should validate");

    assert_eq!(
        validated.as_surface().status,
        InferenceConnectionSurfaceStatus::DriftBlocked
    );
    assert!(!validated.as_surface().validation_summary.executable);
    assert!(
        validated
            .as_surface()
            .drift_report
            .as_ref()
            .expect("blocked surface has drift report")
            .blocking
    );
    assert_eq!(
        validated.as_surface().diagnostics[0].code,
        InferenceDiagnosticCode::InferenceConnectionSurfaceBlocked
    );
}

#[test]
fn connection_surface_current_requires_descriptor_fingerprint() {
    let surface = InferenceConnectionSurface {
        contract_version: 1,
        graph_revision: "77f4c49c8a1b68d2".parse().expect("valid revision"),
        validation_session_id: None,
        node_id: "inference-node-1".parse().expect("valid node id"),
        descriptor_fingerprint: None,
        status: InferenceConnectionSurfaceStatus::Current,
        inputs: Vec::new(),
        outputs: Vec::new(),
        validation_summary: DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Executable,
            executable: true,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        },
        drift_report: None,
        diagnostics: Vec::new(),
    };

    let error = surface
        .validate()
        .expect_err("current surfaces must identify the descriptor");

    assert_eq!(
        error,
        InferenceInterfaceContractError::MissingField {
            field: "connection_surface.descriptor_fingerprint"
        }
    );
}

#[test]
fn connection_surface_rejects_legacy_or_backend_owned_fields() {
    let error = serde_json::from_value::<InferenceConnectionSurface>(serde_json::json!({
        "contract_version": 1,
        "graph_revision": "77f4c49c8a1b68d2",
        "node_id": "inference-node-1",
        "status": "missing",
        "validation_summary": {
            "status": "unresolved",
            "executable": false,
            "enqueue_disabled_reasons": ["descriptor_unresolved"],
            "diagnostics_count": 1,
            "blocking_diagnostics_count": 1
        },
        "diagnostics": [{
            "severity": "error",
            "code": "inference_connection_surface_missing",
            "message": "The inference connection surface has not been resolved."
        }],
        "model_path": "/models/sd15",
        "scheduler_decision": {"device": "cuda:0"},
        "runtime_host_payload": {"task": "generate_image"},
        "frontend_position": {"x": 1, "y": 2}
    }))
    .expect_err("surface must deny paths, scheduler decisions, runtime payloads, and UI state");

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected serde error: {error}"
    );
}

#[test]
fn connection_surface_non_current_requires_diagnostics_and_is_not_executable() {
    let surface = InferenceConnectionSurface {
        contract_version: 1,
        graph_revision: "77f4c49c8a1b68d2".parse().expect("valid revision"),
        validation_session_id: None,
        node_id: "inference-node-1".parse().expect("valid node id"),
        descriptor_fingerprint: Some(
            "iface.sd15.text_to_image.v1"
                .parse()
                .expect("valid fingerprint"),
        ),
        status: InferenceConnectionSurfaceStatus::Stale,
        inputs: Vec::new(),
        outputs: Vec::new(),
        validation_summary: DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Executable,
            executable: true,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        },
        drift_report: None,
        diagnostics: Vec::new(),
    };

    let error = surface
        .validate()
        .expect_err("stale surfaces must fail closed");

    assert_eq!(
        error,
        InferenceInterfaceContractError::InvalidField {
            field: "connection_surface.validation_summary",
            reason: "non-current surfaces must not be executable"
        }
    );
}

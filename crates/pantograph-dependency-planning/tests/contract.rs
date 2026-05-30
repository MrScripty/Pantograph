use pantograph_dependency_planning::{
    dependency_preflight_result_from_environment_result, DependencyEnvironmentAction,
    DependencyEnvironmentFailureState, DependencyEnvironmentInstallState,
    DependencyEnvironmentKind, DependencyEnvironmentReadinessState, DependencyEnvironmentRequest,
    DependencyEnvironmentResult, DependencyEnvironmentValidationState,
    DependencyOperationTimestampMs, DependencyPlanningContractError,
    DependencyPlanningDiagnosticCode, DependencyPlanningIdentityKey,
    DependencyPlanningPlatformContext, DependencyPlanningRequest, DependencyPlanningResult,
    DependencyPlanningState, DependencyPreflightRequest, DependencyPreflightResult,
    DependencyRequirementKind, ModelArtifactKind, PumasArtifactEntryPath,
    PumasArtifactEntryPathError, PumasArtifactLoadPathKind, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyEnvironmentResult, ValidatedDependencyPlanningRequest,
    ValidatedDependencyPreflightRequest, ValidatedDependencyPreflightResult,
};

const VALID_REQUEST: &str = include_str!("fixtures/dependency_planning_request.json");
const READY_RESULT: &str = include_str!("fixtures/dependency_planning_ready_result.json");
const UNAVAILABLE_RESULT: &str =
    include_str!("fixtures/dependency_planning_unavailable_result.json");
const IDENTITY_KEY: &str = include_str!("fixtures/dependency_planning_identity_key.json");
const PREFLIGHT_REQUEST: &str = include_str!("fixtures/dependency_preflight_request.json");
const PREFLIGHT_READY_RESULT: &str =
    include_str!("fixtures/dependency_preflight_ready_result.json");
const PREFLIGHT_UNAVAILABLE_RESULT: &str =
    include_str!("fixtures/dependency_preflight_unavailable_result.json");
const ENV_RESOLVE_REQUEST: &str =
    include_str!("fixtures/dependency_environment_resolve_request.json");
const ENV_CHECK_REQUEST: &str = include_str!("fixtures/dependency_environment_check_request.json");
const ENV_INSTALL_REQUEST: &str =
    include_str!("fixtures/dependency_environment_install_request.json");
const ENV_READY_RESULT: &str = include_str!("fixtures/dependency_environment_ready_result.json");
const ENV_UNAVAILABLE_RESULT: &str =
    include_str!("fixtures/dependency_environment_unavailable_result.json");
const ENV_INVALID_RESULT: &str =
    include_str!("fixtures/dependency_environment_invalid_result.json");
const ENV_NO_BINDING_RESULT: &str =
    include_str!("fixtures/dependency_environment_no_binding_result.json");

#[test]
fn dependency_planning_request_fixture_decodes_and_validates() {
    let request: DependencyPlanningRequest =
        serde_json::from_str(VALID_REQUEST).expect("request fixture should decode");
    let validated = ValidatedDependencyPlanningRequest::try_from(request)
        .expect("request fixture should validate");

    assert_eq!(
        validated.as_request().model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(validated.as_request().task_id.as_str(), "image_generation");
    assert_eq!(
        validated.as_request().expected_artifact_kind,
        Some(ModelArtifactKind::DiffusersBundle)
    );
    assert_eq!(
        validated
            .as_request()
            .platform_context
            .as_ref()
            .map(|context| context.platform_key.as_str()),
        Some("linux-x86_64")
    );
    assert_eq!(
        validated
            .as_request()
            .caller_context
            .source_node_type
            .as_ref()
            .map(|node_type| node_type.as_str()),
        Some("llm-inference")
    );
    assert_eq!(validated.as_request().dependency_override_patches.len(), 1);
}

#[test]
fn dependency_planning_platform_context_derives_stable_os_arch_key() {
    let context = DependencyPlanningPlatformContext::from_os_arch("Linux", "X86_64")
        .expect("os/arch should form a platform key");

    assert_eq!(context.platform_key.as_str(), "linux-x86_64");
}

#[test]
fn dependency_planning_request_rejects_raw_platform_context_json() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        },
        "task_id": "image_generation",
        "platform_context": {
            "os": "linux",
            "arch": "x86_64"
        }
    });

    let error = serde_json::from_value::<DependencyPlanningRequest>(value)
        .expect_err("platform context must use the typed platform_key field");

    assert!(error.to_string().contains("platform_key"));
}

#[test]
fn dependency_planning_result_ready_fixture_requires_load_target() {
    let result: DependencyPlanningResult =
        serde_json::from_str(READY_RESULT).expect("ready result fixture should decode");

    result.validate().expect("ready result should validate");
    assert_eq!(result.state, DependencyPlanningState::Ready);
    let target = result
        .load_target
        .as_ref()
        .expect("ready result should carry load target");
    assert_eq!(target.artifact_kind, ModelArtifactKind::DiffusersBundle);
    assert_eq!(target.load_path_kind, PumasArtifactLoadPathKind::Directory);
}

#[test]
fn dependency_planning_result_unavailable_fixture_has_typed_diagnostics() {
    let result: DependencyPlanningResult =
        serde_json::from_str(UNAVAILABLE_RESULT).expect("unavailable result fixture should decode");

    result
        .validate()
        .expect("unavailable result without load target should validate");
    assert_eq!(result.state, DependencyPlanningState::Unavailable);
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|diagnostic| &diagnostic.code),
        Some(&DependencyPlanningDiagnosticCode::PumasUnavailable)
    );
}

#[test]
fn dependency_planning_request_rejects_empty_pumas_model_id() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": " "
        },
        "task_id": "image_generation"
    });

    let request: DependencyPlanningRequest =
        serde_json::from_value(value).expect("shape should decode before validation");
    let error = ValidatedDependencyPlanningRequest::try_from(request)
        .expect_err("empty model id should fail validation");

    assert_eq!(
        error,
        DependencyPlanningContractError::MissingField {
            field: "pumas_model_ref.model_id"
        }
    );
}

#[test]
fn dependency_planning_request_rejects_raw_local_artifact_entry_paths() {
    let error = PumasArtifactEntryPath::parse("/models/tiny-sd")
        .expect_err("absolute local path should not be a Pumas entry path");

    assert_eq!(error, PumasArtifactEntryPathError::LocalPath);
}

#[test]
fn ready_result_without_target_is_invalid() {
    let result: DependencyPlanningResult = serde_json::from_value(serde_json::json!({
        "state": "ready",
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        }
    }))
    .expect("ready shape should decode");

    assert_eq!(
        result.validate().expect_err("ready result requires target"),
        DependencyPlanningContractError::ReadyResultMissingLoadTarget
    );
}

#[test]
fn dependency_planning_identity_key_fixture_decodes_and_validates() {
    let identity_key: DependencyPlanningIdentityKey =
        serde_json::from_str(IDENTITY_KEY).expect("identity key fixture should decode");

    identity_key
        .validate()
        .expect("path-free identity key should validate");
    assert_eq!(
        identity_key.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        identity_key.model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
    assert_eq!(identity_key.model_ref.selected_artifact_path, None);
    assert_eq!(identity_key.task_id.as_str(), "image_generation");
    assert_eq!(
        identity_key
            .scheduler_intent
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("pytorch")
    );
    assert_eq!(
        identity_key
            .scheduler_intent
            .requested_device_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("cuda:0")
    );
}

#[test]
fn dependency_preflight_request_fixture_decodes_and_validates() {
    let value: serde_json::Value =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("preflight request fixture should parse");
    let validated = ValidatedDependencyPreflightRequest::try_from(value)
        .expect("preflight request fixture should validate");

    assert_eq!(validated.as_request().contract_version, 1);
    assert_eq!(
        validated.as_request().identity_key.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        validated
            .as_request()
            .dependency_requirements_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("tiny-sd:pytorch:linux-x86_64:torch-diffusers")
    );
    assert!(validated.as_request().environment_ref.is_some());
}

#[test]
fn dependency_preflight_result_fixtures_decode_and_validate() {
    let ready_value: serde_json::Value = serde_json::from_str(PREFLIGHT_READY_RESULT)
        .expect("preflight ready result fixture should parse");
    let ready = ValidatedDependencyPreflightResult::try_from(ready_value)
        .expect("preflight ready result fixture should validate");

    assert_eq!(
        ready.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(ready.as_result().diagnostics.len(), 1);

    let unavailable_value: serde_json::Value = serde_json::from_str(PREFLIGHT_UNAVAILABLE_RESULT)
        .expect("preflight unavailable result fixture should parse");
    let unavailable = ValidatedDependencyPreflightResult::try_from(unavailable_value)
        .expect("preflight unavailable result fixture should validate");

    assert_eq!(
        unavailable.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        unavailable
            .as_result()
            .diagnostics
            .iter()
            .map(|diagnostic| &diagnostic.code)
            .collect::<Vec<_>>(),
        vec![
            &DependencyPlanningDiagnosticCode::PumasUnavailable,
            &DependencyPlanningDiagnosticCode::RuntimeUnavailable
        ]
    );
}

#[test]
fn dependency_preflight_projection_preserves_ready_environment_identity() {
    let environment_result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("ready environment result should decode");
    let environment_result = ValidatedDependencyEnvironmentResult::try_from(environment_result)
        .expect("ready environment result should validate");

    let preflight_result = dependency_preflight_result_from_environment_result(&environment_result)
        .expect("ready environment result should project to preflight proof");
    let preflight_result = preflight_result.as_result();

    assert_eq!(
        preflight_result.readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(
        preflight_result.identity_key,
        environment_result.as_result().identity_key
    );
    assert_eq!(
        preflight_result.dependency_requirements_id,
        environment_result.as_result().dependency_requirements_id
    );
    assert_eq!(
        preflight_result.environment_ref,
        environment_result.as_result().environment_ref
    );
    assert!(preflight_result.diagnostics.is_empty());
}

#[test]
fn dependency_preflight_projection_preserves_unavailable_diagnostics() {
    let environment_result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_UNAVAILABLE_RESULT)
            .expect("unavailable environment result should decode");
    let environment_result = ValidatedDependencyEnvironmentResult::try_from(environment_result)
        .expect("unavailable environment result should validate");

    let preflight_result = dependency_preflight_result_from_environment_result(&environment_result)
        .expect("unavailable environment result should project to preflight proof");
    let preflight_result = preflight_result.as_result();

    assert_eq!(
        preflight_result.readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        preflight_result.identity_key,
        environment_result.as_result().identity_key
    );
    assert_eq!(
        preflight_result.diagnostics,
        environment_result.as_result().diagnostics
    );
    assert!(preflight_result.environment_ref.is_none());
}

#[test]
fn dependency_preflight_request_rejects_load_target_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "identity_key": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation"
        },
        "load_target": {
            "local_load_path": "/models/tiny-sd"
        }
    });

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(value)
            .expect_err("preflight identity must reject load target fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_preflight",
            reason: "preflight payload must not contain path-shaped dependency identity fields"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_missing_environment_identity() {
    let mut request: DependencyPreflightRequest =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should decode");
    request.environment_ref = None;

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(request)
            .expect_err("preflight request requires dependency environment identity"),
        DependencyPlanningContractError::MissingField {
            field: "environment_ref"
        }
    );
}

#[test]
fn dependency_preflight_ready_result_rejects_missing_environment_identity() {
    let mut result: DependencyPreflightResult =
        serde_json::from_str(PREFLIGHT_READY_RESULT).expect("fixture should decode");
    result.environment_ref = None;

    assert_eq!(
        ValidatedDependencyPreflightResult::try_from(result)
            .expect_err("ready preflight result requires dependency environment identity"),
        DependencyPlanningContractError::MissingField {
            field: "environment_ref"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_duplicate_selected_binding_ids() {
    let mut request: DependencyPreflightRequest =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should decode");
    request
        .identity_key
        .selected_binding_ids
        .push("torch-diffusers".parse().expect("test binding id is valid"));

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(request)
            .expect_err("selected binding ids must be unique"),
        DependencyPlanningContractError::InvalidField {
            field: "identity_key.selected_binding_ids",
            reason: "selected binding ids must be unique"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_malformed_selected_binding_ids() {
    let mut value: serde_json::Value =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should parse");
    value["identity_key"]["selected_binding_ids"] = serde_json::json!(["bad/binding"]);

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(value)
            .expect_err("selected binding ids must be validated ids"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_preflight_request",
            reason: "request JSON did not match dependency preflight contract"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_legacy_selected_runtime_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should parse");
    value["identity_key"]["selected_runtime_id"] = serde_json::json!("pytorch");

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(value)
            .expect_err("preflight identity must not accept legacy selected runtime fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_preflight_request",
            reason: "request JSON did not match dependency preflight contract"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_nested_path_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should parse");
    value["planning_request"]["model_ref"]["selected_artifact_path"] =
        serde_json::json!("image/stable-diffusion/tiny-sd/model_index.json");

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(value)
            .expect_err("preflight request must reject nested path-shaped fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_preflight",
            reason: "preflight payload must not contain path-shaped dependency identity fields"
        }
    );
}

#[test]
fn dependency_preflight_request_rejects_package_fact_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(PREFLIGHT_REQUEST).expect("fixture should parse");
    value["resolved_model_package_facts"] = serde_json::json!({
        "package_facts_contract_version": 1,
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        }
    });

    assert_eq!(
        ValidatedDependencyPreflightRequest::try_from(value)
            .expect_err("preflight request must reject executable package fact fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_preflight",
            reason: "preflight payload must not contain executable dependency handoff fields"
        }
    );
}

#[test]
fn dependency_planning_identity_key_rejects_model_path_fields() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        },
        "task_id": "image_generation",
        "model_path": "/models/tiny-sd"
    });

    serde_json::from_value::<DependencyPlanningIdentityKey>(value)
        .expect_err("identity key must not deserialize model_path fields");
}

#[test]
fn dependency_planning_identity_key_rejects_selected_artifact_path_identity() {
    let identity_key: DependencyPlanningIdentityKey = serde_json::from_value(serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd",
            "selected_artifact_path": "image/stable-diffusion/tiny-sd/model_index.json"
        },
        "task_id": "image_generation"
    }))
    .expect("shape decodes before path-free validation");

    assert_eq!(
        identity_key
            .validate()
            .expect_err("selected artifact path is not path-free identity"),
        DependencyPlanningContractError::InvalidField {
            field: "pumas_model_ref.selected_artifact_path",
            reason: "path-free dependency identity must not carry selected artifact paths"
        }
    );
}

#[test]
fn dependency_environment_request_fixtures_decode_and_validate() {
    let cases = [
        (
            ENV_RESOLVE_REQUEST,
            DependencyEnvironmentAction::Resolve,
            false,
        ),
        (ENV_CHECK_REQUEST, DependencyEnvironmentAction::Check, true),
        (
            ENV_INSTALL_REQUEST,
            DependencyEnvironmentAction::Install,
            true,
        ),
    ];

    for (fixture, expected_action, requires_requirements_id) in cases {
        let request: DependencyEnvironmentRequest =
            serde_json::from_str(fixture).expect("environment request fixture should decode");
        let validated = ValidatedDependencyEnvironmentRequest::try_from(request)
            .expect("environment request fixture should validate");

        assert_eq!(validated.as_request().action, expected_action);
        assert_eq!(
            validated.as_request().identity_key.model_ref.model_id,
            "image/stable-diffusion/tiny-sd"
        );
        assert_eq!(
            validated
                .as_request()
                .identity_key
                .scheduler_intent
                .requested_runtime_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("pytorch")
        );
        assert_eq!(
            validated.as_request().dependency_requirements_id.is_some(),
            requires_requirements_id
        );
    }
}

#[test]
fn dependency_environment_result_fixtures_decode_and_validate() {
    let ready: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("ready environment result should decode");
    ready
        .validate()
        .expect("ready environment result should validate");
    assert_eq!(
        ready.readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(
        ready.install_state,
        DependencyEnvironmentInstallState::Installed
    );
    assert_eq!(
        ready.validation_state,
        DependencyEnvironmentValidationState::Valid
    );
    assert!(ready.environment_ref.is_some());
    assert_eq!(ready.requirements.len(), 1);
    assert_eq!(
        ready.requirements[0].kind,
        DependencyRequirementKind::PythonPackage
    );
    assert!(ready.requirements[0].python.is_some());
    assert_eq!(ready.bindings.len(), 1);
    assert_eq!(
        ready.bindings[0].environment_kind,
        DependencyEnvironmentKind::Python
    );
    assert_eq!(ready.selected_binding_ids.len(), 1);
    assert_eq!(ready.binding_statuses.len(), 1);
    assert!(ready.operation.is_some());

    let unavailable: DependencyEnvironmentResult = serde_json::from_str(ENV_UNAVAILABLE_RESULT)
        .expect("unavailable environment result should decode");
    unavailable
        .validate()
        .expect("unavailable environment result should validate");
    assert_eq!(
        unavailable.failure_state,
        Some(DependencyEnvironmentFailureState::RequirementsUnavailable)
    );
    assert_eq!(unavailable.diagnostics.len(), 1);

    let invalid: DependencyEnvironmentResult =
        serde_json::from_str(ENV_INVALID_RESULT).expect("invalid environment result should decode");
    invalid
        .validate()
        .expect("invalid environment result should validate");
    assert_eq!(
        invalid.failure_state,
        Some(DependencyEnvironmentFailureState::InvalidRequest)
    );
    assert_eq!(
        invalid
            .diagnostics
            .first()
            .map(|diagnostic| &diagnostic.code),
        Some(&DependencyPlanningDiagnosticCode::InvalidRequest)
    );
    assert_eq!(invalid.binding_statuses.len(), 1);
    assert_eq!(invalid.validation_errors.len(), 1);

    let no_binding: DependencyEnvironmentResult =
        serde_json::from_str(ENV_NO_BINDING_RESULT).expect("no-binding result should decode");
    no_binding
        .validate()
        .expect("no-binding unavailable result should validate");
    assert!(no_binding.selected_binding_ids.is_empty());
    assert_eq!(
        no_binding.failure_state,
        Some(DependencyEnvironmentFailureState::RequirementsUnavailable)
    );
}

#[test]
fn dependency_environment_check_request_requires_requirements_id() {
    let request: DependencyEnvironmentRequest =
        serde_json::from_str(ENV_RESOLVE_REQUEST).expect("fixture should decode");
    let request = DependencyEnvironmentRequest {
        action: DependencyEnvironmentAction::Check,
        ..request
    };

    assert_eq!(
        ValidatedDependencyEnvironmentRequest::try_from(request)
            .expect_err("check request should require dependency requirements id"),
        DependencyPlanningContractError::MissingField {
            field: "dependency_requirements_id"
        }
    );
}

#[test]
fn dependency_environment_request_rejects_path_shaped_json_fields() {
    let value = serde_json::json!({
        "action": "resolve",
        "identity_key": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation"
        },
        "planning_request": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation",
            "model_path": "/models/tiny-sd"
        }
    });

    assert_eq!(
        ValidatedDependencyEnvironmentRequest::try_from(value)
            .expect_err("environment requests must reject path-shaped identity fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_request",
            reason: "request must not contain path-shaped dependency identity fields"
        }
    );
}

#[test]
fn dependency_environment_request_rejects_unknown_mode_field() {
    let value: serde_json::Value =
        serde_json::from_str(ENV_RESOLVE_REQUEST).expect("fixture should parse");
    let mut object = value
        .as_object()
        .expect("fixture should be an object")
        .clone();
    object.insert("mode".to_string(), serde_json::json!("auto"));

    assert_eq!(
        ValidatedDependencyEnvironmentRequest::try_from(serde_json::Value::Object(object))
            .expect_err("environment requests must use typed action, not raw mode"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_request",
            reason: "request JSON did not match dependency environment contract"
        }
    );
}

#[test]
fn dependency_environment_request_rejects_mismatched_identity_key() {
    let mut request: DependencyEnvironmentRequest =
        serde_json::from_str(ENV_RESOLVE_REQUEST).expect("fixture should decode");
    request.identity_key.task_id = "text_generation"
        .parse()
        .expect("test task id should be valid");

    assert_eq!(
        ValidatedDependencyEnvironmentRequest::try_from(request)
            .expect_err("identity key must match planning request"),
        DependencyPlanningContractError::InvalidField {
            field: "identity_key.task_id",
            reason: "identity key task id must match planning request task id"
        }
    );
}

#[test]
fn dependency_environment_request_rejects_malformed_environment_ids() {
    let value = serde_json::json!({
        "action": "check",
        "identity_key": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation"
        },
        "planning_request": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation"
        },
        "dependency_requirements_id": "tiny-sd:pytorch:linux-x86_64",
        "environment_ref": {
            "environment_id": "python/pytorch"
        }
    });

    serde_json::from_value::<DependencyEnvironmentRequest>(value)
        .expect_err("environment id must be a validated id, not a path");
}

#[test]
fn dependency_environment_result_rejects_unknown_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should parse");
    value
        .as_object_mut()
        .expect("fixture should be an object")
        .insert("legacy_status".to_string(), serde_json::json!("ready"));

    serde_json::from_value::<DependencyEnvironmentResult>(value)
        .expect_err("environment results must reject unknown legacy fields");
}

#[test]
fn dependency_environment_result_rejects_duplicate_selected_binding_ids() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result
        .selected_binding_ids
        .push("diffusers.scheduler".parse().expect("valid binding id"));

    assert_eq!(
        result
            .validate()
            .expect_err("selected binding ids must be unique"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_result.selected_binding_ids",
            reason: "selected binding ids must be unique"
        }
    );
}

#[test]
fn dependency_environment_result_rejects_invalid_operation_timestamps() {
    serde_json::from_value::<DependencyOperationTimestampMs>(serde_json::json!(0))
        .expect_err("zero is not a valid operation timestamp");

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    let operation = result
        .operation
        .as_mut()
        .expect("ready fixture includes operation timing");
    operation.started_at_ms = Some(DependencyOperationTimestampMs::parse(200).unwrap());
    operation.completed_at_ms = Some(DependencyOperationTimestampMs::parse(100).unwrap());

    assert_eq!(
        result
            .validate()
            .expect_err("completion cannot precede start"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_operation.completed_at_ms",
            reason: "operation completion timestamp must not be earlier than start timestamp"
        }
    );
}

#[test]
fn dependency_environment_result_rejects_path_shaped_validation_fields() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_INVALID_RESULT).expect("fixture should decode");
    result
        .diagnostics
        .first_mut()
        .expect("invalid fixture has diagnostic")
        .field_path = Some("/tmp/model".to_string());

    assert_eq!(
        result
            .validate()
            .expect_err("diagnostic field path must not be a filesystem path"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_diagnostic.field_path",
            reason: "validation field paths must be contract fields, not filesystem paths"
        }
    );
}

#[test]
fn dependency_environment_result_rejects_python_details_on_non_python_rows() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::RuntimeManagedBinary;

    assert_eq!(
        result
            .validate()
            .expect_err("python details are requirement-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_requirement.python",
            reason: "python details are allowed only for python package requirements"
        }
    );

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.bindings[0].environment_kind = DependencyEnvironmentKind::ManagedBinary;

    assert_eq!(
        result
            .validate()
            .expect_err("python details are binding-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_binding.python",
            reason: "python details are allowed only for python environment bindings"
        }
    );
}

#[test]
fn dependency_environment_result_accepts_typed_non_python_detail_rows() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::RuntimeManagedBinary;
    result.requirements[0].python = None;
    result.requirements[0].managed_runtime = Some(
        serde_json::from_value(serde_json::json!({
            "managed_binary_id": "llama_cpp",
            "runtime_variant_id": "llama_cpp:cpu",
            "version": "b8248",
            "platform_key": "linux-x86_64"
        }))
        .expect("managed runtime details"),
    );
    result.bindings[0].environment_kind = DependencyEnvironmentKind::ManagedBinary;
    result.bindings[0].python = None;
    result.bindings[0].managed_runtime = Some(
        serde_json::from_value(serde_json::json!({
            "managed_binary_id": "llama_cpp",
            "runtime_variant_id": "llama_cpp:cpu",
            "selected_version": "b8248",
            "platform_key": "linux-x86_64"
        }))
        .expect("managed runtime binding details"),
    );

    result
        .validate()
        .expect("managed runtime detail rows should validate");

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::RuntimeFeature;
    result.requirements[0].python = None;
    result.requirements[0].runtime_feature = Some(
        serde_json::from_value(serde_json::json!({
            "runtime_id": "pytorch",
            "feature_id": "attention_slicing",
            "runtime_variant_id": "pytorch:cuda"
        }))
        .expect("runtime feature details"),
    );
    result.bindings[0].environment_kind = DependencyEnvironmentKind::RuntimeFeature;
    result.bindings[0].python = None;
    result.bindings[0].runtime_feature = Some(
        serde_json::from_value(serde_json::json!({
            "runtime_id": "pytorch",
            "feature_id": "attention_slicing",
            "runtime_variant_id": "pytorch:cuda"
        }))
        .expect("runtime feature binding details"),
    );

    result
        .validate()
        .expect("runtime feature detail rows should validate");

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::DeviceToolchain;
    result.requirements[0].python = None;
    result.requirements[0].device_toolchain = Some(
        serde_json::from_value(serde_json::json!({
            "toolchain_id": "cuda_toolkit",
            "device_id": "cuda:0",
            "runtime_id": "pytorch"
        }))
        .expect("device toolchain details"),
    );
    result.bindings[0].environment_kind = DependencyEnvironmentKind::DeviceToolchain;
    result.bindings[0].python = None;
    result.bindings[0].device_toolchain = Some(
        serde_json::from_value(serde_json::json!({
            "toolchain_id": "cuda_toolkit",
            "device_id": "cuda:0",
            "runtime_id": "pytorch"
        }))
        .expect("device toolchain binding details"),
    );

    result
        .validate()
        .expect("device toolchain detail rows should validate");

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::SystemPackage;
    result.requirements[0].python = None;
    result.requirements[0].system_package = Some(
        serde_json::from_value(serde_json::json!({
            "package_id": "libcuda",
            "package_manager_id": "apt",
            "platform_id": "linux-x86_64",
            "architecture": "x86_64"
        }))
        .expect("system package details"),
    );
    result.bindings[0].environment_kind = DependencyEnvironmentKind::SystemPackage;
    result.bindings[0].python = None;
    result.bindings[0].system_package = Some(
        serde_json::from_value(serde_json::json!({
            "package_id": "libcuda",
            "package_manager_id": "apt",
            "platform_id": "linux-x86_64",
            "architecture": "x86_64"
        }))
        .expect("system package binding details"),
    );

    result
        .validate()
        .expect("system package detail rows should validate");
}

#[test]
fn dependency_environment_result_requires_typed_details_for_supported_non_python_rows() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::RuntimeManagedBinary;
    result.requirements[0].python = None;

    assert_eq!(
        result
            .validate()
            .expect_err("managed runtime requirements need typed details"),
        DependencyPlanningContractError::MissingField {
            field: "dependency_requirement.managed_runtime"
        }
    );

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].kind = DependencyRequirementKind::SystemPackage;
    result.requirements[0].python = None;

    assert_eq!(
        result
            .validate()
            .expect_err("system package requirements need typed details"),
        DependencyPlanningContractError::MissingField {
            field: "dependency_requirement.system_package"
        }
    );
}

#[test]
fn dependency_environment_result_rejects_mismatched_non_python_detail_rows() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].managed_runtime = Some(
        serde_json::from_value(serde_json::json!({
            "managed_binary_id": "llama_cpp"
        }))
        .expect("managed runtime details"),
    );

    assert_eq!(
        result
            .validate()
            .expect_err("managed runtime details are requirement-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_requirement.managed_runtime",
            reason:
                "managed runtime details are allowed only for runtime managed binary requirements"
        }
    );

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.requirements[0].system_package = Some(
        serde_json::from_value(serde_json::json!({
            "package_id": "libcuda",
            "package_manager_id": "apt"
        }))
        .expect("system package details"),
    );

    assert_eq!(
        result
            .validate()
            .expect_err("system package details are requirement-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_requirement.system_package",
            reason: "system package details are allowed only for system package requirements"
        }
    );

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.bindings[0].device_toolchain = Some(
        serde_json::from_value(serde_json::json!({
            "toolchain_id": "cuda_toolkit"
        }))
        .expect("device toolchain binding details"),
    );

    assert_eq!(
        result
            .validate()
            .expect_err("device toolchain details are binding-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_binding.device_toolchain",
            reason: "device toolchain details are allowed only for device toolchain bindings"
        }
    );

    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should decode");
    result.bindings[0].system_package = Some(
        serde_json::from_value(serde_json::json!({
            "package_id": "libcuda",
            "package_manager_id": "apt"
        }))
        .expect("system package binding details"),
    );

    assert_eq!(
        result
            .validate()
            .expect_err("system package details are binding-kind scoped"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_binding.system_package",
            reason: "system package details are allowed only for system package bindings"
        }
    );
}

#[test]
fn dependency_environment_result_rejects_unknown_non_python_detail_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should parse");
    let requirement = value["requirements"][0]
        .as_object_mut()
        .expect("requirement should be an object");
    requirement.insert(
        "managed_runtime".to_string(),
        serde_json::json!({
            "managed_binary_id": "llama_cpp",
            "legacy_path": "/usr/bin/llama.cpp"
        }),
    );

    serde_json::from_value::<DependencyEnvironmentResult>(value)
        .expect_err("managed runtime details must reject unknown legacy fields");

    let mut value: serde_json::Value =
        serde_json::from_str(ENV_READY_RESULT).expect("fixture should parse");
    let requirement = value["requirements"][0]
        .as_object_mut()
        .expect("requirement should be an object");
    requirement.insert(
        "system_package".to_string(),
        serde_json::json!({
            "package_id": "libcuda",
            "package_manager_id": "apt",
            "package_name": "libcuda1"
        }),
    );

    serde_json::from_value::<DependencyEnvironmentResult>(value)
        .expect_err("system package details must reject package-name legacy fields");
}

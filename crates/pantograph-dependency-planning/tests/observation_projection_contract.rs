use pantograph_dependency_planning::{
    dependency_environment_result_from_inventory_observations, DependencyBindingId,
    DependencyBindingStatusState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentValidationState, DependencyInventoryObservationProjection,
    DependencyInventoryObservationState, DependencyPlanningContractError,
    DependencyPlanningDiagnostic, DependencyPlanningDiagnosticCode, DependencyPlanningSeverity,
    ValidatedDependencyInventoryObservationProjection,
};

const MIXED_READY_PROJECTION: &str =
    include_str!("fixtures/dependency_inventory_observation_projection_mixed_ready.json");

#[test]
fn observation_projection_fixture_decodes_and_projects_mixed_ready_result() {
    let projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    let projection = ValidatedDependencyInventoryObservationProjection::try_from(projection)
        .expect("projection fixture should validate");

    let result = dependency_environment_result_from_inventory_observations(&projection)
        .expect("projection should build a validated result")
        .into_inner();

    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(result.binding_statuses.len(), 2);
    assert!(result
        .binding_statuses
        .iter()
        .all(|status| status.state == DependencyBindingStatusState::Ready));
}

#[test]
fn observation_projection_requires_one_observation_for_each_selected_binding() {
    let mut projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    projection.observations.pop();

    assert_eq!(
        ValidatedDependencyInventoryObservationProjection::try_from(projection)
            .expect_err("missing selected-binding observation should fail"),
        DependencyPlanningContractError::MissingField {
            field: "dependency_inventory_observation"
        }
    );
}

#[test]
fn observation_projection_rejects_observations_for_unselected_bindings() {
    let mut projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    projection.observations[0].binding_id =
        DependencyBindingId::parse("unselected.binding").expect("binding id");

    assert_eq!(
        ValidatedDependencyInventoryObservationProjection::try_from(projection)
            .expect_err("unselected binding observations should fail"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_inventory_observation.binding_id",
            reason: "observation binding id must reference a selected binding"
        }
    );
}

#[test]
fn observation_projection_rejects_duplicate_observation_rows() {
    let mut projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    projection.observations[1].binding_id = projection.observations[0].binding_id.clone();

    assert_eq!(
        ValidatedDependencyInventoryObservationProjection::try_from(projection)
            .expect_err("duplicate observation rows should fail"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_inventory_observation.binding_id",
            reason: "observation binding ids must be unique"
        }
    );
}

#[test]
fn observation_projection_rejects_unknown_legacy_observation_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should parse");
    let observation = value["observations"][0]
        .as_object_mut()
        .expect("observation should be an object");
    observation.insert(
        "legacy_probe_path".to_string(),
        serde_json::json!("/tmp/probe"),
    );

    ValidatedDependencyInventoryObservationProjection::try_from(value)
        .expect_err("observation contract should reject legacy fields");
}

#[test]
fn observation_projection_requires_stale_observations_to_carry_diagnostics() {
    let mut projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    projection.observations[0].validation_state = DependencyEnvironmentValidationState::Stale;
    projection.observations[0].freshness =
        pantograph_dependency_planning::DependencyInventoryObservationFreshness::Stale;

    assert_eq!(
        ValidatedDependencyInventoryObservationProjection::try_from(projection)
            .expect_err("stale observations should explain their staleness"),
        DependencyPlanningContractError::MissingField {
            field: "dependency_inventory_observation.diagnostics"
        }
    );
}

#[test]
fn observation_projection_projects_not_implemented_with_provider_diagnostics() {
    let mut projection: DependencyInventoryObservationProjection =
        serde_json::from_str(MIXED_READY_PROJECTION).expect("projection fixture should decode");
    projection.observations[1].state = DependencyInventoryObservationState::NotImplemented;
    projection.observations[1].validation_state =
        DependencyEnvironmentValidationState::NotImplemented;
    projection.observations[1].diagnostics = vec![DependencyPlanningDiagnostic {
        code: DependencyPlanningDiagnosticCode::NotImplemented,
        severity: DependencyPlanningSeverity::Error,
        message: "Managed runtime inventory provider is not implemented.".to_string(),
        model_id: None,
        runtime_id: None,
        device_id: None,
        field_path: Some("dependency_inventory.managed_runtime".to_string()),
    }];
    let projection = ValidatedDependencyInventoryObservationProjection::try_from(projection)
        .expect("not-implemented projection should validate with diagnostics");

    let result = dependency_environment_result_from_inventory_observations(&projection)
        .expect("projection should build a validated not-implemented result")
        .into_inner();

    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::NotImplemented
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.binding_statuses[1].state,
        DependencyBindingStatusState::NotImplemented
    );
}

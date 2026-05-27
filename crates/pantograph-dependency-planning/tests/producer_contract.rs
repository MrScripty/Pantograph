use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, produce_dependency_requirements_proof_from_request,
    DependencyOverridePatchV1, DependencyPlanningContractError, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningRequest, DependencyPlanningSeverity,
    DependencyPlanningState, DependencyRequirementsAvailabilityFacts,
    DependencyRequirementsProofRequest, DependencyRequirementsProofStatus, DependencyTraitIntent,
    DependencyTraitIntentId, DependencyTraitIntentValue, ValidatedDependencyPlanningRequest,
    ValidatedDependencyRequirementsProofRequest,
};

const VALID_REQUEST: &str = include_str!("fixtures/dependency_planning_request.json");

fn validated_request() -> ValidatedDependencyPlanningRequest {
    let mut request: DependencyPlanningRequest =
        serde_json::from_str(VALID_REQUEST).expect("request fixture should decode");
    request.model_ref.selected_artifact_path = None;
    ValidatedDependencyPlanningRequest::try_from(request).expect("request should validate")
}

fn produce_current(
    request: &ValidatedDependencyPlanningRequest,
) -> pantograph_dependency_planning::DependencyRequirementsProof {
    produce_dependency_requirements_proof(request, None).expect("proof should be produced")
}

#[test]
fn producer_derives_stable_path_free_requirements_id() {
    let request = validated_request();

    let first = produce_current(&request);
    let second = produce_current(&request);

    assert_eq!(first, second);
    assert_eq!(first.status, DependencyRequirementsProofStatus::Current);
    assert!(first
        .dependency_requirements_id
        .as_str()
        .starts_with("dependency-requirements-blake3:"));
    assert!(first
        .dependency_override_fingerprint
        .as_str()
        .starts_with("dependency-overrides-blake3:"));
    assert_eq!(first.identity_key.model_ref.selected_artifact_path, None);
    first.validate().expect("proof should validate");
}

#[test]
fn producer_identity_changes_when_platform_runtime_bindings_overrides_or_traits_change() {
    let request = validated_request();
    let base = produce_current(&request).dependency_requirements_id;

    let mut without_overrides = request.as_request().clone();
    without_overrides.dependency_override_patches = Vec::<DependencyOverridePatchV1>::new();
    let without_overrides = ValidatedDependencyPlanningRequest::try_from(without_overrides)
        .expect("request without overrides should validate");
    assert_ne!(
        base,
        produce_current(&without_overrides).dependency_requirements_id
    );

    let mut with_trait = request.as_request().clone();
    with_trait.trait_intents.push(DependencyTraitIntent {
        trait_id: DependencyTraitIntentId::parse("denoise").expect("trait id should parse"),
        value: DependencyTraitIntentValue::Text("0.75".to_string()),
    });
    let with_trait = ValidatedDependencyPlanningRequest::try_from(with_trait)
        .expect("request with trait should validate");
    assert_ne!(
        base,
        produce_current(&with_trait).dependency_requirements_id
    );

    let mut without_bindings = request.as_request().clone();
    without_bindings.selected_binding_ids.clear();
    let without_bindings = ValidatedDependencyPlanningRequest::try_from(without_bindings)
        .expect("request without bindings should validate");
    assert_ne!(
        base,
        produce_current(&without_bindings).dependency_requirements_id
    );
}

#[test]
fn producer_maps_typed_availability_facts_to_proof_status_and_diagnostics() {
    let request = validated_request();
    let facts = DependencyRequirementsAvailabilityFacts {
        contract_version: 1,
        state: DependencyPlanningState::NotImplemented,
        diagnostics: vec![DependencyPlanningDiagnostic {
            code: DependencyPlanningDiagnosticCode::NotImplemented,
            severity: DependencyPlanningSeverity::Error,
            message: "dependency planning availability is not implemented for this runtime"
                .to_string(),
            model_id: Some(request.as_request().model_ref.model_id.clone()),
            runtime_id: request
                .as_request()
                .scheduler_intent
                .requested_runtime_id
                .clone(),
            device_id: None,
            field_path: Some("availability.runtime".to_string()),
        }],
    };

    let proof = produce_dependency_requirements_proof(&request, Some(&facts))
        .expect("typed availability facts should produce proof");

    assert_eq!(
        proof.status,
        DependencyRequirementsProofStatus::NotImplemented
    );
    assert_eq!(proof.diagnostics, facts.diagnostics);
}

#[test]
fn producer_rejects_path_carrying_model_refs() {
    let mut request = validated_request().into_inner();
    request.model_ref.selected_artifact_path = Some("models/tiny-sd".to_string());
    let request = ValidatedDependencyPlanningRequest::try_from(request)
        .expect("raw planning request validation allows migration path diagnostics");

    let error = produce_dependency_requirements_proof(&request, None)
        .expect_err("producer proof identity must be path-free");

    assert_eq!(
        error,
        DependencyPlanningContractError::InvalidField {
            field: "pumas_model_ref.selected_artifact_path",
            reason: "path-free dependency identity must not carry selected artifact paths"
        }
    );
}

#[test]
fn producer_request_wrapper_validates_and_produces_same_proof() {
    let planning_request = validated_request().into_inner();
    let wrapper = DependencyRequirementsProofRequest {
        contract_version: 1,
        planning_request: planning_request.clone(),
        availability_facts: None,
    };
    let validated_wrapper = ValidatedDependencyRequirementsProofRequest::try_from(wrapper)
        .expect("producer wrapper should validate");
    let direct = produce_current(
        &ValidatedDependencyPlanningRequest::try_from(planning_request)
            .expect("planning request should validate"),
    );
    let wrapped = produce_dependency_requirements_proof_from_request(&validated_wrapper)
        .expect("wrapper should produce proof");

    assert_eq!(wrapped, direct);
}

use pantograph_runtime_registry::{
    select_runtime_technical_fit, RuntimeTechnicalFitDecision, RuntimeTechnicalFitRequest,
};

#[test]
fn runtime_technical_fit_contract_fixture_round_trips_and_selects() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/technical_fit_contract.json"))
            .expect("fixture parses");

    let request: RuntimeTechnicalFitRequest =
        serde_json::from_value(fixture["technical_fit_request"].clone())
            .expect("technical-fit request fixture matches Rust DTO");
    let expected_decision: RuntimeTechnicalFitDecision =
        serde_json::from_value(fixture["selected_decision"].clone())
            .expect("technical-fit decision fixture matches Rust DTO");
    let traced_decision: RuntimeTechnicalFitDecision =
        serde_json::from_value(fixture["decision_with_selection_policy_trace"].clone())
            .expect("technical-fit policy trace fixture matches Rust DTO");

    assert_eq!(
        serde_json::to_value(&request).expect("serialize technical-fit request"),
        fixture["technical_fit_request"]
    );
    assert_eq!(
        serde_json::to_value(&expected_decision).expect("serialize technical-fit decision"),
        fixture["selected_decision"]
    );
    assert_eq!(
        serde_json::to_value(&traced_decision).expect("serialize traced technical-fit decision"),
        fixture["decision_with_selection_policy_trace"]
    );
    assert_eq!(
        traced_decision
            .selection_policy_trace
            .as_ref()
            .and_then(|trace| trace.candidate_set_summary.as_ref())
            .map(|summary| summary.eligible_candidate_count),
        Some(1)
    );

    let decision = select_runtime_technical_fit(&request);
    assert_eq!(decision, expected_decision);
    assert_eq!(
        decision.selected_runtime_variant_id.as_deref(),
        Some("pytorch/linux-x64/cuda")
    );
    assert_eq!(decision.selected_device_id.as_deref(), Some("cuda:0"));
}

use pantograph_dependency_planning::{
    DependencyOverrideFieldsV1, DependencyOverridePatchV1, DependencyOverrideScope,
    DependencyTaskId, DeviceIntentId, ModelRefMigrationDiagnostic, PumasModelRef, RuntimeIntentId,
};
use pantograph_scheduler::{
    evaluate_scheduler_bounded_grouping, SchedulableTaskIntent, SchedulerBatchCandidate,
    SchedulerBatchMemoryImpact, SchedulerBoundedGroupingDisposition as Disposition,
    SchedulerBoundedGroupingIncompatibility as Incompatibility,
    SchedulerBoundedGroupingRefusal as Refusal, SchedulerEstimateHint, SchedulerEstimateHintKind,
    SchedulerModelResidencyState, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
    SchedulerTaskId, SchedulerTraitId, SchedulerTraitSetting, SchedulerTraitValue,
    SchedulerWorkflowId, SchedulerWorkflowRunId, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
    SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS, SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES,
};

type CandidateMutation = fn(&mut SchedulerBatchCandidate);

fn model() -> PumasModelRef {
    PumasModelRef {
        model_id: "m".to_string(),
        revision: None,
        selected_artifact_id: None,
        selected_artifact_path: None,
        migration_diagnostics: vec![],
    }
}

fn candidate(index: usize) -> SchedulerBatchCandidate {
    let workflow_id = SchedulerWorkflowId::parse("w").unwrap();
    let workflow_run_id = SchedulerWorkflowRunId::parse("r").unwrap();
    let node_id = SchedulerNodeId::parse(format!("n{index}")).unwrap();
    let task_id = SchedulerTaskId::parse(format!("t{index}")).unwrap();
    let task_family = DependencyTaskId::parse("embedding").unwrap();
    SchedulerBatchCandidate {
        workflow_id: workflow_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        node_id: node_id.clone(),
        task_id: task_id.clone(),
        task_intent: SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id,
            workflow_run_id,
            node_id,
            task_id,
            fairness_key: None,
            task_type: task_family.clone(),
            model_ref: model(),
            constraints: SchedulerRuntimeDeviceConstraints::default(),
            trait_settings: vec![],
            dependency_override_patches: vec![],
            estimate_hints: vec![],
        },
        task_family,
        selected_runtime_id: RuntimeIntentId::parse("cpu").unwrap(),
        selected_device_ids: vec![DeviceIntentId::parse("d").unwrap()],
        selected_model_ref: model(),
        model_residency_state: SchedulerModelResidencyState::Resident,
        input_shape_signature: "shape".to_string(),
        estimated_latency_ms: 1,
        memory_impact: SchedulerBatchMemoryImpact {
            incremental_bytes: 10,
            peak_bytes: 10,
        },
    }
}

fn diagnostic() -> ModelRefMigrationDiagnostic {
    ModelRefMigrationDiagnostic {
        code: "d".to_string(),
        message: "m".to_string(),
        input: None,
    }
}

fn patch() -> DependencyOverridePatchV1 {
    DependencyOverridePatchV1 {
        contract_version: 1,
        binding_id: "binding".to_string(),
        scope: DependencyOverrideScope::Binding,
        requirement_name: None,
        fields: DependencyOverrideFieldsV1::default(),
        source: None,
        updated_at: None,
    }
}

fn assert_input_limit(value: &SchedulerBatchCandidate, field: &'static str) {
    assert_eq!(
        evaluate_scheduler_bounded_grouping(std::slice::from_ref(value)),
        Err(Refusal::InputLimit {
            candidate_index: 0,
            field,
        })
    );
}

#[test]
fn accepts_one_two_and_eight_as_immediate_unchanged_prefixes() {
    assert_eq!(SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS, 8);
    for (count, memory, disposition) in [
        (1, 10, Disposition::Singleton),
        (2, 20, Disposition::CompatibleGroup),
        (8, 80, Disposition::CompatibleGroup),
    ] {
        let input: Vec<_> = (0..count).map(candidate).collect();
        let before = input.clone();
        let result = evaluate_scheduler_bounded_grouping(&input).unwrap();
        assert_eq!(result.selected_count(), count);
        assert_eq!(result.total_incremental_memory_bytes(), memory);
        assert_eq!(result.disposition(), disposition);
        assert_eq!(input, before);
    }
}

#[test]
fn refuses_zero_and_nine_without_selecting_a_partial_window() {
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&[]),
        Err(Refusal::InvalidWindow { candidate_count: 0 })
    );
    let mut input: Vec<_> = (0..9).map(candidate).collect();
    input[0].input_shape_signature = " ".repeat(129);
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::InvalidWindow { candidate_count: 9 })
    );
}

#[test]
fn every_compatibility_dimension_has_an_explicit_singleton_outcome() {
    let cases: [(Incompatibility, CandidateMutation); 5] = [
        (Incompatibility::TaskFamily, |c| {
            c.task_family = DependencyTaskId::parse("rerank").unwrap();
            c.task_intent.task_type = c.task_family.clone();
        }),
        (Incompatibility::ModelReference, |c| {
            c.selected_model_ref.revision = Some("other".to_string());
        }),
        (Incompatibility::Runtime, |c| {
            c.selected_runtime_id = RuntimeIntentId::parse("other").unwrap();
        }),
        (Incompatibility::DeviceSet, |c| {
            c.selected_device_ids = vec![DeviceIntentId::parse("other").unwrap()];
        }),
        (Incompatibility::InputShape, |c| {
            c.input_shape_signature = "other".to_string();
        }),
    ];
    for (reason, change) in cases {
        let mut input = vec![candidate(0), candidate(1)];
        input[0].memory_impact.incremental_bytes = 7;
        change(&mut input[1]);
        let result = evaluate_scheduler_bounded_grouping(&input).unwrap();
        assert_eq!(result.selected_count(), 1);
        assert_eq!(result.total_incremental_memory_bytes(), 7);
        assert_eq!(result.disposition(), Disposition::Incompatible(reason));
    }
}

#[test]
fn compares_complete_model_references_including_migration_metadata() {
    let changes: [fn(&mut PumasModelRef); 5] = [
        |m| m.model_id = "other".to_string(),
        |m| m.revision = Some("revision".to_string()),
        |m| m.selected_artifact_id = Some("artifact".to_string()),
        |m| m.selected_artifact_path = Some("relative/path".to_string()),
        |m| m.migration_diagnostics.push(diagnostic()),
    ];
    for change in changes {
        let mut input = vec![candidate(0), candidate(1)];
        change(&mut input[1].selected_model_ref);
        input[1].task_intent.model_ref.model_id = input[1].selected_model_ref.model_id.clone();
        assert_eq!(
            evaluate_scheduler_bounded_grouping(&input)
                .unwrap()
                .disposition(),
            Disposition::Incompatible(Incompatibility::ModelReference)
        );
    }
}

#[test]
fn device_order_is_part_of_exact_compatibility() {
    let mut input = vec![candidate(0), candidate(1)];
    input[0].selected_device_ids = vec![
        DeviceIntentId::parse("a").unwrap(),
        DeviceIntentId::parse("b").unwrap(),
    ];
    input[1].selected_device_ids = input[0].selected_device_ids.iter().rev().cloned().collect();
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input)
            .unwrap()
            .disposition(),
        Disposition::Incompatible(Incompatibility::DeviceSet)
    );
}

#[test]
fn duplicate_identity_refuses_before_an_earlier_incompatibility() {
    let mut input = vec![candidate(0), candidate(1), candidate(0)];
    input[1].input_shape_signature = "other".to_string();
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::DuplicateMember {
            first_index: 0,
            duplicate_index: 2,
        })
    );
    input[2].workflow_run_id = SchedulerWorkflowRunId::parse("other-run").unwrap();
    input[2].task_intent.workflow_run_id = input[2].workflow_run_id.clone();
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input)
            .unwrap()
            .selected_count(),
        1
    );
}

#[test]
fn later_invalid_candidate_refuses_before_earlier_incompatibility() {
    let mut input = vec![candidate(0), candidate(1), candidate(2)];
    input[1].input_shape_signature = "other".to_string();
    input[2].memory_impact.incremental_bytes = 0;
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::InvalidCandidate { candidate_index: 2 })
    );
}

#[test]
fn every_member_is_preflighted_before_any_candidate_validation() {
    let mut input = vec![candidate(0), candidate(1)];
    input[0].estimated_latency_ms = 0;
    input[1].input_shape_signature = " ".repeat(129);
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::InputLimit {
            candidate_index: 1,
            field: "input_shape_signature",
        })
    );
}

#[test]
fn zero_memory_is_invalid_even_for_an_incompatible_unselected_member() {
    let mut input = vec![candidate(0), candidate(1)];
    input[1].input_shape_signature = "other".to_string();
    input[1].memory_impact.incremental_bytes = 0;
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::InvalidCandidate { candidate_index: 1 })
    );
}

#[test]
fn aggregates_exactly_the_selected_members_with_checked_arithmetic() {
    let mut input = vec![candidate(0), candidate(1)];
    input[0].memory_impact.incremental_bytes = u64::MAX;
    input[1].memory_impact.incremental_bytes = 1;
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::MemoryOverflow)
    );
    input[1].input_shape_signature = "other".to_string();
    let singleton = evaluate_scheduler_bounded_grouping(&input).unwrap();
    assert_eq!(singleton.selected_count(), 1);
    assert_eq!(singleton.total_incremental_memory_bytes(), u64::MAX);
    input[1].input_shape_signature = "shape".to_string();
    input[0].memory_impact.incremental_bytes = u64::MAX - 1;
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input)
            .unwrap()
            .total_incremental_memory_bytes(),
        u64::MAX
    );
}

#[test]
fn every_nested_collection_is_capped_before_existing_validation() {
    let cases: [(&str, CandidateMutation); 7] = [
        ("selected_device_ids", |c| {
            c.selected_device_ids = (0..9)
                .map(|n| DeviceIntentId::parse(format!("d{n}")).unwrap())
                .collect();
        }),
        ("trait_settings", |c| {
            c.task_intent.trait_settings = (0..9)
                .map(|n| SchedulerTraitSetting {
                    trait_id: SchedulerTraitId::parse(format!("trait{n}")).unwrap(),
                    value: SchedulerTraitValue::Bool(true),
                })
                .collect();
        }),
        ("dependency_override_patches", |c| {
            c.task_intent.dependency_override_patches = (0..9).map(|_| patch()).collect();
        }),
        ("estimate_hints", |c| {
            c.task_intent.estimate_hints = (0..9)
                .map(|_| SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::BatchSize,
                    value: 1,
                })
                .collect();
        }),
        ("selected_model_ref", |c| {
            c.selected_model_ref.migration_diagnostics = vec![diagnostic(); 9]
        }),
        ("task_intent.model_ref", |c| {
            c.task_intent.model_ref.migration_diagnostics = vec![diagnostic(); 9]
        }),
        ("dependency_override.extra_index_urls", |c| {
            let mut p = patch();
            p.fields.extra_index_urls = Some(vec!["x".to_string(); 9]);
            c.task_intent.dependency_override_patches.push(p);
        }),
    ];
    for (field, change) in cases {
        let mut value = candidate(0);
        change(&mut value);
        assert_input_limit(&value, field);
    }
}

#[test]
fn admits_eight_nested_items_when_the_existing_contract_is_valid() {
    let mut value = candidate(0);
    value.selected_device_ids = (0..8)
        .map(|n| DeviceIntentId::parse(format!("d{n}")).unwrap())
        .collect();
    value.task_intent.trait_settings = (0..8)
        .map(|n| SchedulerTraitSetting {
            trait_id: SchedulerTraitId::parse(format!("trait{n}")).unwrap(),
            value: SchedulerTraitValue::Bool(true),
        })
        .collect();
    value.task_intent.dependency_override_patches = (0..8)
        .map(|_| {
            let mut p = patch();
            p.fields.extra_index_urls = Some(vec!["x".to_string(); 8]);
            p
        })
        .collect();
    value.task_intent.estimate_hints = (0..8)
        .map(|_| SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::BatchSize,
            value: 1,
        })
        .collect();
    value.selected_model_ref.migration_diagnostics = vec![diagnostic(); 8];
    value.task_intent.model_ref.migration_diagnostics = vec![diagnostic(); 8];
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&[value])
            .unwrap()
            .selected_count(),
        1
    );
}

#[test]
fn model_limits_cover_requested_and_selected_raw_fields_and_metadata() {
    let changes: [fn(&mut PumasModelRef); 7] = [
        |m| m.model_id = format!("{}m", " ".repeat(512)),
        |m| m.revision = Some(format!("{}r", " ".repeat(512))),
        |m| m.selected_artifact_id = Some(format!("{}a", " ".repeat(512))),
        |m| m.selected_artifact_path = Some(format!("{}p", " ".repeat(1024))),
        |m| {
            let mut d = diagnostic();
            d.code = "x".repeat(129);
            m.migration_diagnostics.push(d);
        },
        |m| {
            let mut d = diagnostic();
            d.message = "x".repeat(1025);
            m.migration_diagnostics.push(d);
        },
        |m| {
            let mut d = diagnostic();
            d.input = Some("x".repeat(1025));
            m.migration_diagnostics.push(d);
        },
    ];
    for change in changes {
        let mut selected = candidate(0);
        change(&mut selected.selected_model_ref);
        assert_input_limit(&selected, "selected_model_ref");
        let mut requested = candidate(0);
        change(&mut requested.task_intent.model_ref);
        assert_input_limit(&requested, "task_intent.model_ref");
    }
}

#[test]
fn shape_and_trait_limits_use_raw_bytes_before_trimming() {
    let mut value = candidate(0);
    value.input_shape_signature = format!("{}s", " ".repeat(128));
    assert_input_limit(&value, "input_shape_signature");
    value = candidate(0);
    value
        .task_intent
        .trait_settings
        .push(SchedulerTraitSetting {
            trait_id: SchedulerTraitId::parse("trait").unwrap(),
            value: SchedulerTraitValue::String(format!("{}v", " ".repeat(1024))),
        });
    assert_input_limit(&value, "trait_settings");
    value = candidate(0);
    value.input_shape_signature = "é".repeat(65);
    assert_input_limit(&value, "input_shape_signature");
}

#[test]
fn every_override_string_is_preflighted_including_nested_urls() {
    let changes: [fn(&mut DependencyOverridePatchV1, String); 9] = [
        |p, s| p.binding_id = s,
        |p, s| p.requirement_name = Some(s),
        |p, s| p.source = Some(s),
        |p, s| p.updated_at = Some(s),
        |p, s| p.fields.python_executable = Some(s),
        |p, s| p.fields.index_url = Some(s),
        |p, s| p.fields.wheel_source_path = Some(s),
        |p, s| p.fields.package_source_override = Some(s),
        |p, s| p.fields.extra_index_urls = Some(vec![s]),
    ];
    for (index, change) in changes.into_iter().enumerate() {
        let mut value = candidate(0);
        let mut p = patch();
        change(&mut p, format!("{}x", " ".repeat(1024)));
        value.task_intent.dependency_override_patches.push(p);
        assert_input_limit(
            &value,
            if index == 8 {
                "dependency_override.extra_index_urls"
            } else {
                "dependency_override"
            },
        );
    }
}

#[test]
fn aggregate_budget_accepts_exactly_64_kib_and_refuses_one_more_byte() {
    assert_eq!(SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES, 65_536);
    let mut input: Vec<_> = (0..8).map(candidate).collect();
    // The fixed fixture has 41 raw-string bytes/member. Eight diagnostic codes
    // add 8; seven 1024-byte messages and one 975-byte message make 8192 total.
    for value in &mut input {
        value.task_intent.model_ref.migration_diagnostics = (0..8)
            .map(|n| ModelRefMigrationDiagnostic {
                code: "d".to_string(),
                message: "x".repeat(if n < 7 { 1024 } else { 975 }),
                input: None,
            })
            .collect();
    }
    let result = evaluate_scheduler_bounded_grouping(&input).unwrap();
    assert_eq!(result.selected_count(), 8);
    assert_eq!(result.total_incremental_memory_bytes(), 80);
    input[7].task_intent.model_ref.migration_diagnostics[7]
        .message
        .push('x');
    assert_eq!(
        evaluate_scheduler_bounded_grouping(&input),
        Err(Refusal::RawStringBudgetExceeded)
    );
}

#[test]
fn refuses_fixed_context_without_retaining_raw_error_data() {
    let mut value = candidate(0);
    value.input_shape_signature = "private/payload".to_string();
    let error = evaluate_scheduler_bounded_grouping(&[value]).unwrap_err();
    assert_eq!(error, Refusal::InvalidCandidate { candidate_index: 0 });
    assert!(!format!("{error:?}: {error}").contains("private/payload"));
    let mut value = candidate(0);
    let mut p = patch();
    p.fields.index_url = Some(format!("https://private.example/{}", "x".repeat(1024)));
    value.task_intent.dependency_override_patches.push(p);
    let error = evaluate_scheduler_bounded_grouping(&[value]).unwrap_err();
    assert!(!format!("{error:?}: {error}").contains("private.example"));
}

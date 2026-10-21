use pantograph_dependency_planning::{
    DependencyTaskId, DeviceIntentId, ModelRefMigrationDiagnostic, PumasModelRef, RuntimeIntentId,
};
use pantograph_scheduler::{
    evaluate_scheduler_bounded_compatibility, SchedulerBoundedGroupingRefusal as Refusal,
    SchedulerGroupingCompatibility, SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES,
};

#[derive(Clone, PartialEq, Eq)]
struct Facts {
    family: DependencyTaskId,
    model: PumasModelRef,
    runtime: RuntimeIntentId,
    devices: Vec<DeviceIntentId>,
    shape: String,
}

impl Facts {
    fn view(&self) -> SchedulerGroupingCompatibility<'_> {
        SchedulerGroupingCompatibility {
            task_family: &self.family,
            model_ref: &self.model,
            runtime_id: &self.runtime,
            device_ids: &self.devices,
            input_shape_signature: &self.shape,
        }
    }
}

fn facts() -> Facts {
    Facts {
        family: DependencyTaskId::parse("f").unwrap(),
        model: PumasModelRef {
            model_id: "m".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: vec![],
        },
        runtime: RuntimeIntentId::parse("r").unwrap(),
        devices: vec![DeviceIntentId::parse("d").unwrap()],
        shape: "s".to_string(),
    }
}

fn evaluate(input: &[Facts]) -> Result<usize, Refusal> {
    let views: Vec<_> = input.iter().map(Facts::view).collect();
    evaluate_scheduler_bounded_compatibility(&views)
}

fn diagnostic() -> ModelRefMigrationDiagnostic {
    ModelRefMigrationDiagnostic {
        code: "d".to_string(),
        message: "message".to_string(),
        input: None,
    }
}

#[test]
fn known_compatible_windows_select_all_without_mutation_or_cost_facts() {
    for size in [1, 2, 8] {
        let input = vec![facts(); size];
        let before = input.clone();
        assert_eq!(evaluate(&input), Ok(size));
        assert!(input == before);
    }
}

#[test]
fn zero_and_nine_refuse_before_raw_or_semantic_validation() {
    assert_eq!(
        evaluate(&[]),
        Err(Refusal::InvalidWindow { candidate_count: 0 })
    );
    let mut input = vec![facts(); 9];
    input[0].model.model_id.clear();
    input[8].shape = "x".repeat(129);
    assert_eq!(
        evaluate(&input),
        Err(Refusal::InvalidWindow { candidate_count: 9 })
    );
}

type Change = fn(&mut Facts);

#[test]
fn each_compatibility_dimension_selects_only_the_unchanged_anchor() {
    let changes: [Change; 5] = [
        |f| f.family = DependencyTaskId::parse("other").unwrap(),
        |f| f.model.revision = Some("other".to_string()),
        |f| f.runtime = RuntimeIntentId::parse("other").unwrap(),
        |f| f.devices = vec![DeviceIntentId::parse("other").unwrap()],
        |f| f.shape = "other".to_string(),
    ];
    for change in changes {
        let mut input = vec![facts(); 3];
        change(&mut input[1]);
        assert_eq!(evaluate(&input), Ok(1));
    }
}

#[test]
fn every_model_reference_field_and_ordered_device_set_are_exact() {
    let changes: [Change; 5] = [
        |f| f.model.model_id = "other".to_string(),
        |f| f.model.revision = Some("revision".to_string()),
        |f| f.model.selected_artifact_id = Some("artifact".to_string()),
        |f| f.model.selected_artifact_path = Some("relative/path".to_string()),
        |f| f.model.migration_diagnostics.push(diagnostic()),
    ];
    for change in changes {
        let mut input = vec![facts(); 2];
        change(&mut input[1]);
        assert_eq!(evaluate(&input), Ok(1));
    }
    let mut input = vec![facts(); 2];
    input[0].devices = vec![
        DeviceIntentId::parse("a").unwrap(),
        DeviceIntentId::parse("b").unwrap(),
    ];
    input[1].devices = input[0].devices.iter().rev().cloned().collect();
    assert_eq!(evaluate(&input), Ok(1));
}

#[test]
fn all_raw_preflights_precede_model_validation_and_comparison() {
    let mut input = vec![facts(); 3];
    input[0].model.model_id.clear();
    input[1].family = DependencyTaskId::parse("other").unwrap();
    input[2].devices = vec![DeviceIntentId::parse("d").unwrap(); 9];
    assert_eq!(
        evaluate(&input),
        Err(Refusal::InputLimit {
            candidate_index: 2,
            field: "device_ids"
        })
    );
    input[2].devices.truncate(1);
    input[2].model.model_id = " ".repeat(513);
    assert_eq!(
        evaluate(&input),
        Err(Refusal::InputLimit {
            candidate_index: 2,
            field: "model_ref"
        })
    );
}

#[test]
fn invalid_later_model_or_shape_refuses_before_valid_incompatibility() {
    let mut input = vec![facts(); 3];
    input[1].runtime = RuntimeIntentId::parse("other").unwrap();
    for bad_model in ["", " "] {
        input[2].model.model_id = bad_model.to_string();
        assert_eq!(
            evaluate(&input),
            Err(Refusal::InvalidCandidate { candidate_index: 2 })
        );
    }
    input[2].model.model_id = "m".to_string();
    for bad_shape in ["", " ", "tensor with spaces", "non-ascii-λ"] {
        input[2].shape = bad_shape.to_string();
        assert_eq!(
            evaluate(&input),
            Err(Refusal::InvalidCandidate { candidate_index: 2 })
        );
    }
}

#[test]
fn empty_or_duplicate_device_lists_are_invalid_including_later_members() {
    let mut input = vec![facts(); 3];
    input[1].shape = "other".to_string();
    for devices in [vec![], vec![DeviceIntentId::parse("d").unwrap(); 2]] {
        input[2].devices = devices;
        assert_eq!(
            evaluate(&input),
            Err(Refusal::InvalidCandidate { candidate_index: 2 })
        );
    }
}

#[test]
fn nested_model_metadata_and_each_raw_string_limit_are_checked() {
    let changes: [Change; 8] = [
        |f| f.model.model_id = "m".repeat(513),
        |f| f.model.revision = Some("r".repeat(513)),
        |f| f.model.selected_artifact_id = Some("a".repeat(513)),
        |f| f.model.selected_artifact_path = Some("p".repeat(1025)),
        |f| f.model.migration_diagnostics = vec![diagnostic(); 9],
        |f| {
            let mut d = diagnostic();
            d.code = "d".repeat(129);
            f.model.migration_diagnostics.push(d);
        },
        |f| {
            let mut d = diagnostic();
            d.message = "m".repeat(1025);
            f.model.migration_diagnostics.push(d);
        },
        |f| {
            let mut d = diagnostic();
            d.input = Some("i".repeat(1025));
            f.model.migration_diagnostics.push(d);
        },
    ];
    for change in changes {
        let mut value = facts();
        change(&mut value);
        assert_eq!(
            evaluate(&[value]),
            Err(Refusal::InputLimit {
                candidate_index: 0,
                field: "model_ref"
            })
        );
    }
    let mut value = facts();
    value.shape = " ".repeat(129);
    assert_eq!(
        evaluate(&[value]),
        Err(Refusal::InputLimit {
            candidate_index: 0,
            field: "input_shape_signature"
        })
    );
}

#[test]
fn exact_aggregate_string_budget_is_admitted_and_next_byte_refuses() {
    assert_eq!(SCHEDULER_BOUNDED_GROUPING_MAX_RAW_STRING_BYTES, 65_536);
    let mut value = facts();
    // Five one-byte base fields + four one-byte diagnostic codes + 4096
    // message bytes + 4087 input bytes = exactly 8192 bytes per view.
    value.model.migration_diagnostics = (0..4)
        .map(|index| ModelRefMigrationDiagnostic {
            code: "d".to_string(),
            message: "m".repeat(1024),
            input: Some("i".repeat(if index == 3 { 1015 } else { 1024 })),
        })
        .collect();
    let mut input = vec![value; 8];
    assert_eq!(evaluate(&input), Ok(8));
    input[7].model.migration_diagnostics[3]
        .input
        .as_mut()
        .unwrap()
        .push('i');
    assert_eq!(evaluate(&input), Err(Refusal::RawStringBudgetExceeded));
}

#[test]
fn refusal_context_excludes_sensitive_model_metadata() {
    let mut value = facts();
    value.model.migration_diagnostics = vec![ModelRefMigrationDiagnostic {
        code: "d".to_string(),
        message: "secret-diagnostic".repeat(100),
        input: Some("secret-path".to_string()),
    }];
    let error = evaluate(&[value]).unwrap_err();
    assert_eq!(
        error,
        Refusal::InputLimit {
            candidate_index: 0,
            field: "model_ref"
        }
    );
    for rendered in [format!("{error}"), format!("{error:?}")] {
        assert!(!rendered.contains("secret"));
    }
}

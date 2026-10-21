use super::*;
use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerTask, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultValue,
};
use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_scheduler::{SchedulerRuntimeHandoff, SchedulerTaskState, SchedulerTaskStateRecord};
use serde_json::{json, Value};

fn fixture(count: usize) -> (WorkflowExecutionSessionStore, String, String) {
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session = store
        .create_session(
            "workflow.image_generation".into(),
            None,
            None,
            vec![],
            vec![],
            true,
        )
        .unwrap();
    let run = store
        .enqueue_run(
            &session,
            &WorkflowExecutionSessionRunRequest {
                session_id: session.clone(),
                workflow_semantic_version: "0.1.0".into(),
                inputs: vec![],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .unwrap();
    store.begin_queued_run(&session, &run).unwrap().unwrap();
    let handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "../../../pantograph-scheduler/tests/fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .unwrap();
    let active = store
        .active
        .get_mut(&session)
        .unwrap()
        .active_run
        .as_mut()
        .unwrap();
    let mut tasks = vec![];
    for index in 0..count {
        let id = format!("task.cpu.{index}");
        let node = format!("node.cpu.{index}");
        let source = format!("task.source.{index}");
        let mut intent = serde_json::to_value(&handoff.task_intent).unwrap();
        intent["workflow_run_id"] = json!(run);
        intent["task_id"] = json!(id);
        intent["node_id"] = json!(node);
        intent["task_type"] = json!("embedding");
        intent["model_ref"] =
            json!({"model_id":"synthetic.cpu.fixture", "selected_artifact_id":"bert.fixture"});
        intent["constraints"] =
            json!({"requested_runtime_id":"candle", "requested_device_id":"cpu"});
        intent["trait_settings"] = json!([]);
        intent["estimate_hints"] = json!([]);
        let mut proof = serde_json::to_value(&handoff.readiness_proof).unwrap();
        proof["execution_context"]["workflow_run_id"] = json!(run);
        proof["execution_context"]["scheduler_task_id"] = json!(id);
        proof["execution_context"]["node_id"] = json!(node);
        proof["execution_context"]["dependency_override_fingerprint"] = json!("override.fixture");
        proof["preflight_result"]["identity_key"]["model_ref"] = intent["model_ref"].clone();
        proof["preflight_result"]["identity_key"]["task_id"] = json!("embedding");
        proof["preflight_result"]["identity_key"]["scheduler_intent"] =
            intent["constraints"].clone();
        let ctx = &proof["execution_context"];
        let task: WorkflowSchedulerTask = serde_json::from_value(json!({
            "workflow_id":"workflow.image_generation", "workflow_run_id":run,
            "node_id":node, "task_id":id, "node_type":"llm-inference", "execution_class":"runtime_inference",
            "dependency_task_ids":[source],
            "input_bindings":[{"source_node_id":source,"source_task_id":source,"source_port_id":"text","target_port_id":"text"}],
            "schedulable_intent":intent,
            "schedulable_intent_template":{
                "task_type":"embedding", "constraints":intent["constraints"],
                "dependency_readiness_source":{
                    "graph_revision":ctx["graph_revision"], "validation_session_id":ctx["validation_session_id"],
                    "validation_snapshot_id":ctx["validation_snapshot_id"], "descriptor_fingerprint":ctx["descriptor_fingerprint"],
                    "dependency_requirements_id":ctx["dependency_requirements_id"], "dependency_override_fingerprint":"override.fixture"
                }
            },
            "runtime_source_context":{"operation_type":"embedding", "context_shape_key":"embedding.one-text", "cancellation_mode":"cooperative"}
        })).unwrap();
        let record: SchedulerTaskStateRecord = serde_json::from_value(json!({
            "contract_version":1, "workflow_id":"workflow.image_generation", "workflow_run_id":run,
            "node_id":node,"task_id":id,"state_version":1,"last_transition_id":"transition.ready",
            "state":{"kind":"ready", "execution_intent":{"execution_kind":"runtime", "task_intent":intent}}
        })).unwrap();
        let proof: DependencyReadinessProofEnvelope = serde_json::from_value(proof).unwrap();
        active
            .scheduler_task_records
            .insert(id.clone(), record.clone());
        active.runtime_dispatch_readiness_proofs.insert(id, proof);
        active.scheduler_task_results.insert(source.clone(), serde_json::from_value::<WorkflowSchedulerTaskResult>(json!({
            "schema_version":1,"workflow_id":"workflow.image_generation","workflow_run_id":run,
            "node_id":source,"task_id":source,"status":"completed", "outputs":[{"port_id":"text", "value":{"value_type":"string","value":format!("actual text {index}")}}]
        })).unwrap());
        let mut source_task = task.clone();
        source_task.task_id = pantograph_scheduler::SchedulerTaskId::parse(&source).unwrap();
        source_task.execution_class = WorkflowSchedulerTaskExecutionClass::SourceInput;
        let mut source_record = record;
        source_record.task_id = source_task.task_id.clone();
        source_record.state = SchedulerTaskState::Completed {
            execution_intent: source_record.state.execution_intent().unwrap().clone(),
        };
        active.scheduler_task_records.insert(source, source_record);
        tasks.extend([source_task, task]);
    }
    active.scheduler_task_graph = Some(serde_json::from_value::<WorkflowSchedulerTaskGraph>(json!({
        "schema_version":7,"workflow_id":"workflow.image_generation","workflow_run_id":run,"tasks":tasks
    })).unwrap());
    (store, session, run)
}

fn active<'a>(
    store: &'a mut WorkflowExecutionSessionStore,
    session: &str,
) -> &'a mut super::super::WorkflowExecutionSessionActiveRun {
    store
        .active
        .get_mut(session)
        .unwrap()
        .active_run
        .as_mut()
        .unwrap()
}

fn patch_task(
    store: &mut WorkflowExecutionSessionStore,
    session: &str,
    index: usize,
    path: &str,
    value: Value,
) {
    let task = &mut active(store, session)
        .scheduler_task_graph
        .as_mut()
        .unwrap()
        .tasks[2 * index + 1];
    let mut raw = serde_json::to_value(&*task).unwrap();
    *raw.pointer_mut(path).unwrap() = value;
    *task = serde_json::from_value(raw).unwrap();
}

#[test]
fn window_is_first_ready_prefix_up_to_eight_with_actual_inputs_and_no_mutation() {
    for count in [0, 1, 2, 8, 9] {
        let (store, session, run) = fixture(count);
        let before = format!("{store:?}");
        let tick = store.tick;
        let captured = store.serial_ready_window(&session, &run, "task.cpu.0");
        if count == 0 {
            assert!(captured.is_none());
        } else {
            let mut captured = captured.unwrap();
            assert_eq!(captured.len(), count.min(8));
            for (index, member) in captured.iter().enumerate() {
                assert_eq!(member.task.task_id.as_str(), format!("task.cpu.{index}"));
                assert!(member.grouping_bounded());
                assert_eq!(
                    member.record,
                    store.active[&session]
                        .active_run
                        .as_ref()
                        .unwrap()
                        .scheduler_task_records[member.task.task_id.as_str()]
                );
                assert_eq!(
                    member.proof,
                    store.active[&session]
                        .active_run
                        .as_ref()
                        .unwrap()
                        .runtime_dispatch_readiness_proofs[member.task.task_id.as_str()]
                );
                assert_eq!(
                    member.inputs[0].value,
                    pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(
                        format!("actual text {index}")
                    )
                );
            }
            captured[0].task.node_type = "changed clone".into();
            captured[0].inputs.clear();
        }
        assert_eq!(store.tick, tick);
        assert_eq!(format!("{store:?}"), before);
    }
}

#[test]
fn wrong_run_anchor_missing_metadata_and_population_fall_back() {
    let (mut store, session, run) = fixture(2);
    assert!(store
        .serial_ready_window(&session, "wrong-run", "task.cpu.0")
        .is_none());
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.1")
        .is_none());
    assert!(store
        .serial_ready_window("wrong-session", &run, "task.cpu.0")
        .is_none());
    active(&mut store, &session)
        .runtime_dispatch_readiness_proofs
        .remove("task.cpu.1");
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(2);
    active(&mut store, &session)
        .scheduler_task_records
        .remove("task.cpu.1");
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(1);
    let graph = active(&mut store, &session)
        .scheduler_task_graph
        .as_mut()
        .unwrap();
    graph.tasks.resize(129, graph.tasks[0].clone());
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(1);
    active(&mut store, &session).scheduler_task_graph = None;
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
}

#[test]
fn unsupported_selected_members_are_never_skipped_but_ninth_is_outside_window() {
    for unsupported in [0, 1] {
        let (mut store, session, run) = fixture(2);
        patch_task(
            &mut store,
            &session,
            unsupported,
            "/schedulable_intent/model_ref/model_id",
            json!(" ".repeat(129)),
        );
        assert!(store
            .serial_ready_window(&session, &run, "task.cpu.0")
            .is_none());
    }
    let (mut store, session, run) = fixture(9);
    patch_task(
        &mut store,
        &session,
        8,
        "/schedulable_intent/model_ref/model_id",
        json!("x".repeat(100_000)),
    );
    assert_eq!(
        store
            .serial_ready_window(&session, &run, "task.cpu.0")
            .unwrap()
            .len(),
        8
    );
}

#[test]
fn complete_raw_task_and_template_bounds_precede_snapshot_cloning() {
    for (path, value) in [
        ("/node_type", json!(" ".repeat(129))),
        (
            "/runtime_source_context/context_shape_key",
            json!("x".repeat(129)),
        ),
        ("/input_bindings/0/source_port_id", json!("x".repeat(129))),
        (
            "/schedulable_intent/model_ref/revision",
            json!("x".repeat(129)),
        ),
        (
            "/schedulable_intent/trait_settings",
            json!([{"trait_id":"huge","value":{"kind":"string","value":"x".repeat(100_000)}}]),
        ),
        (
            "/schedulable_intent/estimate_hints",
            json!(vec![json!({"kind":"batch_size","value":1}); 9]),
        ),
        (
            "/schedulable_intent_template/trait_settings",
            json!([{"trait_id":"huge","value":{"kind":"string","value":"x".repeat(100_000)}}]),
        ),
        (
            "/schedulable_intent_template/dependency_readiness_source/selected_binding_ids",
            json!(vec!["binding"; 9]),
        ),
    ] {
        let (mut store, session, run) = fixture(2);
        // Optional fields omitted by serialization need insertion, not pointer_mut.
        let task = &mut active(&mut store, &session)
            .scheduler_task_graph
            .as_mut()
            .unwrap()
            .tasks[3];
        let mut raw = serde_json::to_value(&*task).unwrap();
        let (parent, name) = path.rsplit_once('/').unwrap();
        raw.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(name.into(), value);
        *task = serde_json::from_value(raw).unwrap();
        assert!(
            store
                .serial_ready_window(&session, &run, "task.cpu.0")
                .is_none(),
            "{path}"
        );
    }
}

#[test]
fn record_intent_and_proof_nested_payloads_are_bounded_before_equality() {
    let (mut store, session, run) = fixture(1);
    let record = active(&mut store, &session)
        .scheduler_task_records
        .get_mut("task.cpu.0")
        .unwrap();
    if let SchedulerTaskState::Ready {
        execution_intent:
            pantograph_scheduler::SchedulerTaskExecutionIntent::Runtime { task_intent },
    } = &mut record.state
    {
        task_intent.model_ref.model_id = "x".repeat(100_000);
    } else {
        panic!("runtime Ready fixture");
    }
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    for path in [
        "/preflight_result/identity_key/model_ref/model_id",
        "/preflight_result/identity_key/model_ref/revision",
    ] {
        let (mut store, session, run) = fixture(1);
        let proof = active(&mut store, &session)
            .runtime_dispatch_readiness_proofs
            .get_mut("task.cpu.0")
            .unwrap();
        let mut raw = serde_json::to_value(&*proof).unwrap();
        let (parent, name) = path.rsplit_once('/').unwrap();
        raw.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(name.into(), json!("x".repeat(129)));
        *proof = serde_json::from_value(raw).unwrap();
        assert!(store
            .serial_ready_window(&session, &run, "task.cpu.0")
            .is_none());
    }
}

#[test]
fn actual_text_aggregate_has_exact_boundary_and_supported_value_fence() {
    let (mut store, session, run) = fixture(2);
    for result in active(&mut store, &session)
        .scheduler_task_results
        .values_mut()
    {
        result.outputs[0].value = WorkflowSchedulerTaskResultValue::String("x".repeat(32 * 1024));
    }
    assert_eq!(
        store
            .serial_ready_window(&session, &run, "task.cpu.0")
            .unwrap()
            .len(),
        2
    );
    if let WorkflowSchedulerTaskResultValue::String(text) = &mut active(&mut store, &session)
        .scheduler_task_results
        .get_mut("task.source.1")
        .unwrap()
        .outputs[0]
        .value
    {
        text.push('x');
    }
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    for value in [
        WorkflowSchedulerTaskResultValue::String(String::new()),
        WorkflowSchedulerTaskResultValue::Bool(true),
    ] {
        let (mut store, session, run) = fixture(1);
        active(&mut store, &session)
            .scheduler_task_results
            .get_mut("task.source.0")
            .unwrap()
            .outputs[0]
            .value = value;
        assert!(store
            .serial_ready_window(&session, &run, "task.cpu.0")
            .is_none());
    }
}

#[test]
fn current_member_raw_preflight_rejects_oversized_replacement_without_encoding() {
    let (store, session, run) = fixture(1);
    let mut member = store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .unwrap()
        .remove(0);
    member.inputs.push(member.inputs[0].clone());
    assert!(!member.grouping_bounded());
    member.inputs.pop();
    member.inputs[0].value =
        pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(
            "x".repeat(64 * 1024),
        );
    assert!(!member.grouping_bounded()); // text plus metadata exceeds the raw snapshot budget
    member.inputs[0].value =
        pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String("ok".into());
    member
        .task
        .schedulable_intent_template
        .as_mut()
        .unwrap()
        .trait_settings = vec![pantograph_scheduler::SchedulerTraitSetting {
        trait_id: pantograph_scheduler::SchedulerTraitId::parse("large").unwrap(),
        value: pantograph_scheduler::SchedulerTraitValue::String("x".repeat(100_000)),
    }];
    assert!(!member.grouping_bounded());
}

#[test]
fn ready_order_completion_cleanup_and_exact_intent_fences_are_preserved() {
    let (mut store, session, run) = fixture(2);
    active(&mut store, &session)
        .scheduler_task_graph
        .as_mut()
        .unwrap()
        .tasks
        .swap(1, 3);
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    assert_eq!(
        store
            .serial_ready_window(&session, &run, "task.cpu.1")
            .unwrap()[0]
            .task
            .task_id
            .as_str(),
        "task.cpu.1"
    );
    let (mut store, session, run) = fixture(2);
    let record = active(&mut store, &session)
        .scheduler_task_records
        .get_mut("task.cpu.0")
        .unwrap();
    record.state = SchedulerTaskState::Completed {
        execution_intent: record.state.execution_intent().unwrap().clone(),
    };
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    assert_eq!(
        store
            .serial_ready_window(&session, &run, "task.cpu.1")
            .unwrap()
            .len(),
        1
    );
    let (mut store, session, run) = fixture(2);
    active(&mut store, &session).completion_cleanup_gate =
        Some(super::super::WorkflowCompletionCleanupGate {
            first_task_id: "task.cpu.0".into(),
            first_attempt_id: super::super::WorkflowSchedulerTaskAttemptId::parse(
                "attempt.fixture",
            )
            .unwrap(),
            successor_task_id: "task.cpu.1".into(),
            reservation_lease_id: None,
        });
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(1);
    patch_task(
        &mut store,
        &session,
        0,
        "/schedulable_intent/model_ref/model_id",
        json!("changed.bounded.model"),
    );
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
}

#[test]
fn proof_diagnostics_bindings_and_model_migration_are_rejected_raw() {
    for (path, value) in [
        (
            "/execution_context/selected_binding_ids",
            json!(vec!["binding"; 9]),
        ),
        (
            "/preflight_result/identity_key/selected_binding_ids",
            json!(vec!["binding"; 9]),
        ),
        (
            "/preflight_result/diagnostics",
            json!([{"code":"invalid_request","severity":"error","message":"x".repeat(100_000)}]),
        ),
        (
            "/preflight_result/identity_key/model_ref/migration_diagnostics",
            json!([{"code":"legacy","message":"x".repeat(100_000)}]),
        ),
    ] {
        let (mut store, session, run) = fixture(1);
        let proof = active(&mut store, &session)
            .runtime_dispatch_readiness_proofs
            .get_mut("task.cpu.0")
            .unwrap();
        let mut raw = serde_json::to_value(&*proof).unwrap();
        let (parent, name) = path.rsplit_once('/').unwrap();
        raw.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(name.into(), value);
        *proof = serde_json::from_value(raw).unwrap();
        assert!(
            store
                .serial_ready_window(&session, &run, "task.cpu.0")
                .is_none(),
            "{path}"
        );
    }
}

#[test]
fn input_population_and_serialized_snapshot_budget_are_bounded() {
    let (mut store, session, run) = fixture(1);
    let result = active(&mut store, &session)
        .scheduler_task_results
        .get_mut("task.source.0")
        .unwrap();
    result.outputs.resize(33, result.outputs[0].clone());
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(1);
    active(&mut store, &session)
        .scheduler_task_results
        .remove("task.source.0");
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    let (mut store, session, run) = fixture(1);
    // Raw text fits; JSON escaping plus metadata exceeds the complete snapshot.
    active(&mut store, &session)
        .scheduler_task_results
        .get_mut("task.source.0")
        .unwrap()
        .outputs[0]
        .value = WorkflowSchedulerTaskResultValue::String("\0".repeat(11_000));
    assert!(store
        .serial_ready_window(&session, &run, "task.cpu.0")
        .is_none());
    assert!(store
        .serial_ready_window(&"x".repeat(129), &run, "task.cpu.0")
        .is_none());
    assert!(store
        .serial_ready_window(&session, &"x".repeat(129), "task.cpu.0")
        .is_none());
    assert!(store
        .serial_ready_window(&session, &run, &"x".repeat(129))
        .is_none());
}

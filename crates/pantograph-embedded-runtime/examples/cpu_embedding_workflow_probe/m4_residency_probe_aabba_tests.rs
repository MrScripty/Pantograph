//! Hand-computed synthetic oracle inputs. Nothing here loads a model, runs a
//! workflow, measures a phase or supplies native qualification evidence.
use super::*;

fn owner(instance: u8, profile: u8, generation: u64) -> Value {
    json!({"loaded_instance":vec![instance;16],"loaded_profile":vec![profile;32],
        "effective_settings":vec![19_u8;32],"generation":generation,"cpu_threads":1})
}

fn idle_charges(guard: u64) -> Value {
    json!({"active_lease_ids":[guard],"active_claims":[{"reservation_id":guard,
        "claims":[{"kind":"ram_bytes","bytes":48*MIB},{"kind":"vram_bytes","bytes":0}]}],
        "resident_claims":{"claims":[{"kind":"ram_bytes","bytes":32*MIB},{"kind":"vram_bytes","bytes":0}]},
        "resident_uncertain":false})
}

fn empty_charges() -> Value {
    json!({"active_lease_ids":[],"active_claims":[],"resident_claims":null,"resident_uncertain":false})
}

fn record(value: Value) -> Value {
    json!({"target":"embedded::serial_cpu_residency","record":value})
}

fn synthetic_aabba() -> (Value, Vec<Value>) {
    let mut records = Vec::new();
    let mut rows = Vec::new();
    let mut events = Vec::new();
    // Explicit expected turns, not a call to the production residency policy:
    // width, guard, task lease, instance, profile, previous generation,
    // cold publication, preceding owner stop, actual per-kind task envelope.
    let turns = [
        (8, 1, 3, 7, 10, 4, true, None, 536_895_107_u64),
        (8, 1, 5, 7, 10, 6, false, None, 536_895_107),
        (12, 6, 8, 8, 11, 12, true, Some((1, 7, 10, 8)), 536_908_694),
        (12, 6, 10, 8, 11, 14, false, None, 536_908_694),
        (
            8,
            11,
            13,
            9,
            10,
            20,
            true,
            Some((6, 8, 11, 16)),
            536_895_107,
        ),
    ];
    for (index, (width, guard, task, instance, profile, generation, cold, stop, peak)) in
        turns.into_iter().enumerate()
    {
        let run = format!("synthetic-aabba-run-{}", index + 1);
        let preparation = format!("synthetic-aabba-preparation-{}", index + 1);
        let attempt = format!("synthetic-aabba-attempt-{}", index + 1);
        let request = format!("synthetic-aabba-request-{}", index + 1);
        records.push(record(json!({"phase":"preparation_plan","preparation_request_id":preparation,
            "workflow_id":workflow(width),"workflow_run_id":run,"node_id":"infer","task_id":"infer",
            "model_ref":{"model_id":model(width),"revision":null,"selected_artifact_id":"main","selected_artifact_path_digest":null},
            "resolved_model_ref":{"model_id":model(width),"revision":null,"selected_artifact_id":"main","selected_artifact_path_digest":null},
            "runtime_id":"candle","runtime_variant_id":"candle.cpu","device_id":"cpu",
            "actual_task_ram_bytes":peak,"actual_task_vram_bytes":peak,"package_conservative_floor_bytes":peak,
            "declared_guard_ram_bytes":48*MIB,"declared_guard_vram_bytes":0,
            "declared_resident_ram_bytes":32*MIB,"declared_resident_vram_bytes":0,
            "declared_capacity":{"resources":[{"kind":"ram_bytes","total_bytes":1104*MIB,"safety_margin_bytes":0},
                {"kind":"vram_bytes","total_bytes":1024*MIB,"safety_margin_bytes":0}]},
            "guard_or_task_acquired_by_this_preparation":false,"required_ram_bytes":80*MIB+peak,"required_vram_bytes":peak})));
        if let Some((old_guard, old_instance, old_profile, old_generation)) = stop {
            records.push(record(
                json!({"phase":"switch_stop_ack","preparation_request_id":preparation,
                "old_owner":owner(old_instance,old_profile,old_generation),
                "charged_before_stop":idle_charges(old_guard),"charged_after_ack":empty_charges(),
                "stop_ns":31,"acknowledgement_ns":7}),
            ));
        }
        let previous = owner(instance, profile, generation);
        let current = owner(instance, profile, generation + 2);
        if cold {
            records.push(record(json!({"phase":"cold_loaded","preparation_request_id":preparation,
                "owner":previous,"guard_lease_id":guard,"charged_after_publication":idle_charges(guard),
                "load_ns":101,"publication_ns":13})));
        } else {
            records.push(record(
                json!({"phase":"retained","preparation_request_id":preparation,"owner":previous}),
            ));
        }
        let mut drained_charges = idle_charges(guard);
        drained_charges["active_lease_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!(task));
        drained_charges["active_claims"]
            .as_array_mut()
            .unwrap()
            .push(json!({"reservation_id":task,
            "claims":[{"kind":"ram_bytes","bytes":peak},{"kind":"vram_bytes","bytes":peak}]}));
        records.push(record(json!({"phase":"native_drained","workflow_id":workflow(width),"workflow_run_id":run,
            "node_id":"infer","task_id":"infer","attempt_id":attempt,"execution_request_id":request,
            "candidate_id":"runtime.candle","reservation_lease_id":task,
            "previous_owner":previous,"current_owner":current,"forward_and_drain_ns":201+index,
            "charged_after_native_drain":drained_charges,"registry_task_release_acknowledged":false})));
        records.push(record(
            json!({"phase":"task_registry_ack","workflow_id":workflow(width),"workflow_run_id":run,
            "node_id":"infer","task_id":"infer","attempt_id":attempt,"execution_request_id":request,
            "candidate_id":"runtime.candle","reservation_lease_id":task,
            "lifecycle_event_id":format!("synthetic-aabba-cleanup-{}",index+1),
            "registry_task_release_acknowledged":true,"scheduler_cleanup_acknowledged":false,
            "charged_after_registry_ack":idle_charges(guard)}),
        ));
        rows.push(json!({"model_alias":format!("bert-{width}"),"model_id":model(width),"response":{"workflow_run_id":run}}));
        events.push(json!({"event_seq":10*index+5,"workflow_run_id":run,"node_id":"infer","payload":{
            "payload_type":"scheduler_task_attempt_lifecycle_changed","transition":"started","execution_class":"runtime",
            "scheduler_task_id":"infer","scheduler_attempt_id":attempt}}));
        events.push(json!({"event_seq":10*index+6,"workflow_run_id":run,"node_id":"infer","payload":{
            "payload_type":"scheduler_task_attempt_lifecycle_changed","transition":"completed","execution_class":"runtime",
            "scheduler_task_id":"infer","scheduler_attempt_id":attempt,"reservation_id":format!("runtime-registry.{task}"),
            "selected_runtime_id":"candle","selected_runtime_variant_id":"candle.cpu","selected_device_id":"cpu"}}));
    }
    records.push(record(
        json!({"phase":"shutdown_stop_ack","old_owner":owner(9,10,22),
        "charged_before_stop":idle_charges(11),"charged_after_ack":empty_charges(),
        "stop_ns":37,"acknowledgement_ns":11}),
    ));
    (
        json!({"fixture_kind":"synthetic oracle fixture, never native evidence",
        "fixed_work":[8,8,12,12,8],"rows":rows,"native_records":records}),
        events,
    )
}

#[test]
fn hand_computed_aabba_requires_three_loads_two_retains_and_three_stops() {
    let (fixture, events) = synthetic_aabba();
    let observed = verify_native_records(&fixture, &events, false).unwrap();
    assert_eq!(observed["raw_loads"], 3);
    assert_eq!(observed["raw_retains"], 2);
    assert_eq!(observed["raw_native_forwards_and_drains"], 5);
    assert_eq!(observed["raw_task_registry_acks"], 5);
    assert_eq!(observed["raw_switch_stops"], 2);
    assert_eq!(observed["raw_final_stops"], 1);
    assert_eq!(observed["native_instances"].as_array().unwrap().len(), 3);
}

#[test]
fn hand_computed_aabba_refuses_a_switch_ack_for_the_wrong_owner() {
    let (mut fixture, events) = synthetic_aabba();
    fixture["native_records"][9]["record"]["old_owner"] = owner(8, 11, 12);
    assert_eq!(
        verify_native_records(&fixture, &events, false)
            .unwrap_err()
            .to_string(),
        "M4 model switch stopped a different actual owner"
    );
}

#[test]
fn hand_computed_aabba_refuses_reusing_the_first_a_instance_on_reload() {
    let (mut fixture, events) = synthetic_aabba();
    // Keep the reload's internal ancestry consistent, so only the reuse of the
    // earlier stopped incarnation is invalidated by this counterexample.
    let first_instance = fixture["native_records"][1]["record"]["owner"]["loaded_instance"].clone();
    fixture["native_records"][19]["record"]["owner"]["loaded_instance"] = first_instance.clone();
    for field in ["previous_owner", "current_owner"] {
        fixture["native_records"][20]["record"][field]["loaded_instance"] = first_instance.clone();
    }
    fixture["native_records"][22]["record"]["old_owner"]["loaded_instance"] = first_instance;
    assert_eq!(
        verify_native_records(&fixture, &events, false)
            .unwrap_err()
            .to_string(),
        "M4 cold reload reused an old private native instance"
    );
}

#[test]
fn hand_computed_aabba_refuses_loading_b_before_the_old_a_stop_ack() {
    let (mut fixture, events) = synthetic_aabba();
    fixture["native_records"]
        .as_array_mut()
        .unwrap()
        .swap(9, 10);
    assert_eq!(
        verify_native_records(&fixture, &events, false)
            .unwrap_err()
            .to_string(),
        "M4 switch stop did not precede the next cold publication"
    );
}

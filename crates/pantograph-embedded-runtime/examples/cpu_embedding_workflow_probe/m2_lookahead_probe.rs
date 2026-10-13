//! One fixed actual CPU cohort for predecessor-conditioned service advice.
use super::*;

const SINGLETON_WORKFLOWS: [&str; 2] = ["cpu-serial-lookahead-a", "cpu-serial-lookahead-b"];
const SEQUENCE: [usize; 13] = [0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0];
type ConditionedSample = (String, [u8; 32], Option<[u8; 32]>, u64, u64, u64);

struct LookaheadLog(Mutex<(Vec<Value>, Vec<Value>, bool)>);
static LOOKAHEAD_LOG: LookaheadLog = LookaheadLog(Mutex::new((Vec::new(), Vec::new(), false)));
impl log::Log for LookaheadLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        matches!(
            metadata.target(),
            "workflow::serial_lookahead" | "workflow::serial_ready"
        )
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = record.args().to_string();
        let mut state = self.0.lock().expect("isolated lookahead collector");
        if line.len() > 8192 || state.0.len() + state.1.len() >= 64 {
            state.2 = true;
            return;
        }
        match serde_json::from_str(&line) {
            Ok(value) if record.target() == "workflow::serial_lookahead" => state.0.push(value),
            Ok(value) => state.1.push(value),
            Err(_) => state.2 = true,
        }
    }
    fn flush(&self) {}
}

fn inputs_for_positions(texts: &[String; 2], positions: &[usize]) -> Vec<WorkflowPortBinding> {
    positions
        .iter()
        .map(|i| WorkflowPortBinding {
            node_id: format!("prompt_{}", ["a", "b"][*i]),
            port_id: "text".into(),
            value: json!(texts[*i]),
        })
        .collect()
}
fn singleton_graph(position: usize) -> ProbeResult<WorkflowGraph> {
    let mut graph = pair_graph()?;
    let suffix = ["a", "b"][position];
    let retained = [
        format!("prompt_{suffix}"),
        format!("infer_{suffix}"),
        format!("vectors_{suffix}"),
        format!("deps_{suffix}"),
    ];
    graph.nodes.retain(|node| retained.contains(&node.id));
    graph
        .edges
        .retain(|edge| retained.contains(&edge.source) && retained.contains(&edge.target));
    Ok(graph)
}
fn targets(positions: &[usize]) -> Vec<WorkflowOutputTarget> {
    positions
        .iter()
        .map(|i| WorkflowOutputTarget {
            node_id: format!("vectors_{}", ["a", "b"][*i]),
            port_id: "vector".into(),
        })
        .collect()
}
fn verify_output(
    response: &WorkflowRunResponse,
    position: usize,
    expected: &Value,
) -> ProbeResult<()> {
    let id = format!("vectors_{}", ["a", "b"][position]);
    let actual = response
        .outputs
        .iter()
        .find(|o| o.node_id == id)
        .and_then(|o| o.value.as_array())
        .ok_or("missing real CPU output")?;
    let reference = expected[position]
        .as_array()
        .ok_or("missing independent vector")?;
    if actual.len() != 8
        || reference.len() != 8
        || actual
            .iter()
            .zip(reference)
            .any(|(a, b)| match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => !a.is_finite() || (a - b).abs() > 1e-5,
                _ => true,
            })
    {
        return Err("real CPU vector differs from independent Transformers reference".into());
    }
    Ok(())
}

async fn attempt(
    runtime: &EmbeddedRuntime,
    root: &Path,
    index: usize,
    positions: &[usize],
    texts: &[String; 2],
    golden: &Value,
) -> ProbeResult<Value> {
    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: if positions.len() == 1 {
                SINGLETON_WORKFLOWS[positions[0]]
            } else {
                PAIR_WORKFLOW
            }
            .into(),
            usage_profile: None,
            keep_alive: false,
        })
        .await?;
    let before = runtime
        .workflow_service()
        .serial_ready_conditioned_samples_for_test();
    let predecessor = runtime
        .workflow_service()
        .serial_ready_predecessor_for_test();
    let started = Instant::now();
    let result = run(
        runtime,
        &session.session_id,
        inputs_for_positions(texts, positions),
        targets(positions),
    )
    .await;
    let wall_ns = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    let after = runtime
        .workflow_service()
        .serial_ready_conditioned_samples_for_test();
    let close = runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id,
        })
        .await;
    let row = json!({"index":index,"positions":positions,"wall_ns":wall_ns,
        "response":result.as_ref().ok().map(|(r,_)|r),
        "readiness_resume_run_ids":result.as_ref().ok().map(|(_,ids)|ids),
        "execution_error":result.as_ref().err().map(ToString::to_string),
        "close_error":close.as_ref().err().map(ToString::to_string),
        "samples_before":before,"samples_after":after,"predecessor_before":predecessor});
    save(root, &format!("m2-attempt-{index}.json"), &row)?;
    let (response, _) = result?;
    close?;
    for position in positions {
        verify_output(&response, *position, &golden["truncation_vectors"])?;
    }
    if after.len() != before.len() + positions.len() {
        return Err(
            "actual singleton/pair did not append exactly its cleanup-qualified samples".into(),
        );
    }
    Ok(row)
}

// This oracle consumes only independently exported raw ring observations and
// ledger Starts. It does not call either ranking implementation.
fn check_raw(
    rows: &[Value],
    advice: &[Value],
    immediate: &[Value],
    events: &[Value],
) -> ProbeResult<Value> {
    if rows.len() != 14 {
        return Err("fixed M2 run count not completed".into());
    }
    let mut keys = [None; 2];
    let mut previous = None;
    for (index, row) in rows[..13].iter().enumerate() {
        let before: Vec<ConditionedSample> = serde_json::from_value(row["samples_before"].clone())?;
        let after: Vec<ConditionedSample> = serde_json::from_value(row["samples_after"].clone())?;
        let position = SEQUENCE[index];
        let sample = after.last().ok_or("missing calibrated service sample")?;
        if after.len() != before.len() + 1 || sample.2 != previous || sample.4 >= 59_000_000_000 {
            return Err("calibration predecessor/count/freshness correspondence failed".into());
        }
        if keys[position].is_some_and(|key| key != sample.1) {
            return Err("calibration exact workload digest changed across sessions".into());
        }
        let starts: Vec<_> = events
            .iter()
            .filter(|e| {
                e["workflow_run_id"] == row["response"]["workflow_run_id"]
                    && e["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                    && e["payload"]["transition"] == "started"
                    && [json!("infer_a"), json!("infer_b")].contains(&e["node_id"])
            })
            .collect();
        if starts.len() != 1
            || starts[0]["node_id"] != json!(format!("infer_{}", ["a", "b"][position]))
            || starts[0]["payload"]["scheduler_task_id"] != json!(sample.0)
        {
            return Err("singleton calibrated sample differs from its actual CPU Start".into());
        }
        keys[position] = Some(sample.1);
        previous = Some(sample.1);
    }
    let keys = [
        keys[0].ok_or("missing A key")?,
        keys[1].ok_or("missing B key")?,
    ];
    if keys[0] == keys[1] {
        return Err("distinct exact workloads have equal digest".into());
    }
    let pair = &rows[13];
    let run_id = &pair["response"]["workflow_run_id"];
    let captured: Vec<_> = advice
        .iter()
        .filter(|v| &v["workflow_run_id"] == run_id)
        .collect();
    let proposals: Vec<_> = immediate
        .iter()
        .filter(|v| &v["workflow_run_id"] == run_id)
        .collect();
    if captured.len() != 1 || proposals.len() != 1 {
        return Err("missing unique actual two-Ready advice/proposal".into());
    }
    let advice = captured[0];
    let proposal = proposals[0];
    let task_ids = proposal["task_ids"].as_array().ok_or("missing task IDs")?;
    if task_ids.len() != 2
        || task_ids[0] == task_ids[1]
        || task_ids.iter().any(|id| id.as_str().is_none())
        || advice["task_ids"] != proposal["task_ids"]
    {
        return Err("advisory/proposal must identify the same two distinct tasks".into());
    }
    let raw: Vec<ConditionedSample> = serde_json::from_value(pair["samples_before"].clone())?;
    let pred: ([u8; 32], u64) = serde_json::from_value(pair["predecessor_before"].clone())?;
    if pred.0 != keys[0] || raw.len() != 13 {
        return Err("fixed final calibration predecessor differs".into());
    }
    if advice["owner_generation"] != json!(pred.1) {
        return Err("advisory is associated with a different owner generation".into());
    }
    for name in ["first_oldest_age_ns", "successor_oldest_age_ns"] {
        let ages = advice[name]
            .as_array()
            .ok_or("missing advisory freshness")?;
        if ages.len() != 2
            || ages
                .iter()
                .any(|v| v.as_u64().is_none_or(|n| n >= 60_000_000_000))
        {
            return Err("advisory freshness failed at selection".into());
        }
    }
    let appended: Vec<ConditionedSample> = serde_json::from_value(pair["samples_after"].clone())?;
    if appended.len() != 15 {
        return Err("pair raw history did not retain exactly two samples".into());
    }
    for i in 0..2 {
        let matches: Vec<_> = appended[13..]
            .iter()
            .filter(|sample| task_ids[i] == json!(sample.0))
            .collect();
        if matches.len() != 1 || matches[0].1 != keys[i] {
            return Err(
                "later pair task workload digest differs from calibrated exact workload".into(),
            );
        }
    }
    let transitions = [
        (keys[0], pred.0),
        (keys[1], pred.0),
        (keys[1], keys[0]),
        (keys[0], keys[1]),
    ];
    let mut counts = [0usize; 4];
    let mut maxima = [0u64; 4];
    for (i, (key, pred_key)) in transitions.iter().enumerate() {
        for sample in &raw {
            if sample.1 == *key && sample.2 == Some(*pred_key) && sample.5 == pred.1 {
                if sample.4 >= 59_000_000_000 {
                    return Err("ambiguous/stale row freshness".into());
                }
                counts[i] += 1;
                maxima[i] = maxima[i].max(sample.3);
            }
        }
    }
    if counts != [3, 3, 3, 3] {
        return Err(format!("fixed transition matrix incomplete: {counts:?}").into());
    }
    let completion0 = maxima[0]
        .checked_add(maxima[2])
        .ok_or("raw score overflow")?;
    let completion1 = maxima[1]
        .checked_add(maxima[3])
        .ok_or("raw score overflow")?;
    let sum0 = maxima[0]
        .checked_add(completion0)
        .ok_or("raw sum overflow")?;
    let sum1 = maxima[1]
        .checked_add(completion1)
        .ok_or("raw sum overflow")?;
    let selected = usize::from((completion1, sum1) < (completion0, sum0));
    if advice["reason"] != "ranked"
        || advice["workload_keys"] != json!(keys)
        || advice["predecessor_key"] != json!(pred.0)
        || advice["first_counts"] != json!([counts[0], counts[1]])
        || advice["successor_counts"] != json!([counts[2], counts[3]])
        || advice["first_ns"] != json!([maxima[0], maxima[1]])
        || advice["successor_ns"] != json!([maxima[2], maxima[3]])
        || advice["selected_position"] != json!(selected)
        || advice["completions_ns"] != json!([[maxima[0], completion0], [maxima[1], completion1]])
        || advice["completion_sums_ns"] != json!([sum0, sum1])
        || advice["plans_evaluated"] != 2
        || advice["completion_events_evaluated"] != 4
    {
        return Err("advisory differs from independent complete raw-row oracle".into());
    }
    let mut immediate_max = [0u64; 2];
    let mut immediate_counts = [0usize; 2];
    for (i, key) in keys.iter().enumerate() {
        for sample in &raw {
            if sample.1 == *key && sample.5 == pred.1 {
                if sample.4 >= 59_000_000_000 {
                    return Err("M1 raw row lacks unambiguous freshness".into());
                }
                immediate_max[i] = immediate_max[i].max(sample.3);
                immediate_counts[i] += 1;
            }
        }
    }
    let proposal_ages = proposal["oldest_age_ns"]
        .as_array()
        .ok_or("missing immediate freshness")?;
    if proposal_ages.len() != 2
        || proposal_ages
            .iter()
            .any(|v| v.as_u64().is_none_or(|n| n >= 60_000_000_000))
    {
        return Err("immediate proposal is stale at selection".into());
    }
    let actual_position = usize::from(immediate_max[1] < immediate_max[0]);
    if proposal["reason"] != "ranked"
        || proposal["maxima_ns"] != json!(immediate_max)
        || proposal["selected_position"] != json!(actual_position)
        || advice["immediate_position"] != json!(actual_position)
        || immediate_counts != [7, 6]
        || proposal["sample_counts"] != json!(immediate_counts)
    {
        return Err("M1 proposal changed under advisory mode".into());
    }
    let starts: Vec<_> = events
        .iter()
        .filter(|e| {
            &e["workflow_run_id"] == run_id
                && e["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                && e["payload"]["transition"] == "started"
                && task_ids.contains(&e["payload"]["scheduler_task_id"])
        })
        .collect();
    if starts.len() != 2
        || starts[0]["node_id"] != json!(format!("infer_{}", ["a", "b"][actual_position]))
        || starts[1]["node_id"] != json!(format!("infer_{}", ["a", "b"][1 - actual_position]))
        || starts[0]["payload"]["scheduler_task_id"] != task_ids[actual_position]
        || starts[1]["payload"]["scheduler_task_id"] != task_ids[1 - actual_position]
    {
        return Err("actual CPU Starts differ from unchanged M1 proposal".into());
    }
    Ok(
        json!({"transition_counts":counts,"transition_maxima_ns":maxima,
        "completions_ns":[[maxima[0],completion0],[maxima[1],completion1]],"completion_sums_ns":[sum0,sum1],
        "advisory_position":selected,"actual_m1_position":actual_position,"owner_generation":pred.1,
        "workload_keys":keys,"all_vectors_match_reference":true,
        "cold_transfer_wall_clock_speedup_qualified":false}),
    )
}

pub(super) async fn qualify(root: &Path) -> ProbeResult<()> {
    log::set_logger(&LOOKAHEAD_LOG)
        .map_err(|_| "qualification requires exclusive logger ownership")?;
    log::set_max_level(log::LevelFilter::Info);
    seed_fixture(root, MODEL_ID, Some(&["model.safetensors"]))?;
    let golden: Value = serde_json::from_slice(&std::fs::read(
        root.join("shared-resources/models")
            .join(MODEL_ID)
            .join("golden.json"),
    )?)?;
    let texts: [String; 2] = serde_json::from_value(golden["truncation_texts"].clone())?;
    save(
        root,
        "m2-protocol.json",
        &json!({"protocol":"m2-conditional-service-v1","sequence":SEQUENCE,
        "texts":texts,"singleton_runs":13,"pair_runs":1,"threads":1,"minimum_samples":3,
        "singleton_graphs":"pair-derived preserved branches","no_retry_for_winner":true,"service_only":true}),
    )?;
    for directory in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
        "app-data",
    ] {
        std::fs::create_dir_all(root.join(directory))?;
    }
    let api = Arc::new(
        pumas_library::PumasApi::builder(root)
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await?,
    );
    api.rebuild_model_index().await?;
    let package = api.resolve_model_package_facts(MODEL_ID).await?;
    let resolved = api
        .resolve_model_artifact_load_target(ResolveModelArtifactLoadTargetRequest {
            model_ref: package.model_ref.clone(),
            expected_artifact_kind: None,
            caller_observed_entry_path: None,
            caller_observed_package_facts_contract_version: None,
            resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
            consumer: PumasArtifactConsumer {
                consumer_name: "pantograph-embedded-runtime".into(),
                task_kind: Some("embedding".into()),
                runtime_family: Some("candle.cpu".into()),
            },
        })
        .await?;
    let target = resolved
        .target
        .ok_or("missing authoritative Pumas target")?;
    let registry = Arc::new(RuntimeRegistry::new());
    registry.configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
        runtime_id: "candle".into(),
        model_id: target.local_load_path.clone(),
        requirements: RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(RESIDENT_ENVELOPE),
            RuntimeReservationResourceClaim::vram_bytes(0),
        ]),
    }])?;
    let gateway = Arc::new(inference::InferenceGateway::new_calibrated_candle_cpu(
        inference::CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60))?,
    )?);
    let selector = Arc::new(PumasSelectorAccess::Owner(api));
    let baseline = compose(
        root,
        "m2-bootstrap",
        gateway.clone(),
        registry.clone(),
        selector.clone(),
        false,
    )
    .await?;
    let mut baseline_edit = None;
    let mut retaining_session = None;
    let mut graph_edits = Vec::new();
    let mut serial = None;
    let mut rows = Vec::new();
    let mut stage = "bootstrap_authoring";
    let result = async {
        baseline_edit = Some(author_workflow(&baseline, root, &mut stage, true).await?);
        stage = "retaining_session";
        retaining_session = Some(
            baseline
                .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
                    workflow_id: WORKFLOW_ID.into(),
                    usage_profile: None,
                    keep_alive: true,
                })
                .await?
                .session_id,
        );
        stage = "bootstrap_execution";
        let bootstrap = run(
            &baseline,
            retaining_session
                .as_deref()
                .ok_or("missing retaining session")?,
            vec![WorkflowPortBinding {
                node_id: "prompt".into(),
                port_id: "text".into(),
                value: json!("hello world"),
            }],
            vec![WorkflowOutputTarget {
                node_id: "vectors".into(),
                port_id: "vector".into(),
            }],
        )
        .await?;
        save(
            root,
            "m2-bootstrap.json",
            &json!({"response":bootstrap.0,"readiness_resume_run_ids":bootstrap.1}),
        )?;
        stage = "publish_retained_owner";
        gateway
            .publish_resident_cpu_cleanup_owner(&registry)
            .await?;
        stage = "serial_composition";
        serial = Some(
            compose_with_serial_config(
                root,
                "m2",
                gateway.clone(),
                registry.clone(),
                selector,
                Some(WorkflowSerialReadyConfig::default().with_completion_lookahead()),
            )
            .await?,
        );
        let runtime = serial.as_ref().ok_or("missing serial runtime")?;
        graph_edits.push(
            author_graph(
                runtime,
                root,
                &mut stage,
                pair_graph()?,
                PAIR_WORKFLOW,
                &["deps_a", "deps_b"],
                true,
            )
            .await?,
        );
        for (position, workflow) in SINGLETON_WORKFLOWS.iter().enumerate() {
            let dependency = format!("deps_{}", ["a", "b"][position]);
            graph_edits.push(
                author_graph(
                    runtime,
                    root,
                    &mut stage,
                    singleton_graph(position)?,
                    workflow,
                    &[&dependency],
                    true,
                )
                .await?,
            );
        }
        stage = "fixed_calibration";
        for (index, position) in SEQUENCE.iter().enumerate() {
            rows.push(attempt(runtime, root, index, &[*position], &texts, &golden).await?);
        }
        stage = "two_ready_pair";
        rows.push(attempt(runtime, root, 13, &[0, 1], &texts, &golden).await?);
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    // Every result after obtaining a runtime reaches its explicit teardown,
    // including bootstrap, composition, execution, and export failures.
    let mut cleanup_errors = Vec::new();
    if let Some(runtime) = serial.as_ref() {
        for edit in graph_edits {
            if let Err(e) = runtime
                .workflow_service()
                .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: edit,
                })
                .await
            {
                cleanup_errors.push(format!("qualification graph edit close: {e}"));
            }
        }
    }
    if let Some(session) = retaining_session {
        if let Err(e) = baseline
            .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                session_id: session,
            })
            .await
        {
            cleanup_errors.push(format!("retaining close: {e}"));
        }
    }
    if let Some(edit) = baseline_edit {
        if let Err(e) = baseline
            .workflow_service()
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit,
            })
            .await
        {
            cleanup_errors.push(format!("bootstrap edit close: {e}"));
        }
    }
    if let Some(runtime) = serial.as_ref() {
        if let Err(e) = runtime.shutdown().await {
            cleanup_errors.push(format!("serial shutdown: {e}"));
        }
    }
    if let Err(e) = baseline.shutdown().await {
        cleanup_errors.push(format!("bootstrap shutdown: {e}"));
    }
    let model_ref = serde_json::from_value(
        json!({"model_id":package.model_ref.model_id,"selected_artifact_id":package.model_ref.selected_artifact_id}),
    )?;
    let final_registry = registry.snapshot();
    let native_owner_absent = gateway.resident_cpu_serial_owner(&model_ref).is_none();
    save(
        root,
        "m2-shutdown.json",
        &json!({"registry":final_registry,"cleanup_errors":cleanup_errors,
        "native_owner_absent":native_owner_absent,"completed_singletons_and_pair":rows.len(),"failure_stage":stage}),
    )?;
    save(
        root,
        "m2-execution-status.json",
        &json!({"completed":result.is_ok(),"error":result.as_ref().err().map(ToString::to_string),
        "completed_runs":rows.len(),"stage":stage,"cohort_restart_allowed":false}),
    )?;
    let (advice, immediate, overflow) = {
        let log = LOOKAHEAD_LOG.0.lock().expect("collector");
        (log.0.clone(), log.1.clone(), log.2)
    };
    save(root, "m2-advice.json", &advice)?;
    save(root, "m2-immediate.json", &immediate)?;
    let events = collect_events(root, "m2")?;
    result?;
    if !cleanup_errors.is_empty() || !final_registry.reservations.is_empty() || !native_owner_absent
    {
        return Err("actual cleanup/shutdown failed or retained reservation/native owner".into());
    }
    if overflow {
        return Err("bounded invocation log overflow".into());
    }
    let oracle = check_raw(&rows, &advice, &immediate, &events);
    save(
        root,
        "m2-oracle-status.json",
        &json!({"passed":oracle.is_ok(),"error":oracle.as_ref().err().map(ToString::to_string)}),
    )?;
    let oracle = oracle?;
    save(
        root,
        "m2-summary.json",
        &json!({"protocol":"m2-conditional-service-v1","oracle":oracle,
        "scope":"actual retained CPU first-order conditioned service advice","backend":"candle_cpu_f32","threads":1,
        "declared_resident_ram_bytes":RESIDENT_ENVELOPE,"resident_bytes_measured":false,"model_acquisition_bytes":0,
        "singleton_runs":13,"pair_runs":1,"outputs":15,"wall_ns":rows.iter().map(|r|r["wall_ns"].clone()).collect::<Vec<_>>(),
        "performance_improvement_claim":false,"full_completion_or_gpu_or_gui_claim":false}),
    )?;
    println!("{}", serde_json::to_string_pretty(&oracle)?);
    Ok(())
}

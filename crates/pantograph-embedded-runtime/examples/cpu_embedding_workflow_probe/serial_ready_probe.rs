//! Fixed-cohort actual CPU qualification. No injected readiness, clocks or history.
use super::*;
use pantograph_embedded_runtime::EmbeddedRetainedCpuSerialPort;
use pantograph_runtime_registry::{
    RuntimeModelResidentEstimate, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
};
use pantograph_workflow_service::{workflow::WorkflowSerialReadyConfig, WorkflowRunResponse};
use pumas_library::models::{
    PumasArtifactConsumer, PumasArtifactLoadTargetResolutionMode,
    ResolveModelArtifactLoadTargetRequest,
};
use std::sync::Mutex;

#[path = "m2_lookahead_probe.rs"]
mod m2_lookahead_probe;
#[path = "m3_fair_service_probe.rs"]
mod m3_fair_service_probe;

pub(super) async fn qualify_m3(root: &Path) -> ProbeResult<()> {
    m3_fair_service_probe::qualify(root).await
}
pub(super) fn reanalyze_m3(root: &Path) -> ProbeResult<()> {
    m3_fair_service_probe::reanalyze(root)
}

const RUNS: usize = 8; // Three training runs and five observations, never retry for a winner.
const PAIR_WORKFLOW: &str = "cpu-serial-ready-pair";
const RESIDENT_ENVELOPE: u64 = 32 * 1024 * 1024; // Explicit declaration, not allocator measurement.
type RawSamples = Vec<(String, [u8; 32], u64, u64)>;

// Only this isolated example process owns the logger. No filesystem IO in measured dispatch.
struct ProposalLog(Mutex<(Vec<String>, usize, bool)>);
static PROPOSALS: ProposalLog = ProposalLog(Mutex::new((Vec::new(), 0, false)));
impl log::Log for ProposalLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.target() == "workflow::serial_ready"
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = record.args().to_string();
        let mut state = self.0.lock().expect("proposal collector");
        if state.0.len() == 64 || line.len() > 8192 || state.1 + line.len() > 131072 {
            state.2 = true;
        } else {
            state.1 += line.len();
            state.0.push(line);
        }
    }
    fn flush(&self) {}
}

async fn compose(
    root: &Path,
    name: &str,
    gateway: Arc<inference::InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    selector: Arc<PumasSelectorAccess>,
    serial: bool,
) -> ProbeResult<EmbeddedRuntime> {
    compose_with_serial_config(
        root,
        name,
        gateway,
        registry,
        selector,
        serial.then(WorkflowSerialReadyConfig::default),
    )
    .await
}

async fn compose_with_serial_config(
    root: &Path,
    name: &str,
    gateway: Arc<inference::InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    selector: Arc<PumasSelectorAccess>,
    serial_config: Option<WorkflowSerialReadyConfig>,
) -> ProbeResult<EmbeddedRuntime> {
    let serial_port = serial_config.map(|config| {
        let port: Arc<dyn pantograph_runtime_host_contracts::SerialRuntimeHostBatchExecutionPort> =
            Arc::new(EmbeddedRetainedCpuSerialPort::new(
                gateway.clone(),
                registry.clone(),
                selector.clone(),
            ));
        (config, port)
    });
    compose_with_serial_port(root, name, gateway, registry, selector, serial_port).await
}

async fn compose_with_serial_port(
    root: &Path,
    name: &str,
    gateway: Arc<inference::InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    selector: Arc<PumasSelectorAccess>,
    serial_port: Option<(
        WorkflowSerialReadyConfig,
        Arc<dyn pantograph_runtime_host_contracts::SerialRuntimeHostBatchExecutionPort>,
    )>,
) -> ProbeResult<EmbeddedRuntime> {
    let service = if let Some((config, port)) = serial_port {
        WorkflowService::new_serial_ready_cpu(port, config).with_attribution_store(
            pantograph_workflow_service::workflow::SqliteAttributionStore::open_in_memory()?,
        )
    } else {
        WorkflowService::with_ephemeral_attribution_store()?
    };
    let service = service
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(
            root.join(format!("{name}-diagnostics.db")),
        )?)
        .with_artifact_store(ArtifactStore::open(
            root.join(format!("{name}-artifacts")),
            ArtifactPolicy {
                policy_id: "serial-cpu-qualification".into(),
                policy_version: 1,
                ttl_seconds: None,
                max_disk_bytes: Some(1024 * 1024),
                max_memory_bytes: Some(1024 * 1024),
                max_single_artifact_bytes: Some(65536),
                spill_threshold_bytes: None,
                delete_on_consume: false,
            },
        )?);
    let output = EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(
        EmbeddedHostedStartupCompositionInput::new(EmbeddedHostedStartupConfig {
            runtime_registry: registry.clone(),
            runtime_registry_controller: gateway.clone(),
            gateway: gateway.clone(),
            pumas_selector_source: Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                selector,
            )),
            project_root: root.to_path_buf(),
            kv_cache_dir: root.join(format!("{name}-kv-cache")),
            dependency_readiness_runtime_handle: tokio::runtime::Handle::current(),
            max_loaded_sessions: Some(8),
            max_dispatch_source_snapshot_age_ms: 1000,
        })
        .with_workflow_service(service)
        .with_dependency_inventory_app_data_dir(root.join("app-data"))
        .with_dependency_readiness_producer_config(
            EmbeddedDependencyReadinessSnapshotProducerConfig {
                poll_interval: Duration::from_millis(50),
            },
        ),
    )
    .await?;
    Ok(EmbeddedRuntime::from_hosted_composition(
        EmbeddedRuntimeConfig::new(root.join("app-data"), root.to_path_buf()),
        gateway,
        output.shared_extensions,
        output.workflow_service,
        None,
        Some(registry),
        None,
    )
    .with_dependency_readiness_snapshot_producer(output.dependency_readiness_snapshot_producer))
}

fn pair_graph() -> ProbeResult<WorkflowGraph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for suffix in ["a", "b"] {
        let infer = format!("infer_{suffix}");
        let prompt = format!("prompt_{suffix}");
        let vector = format!("vectors_{suffix}");
        let dependency = format!("deps_{suffix}");
        nodes.extend([
            json!({"id":dependency,"node_type":"dependency-environment","position":{"x":0,"y":0},"data":{"mode":"manual"}}),
            json!({"id":prompt,"node_type":"text-input","position":{"x":0,"y":100},"data":{}}),
            json!({"id":infer,"node_type":"llm-inference","position":{"x":300,"y":100},"data":{
                "task_kind":"embedding","runtime":"candle","device":"cpu",
                "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
                "pumas_model_ref":{"model_id":MODEL_ID,"selected_artifact_id":"main"}}}),
            json!({"id":vector,"node_type":"vector-output","position":{"x":600,"y":100},"data":{}}),
        ]);
        edges.extend([
            json!({"id":format!("text-{suffix}"),"source":prompt,"source_handle":"text","target":infer,"target_handle":"text"}),
            json!({"id":format!("vector-{suffix}"),"source":infer,"source_handle":"embedding","target":vector,"target_handle":"vector"}),
            json!({"id":format!("dep-{suffix}"),"source":dependency,"source_handle":"dependency_environment_sidecar","target":infer,"target_handle":"dependency_environment_sidecar"}),
        ]);
    }
    Ok(serde_json::from_value(
        json!({"nodes":nodes,"edges":edges}),
    )?)
}

async fn run(
    runtime: &EmbeddedRuntime,
    session: &str,
    inputs: Vec<WorkflowPortBinding>,
    targets: Vec<WorkflowOutputTarget>,
) -> ProbeResult<(WorkflowRunResponse, Vec<String>)> {
    let execution = async {
        let mut resumed_runs = Vec::new();
        let mut result = runtime
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: session.into(),
                workflow_semantic_version: "1.0.0".into(),
                inputs,
                output_targets: Some(targets),
                override_selection: None,
                // Native serial execution has no workflow deadline contract.
                // The outer caller watchdog cannot stop work or release custody.
                timeout_ms: None,
                priority: None,
            })
            .await;
        while matches!(
            result,
            Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
        ) {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(request) = runtime
                .workflow_service()
                .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?
                .into_iter()
                .find(|item| item.session_id == session)
            {
                if resumed_runs.len() >= 32 {
                    return Err(WorkflowServiceError::Internal(
                        "readiness-resume evidence bound exceeded".into(),
                    ));
                }
                resumed_runs.push(request.workflow_run_id.clone());
                result = runtime
                    .resume_workflow_execution_session_runtime_dependency_readiness(request)
                    .await;
            }
        }
        let response = result?;
        if resumed_runs
            .iter()
            .any(|id| id != &response.workflow_run_id)
        {
            return Err(WorkflowServiceError::Internal(
                "readiness resume changed workflow run identity".into(),
            ));
        }
        Ok((response, resumed_runs))
    };
    Ok(tokio::time::timeout(Duration::from_secs(30), execution).await??)
}

fn verify_vectors(response: &WorkflowRunResponse, expected: &Value) -> ProbeResult<()> {
    for (i, node) in ["vectors_a", "vectors_b"].iter().enumerate() {
        let vector = response
            .outputs
            .iter()
            .find(|o| &o.node_id == node)
            .and_then(|o| o.value.as_array())
            .ok_or("missing actual vector")?;
        let golden = expected[i].as_array().ok_or("missing reference vector")?;
        if vector.len() != golden.len()
            || vector
                .iter()
                .zip(golden)
                .any(|(a, b)| match (a.as_f64(), b.as_f64()) {
                    (Some(a), Some(b)) => !a.is_finite() || (a - b).abs() > 1e-5,
                    _ => true,
                })
        {
            return Err(
                "actual CPU output disagrees with independent Transformer reference".into(),
            );
        }
    }
    Ok(())
}

async fn cohort(
    runtime: &EmbeddedRuntime,
    root: &Path,
    name: &str,
    golden: &Value,
    texts: &[String; 2],
    preserve_failed_attempts: bool,
) -> ProbeResult<Vec<Value>> {
    let mut stage = "pair_authoring";
    let graph_session = author_graph(
        runtime,
        root,
        &mut stage,
        pair_graph()?,
        PAIR_WORKFLOW,
        &["deps_a", "deps_b"],
        true,
    )
    .await?;
    let mut runs = Vec::new();
    for i in 0..RUNS {
        let session = runtime
            .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
                workflow_id: PAIR_WORKFLOW.into(),
                usage_profile: None,
                keep_alive: false,
            })
            .await?;
        let samples_before = runtime.workflow_service().serial_ready_samples_for_test();
        let began = Instant::now();
        let result = run(
            runtime,
            &session.session_id,
            ["a", "b"]
                .iter()
                .enumerate()
                .map(|(j, s)| WorkflowPortBinding {
                    node_id: format!("prompt_{s}"),
                    port_id: "text".into(),
                    value: json!(texts[j]),
                })
                .collect(),
            ["a", "b"]
                .iter()
                .map(|s| WorkflowOutputTarget {
                    node_id: format!("vectors_{s}"),
                    port_id: "vector".into(),
                })
                .collect(),
        )
        .await;
        let wall_ms = began.elapsed().as_secs_f64() * 1000.0;
        let samples_after = runtime.workflow_service().serial_ready_samples_for_test();
        let close = runtime
            .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                session_id: session.session_id,
            })
            .await;
        if preserve_failed_attempts {
            // Preserve returned outputs and raw samples even when an oracle or
            // execution error ends the one admitted supplemental cohort.
            save(
                root,
                &format!("{name}-attempt-{i}.json"),
                &json!({"index":i,"wall_ms":wall_ms,
                    "response":result.as_ref().ok().map(|(response, _)| response),
                    "readiness_resume_run_ids":result.as_ref().ok().map(|(_, ids)| ids),
                    "execution_error":result.as_ref().err().map(ToString::to_string),
                    "close_error":close.as_ref().err().map(ToString::to_string),
                    "samples_before":samples_before,"samples_after":samples_after}),
            )?;
        }
        let (response, resumed_runs) = result?;
        close?;
        verify_vectors(&response, &golden["truncation_vectors"])?;
        let row = json!({"index":i,"response":response,"wall_ms":wall_ms,"samples_before":samples_before,"samples_after":samples_after,"readiness_resume_run_ids":resumed_runs});
        save(root, &format!("{name}-run-{i}.json"), &row)?;
        runs.push(row);
    }
    runtime
        .workflow_service()
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: graph_session,
        })
        .await?;
    Ok(runs)
}

fn collect_events(root: &Path, name: &str) -> ProbeResult<Vec<Value>> {
    let ledger = SqliteDiagnosticsLedger::open(root.join(format!("{name}-diagnostics.db")))?;
    let mut records = Vec::new();
    let mut after = 0;
    loop {
        let page = ledger.diagnostic_events_after(after, 500)?;
        let done = page.len() < 500;
        if let Some(last) = page.last() {
            after = last.event_seq;
        }
        records.extend(page);
        if records.len() >= 4000 {
            return Err("ledger evidence exceeds bounded collector".into());
        }
        if done {
            break;
        }
    }
    let rows: Vec<_> = records.into_iter().map(|r| {
        let payload: Value = serde_json::from_str(&r.payload_json)?;
        Ok(json!({"event_seq":r.event_seq,"workflow_run_id":r.workflow_run_id,"node_id":r.node_id,"occurred_at_ms":r.occurred_at_ms,"payload":payload}))
    }).collect::<Result<_,serde_json::Error>>()?;
    save(root, &format!("{name}-events.json"), &rows)?;
    Ok(rows)
}

fn independent_oracle(runs: &[Value], proposals: &[Value], events: &[Value]) -> ProbeResult<Value> {
    let mut ranked = 0;
    let mut second = 0;
    for row in runs {
        let id = &row["response"]["workflow_run_id"];
        let decisions: Vec<_> = proposals
            .iter()
            .filter(|p| &p["workflow_run_id"] == id)
            .collect();
        if decisions.len() != 1 {
            return Err(format!(
                "expected one genuine two-Ready proposal per run, got {}",
                decisions.len()
            )
            .into());
        }
        let decision = decisions[0];
        let ids = decision["task_ids"].as_array().ok_or("missing task pair")?;
        let raw: RawSamples = serde_json::from_value(row["samples_before"].clone())?;
        let mut maxima = [0u64; 2];
        let mut counts = [0usize; 2];
        for (i, id) in ids.iter().enumerate() {
            for sample in raw.iter().filter(|s| Some(s.0.as_str()) == id.as_str()) {
                if sample.3 >= 59_000_000_000 {
                    return Err(
                        "raw samples are too old for an unambiguous freshness oracle".into(),
                    );
                }
                maxima[i] = maxima[i].max(sample.2);
                counts[i] += 1;
            }
        }
        if decision["sample_counts"] != json!(counts) || decision["maxima_ns"] != json!(maxima) {
            return Err("selector summary differs from independent raw-sample aggregation".into());
        }
        let selected = if counts.iter().all(|n| *n >= 3) {
            if decision["reason"] != "ranked" {
                return Err("fresh comparable history was not ranked".into());
            }
            ranked += 1;
            usize::from(maxima[1] < maxima[0])
        } else {
            if decision["reason"] != "insufficient_fresh_samples" {
                return Err("unexpected FIFO reason".into());
            }
            0
        };
        if decision["selected_position"] != json!(selected) {
            return Err("ranking/tie oracle failed".into());
        }
        second += usize::from(selected == 1);
        let starts: Vec<_> = events
            .iter()
            .filter(|e| {
                &e["workflow_run_id"] == id
                    && e["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                    && e["payload"]["transition"] == "started"
                    && ids.contains(&e["payload"]["scheduler_task_id"])
            })
            .collect();
        if starts.len() != 2
            || starts[0]["payload"]["scheduler_task_id"] != ids[selected]
            || starts[1]["payload"]["scheduler_task_id"] != ids[1 - selected]
        {
            return Err("actual Start order does not match the bounded proposal".into());
        }
        let before: RawSamples = serde_json::from_value(row["samples_before"].clone())?;
        let after: RawSamples = serde_json::from_value(row["samples_after"].clone())?;
        if after.len() != before.len() + 2 {
            return Err("both actual cleanup-qualified samples were not retained".into());
        }
    }
    Ok(
        json!({"ranked_runs":ranked,"second_position_starts":second,"first_two_order_matches_raw_sample_maxima":true,
        "second_position_claim":if second > 0 {"passed"} else {"pending_no_observed_second_winner"}}),
    )
}

pub(super) async fn qualify_m2(root: &Path) -> ProbeResult<()> {
    m2_lookahead_probe::qualify(root).await
}

pub(super) async fn qualify(root: &Path) -> ProbeResult<()> {
    qualify_workload(root, false).await
}

pub(super) async fn qualify_c3_768b(root: &Path) -> ProbeResult<()> {
    qualify_workload(root, true).await
}

async fn qualify_workload(root: &Path, c3_768b: bool) -> ProbeResult<()> {
    log::set_logger(&PROPOSALS).map_err(|_| "example requires exclusive logger ownership")?;
    log::set_max_level(log::LevelFilter::Info);
    seed_fixture(root, MODEL_ID, Some(&["model.safetensors"]))?;
    let golden: Value = serde_json::from_slice(&std::fs::read(
        root.join("shared-resources/models")
            .join(MODEL_ID)
            .join("golden.json"),
    )?)?;
    let texts: [String; 2] = if c3_768b {
        // Frozen published protocol c3-768b-v1; no configurable size or retry.
        ["hello world ".repeat(64), "hello".into()]
    } else {
        serde_json::from_value(golden["truncation_texts"].clone())?
    };
    if c3_768b {
        save(
            root,
            "workload.json",
            &json!({"protocol_id":"c3-768b-v1","texts":texts,
                "utf8_bytes":[texts[0].len(),texts[1].len()],
                "fixed_runs_per_mode":RUNS,"original_negative_evidence_preserved":true}),
        )?;
    }
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
        .ok_or("Pumas did not provide a load target")?;
    let registry = Arc::new(RuntimeRegistry::new());
    registry.configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
        runtime_id: "candle".into(),
        model_id: target.local_load_path.clone(),
        requirements: RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(RESIDENT_ENVELOPE),
            // This exact calibrated backend loads all tensors on Device::Cpu.
            // Keep the ordinary conservative task VRAM charge; declare its
            // retained VRAM explicitly as known zero instead of omitting it.
            RuntimeReservationResourceClaim::vram_bytes(0),
        ]),
    }])?;
    let gateway = Arc::new(inference::InferenceGateway::new_calibrated_candle_cpu(
        inference::CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60))?,
    )?);
    let selector = Arc::new(PumasSelectorAccess::Owner(api));
    let baseline = compose(
        root,
        "baseline",
        gateway.clone(),
        registry.clone(),
        selector.clone(),
        false,
    )
    .await?;
    let mut stage = "bootstrap";
    let edit = author_workflow(&baseline, root, &mut stage, true).await?;
    let retaining = baseline
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: WORKFLOW_ID.into(),
            usage_profile: None,
            keep_alive: true,
        })
        .await?;
    let (bootstrap, bootstrap_resumed_runs) = run(
        &baseline,
        &retaining.session_id,
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
    gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await?;
    save(
        root,
        "bootstrap.json",
        &json!({"response":bootstrap,"readiness_resume_run_ids":bootstrap_resumed_runs,"registry":registry.snapshot()}),
    )?;
    let serial = compose(
        root,
        "serial",
        gateway.clone(),
        registry.clone(),
        selector,
        true,
    )
    .await?;
    // Keep the same two live compositions/producers during both measurements.
    let baseline_result = cohort(&baseline, root, "baseline", &golden, &texts, c3_768b).await;
    if c3_768b && baseline_result.is_err() {
        save(
            root,
            "baseline-execution-status.json",
            &json!({"ok":false,
            "error":baseline_result.as_ref().err().map(ToString::to_string),
            "shutdown_qualified":false,"cohort_restart_allowed":false}),
        )?;
        collect_events(root, "baseline")?;
        save(
            root,
            "registry-at-baseline-failure.json",
            &registry.snapshot(),
        )?;
    }
    let baseline_runs = baseline_result?;
    let serial_result = cohort(&serial, root, "serial", &golden, &texts, c3_768b).await;
    save(
        root,
        "serial-execution-status.json",
        &json!({"ok":serial_result.is_ok(),"error":serial_result.as_ref().err().map(ToString::to_string)}),
    )?;
    let (proposal_lines, overflow) = {
        let log = PROPOSALS.0.lock().expect("proposal collector");
        (log.0.clone(), log.2)
    };
    std::fs::write(
        root.join("serial-proposals.jsonl"),
        proposal_lines.join("\n") + "\n",
    )?;
    let events = collect_events(root, "serial")?;
    collect_events(root, "baseline")?;
    save(root, "registry-after-serial.json", &registry.snapshot())?;
    baseline
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: retaining.session_id,
        })
        .await?;
    baseline
        .workflow_service()
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: edit,
        })
        .await?;
    serial.shutdown().await?;
    baseline.shutdown().await?;
    let final_registry = registry.snapshot();
    save(root, "registry-after-shutdown.json", &final_registry)?;
    let model_ref: inference::PumasModelRef = serde_json::from_value(
        json!({"model_id":package.model_ref.model_id,"selected_artifact_id":package.model_ref.selected_artifact_id}),
    )?;
    if !final_registry.reservations.is_empty()
        || gateway.resident_cpu_serial_owner(&model_ref).is_some()
    {
        return Err("shutdown retained a reservation or native CPU owner".into());
    }
    let serial_runs = serial_result?;
    if overflow {
        return Err("bounded proposal collector overflow; ordering claim unqualified".into());
    }
    let proposals: Vec<Value> = proposal_lines
        .iter()
        .map(|line| serde_json::from_str(line))
        .collect::<Result<_, _>>()?;
    let oracle = independent_oracle(&serial_runs, &proposals, &events)?;
    let mut report = json!({"scope":"actual_hosted_retained_cpu_first_two_ready","backend":"candle","device":"cpu_f32","threads":1,
        "model":MODEL_ID,"training":"untrained","weights_bytes":6464,"model_acquisition_bytes":0,
        "declared_resident_ram_bytes":RESIDENT_ENVELOPE,"resident_bytes_are_measured":false,"declared_resident_vram_bytes":0,"vram_basis":"calibrated Candle embedding tensors are Device::Cpu only",
        "fixed_runs_per_mode":RUNS,"baseline":baseline_runs,"serial":serial_runs,"oracle":oracle,
        "default_policy_changed":false,"gui_qualified":false,"shutdown_ok":true,
        "shutdown_scope":"shutdown calls returned; no reservation or native serial owner; registry may retain stopping state and declared resident accounting",
        "decision_ns_scope":"mode.select only; excludes pair capture, log projection and Start fencing",
        "profile":if cfg!(debug_assertions) {"dev"} else {"release"}});
    if c3_768b {
        report["supplemental_protocol"] = json!("c3-768b-v1");
        report["workload_texts"] = json!(texts);
        report["original_negative_evidence_preserved"] = json!(true);
    }
    save(root, "serial-result.json", &report)?;
    println!("{}", serde_json::to_string_pretty(&report["oracle"])?);
    if report["oracle"]["second_position_claim"] != "passed" {
        return Err(
            "fixed cohort did not demonstrate second-position selection; claim remains pending"
                .into(),
        );
    }
    Ok(())
}

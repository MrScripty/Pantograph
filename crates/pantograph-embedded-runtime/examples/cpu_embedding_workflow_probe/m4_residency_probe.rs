//! Fixed M4 qualification producer. No execution is implied by compiling it.
//! Uses the sealed facade, public sessions and explicit bounded diagnostics
//! projection refresh/readback; no native owner authority escapes.
use super::*;
use pantograph_embedded_runtime::{
    EmbeddedSerialCpuModelDeclaration, EmbeddedSerialCpuResidency, EmbeddedSerialCpuResidencyConfig,
};
use pantograph_runtime_attribution::{
    BucketSelection, ClientRegistrationRequest, ClientSessionOpenRequest,
};
use pantograph_workflow_service::workflow::WorkflowSchedulerTaskStateReadModelQueryRequest;
use pantograph_workflow_service::*;
use std::process::Command;
use std::sync::Mutex;

const PROTOCOL: &str = "m4-serial-residency-v1";
const FIXED_WORK: [usize; 5] = [8, 8, 12, 12, 8];
const MIB: u64 = 1024 * 1024;
const RAM_CAPACITY: u64 = 1104 * MIB;
const VRAM_CAPACITY: u64 = 1024 * MIB;
const RESIDENT_RAM: u64 = 32 * MIB;
const LOAD_PEAK_RAM: u64 = 48 * MIB;
const PRESSURE_RAM_CAPACITY: u64 = 80 * MIB;
const WATCHDOG: Duration = Duration::from_secs(60);

struct BoundedLog {
    rows: Vec<Value>,
    bytes: usize,
    overflow: bool,
}
struct NativeLog(Mutex<BoundedLog>);
static NATIVE_LOG: NativeLog = NativeLog(Mutex::new(BoundedLog {
    rows: Vec::new(),
    bytes: 0,
    overflow: false,
}));
impl log::Log for NativeLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        matches!(
            metadata.target(),
            "workflow::serial_ready" | "embedded::serial_cpu_residency"
        )
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = record.args().to_string();
        let mut buffer = self.0.lock().expect("isolated M4 evidence log");
        if buffer.rows.len() >= 128 || line.len() >= 8192 || buffer.bytes + line.len() > 1024 * 1024
        {
            buffer.overflow = true;
            return;
        }
        buffer.bytes += line.len();
        buffer.rows.push(
            json!({"target":record.target(), "record":serde_json::from_str::<Value>(&line)
            .unwrap_or_else(|_|json!({"raw_message":line}))}),
        );
    }
    fn flush(&self) {}
}

fn workflow(width: usize) -> String {
    format!("cpu-m4-bert-{width}")
}
fn model(width: usize) -> &'static str {
    match width {
        8 => MODEL_ID,
        12 => rich_workflow::SECOND_MODEL,
        _ => unreachable!("fixed fixture widths"),
    }
}

fn fixture_contract(root: &Path) -> ProbeResult<Value> {
    if std::env::var_os("PANTOGRAPH_CPU_EMBEDDING_FIXTURE_ROOT").is_some() {
        return Err(
            "M4 uses the exact committed fixture root; fixture overrides are refused".into(),
        );
    }
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/cpu_embedding_workflow_probe/m4_fixture_contract.py");
    let output = root.join("fixture-admission.json");
    let result = Command::new("python3").arg(script).arg(&output).output()?;
    save(
        root,
        "fixture-check-process.json",
        &json!({"success":result.status.success(),
        "stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)}),
    )?;
    if !result.status.success() {
        return Err("read-only M4 fixture admission failed".into());
    }
    Ok(serde_json::from_slice(&std::fs::read(output)?)?)
}

fn provenance(root: &Path) -> ProbeResult<()> {
    // Hashing uses only Python's standard library; no new native dependency or
    // model/reference invocation is introduced. The environment allowlist omits
    // credentials and unrelated personal configuration.
    let script = r#"import hashlib,json,os,pathlib,platform,subprocess,sys
repo=pathlib.Path(sys.argv[1]); binary=pathlib.Path(sys.argv[2])
def git(*args): return subprocess.check_output(['git',*args],cwd=repo,text=True).strip()
def read(path):
 p=pathlib.Path(path)
 return p.read_text().strip() if p.exists() else None
result={'source_head':git('rev-parse','HEAD'),'source_head_tree':git('rev-parse','HEAD^{tree}'),
 'source_status':git('status','--porcelain=v1'),'source_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','--binary','HEAD'],cwd=repo)).hexdigest(),
 'binary_path':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
 'platform':platform.platform(),'cpu_affinity':sorted(os.sched_getaffinity(0)),
 'cpu_max':read('/sys/fs/cgroup/cpu.max'),'memory_max':read('/sys/fs/cgroup/memory.max'),
 'environment':{k:os.environ.get(k) for k in ['TOKENIZERS_PARALLELISM','RAYON_NUM_THREADS','OMP_NUM_THREADS','PUMAS_REGISTRY_DB_PATH']},
 'pumas_registry_scope':'shared process metadata SQLite; each facade owns distinct project/library/appdata and a private native gateway/RuntimeRegistry; no env mutation'}
print(json.dumps(result))"#;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(repo)
        .arg(std::env::current_exe()?)
        .output()?;
    if !output.status.success() {
        return Err("M4 source/binary/environment provenance failed".into());
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    save(root, "provenance.json", &value)
}

fn graph(width: usize, text: &str) -> ProbeResult<WorkflowGraph> {
    Ok(serde_json::from_value(json!({"nodes":[
        {"id":"prompt","node_type":"text-input","position":{"x":0,"y":100},"data":{"text":text}},
        {"id":"infer","node_type":"llm-inference","position":{"x":300,"y":100},"data":{
            "task_kind":"embedding","runtime":"candle","device":"cpu",
            "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":model(width),"selected_artifact_id":"main"}}},
        {"id":"vectors","node_type":"vector-output","position":{"x":700,"y":100},"data":{}},
        {"id":"deps","node_type":"dependency-environment","position":{"x":300,"y":400},"data":{"mode":"manual"}}
    ],"edges":[
        {"id":"text-to-infer","source":"prompt","source_handle":"text","target":"infer","target_handle":"text"},
        {"id":"infer-to-vector","source":"infer","source_handle":"embedding","target":"vectors","target_handle":"vector"},
        {"id":"deps-to-infer","source":"deps","source_handle":"dependency_environment_sidecar","target":"infer","target_handle":"dependency_environment_sidecar"}
    ]}))?)
}

async fn author(
    facade: &EmbeddedSerialCpuResidency,
    root: &Path,
    width: usize,
    reference: &Value,
) -> ProbeResult<String> {
    let name = workflow(width);
    let mut graph = graph(
        width,
        reference["text"].as_str().ok_or("missing reference text")?,
    )?;
    let registry = NodeRegistry::new();
    for node in &mut graph.nodes {
        if node.node_type != "llm-inference" {
            node.data["definition"] = serde_json::to_value(
                registry
                    .get_definition(&node.node_type)
                    .ok_or("missing built-in node definition")?,
            )?;
        }
    }
    let preview = facade
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: None,
        })
        .await?;
    let validation = facade
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id.clone(),
                graph_revision: preview.graph_revision.parse()?,
            },
        )
        .await?;
    save(
        root,
        &format!("{name}-descriptor-validation.json"),
        &validation,
    )?;
    let inference = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .ok_or("missing inference node")?;
    let projection = validation
        .node_projections
        .iter()
        .find(|projection| projection.node_id.as_str() == "infer")
        .ok_or("missing backend inference descriptor")?;
    inference.data["inference_interface_snapshot"] =
        serde_json::to_value(authored_snapshot_from_descriptor(&projection.descriptor)?)?;
    facade
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: preview.session_id,
        })
        .await?;
    facade.workflow_graph_save(WorkflowGraphSaveRequest {
        name: name.clone(),
        graph: graph.clone(),
    })?;
    let edit = facade
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: Some(name.clone()),
        })
        .await?;
    let validation = facade
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: edit.session_id.clone(),
                graph_revision: edit.graph_revision.parse()?,
            },
        )
        .await?;
    save(
        root,
        &format!("{name}-executable-validation.json"),
        &validation,
    )?;
    if validation
        .summary
        .summary
        .as_ref()
        .map(|summary| &summary.status)
        != Some(&DraftGraphValidationStatus::Executable)
    {
        return Err("M4 graph validation is not executable".into());
    }
    let validation_id = validation
        .summary
        .validation_session_id
        .ok_or("missing executable validation identity")?;
    let resolved = facade
        .workflow_graph_resolve_dependency_environment_action_intent(
            DependencyEnvironmentActionIntent {
                contract_version: 1,
                graph_session_id: edit.session_id.parse()?,
                graph_revision: edit.graph_revision.parse()?,
                validation_session_id: Some(validation_id.clone()),
                target_node_id: "deps".parse()?,
                action: DependencyEnvironmentAction::Resolve,
            },
        )
        .await?;
    save(
        root,
        &format!("{name}-dependency-resolution.json"),
        &resolved,
    )?;
    if resolved.status != DependencyEnvironmentActionIntentStatus::RequestReady {
        return Err("M4 public dependency Resolve is blocking".into());
    }
    let published = facade
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: name,
                workflow_semantic_version: "1.0.0".into(),
                graph_session_id: edit.session_id.clone(),
                validation_session_id: Some(validation_id),
                validation_snapshot_id: None,
            },
        )
        .await?;
    save(
        root,
        &format!("bert-{width}-published-validation.json"),
        published.as_record(),
    )?;
    Ok(edit.session_id)
}

async fn construct(root: &Path, ram: u64) -> ProbeResult<EmbeddedSerialCpuResidency> {
    std::fs::create_dir(root)?;
    seed_fixture(root, MODEL_ID, Some(&["model.safetensors"]))?;
    seed_fixture(
        root,
        rich_workflow::SECOND_MODEL,
        Some(&["model.safetensors"]),
    )?;
    for directory in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
        "app-data",
    ] {
        std::fs::create_dir_all(root.join(directory))?;
    }
    let models = [8, 12]
        .into_iter()
        .map(|width| EmbeddedSerialCpuModelDeclaration {
            model_ref: serde_json::from_value(
                json!({"model_id":model(width),"selected_artifact_id":"main"}),
            )
            .expect("fixed model reference"),
            resident_ram_bytes: RESIDENT_RAM,
            load_peak_ram_bytes: LOAD_PEAK_RAM,
            load_peak_vram_bytes: 0,
        })
        .collect();
    Ok(
        EmbeddedSerialCpuResidency::new(EmbeddedSerialCpuResidencyConfig {
            runtime: EmbeddedRuntimeConfig::new(root.join("app-data"), root.to_path_buf()),
            pumas_library_root: root.to_path_buf(),
            models,
            ram_capacity_bytes: ram,
            vram_capacity_bytes: VRAM_CAPACITY,
            ram_safety_margin_bytes: 0,
            vram_safety_margin_bytes: 0,
            cpu_threads: 1,
            readiness_poll_interval: Duration::from_millis(50),
        })
        .await?,
    )
}

async fn sessions(
    facade: &EmbeddedSerialCpuResidency,
    root: &Path,
    widths: &[usize],
) -> ProbeResult<Vec<(usize, WorkflowExecutionSessionCreateResponse)>> {
    let registered = facade.register_attribution_client(ClientRegistrationRequest {
        display_name: Some("M4 fixed CPU qualification".into()),
        metadata_json: None,
    })?;
    let opened = facade.open_client_session(ClientSessionOpenRequest {
        credential: registered.credential_proof_request(),
        takeover: false,
        reason: Some(PROTOCOL.into()),
    })?;
    let mut result = Vec::new();
    for &width in widths {
        let session = facade
            .create_attributed_workflow_execution_session(
                WorkflowExecutionSessionAttributedCreateRequest {
                    workflow_id: workflow(width),
                    usage_profile: None,
                    keep_alive: false,
                    attribution: WorkflowExecutionSessionAttributionRequest {
                        credential: registered.credential_proof_request(),
                        client_session_id: opened.session.client_session_id.as_str().into(),
                        bucket_selection: BucketSelection::Default,
                    },
                },
            )
            .await?;
        // No registration credential or proof is serialized into evidence.
        save(
            root,
            &format!("bert-{width}-authenticated-session.json"),
            &session,
        )?;
        result.push((width, session));
    }
    Ok(result)
}

async fn run(
    facade: &EmbeddedSerialCpuResidency,
    session: &str,
    text: &str,
) -> ProbeResult<(WorkflowRunResponse, Vec<String>)> {
    let execution = async {
        let mut resumes = Vec::new();
        let mut result = facade
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: session.into(),
                workflow_semantic_version: "1.0.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "prompt".into(),
                    port_id: "text".into(),
                    value: json!(text),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vectors".into(),
                    port_id: "vector".into(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: Some(0),
            })
            .await;
        while matches!(
            result,
            Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
        ) {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(request) = facade
                .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?
                .into_iter()
                .find(|request| request.session_id == session)
            {
                if resumes.len() >= 32 {
                    return Err("M4 readiness resume bound exceeded".into());
                }
                resumes.push(request.workflow_run_id.clone());
                result = facade
                    .resume_workflow_execution_session_runtime_dependency_readiness(request)
                    .await;
            }
        }
        let response = result?;
        if resumes.iter().any(|run| run != &response.workflow_run_id) {
            return Err("M4 resume changed active run identity".into());
        }
        Ok((response, resumes))
    };
    tokio::time::timeout(WATCHDOG, execution).await?
}

fn verify_vector(response: &WorkflowRunResponse, reference: &Value) -> ProbeResult<()> {
    let actual = response
        .outputs
        .iter()
        .find(|port| port.node_id == "vectors" && port.port_id == "vector")
        .and_then(|port| port.value.as_array())
        .ok_or("M4 vector output missing")?;
    let expected = reference["golden_vector"]
        .as_array()
        .ok_or("M4 reference vector missing")?;
    let atol = reference["atol"]
        .as_f64()
        .ok_or("M4 reference atol missing")?;
    let rtol = reference["rtol"]
        .as_f64()
        .ok_or("M4 reference rtol missing")?;
    if actual.len() != expected.len()
        || actual.iter().zip(expected).any(|(actual, expected)| {
            match (actual.as_f64(), expected.as_f64()) {
                (Some(actual), Some(expected)) => {
                    !actual.is_finite() || (actual - expected).abs() > atol + rtol * expected.abs()
                }
                _ => true,
            }
        })
    {
        return Err(
            "M4 actual output disagrees with independent committed Transformers golden".into(),
        );
    }
    Ok(())
}

async fn retain_readback(
    facade: &EmbeddedSerialCpuResidency,
    root: &Path,
    index: usize,
    session: &str,
    response: &WorkflowRunResponse,
) -> ProbeResult<Value> {
    // Inspection reads derived tables. Its projection_batch_size validates a
    // bound but does not advance them. Refresh only diagnostics, explicitly,
    // before relying on the projected artifact identity.
    let refresh = facade.workflow_diagnostics_projection_refresh(
        WorkflowDiagnosticsProjectionRefreshRequest {
            projections: vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus,
                WorkflowDiagnosticsProjectionKind::IoArtifact,
            ],
            workflow_run_id: Some(response.workflow_run_id.clone()),
            workflow_id: None,
            reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
            batch_size: 500,
        },
    )?;
    save(
        root,
        &format!("step-{index}-projection-refresh.json"),
        &refresh,
    )?;
    let inspection = facade.workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
        workflow_run_id: response.workflow_run_id.clone(),
        artifact_limit: Some(128),
        projection_batch_size: Some(500),
    })?;
    save(root, &format!("step-{index}-inspection.json"), &inspection)?;
    let boundary = read_projection_boundary(root)?;
    save(
        root,
        &format!("step-{index}-projection-boundary.json"),
        &boundary,
    )?;
    verify_projection_catchup(&refresh, &boundary)?;
    let task_states = facade
        .workflow_get_scheduler_task_state_read_models(
            WorkflowSchedulerTaskStateReadModelQueryRequest {
                session_id: session.into(),
                workflow_run_id: response.workflow_run_id.clone(),
            },
        )
        .await?;
    let vector = response
        .outputs
        .iter()
        .find(|port| port.node_id == "vectors" && port.port_id == "vector")
        .ok_or("returned M4 vector absent")?;
    let mut bodies = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for artifact in inspection.io_artifacts.iter().filter(|artifact| {
        artifact.producer_node_id.as_deref() == Some("vectors")
            && artifact.producer_port_id.as_deref() == Some("vector")
            && matches!(
                artifact.artifact_role.as_str(),
                "workflow_output" | "node_output"
            )
    }) {
        if artifact.workflow_run_id.as_str() != response.workflow_run_id {
            return Err("retained artifact belongs to another M4 run".into());
        }
        if !seen.insert(artifact.payload_artifact_id.clone()) {
            continue;
        }
        let body = facade.workflow_read_artifact_body(ArtifactReadRequest {
            artifact_id: artifact.payload_artifact_id.clone(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        })?;
        let value: Value = serde_json::from_slice(&body.body)?;
        if value != vector.value {
            return Err("M4 retained output differs from public returned vector".into());
        }
        bodies.push(json!({"artifact":artifact,"read":body.response,"body_blake3":blake3::hash(&body.body).to_hex().to_string(),"value":value}));
    }
    if bodies.is_empty() {
        return Err("M4 retained vector artifact missing".into());
    }
    Ok(
        json!({"projection_refresh":refresh,"projection_boundary":boundary,"inspection":inspection,"task_states":task_states,"retained_vectors":bodies}),
    )
}

fn read_projection_boundary(root: &Path) -> ProbeResult<Value> {
    use pantograph_diagnostics_ledger::DiagnosticEventKind::*;
    let ledger = SqliteDiagnosticsLedger::open(root.join("app-data/serial-cpu-diagnostics.db"))?;
    let events = ledger.diagnostic_events_after(0, 500)?;
    if events.len() >= 500 {
        return Err("M4 projection evidence history exceeds its single-page bound".into());
    }
    let (mut detail, mut node, mut io) = (0, 0, 0);
    // These relevant-event sets follow the existing three ledger projection
    // readers. Their cursors need not reach the latest unrelated event.
    for event in &events {
        if matches!(
            event.event_kind,
            SchedulerEstimateProduced
                | SchedulerQueuePlacement
                | SchedulerQueueControl
                | SchedulerRunDelayed
                | SchedulerModelLifecycleChanged
                | SchedulerRunAdmitted
                | RunStarted
                | RunTerminal
                | RunSnapshotAccepted
                | NodeExecutionStatus
                | InferenceExecutionDiagnosticObserved
                | IoArtifactObserved
                | DiagnosticErrorOccurred
        ) {
            detail = event.event_seq;
        }
        if matches!(
            event.event_kind,
            NodeExecutionStatus
                | SchedulerTaskAttemptLifecycleChanged
                | DiagnosticErrorOccurred
                | InferenceExecutionDiagnosticObserved
        ) {
            node = event.event_seq;
        }
        if matches!(
            event.event_kind,
            IoArtifactObserved | RetentionArtifactStateChanged
        ) {
            io = event.event_seq;
        }
    }
    Ok(
        json!({"visible_event_count":events.len(),"visible_latest_event_seq":events.last().map(|event|event.event_seq).unwrap_or(0),
        "required_cursors":{"run_detail":detail,"node_status":node,"io_artifact":io},"single_page_limit":500}),
    )
}

fn verify_projection_catchup(
    refresh: &WorkflowDiagnosticsProjectionRefreshResponse,
    boundary: &Value,
) -> ProbeResult<()> {
    if !refresh.failed.is_empty() || refresh.advanced.len() != 3 {
        return Err(
            "M4 bounded diagnostics projection refresh did not advance every requested projection"
                .into(),
        );
    }
    for (kind, name) in [
        (WorkflowDiagnosticsProjectionKind::RunDetail, "run_detail"),
        (WorkflowDiagnosticsProjectionKind::NodeStatus, "node_status"),
        (WorkflowDiagnosticsProjectionKind::IoArtifact, "io_artifact"),
    ] {
        let states: Vec<_> = refresh
            .advanced
            .iter()
            .filter(|advance| advance.projection_kind == kind)
            .collect();
        if states.len() != 1
            || states[0].projection_state.status
                != pantograph_diagnostics_ledger::ProjectionStatus::Current
            || Some(states[0].projection_state.last_applied_event_seq)
                != boundary["required_cursors"][name].as_i64()
        {
            return Err(format!(
                "M4 projection {name} did not reach its exact visible relevant-event boundary"
            )
            .into());
        }
    }
    Ok(())
}

fn collect_events(root: &Path) -> ProbeResult<Vec<Value>> {
    let ledger = SqliteDiagnosticsLedger::open(root.join("app-data/serial-cpu-diagnostics.db"))?;
    let mut rows = Vec::new();
    let mut after = 0;
    loop {
        let page = ledger.diagnostic_events_after(after, 500)?;
        let done = page.len() < 500;
        if let Some(last) = page.last() {
            after = last.event_seq;
        }
        for event in page {
            let payload: Value = serde_json::from_str(&event.payload_json)?;
            rows.push(
                json!({"event_seq":event.event_seq,"workflow_run_id":event.workflow_run_id,
                "node_id":event.node_id,"occurred_at_ms":event.occurred_at_ms,"payload":payload}),
            );
        }
        if rows.len() > 4000 {
            return Err("M4 diagnostics event bound exceeded".into());
        }
        if done {
            break;
        }
    }
    save(root, "events.json", &rows)?;
    Ok(rows)
}

async fn owner_protocol(
    top: &Path,
    name: &str,
    work: &[usize],
    references: &Value,
    ram: u64,
    pressure: bool,
) -> ProbeResult<Value> {
    let log_start = NATIVE_LOG.0.lock().expect("M4 evidence log").rows.len();
    let root = top.join(name);
    let facade = construct(&root, ram).await?;
    let mut opened = Vec::new();
    let mut runs = Vec::new();
    let mut stage = "author";
    let protocol:ProbeResult<()> = async {
        let widths:Vec<_>=[8,12].into_iter().filter(|width|work.contains(width)).collect();
        for &width in &widths {
            let edit=author(&facade,&root,width,&references[format!("bert-{width}")]).await?;
            opened.push((width,None::<WorkflowExecutionSessionCreateResponse>,edit));
        }
        stage="authenticated_sessions";
        for (width,session) in sessions(&facade,&root,&widths).await? {
            opened.iter_mut().find(|row|row.0==width).ok_or("M4 authored workflow missing")?.1=Some(session);
        }
        for (index,&width) in work.iter().enumerate() {
            stage="public_run";
            let reference=&references[format!("bert-{width}")];
            let session=opened.iter().find(|row|row.0==width).and_then(|row|row.1.as_ref()).ok_or("M4 session missing")?;
            let before=facade.registry_snapshot();
            let diagnostics_before=facade.residency_diagnostics();
            let began=Instant::now();
            let outcome=run(&facade,&session.session_id,reference["text"].as_str().ok_or("missing reference text")?).await;
            let row=json!({"index":index,"model_alias":format!("bert-{width}"),"model_id":model(width),
                "session_id":session.session_id,"response":outcome.as_ref().ok().map(|(response,_)|response),
                "readiness_resume_run_ids":outcome.as_ref().ok().map(|(_,resumes)|resumes),
                "error":outcome.as_ref().err().map(ToString::to_string),"submit_to_result_elapsed_ns":began.elapsed().as_nanos(),
                "submit_to_result_is_native_phase":false,"registry_before":before,"registry_after":facade.registry_snapshot(),
                "diagnostics_before":diagnostics_before,"diagnostics_after":facade.residency_diagnostics()});
            save(&root,&format!("step-{index}.json"),&row)?;
            runs.push(row);
            if pressure {
                if outcome.is_ok() {return Err("M4 fixed pressure request unexpectedly executed".into());}
                let diagnostics=facade.residency_diagnostics().ok_or("M4 diagnostics missing")?;
                let snapshot=facade.registry_snapshot();
                if diagnostics.loads!=0||diagnostics.retains!=0||diagnostics.stops!=0||diagnostics.forward_drains!=0||!snapshot.reservations.is_empty()
                    ||snapshot.runtimes.iter().any(|runtime|runtime.model_resource_residency.is_some()) {
                    return Err("M4 pressure refusal mutated native/load/reservation state".into());
                }
            } else {
                let (response,_)=outcome?;
                verify_vector(&response,reference)?;
                let retained=retain_readback(&facade,&root,index,&session.session_id,&response).await?;
                save(&root,&format!("step-{index}-retained-readback.json"),&retained)?;
                let snapshot=facade.registry_snapshot();
                if snapshot.reservations.len()!=1||snapshot.runtimes.iter().filter(|runtime|runtime.model_resource_residency.is_some()).count()!=1 {
                    return Err("M4 task ACK did not leave exactly its idle guard and resident owner".into());
                }
            }
        }
        Ok(())
    }.await;
    let mut close_rows = Vec::new();
    for (_, session, edit) in opened {
        if let Some(session) = session {
            let close = facade
                .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                    session_id: session.session_id.clone(),
                })
                .await;
            close_rows.push(json!({"session_id":session.session_id,"kind":"execution_session","ok":close.is_ok(),
                "error":close.as_ref().err().map(ToString::to_string)}));
        }
        let close = facade
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit.clone(),
            })
            .await;
        close_rows.push(
            json!({"edit_session_id":edit,"kind":"graph_session","ok":close.is_ok(),
            "error":close.as_ref().err().map(ToString::to_string)}),
        );
    }
    let before_shutdown = facade.registry_snapshot();
    // The facade retains its teardown supervisor when this borrowed waiter is
    // dropped. A caller watchdog records uncertainty and ends the cohort; it
    // supplies no stop/ACK evidence and never permits another owner to start.
    let shutdown_wait = tokio::time::timeout(WATCHDOG, facade.shutdown()).await;
    let shutdown_wait_timed_out = shutdown_wait.is_err();
    let shutdown: ProbeResult<()> = match shutdown_wait {
        Ok(result) => result.map_err(Into::into),
        Err(error) => Err(format!(
            "M4 shutdown watchdog elapsed; native stop/ACK outcome uncertain: {error}"
        )
        .into()),
    };
    let final_registry = facade.registry_snapshot();
    let final_diagnostics = facade.residency_diagnostics();
    save(&root, "session-closes.json", &close_rows)?;
    let events = collect_events(&root);
    let native_records = NATIVE_LOG.0.lock().expect("M4 evidence log").rows[log_start..].to_vec();
    save(&root, "native-and-service-log.json", &native_records)?;
    let result = json!({"owner":name,"fixed_work":work,"stage":stage,"rows":runs,"execution_ok":protocol.is_ok(),
        "execution_error":protocol.as_ref().err().map(ToString::to_string),"session_closes":close_rows,
        "registry_before_shutdown":before_shutdown,"registry_after_shutdown":final_registry,
        "diagnostics_after_shutdown":final_diagnostics,"shutdown_ok":shutdown.is_ok(),
        "shutdown_wait_timed_out":shutdown_wait_timed_out,"shutdown_watchdog_is_native_phase":false,
        "shutdown_error":shutdown.as_ref().err().map(ToString::to_string),"event_count":events.as_ref().ok().map(Vec::len),
        "event_error":events.as_ref().err().map(ToString::to_string),"native_records":native_records});
    save(&root, "owner-result.json", &result)?;
    // Drop all old facade/Pumas/native owners before constructing the next one.
    drop(facade);
    protocol?;
    shutdown?;
    let events = events?;
    if close_rows.iter().any(|row| row["ok"] != true)
        || !final_registry.reservations.is_empty()
        || final_registry
            .runtimes
            .iter()
            .any(|runtime| runtime.model_resource_residency.is_some())
    {
        return Err("M4 close/shutdown did not acknowledge every actual owner".into());
    }
    let expected_starts = if pressure { 0 } else { work.len() };
    let starts = events
        .iter()
        .filter(|event| {
            event["node_id"] == "infer"
                && event["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                && event["payload"]["transition"] == "started"
        })
        .count();
    if starts != expected_starts {
        return Err("M4 ledger runtime Start population disagrees with fixed work".into());
    }
    let oracle = verify_native_records(&result, &events, pressure)?;
    save(&root, "native-oracle.json", &oracle)?;
    Ok(result)
}

fn integer(row: &Value, field: &str) -> ProbeResult<u64> {
    row[field]
        .as_u64()
        .ok_or_else(|| format!("M4 scalar {field} missing or not u64").into())
}
fn owner_shape(owner: &Value) -> ProbeResult<()> {
    for (field, size) in [
        ("loaded_instance", 16),
        ("loaded_profile", 32),
        ("effective_settings", 32),
    ] {
        if owner[field].as_array().is_none_or(|bytes| {
            bytes.len() != size
                || bytes
                    .iter()
                    .any(|byte| byte.as_u64().is_none_or(|byte| byte > 255))
        }) {
            return Err(format!("M4 native owner {field} is missing or malformed").into());
        }
    }
    if integer(owner, "cpu_threads")? != 1 || integer(owner, "generation")? == 0 {
        return Err("M4 actual CPU owner settings/generation refused".into());
    }
    Ok(())
}

fn matching_run<'a>(records: &[&'a Value], run: &str) -> Vec<&'a Value> {
    records
        .iter()
        .copied()
        .filter(|record| record["workflow_run_id"] == run)
        .collect()
}
fn verify_warm_lineage(previous: &Value, current: &Value) -> ProbeResult<()> {
    owner_shape(previous)?;
    owner_shape(current)?;
    for field in [
        "loaded_instance",
        "loaded_profile",
        "effective_settings",
        "cpu_threads",
    ] {
        if previous[field] != current[field] {
            return Err("M4 warm receipt changed native instance/profile/settings".into());
        }
    }
    if integer(previous, "generation")?.checked_add(2) != Some(integer(current, "generation")?) {
        return Err("M4 warm receipt did not carry its actual +2 lineage".into());
    }
    Ok(())
}
fn verify_charges(row: &Value, empty: bool) -> ProbeResult<()> {
    let ids = row["active_lease_ids"]
        .as_array()
        .ok_or("M4 active charge identity absent")?;
    let claims = row["active_claims"]
        .as_array()
        .ok_or("M4 active claims absent")?;
    if row["resident_uncertain"] != false {
        return Err("M4 resident accounting is unknown".into());
    }
    if empty {
        if !ids.is_empty() || !claims.is_empty() || !row["resident_claims"].is_null() {
            return Err("M4 stop ACK did not remove the exact guard/resident charges".into());
        }
    } else {
        if ids.len() != 1 || claims.len() != 1 || claims[0]["reservation_id"] != ids[0] {
            return Err("M4 idle stop/load charge does not have its sole exact guard".into());
        }
        for (set, ram) in [
            (&claims[0]["claims"], LOAD_PEAK_RAM),
            (&row["resident_claims"]["claims"], RESIDENT_RAM),
        ] {
            let set = set
                .as_array()
                .ok_or("M4 explicit charged resource set missing")?;
            if set.len() != 2
                || set
                    .iter()
                    .find(|claim| claim["kind"] == "ram_bytes")
                    .and_then(|claim| claim["bytes"].as_u64())
                    != Some(ram)
                || set
                    .iter()
                    .find(|claim| claim["kind"] == "vram_bytes")
                    .and_then(|claim| claim["bytes"].as_u64())
                    != Some(0)
            {
                return Err("M4 complete additive guard/resident declaration was changed".into());
            }
        }
    }
    Ok(())
}

fn verify_task_cleanup(plan: &Value, native: &Value, ack: &Value, guard: u64) -> ProbeResult<()> {
    let task = integer(native, "reservation_lease_id")?;
    let before = &native["charged_after_native_drain"];
    let after = &ack["charged_after_registry_ack"];
    verify_charges(after, false)?;
    if task == guard || after["active_lease_ids"] != json!([guard]) {
        return Err("M4 task ACK changed the exact idle guard".into());
    }
    let ids = before["active_lease_ids"]
        .as_array()
        .ok_or("M4 drain lease IDs absent")?;
    let claims = before["active_claims"]
        .as_array()
        .ok_or("M4 drain claims absent")?;
    if ids.len() != 2
        || claims.len() != 2
        || !ids.contains(&json!(task))
        || !ids.contains(&json!(guard))
    {
        return Err("M4 drain did not hold its exact task and guard leases".into());
    }
    let task_claims: Vec<_> = claims
        .iter()
        .filter(|claim| claim["reservation_id"] == task)
        .collect();
    if task_claims.len() != 1 {
        return Err("M4 drain task claims do not identify its exact native lease".into());
    }
    let task_claims = task_claims[0]["claims"]
        .as_array()
        .ok_or("M4 drain task resource claims absent")?;
    if task_claims.len() != 2
        || [
            ("ram_bytes", "actual_task_ram_bytes"),
            ("vram_bytes", "actual_task_vram_bytes"),
        ]
        .iter()
        .any(|(kind, field)| {
            task_claims
                .iter()
                .find(|claim| claim["kind"] == *kind)
                .and_then(|claim| claim["bytes"].as_u64())
                != plan[*field].as_u64()
        })
    {
        return Err("M4 drain task claims differ from this preparation's actual maxima".into());
    }
    let mut removed = before.clone();
    removed["active_lease_ids"]
        .as_array_mut()
        .ok_or("M4 drain lease IDs absent")?
        .retain(|id| *id != task);
    removed["active_claims"]
        .as_array_mut()
        .ok_or("M4 drain claims absent")?
        .retain(|claim| claim["reservation_id"] != task);
    if removed != *after {
        return Err("M4 registry ACK changed resources beyond its exact task lease".into());
    }
    Ok(())
}

fn verify_plan_target(plan: &Value, width: usize) -> ProbeResult<()> {
    for field in ["model_ref", "resolved_model_ref"] {
        let logical = plan[field]["model_id"]
            .as_str()
            .ok_or("M4 resolved/logical model absent")?;
        if logical.strip_prefix("pumas://models/").unwrap_or(logical) != model(width)
            || plan[field]["selected_artifact_id"] != "main"
            || !plan[field]["revision"].is_null()
            || !plan[field]["selected_artifact_path_digest"].is_null()
        {
            return Err(
                "M4 resolved/logical model/artifact differs from the fixed fixture target".into(),
            );
        }
    }
    if plan["runtime_id"] != "candle"
        || plan["runtime_variant_id"] != "candle.cpu"
        || plan["device_id"] != "cpu"
        || plan["workflow_id"] != workflow(width)
        || plan["node_id"] != "infer"
        || ["workflow_run_id", "task_id", "preparation_request_id"]
            .iter()
            .any(|field| plan[*field].as_str().is_none_or(str::is_empty))
    {
        return Err("M4 prepared identity differs from the fixed workflow/CPU target".into());
    }
    Ok(())
}

fn verify_native_records(result: &Value, events: &[Value], pressure: bool) -> ProbeResult<Value> {
    let records: Vec<_> = result["native_records"]
        .as_array()
        .ok_or("M4 native records absent")?
        .iter()
        .filter(|row| row["target"] == "embedded::serial_cpu_residency")
        .map(|row| &row["record"])
        .collect();
    let phase = |name: &str| {
        records
            .iter()
            .copied()
            .filter(|record| record["phase"] == name)
            .collect::<Vec<_>>()
    };
    let work = result["fixed_work"]
        .as_array()
        .ok_or("M4 fixed work absent")?;
    if work.is_empty() || (pressure && *work != vec![json!(8)]) {
        return Err("M4 fixed work population refused".into());
    }
    let rows = result["rows"]
        .as_array()
        .ok_or("M4 public response rows absent")?;
    if rows.len() != work.len() {
        return Err("M4 public result population differs from fixed work".into());
    }
    let plans = phase("preparation_plan");
    if plans.len() != work.len() {
        return Err("M4 preparation population differs from fixed public requests".into());
    }
    let position = |record: &Value| {
        records
            .iter()
            .position(|row| std::ptr::eq(*row, record))
            .expect("record belongs to collector")
    };
    for (index, plan) in plans.iter().enumerate() {
        let width = work[index].as_u64().ok_or("M4 fixed model width absent")? as usize;
        if ![8, 12].contains(&width) {
            return Err("M4 fixed model width refused".into());
        }
        verify_plan_target(plan, width)?;
        let ram = integer(plan, "actual_task_ram_bytes")?;
        let vram = integer(plan, "actual_task_vram_bytes")?;
        let floor = integer(plan, "package_conservative_floor_bytes")?;
        if ram < floor
            || vram < floor
            || ram > 1024 * MIB
            || vram > 1024 * MIB
            || integer(plan, "declared_guard_ram_bytes")? != LOAD_PEAK_RAM
            || integer(plan, "declared_guard_vram_bytes")? != 0
            || integer(plan, "declared_resident_ram_bytes")? != RESIDENT_RAM
            || integer(plan, "declared_resident_vram_bytes")? != 0
            || LOAD_PEAK_RAM
                .checked_add(RESIDENT_RAM)
                .and_then(|n| n.checked_add(ram))
                != Some(integer(plan, "required_ram_bytes")?)
            || integer(plan, "required_vram_bytes")? != vram
            || plan["guard_or_task_acquired_by_this_preparation"] != false
        {
            return Err("M4 authoritative actual claims/declarations do not match the prospectively fixed envelope".into());
        }
        let capacity = if pressure {
            PRESSURE_RAM_CAPACITY
        } else {
            RAM_CAPACITY
        };
        let declared = plan["declared_capacity"]["resources"]
            .as_array()
            .ok_or("M4 declared capacity absent")?;
        if declared.len() != 2
            || [("ram_bytes", capacity), ("vram_bytes", VRAM_CAPACITY)]
                .iter()
                .any(|(kind, total)| {
                    declared
                        .iter()
                        .filter(|row| {
                            row["kind"] == *kind
                                && row["total_bytes"] == *total
                                && row["safety_margin_bytes"] == 0
                        })
                        .count()
                        != 1
                })
        {
            return Err("M4 declared capacity differs from the fixed envelope".into());
        }
        if (integer(plan, "required_ram_bytes")? > capacity) != pressure || vram > VRAM_CAPACITY {
            return Err(
                "M4 actual pressure/fitting arithmetic does not match the fixed case".into(),
            );
        }
    }
    let drained = phase("native_drained");
    let acknowledgements = phase("task_registry_ack");
    let cold = phase("cold_loaded");
    let retained = phase("retained");
    let switches = phase("switch_stop_ack");
    let shutdown = phase("shutdown_stop_ack");
    if pressure {
        let refused = phase("preparation_refused");
        let terminal: Vec<_> = events
            .iter()
            .filter(|event| {
                event["workflow_run_id"] == plans[0]["workflow_run_id"]
                    && event["payload"]["payload_type"] == "run_terminal"
            })
            .collect();
        if !drained.is_empty()
            || !acknowledgements.is_empty()
            || !cold.is_empty()
            || !retained.is_empty()
            || !switches.is_empty()
            || !shutdown.is_empty()
            || refused.len() != 1
            || refused[0]["refusal_stage"] != "headroom_admission"
            || refused[0]["preparation_request_id"] != plans[0]["preparation_request_id"]
            || refused[0]["no_task_start_authorized"] != true
            || position(plans[0]) >= position(refused[0])
            || rows[0]["model_id"] != model(8)
            || rows[0]["model_alias"] != "bert-8"
            || !rows[0]["response"].is_null()
            || rows[0]["error"].as_str().is_none_or(str::is_empty)
            || terminal.len() != 1
            || terminal[0]["payload"]["status"] != "failed"
            || events.iter().any(|event| {
                event["node_id"] == "infer"
                    && event["payload"]["payload_type"]
                        == "scheduler_task_attempt_lifecycle_changed"
                    && event["payload"]["transition"] == "started"
            })
        {
            return Err(
                "M4 pressure case produced a native action or lacked its exact admission refusal"
                    .into(),
            );
        }
        return Ok(
            json!({"scope":"main-counts-v1+pressure","actual_pressure_refusal_before_native":true,
            "actual_task_claims":plans,"native_actions":0}),
        );
    }
    let expected_retains = work.windows(2).filter(|pair| pair[0] == pair[1]).count();
    let expected_loads = work.len() - expected_retains;
    if drained.len() != work.len()
        || acknowledgements.len() != work.len()
        || cold.len() != expected_loads
        || retained.len() != expected_retains
        || switches.len() + 1 != expected_loads
        || shutdown.len() != 1
    {
        return Err("M4 raw native load/retain/drain/taskACK/stop populations differ from the hand-declared work".into());
    }
    let mut prior_owner: Option<&Value> = None;
    let mut active_guard = None;
    let mut seen_runs = Vec::new();
    let mut previous_ack_position = None;
    let mut seen_instances = Vec::new();
    let mut forward_ns = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let run = row["response"]["workflow_run_id"]
            .as_str()
            .ok_or("M4 public result run identity missing")?;
        let width = work[index].as_u64().ok_or("M4 fixed model width absent")? as usize;
        if ![8, 12].contains(&width)
            || row["model_id"] != model(width)
            || row["model_alias"] != format!("bert-{width}")
            || run.is_empty()
            || seen_runs.contains(&run)
        {
            return Err(
                "M4 public results do not bijectively match the fixed model/run sequence".into(),
            );
        }
        seen_runs.push(run);
        let planned = matching_run(&plans, run);
        let native = matching_run(&drained, run);
        let ack = matching_run(&acknowledgements, run);
        if planned.len() != 1 || native.len() != 1 || ack.len() != 1 {
            return Err("M4 exact public run lacks its unique native plan/drain/ACK".into());
        }
        let (plan, native, ack) = (planned[0], native[0], ack[0]);
        verify_plan_target(plan, width)?;
        for field in ["workflow_id", "workflow_run_id", "node_id", "task_id"] {
            if plan[field].is_null() || plan[field] != native[field] {
                return Err("M4 preparation and native task identity differ".into());
            }
        }
        for field in [
            "workflow_id",
            "workflow_run_id",
            "node_id",
            "task_id",
            "attempt_id",
            "execution_request_id",
            "candidate_id",
            "reservation_lease_id",
        ] {
            if native[field].is_null() || native[field] != ack[field] {
                return Err(format!("M4 native task ACK identity mismatch: {field}").into());
            }
            if field != "reservation_lease_id" && native[field].as_str().is_none_or(str::is_empty) {
                return Err(format!("M4 native task identity empty or malformed: {field}").into());
            }
        }
        if native["node_id"] != "infer"
            || native["registry_task_release_acknowledged"] != false
            || ack["scheduler_cleanup_acknowledged"] != false
            || ack["registry_task_release_acknowledged"] != true
            || ack["lifecycle_event_id"].as_str().is_none_or(str::is_empty)
        {
            return Err(
                "M4 diagnostic confused native drain, registry task release or scheduler ACK"
                    .into(),
            );
        }
        verify_warm_lineage(&native["previous_owner"], &native["current_owner"])?;
        let same_model = index > 0 && work[index - 1] == work[index];
        let prepared = if same_model { &retained } else { &cold };
        let prepared: Vec<_> = prepared
            .iter()
            .copied()
            .filter(|record| record["preparation_request_id"] == plan["preparation_request_id"])
            .collect();
        if prepared.len() != 1 || prepared[0]["owner"] != native["previous_owner"] {
            return Err(
                "M4 prepared owner does not match the genuine warm receipt ancestry".into(),
            );
        }
        if position(plan) >= position(prepared[0])
            || position(prepared[0]) >= position(native)
            || position(native) >= position(ack)
            || previous_ack_position.is_some_and(|previous| previous >= position(plan))
        {
            return Err(
                "M4 native records do not preserve serial preparation/drain/ACK order".into(),
            );
        }
        previous_ack_position = Some(position(ack));
        if let Some(prior) = prior_owner {
            if same_model {
                if prior != &native["previous_owner"] {
                    return Err(
                        "M4 same-model retain did not preserve exact updated native owner".into(),
                    );
                }
            } else {
                let stop: Vec<_> = switches
                    .iter()
                    .copied()
                    .filter(|record| {
                        record["preparation_request_id"] == plan["preparation_request_id"]
                    })
                    .collect();
                if stop.len() != 1 || &stop[0]["old_owner"] != prior {
                    return Err("M4 model switch stopped a different actual owner".into());
                }
                if position(plan) >= position(stop[0]) || position(stop[0]) >= position(prepared[0])
                {
                    return Err("M4 switch stop did not precede the next cold publication".into());
                }
                if stop[0]["charged_before_stop"]["active_lease_ids"]
                    != json!([active_guard.ok_or("M4 preceding guard absent")?])
                {
                    return Err("M4 switch stop did not release its preceding guard".into());
                }
            }
        }
        if !same_model {
            active_guard = Some(integer(prepared[0], "guard_lease_id")?);
            let instance = native["previous_owner"]["loaded_instance"].clone();
            if seen_instances.contains(&instance) {
                return Err("M4 cold reload reused an old private native instance".into());
            }
            seen_instances.push(instance);
        }
        verify_task_cleanup(
            plan,
            native,
            ack,
            active_guard.ok_or("M4 loaded guard absent")?,
        )?;
        let starts: Vec<_> = events
            .iter()
            .filter(|event| {
                event["workflow_run_id"] == run
                    && event["node_id"] == "infer"
                    && event["payload"]["payload_type"]
                        == "scheduler_task_attempt_lifecycle_changed"
                    && event["payload"]["transition"] == "started"
            })
            .collect();
        if starts.len() != 1
            || starts[0]["payload"]["scheduler_task_id"] != native["task_id"]
            || starts[0]["payload"]["scheduler_attempt_id"] != native["attempt_id"]
            || starts[0]["payload"]["execution_class"] != "runtime"
        {
            return Err("M4 native receipt identity does not match actual ledger Start".into());
        }
        // Started records identify the scheduler attempt before selected lease,
        // runtime and device are emitted. Completed carries that dispatch
        // evidence; do not invent it on the earlier Start observation.
        let completed: Vec<_> = events
            .iter()
            .filter(|event| {
                event["workflow_run_id"] == run
                    && event["node_id"] == "infer"
                    && event["payload"]["payload_type"]
                        == "scheduler_task_attempt_lifecycle_changed"
                    && event["payload"]["transition"] == "completed"
            })
            .collect();
        if completed.len() != 1
            || completed[0]["payload"]["scheduler_task_id"] != native["task_id"]
            || completed[0]["payload"]["scheduler_attempt_id"] != native["attempt_id"]
            || completed[0]["payload"]["execution_class"] != "runtime"
            || completed[0]["payload"]["reservation_id"]
                != format!(
                    "runtime-registry.{}",
                    integer(native, "reservation_lease_id")?
                )
            || completed[0]["payload"]["selected_runtime_id"] != "candle"
            || completed[0]["payload"]["selected_runtime_variant_id"] != "candle.cpu"
            || completed[0]["payload"]["selected_device_id"] != "cpu"
            || integer(starts[0], "event_seq")? >= integer(completed[0], "event_seq")?
        {
            return Err("M4 native receipt identity does not match actual ledger Completed".into());
        }
        forward_ns.push(integer(native, "forward_and_drain_ns")?);
        prior_owner = Some(&native["current_owner"]);
    }
    if prior_owner != Some(&shutdown[0]["old_owner"])
        || previous_ack_position.is_none_or(|ack| ack >= position(shutdown[0]))
    {
        return Err("M4 final shutdown did not stop the actual last warm owner".into());
    }
    if shutdown[0]["charged_before_stop"]["active_lease_ids"]
        != json!([active_guard.ok_or("M4 final guard absent")?])
    {
        return Err("M4 shutdown did not stop its exact final guard".into());
    }
    for load in &cold {
        owner_shape(&load["owner"])?;
        integer(load, "load_ns")?;
        integer(load, "publication_ns")?;
        verify_charges(&load["charged_after_publication"], false)?;
        if load["charged_after_publication"]["active_lease_ids"]
            != json!([integer(load, "guard_lease_id")?])
        {
            return Err("M4 cold publication did not charge its exact actual guard".into());
        }
    }
    for stop in switches.iter().chain(shutdown.iter()) {
        integer(stop, "stop_ns")?;
        integer(stop, "acknowledgement_ns")?;
        verify_charges(&stop["charged_before_stop"], false)?;
        verify_charges(&stop["charged_after_ack"], true)?;
    }
    Ok(
        json!({"scope":"main-counts-v1+pressure","raw_loads":cold.len(),"raw_retains":retained.len(),
        "raw_native_forwards_and_drains":drained.len(),"raw_task_registry_acks":acknowledgements.len(),
        "raw_switch_stops":switches.len(),"raw_final_stops":shutdown.len(),"forward_and_drain_ns":forward_ns,
        "native_instances":seen_instances,"actual_task_claims":plans,"latency_or_speedup_claim":false}),
    )
}

pub(super) async fn qualify(root: &Path) -> ProbeResult<()> {
    let fixture = fixture_contract(root)?;
    provenance(root)?;
    log::set_logger(&NATIVE_LOG).map_err(|_| "M4 logger already installed")?;
    log::set_max_level(log::LevelFilter::Info);
    save(
        root,
        "prospective-protocol.json",
        &json!({"protocol":PROTOCOL,"fixed_work":FIXED_WORK,
        "retained":{"owners":1,"public_calls":5,"expected_loads":3,"expected_retains":2,"expected_stops":3},
        "comparison":{"owners":5,"public_calls_per_owner":1,"acknowledged_shutdown_before_next_owner":true,
            "expected_loads":5,"expected_stops":5,"comparison_scope":"native counts/vectors/ownership only; startup/session/history/latency not comparable"},
        "pressure":{"owners":1,"public_calls":1,"width":8,"ram_capacity_bytes":PRESSURE_RAM_CAPACITY,"expected_loads":0,"expected_starts":0},
        "resource_declarations":{"ram_capacity_bytes":RAM_CAPACITY,"vram_capacity_bytes":VRAM_CAPACITY,
            "resident_ram_bytes":RESIDENT_RAM,"resident_vram_bytes":0,"guard_load_peak_ram_bytes":LOAD_PEAK_RAM,
            "guard_load_peak_vram_bytes":0,"cpu_threads":1,"estimates_are_operator_declarations":true},
        "native_postload_cancellation_case":"pending separately reviewed sealed-facade/native protocol; not silently substituted",
        "public_running_postdrain_cancellation_case":"pending reviewed observation barrier; no racing cancellation claim",
        "controlled_fault_cases":"supporting named native/registry tests; not main-cohort native failure",
        "retries":0,"model_acquisition_bytes":0,"performance_claim":false,"default_activation":false}),
    )?;
    let mut outcomes = Vec::new();
    let execution: ProbeResult<()> = async {
        let retained = owner_protocol(
            root,
            "retained",
            &FIXED_WORK,
            &fixture["models"],
            RAM_CAPACITY,
            false,
        )
        .await?;
        let diagnostics = &retained["diagnostics_after_shutdown"];
        if diagnostics["loads"] != 3 || diagnostics["retains"] != 2 || diagnostics["stops"] != 3 {
            return Err(
                "M4 retained protocol did not produce exactly 3 loads, 2 retains, 3 stops".into(),
            );
        }
        outcomes.push(retained);
        for (index, width) in FIXED_WORK.into_iter().enumerate() {
            let comparison = owner_protocol(
                root,
                &format!("comparison-{index}"),
                &[width],
                &fixture["models"],
                RAM_CAPACITY,
                false,
            )
            .await?;
            if comparison["diagnostics_after_shutdown"]["loads"] != 1
                || comparison["diagnostics_after_shutdown"]["stops"] != 1
            {
                return Err("M4 comparison owner did not produce exactly one load and stop".into());
            }
            outcomes.push(comparison);
        }
        outcomes.push(
            owner_protocol(
                root,
                "pressure",
                &[8],
                &fixture["models"],
                PRESSURE_RAM_CAPACITY,
                true,
            )
            .await?,
        );
        Ok(())
    }
    .await;
    let log = NATIVE_LOG.0.lock().expect("isolated M4 evidence log");
    save(
        root,
        "native-and-service-log.json",
        &json!({"rows":log.rows,"overflow":log.overflow,"bytes":log.bytes}),
    )?;
    let report = json!({"protocol":PROTOCOL,"scope":"main-counts-v1+pressure","main_and_pressure_protocols_ok":execution.is_ok(),
        "execution_error":execution.as_ref().err().map(ToString::to_string),"owners":outcomes,"log_overflow":log.overflow,
        "native_cohort_retries":0,"native_forward_and_drain_timing":"actual native receipt scalar at admitted submission-to-join/drain seam",
        "cancellation_qualification":"pending separately admitted native and public Running observation cases",
        "M4_C4_acceptance":"partial; fixed counts do not replace pending cancellation/fault qualification"});
    save(root, "m4-result.json", &report)?;
    if log.overflow {
        return Err("M4 evidence log bound exceeded".into());
    }
    execution?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(generation: u64) -> Value {
        json!({"loaded_instance":vec![7_u8;16],"loaded_profile":vec![8_u8;32],
            "effective_settings":vec![9_u8;32],"generation":generation,"cpu_threads":1})
    }
    fn charges() -> Value {
        json!({"active_lease_ids":[5],"active_claims":[{"reservation_id":5,
            "claims":[{"kind":"ram_bytes","bytes":LOAD_PEAK_RAM},{"kind":"vram_bytes","bytes":0}]}],
            "resident_claims":{"claims":[{"kind":"ram_bytes","bytes":RESIDENT_RAM},{"kind":"vram_bytes","bytes":0}]},
            "resident_uncertain":false})
    }
    // Controlled saved-JSON fixture only. Durations and identities here are
    // hand-declared oracle inputs, never native evidence or scheduler inputs.
    fn recorded_singleton() -> (Value, Vec<Value>) {
        let plan = json!({"phase":"preparation_plan","preparation_request_id":"prep-1",
            "workflow_id":workflow(8),"workflow_run_id":"run-1","node_id":"infer","task_id":"runtime:infer",
            "model_ref":{"model_id":MODEL_ID,"selected_artifact_id":"main"},
            "resolved_model_ref":{"model_id":MODEL_ID,"selected_artifact_id":"main"},
            "runtime_id":"candle","runtime_variant_id":"candle.cpu","device_id":"cpu",
            "actual_task_ram_bytes":512*MIB,"actual_task_vram_bytes":512*MIB,"package_conservative_floor_bytes":512*MIB,
            "declared_guard_ram_bytes":LOAD_PEAK_RAM,"declared_guard_vram_bytes":0,
            "declared_resident_ram_bytes":RESIDENT_RAM,"declared_resident_vram_bytes":0,
            "declared_capacity":{"resources":[{"kind":"ram_bytes","total_bytes":RAM_CAPACITY,"safety_margin_bytes":0},
                {"kind":"vram_bytes","total_bytes":VRAM_CAPACITY,"safety_margin_bytes":0}]},
            "guard_or_task_acquired_by_this_preparation":false,"required_ram_bytes":592*MIB,"required_vram_bytes":512*MIB});
        let mut drained = json!({"phase":"native_drained","workflow_id":workflow(8),"workflow_run_id":"run-1","node_id":"infer",
            "task_id":"runtime:infer","attempt_id":"attempt-1","execution_request_id":"request-1","candidate_id":"candidate-1",
            "reservation_lease_id":42,"previous_owner":owner(2),"current_owner":owner(4),"forward_and_drain_ns":13,
            "registry_task_release_acknowledged":false});
        let mut drain_charges = charges();
        drain_charges["active_lease_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!(42));
        drain_charges["active_claims"].as_array_mut().unwrap().push(json!({"reservation_id":42,
            "claims":[{"kind":"ram_bytes","bytes":512*MIB},{"kind":"vram_bytes","bytes":512*MIB}]}));
        drained["charged_after_native_drain"] = drain_charges;
        let mut ack = drained.clone();
        ack["phase"] = json!("task_registry_ack");
        ack["lifecycle_event_id"] = json!("cleanup-1");
        ack["scheduler_cleanup_acknowledged"] = json!(false);
        ack["registry_task_release_acknowledged"] = json!(true);
        ack["charged_after_registry_ack"] = charges();
        let records = vec![
            plan,
            json!({"phase":"cold_loaded","preparation_request_id":"prep-1","owner":owner(2),"guard_lease_id":5,
                "charged_after_publication":charges(),"load_ns":11,"publication_ns":7}),
            drained,
            ack,
            json!({"phase":"shutdown_stop_ack","old_owner":owner(4),"charged_before_stop":charges(),
                "charged_after_ack":{"active_lease_ids":[],"active_claims":[],"resident_claims":null,"resident_uncertain":false},
                "stop_ns":5,"acknowledgement_ns":3}),
        ];
        let result = json!({"fixed_work":[8],"rows":[{"model_id":MODEL_ID,"model_alias":"bert-8","response":{"workflow_run_id":"run-1"}}],
            "native_records":records.into_iter().map(|record|json!({"target":"embedded::serial_cpu_residency","record":record})).collect::<Vec<_>>()});
        let events = vec![
            json!({"event_seq":5,"workflow_run_id":"run-1","node_id":"infer","payload":{
            "payload_type":"scheduler_task_attempt_lifecycle_changed","transition":"started",
            "execution_class":"runtime","scheduler_task_id":"runtime:infer","scheduler_attempt_id":"attempt-1",
            "selected_runtime_id":null,"selected_device_id":null}}),
            json!({"event_seq":6,"workflow_run_id":"run-1","node_id":"infer","payload":{
            "payload_type":"scheduler_task_attempt_lifecycle_changed","transition":"completed","execution_class":"runtime",
            "scheduler_task_id":"runtime:infer","scheduler_attempt_id":"attempt-1","reservation_id":"runtime-registry.42",
            "selected_runtime_id":"candle","selected_runtime_variant_id":"candle.cpu","selected_device_id":"cpu"}}),
        ];
        (result, events)
    }

    #[test]
    fn raw_oracle_matches_full_task_identity_instead_of_equating_node_and_task_names() {
        let (result, mut events) = recorded_singleton();
        assert!(verify_native_records(&result, &events, false).is_ok());
        events[0]["payload"]["scheduler_task_id"] = json!("infer");
        assert_eq!(
            verify_native_records(&result, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 native receipt identity does not match actual ledger Start"
        );
    }

    #[test]
    fn raw_oracle_accepts_start_without_lease_and_joins_the_completed_dispatch() {
        let (result, events) = recorded_singleton();
        assert!(events[0]["payload"].get("reservation_id").is_none());
        assert!(events[0]["payload"]["selected_runtime_id"].is_null());
        assert_eq!(
            events[1]["payload"]["reservation_id"],
            "runtime-registry.42"
        );
        assert!(verify_native_records(&result, &events, false).is_ok());
    }

    #[test]
    fn raw_oracle_refuses_completed_foreign_lease_attempt_or_target() {
        let (result, base) = recorded_singleton();
        assert!(verify_native_records(&result, &base, false).is_ok());
        for (field, value) in [
            ("reservation_id", "runtime-registry.99"),
            ("scheduler_attempt_id", "foreign-attempt"),
            ("selected_runtime_id", "foreign-runtime"),
            ("selected_runtime_variant_id", "foreign-variant"),
            ("selected_device_id", "gpu"),
        ] {
            let mut events = base.clone();
            events[1]["payload"][field] = json!(value);
            assert_eq!(
                verify_native_records(&result, &events, false)
                    .unwrap_err()
                    .to_string(),
                "M4 native receipt identity does not match actual ledger Completed",
                "accepted foreign {field}"
            );
        }
        let mut reversed = base;
        reversed[1]["event_seq"] = json!(4);
        assert!(verify_native_records(&result, &reversed, false).is_err());
    }

    #[test]
    fn projection_evidence_requires_each_relevant_cursor_not_an_unrelated_global_cursor() {
        // Controlled projection DTOs only; no native load or actual run occurs.
        let advanced:Vec<_>=[("run_detail",12),("node_status",8),("io_artifact",12)].into_iter().map(|(name,seq)|json!({
            "projection_kind":name,"projection_state":{"projection_name":name,"projection_version":1,
                "last_applied_event_seq":seq,"status":"current","rebuilt_at_ms":null,"updated_at_ms":0,
                "last_error":null,"last_error_at_ms":null,"last_failed_event_seq":null}})).collect();
        let mut refresh: WorkflowDiagnosticsProjectionRefreshResponse =
            serde_json::from_value(json!({"advanced":advanced,"failed":[],"invalidations":[]}))
                .unwrap();
        let boundary = json!({"visible_event_count":12,"visible_latest_event_seq":12,
            "required_cursors":{"run_detail":12,"node_status":8,"io_artifact":12},"single_page_limit":500});
        assert!(verify_projection_catchup(&refresh, &boundary).is_ok());
        refresh.advanced[2].projection_state.last_applied_event_seq = 0;
        assert_eq!(
            verify_projection_catchup(&refresh, &boundary)
                .unwrap_err()
                .to_string(),
            "M4 projection io_artifact did not reach its exact visible relevant-event boundary"
        );
        refresh.advanced[2].projection_state.last_applied_event_seq = 12;
        refresh.advanced[1].projection_state.last_applied_event_seq = 12;
        assert_eq!(
            verify_projection_catchup(&refresh, &boundary)
                .unwrap_err()
                .to_string(),
            "M4 projection node_status did not reach its exact visible relevant-event boundary"
        );
    }
    #[test]
    fn raw_oracle_refuses_missing_native_phase_instead_of_substituting_zero() {
        let (mut result, events) = recorded_singleton();
        result["native_records"][2]["record"]["forward_and_drain_ns"] = Value::Null;
        assert_eq!(
            verify_native_records(&result, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 scalar forward_and_drain_ns missing or not u64"
        );
    }
    #[test]
    fn raw_oracle_refuses_foreign_warm_ancestry_and_unknown_charged_kind() {
        let previous = owner(2);
        let mut current = owner(4);
        assert!(verify_warm_lineage(&previous, &current).is_ok());
        current["loaded_profile"] = json!(vec![10_u8; 32]);
        assert_eq!(
            verify_warm_lineage(&previous, &current)
                .unwrap_err()
                .to_string(),
            "M4 warm receipt changed native instance/profile/settings"
        );
        let mut charged = charges();
        assert!(verify_charges(&charged, false).is_ok());
        charged["resident_claims"]["claims"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_eq!(
            verify_charges(&charged, false).unwrap_err().to_string(),
            "M4 complete additive guard/resident declaration was changed"
        );
    }

    #[test]
    fn raw_oracle_requires_this_tasks_actual_maxima_and_only_its_exact_release() {
        let (base, events) = recorded_singleton();
        assert!(verify_native_records(&base, &events, false).is_ok());
        let mut wrong_task = base.clone();
        wrong_task["native_records"][2]["record"]["charged_after_native_drain"]["active_claims"]
            [1]["claims"][0]["bytes"] = json!(512 * MIB + 1);
        assert_eq!(
            verify_native_records(&wrong_task, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 drain task claims differ from this preparation's actual maxima"
        );
        let mut foreign_lease = base.clone();
        foreign_lease["native_records"][3]["record"]["charged_after_registry_ack"]
            ["active_lease_ids"] = json!([42]);
        foreign_lease["native_records"][3]["record"]["charged_after_registry_ack"]
            ["active_claims"][0]["reservation_id"] = json!(42);
        assert_eq!(
            verify_native_records(&foreign_lease, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 task ACK changed the exact idle guard"
        );
        let mut erased_guard = base.clone();
        erased_guard["native_records"][3]["record"]["charged_after_registry_ack"]
            ["active_claims"] = json!([]);
        assert!(verify_native_records(&erased_guard, &events, false).is_err());
        let mut changed_resident = base;
        changed_resident["native_records"][2]["record"]["charged_after_native_drain"]
            ["resident_claims"]["claims"][0]["bytes"] = json!(RESIDENT_RAM + 1);
        assert_eq!(
            verify_native_records(&changed_resident, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 registry ACK changed resources beyond its exact task lease"
        );
    }

    #[test]
    fn raw_oracle_binds_resolved_artifact_full_identity_and_fixed_public_sequence() {
        let (base, events) = recorded_singleton();
        assert!(verify_native_records(&base, &events, false).is_ok());
        let mut foreign_artifact = base.clone();
        foreign_artifact["native_records"][0]["record"]["resolved_model_ref"]
            ["selected_artifact_id"] = json!("foreign");
        assert_eq!(
            verify_native_records(&foreign_artifact, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 resolved/logical model/artifact differs from the fixed fixture target"
        );
        let mut wrong_model = base.clone();
        wrong_model["rows"][0]["model_id"] = json!(model(12));
        assert!(verify_native_records(&wrong_model, &events, false).is_err());
        let mut foreign_task = base.clone();
        foreign_task["native_records"][0]["record"]["task_id"] = json!("foreign-task");
        assert_eq!(
            verify_native_records(&foreign_task, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 preparation and native task identity differ"
        );
        let mut unacknowledged = base.clone();
        unacknowledged["native_records"][3]["record"]["registry_task_release_acknowledged"] =
            json!(false);
        assert!(verify_native_records(&unacknowledged, &events, false).is_err());
        let mut extra_row = base;
        let duplicate = extra_row["rows"][0].clone();
        extra_row["rows"].as_array_mut().unwrap().push(duplicate);
        assert!(verify_native_records(&extra_row, &events, false).is_err());
    }

    #[test]
    fn raw_oracle_requires_matching_headroom_refusal_before_any_native_action() {
        let (mut pressure, _) = recorded_singleton();
        pressure["rows"][0]["response"] = Value::Null;
        pressure["rows"][0]["error"] = json!("controlled admission refusal");
        let mut plan = pressure["native_records"][0].clone();
        plan["record"]["declared_capacity"]["resources"][0]["total_bytes"] =
            json!(PRESSURE_RAM_CAPACITY);
        pressure["native_records"] = json!([plan,{"target":"embedded::serial_cpu_residency","record":{
            "phase":"preparation_refused","preparation_request_id":"prep-1","refusal_stage":"headroom_admission","no_task_start_authorized":true}}]);
        let terminal = vec![
            json!({"workflow_run_id":"run-1","payload":{"payload_type":"run_terminal","status":"failed"}}),
        ];
        assert!(verify_native_records(&pressure, &terminal, true).is_ok());
        assert!(verify_native_records(&pressure, &[], true).is_err());
        let mut wrong_stage = pressure.clone();
        wrong_stage["native_records"][1]["record"]["refusal_stage"] = json!("load");
        assert!(verify_native_records(&wrong_stage, &terminal, true).is_err());
        let mut foreign_refusal = pressure.clone();
        foreign_refusal["native_records"][1]["record"]["preparation_request_id"] =
            json!("foreign-preparation");
        assert!(verify_native_records(&foreign_refusal, &terminal, true).is_err());
        pressure["native_records"][0]["record"]["declared_capacity"]["resources"][0]
            ["total_bytes"] = json!(RAM_CAPACITY);
        assert_eq!(
            verify_native_records(&pressure, &terminal, true)
                .unwrap_err()
                .to_string(),
            "M4 declared capacity differs from the fixed envelope"
        );
    }

    #[test]
    fn raw_oracle_requires_serial_preparation_drain_ack_and_shutdown_order() {
        let (base, events) = recorded_singleton();
        assert!(verify_native_records(&base, &events, false).is_ok());
        for (first, second) in [(0, 1), (1, 2), (2, 3), (3, 4)] {
            let mut reordered = base.clone();
            reordered["native_records"]
                .as_array_mut()
                .unwrap()
                .swap(first, second);
            assert!(
                verify_native_records(&reordered, &events, false).is_err(),
                "accepted swapped records {first}/{second}"
            );
        }
    }

    #[test]
    fn raw_oracle_requires_bijective_public_runs_across_a_retained_pair() {
        let (mut pair, mut events) = recorded_singleton();
        let original = pair["native_records"].as_array().unwrap().clone();
        let mut plan = original[0].clone();
        plan["record"]["preparation_request_id"] = json!("prep-2");
        plan["record"]["workflow_run_id"] = json!("run-2");
        let retained = json!({"target":"embedded::serial_cpu_residency","record":{
            "phase":"retained","preparation_request_id":"prep-2","owner":owner(4)}});
        let mut drained = original[2].clone();
        let mut ack = original[3].clone();
        for row in [&mut drained, &mut ack] {
            for (field, value) in [
                ("workflow_run_id", "run-2"),
                ("attempt_id", "attempt-2"),
                ("execution_request_id", "request-2"),
                ("candidate_id", "candidate-2"),
            ] {
                row["record"][field] = json!(value);
            }
            row["record"]["reservation_lease_id"] = json!(43);
            row["record"]["previous_owner"] = owner(4);
            row["record"]["current_owner"] = owner(6);
        }
        drained["record"]["charged_after_native_drain"]["active_lease_ids"][1] = json!(43);
        drained["record"]["charged_after_native_drain"]["active_claims"][1]["reservation_id"] =
            json!(43);
        ack["record"]["lifecycle_event_id"] = json!("cleanup-2");
        let mut shutdown = original[4].clone();
        shutdown["record"]["old_owner"] = owner(6);
        pair["native_records"] = json!([
            original[0],
            original[1],
            original[2],
            original[3],
            plan,
            retained,
            drained,
            ack,
            shutdown
        ]);
        pair["fixed_work"] = json!([8, 8]);
        let mut second_result = pair["rows"][0].clone();
        second_result["response"]["workflow_run_id"] = json!("run-2");
        pair["rows"].as_array_mut().unwrap().push(second_result);
        let mut second_start = events[0].clone();
        second_start["workflow_run_id"] = json!("run-2");
        second_start["payload"]["scheduler_attempt_id"] = json!("attempt-2");
        second_start["event_seq"] = json!(15);
        let mut second_completed = events[1].clone();
        second_completed["workflow_run_id"] = json!("run-2");
        second_completed["payload"]["scheduler_attempt_id"] = json!("attempt-2");
        second_completed["payload"]["reservation_id"] = json!("runtime-registry.43");
        second_completed["event_seq"] = json!(16);
        events.extend([second_start, second_completed]);
        assert!(verify_native_records(&pair, &events, false).is_ok());
        let mut duplicate = pair.clone();
        duplicate["rows"][1]["response"]["workflow_run_id"] = json!("run-1");
        assert_eq!(
            verify_native_records(&duplicate, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 public results do not bijectively match the fixed model/run sequence"
        );
        pair["native_records"].as_array_mut().unwrap().swap(3, 4);
        assert_eq!(
            verify_native_records(&pair, &events, false)
                .unwrap_err()
                .to_string(),
            "M4 native records do not preserve serial preparation/drain/ACK order"
        );
    }
}

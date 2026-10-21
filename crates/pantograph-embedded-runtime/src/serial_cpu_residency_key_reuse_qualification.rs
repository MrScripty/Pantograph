//! One explicitly held native comparison; ordinary tests exercise only its oracle.
//! Public facade methods perform all actions. Private test-support reads observe
//! scalar work counts only and never supply readiness, native output or custody.

use serde_json::{json, Value};

const WORKFLOW: &str = "cpu-m5-key-reuse-pair";
const NODES: [&str; 2] = ["infer_a", "infer_b"];
const OUTPUTS: [&str; 2] = ["vectors_a", "vectors_b"];
const PHASES: [&str; 5] = ["probe", "capture", "fill", "lookup", "key_calculation"];

fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.into())
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    value[field]
        .as_array()
        .ok_or_else(|| format!("missing {field}"))
}

fn rows<'a>(arm: &'a Value, phase: &str) -> Result<Vec<&'a Value>, String> {
    Ok(array(arm, "logs")?
        .iter()
        .filter(|row| row["record"]["phase"] == phase)
        .collect())
}

fn vector_matches(actual: &Value, expected: &Value) -> bool {
    let (Some(actual), Some(expected)) = (actual.as_array(), expected.as_array()) else {
        return false;
    };
    actual.len() == 8
        && expected.len() == 8
        && actual.iter().zip(expected).all(|(actual, expected)| {
            match (actual.as_f64(), expected.as_f64()) {
                (Some(actual), Some(expected)) => {
                    actual.is_finite() && expected.is_finite() && (actual - expected).abs() <= 1e-5
                }
                _ => false,
            }
        })
}

fn verify_owner(owner: &Value) -> Result<(), String> {
    for (field, length) in [
        ("loaded_instance", 16),
        ("loaded_profile", 32),
        ("effective_settings", 32),
    ] {
        require(
            owner[field].as_array().is_some_and(|bytes| {
                bytes.len() == length
                    && bytes
                        .iter()
                        .all(|byte| byte.as_u64().is_some_and(|byte| byte <= 255))
            }),
            "native owner identity/settings missing",
        )?;
    }
    require(
        owner["cpu_threads"] == 1
            && owner["generation"]
                .as_u64()
                .is_some_and(|generation| generation > 0),
        "native owner generation/threads invalid",
    )
}

/// Hand-declared identities/counts are independent of production selection.
fn verify_arm(arm: &Value, enabled: bool, golden: &Value) -> Result<(), String> {
    require(
        arm["enabled"] == enabled && arm["configuration"]["key_reuse"] == enabled,
        "wrong constructor configuration",
    )?;
    require(
        arm["configuration"]["fair"] == true && arm["configuration"]["residency"] == true,
        "required per-member modes missing",
    )?;
    require(
        arm["error"].is_null()
            && arm["collector_error"].is_null()
            && arm["diagnostic_io_error"].is_null()
            && arm["event_error"].is_null(),
        "execution or collector failed",
    )?;
    require(
        arm["session_close"]["ok"] == true
            && arm["graph_closes_ok"] == true
            && arm["shutdown"]["ok"] == true
            && arm["shutdown"]["timed_out"] == false,
        "teardown incomplete or uncertain",
    )?;
    require(
        array(&arm["final_registry"], "reservations")?.is_empty()
            && array(&arm["final_registry"], "runtimes")?
                .iter()
                .all(|runtime| runtime["model_resource_residency"].is_null()),
        "final resources remain",
    )?;
    let diagnostics = &arm["diagnostics"];
    require(
        diagnostics["loads"] == 1
            && diagnostics["retains"] == 1
            && diagnostics["forward_drains"] == 2
            && diagnostics["stops"] == 1
            && diagnostics["has_idle_owner"] == false,
        "native populations differ",
    )?;
    require(
        arm["work"]["error"].is_null() && arm["work"]["retained"] == 0,
        "cache observation unavailable or entries retained",
    )?;
    require(
        arm["work"]["encodings"] == if enabled { 4 } else { 6 }
            && arm["work"]["hits"] == if enabled { 4 } else { 0 },
        "fixed mechanism counts differ",
    )?;
    let run = arm["response"]["workflow_run_id"]
        .as_str()
        .filter(|run| !run.is_empty())
        .ok_or("missing run")?;
    require(
        array(arm, "resume_run_ids")?.iter().all(|id| id == run),
        "readiness resume changed run",
    )?;
    let returned = array(&arm["response"], "outputs")?;
    require(returned.len() == 2, "public output population differs")?;
    for (index, node) in OUTPUTS.iter().enumerate() {
        let values: Vec<_> = returned
            .iter()
            .filter(|output| output["node_id"] == *node && output["port_id"] == "vector")
            .collect();
        require(
            values.len() == 1
                && vector_matches(&values[0]["value"], &golden["single_vectors"][index]),
            "public golden vector differs",
        )?;
        let retained: Vec<_> = array(arm, "retained")?
            .iter()
            .filter(|row| row["node_id"] == *node)
            .collect();
        require(
            !retained.is_empty()
                && retained.iter().all(|row| {
                    row["workflow_run_id"] == run
                        && row["value"] == values[0]["value"]
                        && row["read"]["complete"] == true
                        && row["read"]["artifact_id"] == row["artifact_id"]
                        && row["read"]["byte_length"] == row["body_bytes"]
                        && row["artifact_id"].as_str().is_some_and(|id| !id.is_empty())
                }),
            "retained artifact missing or differs",
        )?;
    }

    let grants = rows(arm, "service_opportunity")?;
    let drained = rows(arm, "native_drained")?;
    let acks = rows(arm, "task_registry_ack")?;
    require(
        grants.len() == 2 && drained.len() == 2 && acks.len() == 2,
        "grant/drain/registry-ACK populations differ",
    )?;
    let events = array(arm, "events")?;
    let lifecycle = |transition: &str| {
        events
            .iter()
            .filter(|event| {
                event["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                    && event["payload"]["execution_class"] == "runtime"
                    && event["payload"]["transition"] == transition
            })
            .collect::<Vec<_>>()
    };
    let starts = lifecycle("started");
    let completed = lifecycle("completed");
    require(
        starts.len() == 2 && completed.len() == 2,
        "Start/Completed populations differ",
    )?;
    require(
        grants[0]["record"]["ticket"].as_u64().is_some()
            && grants[1]["record"]["ticket"].as_u64().is_some()
            && grants[0]["record"]["ticket"] != grants[1]["record"]["ticket"],
        "service ticket reused",
    )?;
    for field in [
        "task_id",
        "attempt_id",
        "execution_request_id",
        "candidate_id",
        "reservation_lease_id",
    ] {
        require(
            !drained[0]["record"][field].is_null()
                && drained[0]["record"][field] != drained[1]["record"][field],
            "native identity reused",
        )?;
    }
    let mut task_ids = Vec::new();
    for (index, row) in drained.iter().enumerate() {
        let native = &row["record"];
        let (previous, current) = (&native["previous_owner"], &native["current_owner"]);
        verify_owner(previous)?;
        verify_owner(current)?;
        for field in [
            "loaded_instance",
            "loaded_profile",
            "effective_settings",
            "cpu_threads",
        ] {
            require(
                previous[field] == current[field],
                "native warm identity/settings changed",
            )?;
        }
        require(
            previous["generation"]
                .as_u64()
                .and_then(|generation| generation.checked_add(2))
                == current["generation"].as_u64(),
            "native warm generation did not advance by two",
        )?;
        if index == 1 {
            require(
                drained[0]["record"]["current_owner"] == *previous,
                "native owner continuity broken",
            )?;
        }
        require(
            native["workflow_id"] == WORKFLOW
                && native["workflow_run_id"] == run
                && native["node_id"] == NODES[index],
            "native node/run order differs",
        )?;
        let task = native["task_id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("native task absent")?;
        task_ids.push(json!(task));
        let lease = native["reservation_lease_id"]
            .as_u64()
            .ok_or("native lease absent")?;
        let ack_rows: Vec<_> = acks
            .iter()
            .filter(|ack| ack["record"]["task_id"] == task)
            .collect();
        let start_rows: Vec<_> = starts
            .iter()
            .filter(|event| event["payload"]["scheduler_task_id"] == task)
            .collect();
        let complete_rows: Vec<_> = completed
            .iter()
            .filter(|event| event["payload"]["scheduler_task_id"] == task)
            .collect();
        require(
            ack_rows.len() == 1 && start_rows.len() == 1 && complete_rows.len() == 1,
            "identity joins are not bijective",
        )?;
        let ack = &ack_rows[0]["record"];
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
            require(
                !native[field].is_null() && ack[field] == native[field],
                "registry ACK identity differs",
            )?;
        }
        require(
            ack["registry_task_release_acknowledged"] == true
                && ack["scheduler_cleanup_acknowledged"] == false
                && ack["lifecycle_event_id"]
                    .as_str()
                    .is_some_and(|id| !id.is_empty()),
            "registry ACK missing or mislabeled",
        )?;
        let start = start_rows[0];
        let complete = complete_rows[0];
        require(
            start["workflow_run_id"] == run
                && complete["workflow_run_id"] == run
                && start["node_id"] == native["node_id"]
                && complete["node_id"] == native["node_id"]
                && start["payload"]["scheduler_attempt_id"] == native["attempt_id"]
                && complete["payload"]["scheduler_attempt_id"] == native["attempt_id"]
                && complete["payload"]["reservation_id"] == format!("runtime-registry.{lease}")
                && complete["payload"]["selected_runtime_id"] == "candle"
                && complete["payload"]["selected_runtime_variant_id"] == "candle.cpu"
                && complete["payload"]["selected_device_id"] == "cpu"
                && start["event_seq"]
                    .as_u64()
                    .zip(complete["event_seq"].as_u64())
                    .is_some_and(|(a, b)| a < b),
            "ledger/native dispatch join differs",
        )?;
        require(
            grants[index]["record"]["workflow_run_id"] == run
                && grants[index]["record"]["task_id"] == task,
            "service grant belongs to another member",
        )?;
        require(
            grants[index]["sequence"]
                .as_u64()
                .zip(row["sequence"].as_u64())
                .is_some_and(|(a, b)| a < b)
                && row["sequence"]
                    .as_u64()
                    .zip(ack_rows[0]["sequence"].as_u64())
                    .is_some_and(|(a, b)| a < b),
            "grant/native/ACK order differs",
        )?;
        if index == 0 {
            require(
                ack_rows[0]["sequence"]
                    .as_u64()
                    .zip(grants[1]["sequence"].as_u64())
                    .is_some_and(|(a, b)| a < b),
                "next grant preceded member registry ACK",
            )?;
        }
    }
    let proposals = rows(arm, "proposal")?;
    require(
        proposals.len() == 1
            && proposals[0]["record"]["workflow_run_id"] == run
            && proposals[0]["record"]["task_ids"] == json!(task_ids)
            && proposals[0]["sequence"]
                .as_u64()
                .zip(drained[0]["sequence"].as_u64())
                .is_some_and(|(a, b)| a < b),
        "genuine two-Ready pair missing",
    )?;
    let metrics = rows(arm, "grouping_observations")?;
    let forecasts = rows(arm, "warm_service_forecasts")?;
    if enabled {
        require(
            !metrics.is_empty()
                && forecasts.len() == 1
                && array(&forecasts[0]["record"], "forecasts")?.len() == 2
                && forecasts[0]["record"]["coverage"] == "captured_compatible_prefix",
            "two-member captured prefix missing",
        )?;
        let snapshot = &metrics.last().ok_or("phase snapshot absent")?["record"];
        require(
            snapshot["scope"]
                .as_str()
                .is_some_and(|scope| !scope.is_empty())
                && snapshot["scope"] == forecasts[0]["record"]["scope"],
            "phase scope differs",
        )?;
        let phases = array(snapshot, "phases")?;
        require(phases.len() == 5, "phase population differs")?;
        for name in PHASES {
            let phase: Vec<_> = phases
                .iter()
                .filter(|phase| phase["phase"] == name)
                .collect();
            require(
                phase.len() == 1
                    && phase[0]["aggregate"]["state"] == "available"
                    && phase[0]["aggregate"]["calls"]
                        .as_u64()
                        .is_some_and(|calls| calls > 0)
                    && phase[0]["aggregate"]["elapsed_ns"].as_u64().is_some(),
                "phase unavailable or unobserved",
            )?;
        }
    } else {
        require(
            metrics.is_empty() && forecasts.is_empty(),
            "disabled baseline acquired grouping measurements",
        )?;
    }
    Ok(())
}

fn verify_comparison(arms: &[Value], golden: &Value) -> Result<(), String> {
    require(arms.len() == 2, "two arms required")?;
    verify_arm(&arms[0], false, golden)?;
    verify_arm(&arms[1], true, golden)?;
    require(
        arms[0]["response"]["workflow_run_id"] != arms[1]["response"]["workflow_run_id"],
        "fresh arms reused run identity",
    )?;
    for output in OUTPUTS {
        let value = |arm: &Value| {
            array(&arm["response"], "outputs")
                .ok()?
                .iter()
                .find(|row| row["node_id"] == output)
                .map(|row| row["value"].clone())
        };
        require(
            value(&arms[0]) == value(&arms[1]),
            "baseline/enabled output differs",
        )?;
    }
    Ok(())
}

#[cfg(feature = "host-dependency-inventory")]
mod held {
    use super::super::{config, seed, MODEL};
    use super::*;
    use crate::EmbeddedSerialCpuResidency;
    use pantograph_diagnostics_ledger::{DiagnosticsLedgerRepository, SqliteDiagnosticsLedger};
    use pantograph_workflow_service::graph::{
        authored_snapshot_from_descriptor, NodeRegistry, WorkflowGraph,
    };
    use pantograph_workflow_service::*;
    use std::{
        error::Error,
        fs::{File, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
        sync::Mutex,
        time::{Duration, Instant},
    };

    type Outcome<T> = Result<T, Box<dyn Error>>;
    const PROTOCOL: &str = "m5-native-two-ready-key-reuse-v1";
    const TEXTS: [&str; 2] = ["hello world", "hello"];
    const RUN_BUDGET: Duration = Duration::from_secs(60);
    const SHUTDOWN_BUDGET: Duration = Duration::from_secs(30);
    struct LogBuffer {
        rows: Vec<Value>,
        bytes: usize,
        error: Option<&'static str>,
        file: Option<File>,
        arm: &'static str,
    }
    struct Collector(Mutex<LogBuffer>);
    static LOG: Collector = Collector(Mutex::new(LogBuffer {
        rows: Vec::new(),
        bytes: 0,
        error: None,
        file: None,
        arm: "preflight",
    }));
    impl log::Log for Collector {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            matches!(
                metadata.target(),
                "workflow::serial_ready"
                    | "workflow::serial_observations"
                    | "embedded::serial_cpu_residency"
            )
        }
        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let Ok(mut buffer) = self.0.lock() else {
                return;
            };
            let text = record.args().to_string();
            match serde_json::from_str::<Value>(&text) {
                Ok(value) => {
                    let sequence = buffer.rows.len();
                    let row = json!({"sequence":sequence,"arm":buffer.arm,"target":record.target(),"record":value});
                    let Ok(mut bytes) = serde_json::to_vec(&row) else {
                        buffer.error = Some("structured row serialization failed");
                        return;
                    };
                    bytes.push(b'\n');
                    if buffer.rows.len() >= 2048
                        || bytes.len() > 64 * 1024
                        || buffer.bytes + bytes.len() > 4 * 1024 * 1024
                    {
                        buffer.error = Some("bounded log capacity exceeded");
                        return;
                    }
                    buffer.bytes += bytes.len();
                    buffer.rows.push(row);
                    if let Some(file) = buffer.file.as_mut() {
                        if file
                            .write_all(&bytes)
                            .and_then(|_| file.sync_all())
                            .is_err()
                        {
                            buffer.error = Some("native row persistence failed");
                            buffer.file = None;
                        }
                    }
                }
                Err(_) => buffer.error = Some("structured evidence parse failed"),
            }
        }
        fn flush(&self) {}
    }
    fn logs(from: usize) -> (Vec<Value>, Option<&'static str>) {
        LOG.0
            .lock()
            .map_or((Vec::new(), Some("collector lock unavailable")), |buffer| {
                (buffer.rows[from..].to_vec(), buffer.error)
            })
    }
    fn save(root: &Path, name: &str, value: &Value) -> Outcome<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
    fn progress(file: &mut File, arm: &str, stage: &str, detail: Value) -> Outcome<()> {
        serde_json::to_writer(
            &mut *file,
            &json!({"arm":arm,"stage":stage,"detail":detail}),
        )?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }

    async fn author_pair(
        facade: &EmbeddedSerialCpuResidency,
        edits: &mut Vec<String>,
    ) -> Outcome<()> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for suffix in ["a", "b"] {
            nodes.extend([
                json!({"id":format!("deps_{suffix}"),"node_type":"dependency-environment","position":{"x":0,"y":0},"data":{"mode":"manual"}}),
                json!({"id":format!("prompt_{suffix}"),"node_type":"text-input","position":{"x":0,"y":100},"data":{}}),
                json!({"id":format!("infer_{suffix}"),"node_type":"llm-inference","position":{"x":300,"y":100},"data":{
                    "task_kind":"embedding","runtime":"candle","device":"cpu",
                    "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
                    "pumas_model_ref":{"model_id":MODEL,"selected_artifact_id":"main"}}}),
                json!({"id":format!("vectors_{suffix}"),"node_type":"vector-output","position":{"x":600,"y":100},"data":{}}),
            ]);
            for (prefix, source, source_handle, target, target_handle) in [
                (
                    "text",
                    format!("prompt_{suffix}"),
                    "text",
                    format!("infer_{suffix}"),
                    "text",
                ),
                (
                    "vector",
                    format!("infer_{suffix}"),
                    "embedding",
                    format!("vectors_{suffix}"),
                    "vector",
                ),
                (
                    "dep",
                    format!("deps_{suffix}"),
                    "dependency_environment_sidecar",
                    format!("infer_{suffix}"),
                    "dependency_environment_sidecar",
                ),
            ] {
                edges.push(json!({"id":format!("{prefix}_{suffix}"),"source":source,"source_handle":source_handle,"target":target,"target_handle":target_handle}));
            }
        }
        let mut graph: WorkflowGraph =
            serde_json::from_value(json!({"nodes":nodes,"edges":edges}))?;
        let registry = NodeRegistry::new();
        for node in &mut graph.nodes {
            if node.node_type != "llm-inference" {
                node.data["definition"] = serde_json::to_value(
                    registry
                        .get_definition(&node.node_type)
                        .ok_or("missing built-in definition")?,
                )?;
            }
        }
        let preview = facade
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
                workflow_id: None,
            })
            .await?;
        edits.push(preview.session_id.clone());
        let validation = facade
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: preview.session_id.clone(),
                    graph_revision: preview.graph_revision.parse()?,
                },
            )
            .await?;
        for id in NODES {
            let descriptor = &validation
                .node_projections
                .iter()
                .find(|node| node.node_id.as_str() == id)
                .ok_or("missing native descriptor")?
                .descriptor;
            graph
                .nodes
                .iter_mut()
                .find(|node| node.id == id)
                .ok_or("missing infer node")?
                .data["inference_interface_snapshot"] =
                serde_json::to_value(authored_snapshot_from_descriptor(descriptor)?)?;
        }
        facade
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: preview.session_id,
            })
            .await?;
        edits.pop();
        facade.workflow_graph_save(WorkflowGraphSaveRequest {
            name: WORKFLOW.into(),
            graph: graph.clone(),
        })?;
        let edit = facade
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph,
                workflow_id: Some(WORKFLOW.into()),
            })
            .await?;
        edits.push(edit.session_id.clone());
        let validation = facade
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: edit.session_id.clone(),
                    graph_revision: edit.graph_revision.parse()?,
                },
            )
            .await?;
        for id in ["deps_a", "deps_b"] {
            let resolved = facade.workflow_graph_resolve_dependency_environment_action_intent(pantograph_inference_interface_contracts::DependencyEnvironmentActionIntent {
                contract_version: 1, graph_session_id: edit.session_id.parse()?, graph_revision: edit.graph_revision.parse()?,
                validation_session_id: validation.summary.validation_session_id.clone(), target_node_id: id.parse()?,
                action: pantograph_inference_interface_contracts::DependencyEnvironmentAction::Resolve,
            }).await?;
            if resolved.status != pantograph_inference_interface_contracts::DependencyEnvironmentActionIntentStatus::RequestReady { return Err("dependency action not ready".into()); }
        }
        facade
            .publish_graph_session_executable_validation_snapshot(
                WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                    workflow_id: WORKFLOW.into(),
                    workflow_semantic_version: "1.0.0".into(),
                    graph_session_id: edit.session_id,
                    validation_snapshot_id: None,
                    validation_session_id: Some(
                        validation
                            .summary
                            .validation_session_id
                            .ok_or("validation session absent")?,
                    ),
                },
            )
            .await?;
        Ok(())
    }

    async fn run_pair(
        facade: &EmbeddedSerialCpuResidency,
        session: &str,
        arm: &mut Value,
    ) -> Outcome<WorkflowRunResponse> {
        let operation = async {
            let mut result = facade
                .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                    session_id: session.into(),
                    workflow_semantic_version: "1.0.0".into(),
                    inputs: ["a", "b"]
                        .into_iter()
                        .zip(TEXTS)
                        .map(|(suffix, text)| WorkflowPortBinding {
                            node_id: format!("prompt_{suffix}"),
                            port_id: "text".into(),
                            value: json!(text),
                        })
                        .collect(),
                    output_targets: Some(
                        OUTPUTS
                            .into_iter()
                            .map(|node| WorkflowOutputTarget {
                                node_id: node.into(),
                                port_id: "vector".into(),
                            })
                            .collect(),
                    ),
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(0),
                })
                .await;
            let mut resumes = Vec::new();
            let mut polls = 0;
            while matches!(
                result,
                Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
            ) {
                arm["readiness_pending"] = json!(result.as_ref().err().map(ToString::to_string));
                polls += 1;
                if polls > 1200 {
                    return Err("readiness poll bound exceeded".into());
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
                if let Some(request) = facade
                    .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?
                    .into_iter()
                    .find(|request| request.session_id == session)
                {
                    if resumes.len() >= 32 {
                        return Err("readiness resume bound exceeded".into());
                    }
                    resumes.push(request.workflow_run_id.clone());
                    arm["resume_run_ids"] = json!(resumes);
                    result = facade
                        .resume_workflow_execution_session_runtime_dependency_readiness(request)
                        .await;
                }
            }
            Ok::<_, Box<dyn Error>>(result?)
        };
        tokio::time::timeout(RUN_BUDGET, operation).await?
    }

    fn events(root: &Path) -> Outcome<Vec<Value>> {
        let ledger =
            SqliteDiagnosticsLedger::open(root.join("app-data/serial-cpu-diagnostics.db"))?;
        let events = ledger.diagnostic_events_after(0, 500)?;
        if events.len() >= 500 {
            return Err("ledger page bound exceeded".into());
        }
        events
            .into_iter()
            .map(|event| {
                Ok(
                    json!({"event_seq":event.event_seq,"workflow_run_id":event.workflow_run_id,
            "node_id":event.node_id,"payload":serde_json::from_str::<Value>(&event.payload_json)?}),
                )
            })
            .collect()
    }

    async fn retained(
        facade: &EmbeddedSerialCpuResidency,
        session: &str,
        response: &WorkflowRunResponse,
        arm: &mut Value,
    ) -> Outcome<()> {
        let refresh = facade.workflow_diagnostics_projection_refresh(
            WorkflowDiagnosticsProjectionRefreshRequest {
                projections: vec![
                    WorkflowDiagnosticsProjectionKind::RunDetail,
                    WorkflowDiagnosticsProjectionKind::NodeStatus,
                    WorkflowDiagnosticsProjectionKind::IoArtifact,
                ],
                workflow_run_id: Some(response.workflow_run_id.clone()),
                workflow_id: Some(WORKFLOW.into()),
                reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                batch_size: 500,
            },
        )?;
        arm["projection_refresh"] = json!(refresh);
        if !refresh.failed.is_empty() || refresh.advanced.len() != 3 {
            return Err("projection refresh incomplete".into());
        }
        let inspection =
            facade.workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
                workflow_run_id: response.workflow_run_id.clone(),
                artifact_limit: Some(128),
                projection_batch_size: Some(500),
            })?;
        let mut values = Vec::new();
        for artifact in &inspection.io_artifacts {
            let Some(node) = artifact
                .producer_node_id
                .as_deref()
                .filter(|node| OUTPUTS.contains(node))
            else {
                continue;
            };
            if artifact.producer_port_id.as_deref() != Some("vector") {
                continue;
            }
            let body = facade.workflow_read_artifact_body(ArtifactReadRequest {
                artifact_id: artifact.payload_artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })?;
            values.push(json!({"node_id":node,"workflow_run_id":artifact.workflow_run_id,"artifact_id":artifact.payload_artifact_id,
                "read":body.response,"body_bytes":body.body.len(),"value":serde_json::from_slice::<Value>(&body.body)?,"body_blake3":blake3::hash(&body.body).to_hex().to_string()}));
        }
        arm["inspection"] = json!(inspection);
        arm["retained"] = json!(values);
        arm["task_states"] = json!(facade.workflow_get_scheduler_task_state_read_models(
            pantograph_workflow_service::workflow::WorkflowSchedulerTaskStateReadModelQueryRequest { session_id: session.into(), workflow_run_id: response.workflow_run_id.clone() }).await?);
        Ok(())
    }

    async fn run_arm(root: &Path, enabled: bool, progress_file: &mut File) -> Outcome<Value> {
        std::fs::create_dir(root)?;
        let name = if enabled { "enabled" } else { "baseline" };
        LOG.0.lock().map_err(|_| "collector lock unavailable")?.arm = name;
        progress(progress_file, name, "seed", json!({}))?;
        seed(root);
        let log_start = logs(0).0.len();
        let constructed = if enabled {
            EmbeddedSerialCpuResidency::new_with_bounded_key_reuse(config(root)).await
        } else {
            EmbeddedSerialCpuResidency::new(config(root)).await
        };
        let facade = match constructed {
            Ok(facade) => facade,
            Err(error) => {
                let row = json!({"enabled":enabled,"error":error.to_string(),"stage":"construct","construction_failed":true});
                save(root, "arm.json", &row)?;
                return Err(error.into());
            }
        };
        let snapshot = facade.scheduler_configuration();
        let mut arm = json!({"enabled":enabled,"configuration":{"key_reuse":snapshot.serial_key_reuse(),"fair":snapshot.serial_fair_opportunities(),"residency":snapshot.serial_residency()},
            "error":null,"collector_error":null,"resume_run_ids":[],"retained":[],"scheduler_cleanup_match":"source_assisted_adapter_validation; registry ACK is distinct"});
        let mut edit_sessions = Vec::new();
        let mut session = None;
        let body: Outcome<()> = async {
            progress(progress_file, name, "author", json!({}))?;
            author_pair(&facade, &mut edit_sessions).await?;
            let client = facade.register_attribution_client(ClientRegistrationRequest {
                display_name: Some(PROTOCOL.into()),
                metadata_json: None,
            })?;
            let opened = facade.open_client_session(ClientSessionOpenRequest {
                credential: client.credential_proof_request(),
                takeover: false,
                reason: Some(PROTOCOL.into()),
            })?;
            let created = facade
                .create_attributed_workflow_execution_session(
                    WorkflowExecutionSessionAttributedCreateRequest {
                        workflow_id: WORKFLOW.into(),
                        usage_profile: None,
                        keep_alive: false,
                        attribution: WorkflowExecutionSessionAttributionRequest {
                            credential: client.credential_proof_request(),
                            client_session_id: opened.session.client_session_id.as_str().into(),
                            bucket_selection:
                                pantograph_runtime_attribution::BucketSelection::Default,
                        },
                    },
                )
                .await?;
            session = Some(created.session_id.clone());
            arm["session"] = json!(created);
            progress(progress_file, name, "run", json!({"session_id":session}))?;
            let start = Instant::now();
            let response = run_pair(
                &facade,
                session.as_deref().ok_or("session missing")?,
                &mut arm,
            )
            .await?;
            arm["submit_to_result_elapsed_ns"] = json!(start.elapsed().as_nanos());
            arm["response"] = json!(response);
            progress(progress_file, name, "run_returned", arm["response"].clone())?;
            arm["work"] = match facade
                .runtime
                .workflow_service
                .try_serial_grouping_work_for_test()
            {
                Ok((encodings, hits, retained)) => {
                    json!({"encodings":encodings,"hits":hits,"retained":retained,"error":null})
                }
                Err(error) => json!({"error":error.to_string()}),
            };
            retained(
                &facade,
                session.as_deref().ok_or("session missing")?,
                &response,
                &mut arm,
            )
            .await?;
            Ok(())
        }
        .await;
        if let Err(error) = &body {
            arm["error"] = json!(error.to_string());
        }
        // No fallible diagnostic write may bypass close and the existing owner.
        let mut io_error = progress(
            progress_file,
            name,
            "teardown",
            json!({"body_error":arm["error"]}),
        )
        .err()
        .map(|error| error.to_string());
        let close_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        if let Some(session_id) = session {
            let close = tokio::time::timeout_at(
                close_deadline,
                facade.close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                    session_id,
                }),
            )
            .await;
            arm["session_close"] = match close {
                Ok(result) => {
                    json!({"ok":result.is_ok(),"error":result.err().map(|error|error.to_string())})
                }
                Err(error) => json!({"ok":false,"timed_out":true,"error":error.to_string()}),
            };
        }
        let mut graph_closes_ok = true;
        let mut graph_closes = Vec::new();
        for session_id in edit_sessions {
            let close = tokio::time::timeout_at(
                close_deadline,
                facade.workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: session_id.clone(),
                }),
            )
            .await;
            let row = match close {
                Ok(result) => {
                    json!({"session_id":session_id,"ok":result.is_ok(),"error":result.err().map(|error|error.to_string())})
                }
                Err(error) => {
                    json!({"session_id":session_id,"ok":false,"timed_out":true,"error":error.to_string()})
                }
            };
            graph_closes_ok &= row["ok"] == true;
            graph_closes.push(row);
        }
        arm["graph_closes_ok"] = json!(graph_closes_ok);
        arm["graph_closes"] = json!(graph_closes);
        arm["before_shutdown_registry"] = json!(facade.registry_snapshot());
        let shutdown = tokio::time::timeout(SHUTDOWN_BUDGET, facade.shutdown()).await;
        arm["shutdown"] = match shutdown {
            Ok(result) => {
                json!({"ok":result.is_ok(),"timed_out":false,"error":result.err().map(|error|error.to_string())})
            }
            Err(error) => {
                json!({"ok":false,"timed_out":true,"error":error.to_string(),"custody":"supervisor retained; timeout is not release evidence"})
            }
        };
        arm["final_registry"] = json!(facade.registry_snapshot());
        arm["diagnostics"] = json!(facade.residency_diagnostics());
        match events(root) {
            Ok(events) => arm["events"] = json!(events),
            Err(error) => {
                arm["event_error"] = json!(error.to_string());
            }
        }
        let (records, error) = logs(log_start);
        arm["logs"] = json!(records);
        arm["collector_error"] = json!(error);
        if let Err(error) = progress(
            progress_file,
            name,
            "teardown_returned",
            arm["shutdown"].clone(),
        ) {
            io_error.get_or_insert_with(|| error.to_string());
        }
        arm["diagnostic_io_error"] = json!(io_error);
        // Save raw outcomes before the caller runs any acceptance oracle.
        save(root, "arm.json", &arm)?;
        drop(facade);
        if io_error.is_some() {
            return Err("diagnostic output failed; retained prefix preserved".into());
        }
        Ok(arm)
    }

    #[tokio::test]
    #[ignore = "one held native protocol; requires separate exact-source admission and evidence root"]
    async fn fixed_two_ready_native_key_reuse_comparison() -> Outcome<()> {
        let root = PathBuf::from(
            std::env::var_os("PANTOGRAPH_M5_KEY_REUSE_EVIDENCE_ROOT")
                .ok_or("mandatory evidence root missing")?,
        );
        if !root.is_absolute() {
            return Err("absolute evidence root required".into());
        }
        if !root.is_dir() {
            return Err("outer source/binary freeze must create the evidence root first".into());
        }
        let metadata = PathBuf::from(
            std::env::var_os("PUMAS_REGISTRY_DB_PATH")
                .ok_or("static Pumas metadata path missing")?,
        );
        if metadata != root.join("metadata.db") || metadata.exists() {
            return Err(
                "fresh root/metadata.db must be selected before this process starts".into(),
            );
        }
        save(
            &root,
            "protocol.json",
            &json!({"protocol":PROTOCOL,"arms":["baseline","enabled"],"texts":TEXTS,
            "public_runs_per_arm":1,"native_forwards_per_arm":2,"cpu_threads":1,"declared_ram_bytes":2_u64*1024*1024*1024,
            "run_readiness_seconds":60,"teardown_close_seconds_total":5,"shutdown_seconds":30,"retries":0,"speedup_claim":false,"memory_claim":false,
            "input_source":"committed BERT8 golden texts[0..2]","scheduler_cleanup_match":"source-assisted, registry ACK distinct",
            "pumas_metadata":"one process-wide fresh metadata.db shared by arms; private library/project/RuntimeRegistry roots differ"}),
        )?;
        let mut progress_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("progress.jsonl"))?;
        LOG.0.lock().map_err(|_| "collector lock unavailable")?.file = Some(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join("native-rows.jsonl"))?,
        );
        log::set_logger(&LOG)
            .map_err(|_| "isolated held process requires exclusive logger ownership")?;
        log::set_max_level(log::LevelFilter::Info);
        let golden: Value = serde_json::from_slice(include_bytes!(
            "../../inference/tests/fixtures/candle_bert/bert-8/golden.json"
        ))?;
        require(
            golden["texts"][0] == TEXTS[0] && golden["texts"][1] == TEXTS[1],
            "fixed golden inputs changed",
        )?;
        let mut arms = Vec::new();
        let result: Outcome<()> = async {
            for enabled in [false, true] {
                let arm = run_arm(
                    &root.join(if enabled { "enabled" } else { "baseline" }),
                    enabled,
                    &mut progress_file,
                )
                .await?;
                arms.push(arm);
                verify_arm(arms.last().ok_or("arm absent")?, enabled, &golden)?;
            }
            verify_comparison(&arms, &golden)?;
            Ok(())
        }
        .await;
        save(
            &root,
            "comparison.json",
            &json!({"ok":result.is_ok(),"error":result.as_ref().err().map(ToString::to_string),"arms":arms,
            "speedup_claim":false,"complete_memory_claim":false,"native_batching_claim":false}),
        )?;
        result
    }
}

#[cfg(test)]
mod oracle_tests {
    use super::*;

    fn positive(enabled: bool) -> (Value, Value) {
        let run = if enabled {
            "run-enabled"
        } else {
            "run-baseline"
        };
        let golden =
            json!({"single_vectors":[[1.,0.,0.,0.,0.,0.,0.,0.],[0.,1.,0.,0.,0.,0.,0.,0.]]});
        let mut logs = Vec::new();
        let mut events = Vec::new();
        for (index, node) in NODES.iter().enumerate() {
            let task = format!("task-{index}");
            let attempt = format!("attempt-{index}");
            let lease = 10 + index;
            logs.push(json!({"sequence":index*10,"record":{"phase":"service_opportunity","workflow_run_id":run,"task_id":task,"ticket":index+1}}));
            let native = json!({"phase":"native_drained","workflow_id":WORKFLOW,"workflow_run_id":run,"node_id":node,"task_id":task,
                "attempt_id":attempt,"execution_request_id":format!("request-{index}"),"candidate_id":format!("candidate-{index}"),"reservation_lease_id":lease,
                "previous_owner":{"loaded_instance":vec![7;16],"loaded_profile":vec![8;32],"effective_settings":vec![9;32],"generation":2+index*2,"cpu_threads":1},
                "current_owner":{"loaded_instance":vec![7;16],"loaded_profile":vec![8;32],"effective_settings":vec![9;32],"generation":4+index*2,"cpu_threads":1}});
            logs.push(json!({"sequence":index*10+4,"record":native}));
            let mut ack = native;
            ack["phase"] = json!("task_registry_ack");
            ack["registry_task_release_acknowledged"] = json!(true);
            ack["scheduler_cleanup_acknowledged"] = json!(false);
            ack["lifecycle_event_id"] = json!(format!("lifecycle-{index}"));
            logs.push(json!({"sequence":index*10+5,"record":ack}));
            for (offset, transition) in ["started", "completed"].iter().enumerate() {
                events.push(json!({"event_seq":index*2+offset+1,"workflow_run_id":run,"node_id":node,"payload":{
                    "payload_type":"scheduler_task_attempt_lifecycle_changed","execution_class":"runtime","transition":transition,
                    "scheduler_task_id":task,"scheduler_attempt_id":attempt,"reservation_id":format!("runtime-registry.{lease}"),
                    "selected_runtime_id":"candle","selected_runtime_variant_id":"candle.cpu","selected_device_id":"cpu"}}));
            }
        }
        logs.push(json!({"sequence":2,"record":{"phase":"proposal","workflow_run_id":run,"task_ids":["task-0","task-1"]}}));
        if enabled {
            logs.push(json!({"sequence":1,"record":{"phase":"warm_service_forecasts","coverage":"captured_compatible_prefix","scope":"opaque","forecasts":[{},{}]}}));
            logs.push(json!({"sequence":30,"record":{"phase":"grouping_observations","scope":"opaque","phases":PHASES.iter().map(|phase|json!({"phase":phase,"aggregate":{"state":"available","calls":1,"elapsed_ns":0}})).collect::<Vec<_>>()}}));
        }
        let arm = json!({"enabled":enabled,"configuration":{"key_reuse":enabled,"fair":true,"residency":true},"error":null,"collector_error":null,
            "session_close":{"ok":true},"graph_closes_ok":true,"shutdown":{"ok":true,"timed_out":false},"final_registry":{"reservations":[],"runtimes":[{}]},
            "diagnostics":{"loads":1,"retains":1,"forward_drains":2,"stops":1,"has_idle_owner":false},
            "work":{"encodings":if enabled {4}else{6},"hits":if enabled {4}else{0},"retained":0,"error":null},
            "response":{"workflow_run_id":run,"outputs":OUTPUTS.iter().enumerate().map(|(index,node)|json!({"node_id":node,"port_id":"vector","value":golden["single_vectors"][index]})).collect::<Vec<_>>()},
            "retained":OUTPUTS.iter().enumerate().map(|(index,node)|json!({"node_id":node,"workflow_run_id":run,"artifact_id":format!("artifact-{index}"),
                "read":{"artifact_id":format!("artifact-{index}"),"complete":true,"byte_length":32},"body_bytes":32,"value":golden["single_vectors"][index]})).collect::<Vec<_>>(),
            "resume_run_ids":[run],"logs":logs,"events":events});
        (arm, golden)
    }

    #[test]
    fn hand_declared_pair_oracle_accepts_both_arms_and_zero_resolution_elapsed() {
        let (baseline, golden) = positive(false);
        let (enabled, _) = positive(true);
        assert_eq!(verify_comparison(&[baseline, enabled], &golden), Ok(()));
    }

    #[test]
    fn mechanism_oracle_refuses_missing_hit_and_unavailable_or_nonempty_cache() {
        let (arm, golden) = positive(true);
        for work in [
            json!({"encodings":4,"hits":0,"retained":0,"error":null}),
            json!({"error":"grouping cache unavailable"}),
            json!({"encodings":4,"hits":4,"retained":1,"error":null}),
        ] {
            let mut changed = arm.clone();
            changed["work"] = work;
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
    }

    #[test]
    fn identity_oracle_refuses_foreign_attempt_lease_missing_start_and_reused_grant() {
        let (arm, golden) = positive(true);
        for field in ["attempt_id", "reservation_lease_id"] {
            let mut changed = arm.clone();
            changed["logs"][2]["record"][field] = json!("foreign");
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
        let mut changed = arm.clone();
        changed["events"].as_array_mut().unwrap().remove(0);
        assert!(verify_arm(&changed, true, &golden).is_err());
        let mut changed = arm;
        changed["logs"][3]["record"]["ticket"] = json!(1);
        assert!(verify_arm(&changed, true, &golden).is_err());
    }

    #[test]
    fn phase_oracle_refuses_missing_unavailable_or_unobserved_spans_and_baseline_clocks() {
        let (arm, golden) = positive(true);
        for aggregate in [
            json!({"state":"unavailable","reason":"lock_unavailable"}),
            json!({"state":"available","calls":0,"elapsed_ns":1}),
        ] {
            let mut changed = arm.clone();
            changed["logs"][8]["record"]["phases"][1]["aggregate"] = aggregate;
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
        let mut changed = arm.clone();
        changed["logs"][8]["record"]["phases"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(verify_arm(&changed, true, &golden).is_err());
        let (mut baseline, _) = positive(false);
        baseline["logs"]
            .as_array_mut()
            .unwrap()
            .push(arm["logs"][8].clone());
        assert!(verify_arm(&baseline, false, &golden).is_err());
    }

    #[test]
    fn cleanup_and_output_oracle_refuses_uncertain_shutdown_false_ack_and_bad_golden() {
        let (arm, golden) = positive(true);
        let mut changed = arm.clone();
        changed["shutdown"]["timed_out"] = json!(true);
        assert!(verify_arm(&changed, true, &golden).is_err());
        let mut changed = arm.clone();
        changed["logs"][2]["record"]["registry_task_release_acknowledged"] = json!(false);
        assert!(verify_arm(&changed, true, &golden).is_err());
        let mut changed = arm.clone();
        changed["response"]["outputs"][0]["value"][0] = json!(0.1);
        assert!(verify_arm(&changed, true, &golden).is_err());
        let mut changed = arm;
        changed["retained"][0]["workflow_run_id"] = json!("foreign");
        assert!(verify_arm(&changed, true, &golden).is_err());
    }

    #[test]
    fn native_owner_oracle_refuses_missing_settings_wrong_threads_and_broken_lineage() {
        let (arm, golden) = positive(true);
        for owner in [Value::Null, json!({"cpu_threads":2})] {
            let mut changed = arm.clone();
            changed["logs"][1]["record"]["previous_owner"] = owner;
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
        for (field, value) in [
            ("generation", json!(99)),
            ("cpu_threads", json!(2)),
            ("effective_settings", json!(vec![10; 32])),
        ] {
            let mut changed = arm.clone();
            changed["logs"][1]["record"]["current_owner"][field] = value;
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
        let mut changed = arm;
        changed["logs"][4]["record"]["previous_owner"]["generation"] = json!(8);
        changed["logs"][4]["record"]["current_owner"]["generation"] = json!(10);
        assert!(verify_arm(&changed, true, &golden).is_err());
    }

    #[test]
    fn retained_read_oracle_refuses_truncated_foreign_or_inconsistent_byte_reads() {
        let (arm, golden) = positive(true);
        for (field, value) in [
            ("complete", json!(false)),
            ("artifact_id", json!("foreign")),
            ("byte_length", json!(1)),
        ] {
            let mut changed = arm.clone();
            changed["retained"][0]["read"][field] = value;
            assert!(verify_arm(&changed, true, &golden).is_err());
        }
    }
}

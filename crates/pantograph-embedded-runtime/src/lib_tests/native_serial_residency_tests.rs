//! Prospective native cancellation qualification. Gates hold genuine native
//! capabilities/receipts; they never substitute readiness, output or cleanup.
use super::*;
use crate::serial_cpu_residency::tests::{author, config, seed};
use crate::EmbeddedSerialCpuResidency;
use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, RunListProjectionStatus, SqliteDiagnosticsLedger,
};
use pantograph_runtime_registry::RuntimeRegistrySnapshot;
use pantograph_scheduler::SchedulerTaskStateKind;
use pantograph_workflow_service::workflow::{
    WorkflowSchedulerTaskStateReadModelQueryRequest,
    WorkflowSchedulerTaskStateReadModelQueryResponse,
};
use pantograph_workflow_service::{
    ClientRegistrationRequest, ClientSessionOpenRequest, WorkflowDiagnosticsProjectionKind,
    WorkflowDiagnosticsProjectionRefreshReason, WorkflowDiagnosticsProjectionRefreshRequest,
    WorkflowExecutionSessionActiveTaskCancelRequest,
    WorkflowExecutionSessionActiveTaskCancelStatus,
    WorkflowExecutionSessionAttributedCreateRequest, WorkflowExecutionSessionAttributionRequest,
    WorkflowRunInspectionQueryRequest, WorkflowSchedulerSnapshotRequest,
};
use serde_json::{json, Value};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

type CaseResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
const WATCHDOG: Duration = Duration::from_secs(40);
const CASE_OBSERVER: Duration = Duration::from_secs(120);

fn require(condition: bool, message: &'static str) -> CaseResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

// Releasing a test gate during an early failure lets the existing execution
// owner finish/refuse its work. It does not cancel or release native custody.
struct ReleaseGate(Arc<Notify>);
impl Drop for ReleaseGate {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

fn registry_facts(snapshot: &RuntimeRegistrySnapshot) -> Value {
    json!({
        "leases":snapshot.reservations.iter().map(|lease|json!({
            "reservation_id":lease.reservation_id,"runtime_id":lease.runtime_id,
            "workflow_id":lease.workflow_id,"owner_id":lease.reservation_owner_id,
            "pinned":lease.pin_runtime,"retention":lease.retention_hint
        })).collect::<Vec<_>>(),
        "runtimes":snapshot.runtimes.iter().map(|runtime|json!({
            "runtime_id":runtime.runtime_id,"status":runtime.status,
            "instance_id":runtime.runtime_instance_id,"last_error":runtime.last_error,
            "active_lease_ids":runtime.active_reservation_ids,
            "active_claims":runtime.active_reservation_claims,
            "resident_uncertain":runtime.resident_resources_uncertain,
            "resident_instance_id":runtime.model_resource_residency.as_ref().and_then(|resident|resident.runtime_instance_id.as_ref()),
            "resident_claims":runtime.model_resource_residency.as_ref().and_then(|resident|resident.requirements.as_ref())
        })).collect::<Vec<_>>()
    })
}

fn same_charges(left: &RuntimeRegistrySnapshot, right: &RuntimeRegistrySnapshot) -> bool {
    left.reservations == right.reservations
        && left.runtimes.len() == right.runtimes.len()
        && left.runtimes.iter().zip(&right.runtimes).all(|(a, b)| {
            a.runtime_id == b.runtime_id
                && a.active_reservation_ids == b.active_reservation_ids
                && a.active_reservation_claims == b.active_reservation_claims
                && a.model_resource_residency == b.model_resource_residency
        })
}

fn save_evidence(case: &str, evidence: &Value) -> CaseResult<()> {
    let Some(root) = std::env::var_os("PANTOGRAPH_M4_CANCELLATION_EVIDENCE_ROOT") else {
        return Ok(());
    };
    std::fs::create_dir_all(&root)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(std::path::PathBuf::from(root).join(format!("{case}.json")))?;
    serde_json::to_writer_pretty(&mut file, evidence)?;
    file.sync_all()?;
    Ok(())
}

async fn session(facade: &EmbeddedSerialCpuResidency, evidence: &mut Value) -> CaseResult<String> {
    label_await(evidence, "session:author_workflow");
    author(facade).await.map_err(|error| error.to_string())?;
    let registered = facade.register_attribution_client(ClientRegistrationRequest {
        display_name: Some("M4 native cancellation qualification".into()),
        metadata_json: None,
    })?;
    let client = facade.open_client_session(ClientSessionOpenRequest {
        credential: registered.credential_proof_request(),
        takeover: false,
        reason: None,
    })?;
    label_await(evidence, "session:create_attributed_session");
    Ok(facade
        .create_attributed_workflow_execution_session(
            WorkflowExecutionSessionAttributedCreateRequest {
                workflow_id: "serial-residency-test".into(),
                usage_profile: None,
                keep_alive: false,
                attribution: WorkflowExecutionSessionAttributionRequest {
                    credential: registered.credential_proof_request(),
                    client_session_id: client.session.client_session_id.as_str().into(),
                    bucket_selection: pantograph_runtime_attribution::BucketSelection::Default,
                },
            },
        )
        .await?
        .session_id)
}

async fn run(
    facade: Arc<EmbeddedSerialCpuResidency>,
    session_id: String,
    observation: Arc<Mutex<Value>>,
) -> CaseResult<WorkflowRunResponse> {
    observation.lock().unwrap()["await"] = json!("physical_run");
    let mut result = facade
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
            workflow_semantic_version: "1.0.0".into(),
            inputs: vec![WorkflowPortBinding {
                node_id: "prompt".into(),
                port_id: "text".into(),
                value: json!("hello world"),
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
    retain_first_run_error(&observation, &result);
    while matches!(
        result,
        Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
    ) {
        observation.lock().unwrap()["await"] = json!("readiness_poll_interval");
        tokio::time::sleep(Duration::from_millis(20)).await;
        if let Some(request) = facade
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()?
            .into_iter()
            .find(|request| request.session_id == session_id)
        {
            observation.lock().unwrap()["resume_request"] = json!(request);
            observation.lock().unwrap()["await"] = json!("physical_readiness_resume");
            result = facade
                .resume_workflow_execution_session_runtime_dependency_readiness(request)
                .await;
            retain_first_run_error(&observation, &result);
        }
    }
    Ok(result?)
}

async fn task_states(
    facade: &EmbeddedSerialCpuResidency,
    session_id: &str,
    evidence: &mut Value,
) -> CaseResult<WorkflowSchedulerTaskStateReadModelQueryResponse> {
    label_await(evidence, "task_states:scheduler_snapshot");
    let snapshot = facade
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: session_id.into(),
        })
        .await?;
    let workflow_run_id = snapshot
        .workflow_run_id
        .ok_or("missing active workflow run")?;
    label_await(evidence, "task_states:read_models");
    Ok(facade
        .workflow_get_scheduler_task_state_read_models(
            WorkflowSchedulerTaskStateReadModelQueryRequest {
                session_id: session_id.into(),
                workflow_run_id,
            },
        )
        .await?)
}

fn record_result<T: serde::Serialize>(result: &CaseResult<T>) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => error_record(error.as_ref()),
    }
}

fn error_record(error: &(dyn Error + 'static)) -> Value {
    let mut causes = Vec::new();
    let mut source = error.source();
    while let Some(cause) = source {
        causes.push(cause.to_string());
        source = cause.source();
        if causes.len() == 16 {
            break;
        }
    }
    json!({"ok":false,"error":error.to_string(),"debug":format!("{error:?}"),"causes":causes,"cause_limit":16,"causes_truncated":source.is_some()})
}

fn retain_first_run_error(
    observation: &Mutex<Value>,
    result: &Result<WorkflowRunResponse, WorkflowServiceError>,
) {
    if let Err(error) = result {
        let mut observation = observation.lock().unwrap();
        let record = error_record(error);
        if observation["first_error"].is_null() {
            observation["first_error"] = record.clone();
        }
        // One first readiness error and the latest outcome are bounded even
        // when the existing readiness polling loop observes pending repeatedly.
        observation["latest_error"] = record;
    }
}

fn label_await(evidence: &mut Value, label: &str) {
    evidence["await"] = json!(label);
    let history = evidence
        .as_object_mut()
        .unwrap()
        .entry("await_history")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .unwrap();
    if history.len() < 64 {
        history.push(json!(label));
    }
}

type Execution = tokio::task::JoinHandle<CaseResult<WorkflowRunResponse>>;

// Borrow the retained handle: cancelling the observer never detaches the run.
async fn await_boundary(
    execution: &mut Option<Execution>,
    entered: &Notify,
    evidence: &mut Value,
    label: &str,
) -> CaseResult<()> {
    label_await(evidence, label);
    let terminal = tokio::time::timeout(WATCHDOG, async {
        tokio::select! {
            biased;
            _ = entered.notified() => Ok(None),
            result = execution.as_mut().ok_or("execution owner absent")? => Ok(Some(result)),
        }
    })
    .await?;
    let terminal: CaseResult<_> = terminal;
    if let Some(terminal) = terminal? {
        // The terminal outcome is observed exactly once, including join errors.
        *execution = None;
        match terminal {
            Ok(result) => {
                evidence["execution_before_boundary"] = record_result(&result);
                result?;
                return Err(format!("execution completed before {label}").into());
            }
            Err(error) => {
                evidence["execution_before_boundary"] = error_record(&error);
                return Err(error.into());
            }
        }
    }
    Ok(())
}

// Capture before shutdown closes the public observation surface. These reads
// neither refresh projections nor enqueue readiness work or native execution.
async fn capture_case_snapshots(
    facade: &EmbeddedSerialCpuResidency,
    directory: &Path,
    evidence: &mut Value,
    phase: &str,
) {
    let mut snapshot = json!({"registry":registry_facts(&facade.registry_snapshot()),"native":facade.residency_diagnostics(),
        "raw_readiness":{"ok":false,"unavailable":"facade does not expose raw readiness Check/request/binding/provider snapshots; scheduler diagnostics and resume requests are observations only"}});
    if let Some(session_id) = evidence["session_id"].as_str().map(str::to_owned) {
        label_await(evidence, &format!("{phase}:scheduler_snapshot"));
        snapshot["scheduler"] = match tokio::time::timeout(
            WATCHDOG,
            facade.workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: session_id.clone(),
            }),
        )
        .await
        {
            Ok(Ok(value)) => json!({"ok":true,"value":value}),
            Ok(Err(error)) => error_record(&error),
            Err(error) => error_record(&error),
        };
        label_await(evidence, &format!("{phase}:task_states"));
        snapshot["tasks"] = match tokio::time::timeout(
            WATCHDOG,
            task_states(facade, &session_id, evidence),
        )
        .await
        {
            Ok(result) => record_result(&result),
            Err(error) => error_record(&error),
        };
        snapshot["readiness_resume_candidates"] = match facade
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        {
            Ok(value) => json!({"ok":true,"value":value}),
            Err(error) => error_record(&error),
        };
    }
    let path = directory.join("app-data/serial-cpu-diagnostics.db");
    snapshot["raw_ledger"] = if path.is_file() {
        match SqliteDiagnosticsLedger::open(path)
            .and_then(|ledger| ledger.diagnostic_events_after(0, 129))
        {
            Ok(mut events) => {
                let truncated = events.len() > 128;
                events.truncate(128);
                json!({"ok":true,"after_event_seq":0,"limit":128,"truncated":truncated,"events":events})
            }
            Err(error) => error_record(&error),
        }
    } else {
        json!({"ok":false,"error":"diagnostics ledger was not created"})
    };
    evidence[phase] = snapshot;
}

async fn join_execution(
    execution: &mut Option<Execution>,
) -> CaseResult<CaseResult<WorkflowRunResponse>> {
    let result = tokio::time::timeout(
        WATCHDOG,
        execution.as_mut().ok_or("execution owner absent")?,
    )
    .await?;
    // Completed handles cannot be polled twice, including when their join failed.
    *execution = None;
    Ok(result?)
}

// Early oracle errors must not silently detach a still-owned execution. Gates
// have already been released by Drop; the existing facade supervisor owns the
// actual cancellation/drain/stop, including its honest fenced failure outcome.
async fn finish_failed_case(
    facade: Option<&Arc<EmbeddedSerialCpuResidency>>,
    execution: &mut Option<Execution>,
    evidence: &mut Value,
) {
    let Some(facade) = facade else { return };
    label_await(evidence, "failure_teardown:facade_shutdown");
    let shutdown = match tokio::time::timeout(WATCHDOG, facade.shutdown()).await {
        Ok(result) => match result {
            Ok(()) => json!({"ok":true}),
            Err(error) => json!({"ok":false,"error":error.to_string()}),
        },
        Err(error) => {
            json!({"ok":false,"watchdog":error.to_string(),"supervisor":"retained_by_facade"})
        }
    };
    label_await(evidence, "failure_teardown:execution_join");
    let run = if execution.is_some() {
        match join_execution(execution).await {
            Ok(result) => record_result(&result),
            Err(error) => error_record(error.as_ref()),
        }
    } else {
        // An early terminal error was already retained by await_boundary.
        evidence
            .get("execution_before_boundary")
            .or_else(|| evidence.get("run"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    evidence["failure_teardown"] = json!({"shutdown":shutdown,"run":run,
        "registry":registry_facts(&facade.registry_snapshot()),"native":facade.residency_diagnostics()});
}

#[tokio::test]
async fn actual_post_load_pre_start_shutdown_signal_fences_guard_without_running_attempt() {
    const CASE: &str =
        "actual_post_load_pre_start_shutdown_signal_fences_guard_without_running_attempt";
    let mut evidence = json!({"case_id":CASE,"signal_source":"private_facade_owned_preparation_shutdown_signal","boundary":"after_actual_load_and_publication_before_idle_cap_and_Start"});
    let directory = TempDir::new().expect("isolated native fixture directory");
    let mut owned_facade = None;
    let mut execution = None;
    let run_observation = Arc::new(Mutex::new(json!({})));
    let result: CaseResult<()> = match tokio::time::timeout(CASE_OBSERVER, async {
        seed(directory.path());
        label_await(&mut evidence, "facade_construction");
        let facade = Arc::new(EmbeddedSerialCpuResidency::new(config(directory.path())).await?);
        owned_facade = Some(facade.clone());
        label_await(&mut evidence, "attributed_session_creation");
        let session_id = session(&facade, &mut evidence).await?;
        evidence["session_id"] = json!(session_id);
        let entered = Arc::new(Notify::new());
        let proceed = Arc::new(Notify::new());
        facade.set_residency_after_load_gate(entered.clone(), proceed.clone());
        let release = ReleaseGate(proceed.clone());
        execution = Some(tokio::spawn(run(
            facade.clone(),
            session_id.clone(),
            run_observation.clone(),
        )));
        await_boundary(
            &mut execution,
            &entered,
            &mut evidence,
            "after_actual_load_before_Start",
        )
        .await?;
        let before = facade.registry_snapshot();
        label_await(&mut evidence, "gate:task_states");
        let states = task_states(&facade, &session_id, &mut evidence).await?;
        let diagnostic = facade
            .residency_diagnostics()
            .ok_or("missing diagnostics")?;
        evidence["at_gate"] =
            json!({"tasks":states,"registry":registry_facts(&before),"native":diagnostic});
        capture_case_snapshots(&facade, directory.path(), &mut evidence, "gate_snapshot").await;
        let task = states
            .tasks
            .iter()
            .find(|task| task.node_id == "infer")
            .ok_or("missing inference task")?;
        require(
            task.state == SchedulerTaskStateKind::Ready && task.active_attempt_id.is_none(),
            "post-load task must still be Ready without a Running attempt",
        )?;
        require(
            diagnostic.loads == 1 && diagnostic.forward_drains == 0 && diagnostic.stops == 0,
            "exactly one genuine load and zero forwards/stops required",
        )?;
        require(
            before.reservations.len() == 1
                && before
                    .runtimes
                    .iter()
                    .any(|runtime| runtime.model_resource_residency.is_some()),
            "actual guard and resident charges required before shutdown",
        )?;
        let mut shutdown = Box::pin(facade.shutdown());
        require(
            futures_util::poll!(shutdown.as_mut()).is_pending(),
            "shutdown must close admission while native gate remains held",
        )?;
        evidence["owned_shutdown_signal_requested"] = json!(true);
        proceed.notify_one();
        drop(release);
        label_await(&mut evidence, "pre_Start:facade_shutdown");
        let shutdown_result: CaseResult<()> = tokio::time::timeout(WATCHDOG, shutdown)
            .await?
            .map_err(Into::into);
        evidence["shutdown"] = record_result(&shutdown_result);
        label_await(&mut evidence, "terminal:execution_join");
        let run_result = join_execution(&mut execution).await?;
        evidence["run"] = record_result(&run_result);
        let after = facade.registry_snapshot();
        let diagnostic = facade
            .residency_diagnostics()
            .ok_or("missing final diagnostics")?;
        evidence["after_shutdown"] = json!({"registry":registry_facts(&after),"native":diagnostic});
        require(
            shutdown_result.is_err() && run_result.is_err(),
            "pre-Start shutdown must refuse run and report fenced shutdown",
        )?;
        require(
            same_charges(&before, &after),
            "failed pre-Start shutdown must preserve exact full guard/resident charges",
        )?;
        require(
            diagnostic.loads == 1
                && diagnostic.forward_drains == 0
                && diagnostic.stops == 0
                && diagnostic.fenced
                && !diagnostic.has_idle_owner,
            "post-load signal must retain an inaccessible full fence without forward or stop",
        )?;
        Ok(())
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(error.into()),
    };
    evidence["run_observation"] = run_observation.lock().unwrap().clone();
    if result.is_err() {
        evidence["first_observer_error"] = record_result(&result);
        evidence["failed_await"] = evidence["await"].clone();
        if let Some(facade) = owned_facade.as_ref() {
            capture_case_snapshots(
                facade,
                directory.path(),
                &mut evidence,
                "before_failure_teardown",
            )
            .await;
        }
        finish_failed_case(owned_facade.as_ref(), &mut execution, &mut evidence).await;
    }
    if let Some(facade) = owned_facade.as_ref() {
        capture_case_snapshots(facade, directory.path(), &mut evidence, "final_snapshot").await;
    }
    evidence["run_observation"] = run_observation.lock().unwrap().clone();
    evidence["oracle"] = record_result(&result);
    save_evidence(CASE, &evidence)
        .expect("preserve native qualification evidence without replacement");
    result.unwrap();
}

#[tokio::test]
async fn actual_public_running_post_drain_cancellation_preserves_cap_and_exact_cleanup() {
    const CASE: &str =
        "actual_public_running_post_drain_cancellation_preserves_cap_and_exact_cleanup";
    let mut evidence = json!({"case_id":CASE,"signal_source":"public_active_task_cancellation","boundary":"actual_native_drained_receipt_held_while_public_Running","claim":"post-drain request; completion may win; no in-flight interruption or training claim"});
    let directory = TempDir::new().expect("isolated native fixture directory");
    let mut owned_facade = None;
    let mut execution = None;
    let run_observation = Arc::new(Mutex::new(json!({})));
    let result: CaseResult<()> = match tokio::time::timeout(CASE_OBSERVER, async {
        seed(directory.path());
        label_await(&mut evidence, "facade_construction");
        let facade = Arc::new(EmbeddedSerialCpuResidency::new(config(directory.path())).await?);
        owned_facade = Some(facade.clone());
        label_await(&mut evidence, "attributed_session_creation");
        let session_id = session(&facade, &mut evidence).await?;
        evidence["session_id"] = json!(session_id);
        let entered = Arc::new(Notify::new());
        let proceed = Arc::new(Notify::new());
        facade.set_residency_after_drain_gate(entered.clone(), proceed.clone());
        let release = ReleaseGate(proceed.clone());
        execution = Some(tokio::spawn(run(facade.clone(), session_id.clone(), run_observation.clone())));
        await_boundary(&mut execution, &entered, &mut evidence, "after_actual_forward_drain_while_Running").await?;
        let before = facade.registry_snapshot();
        label_await(&mut evidence, "gate:task_states");
        let states = task_states(&facade, &session_id, &mut evidence).await?;
        let task = states.tasks.iter().find(|task|task.node_id == "infer").ok_or("missing inference task")?;
        let diagnostic = facade.residency_diagnostics().ok_or("missing diagnostics")?;
        evidence["at_gate"] = json!({"tasks":states,"registry":registry_facts(&before),"native":diagnostic});
        capture_case_snapshots(&facade, directory.path(), &mut evidence, "gate_snapshot").await;
        require(task.state == SchedulerTaskStateKind::Running && task.active_attempt_id.is_some(), "genuine drained result must be held during public Running")?;
        require(diagnostic.loads == 1 && diagnostic.forward_drains == 1 && diagnostic.stops == 0 && diagnostic.fenced && !diagnostic.has_idle_owner, "exactly one genuine load/forward/drain with cap in receipt required")?;
        require(before.reservations.len() == 2, "mode guard and exact task lease must remain charged at drain gate")?;
        let task_owner = format!("{}:{}", states.workflow_run_id, task.task_id);
        let task_lease = before.reservations.iter().find(|lease| lease.reservation_owner_id.as_deref() == Some(task_owner.as_str())).ok_or("exact Running task lease absent")?;
        let guard_lease = before.reservations.iter().find(|lease| lease.workflow_id == "cpu-serial-residency-owner").ok_or("mode guard lease absent")?;
        require(task_lease.reservation_id != guard_lease.reservation_id, "guard and task must be distinct actual leases")?;
        evidence["actual_task_identity"] = json!({"workflow_run_id":states.workflow_run_id,"task_id":task.task_id,"attempt_id":task.active_attempt_id,"task_lease_id":task_lease.reservation_id,"guard_lease_id":guard_lease.reservation_id});
        label_await(&mut evidence, "public_active_task_cancellation");
        let cancellation = facade.workflow_cancel_active_execution_session_task(WorkflowExecutionSessionActiveTaskCancelRequest {
            session_id:session_id.clone(), workflow_run_id:states.workflow_run_id.clone(), task_id:task.task_id.clone(),
            reason:Some("native M4 qualification: actual post-drain request".into()),
        }).await?;
        let after_request = facade.registry_snapshot();
        label_await(&mut evidence, "after_cancellation:task_states");
        let requested_states = task_states(&facade, &session_id, &mut evidence).await?;
        evidence["public_cancellation"] = json!(cancellation);
        evidence["after_request"] = json!({"tasks":requested_states,"registry":registry_facts(&after_request)});
        require(cancellation.ok && cancellation.status == WorkflowExecutionSessionActiveTaskCancelStatus::CancellationRequested && Some(cancellation.active_attempt_id.as_str()) == task.active_attempt_id.as_deref(), "public cancellation must target the actual Running attempt")?;
        require(requested_states.tasks.iter().any(|requested|requested.task_id == task.task_id && requested.state == SchedulerTaskStateKind::Running && requested.active_attempt_id == task.active_attempt_id), "public cancellation request must preserve the held Running attempt before receipt release")?;
        require(same_charges(&before, &after_request), "public cancellation request must not release any task or resident charge")?;
        proceed.notify_one();
        drop(release);
        label_await(&mut evidence, "terminal:execution_join");
        let run_result = join_execution(&mut execution).await?;
        evidence["run"] = record_result(&run_result);
        let after = facade.registry_snapshot();
        let diagnostic = facade.residency_diagnostics().ok_or("missing final diagnostics")?;
        evidence["after_run"] = json!({"registry":registry_facts(&after),"native":diagnostic});
        // Inspection reads persisted projections. Advance only the two needed
        // for terminal observation, once, after the actual execution has joined.
        // This metadata operation cannot supply a native completion or ACK.
        let refresh = facade.workflow_diagnostics_projection_refresh(
            WorkflowDiagnosticsProjectionRefreshRequest {
                projections: vec![WorkflowDiagnosticsProjectionKind::RunDetail, WorkflowDiagnosticsProjectionKind::NodeStatus],
                workflow_run_id: Some(states.workflow_run_id.clone()),
                workflow_id: Some(task.workflow_id.clone()),
                reason: WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                batch_size: 128,
            },
        );
        evidence["terminal_projection_refresh"] = match &refresh {
            Ok(refresh) => json!({"ok":true,"response":refresh}),
            Err(error) => json!({"ok":false,"error":error.to_string()}),
        };
        let refresh = refresh?;
        require(refresh.failed.is_empty() && refresh.advanced.len() == 2
            && [WorkflowDiagnosticsProjectionKind::RunDetail, WorkflowDiagnosticsProjectionKind::NodeStatus].iter().all(|kind|refresh.advanced.iter().filter(|advanced|advanced.projection_kind == *kind).count() == 1),
            "explicit terminal metadata refresh must advance both requested projections without failure")?;
        // Both projections start empty in this fresh owner. Each positive
        // cursor below 128 implies fewer than 128 distinct relevant events in
        // its first page. Their event-kind filters may end at different cursors.
        let page_caught_up = refresh.advanced.iter().all(|advanced| {
            let cursor = advanced.projection_state.last_applied_event_seq;
            cursor > 0 && cursor < 128
        });
        evidence["terminal_projection_page_caught_up"] = json!(page_caught_up);
        require(page_caught_up, "fresh terminal projections must catch up within their one explicit 128-event page")?;
        let metadata_preserved_charges = same_charges(&after, &facade.registry_snapshot());
        evidence["metadata_refresh_preserved_charges"] = json!(metadata_preserved_charges);
        require(metadata_preserved_charges, "terminal metadata refresh must preserve every native charge")?;
        let inspection = facade.workflow_run_inspection_query(WorkflowRunInspectionQueryRequest {
            workflow_run_id:states.workflow_run_id.clone(), artifact_limit:Some(1), projection_batch_size:Some(128),
        });
        let terminal_observed = inspection.as_ref().ok().and_then(|inspection|inspection.run.as_ref()).is_some_and(|run|matches!(run.status,RunListProjectionStatus::Completed | RunListProjectionStatus::Failed | RunListProjectionStatus::Cancelled));
        evidence["terminal_inspection"] = match inspection {
            Ok(inspection) => json!({"ok":true,
                "run":inspection.run.as_ref().map(|run|json!({"workflow_run_id":run.workflow_run_id,"status":run.status,"started_at_ms":run.started_at_ms,"completed_at_ms":run.completed_at_ms,"terminal_error":run.terminal_error})),
                "nodes":inspection.node_statuses.iter().map(|node|json!({"workflow_run_id":node.workflow_run_id,"node_id":node.node_id,"task_id":node.task_id,"status":node.status,"started_at_ms":node.started_at_ms,"completed_at_ms":node.completed_at_ms,"error":node.error})).collect::<Vec<_>>(),
                "run_projection_state":inspection.run_projection_state,"node_projection_state":inspection.node_projection_state}),
            Err(error) => json!({"ok":false,"error":error.to_string()}),
        };
        require(terminal_observed, "actual run terminal state must be observed without inventing a cancellation winner")?;
        let mut expected_after_task_ack = before.clone();
        expected_after_task_ack.reservations.retain(|lease|lease.reservation_id == guard_lease.reservation_id);
        for runtime in &mut expected_after_task_ack.runtimes {
            runtime.active_reservation_ids.retain(|id|*id == guard_lease.reservation_id);
            runtime.active_reservation_claims.retain(|claim|claim.reservation_id == guard_lease.reservation_id);
        }
        evidence["expected_after_task_ack"] = registry_facts(&expected_after_task_ack);
        let exact_cleanup = same_charges(&expected_after_task_ack,&after) && diagnostic.has_idle_owner && !diagnostic.fenced;
        let honest_full_fence = same_charges(&before,&after) && diagnostic.fenced && !diagnostic.has_idle_owner;
        evidence["cleanup_disposition"] = json!(if exact_cleanup {"exact_task_release_ack_and_cap_returned"} else if honest_full_fence {"full_charges_fenced"} else {"invalid"});
        require(exact_cleanup || honest_full_fence, "receipt must produce exact task cleanup/cap return or preserve every charge behind a full fence")?;
        require(diagnostic.loads == 1 && diagnostic.forward_drains == 1 && diagnostic.stops == 0, "no extra native work is admitted by cancellation")?;
        label_await(&mut evidence, "post_drain:facade_shutdown");
        let shutdown_result: CaseResult<()> = tokio::time::timeout(WATCHDOG,facade.shutdown()).await?.map_err(Into::into);
        let stopped = facade.registry_snapshot();
        let final_diagnostic = facade.residency_diagnostics().ok_or("missing shutdown diagnostics")?;
        evidence["shutdown"] = record_result(&shutdown_result);
        evidence["after_shutdown"] = json!({"registry":registry_facts(&stopped),"native":final_diagnostic});
        if exact_cleanup {
            require(shutdown_result.is_ok() && stopped.reservations.is_empty() && stopped.runtimes.iter().all(|runtime|runtime.active_reservation_ids.is_empty() && runtime.active_reservation_claims.is_empty() && runtime.model_resource_residency.is_none() && !runtime.resident_resources_uncertain) && final_diagnostic.stops == 1 && !final_diagnostic.has_idle_owner && !final_diagnostic.fenced, "returned cap must stop/ACK once and clear all charges at shutdown")?;
        } else {
            require(shutdown_result.is_err() && same_charges(&after,&stopped) && final_diagnostic.fenced && final_diagnostic.stops == 0, "uncertain cleanup must remain fully charged and report fenced shutdown")?;
        }
        Ok(())
    }).await {
        Ok(result) => result,
        Err(error) => Err(error.into()),
    };
    evidence["run_observation"] = run_observation.lock().unwrap().clone();
    if result.is_err() {
        evidence["first_observer_error"] = record_result(&result);
        evidence["failed_await"] = evidence["await"].clone();
        if let Some(facade) = owned_facade.as_ref() {
            capture_case_snapshots(
                facade,
                directory.path(),
                &mut evidence,
                "before_failure_teardown",
            )
            .await;
        }
        finish_failed_case(owned_facade.as_ref(), &mut execution, &mut evidence).await;
    }
    if let Some(facade) = owned_facade.as_ref() {
        capture_case_snapshots(facade, directory.path(), &mut evidence, "final_snapshot").await;
    }
    evidence["run_observation"] = run_observation.lock().unwrap().clone();
    evidence["oracle"] = record_result(&result);
    save_evidence(CASE, &evidence)
        .expect("preserve native qualification evidence without replacement");
    result.unwrap();
}

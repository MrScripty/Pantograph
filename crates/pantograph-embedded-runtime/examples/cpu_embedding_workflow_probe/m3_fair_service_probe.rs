//! One fixed actual CPU cohort, using authenticated attribution and native receipts.
//! Admission barriers control arrival order; they do not mint Ready or timing facts.
use super::*;
use async_trait::async_trait;
use pantograph_runtime_attribution::{
    BucketSelection, ClientRegistrationRequest, ClientSessionOpenRequest,
};
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerSerialBoundDispatch, SchedulerSerialDrainedDispatch,
    SchedulerSerialOwnerSnapshot,
};
use pantograph_workflow_service::{
    WorkflowExecutionSessionActiveTaskCancelRequest,
    WorkflowExecutionSessionAttributedCreateRequest, WorkflowExecutionSessionAttributionRequest,
    WorkflowExecutionSessionCreateResponse, WorkflowExecutionSessionQueueListRequest,
};
use tokio::sync::{mpsc, oneshot, watch};

const PROTOCOL: &str = "m3-authenticated-service-v1";
const FOREGROUND_SESSIONS: usize = 4;
const CALLS: usize = 10; // Four foreground pairs, one background pair. Never restart.
const WATCHDOG: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Observations {
    rows: Vec<Value>,
    overflow: bool,
}
type SharedObservations = Arc<Mutex<Observations>>;

fn record(observations: &SharedObservations, row: Value) {
    let mut state = observations.lock().expect("isolated M3 observations");
    if state.rows.len() >= 128 {
        state.overflow = true;
    } else {
        state.rows.push(row);
    }
}

struct ServiceLog(Mutex<Observations>);
static SERVICE_LOG: ServiceLog = ServiceLog(Mutex::new(Observations {
    rows: Vec::new(),
    overflow: false,
}));
impl log::Log for ServiceLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.target() == "workflow::serial_ready"
    }
    fn log(&self, event: &log::Record<'_>) {
        if !self.enabled(event.metadata()) {
            return;
        }
        let line = event.args().to_string();
        let mut state = self.0.lock().expect("isolated M3 service log");
        if line.len() > 8192 || state.rows.len() >= 128 {
            state.overflow = true;
            return;
        }
        match serde_json::from_str(&line) {
            Ok(row) => state.rows.push(row),
            Err(_) => state.overflow = true,
        }
    }
    fn flush(&self) {}
}

struct Barrier {
    phase: &'static str,
    identity: Value,
    release: oneshot::Sender<()>,
}

/// A transparent native delegate. The bounded coordinator owns each barrier;
/// coordinator loss releases it by dropping the oneshot sender. Actual execution
/// stays supervised by the workflow worker and participates in runtime shutdown.
struct ControlledPort {
    native: EmbeddedRetainedCpuSerialPort,
    barriers: mpsc::Sender<Barrier>,
    observations: SharedObservations,
}

impl ControlledPort {
    async fn barrier(&self, phase: &'static str, identity: &Value) {
        let (release, wait) = oneshot::channel();
        if self
            .barriers
            .send(Barrier {
                phase,
                identity: identity.clone(),
                release,
            })
            .await
            .is_ok()
        {
            // Coordinator loss releases admission, never certifies native stop.
            let _ = wait.await;
        }
    }
}

struct ObserveCleanup {
    native: Box<dyn SerialRuntimeHostCleanupReceipt>,
    identity: Value,
    observations: SharedObservations,
}

fn owner_row(owner: SerialRuntimeHostCpuOwnerEvidence) -> Value {
    let snapshot = owner.snapshot;
    json!({"loaded_instance":snapshot.loaded_instance,"loaded_profile":snapshot.loaded_profile,
        "effective_settings":snapshot.effective_settings,"generation":snapshot.generation,
        "cpu_threads":snapshot.cpu_threads,"current_at_observation":owner.is_current()})
}

#[async_trait]
impl SerialRuntimeHostCleanupReceipt for ObserveCleanup {
    fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.native.previous_owner()
    }
    fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.native.current_owner()
    }
    async fn apply(
        self: Box<Self>,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let input = json!({"phase":"cleanup_requested","identity":self.identity,"event":event});
        record(&self.observations, input);
        let result = self.native.apply(event).await;
        record(
            &self.observations,
            json!({"phase":"cleanup_result","identity":self.identity,
            "application":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string)}),
        );
        result
    }
}

#[async_trait]
impl RuntimeHostBatchExecutionPort for ControlledPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        self.native
            .execute_runtime_host_batch_request(request, cancellation)
            .await
    }
}
#[async_trait]
impl ReservationLifecyclePort for ControlledPort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native.apply_reservation_lifecycle(event).await
    }
}
#[async_trait]
impl SerialRuntimeHostBatchExecutionPort for ControlledPort {
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        self.native.resident_serial_cpu_owner(intent)
    }
    async fn execute_serial_singleton(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    > {
        self.native
            .execute_serial_singleton(request, cancellation, ownership)
            .await
    }
    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected_owner: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native
            .apply_serial_cleanup(event, expected_owner)
            .await
    }
    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        let id = ownership.identity();
        let identity = json!({"workflow_id":id.workflow_id,"workflow_run_id":id.workflow_run_id,
            "node_id":id.node_id,"task_id":id.task_id,"attempt_id":id.attempt_id,
            "execution_request_id":id.execution_request_id,"candidate_id":id.candidate_id,
            "reservation_lease_id":id.reservation_lease_id});
        record(
            &self.observations,
            json!({"phase":"pre_native","identity":identity,
            "request":request,"request_blake3":blake3::hash(&serde_json::to_vec(&request)
                .expect("native request serializes")).to_hex().to_string(),
            "cancellation":cancellation.snapshot()}),
        );
        self.barrier("pre_native", &identity).await;
        record(
            &self.observations,
            json!({"phase":"native_delegate","identity":identity,
            "cancellation":cancellation.snapshot()}),
        );
        let result = self
            .native
            .execute_serial_singleton_with_cleanup(request, cancellation.clone(), ownership)
            .await;
        match result {
            Ok(mut execution) => {
                record(
                    &self.observations,
                    json!({"phase":"native_result","identity":identity,
                    "response":execution.response,"owner":execution.owner.as_ref().map(|o|owner_row(o.clone())),
                    "receipt_previous_owner":execution.cleanup.as_ref().map(|r|owner_row(r.previous_owner())),
                    "receipt_current_owner":execution.cleanup.as_ref().map(|r|owner_row(r.current_owner())),
                    "cancellation":cancellation.snapshot()}),
                );
                self.barrier("post_native_drain", &identity).await;
                record(
                    &self.observations,
                    json!({"phase":"native_return","identity":identity,
                    "cancellation":cancellation.snapshot()}),
                );
                if let Some(native) = execution.cleanup.take() {
                    execution.cleanup = Some(Box::new(ObserveCleanup {
                        native,
                        identity,
                        observations: self.observations.clone(),
                    }));
                }
                Ok(execution)
            }
            Err(error) => {
                record(
                    &self.observations,
                    json!({"phase":"native_result","identity":identity,
                    "error":error.to_string(),"cancellation":cancellation.snapshot(),
                    "linear_cleanup_receipt_returned":false}),
                );
                Err(error)
            }
        }
    }
}

async fn attributed_sessions(
    runtime: &EmbeddedRuntime,
    sessions: &mut Vec<WorkflowExecutionSessionCreateResponse>,
) -> ProbeResult<()> {
    for (name, count) in [("foreground", FOREGROUND_SESSIONS), ("background", 1)] {
        let registration =
            runtime
                .workflow_service()
                .register_attribution_client(ClientRegistrationRequest {
                    display_name: Some(format!("M3 {name}")),
                    metadata_json: None,
                })?;
        let opened = runtime
            .workflow_service()
            .open_client_session(ClientSessionOpenRequest {
                credential: registration.credential_proof_request(),
                takeover: false,
                reason: Some("fixed CPU qualification".into()),
            })?;
        for _ in 0..count {
            sessions.push(
                runtime
                    .create_attributed_workflow_execution_session(
                        WorkflowExecutionSessionAttributedCreateRequest {
                            workflow_id: PAIR_WORKFLOW.into(),
                            usage_profile: None,
                            keep_alive: false,
                            attribution: WorkflowExecutionSessionAttributionRequest {
                                credential: registration.credential_proof_request(),
                                client_session_id: opened.session.client_session_id.as_str().into(),
                                bucket_selection: BucketSelection::Default,
                            },
                        },
                    )
                    .await?,
            );
        }
    }
    Ok(())
}

async fn attempt(
    runtime: &EmbeddedRuntime,
    root: &Path,
    session: &str,
    index: usize,
    texts: &[String; 2],
    mut start: Option<watch::Receiver<bool>>,
) -> ProbeResult<Value> {
    if let Some(start) = start.as_mut() {
        start.wait_for(|ready| *ready).await?;
    }
    let mut resume_ids = Vec::new();
    let started = Instant::now();
    let mut result = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.into(),
            workflow_semantic_version: "1.0.0".into(),
            inputs: texts
                .iter()
                .enumerate()
                .map(|(position, text)| WorkflowPortBinding {
                    node_id: format!("prompt_{}", ["a", "b"][position]),
                    port_id: "text".into(),
                    value: json!(text),
                })
                .collect(),
            output_targets: Some(
                ["vectors_a", "vectors_b"]
                    .into_iter()
                    .map(|node| WorkflowOutputTarget {
                        node_id: node.into(),
                        port_id: "vector".into(),
                    })
                    .collect(),
            ),
            override_selection: None,
            timeout_ms: None,
            priority: Some(if index < FOREGROUND_SESSIONS {
                100
            } else {
                -100
            }),
        })
        .await;
    while matches!(
        result,
        Err(WorkflowServiceError::RuntimeDependencyReadinessPending { .. })
    ) {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let candidates = match runtime
            .workflow_service()
            .workflow_execution_session_runtime_dependency_readiness_resume_candidates()
        {
            Ok(candidates) => candidates,
            Err(error) => {
                result = Err(error);
                break;
            }
        };
        if let Some(request) = candidates.into_iter().find(|r| r.session_id == session) {
            if resume_ids.len() == 64 {
                break;
            }
            resume_ids.push(request.workflow_run_id.clone());
            result = runtime
                .resume_workflow_execution_session_runtime_dependency_readiness(request)
                .await;
        }
        if started.elapsed() > WATCHDOG {
            break;
        }
    }
    let row = json!({"index":index,"session_id":session,"priority":if index<FOREGROUND_SESSIONS {100}else{-100},
        "wall_ns":started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        "response":result.as_ref().ok(),"execution_error":result.as_ref().err().map(ToString::to_string),
        "readiness_resume_run_ids":resume_ids});
    // Failed native/public results are exported before any propagation.
    save(root, &format!("m3-attempt-{index}.json"), &row)?;
    Ok(row)
}

async fn coordinate(
    runtime: &EmbeddedRuntime,
    registry: &RuntimeRegistry,
    root: &Path,
    sessions: &[WorkflowExecutionSessionCreateResponse],
    mut barriers: mpsc::Receiver<Barrier>,
    start: watch::Sender<bool>,
    observations: SharedObservations,
) -> ProbeResult<()> {
    let mut foreground_calls = 0;
    for ordinal in 0..(CALLS * 2) {
        let barrier = tokio::time::timeout(WATCHDOG, barriers.recv())
            .await?
            .ok_or("native barrier channel ended before fixed cohort")?;
        let mut session_index = None;
        for (index, session) in sessions.iter().enumerate() {
            let queue = runtime
                .workflow_service()
                .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
                    session_id: session.session_id.clone(),
                })
                .await?;
            if queue
                .items
                .iter()
                .any(|item| json!(item.workflow_run_id) == barrier.identity["workflow_run_id"])
                && session_index.replace(index).is_some()
            {
                return Err("run appears in two session queues".into());
            }
        }
        let session_index = session_index.ok_or("native run absent from public session queues")?;
        record(
            &observations,
            json!({"phase":"barrier","barrier_phase":barrier.phase,"ordinal":ordinal,
            "session_index":session_index,"identity":barrier.identity,
            "registry":registry.snapshot(),"pending":runtime.workflow_service().serial_service_pending_for_test()?}),
        );
        if ordinal == 0 {
            start.send(true)?;
            let deadline = Instant::now();
            while runtime
                .workflow_service()
                .serial_service_pending_for_test()?
                != 4
            {
                if deadline.elapsed() > WATCHDOG {
                    return Err("four real Ready waiters did not register".into());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            record(
                &observations,
                json!({"phase":"arrival_barrier_satisfied","pending":4,
                "registry":registry.snapshot()}),
            );
        }
        if barrier.phase == "post_native_drain" && session_index < FOREGROUND_SESSIONS {
            foreground_calls += 1;
            if foreground_calls == 8 {
                let cancellation = runtime
                    .workflow_service()
                    .workflow_cancel_active_execution_session_task(
                        WorkflowExecutionSessionActiveTaskCancelRequest {
                            session_id: sessions[session_index].session_id.clone(),
                            workflow_run_id: barrier.identity["workflow_run_id"]
                                .as_str()
                                .ok_or("missing run ID")?
                                .into(),
                            task_id: barrier.identity["task_id"]
                                .as_str()
                                .ok_or("missing task ID")?
                                .into(),
                            reason: Some(
                                "fixed M3 final foreground cancellation after actual native drain"
                                    .into(),
                            ),
                        },
                    )
                    .await;
                record(
                    &observations,
                    json!({"phase":"cancellation_request","identity":barrier.identity,
                    "response":cancellation.as_ref().ok(),"error":cancellation.as_ref().err().map(ToString::to_string),
                    "registry_after_drain_before_cleanup":registry.snapshot(),"in_flight_compute_cancellation_claim":false}),
                );
                // Release even on cancellation refusal; the real owner still drains.
                let _ = barrier.release.send(());
                cancellation?;
                continue;
            }
        }
        let _ = barrier.release.send(());
    }
    save(
        root,
        "m3-coordination.json",
        &json!({"barriers":CALLS*2,"foreground_drains":foreground_calls}),
    )?;
    Ok(())
}

fn check_evidence(
    rows: &[Value],
    sessions: &[WorkflowExecutionSessionCreateResponse],
    native: &[Value],
    service_log: &[Value],
    events: &[Value],
    samples: &RawSamples,
    golden: &Value,
) -> ProbeResult<Value> {
    if rows.len() != FOREGROUND_SESSIONS + 1 || sessions.len() != FOREGROUND_SESSIONS + 1 {
        return Err("fixed five-session cohort incomplete".into());
    }
    let foreground = sessions[0]
        .attribution
        .as_ref()
        .ok_or("missing authenticated foreground context")?;
    let background_unit = sessions[FOREGROUND_SESSIONS]
        .attribution
        .as_ref()
        .ok_or("missing authenticated background context")?;
    if sessions[..FOREGROUND_SESSIONS]
        .iter()
        .any(|session| session.attribution.as_ref() != Some(foreground))
        || (foreground.client_id == background_unit.client_id
            && foreground.bucket_id == background_unit.bucket_id)
    {
        return Err(
            "actual session contexts do not prove two authenticated entitlement units".into(),
        );
    }
    let mut run_ids = Vec::new();
    for row in rows {
        let response: WorkflowRunResponse = serde_json::from_value(row["response"].clone())?;
        if response.workflow_run_id.is_empty() || run_ids.contains(&response.workflow_run_id) {
            return Err("public runs must be five distinct successful runs".into());
        }
        if row["readiness_resume_run_ids"]
            .as_array()
            .ok_or("missing resume evidence")?
            .iter()
            .any(|id| id != &json!(response.workflow_run_id))
        {
            return Err("readiness resume changed actual run identity".into());
        }
        verify_vectors(&response, &golden["truncation_vectors"])?;
        run_ids.push(response.workflow_run_id);
    }
    let starts: Vec<_> = events
        .iter()
        .filter(|event| {
            event["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"
                && event["payload"]["transition"] == "started"
                && [json!("infer_a"), json!("infer_b")].contains(&event["node_id"])
        })
        .collect();
    let grants: Vec<_> = service_log
        .iter()
        .filter(|row| row["phase"] == "service_opportunity")
        .collect();
    let results: Vec<_> = native
        .iter()
        .filter(|row| row["phase"] == "native_result")
        .collect();
    let cleanups: Vec<_> = native
        .iter()
        .filter(|row| row["phase"] == "cleanup_result")
        .collect();
    if starts.len() != CALLS
        || grants.len() != CALLS
        || results.len() != CALLS
        || cleanups.len() != CALLS
    {
        return Err(format!(
            "fixed grant/Start/native/cleanup counts differ: {}/{}/{}/{}",
            grants.len(),
            starts.len(),
            results.len(),
            cleanups.len()
        )
        .into());
    }
    let background = &run_ids[FOREGROUND_SESSIONS];
    let mut background_positions = Vec::new();
    let mut tickets = Vec::new();
    for (index, ((grant, start), result)) in grants.iter().zip(&starts).zip(&results).enumerate() {
        let id = &result["identity"];
        let reservation_id = id["reservation_lease_id"]
            .as_str()
            .and_then(|lease| lease.strip_prefix("runtime-registry."))
            .and_then(|lease| lease.parse::<u64>().ok())
            .ok_or("noncanonical native lease")?;
        let barriers: Vec<_> = native
            .iter()
            .filter(|row| row["phase"] == "barrier" && &row["identity"] == id)
            .collect();
        if barriers.len() != 2
            || barriers.iter().any(|row| {
                row["registry"]["reservations"]
                    .as_array()
                    .is_none_or(|leases| {
                        !leases
                            .iter()
                            .any(|lease| lease["reservation_id"] == reservation_id)
                    })
            })
        {
            return Err(
                "actual lease not charged at both pre-native and post-drain barriers".into(),
            );
        }
        let ticket = grant["ticket"]
            .as_u64()
            .ok_or("grant lacks registration ticket")?;
        if tickets.contains(&ticket) {
            return Err("grant reused a registration ticket".into());
        }
        tickets.push(ticket);
        if grant["workflow_run_id"] != id["workflow_run_id"]
            || grant["task_id"] != id["task_id"]
            || start["workflow_run_id"] != id["workflow_run_id"]
            || start["payload"]["scheduler_task_id"] != id["task_id"]
            || start["payload"]["scheduler_attempt_id"] != id["attempt_id"]
        {
            return Err("grant, actual Start and native identity differ".into());
        }
        let response: RuntimeHostBatchExecutionResponse =
            serde_json::from_value(result["response"].clone())?;
        if response.state != RuntimeHostBatchExecutionState::Completed
            || response.members.len() != 1
            || response.members[0].state != RuntimeHostBatchExecutionMemberState::Completed
            || result["receipt_previous_owner"].is_null()
            || result["receipt_current_owner"].is_null()
        {
            return Err("actual completed native result/linear receipt missing".into());
        }
        let position = match id["node_id"].as_str() {
            Some("infer_a") => 0,
            Some("infer_b") => 1,
            _ => return Err("unexpected native node".into()),
        };
        let output = response.members[0]
            .outputs
            .iter()
            .find(|o| o.port_id == "embedding")
            .ok_or("missing native embedding output")?;
        let RuntimeHostExecutionOutputValue::Json(vector) = &output.value else {
            return Err("native embedding is not structured JSON".into());
        };
        let actual = vector.as_array().ok_or("native embedding is not array")?;
        let expected = golden["truncation_vectors"][position]
            .as_array()
            .ok_or("missing reference")?;
        if actual.len() != expected.len()
            || actual
                .iter()
                .zip(expected)
                .any(|(a, b)| match (a.as_f64(), b.as_f64()) {
                    (Some(a), Some(b)) => !a.is_finite() || (a - b).abs() > 1e-5,
                    _ => true,
                })
        {
            return Err("native output differs from independent Transformers reference".into());
        }
        let cleanup = cleanups[index];
        let requested = native
            .iter()
            .find(|r| r["phase"] == "cleanup_requested" && &r["identity"] == id)
            .ok_or("cleanup input not retained")?;
        let application: ReservationLifecycleApplication =
            serde_json::from_value(cleanup["application"].clone())?;
        let event: ReservationLifecycleEvent = serde_json::from_value(requested["event"].clone())?;
        if cleanup["identity"] != *id
            || application.state != ReservationLifecycleApplicationState::Applied
            || application.lifecycle_event_id != event.lifecycle_event_id
            || application.reservation_lease_id != event.reservation_lease_id
            || json!(application.reservation_lease_id) != id["reservation_lease_id"]
        {
            return Err(
                "actual native cleanup receipt application does not match its lease/event".into(),
            );
        }
        if grant["workflow_run_id"] == json!(background) {
            background_positions.push(index);
        }
    }
    if background_positions.len() != 2 || background_positions[0] != 3 {
        return Err(format!("background not protected after exactly two observed bypass grants: {background_positions:?}").into());
    }
    let protected = grants[background_positions[0]];
    if protected["protected"] != true
        || protected["bypasses"] != 2
        || protected["competing_priority"] != 100
    {
        return Err(
            "background grant lacks protected bypass2 over a pending higher priority unit".into(),
        );
    }
    let cancellations: Vec<_> = native
        .iter()
        .filter(|r| r["phase"] == "cancellation_request")
        .collect();
    if cancellations.len() != 1 || cancellations[0]["response"]["ok"] != true {
        return Err("missing unique successful public Running cancellation request".into());
    }
    let cancelled = &cancellations[0]["identity"];
    let cancelled_lease = cancelled["reservation_lease_id"]
        .as_str()
        .and_then(|lease| lease.strip_prefix("runtime-registry."))
        .and_then(|lease| lease.parse::<u64>().ok())
        .ok_or("cancelled lease identity absent")?;
    if cancellations[0]["registry_after_drain_before_cleanup"]["reservations"]
        .as_array()
        .is_none_or(|leases| {
            !leases
                .iter()
                .any(|lease| lease["reservation_id"] == cancelled_lease)
        })
    {
        return Err(
            "actual cancellation request released its charged lease before receipt cleanup".into(),
        );
    }
    let phases = [
        "native_result",
        "cancellation_request",
        "native_return",
        "cleanup_requested",
        "cleanup_result",
    ];
    let mut positions = Vec::new();
    for phase in phases {
        let matching: Vec<_> = native
            .iter()
            .enumerate()
            .filter(|(_, row)| row["phase"] == phase && &row["identity"] == cancelled)
            .collect();
        if matching.len() != 1 {
            return Err("cancelled attempt lifecycle observation missing or duplicated".into());
        }
        positions.push(matching[0].0);
    }
    if positions.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("native result/cancellation/return/receipt cleanup order differs".into());
    }
    if native[..positions[1]]
        .iter()
        .filter(|row| {
            row["phase"] == "native_result"
                && run_ids[..FOREGROUND_SESSIONS]
                    .iter()
                    .any(|run| row["identity"]["workflow_run_id"] == json!(run))
        })
        .count()
        != 8
    {
        return Err("public cancellation did not follow the eighth actual foreground drain".into());
    }
    let returns: Vec<_> = native
        .iter()
        .filter(|row| row["phase"] == "native_return")
        .collect();
    verify_sample_sequence(&results, &returns, cancelled, samples)?;
    Ok(
        json!({"public_sessions":rows.len(),"authenticated_units":2,"native_outputs":CALLS,
        "public_vectors":CALLS,"matched_cleanup_receipts":CALLS,"background_grant_positions":background_positions,
        "protected_background_bypasses":2,"competing_foreground_priority":100,"background_priority":-100,
        "successful_uncancelled_samples":samples.len(),"cancelled_attempt_excluded_from_training_source_assisted":true,
        "sample_run_attempt_identity_available":false,
        "cancellation_phase":"after_actual_native_drain_before_receipt_return",
        "in_flight_compute_cancellation_claim":false,"all_vectors_match_reference":true}),
    )
}

/// The ring exposes local task names, digest, cost and age, not run/attempt IDs.
/// Frozen source appends successful uncancelled matched-cleanup observations in
/// serial order, with no eviction in this five-per-key cohort. Check aggregate
/// order/multiplicity; never infer per-attempt timing identity from a local name.
fn verify_sample_sequence(
    results: &[&Value],
    returns: &[&Value],
    cancelled: &Value,
    samples: &RawSamples,
) -> ProbeResult<()> {
    if results.len() != returns.len()
        || results
            .iter()
            .zip(returns)
            .enumerate()
            .any(|(index, (result, returned))| {
                result["identity"] != returned["identity"]
                    || returns[..index]
                        .iter()
                        .any(|earlier| earlier["identity"] == returned["identity"])
            })
    {
        return Err(
            "native returns do not uniquely match completed result identities in serial order"
                .into(),
        );
    }
    let mut expected = Vec::new();
    let mut cancelled_count = 0;
    for returned in returns {
        if &returned["identity"] == cancelled {
            cancelled_count += 1;
            if returned["cancellation"]["state"] != "cancellation_requested" {
                return Err("cancelled native attempt did not observe cancellation".into());
            }
        } else {
            if returned["cancellation"]["state"] != "running" {
                return Err("unexpected cancellation state on other native attempt".into());
            }
            expected.push(
                returned["identity"]["task_id"]
                    .as_str()
                    .ok_or("native local task ID missing")?,
            );
        }
    }
    if cancelled_count != 1 || !samples.iter().map(|sample| sample.0.as_str()).eq(expected) {
        return Err("aggregate ring sequence differs from uncancelled native returns".into());
    }
    Ok(())
}

/// Re-evaluate saved data only. Preserve original native/oracle status and binary.
pub(super) fn reanalyze(root: &Path) -> ProbeResult<()> {
    fn read(root: &Path, name: &str) -> ProbeResult<Value> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(name))?)?)
    }
    let rows = read(root, "m3-cohort-results.json")?["rows"]
        .as_array()
        .ok_or("missing saved cohort rows")?
        .clone();
    let sessions: Vec<WorkflowExecutionSessionCreateResponse> =
        serde_json::from_value(read(root, "m3-sessions.json")?)?;
    let native = read(root, "m3-native.json")?
        .as_array()
        .ok_or("missing native rows")?
        .clone();
    let service_log = read(root, "m3-service-log.json")?
        .as_array()
        .ok_or("missing grant rows")?
        .clone();
    let events = read(root, "m3-events.json")?
        .as_array()
        .ok_or("missing ledger rows")?
        .clone();
    let samples: RawSamples = serde_json::from_value(read(root, "m3-samples.json")?)?;
    let golden: Value = serde_json::from_str(include_str!(
        "../../../inference/tests/fixtures/candle_bert/bert-8/golden.json"
    ))?;
    let result = check_evidence(
        &rows,
        &sessions,
        &native,
        &service_log,
        &events,
        &samples,
        &golden,
    );
    let output = json!({"kind":"read_only_reanalysis_of_original_cohort","passed":result.is_ok(),
        "result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),
        "original_oracle_status":read(root,"m3-oracle-status.json")?,
        "native_reexecuted":false,"sample_timing_attempt_attribution_claim":false,
        "training_evidence":"aggregate ring correspondence plus frozen serial append/cancellation source and native regressions"});
    let target = root
        .parent()
        .ok_or("reanalysis root lacks parent")?
        .join("reanalysis.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec_pretty(&output)?)?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    result.map(|_| ())
}

#[cfg(test)]
mod sample_oracle_tests {
    use super::*;
    fn returned(run: &str, state: &str) -> Value {
        json!({"identity":{"workflow_run_id":run,"task_id":"infer_b","attempt_id":format!("attempt.{run}")},"cancellation":{"state":state}})
    }
    #[test]
    fn repeated_local_names_allow_prior_successes_and_exclude_only_cancelled_identity() {
        let rows = [
            returned("a", "running"),
            returned("b", "running"),
            returned("c", "cancellation_requested"),
        ];
        let samples = vec![
            ("infer_b".into(), [1; 32], 10, 20),
            ("infer_b".into(), [1; 32], 10, 10),
        ];
        assert!(verify_sample_sequence(
            &rows.iter().collect::<Vec<_>>(),
            &rows.iter().collect::<Vec<_>>(),
            &rows[2]["identity"],
            &samples
        )
        .is_ok());
    }
    #[test]
    fn missing_or_extra_sample_rejects_even_with_identical_local_names() {
        let rows = [
            returned("a", "running"),
            returned("b", "running"),
            returned("c", "cancellation_requested"),
        ];
        let sample = ("infer_b".into(), [1; 32], 10, 10);
        for samples in [
            vec![sample.clone()],
            vec![sample.clone(), sample.clone(), sample],
        ] {
            assert!(verify_sample_sequence(
                &rows.iter().collect::<Vec<_>>(),
                &rows.iter().collect::<Vec<_>>(),
                &rows[2]["identity"],
                &samples
            )
            .is_err());
        }
    }
    #[test]
    fn matching_local_names_cannot_hide_foreign_or_duplicate_native_returns() {
        let rows = [
            returned("a", "running"),
            returned("b", "cancellation_requested"),
        ];
        let samples = vec![("infer_b".into(), [1; 32], 10, 10)];
        for bad in [
            vec![
                rows[0].clone(),
                returned("foreign", "cancellation_requested"),
            ],
            vec![rows[0].clone(), rows[0].clone()],
        ] {
            assert!(verify_sample_sequence(
                &rows.iter().collect::<Vec<_>>(),
                &bad.iter().collect::<Vec<_>>(),
                &rows[1]["identity"],
                &samples
            )
            .is_err());
        }
    }
    #[test]
    fn matching_name_and_count_cannot_replace_actual_cancellation_observation() {
        let rows = [returned("a", "running"), returned("b", "running")];
        let samples = vec![("infer_b".into(), [1; 32], 10, 10)];
        assert!(verify_sample_sequence(
            &rows.iter().collect::<Vec<_>>(),
            &rows.iter().collect::<Vec<_>>(),
            &rows[1]["identity"],
            &samples
        )
        .is_err());
    }
}

pub(super) async fn qualify(root: &Path) -> ProbeResult<()> {
    log::set_logger(&SERVICE_LOG)
        .map_err(|_| "M3 qualification requires exclusive logger ownership")?;
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
        "m3-protocol.json",
        &json!({"protocol":PROTOCOL,"foreground_sessions":4,
        "background_sessions":1,"authenticated_units":2,"branches_per_session":2,"native_calls":CALLS,
        "foreground_priority":100,"background_priority":-100,"minimum_samples":8,"maximum_age_seconds":60,
        "first_native_admission":"pause Running pre-forward, launch remaining sessions, require4 actual Ready waiters",
        "cancellation":"eighth foreground after actual native drain before unchanged receipt return",
        "barriers_per_call":["pre_native","post_native_drain"],"gate_watchdog_seconds":60,
        "cohort_watchdog_seconds":120,"cohort_restarts":0,"threads":1,"performance_claim":false}),
    )?;
    save(
        root,
        "m3-inputs.json",
        &json!({"texts":texts,"text_blake3":texts.iter()
        .map(|text|blake3::hash(text.as_bytes()).to_hex().to_string()).collect::<Vec<_>>(),
        "graph":pair_graph()?,"graph_blake3":blake3::hash(&serde_json::to_vec(&pair_graph()?)?).to_hex().to_string()}),
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
        "m3-bootstrap",
        gateway.clone(),
        registry.clone(),
        selector.clone(),
        false,
    )
    .await?;
    let mut baseline_edit = None;
    let mut retaining = None;
    let mut graph_edit = None;
    let mut serial = None;
    let mut sessions = Vec::new();
    let mut rows = Vec::new();
    let mut samples = Vec::new();
    let observations = Arc::new(Mutex::new(Observations::default()));
    let mut stage = "bootstrap_authoring";
    let result = async {
        baseline_edit = Some(author_workflow(&baseline, root, &mut stage, true).await?);
        stage = "bootstrap_session";
        retaining = Some(
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
            retaining.as_deref().ok_or("missing retaining session")?,
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
            "m3-bootstrap.json",
            &json!({"response":bootstrap.0,"readiness_resume_run_ids":bootstrap.1}),
        )?;
        gateway
            .publish_resident_cpu_cleanup_owner(&registry)
            .await?;
        stage = "serial_composition";
        let (barrier_tx, barrier_rx) = mpsc::channel(1);
        let port = Arc::new(ControlledPort {
            native: EmbeddedRetainedCpuSerialPort::new(
                gateway.clone(),
                registry.clone(),
                selector.clone(),
            ),
            barriers: barrier_tx,
            observations: observations.clone(),
        });
        serial = Some(
            compose_with_serial_port(
                root,
                "m3",
                gateway.clone(),
                registry.clone(),
                selector.clone(),
                Some((
                    WorkflowSerialReadyConfig::new(8, Duration::from_secs(60))?
                        .with_fair_service_opportunities(),
                    port,
                )),
            )
            .await?,
        );
        let runtime = serial.as_ref().ok_or("missing M3 runtime")?;
        graph_edit = Some(
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
        stage = "authenticated_session_creation";
        attributed_sessions(runtime, &mut sessions).await?;
        save(root, "m3-sessions.json", &sessions)?;
        let first = sessions[0]
            .attribution
            .as_ref()
            .ok_or("foreground attribution absent")?;
        let background = sessions[FOREGROUND_SESSIONS]
            .attribution
            .as_ref()
            .ok_or("background attribution absent")?;
        if sessions[..FOREGROUND_SESSIONS]
            .iter()
            .any(|s| s.attribution.as_ref() != Some(first))
            || (first.client_id == background.client_id && first.bucket_id == background.bucket_id)
        {
            return Err(
                "qualification sessions do not form exactly two authenticated units".into(),
            );
        }
        stage = "fixed_five_session_cohort";
        let (start, ready) = watch::channel(false);
        let execution = tokio::time::timeout(WATCHDOG * 2, async {
            tokio::join!(
                attempt(runtime, root, &sessions[0].session_id, 0, &texts, None),
                attempt(
                    runtime,
                    root,
                    &sessions[1].session_id,
                    1,
                    &texts,
                    Some(ready.clone())
                ),
                attempt(
                    runtime,
                    root,
                    &sessions[2].session_id,
                    2,
                    &texts,
                    Some(ready.clone())
                ),
                attempt(
                    runtime,
                    root,
                    &sessions[3].session_id,
                    3,
                    &texts,
                    Some(ready.clone())
                ),
                attempt(
                    runtime,
                    root,
                    &sessions[4].session_id,
                    4,
                    &texts,
                    Some(ready)
                ),
                coordinate(
                    runtime,
                    &registry,
                    root,
                    &sessions,
                    barrier_rx,
                    start,
                    observations.clone()
                ),
            )
        })
        .await;
        samples = runtime.workflow_service().serial_ready_samples_for_test();
        save(root, "m3-samples.json", &samples)?;
        let (a, b, c, d, e, coordinator) = execution?;
        // Observe all results, including independent failures, before propagation.
        let mut errors = Vec::new();
        for (index, result) in [a, b, c, d, e].into_iter().enumerate() {
            match result {
                Ok(row) => rows.push(row),
                Err(error) => errors.push(format!("session{index}: {error}")),
            }
        }
        if let Err(error) = coordinator {
            errors.push(format!("coordinator: {error}"));
        }
        let pending = runtime
            .workflow_service()
            .serial_service_pending_for_test()?;
        record(
            &observations,
            json!({"phase":"cohort_terminal","pending":pending,"registry":registry.snapshot()}),
        );
        if pending != 0 {
            errors.push("service registrations remain after observed terminal results".into());
        }
        save(
            root,
            "m3-cohort-results.json",
            &json!({"rows":rows,"errors":errors}),
        )?;
        if !errors.is_empty() {
            return Err(errors.join("; ").into());
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    // Cleanup is explicit after every stage obtaining a runtime. A watchdog
    // stops the caller's wait, not native execution or custody; shutdown drains.
    let mut cleanup_errors = Vec::new();
    if let Some(runtime) = serial.as_ref() {
        for session in &sessions {
            if let Err(error) = runtime
                .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                    session_id: session.session_id.clone(),
                })
                .await
            {
                cleanup_errors.push(format!("qualification session close: {error}"));
            }
        }
        if let Some(edit) = graph_edit {
            if let Err(error) = runtime
                .workflow_service()
                .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: edit,
                })
                .await
            {
                cleanup_errors.push(format!("qualification edit close: {error}"));
            }
        }
    }
    if let Some(session) = retaining {
        if let Err(error) = baseline
            .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
                session_id: session,
            })
            .await
        {
            cleanup_errors.push(format!("retaining session close: {error}"));
        }
    }
    if let Some(edit) = baseline_edit {
        if let Err(error) = baseline
            .workflow_service()
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit,
            })
            .await
        {
            cleanup_errors.push(format!("bootstrap edit close: {error}"));
        }
    }
    if let Some(runtime) = serial.as_ref() {
        if let Err(error) = runtime.shutdown().await {
            cleanup_errors.push(format!("serial shutdown: {error}"));
        }
    }
    if let Err(error) = baseline.shutdown().await {
        cleanup_errors.push(format!("bootstrap shutdown: {error}"));
    }
    let model_ref = serde_json::from_value(json!({"model_id":package.model_ref.model_id,
        "selected_artifact_id":package.model_ref.selected_artifact_id}))?;
    let final_registry = registry.snapshot();
    let native_owner_absent = gateway.resident_cpu_serial_owner(&model_ref).is_none();
    save(
        root,
        "m3-shutdown.json",
        &json!({"registry":final_registry,"cleanup_errors":cleanup_errors,
        "native_owner_absent":native_owner_absent,"stage":stage}),
    )?;
    save(
        root,
        "m3-execution-status.json",
        &json!({"completed":result.is_ok(),"stage":stage,
        "error":result.as_ref().err().map(ToString::to_string),"completed_runs":rows.len(),"cohort_restart_allowed":false}),
    )?;
    let (native, native_overflow) = {
        let state = observations.lock().expect("M3 collector");
        (state.rows.clone(), state.overflow)
    };
    let (service_log, log_overflow) = {
        let state = SERVICE_LOG.0.lock().expect("M3 log");
        (state.rows.clone(), state.overflow)
    };
    save(root, "m3-native.json", &native)?;
    save(root, "m3-service-log.json", &service_log)?;
    let events = if serial.is_some() {
        collect_events(root, "m3")?
    } else {
        Vec::new()
    };
    if let Err(error) = result {
        save(
            root,
            "m3-oracle-status.json",
            &json!({"passed":false,"evaluated":false,
            "reason":"execution incomplete","error":error.to_string()}),
        )?;
        return Err(error);
    }
    if !cleanup_errors.is_empty() || !final_registry.reservations.is_empty() || !native_owner_absent
    {
        return Err("M3 cleanup/shutdown incomplete or native owner/reservation remains".into());
    }
    if native_overflow || log_overflow {
        return Err("M3 bounded evidence collector overflow".into());
    }
    let oracle = check_evidence(
        &rows,
        &sessions,
        &native,
        &service_log,
        &events,
        &samples,
        &golden,
    );
    save(
        root,
        "m3-oracle-status.json",
        &json!({"passed":oracle.is_ok(),"error":oracle.as_ref().err().map(ToString::to_string)}),
    )?;
    let oracle = oracle?;
    save(
        root,
        "m3-summary.json",
        &json!({"protocol":PROTOCOL,"oracle":oracle,
        "backend":"candle_cpu_f32","threads":1,"declared_resident_ram_bytes":RESIDENT_ENVELOPE,
        "resident_bytes_measured":false,"model_acquisition_bytes":0,"controlled_admission_barriers":true,
        "default_policy_change":false,"performance_improvement_claim":false,"gpu_or_gui_claim":false}),
    )?;
    println!("{}", serde_json::to_string_pretty(&oracle)?);
    Ok(())
}

//! Actual tiny CPU load/forward/drain/release against the same controlled queue
//! store. Controlled Started/envelope facts are NOT production owner evidence.
use super::*;
use pantograph_workflow_service::workflow::{
    native_task_release::{WorkflowControlledNativeRelease, WorkflowNativeTaskReleaseObserver},
    WorkflowService, WorkflowServiceError,
};

fn identity(request: &RuntimeHostBatchExecutionRequest) -> SchedulerSerialAttemptIdentity<'_> {
    let m = &request.members[0];
    SchedulerSerialAttemptIdentity {
        workflow_id: m.handoff.workflow_id.as_str(),
        workflow_run_id: m.handoff.workflow_run_id.as_str(),
        node_id: m.handoff.node_id.as_str(),
        task_id: m.handoff.task_id.as_str(),
        attempt_id: "native.attempt",
        execution_request_id: &m.execution_request_id,
        candidate_id: "native.candidate",
        reservation_lease_id: m
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .reservation_lease_id
            .as_str(),
    }
}
fn controlled(
    f: &Fixture,
    request: &RuntimeHostBatchExecutionRequest,
    gateway: Arc<inference::InferenceGateway>,
    registry: SharedRuntimeRegistry,
) -> (
    WorkflowControlledNativeRelease,
    Arc<dyn WorkflowNativeTaskReleaseObserver>,
) {
    let service = WorkflowService::new();
    let out = service
        .controlled_native_task_release(request, identity(request), gateway, registry)
        .unwrap();
    // This is the actual service store, not a second controller in the native port.
    assert!(out.0.protected());
    assert!(!out.0.acknowledged());
    let _ = f;
    out
}
fn port_with(
    f: &Fixture,
    observer: Arc<dyn WorkflowNativeTaskReleaseObserver>,
) -> EmbeddedRetainedCpuSerialPort {
    EmbeddedRetainedCpuSerialPort {
        gateway: f.port.gateway.clone(),
        registry: f.port.registry.clone(),
        package: f.port.package.clone(),
        target: f.port.target.clone(),
        session_reservations: None,
        release_observer: Some(observer),
    }
}
async fn execute(
    port: &EmbeddedRetainedCpuSerialPort,
    request: &RuntimeHostBatchExecutionRequest,
) -> SerialRuntimeHostDrainedExecution {
    let admission = SchedulerSerialAdmission::new();
    port.execute_serial_singleton_with_cleanup(
        request.clone(),
        RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone()),
        bind(&admission, request, None),
    )
    .await
    .unwrap()
}
fn task_result(
    response: &RuntimeHostBatchExecutionResponse,
) -> pantograph_workflow_service::workflow::WorkflowSchedulerTaskResult {
    use pantograph_workflow_service::workflow::*;
    let m = &response.members[0];
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: m.workflow_id.to_string(),
        workflow_run_id: m.workflow_run_id.to_string(),
        node_id: m.node_id.to_string(),
        task_id: m.task_id.to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs: m
            .outputs
            .iter()
            .map(|o| WorkflowSchedulerTaskResultOutput {
                port_id: o.port_id.clone(),
                value: match &o.value {
                    RuntimeHostExecutionOutputValue::Json(v) => {
                        WorkflowSchedulerTaskResultValue::Json(v.clone())
                    }
                    _ => panic!("native embedding JSON"),
                },
            })
            .collect(),
        diagnostics: vec![],
        terminal_metadata: None,
    }
}
struct DuplicateObserver(Arc<dyn WorkflowNativeTaskReleaseObserver>);
impl WorkflowNativeTaskReleaseObserver for DuplicateObserver {
    fn observe_task_output(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.0.observe_task_output(proof)
    }
    fn check_dispatch(
        &self,
        r: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.0.check_dispatch(r, id)
    }
    fn acknowledge_task_release(
        &self,
        p: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        self.0.acknowledge_task_release(p)?;
        self.0.acknowledge_task_release(p)
    }
}
#[tokio::test]
async fn actual_release_latches_without_fresh_capture_or_output_acceptance() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.protected",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let port = port_with(&f, Arc::new(DuplicateObserver(observer)));
    let execution = execute(&port, &request).await;
    let result = task_result(&execution.response);
    assert!(
        !owner.acknowledged(),
        "drain alone is not reservation release"
    );
    assert!(
        owner
            .finish(SchedulerProtectedOutcome::AcceptedOutput)
            .is_err(),
        "a caller boolean cannot ACK physical release"
    );
    owner.set_stale_evidence(true);
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(
        owner.acknowledged(),
        "irreversible release is latched even while provider is stale"
    );
    assert!(
        owner.protected(),
        "cleanup must not finish the turn or accept output"
    );
    assert!(owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .is_err());
    assert!(
        owner.acknowledged(),
        "failed fresh advancement must not erase ACK"
    );
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_none());
    assert_eq!(
        f.port
            .registry
            .reservation_lease(f.successor.reservation_id),
        Some(f.successor.clone())
    );
    let residency = f.port.registry.snapshot().runtimes[0]
        .model_resource_residency
        .clone()
        .unwrap();
    assert_eq!(residency.requirements.unwrap().claims[0].bytes, 32768);
    owner.set_stale_evidence(false);
    assert!(owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .is_err());
    assert!(!owner.output_accepted());
    owner.accept_task_output(result).unwrap();
    assert!(owner.output_accepted());
    owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .unwrap();
    assert!(!owner.protected());
    assert!(
        !owner.acknowledged(),
        "successful explicit finish consumes the latch"
    );
}
#[tokio::test]
async fn full_intent_input_and_attempt_changes_refuse_before_native_effect() {
    for change in [
        "intent",
        "input",
        "attempt",
        "candidate",
        "request",
        "dispatch",
    ] {
        let f = fixture().await;
        let lease = task_lease(
            &f,
            f.request.handoff.workflow_id.as_str(),
            "native.protected",
        );
        let original = batch(&f, &lease);
        let (owner, observer) = controlled(
            &f,
            &original,
            f.port.gateway.clone(),
            f.port.registry.clone(),
        );
        let port = port_with(&f, observer);
        let mut request = original.clone();
        if change == "intent" {
            request.members[0].handoff.task_intent.fairness_key =
                Some("changed.app".parse().unwrap());
            request.members[0]
                .handoff
                .dispatch_decision
                .as_mut()
                .unwrap()
                .task_intent
                .fairness_key = Some("changed.app".parse().unwrap());
        }
        if change == "dispatch" {
            request.members[0]
                .handoff
                .dispatch_decision
                .as_mut()
                .unwrap()
                .reservations[0]
                .reserved_bytes -= 1;
        }
        if change == "input" {
            request.members[0].materialized_inputs[0].value =
                RuntimeHostExecutionInputValue::String("changed text".into());
        }
        if change == "request" {
            request.members[0].execution_request_id = "changed.request".into();
            request.anchor_execution_request_id = "changed.request".into();
        }
        request.validate().unwrap();
        let admission = SchedulerSerialAdmission::new();
        let mut id = identity(&request);
        if change == "attempt" {
            id.attempt_id = "changed.attempt";
        }
        if change == "candidate" {
            id.candidate_id = "changed.candidate";
        }
        let bound = admission
            .try_prepare()
            .unwrap()
            .begin_dispatch()
            .bind_unranked_attempt(id)
            .unwrap();
        let before = port
            .resident_serial_cpu_owner(&original.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        let result = port
            .execute_serial_singleton_with_cleanup(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                ),
                bound,
            )
            .await;
        assert!(result.is_err(), "{change}");
        if change == "dispatch" {
            assert!(result
                .err()
                .unwrap()
                .to_string()
                .contains("native selected dispatch changed"));
        }
        assert_eq!(
            port.resident_serial_cpu_owner(&original.members[0].handoff.task_intent)
                .unwrap()
                .snapshot,
            before,
            "{change}"
        );
        assert!(!owner.acknowledged());
        assert!(owner.protected());
        assert_eq!(
            f.port.registry.reservation_lease(lease.reservation_id),
            Some(lease)
        );
    }
}
struct CheckForeignOutput(Arc<dyn WorkflowNativeTaskReleaseObserver>);
impl WorkflowNativeTaskReleaseObserver for CheckForeignOutput {
    fn check_dispatch(
        &self,
        r: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.0.check_dispatch(r, id)
    }
    fn observe_task_output(
        &self,
        p: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        assert!(self.0.observe_task_output(p).is_err());
        Ok(())
    }
    fn acknowledge_task_release(
        &self,
        p: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        self.0.acknowledge_task_release(p)
    }
}
#[tokio::test]
async fn foreign_gateway_or_registry_proof_cannot_ack_bound_owner() {
    for foreign_gateway in [false, true] {
        let f = fixture().await;
        let lease = task_lease(
            &f,
            f.request.handoff.workflow_id.as_str(),
            "native.protected",
        );
        let request = batch(&f, &lease);
        let gateway = if foreign_gateway {
            Arc::new(
                inference::InferenceGateway::new_calibrated_candle_cpu(
                    inference::CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60))
                        .unwrap(),
                )
                .unwrap(),
            )
        } else {
            f.port.gateway.clone()
        };
        let registry = if foreign_gateway {
            f.port.registry.clone()
        } else {
            Arc::new(RuntimeRegistry::new())
        };
        let (owner, observer) = controlled(&f, &request, gateway, registry);
        let port = port_with(&f, Arc::new(CheckForeignOutput(observer)));
        let execution = execute(&port, &request).await;
        assert!(execution
            .cleanup
            .unwrap()
            .apply(cleanup_event(&request))
            .await
            .is_err());
        assert!(!owner.acknowledged());
        assert!(owner.protected());
        assert!(owner.finish(SchedulerProtectedOutcome::Failed).is_err());
        assert!(
            f.port
                .registry
                .reservation_lease(lease.reservation_id)
                .is_none(),
            "real release happened; observer refusal cannot undo it"
        );
        assert!(f
            .port
            .registry
            .reservation_lease(f.successor.reservation_id)
            .is_some());
    }
}
#[tokio::test]
async fn failed_retained_cleanup_never_mints_ack() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.protected",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let port = port_with(&f, observer);
    let execution = execute(&port, &request).await;
    f.port
        .registry
        .release_reservation(f.successor.reservation_id)
        .unwrap();
    assert!(
        execution
            .cleanup
            .unwrap()
            .apply(cleanup_event(&request))
            .await
            .is_err(),
        "last-lease eviction cannot use retained release"
    );
    assert!(!owner.acknowledged());
    assert!(owner.protected());
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_some());
}
#[tokio::test]
async fn collector_abort_during_resolver_has_no_release_ack() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.protected",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let entered = Arc::new(tokio::sync::Notify::new());
    let proceed = Arc::new(tokio::sync::Notify::new());
    let port = EmbeddedRetainedCpuSerialPort {
        package: Arc::new(PausedPackage {
            package: f.package.clone(),
            entered: entered.clone(),
            proceed,
        }),
        ..port_with(&f, observer)
    };
    let before = port
        .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
        .unwrap()
        .snapshot;
    let request_task = request.clone();
    let task = tokio::spawn(async move { execute(&port, &request_task).await });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert!(!owner.acknowledged());
    assert!(owner.protected());
    assert!(owner.finish(SchedulerProtectedOutcome::Withdrawn).is_err());
    assert_eq!(
        f.port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot,
        before
    );
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_some());
}

struct TimedObserver {
    inner: Arc<dyn WorkflowNativeTaskReleaseObserver>,
    samples: Arc<std::sync::Mutex<Vec<(&'static str, u128)>>>,
}
impl WorkflowNativeTaskReleaseObserver for TimedObserver {
    fn observe_task_output(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.inner.observe_task_output(proof)
    }
    fn check_dispatch(
        &self,
        r: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        let start = std::time::Instant::now();
        let result = self.inner.check_dispatch(r, id);
        self.samples
            .lock()
            .unwrap()
            .push(("dispatch_check", start.elapsed().as_nanos()));
        result
    }
    fn acknowledge_task_release(
        &self,
        p: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        let start = std::time::Instant::now();
        let result = self.inner.acknowledge_task_release(p);
        self.samples
            .lock()
            .unwrap()
            .push(("release_ack", start.elapsed().as_nanos()));
        result
    }
}
#[tokio::test]
async fn native_release_observer_cost_diagnostic() {
    let f = fixture().await;
    let samples = Arc::new(std::sync::Mutex::new(Vec::with_capacity(72)));
    for sample in 0..36 {
        let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.cost");
        let request = batch(&f, &lease);
        let (owner, observer) = controlled(
            &f,
            &request,
            f.port.gateway.clone(),
            f.port.registry.clone(),
        );
        let port = port_with(
            &f,
            Arc::new(TimedObserver {
                inner: observer,
                samples: samples.clone(),
            }),
        );
        let execution = execute(&port, &request).await;
        owner
            .accept_task_output(task_result(&execution.response))
            .unwrap();
        execution
            .cleanup
            .unwrap()
            .apply(cleanup_event(&request))
            .await
            .unwrap();
        assert!(owner.acknowledged());
        owner
            .finish(SchedulerProtectedOutcome::AcceptedOutput)
            .unwrap();
        if sample < 4 {
            samples.lock().unwrap().clear();
        }
    }
    let all = samples.lock().unwrap();
    let mut report = serde_json::Map::new();
    for kind in ["dispatch_check", "release_ack"] {
        let mut times: Vec<_> = all
            .iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, ns)| *ns)
            .collect();
        times.sort_unstable();
        assert_eq!(times.len(), 32);
        report.insert(
            kind.into(),
            serde_json::json!({"samples": 32, "warmups": 4,
            "p50_ns": times[15], "p95_ns": times[30], "max_ns": times[31]}),
        );
    }
    eprintln!(
        "NATIVE_TASK_RELEASE_COST {}",
        serde_json::Value::Object(report)
    );
}

struct RetryReplayObserver {
    inner: Arc<dyn WorkflowNativeTaskReleaseObserver>,
    owner: Arc<std::sync::Mutex<WorkflowControlledNativeRelease>>,
    next: RuntimeHostBatchExecutionRequest,
    gateway: Arc<inference::InferenceGateway>,
    registry: SharedRuntimeRegistry,
    retried: std::sync::atomic::AtomicBool,
}
impl WorkflowNativeTaskReleaseObserver for RetryReplayObserver {
    fn observe_task_output(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.inner.observe_task_output(proof)
    }
    fn check_dispatch(
        &self,
        r: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.inner.check_dispatch(r, id)
    }
    fn acknowledge_task_release(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        self.inner.acknowledge_task_release(proof)?;
        if !self.retried.swap(true, std::sync::atomic::Ordering::SeqCst) {
            // Explicit test result-owner interleaving, not an automatic native
            // terminal event: retry the same persistent turn with a NEW lease.
            let mut owner = self.owner.lock().unwrap();
            owner.finish(SchedulerProtectedOutcome::Failed)?;
            let mut id = identity(&self.next);
            id.attempt_id = "native.attempt.2";
            owner.retry(&self.next, id, self.gateway.clone(), self.registry.clone())?;
            assert!(
                self.inner.acknowledge_task_release(proof).is_err(),
                "old actual proof cannot ACK a new attempt/incarnation"
            );
            assert!(!owner.acknowledged());
            assert!(owner.protected());
        }
        Ok(())
    }
}
#[tokio::test]
async fn old_real_release_proof_cannot_overwrite_retry_with_new_incarnation() {
    let f = fixture().await;
    let first = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.first");
    let second = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.retry");
    assert_ne!(first.reservation_id, second.reservation_id);
    let request = batch(&f, &first);
    let next = batch(&f, &second);
    let (owner, inner) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let owner = Arc::new(std::sync::Mutex::new(owner));
    let observer = Arc::new(RetryReplayObserver {
        inner,
        owner: owner.clone(),
        next: next.clone(),
        gateway: f.port.gateway.clone(),
        registry: f.port.registry.clone(),
        retried: std::sync::atomic::AtomicBool::new(false),
    });
    let port = port_with(&f, observer);
    let execution = execute(&port, &request).await;
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(!owner.lock().unwrap().acknowledged());
    assert!(f
        .port
        .registry
        .reservation_lease(first.reservation_id)
        .is_none());
    assert_eq!(
        f.port.registry.reservation_lease(second.reservation_id),
        Some(second.clone())
    );
    let admission = SchedulerSerialAdmission::new();
    let mut id = identity(&next);
    id.attempt_id = "native.attempt.2";
    let bound = admission
        .try_prepare()
        .unwrap()
        .begin_dispatch()
        .bind_unranked_attempt(id)
        .unwrap();
    let execution = port
        .execute_serial_singleton_with_cleanup(
            next.clone(),
            RuntimeHostExecutionCancellationHandle::running(next.cancellation_context.clone()),
            bound,
        )
        .await
        .unwrap();
    owner
        .lock()
        .unwrap()
        .accept_task_output(task_result(&execution.response))
        .unwrap();
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&next))
        .await
        .unwrap();
    assert!(owner.lock().unwrap().acknowledged());
    owner
        .lock()
        .unwrap()
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .unwrap();
    assert!(!owner.lock().unwrap().protected());
    assert!(f
        .port
        .registry
        .reservation_lease(second.reservation_id)
        .is_none());
    assert!(f
        .port
        .registry
        .reservation_lease(f.successor.reservation_id)
        .is_some());
}
#[tokio::test]
async fn over_limit_independent_identity_refuses_before_store_installation() {
    let f = fixture().await;
    let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.bound");
    let request = batch(&f, &lease);
    let service = WorkflowService::new();
    let huge = format!("runtime-registry.{}", "0".repeat(65536));
    let mut id = identity(&request);
    id.reservation_lease_id = &huge;
    assert!(service
        .controlled_native_task_release(
            &request,
            id,
            f.port.gateway.clone(),
            f.port.registry.clone()
        )
        .is_err());
    service
        .controlled_native_task_release(
            &request,
            identity(&request),
            f.port.gateway.clone(),
            f.port.registry.clone(),
        )
        .unwrap();
}

#[tokio::test]
async fn task_output_commit_refuses_unproduced_malformed_and_stale_results_atomically() {
    use pantograph_workflow_service::workflow::WorkflowSchedulerTaskResultValue;
    let f = fixture().await;
    let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.output");
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    // A DTO cannot substitute for an actual producer proof.
    let fabricated = pantograph_workflow_service::workflow::WorkflowSchedulerTaskResult {
        schema_version: 1,
        workflow_id: request.members[0].handoff.workflow_id.to_string(),
        workflow_run_id: request.members[0].handoff.workflow_run_id.to_string(),
        node_id: request.members[0].handoff.node_id.to_string(),
        task_id: request.members[0].handoff.task_id.to_string(),
        status: pantograph_workflow_service::workflow::WorkflowSchedulerTaskResultStatus::Completed,
        outputs: vec![],
        diagnostics: vec![],
        terminal_metadata: None,
    };
    assert!(owner.accept_task_output(fabricated).is_err());
    assert!(!owner.output_accepted());
    let execution = execute(&port_with(&f, observer), &request).await;
    let result = task_result(&execution.response);
    for change in [
        "empty",
        "diagnostic",
        "duplicate",
        "wrong_port",
        "wrong_vector",
        "non_numeric",
        "oversize",
        "node",
        "workflow",
    ] {
        let mut changed = result.clone();
        let vector = changed
            .outputs
            .iter_mut()
            .find(|o| o.port_id == "embedding")
            .unwrap();
        match change {
            "empty" => changed.outputs.clear(),
            "diagnostic" => vector.value = WorkflowSchedulerTaskResultValue::DiagnosticOnly,
            "duplicate" => {
                let copy = vector.clone();
                changed.outputs.push(copy);
            }
            "wrong_port" => vector.port_id = "vector".into(),
            "wrong_vector" => {
                vector.value = WorkflowSchedulerTaskResultValue::Json(serde_json::json!([0.125]))
            }
            "non_numeric" => {
                vector.value = WorkflowSchedulerTaskResultValue::Json(serde_json::json!(["nan"]))
            }
            "oversize" => {
                vector.value =
                    WorkflowSchedulerTaskResultValue::Json(serde_json::json!(vec![0; 4097]))
            }
            "node" => changed.node_id = "foreign.node".into(),
            "workflow" => changed.workflow_id = "foreign.workflow".into(),
            _ => unreachable!(),
        }
        assert!(owner.accept_task_output(changed).is_err(), "{change}");
        assert!(!owner.output_accepted(), "{change}");
        assert!(owner.protected());
    }
    assert!(owner
        .accept_task_output_for_attempt(result.clone(), "old.attempt")
        .is_err());
    assert!(!owner.output_accepted());
    owner.set_stale_evidence(true);
    owner.accept_task_output(result.clone()).unwrap();
    assert!(
        owner.output_accepted(),
        "accepted transaction is historical, not a fresh provider claim"
    );
    assert!(
        owner.accept_task_output(result).is_err(),
        "duplicate commits cannot mint another result"
    );
    assert!(!owner.acknowledged());
    assert!(owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .is_err());
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(owner.acknowledged());
    assert!(
        owner
            .finish(SchedulerProtectedOutcome::AcceptedOutput)
            .is_err(),
        "both latches still require fresh owner evidence"
    );
    owner.set_stale_evidence(false);
    owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .unwrap();
    assert!(!owner.protected());
    assert!(f
        .port
        .registry
        .reservation_lease(f.successor.reservation_id)
        .is_some());
}

#[tokio::test]
async fn native_task_failure_commits_without_accepting_output_or_implying_no_effect() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.failed-result",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let execution = execute(&port_with(&f, observer), &request).await;
    let mut result = task_result(&execution.response);
    result.status =
        pantograph_workflow_service::workflow::WorkflowSchedulerTaskResultStatus::Failed;
    result.outputs.clear();
    owner.accept_task_output(result).unwrap();
    assert!(!owner.output_accepted());
    assert!(!owner.acknowledged());
    assert!(owner.finish(SchedulerProtectedOutcome::Failed).is_err());
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(owner.acknowledged());
    assert!(owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .is_err());
    owner.finish(SchedulerProtectedOutcome::Failed).unwrap();
    assert!(
        owner.protected(),
        "explicit failure keeps the bounded retry turn"
    );
}

struct LegacyReleaseObserver(Arc<dyn WorkflowNativeTaskReleaseObserver>);
impl WorkflowNativeTaskReleaseObserver for LegacyReleaseObserver {
    fn check_dispatch(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.0.check_dispatch(request, id)
    }
    fn acknowledge_task_release(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        self.0.acknowledge_task_release(proof)
    }
}
#[tokio::test]
async fn release_only_observers_remain_compatible_without_minting_output_acceptance() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.legacy-observer",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let execution = execute(
        &port_with(&f, Arc::new(LegacyReleaseObserver(observer))),
        &request,
    )
    .await;
    assert!(owner
        .accept_task_output(task_result(&execution.response))
        .is_err());
    assert!(!owner.output_accepted());
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(owner.acknowledged());
    assert!(owner
        .finish(SchedulerProtectedOutcome::AcceptedOutput)
        .is_err());
    owner.finish(SchedulerProtectedOutcome::Withdrawn).unwrap();
}

struct RefuseProducedOutput(Arc<dyn WorkflowNativeTaskReleaseObserver>);
impl WorkflowNativeTaskReleaseObserver for RefuseProducedOutput {
    fn check_dispatch(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        id: &SchedulerSerialAttemptIdentity<'_>,
    ) -> Result<(), WorkflowServiceError> {
        self.0.check_dispatch(request, id)
    }
    fn observe_task_output(
        &self,
        _proof: &inference::gateway::CandleCpuVerifiedTaskOutput<'_>,
    ) -> Result<(), WorkflowServiceError> {
        Err(WorkflowServiceError::InvalidRequest(
            "fixture lost result owner".into(),
        ))
    }
    fn acknowledge_task_release(
        &self,
        proof: &inference::gateway::CandleCpuVerifiedTaskRelease,
    ) -> Result<(), WorkflowServiceError> {
        self.0.acknowledge_task_release(proof)
    }
}
#[tokio::test]
async fn producer_observer_refusal_after_native_drain_keeps_original_custody_charged() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "native.output-owner-lost",
    );
    let request = batch(&f, &lease);
    let (owner, observer) = controlled(
        &f,
        &request,
        f.port.gateway.clone(),
        f.port.registry.clone(),
    );
    let port = port_with(&f, Arc::new(RefuseProducedOutput(observer)));
    let admission = SchedulerSerialAdmission::new();
    let result = port
        .execute_serial_singleton_with_cleanup(
            request.clone(),
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone()),
            bind(&admission, &request, None),
        )
        .await;
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("lost result owner"));
    assert!(!owner.output_accepted());
    assert!(!owner.acknowledged());
    assert!(owner.protected());
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease.clone())
    );
    assert!(
        f.port
            .registry
            .release_reservation(lease.reservation_id)
            .is_err(),
        "post-start abandonment must retain fenced custody"
    );
    assert!(owner.finish(SchedulerProtectedOutcome::Withdrawn).is_err());
}

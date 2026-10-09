use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use pantograph_diagnostics_ledger::{
    RuntimeHostObservationOutcome, RuntimeHostObservationProfile, RuntimeHostRequestObservation,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionPort, RuntimeHostExecutionPortError, RuntimeHostExecutionRequest,
    RuntimeHostExecutionResponse, RuntimeHostExecutionState, ValidatedRuntimeHostExecutionRequest,
};
use pantograph_workflow_service::workflow::WorkflowRuntimeHostObservationRecorder;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

// Bound running and queued diagnostic writes across all embedded execution ports.
// Saturation drops a sample rather than delaying admission or growing a queue.
static OBSERVATION_RECORDING_SLOTS: Semaphore = Semaphore::const_new(64);

/// Measures the single-request host boundary. Batch member compute, gateway
/// phases, physical hardware identity and competing gateway clients are unknown.
pub(crate) struct ObservedRuntimeHostExecutionPort {
    inner: Arc<dyn RuntimeHostExecutionPort>,
    recorder: WorkflowRuntimeHostObservationRecorder,
    host_epoch: String,
}

impl ObservedRuntimeHostExecutionPort {
    pub(crate) fn new(
        inner: Arc<dyn RuntimeHostExecutionPort>,
        recorder: WorkflowRuntimeHostObservationRecorder,
    ) -> Self {
        Self {
            inner,
            recorder,
            host_epoch: uuid::Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl RuntimeHostExecutionPort for ObservedRuntimeHostExecutionPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        let observation = match ValidatedRuntimeHostExecutionRequest::try_from(request.clone()) {
            Ok(validated) => match observation_profile(&validated, &self.host_epoch) {
                Ok(profile) => Some(RuntimeHostObservationAttempt {
                    recorder: self.recorder.clone(),
                    started: Instant::now(),
                    observation: Some(RuntimeHostRequestObservation {
                        observation_id: uuid::Uuid::new_v4().to_string(),
                        execution_request_id: request.execution_request_id.clone(),
                        workflow_id: request.handoff.workflow_id.to_string(),
                        workflow_run_id: request.handoff.workflow_run_id.to_string(),
                        task_id: request.handoff.task_id.to_string(),
                        profile,
                        outcome: RuntimeHostObservationOutcome::Abandoned,
                        host_elapsed_ms: 0,
                        recorded_at_ms: 0,
                    }),
                }),
                Err(error) => {
                    log::warn!("runtime-host observation profile unavailable: {error}");
                    None
                }
            },
            // The execution owner retains the invalid-request failure contract.
            Err(_) => None,
        };
        let result = self
            .inner
            .execute_runtime_host_request(request.clone(), cancellation)
            .await;
        if let Some(mut observation) = observation {
            if let Some(recording) = observation.finish(observation_outcome(&request, &result)) {
                if let Err(error) = recording.await {
                    log::warn!("runtime-host observation recording task failed: {error}");
                }
            }
        }
        result
    }
}

/// Conservative exact matching hashes selected declared model/revision/artifact,
/// runtime/device/environment, effective traits and materialized inputs. No raw
/// prompt, media reference, secret or configuration value is stored in the journal.
/// An absent model revision is hashed as absent, never invented. This fingerprint
/// does not establish immutable model contents or a calibrated hardware profile.
fn observation_profile(
    request: &ValidatedRuntimeHostExecutionRequest,
    host_epoch: &str,
) -> Result<RuntimeHostObservationProfile, String> {
    let request = request.as_ref();
    let decision = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or_else(|| "dispatch decision missing".to_owned())?;
    if decision.selected_model_ref.selected_artifact_path.is_some() {
        return Err("path-carrying model identity".into());
    }
    let model = &decision.selected_model_ref;
    let payload = serde_json::to_vec(&serde_json::json!({
        "profile_version": 1,
        "scope": "single_host_request_elapsed",
        "model_id": model.model_id,
        "model_revision": model.revision,
        "artifact_id": model.selected_artifact_id,
        "task_type": request.handoff.task_intent.task_type,
        "runtime_id": decision.selected_runtime_id,
        "runtime_variant": decision.selected_runtime_variant_id,
        "devices": decision.selected_device_ids,
        "environment": decision.environment_ref,
        "traits": decision.runtime_trait_settings,
        "inputs": request.materialized_inputs,
    }))
    .map_err(|error| format!("profile serialization failed: {error}"))?;
    Ok(RuntimeHostObservationProfile {
        host_epoch: host_epoch.to_owned(),
        request_fingerprint: blake3::hash(&payload).to_hex().to_string(),
    })
}

fn observation_outcome(
    request: &RuntimeHostExecutionRequest,
    result: &Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError>,
) -> RuntimeHostObservationOutcome {
    let Ok(response) = result else {
        return RuntimeHostObservationOutcome::Failed;
    };
    if response.validate().is_err()
        || response.execution_request_id != request.execution_request_id
        || response.workflow_id != request.handoff.workflow_id
        || response.workflow_run_id != request.handoff.workflow_run_id
        || response.node_id != request.handoff.node_id
        || response.task_id != request.handoff.task_id
    {
        return RuntimeHostObservationOutcome::Failed;
    }
    match response.state {
        RuntimeHostExecutionState::Completed => RuntimeHostObservationOutcome::Completed,
        RuntimeHostExecutionState::Rejected | RuntimeHostExecutionState::Failed => {
            if response.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == RuntimeHostExecutionDiagnosticCode::ShutdownRequested
            }) {
                RuntimeHostObservationOutcome::ShutdownAcknowledged
            } else if response.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == RuntimeHostExecutionDiagnosticCode::CancellationRequested
            }) {
                RuntimeHostObservationOutcome::CancellationAcknowledged
            } else if response.state == RuntimeHostExecutionState::Rejected {
                RuntimeHostObservationOutcome::Rejected
            } else {
                RuntimeHostObservationOutcome::Failed
            }
        }
        RuntimeHostExecutionState::Accepted => RuntimeHostObservationOutcome::Nonterminal,
        _ => RuntimeHostObservationOutcome::Nonterminal,
    }
}

struct RuntimeHostObservationAttempt {
    recorder: WorkflowRuntimeHostObservationRecorder,
    started: Instant,
    observation: Option<RuntimeHostRequestObservation>,
}

impl RuntimeHostObservationAttempt {
    fn finish(&mut self, outcome: RuntimeHostObservationOutcome) -> Option<JoinHandle<()>> {
        let mut observation = self.observation.take()?;
        observation.outcome = outcome;
        observation.host_elapsed_ms =
            u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        observation.recorded_at_ms = chrono::Utc::now().timestamp_millis();
        let recorder = self.recorder.clone();
        spawn_observation_recording(&OBSERVATION_RECORDING_SLOTS, move || {
            if let Err(error) = recorder.record(observation) {
                // Observation failure never changes the execution result. Queries
                // remain fallible and missing data cannot authorize prediction.
                log::warn!("runtime-host observation recording failed: {error}");
            }
        })
    }
}

impl Drop for RuntimeHostObservationAttempt {
    fn drop(&mut self) {
        // Never wait for SQLite from Drop, including cancellation on a Tokio
        // worker. The submitted write retains its admission permit until done.
        drop(self.finish(RuntimeHostObservationOutcome::Abandoned));
    }
}

fn spawn_observation_recording(
    slots: &'static Semaphore,
    record: impl FnOnce() + Send + 'static,
) -> Option<JoinHandle<()>> {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        log::warn!("runtime-host observation recording skipped: no Tokio runtime");
        return None;
    };
    let Ok(permit) = slots.try_acquire() else {
        log::warn!("runtime-host observation recording skipped: writer capacity exhausted");
        return None;
    };
    Some(runtime.spawn_blocking(move || {
        let _permit = permit;
        record();
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_diagnostics_ledger::RuntimeHostObservationQuery;
    use pantograph_workflow_service::WorkflowService;

    fn request() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        )).unwrap()
    }

    fn response() -> RuntimeHostExecutionResponse {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_completed_outputs.json"
        )).unwrap()
    }

    fn query(port: &ObservedRuntimeHostExecutionPort) -> RuntimeHostObservationQuery {
        RuntimeHostObservationQuery {
            profile: observation_profile(
                &ValidatedRuntimeHostExecutionRequest::try_from(request()).unwrap(),
                &port.host_epoch,
            )
            .unwrap(),
            since_ms: 0,
            until_ms: i64::MAX,
            sample_limit: 500,
        }
    }

    async fn recorded_summary(
        port: &ObservedRuntimeHostExecutionPort,
    ) -> pantograph_diagnostics_ledger::RuntimeHostObservationSummary {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let recorder = port.recorder.clone();
                let query = query(port);
                let summary = tokio::task::spawn_blocking(move || recorder.summary(query))
                    .await
                    .unwrap()
                    .unwrap();
                if summary.observed_count > 0 {
                    break summary;
                }
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("observation recording completes")
    }

    struct ControlledPort {
        response: RuntimeHostExecutionResponse,
    }

    #[async_trait]
    impl RuntimeHostExecutionPort for ControlledPort {
        async fn execute_runtime_host_request(
            &self,
            _request: RuntimeHostExecutionRequest,
            _cancellation: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
            Ok(self.response.clone())
        }
    }

    fn port(response: RuntimeHostExecutionResponse) -> ObservedRuntimeHostExecutionPort {
        let service = WorkflowService::with_ephemeral_diagnostics_ledger().unwrap();
        ObservedRuntimeHostExecutionPort::new(
            Arc::new(ControlledPort { response }),
            service.runtime_host_observation_recorder().unwrap(),
        )
    }

    #[tokio::test]
    async fn completed_response_is_preserved_and_recorded_once() {
        let original = response();
        let port = port(original.clone());
        let request = request();
        let returned = port
            .execute_runtime_host_request(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(request.cancellation_context),
            )
            .await
            .unwrap();
        assert_eq!(returned, original);
        let summary = port.recorder.summary(query(&port)).unwrap();
        assert_eq!(summary.observed_count, 1);
        assert_eq!(summary.completed_count, 1);
        assert!(summary.median_completed_host_elapsed_ms.is_some());
    }

    #[tokio::test]
    async fn mismatched_response_is_never_a_completed_sample() {
        let mut mismatched = response();
        mismatched.execution_request_id = "unrelated.request".into();
        let port = port(mismatched.clone());
        let request = request();
        let returned = port
            .execute_runtime_host_request(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(request.cancellation_context),
            )
            .await
            .unwrap();
        // The dispatcher still owns rejecting the unchanged response.
        assert_eq!(returned, mismatched);
        let summary = port.recorder.summary(query(&port)).unwrap();
        assert_eq!(summary.other_outcome_count, 1);
        assert_eq!(summary.median_completed_host_elapsed_ms, None);
    }

    #[test]
    fn cancellation_and_nonterminal_outcomes_do_not_become_success() {
        let request = request();
        for (code, expected) in [
            (
                RuntimeHostExecutionDiagnosticCode::CancellationRequested,
                RuntimeHostObservationOutcome::CancellationAcknowledged,
            ),
            (
                RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
                RuntimeHostObservationOutcome::ShutdownAcknowledged,
            ),
        ] {
            let mut response = response();
            response.state = RuntimeHostExecutionState::Rejected;
            response.outputs.clear();
            response.diagnostics[0].code = code;
            assert_eq!(observation_outcome(&request, &Ok(response)), expected);
        }
        let mut accepted = response();
        accepted.state = RuntimeHostExecutionState::Accepted;
        accepted.outputs.clear();
        assert_eq!(
            observation_outcome(&request, &Ok(accepted)),
            RuntimeHostObservationOutcome::Nonterminal
        );
        let error = RuntimeHostExecutionPortError::ExecutionFailed {
            message: "controlled failure".into(),
        };
        assert_eq!(
            observation_outcome(&request, &Err(error)),
            RuntimeHostObservationOutcome::Failed
        );
    }

    struct PendingPort;

    #[async_trait]
    impl RuntimeHostExecutionPort for PendingPort {
        async fn execute_runtime_host_request(
            &self,
            _request: RuntimeHostExecutionRequest,
            _cancellation: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
            std::future::pending().await
        }
    }

    #[tokio::test]
    async fn dropping_an_inflight_future_records_abandonment_without_backend_cancel_claim() {
        let service = WorkflowService::with_ephemeral_diagnostics_ledger().unwrap();
        let port = ObservedRuntimeHostExecutionPort::new(
            Arc::new(PendingPort),
            service.runtime_host_observation_recorder().unwrap(),
        );
        let request = request();
        let mut execution = Box::pin(port.execute_runtime_host_request(
            request.clone(),
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context),
        ));
        assert!(futures_util::poll!(execution.as_mut()).is_pending());
        drop(execution);
        // Drop submits best-effort recording without waiting for its SQLite write.
        let summary = recorded_summary(&port).await;
        assert_eq!(summary.observed_count, 1);
        assert_eq!(summary.other_outcome_count, 1);
        assert_eq!(summary.median_completed_host_elapsed_ms, None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn detached_recording_keeps_its_bounded_slot_without_blocking_the_executor() {
        static SLOTS: Semaphore = Semaphore::const_new(1);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let recording = spawn_observation_recording(&SLOTS, move || {
            started_tx.send(()).unwrap();
            // The timeout bounds teardown even if a regression fails the test.
            release_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        })
        .expect("first write admitted");
        started_rx.await.unwrap();
        drop(recording);
        assert_eq!(SLOTS.available_permits(), 0);
        assert!(spawn_observation_recording(&SLOTS, || {
            panic!("saturated writer must not enqueue another write");
        })
        .is_none());
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        release_tx.send(()).unwrap();
        let permit = tokio::time::timeout(std::time::Duration::from_secs(5), SLOTS.acquire())
            .await
            .expect("detached write releases its slot")
            .unwrap();
        drop(permit);
        spawn_observation_recording(&SLOTS, || {})
            .expect("capacity is reusable")
            .await
            .unwrap();
    }

    #[test]
    fn recording_without_a_runtime_is_skipped_without_running_inline() {
        static SLOTS: Semaphore = Semaphore::const_new(1);
        assert!(spawn_observation_recording(&SLOTS, || {
            panic!("recording must never fall back to the calling thread");
        })
        .is_none());
        assert_eq!(SLOTS.available_permits(), 1);
    }

    #[test]
    fn dropping_while_a_completed_write_is_queued_does_not_record_abandonment() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
            });
            started_rx.await.unwrap();
            let port = port(response());
            let request = request();
            let mut execution = Box::pin(port.execute_runtime_host_request(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(request.cancellation_context),
            ));
            // The host is already complete, but its diagnostic write is queued
            // behind the occupied blocking worker rather than running inline.
            assert!(futures_util::poll!(execution.as_mut()).is_pending());
            drop(execution);
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            let summary = recorded_summary(&port).await;
            assert_eq!(summary.observed_count, 1);
            assert_eq!(summary.completed_count, 1);
            assert_eq!(summary.other_outcome_count, 0);
        });
    }

    #[test]
    fn exact_profile_changes_with_materialized_configuration_but_not_attempt_ids() {
        let original = request();
        let profile = observation_profile(
            &ValidatedRuntimeHostExecutionRequest::try_from(original.clone()).unwrap(),
            "epoch.a",
        )
        .unwrap();
        let mut other_attempt = original.clone();
        other_attempt.execution_request_id = "another.request".into();
        assert_eq!(
            observation_profile(
                &ValidatedRuntimeHostExecutionRequest::try_from(other_attempt).unwrap(),
                "epoch.a"
            )
            .unwrap(),
            profile
        );
        let mut configured = original;
        configured.materialized_inputs[1].value =
            pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::U64(43);
        assert_ne!(
            observation_profile(
                &ValidatedRuntimeHostExecutionRequest::try_from(configured).unwrap(),
                "epoch.a"
            )
            .unwrap(),
            profile
        );
        assert!(!profile.request_fingerprint.contains("red cube"));
        assert_ne!(
            observation_profile(
                &ValidatedRuntimeHostExecutionRequest::try_from(request()).unwrap(),
                "epoch.b"
            )
            .unwrap(),
            profile
        );
    }
}

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use pantograph_workflow_service::{
    WorkflowExecutionSessionResumeRequest, WorkflowRunResponse, WorkflowServiceError,
};

use crate::dependency_readiness_auto_resume::{
    DependencyReadinessAutoResumePort, EmbeddedDependencyReadinessAutoResume,
    EmbeddedDependencyReadinessAutoResumeConfig,
};

#[tokio::test]
async fn auto_resume_shutdown_is_idempotent_and_noops_without_candidates() {
    let port = Arc::new(FakeAutoResumePort::new(vec![Ok(Vec::new())], Vec::new()));
    let handle = EmbeddedDependencyReadinessAutoResume::new(port.clone())
        .with_config(test_config())
        .spawn(tokio::runtime::Handle::current())
        .expect("auto-resume should spawn");

    tokio::time::sleep(Duration::from_millis(20)).await;

    handle.shutdown().await;
    handle.shutdown().await;
    assert!(
        port.resume_requests().is_empty(),
        "empty candidate polls must not call resume"
    );
    assert!(
        port.candidate_polls() > 0,
        "auto-resume should poll candidates before shutdown"
    );
}

#[tokio::test]
async fn auto_resume_retries_one_eligible_candidate_once() {
    let candidate = resume_request("session-a", "run-a");
    let port = Arc::new(FakeAutoResumePort::new(
        vec![Ok(vec![candidate.clone()]), Ok(Vec::new())],
        vec![Ok(WorkflowRunResponse {
            workflow_run_id: candidate.workflow_run_id.clone(),
            outputs: Vec::new(),
            timing_ms: 0,
        })],
    ));
    let handle = EmbeddedDependencyReadinessAutoResume::new(port.clone())
        .with_config(test_config())
        .spawn(tokio::runtime::Handle::current())
        .expect("auto-resume should spawn");

    wait_for_resume_count(port.as_ref(), 1).await;

    handle.shutdown().await;
    assert_eq!(port.resume_requests(), vec![candidate]);
}

#[tokio::test]
async fn auto_resume_skips_duplicate_candidates_in_one_poll() {
    let candidate = resume_request("session-a", "run-a");
    let port = Arc::new(FakeAutoResumePort::new(
        vec![
            Ok(vec![candidate.clone(), candidate.clone()]),
            Ok(Vec::new()),
        ],
        vec![Ok(WorkflowRunResponse {
            workflow_run_id: candidate.workflow_run_id.clone(),
            outputs: Vec::new(),
            timing_ms: 0,
        })],
    ));
    let handle = EmbeddedDependencyReadinessAutoResume::new(port.clone())
        .with_config(test_config())
        .spawn(tokio::runtime::Handle::current())
        .expect("auto-resume should spawn");

    wait_for_resume_count(port.as_ref(), 1).await;

    handle.shutdown().await;
    assert_eq!(port.resume_requests(), vec![candidate]);
}

#[tokio::test]
async fn auto_resume_treats_pending_readiness_as_non_terminal() {
    let candidate = resume_request("session-a", "run-a");
    let port = Arc::new(FakeAutoResumePort::new(
        vec![Ok(vec![candidate.clone()]), Ok(Vec::new())],
        vec![Err(
            WorkflowServiceError::RuntimeDependencyReadinessPending {
                message: "runtime dependency readiness is pending".to_string(),
                task_ids: vec!["infer".to_string()],
            },
        )],
    ));
    let handle = EmbeddedDependencyReadinessAutoResume::new(port.clone())
        .with_config(test_config())
        .spawn(tokio::runtime::Handle::current())
        .expect("auto-resume should spawn");

    wait_for_resume_count(port.as_ref(), 1).await;

    handle.shutdown().await;
    assert_eq!(port.resume_requests(), vec![candidate]);
}

#[tokio::test]
async fn auto_resume_rejects_zero_poll_interval() {
    let port = Arc::new(FakeAutoResumePort::new(Vec::new(), Vec::new()));
    let error = EmbeddedDependencyReadinessAutoResume::new(port)
        .with_config(EmbeddedDependencyReadinessAutoResumeConfig {
            poll_interval: Duration::ZERO,
        })
        .spawn(tokio::runtime::Handle::current())
        .expect_err("zero poll interval must reject");

    assert!(error
        .to_string()
        .contains("dependency-readiness auto-resume poll interval"));
}

fn test_config() -> EmbeddedDependencyReadinessAutoResumeConfig {
    EmbeddedDependencyReadinessAutoResumeConfig {
        poll_interval: Duration::from_millis(5),
    }
}

fn resume_request(
    session_id: &str,
    workflow_run_id: &str,
) -> WorkflowExecutionSessionResumeRequest {
    WorkflowExecutionSessionResumeRequest {
        session_id: session_id.to_string(),
        workflow_run_id: workflow_run_id.to_string(),
    }
}

async fn wait_for_resume_count(port: &FakeAutoResumePort, expected: usize) {
    for _ in 0..20 {
        if port.resume_requests().len() >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!(
        "timed out waiting for {expected} resume request(s), observed {:?}",
        port.resume_requests()
    );
}

struct FakeAutoResumePort {
    candidate_results:
        Mutex<VecDeque<Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError>>>,
    resume_results: Mutex<VecDeque<Result<WorkflowRunResponse, WorkflowServiceError>>>,
    candidate_polls: Mutex<usize>,
    resume_requests: Mutex<Vec<WorkflowExecutionSessionResumeRequest>>,
}

impl FakeAutoResumePort {
    fn new(
        candidate_results: Vec<
            Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError>,
        >,
        resume_results: Vec<Result<WorkflowRunResponse, WorkflowServiceError>>,
    ) -> Self {
        Self {
            candidate_results: Mutex::new(VecDeque::from(candidate_results)),
            resume_results: Mutex::new(VecDeque::from(resume_results)),
            candidate_polls: Mutex::new(0),
            resume_requests: Mutex::new(Vec::new()),
        }
    }

    fn candidate_polls(&self) -> usize {
        *self.candidate_polls.lock().expect("candidate poll lock")
    }

    fn resume_requests(&self) -> Vec<WorkflowExecutionSessionResumeRequest> {
        self.resume_requests
            .lock()
            .expect("resume request lock")
            .clone()
    }
}

#[async_trait]
impl DependencyReadinessAutoResumePort for FakeAutoResumePort {
    fn dependency_readiness_resume_candidates(
        &self,
    ) -> Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError> {
        *self.candidate_polls.lock().expect("candidate poll lock") += 1;
        self.candidate_results
            .lock()
            .expect("candidate result lock")
            .pop_front()
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    async fn resume_dependency_readiness(
        &self,
        request: WorkflowExecutionSessionResumeRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.resume_requests
            .lock()
            .expect("resume request lock")
            .push(request);
        self.resume_results
            .lock()
            .expect("resume result lock")
            .pop_front()
            .unwrap_or_else(|| {
                Ok(WorkflowRunResponse {
                    workflow_run_id: "default-run".to_string(),
                    outputs: Vec::new(),
                    timing_ms: 0,
                })
            })
    }
}

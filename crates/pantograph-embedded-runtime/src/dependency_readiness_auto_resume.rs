use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pantograph_workflow_service::{
    WorkflowExecutionSessionResumeRequest, WorkflowRunResponse, WorkflowServiceError,
};

use crate::EmbeddedRuntimeError;

/// Configuration for the embedded dependency-readiness auto-resume loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedDependencyReadinessAutoResumeConfig {
    pub poll_interval: Duration,
}

impl Default for EmbeddedDependencyReadinessAutoResumeConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(15),
        }
    }
}

#[async_trait]
pub trait DependencyReadinessAutoResumePort: Send + Sync {
    fn dependency_readiness_resume_candidates(
        &self,
    ) -> Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError>;

    async fn resume_dependency_readiness(
        &self,
        request: WorkflowExecutionSessionResumeRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError>;
}

/// Embedded-runtime owner for retrying active runs paused at dependency readiness.
#[derive(Clone)]
pub struct EmbeddedDependencyReadinessAutoResume {
    resume_port: Arc<dyn DependencyReadinessAutoResumePort>,
    config: EmbeddedDependencyReadinessAutoResumeConfig,
}

impl EmbeddedDependencyReadinessAutoResume {
    #[must_use]
    pub fn new(resume_port: Arc<dyn DependencyReadinessAutoResumePort>) -> Self {
        Self {
            resume_port,
            config: EmbeddedDependencyReadinessAutoResumeConfig::default(),
        }
    }

    #[must_use]
    pub fn with_config(mut self, config: EmbeddedDependencyReadinessAutoResumeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn spawn(
        self,
        runtime_handle: tokio::runtime::Handle,
    ) -> Result<EmbeddedDependencyReadinessAutoResumeHandle, EmbeddedRuntimeError> {
        if self.config.poll_interval.is_zero() {
            return Err(EmbeddedRuntimeError::Config {
                message: "dependency-readiness auto-resume poll interval must be greater than zero"
                    .to_string(),
            });
        }

        let resume_port = self.resume_port;
        let poll_interval = self.config.poll_interval;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        resume_dependency_readiness_candidates(resume_port.as_ref()).await;
                    }
                }
            }
        });

        Ok(EmbeddedDependencyReadinessAutoResumeHandle::new(
            shutdown_tx,
            join_handle,
        ))
    }
}

async fn resume_dependency_readiness_candidates(
    resume_port: &dyn DependencyReadinessAutoResumePort,
) {
    let candidates = match resume_port.dependency_readiness_resume_candidates() {
        Ok(candidates) => candidates,
        Err(error) => {
            log::warn!(
                "dependency-readiness auto-resume failed to list resume candidates: {error}"
            );
            return;
        }
    };
    if candidates.is_empty() {
        log::trace!("dependency-readiness auto-resume heartbeat: no resume candidates");
        return;
    }

    let mut attempted = BTreeSet::new();
    for candidate in candidates {
        let identity = (
            candidate.session_id.clone(),
            candidate.workflow_run_id.clone(),
        );
        if !attempted.insert(identity.clone()) {
            log::trace!(
                "dependency-readiness auto-resume skipped duplicate candidate for session '{}' run '{}'",
                identity.0,
                identity.1
            );
            continue;
        }
        match resume_port.resume_dependency_readiness(candidate).await {
            Ok(_) => {
                log::debug!(
                    "dependency-readiness auto-resume retried session '{}' run '{}'",
                    identity.0,
                    identity.1
                );
            }
            Err(error @ WorkflowServiceError::RuntimeDependencyReadinessPending { .. }) => {
                log::trace!(
                    "dependency-readiness auto-resume kept session '{}' run '{}' pending: {error}",
                    identity.0,
                    identity.1
                );
            }
            Err(error) => {
                log::warn!(
                    "dependency-readiness auto-resume failed for session '{}' run '{}': {error}",
                    identity.0,
                    identity.1
                );
            }
        }
    }
}

/// Handle for a tracked dependency-readiness auto-resume task.
#[derive(Debug)]
pub struct EmbeddedDependencyReadinessAutoResumeHandle {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl EmbeddedDependencyReadinessAutoResumeHandle {
    fn new(
        shutdown_tx: tokio::sync::watch::Sender<bool>,
        join_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            match join_handle.await {
                Ok(()) => {}
                Err(error) if error.is_panic() => {
                    log::error!(
                        "dependency-readiness auto-resume panicked during shutdown: {error}"
                    );
                }
                Err(error) => {
                    log::warn!(
                        "dependency-readiness auto-resume was cancelled during shutdown: {error}"
                    );
                }
            }
        }
    }
}

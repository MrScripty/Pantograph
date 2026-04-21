use std::sync::Arc;

use crate::scheduler::unix_timestamp_ms;

use super::{
    WorkflowHost, WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCloseResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionStaleCleanupRequest,
    WorkflowSessionStaleCleanupResponse, WorkflowSessionStaleCleanupWorker,
    WorkflowSessionStaleCleanupWorkerConfig, WorkflowSessionState, WorkflowSessionUnloadReason,
};

impl WorkflowService {
    pub async fn workflow_cleanup_stale_sessions(
        &self,
        request: WorkflowSessionStaleCleanupRequest,
    ) -> Result<WorkflowSessionStaleCleanupResponse, WorkflowServiceError> {
        if request.idle_timeout_ms == 0 {
            return Err(WorkflowServiceError::InvalidRequest(
                "idle_timeout_ms must be greater than zero".to_string(),
            ));
        }

        let now_ms = unix_timestamp_ms();
        let candidates = {
            let store = self.session_store_guard()?;
            store.stale_cleanup_candidates(now_ms, request.idle_timeout_ms)
        };

        let mut cleaned_session_ids = Vec::new();
        for candidate in candidates {
            let cleaned = {
                let mut store = self.session_store_guard()?;
                store.close_stale_session_if_unchanged(&candidate, now_ms, request.idle_timeout_ms)
            };
            if cleaned {
                cleaned_session_ids.push(candidate.session_id);
            }
        }

        Ok(WorkflowSessionStaleCleanupResponse {
            cleaned_session_ids,
        })
    }

    pub fn spawn_workflow_session_stale_cleanup_worker(
        self: &Arc<Self>,
        config: WorkflowSessionStaleCleanupWorkerConfig,
    ) -> Result<WorkflowSessionStaleCleanupWorker, WorkflowServiceError> {
        if config.interval.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup interval must be greater than zero".to_string(),
            ));
        }
        if config.idle_timeout.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup idle timeout must be greater than zero".to_string(),
            ));
        }

        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow-session stale cleanup worker requires an active Tokio runtime"
                    .to_string(),
            )
        })?;
        self.spawn_workflow_session_stale_cleanup_worker_with_handle(config, runtime_handle)
    }

    pub fn spawn_workflow_session_stale_cleanup_worker_with_handle(
        self: &Arc<Self>,
        config: WorkflowSessionStaleCleanupWorkerConfig,
        runtime_handle: tokio::runtime::Handle,
    ) -> Result<WorkflowSessionStaleCleanupWorker, WorkflowServiceError> {
        if config.interval.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup interval must be greater than zero".to_string(),
            ));
        }
        if config.idle_timeout.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup idle timeout must be greater than zero".to_string(),
            ));
        }

        let idle_timeout_ms = config.idle_timeout.as_millis().min(u128::from(u64::MAX)) as u64;
        let interval = config.interval;
        let service = Arc::clone(self);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(async move {
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(interval) => {
                        let _ = service
                            .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
                                idle_timeout_ms,
                            })
                            .await;
                    }
                }
            }
        });

        Ok(WorkflowSessionStaleCleanupWorker::new(
            shutdown_tx,
            join_handle,
        ))
    }

    pub async fn workflow_set_session_keep_alive<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionKeepAliveRequest,
    ) -> Result<WorkflowSessionKeepAliveResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let (state_after_update, unload_workflow_id) = {
            let mut store = self.session_store_guard()?;
            store.set_keep_alive(&session_id, request.keep_alive)?
        };

        if let Some(workflow_id) = unload_workflow_id {
            host.unload_session_runtime(
                &session_id,
                &workflow_id,
                WorkflowSessionUnloadReason::KeepAliveDisabled,
            )
            .await?;
        } else if request.keep_alive
            && matches!(state_after_update, WorkflowSessionState::IdleUnloaded)
        {
            let workflow_id = {
                let store = self.session_store_guard()?;
                store.session_summary(&session_id)?.workflow_id
            };
            if let Err(error) = self
                .ensure_keep_alive_session_runtime_ready(host, &session_id, &workflow_id)
                .await
            {
                if let Ok(mut rollback_store) = self.session_store.lock() {
                    let _ = rollback_store.set_keep_alive(&session_id, false);
                }
                return Err(error);
            }
        }

        let state = {
            let store = self.session_store_guard()?;
            store.session_summary(&session_id)?.state
        };
        Ok(WorkflowSessionKeepAliveResponse {
            session_id,
            keep_alive: request.keep_alive,
            state,
        })
    }

    pub async fn close_workflow_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionCloseRequest,
    ) -> Result<WorkflowSessionCloseResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let close_state = {
            let mut store = self.session_store_guard()?;
            store.close_session(&session_id)?
        };
        if close_state.runtime_loaded {
            host.unload_session_runtime(
                &session_id,
                &close_state.workflow_id,
                WorkflowSessionUnloadReason::SessionClosed,
            )
            .await?;
        }

        Ok(WorkflowSessionCloseResponse { ok: true })
    }
}

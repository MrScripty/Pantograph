use crate::scheduler::WorkflowSessionPreflightCache;
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::{
    WorkflowHost, WorkflowRuntimeCapability, WorkflowService, WorkflowServiceError,
    WorkflowSessionRetentionHint, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowSessionUnloadReason,
};

fn compute_runtime_capability_fingerprint(
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> String {
    let mut normalized = runtime_capabilities.to_vec();
    normalized.sort_by(|a, b| a.runtime_id.cmp(&b.runtime_id));
    for capability in &mut normalized {
        capability.backend_keys.sort();
        capability.missing_files.sort();
    }

    let encoded = serde_json::to_string(&normalized).unwrap_or_default();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in encoded.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

impl WorkflowService {
    pub fn invalidate_all_session_runtimes(&self) -> Result<Vec<String>, WorkflowServiceError> {
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        Ok(store.invalidate_all_loaded_session_runtimes())
    }

    pub(super) async fn ensure_session_runtime_loaded<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        enum RuntimeDecision {
            Ready,
            SelectUnloadCandidate {
                target: WorkflowSessionRuntimeSelectionTarget,
                candidates: Vec<WorkflowSessionRuntimeUnloadCandidate>,
                loaded_session_count: usize,
                max_loaded_sessions: usize,
            },
            LoadTarget {
                workflow_id: String,
                usage_profile: Option<String>,
                retention_hint: WorkflowSessionRetentionHint,
            },
        }

        loop {
            let decision = {
                let store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                let target = store.active.get(session_id).ok_or_else(|| {
                    WorkflowServiceError::SessionNotFound(format!(
                        "session '{}' not found",
                        session_id
                    ))
                })?;
                if target.runtime_loaded {
                    RuntimeDecision::Ready
                } else if store.loaded_session_count() >= store.max_loaded_sessions {
                    let loaded_session_count = store.loaded_session_count();
                    RuntimeDecision::SelectUnloadCandidate {
                        target: WorkflowSessionRuntimeSelectionTarget {
                            session_id: session_id.to_string(),
                            workflow_id: target.workflow_id.clone(),
                            usage_profile: target.usage_profile.clone(),
                            required_backends: target.required_backends.clone(),
                            required_models: target.required_models.clone(),
                        },
                        candidates: store.runtime_unload_candidates(session_id),
                        loaded_session_count,
                        max_loaded_sessions: store.max_loaded_sessions,
                    }
                } else {
                    RuntimeDecision::LoadTarget {
                        workflow_id: target.workflow_id.clone(),
                        usage_profile: target.usage_profile.clone(),
                        retention_hint: if target.keep_alive {
                            WorkflowSessionRetentionHint::KeepAlive
                        } else {
                            WorkflowSessionRetentionHint::Ephemeral
                        },
                    }
                }
            };

            match decision {
                RuntimeDecision::Ready => return Ok(()),
                RuntimeDecision::SelectUnloadCandidate {
                    target,
                    candidates,
                    loaded_session_count,
                    max_loaded_sessions,
                } => {
                    let Some(candidate) = host
                        .select_runtime_unload_candidate(&target, &candidates)
                        .await?
                    else {
                        return Err(WorkflowServiceError::scheduler_runtime_capacity_exhausted(
                            loaded_session_count,
                            max_loaded_sessions,
                            candidates.len(),
                        ));
                    };
                    host.unload_session_runtime(
                        &candidate.session_id,
                        &candidate.workflow_id,
                        WorkflowSessionUnloadReason::CapacityRebalance,
                    )
                    .await?;
                    if let Ok(mut store) = self.session_store.lock() {
                        let _ = store.mark_runtime_loaded(&candidate.session_id, false);
                    }
                }
                RuntimeDecision::LoadTarget {
                    workflow_id,
                    usage_profile,
                    retention_hint,
                } => {
                    host.load_session_runtime(
                        session_id,
                        &workflow_id,
                        usage_profile.as_deref(),
                        retention_hint,
                    )
                    .await?;
                    let mut store = self.session_store.lock().map_err(|_| {
                        WorkflowServiceError::Internal("session store lock poisoned".to_string())
                    })?;
                    store.mark_runtime_loaded(session_id, true)?;
                    return Ok(());
                }
            }
        }
    }

    pub(super) async fn ensure_session_runtime_preflight<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowSessionPreflightCache, WorkflowServiceError> {
        let graph_fingerprint = host.workflow_graph_fingerprint(workflow_id).await?;
        let runtime_capabilities = host.runtime_capabilities().await?;
        let runtime_capability_fingerprint =
            compute_runtime_capability_fingerprint(&runtime_capabilities);

        {
            let store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            if let Some(cached) = store.cached_preflight(session_id)? {
                if cached.graph_fingerprint == graph_fingerprint
                    && cached.runtime_capability_fingerprint == runtime_capability_fingerprint
                    && cached.override_selection == override_selection
                {
                    return Ok(cached);
                }
            }
        }

        let capabilities = host.workflow_capabilities(workflow_id).await?;
        let runtime_preflight = self
            .workflow_session_runtime_preflight_assessment(
                host,
                session_id,
                &capabilities,
                override_selection.clone(),
            )
            .await?;
        let cache = WorkflowSessionPreflightCache {
            graph_fingerprint,
            runtime_capability_fingerprint,
            override_selection,
            required_backends: capabilities.runtime_requirements.required_backends.clone(),
            required_models: capabilities.runtime_requirements.required_models.clone(),
            blocking_runtime_issues: runtime_preflight.blocking_runtime_issues,
        };

        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.cache_preflight(session_id, cache.clone())?;
        Ok(cache)
    }

    pub(super) async fn ensure_keep_alive_session_runtime_ready<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.refresh_session_runtime_affinity_basis(host, session_id, workflow_id)
            .await?;
        let cache = self
            .ensure_session_runtime_preflight(host, session_id, workflow_id, None)
            .await?;
        if !cache.blocking_runtime_issues.is_empty() {
            return Err(WorkflowServiceError::RuntimeNotReady(
                super::format_runtime_not_ready_message(&cache.blocking_runtime_issues),
            ));
        }
        self.ensure_session_runtime_loaded(host, session_id).await
    }

    pub(super) async fn refresh_session_runtime_affinity_basis<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let capabilities = host.workflow_capabilities(workflow_id).await?;
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.update_runtime_affinity_basis(
            session_id,
            capabilities.runtime_requirements.required_backends,
            capabilities.runtime_requirements.required_models,
        )?;
        Ok(())
    }
}

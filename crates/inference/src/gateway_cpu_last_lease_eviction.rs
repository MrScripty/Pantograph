//! Explicit drained-success to supervised actual last-lease stop handoff.
use super::*;

impl CandleCpuDrainedAttempt {
    /// Consume this custody-bearing actual success/drain receipt to evict its
    /// sole ephemeral lease. Retaining successors/pins refuse before stop.
    /// Cancellation before the irreversible boundary leaves execution fenced;
    /// after begin, supervision completes stop and ACK even if collector drops.
    /// This never recovers abandoned execution or accepts replacement snapshots.
    pub async fn evict_last(
        self,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<RuntimeRetentionDisposition, GatewayError> {
        let mut binding = self.binding;
        let gateway = binding.gateway.clone();
        if binding.custody.is_none() {
            return Err(refusal());
        }
        gateway.embedding_replacement.drain().await?;
        let mut backend = gateway.backend.clone().write_owned().await;
        binding.current(backend.as_ref())?;
        let lifecycle = gateway.runtime_lifecycle.read().await;
        let config = gateway.current_runtime_config.read().await;
        if canonical_backend_key(backend.name()) != "candle"
            || !backend.is_ready()
            || !lifecycle.active
            || lifecycle.runtime_id.as_deref() != Some("candle")
            || lifecycle.runtime_instance_id != binding.runtime_instance_id
            || config.as_ref().and_then(config_model_target).as_deref()
                != Some(binding.model_target.as_str())
            || config.as_ref().and_then(|c| c.model_name.as_deref())
                != Some(binding.owner.model_ref().model_id.as_str())
        {
            return Err(refusal());
        }
        binding.current(backend.as_ref())?;
        reject_cancelled_execution_handle("native CPU eviction", &host)?;
        let ticket = binding
            .custody
            .take()
            .ok_or_else(refusal)?
            .begin_last_lease_eviction(RuntimeRetainedOwnerIdentity {
                runtime_id: "candle",
                source_id: &gateway.resident_source_id,
                runtime_instance_id: lifecycle
                    .runtime_instance_id
                    .as_deref()
                    .ok_or_else(refusal)?,
                model_target: &binding.model_target,
            })
            .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;
        drop(config);
        drop(lifecycle);
        let dropped = Arc::new(AtomicBool::new(false));
        let mut cancel = CancelOnDrop(Some(dropped.clone()));
        let signal = InferenceExecutionCancellationHandle::with_signal(Arc::new(CallerSignal {
            dropped,
            host,
        }));
        // No await between the synchronous handoff and transferring both exact
        // owners into supervision. JoinHandle Drop does not abort this task.
        let task = tokio::spawn(async move {
            #[cfg(test)]
            eviction_test_phase(&gateway, "eviction_before_stop").await?;
            reject_cancelled_execution_handle("native CPU eviction before stop", &signal)?;
            binding.current(backend.as_ref())?;
            // Irreversible boundary: cancellation is advisory after this point.
            backend.stop().await?;
            if backend.is_ready() || backend.resident_cpu_calibration_instance().is_some() {
                return Err(refusal());
            }
            #[cfg(test)]
            eviction_test_phase(&gateway, "eviction_after_stop").await?;
            *gateway.embedding_mode.write().await = false;
            *gateway.reranking_mode.write().await = false;
            *gateway.external_mode.write().await = false;
            *gateway.current_runtime_config.write().await = None;
            let mut lifecycle = gateway.runtime_lifecycle.write().await;
            lifecycle.active = false;
            lifecycle.lifecycle_decision_reason = Some("native_last_lease_evicted".into());
            lifecycle.last_error = None;
            if lifecycle.warmup_completed_at_ms.is_none() {
                lifecycle.warmup_started_at_ms = None;
                lifecycle.warmup_timing_attempt_id = None;
                lifecycle.warmup_duration_ms = None;
            }
            // Allocate after actual stop while still holding the same writer.
            // All prior sampled frames, even undelivered ones, are now older.
            let sequence = gateway
                .resident_observation_sequence
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| s.checked_add(1))
                .map_err(|_| refusal())?
                + 1;
            let result = ticket
                .acknowledge_stopped(sequence)
                .map_err(|e| GatewayError::SwitchFailed(e.to_string()));
            drop(lifecycle);
            drop(backend);
            result
        });
        let result = task.await.map_err(|e| {
            GatewayError::SwitchFailed(format!("native CPU eviction supervision failed: {e}"))
        })??;
        cancel.0 = None;
        Ok(result)
    }
}

#[cfg(test)]
async fn eviction_test_phase(
    gateway: &InferenceGateway,
    phase: &'static str,
) -> Result<(), GatewayError> {
    let owner = gateway
        .candle_cpu_calibration
        .as_ref()
        .ok_or_else(refusal)?
        .clone();
    tokio::task::spawn_blocking(move || owner.test_attempt_phase(phase))
        .await
        .map_err(|_| refusal())
}
